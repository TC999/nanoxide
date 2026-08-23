/**************************************************************************
 * text.rs  --  GNU nano 文本编辑操作（对应 text.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 文本编辑核心：插入、删除、换行、缩进、注释、撤销/重做、自动换行。
//!
//! 转换说明：
//! - `linestruct` 链表用 `Rc<RefCell<LineStruct>>`；
//! - 所有全局状态经 [`openfile_ref`] 取出后操作；调用其他访问全局的
//!   函数前先释放 `RefMut` 借用，避免 `RefCell` 双重借用；
//! - `memmove`/`strcat` 等改为 `String`/`Vec<u8>` 等价操作；
//! - undo 数据结构（[`UndoStruct`]/[`GroupStruct`]）在 [`crate::definitions`]。

use crate::definitions::*;
use crate::chars;
use std::cell::RefCell;
use crate::cut;
use crate::files;
use crate::utils;
use crate::winio;
use std::rc::Rc;
#[cfg(not(target_os = "windows"))]
use std::io::Write;

/// 获取当前打开的缓冲区引用（克隆 Rc，释放全局借用）。
fn openfile_ref() -> OpenFileRef {
    with_global(|g| g.openfile.as_ref().expect("no open file").clone())
}

// ======================== 终端辅助（保留的简化接口） ========================

/// 初始化终端。
pub fn terminal_init() {
    let _ = crossterm::terminal::enable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
}

/// 退出 curses 模式。
pub fn endwin() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::LeaveAlternateScreen);
    let _ = crossterm::terminal::disable_raw_mode();
}

/// 在回答末尾放置光标。
pub fn put_cursor_at_end_of_answer() {
    // 由 winio 层处理
}

/// 取消操作。
pub fn do_cancel() {}

/// 退出编辑器。
/// 缓冲区已修改时先询问是否保存（对应 C 版 `do_exit`）：
/// - "是"：跳转到 Write to File 逻辑，保存成功（清除修改标记）后退出；
/// - "否"：直接退出；
/// - 取消（^C）：返回编辑器，不退出。
pub fn do_exit() {
    /* 未修改时直接退出。 */
    if !files::is_modified() {
        files::delete_lockfile_of_current_buffer();
        with_global_mut(|g| g.we_are_running = false);
        return;
    }

    let choice = crate::prompt::ask_user(false, &crate::t!("prompt-save_modified_buffer"));

    match choice {
        /* "是"：跳转到 Write to File 逻辑；保存成功（修改标记被清除）后退出。 */
        YES => {
            files::do_writeout();
            if !files::is_modified() {
                files::delete_lockfile_of_current_buffer();
                with_global_mut(|g| g.we_are_running = false);
            }
        }
        /* "否"：直接退出。 */
        NO => {
            files::delete_lockfile_of_current_buffer();
            with_global_mut(|g| g.we_are_running = false);
        }
        /* 取消：留在编辑器中。 */
        _ => winio::statusbar(&crate::t!("files-cancelled")),
    }
}

/// 刷新屏幕。
pub fn do_refresh() {
    with_global_mut(|g| g.refresh_needed = true);
}

/// 挂起编辑器（对应 nano.c 的 `do_suspend`）：受限模式时拒绝；
/// 否则恢复终端、显示提示，并把 SIGSTOP 发送给自己（Unix）。
/// Windows 平台没有 SIGSTOP，仅恢复终端后立即重绘。
pub fn do_suspend() {
    if files::in_restricted_mode() {
        return;
    }

    suspend_nano();
    with_global_mut(|g| g.ran_a_tool = true);
}

/// 实际执行挂起（对应 nano.c 的 `suspend_nano`）。
#[cfg(unix)]
fn suspend_nano() {
    winio::leave_terminal();
    println!("\n\n{}", crate::t!("text-use_fg"));
    let _ = std::io::stdout().flush();
    with_global_mut(|g| g.lastmessage = MessageType::Hush);
    unsafe {
        libc::signal(libc::SIGTSTP, libc::SIG_DFL);
        libc::raise(libc::SIGTSTP);
    }
    /* 从挂起恢复后：重新初始化终端并重绘。 */
    winio::enter_terminal();
    winio::full_refresh();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.focusing = true;
    });
}

/// 非 Unix 平台：恢复终端后立即重绘（无真正的挂起能力）。
#[cfg(not(unix))]
fn suspend_nano() {
    winio::leave_terminal();
    winio::enter_terminal();
    winio::full_refresh();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.focusing = true;
    });
}

// TODO: 翻译时单复数逻辑未翻译到位（两分支都返回 singular），暂注释占位，后续补上。
// 原型：
// pub fn P_<'a>(singular: &'a str, _plural: &'a str, number: usize) -> &'a str {
//     if number == 1 { singular } else { singular }
// }

/// 向缓冲区插入单个字符（旧接口）。
pub fn insert_char(ch: char) {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    inject(s.as_bytes(), s.len());
}

// ======================== 标记与制表符（对应 text.c） ========================

/// 切换标记（对应 `do_mark`）。
pub fn do_mark() {
    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();
    if of_ref.mark.is_some() {
        of_ref.mark = None;
        drop(of_ref);
        winio::statusbar(&crate::t!("text-mark_unset"));
        with_global_mut(|g| g.refresh_needed = true);
    } else {
        of_ref.mark = of_ref.current.clone();
        of_ref.mark_x = of_ref.current_x;
        of_ref.softmark = false;
        drop(of_ref);
        winio::statusbar(&crate::t!("text-mark_set"));
    }
}

/// 插入制表符（对应 `do_tab`）。
pub fn do_tab() {
    let of = openfile_ref();
    let (marked, mark_not_current) = {
        let of_ref = of.borrow();
        let marked = of_ref.mark.is_some();
        let same = of_ref.mark.as_ref().map(|m| {
            of_ref.current.as_ref().map(|c| Rc::ptr_eq(m, c)).unwrap_or(false)
        }).unwrap_or(false);
        (marked, marked && !same)
    };
    if marked && mark_not_current {
        do_indent();
        return;
    }

    /* 语法定义的 tabstring。 */
    let tabstring = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow().syntax.clone().and_then(|s| s.borrow().tabstring.clone())
        })
    });
    if let Some(ts) = tabstring {
        inject(ts.as_bytes(), ts.len());
        return;
    }

    if ISSET(TABS_TO_SPACES) {
        let (tabsize, col) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (g.tabsize, of.placewewant)
        });
        let length = tabsize - (col % tabsize);
        let spaces = vec![b' '; length];
        inject(&spaces, length);
    } else {
        inject(b"\t", 1);
    }
}

// ======================== 缩进（对应 text.c） ========================

/// 给给定行添加一个缩进（对应 `indent_a_line`）。
pub fn indent_a_line(line: &LineRef, indentation: &str) {
    let indent_len = indentation.len();

    /* 若请求的缩进为空，不改变该行。 */
    if indent_len == 0 {
        return;
    }

    /* 在行首添加构造的缩进。 */
    {
        let mut data = line.borrow().data.clone();
        data.insert_str(0, indentation);
        line.borrow_mut().data = data;
    }

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.totsize += indent_len;

            /* 补偿当前行中的变化。 */
            if of.mark.as_ref().map(|m| Rc::ptr_eq(m, line)).unwrap_or(false) && of.mark_x > 0 {
                of.mark_x += indent_len;
            }
            if of.current.as_ref().map(|c| Rc::ptr_eq(c, line)).unwrap_or(false) && of.current_x > 0 {
                of.current_x += indent_len;
                let cur = of.current.clone().unwrap();
                of.placewewant = utils::wideness(cur.borrow().data.as_bytes(), of.current_x);
            }
        }
    });
}

/// 返回给定文本开头空白字符的字节数，但最多一个制表符宽度
/// （对应 `length_of_white`）。
pub fn length_of_white(text: &[u8]) -> usize {
    /* 语法定义的 tabstring。 */
    let tabstring = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow().syntax.clone().and_then(|s| s.borrow().tabstring.clone())
        })
    });
    if let Some(ts) = tabstring {
        let thelength = ts.len();
        let mut white_count = 0;
        while text.get(white_count).copied().unwrap_or(0) == ts.as_bytes().get(white_count).copied().unwrap_or(0) {
            white_count += 1;
            if white_count == thelength {
                return thelength;
            }
        }
    }

    let tabsize = with_global(|g| g.tabsize);
    let mut white_count = 0;
    let mut pos = 0;
    loop {
        let c = text.get(pos).copied().unwrap_or(0);
        if c == b'\t' {
            return white_count + 1;
        }
        if c != b' ' {
            return white_count;
        }
        white_count += 1;
        if white_count == tabsize {
            return tabsize;
        }
        pos += 1;
    }
}

/// 当标记和光标位于给定行上时调整它们的位置（对应 `compensate_leftward`）。
pub fn compensate_leftward(line: &LineRef, leftshift: usize) {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            if of.mark.as_ref().map(|m| Rc::ptr_eq(m, line)).unwrap_or(false) {
                if of.mark_x < leftshift {
                    of.mark_x = 0;
                } else {
                    of.mark_x -= leftshift;
                }
            }
            if of.current.as_ref().map(|c| Rc::ptr_eq(c, line)).unwrap_or(false) {
                if of.current_x < leftshift {
                    of.current_x = 0;
                } else {
                    of.current_x -= leftshift;
                }
                let cur = of.current.clone().unwrap();
                of.placewewant = utils::wideness(cur.borrow().data.as_bytes(), of.current_x);
            }
        }
    });
}

/// 从给定行移除一个缩进（对应 `unindent_a_line`）。
pub fn unindent_a_line(line: &LineRef, indent_len: usize) {
    /* 若缩进为空，不改变该行。 */
    if indent_len == 0 {
        return;
    }

    /* 从该行移除第一个制表符宽度的空白。 */
    {
        let data = &mut line.borrow_mut().data;
        data.drain(..indent_len);
    }

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.totsize = of.totsize.saturating_sub(indent_len);
        }
    });

    /* 调整标记与光标的位置（若受影响）。 */
    compensate_leftward(line, indent_len);
}

/// 缩进当前行（或标记的各行）tabsize 列（对应 `do_indent`）。
pub fn do_indent() {
    /* 使用所有标记行或仅当前行。 */
    let (mut top, bot) = utils::get_range();

    /* 跳过开头的空行。 */
    loop {
        let is_bot_next = bot.borrow().next.as_ref().map(|n| Rc::ptr_eq(&top, n)).unwrap_or(false);
        if is_bot_next {
            break;
        }
        if !top.borrow().data.is_empty() {
            break;
        }
        let next = { let r = top.borrow(); r.next.clone() }.unwrap();
        top = next;
    }

    /* 若所有行都是空行，则无事可做。 */
    if bot.borrow().next.as_ref().map(|n| Rc::ptr_eq(&top, n)).unwrap_or(false) {
        return;
    }

    /* 构造缩进：语法 tabstring，或一串空格，或单个制表符。 */
    let tabstring = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow().syntax.clone().and_then(|s| s.borrow().tabstring.clone())
        })
    });
    let indentation: String = if let Some(ts) = tabstring {
        ts
    } else if ISSET(TABS_TO_SPACES) {
        " ".repeat(with_global(|g| g.tabsize))
    } else {
        "\t".to_string()
    };

    add_undo(UndoType::Indent, None);

    /* 逐行添加缩进，并在 undo 项中记录添加的内容。 */
    let mut line = Some(top.clone());
    loop {
        let Some(l) = line else { break };
        let is_bot = Rc::ptr_eq(&l, &bot);
        let real_indent = if l.borrow().data.is_empty() { "" } else { &indentation };
        indent_a_line(&l, real_indent);
        let lineno = l.borrow().lineno;
        update_multiline_undo(lineno, real_indent);
        if is_bot {
            break;
        }
        let next = { let r = l.borrow(); r.next.clone() };
        line = next;
    }

    files::set_modified();
    ensure_firstcolumn_is_aligned();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.shift_held = true;
    });
}

/// 取消当前行（或标记的各行）的缩进（对应 `do_unindent`）。
pub fn do_unindent() {
    let (mut top, bot) = utils::get_range();

    /* 跳过开头无法取消缩进的行。 */
    loop {
        let is_bot_next = bot.borrow().next.as_ref().map(|n| Rc::ptr_eq(&top, n)).unwrap_or(false);
        if is_bot_next {
            break;
        }
        if length_of_white(top.borrow().data.as_bytes()) != 0 {
            break;
        }
        let next = { let r = top.borrow(); r.next.clone() }.unwrap();
        top = next;
    }

    /* 若没有行可取消缩进，则无事可做。 */
    if bot.borrow().next.as_ref().map(|n| Rc::ptr_eq(&top, n)).unwrap_or(false) {
        return;
    }

    add_undo(UndoType::Unindent, None);

    /* 逐行移除其开头的缩进，并将移除的空白存入 undo 项。 */
    let mut line = Some(top.clone());
    loop {
        let Some(l) = line else { break };
        let is_bot = Rc::ptr_eq(&l, &bot);
        let data = l.borrow().data.clone();
        let indent_len = length_of_white(data.as_bytes());
        let indentation = String::from_utf8_lossy(&data.as_bytes()[..indent_len]).into_owned();

        unindent_a_line(&l, indent_len);
        let lineno = l.borrow().lineno;
        update_multiline_undo(lineno, &indentation);

        if is_bot {
            break;
        }
        let next = { let r = l.borrow(); r.next.clone() };
        line = next;
    }

    files::set_modified();
    ensure_firstcolumn_is_aligned();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.shift_held = true;
    });
}

/// 执行缩进或取消缩进动作的撤销或重做（对应 `handle_indent_action`）。
pub fn handle_indent_action(u: &UndoRef, undoing: bool, add_indent: bool) {
    let group = u.borrow().grouping.clone();
    let (head_lineno, head_x) = {
        let r = u.borrow();
        (r.head_lineno, r.head_x)
    };

    /* 重做时，重新定位光标并让缩进器调整它。 */
    if !undoing {
        crate::search::goto_line_posx(head_lineno, head_x);
    }

    if let Some(g) = &group {
        let mut group = Some(g.clone());
        while let Some(gr) = group {
            let top_line = gr.borrow().top_line;
            let bottom_line = gr.borrow().bottom_line;
            let indentations = gr.borrow().indentations.clone();

            let mut line = utils::line_from_number(top_line);
            loop {
                let lineno = line.borrow().lineno;
                if lineno > bottom_line {
                    break;
                }
                let idx = (lineno - top_line) as usize;
                if let Some(blanks) = &indentations[idx] {
                    if undoing ^ add_indent {
                        indent_a_line(&line, blanks);
                    } else {
                        unindent_a_line(&line, blanks.len());
                    }
                }
                let next = { let r = line.borrow(); r.next.clone() };
                match next {
                    Some(n) => line = n,
                    None => break,
                }
            }

            let next = { let r = gr.borrow(); r.next.clone() };
            group = next;
        }
    }

    /* 撤销时，把光标重新定位到记录的位置。 */
    if undoing {
        crate::search::goto_line_posx(head_lineno, head_x);
    }

    with_global_mut(|g| g.refresh_needed = true);
}

// ======================== 注释（对应 text.c） ========================

/// 测试给定行能否取消注释，或根据 action 添加/移除注释。
/// 返回 TRUE 当该行可取消注释，或添加/移除了任何内容
/// （对应 `comment_line`）。
pub fn comment_line(action: UndoType, line: &LineRef, comment_seq: &str) -> bool {
    let comment_seq_len = comment_seq.len();
    /* postfix：若是成对注释序列，post_seq 指向 '|' 之后。 */
    let post_pos = comment_seq.find('|');
    let pre_len = match post_pos {
        Some(p) => p,
        None => comment_seq_len,
    };
    let post_len = match post_pos {
        Some(_p) => comment_seq_len - pre_len - 1,
        None => 0,
    };
    let line_len = line.borrow().data.len();

    let (is_filebot, no_newlines) = with_global(|g| {
        (
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                of.filebot.as_ref().map(|b| {
                    of.current.as_ref().map(|c| Rc::ptr_eq(b, c)).unwrap_or(false)
                }).unwrap_or(false)
            }).unwrap_or(false),
            ISSET(NO_NEWLINES),
        )
    });
    if !no_newlines && is_filebot {
        return false;
    }

    if action == UndoType::Comment {
        /* 为注释序列腾出空间，把文本右移并复制进去。 */
        let cs = comment_seq.as_bytes();
        let mut data = line.borrow().data.clone().into_bytes();
        data.splice(0..0, cs[..pre_len].iter().cloned());
        if post_len > 0 {
            let post_seq = &cs[pre_len + 1..comment_seq_len];
            data.extend_from_slice(&post_seq[..post_len]);
        }
        line.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();

        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.totsize += pre_len + post_len;
                if of.mark.as_ref().map(|m| Rc::ptr_eq(m, line)).unwrap_or(false) && of.mark_x > 0 {
                    of.mark_x += pre_len;
                }
                if of.current.as_ref().map(|c| Rc::ptr_eq(c, line)).unwrap_or(false) && of.current_x > 0 {
                    of.current_x += pre_len;
                    let cur = of.current.clone().unwrap();
                    of.placewewant = utils::wideness(cur.borrow().data.as_bytes(), of.current_x);
                }
            }
        });
        return true;
    }

    /* 若该行已注释，报告为可取消注释，或取消注释。 */
    let data = line.borrow().data.clone();
    let bytes = data.as_bytes();
    let starts_with_comment = bytes.len() >= pre_len && &bytes[..pre_len] == comment_seq.as_bytes().get(..pre_len).unwrap_or(&[]);
    let ends_with_comment = post_len == 0
        || (line_len >= post_len && &bytes[line_len - post_len..] == comment_seq.as_bytes().get(pre_len + 1..).unwrap_or(&[]));

    if starts_with_comment && ends_with_comment {
        if action == UndoType::Preflight {
            return true;
        }

        /* 擦除注释前缀：移动非注释部分。 */
        let mut ndata = data.clone().into_bytes();
        ndata.drain(..pre_len);
        /* 截断后缀（若有）。 */
        let keep = ndata.len().saturating_sub(post_len);
        ndata.truncate(keep);
        line.borrow_mut().data = String::from_utf8_lossy(&ndata).into_owned();

        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.totsize = of.totsize.saturating_sub(pre_len + post_len);
            }
        });

        /* 需要时调整标记与光标的位置。 */
        compensate_leftward(line, pre_len);
        return true;
    }

    false
}

/// 注释或取消注释当前行或标记的各行（对应 `do_comment`）。
pub fn do_comment() {
    let comment_seq = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow().syntax.clone().and_then(|s| s.borrow().comment.clone())
        })
    }).unwrap_or_else(|| GENERAL_COMMENT_CHARACTER.to_string());

    if comment_seq.is_empty() {
        winio::statusline(MessageType::Ahem, &crate::t!("text-no_comment_syntax"));
        return;
    }

    /* 确定处理哪些行。 */
    let (top, bot) = utils::get_range();

    /* 若只选中了魔法行，不做任何事。 */
    let (is_filebot, _is_current, no_newlines) = with_global(|g| {
        (
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                of.filebot.as_ref().map(|b| {
                    of.current.as_ref().map(|c| Rc::ptr_eq(b, c)).unwrap_or(false)
                }).unwrap_or(false)
            }).unwrap_or(false),
            false,
            ISSET(NO_NEWLINES),
        )
    });
    if Rc::ptr_eq(&top, &bot) && is_filebot && !no_newlines {
        winio::statusline(MessageType::Ahem, &crate::t!("text-no_comment_past_eof"));
        return;
    }

    /* 判断选中行是要注释还是取消注释。 */
    let mut action = UndoType::Uncomment;
    let mut all_empty = true;
    let mut line = Some(top.clone());
    loop {
        let Some(l) = line else { break };
        let is_bot = Rc::ptr_eq(&l, &bot);
        let empty = {
            let data = l.borrow().data.clone();
            chars::white_string(data.as_bytes())
        };
        if !empty && !comment_line(UndoType::Preflight, &l, &comment_seq) {
            action = UndoType::Comment;
            break;
        }
        all_empty = all_empty && empty;
        if is_bot {
            break;
        }
        let next = { let r = l.borrow(); r.next.clone() };
        line = next;
    }

    /* 若所有选中行都是空行，则注释它们。 */
    if all_empty {
        action = UndoType::Comment;
    }

    add_undo(action, None);

    /* 存储操作使用的注释序列（文件名改变时它可能改变）。 */
    if let Some(u) = with_global(|g| g.openfile.as_ref().and_then(|of| of.borrow().current_undo.clone())) {
        u.borrow_mut().strdata = Some(comment_seq.clone());
    }

    /* 逐行注释/取消注释并存储 undo 数据。 */
    let mut line = Some(top.clone());
    loop {
        let Some(l) = line else { break };
        let is_bot = Rc::ptr_eq(&l, &bot);
        if comment_line(action, &l, &comment_seq) {
            let lineno = l.borrow().lineno;
            update_multiline_undo(lineno, "");
        }
        if is_bot {
            break;
        }
        let next = { let r = l.borrow(); r.next.clone() };
        line = next;
    }

    files::set_modified();
    ensure_firstcolumn_is_aligned();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.shift_held = true;
    });
}

/// 执行注释或取消注释动作的撤销或重做（对应 `handle_comment_action`）。
pub fn handle_comment_action(u: &UndoRef, undoing: bool, add_comment: bool) {
    let (head_lineno, head_x, strdata) = {
        let r = u.borrow();
        (r.head_lineno, r.head_x, r.strdata.clone())
    };

    /* 重做时，重新定位光标并让注释器调整它。 */
    if !undoing {
        crate::search::goto_line_posx(head_lineno, head_x);
    }

    let comment_seq = strdata.unwrap_or_default();

    let group = u.borrow().grouping.clone();
    let mut group = group;
    while let Some(gr) = group {
        let top_line = gr.borrow().top_line;
        let bottom_line = gr.borrow().bottom_line;

        let mut line = utils::line_from_number(top_line);
        loop {
            let lineno = line.borrow().lineno;
            if lineno > bottom_line {
                break;
            }
            let action = if undoing ^ add_comment { UndoType::Comment } else { UndoType::Uncomment };
            comment_line(action, &line, &comment_seq);
            let next = { let r = line.borrow(); r.next.clone() };
            match next {
                Some(n) => line = n,
                None => break,
            }
        }

        let next = { let r = gr.borrow(); r.next.clone() };
        group = next;
    }

    /* 撤销时，把光标重新定位到记录的位置。 */
    if undoing {
        crate::search::goto_line_posx(head_lineno, head_x);
    }

    with_global_mut(|g| g.refresh_needed = true);
}

// ======================== 行操作辅助（对应 text.c 的行操作） ========================

/// 返回给定行中缩进部分的长度（对应 `indent_length`）。
pub fn indent_length(line: &[u8]) -> usize {
    let mut pos = 0;
    while chars::byte_at(line, pos) != 0 && chars::is_blank_char(&line[pos..]) {
        pos += chars::char_length(&line[pos..]);
    }
    pos
}

// ======================== undo 系统（对应 text.c） ========================

/// 撤销一次剪切，或重做一次粘贴（对应 `undo_cut`）。
pub fn undo_cut(u: &UndoRef) {
    let (head_lineno, head_x, xflags) = {
        let r = u.borrow();
        (r.head_lineno, r.head_x, r.xflags)
    };

    crate::search::goto_line_posx(head_lineno, if xflags & WAS_WHOLE_LINE != 0 { 0 } else { head_x });

    /* 清除继承的锚点但保留用户放置的锚点。 */
    let of = openfile_ref();
    {
        let of_ref = of.borrow_mut();
        let current = of_ref.current.clone().unwrap();
        if xflags & HAD_ANCHOR_AT_START == 0 {
            current.borrow_mut().has_anchor = false;
        }
    }

    let cutbuffer = u.borrow().cutbuffer.clone();
    if let Some(cb) = cutbuffer {
        cut::copy_from_buffer(&cb);
    }

    /* 若原本也剪掉了最后一行，移除多余的魔法行。 */
    if xflags & INCLUDED_LAST_LINE != 0 && !ISSET(NO_NEWLINES) {
        let of = openfile_ref();
        let (filebot_ne_current, filebot_prev_empty) = {
            let of_ref = of.borrow();
            let filebot = of_ref.filebot.clone().unwrap();
            let prev = filebot.borrow().prev.clone();
            let not_current = of_ref.current.as_ref().map(|c| !Rc::ptr_eq(&filebot, c)).unwrap_or(false);
            let prev_empty = prev.as_ref().and_then(|w| w.upgrade())
                .map(|p| p.borrow().data.is_empty()).unwrap_or(false);
            (not_current, prev_empty)
        };
        if filebot_ne_current && filebot_prev_empty {
            utils::remove_magicline();
        }
    }

    if xflags & CURSOR_WAS_AT_HEAD != 0 {
        crate::search::goto_line_posx(head_lineno, head_x);
    }
}

/// 重做一次剪切，或撤销一次粘贴（对应 `redo_cut`）。
pub fn redo_cut(u: &UndoRef) {
    let oldcutbuffer = with_global(|g| g.cutbuffer.clone());
    with_global_mut(|g| g.cutbuffer = None);

    let (head_lineno, head_x, tail_lineno, tail_x, type_) = {
        let r = u.borrow();
        (r.head_lineno, r.head_x, r.tail_lineno, r.tail_x, r.type_)
    };

    let of = openfile_ref();
    {
        let mut of_ref = of.borrow_mut();
        of_ref.mark = Some(utils::line_from_number(head_lineno));
        of_ref.mark_x = if u.borrow().xflags & WAS_WHOLE_LINE != 0 { 0 } else { head_x };
    }
    crate::search::goto_line_posx(tail_lineno, tail_x);

    cut::do_snip(true, false, type_ == UndoType::Zap);

    files::free_lines(with_global(|g| g.cutbuffer.clone()));
    with_global_mut(|g| g.cutbuffer = oldcutbuffer);
}

/// 撤销最后所做（的若干）事情（对应 `do_undo`）。
pub fn do_undo() {
    let of = openfile_ref();
    let u = {
        let of_ref = of.borrow();
        of_ref.current_undo.clone()
    };
    let Some(u) = u else {
        drop(of);
        winio::statusline(MessageType::Ahem, &crate::t!("text-nothing_to_undo"));
        return;
    };

    let (type_, head_lineno, head_x, tail_lineno, tail_x, xflags, _wassize) = {
        let r = u.borrow();
        (r.type_, r.head_lineno, r.head_x, r.tail_lineno, r.tail_x, r.xflags, r.wassize)
    };

    let mut undidmsg: Option<&'static str> = None;

    if type_ as i32 <= UndoType::Replace as i32 {
        drop(of);
    } else {
        drop(of);
    }

    let line = if type_ as i32 <= UndoType::Replace as i32 {
        Some(utils::line_from_number(tail_lineno))
    } else {
        None
    };

    match type_ {
        UndoType::Add => {
            undidmsg = Some("addition");
            if xflags & INCLUDED_LAST_LINE != 0 && !ISSET(NO_NEWLINES) {
                utils::remove_magicline();
            }
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let slen = strdata.len();
                data.drain(head_x..head_x + slen);
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();
            }
            crate::search::goto_line_posx(head_lineno, head_x);
        }
        UndoType::Enter => {
            undidmsg = Some("line break");
            /* 自动缩进在行首空白处按 Enter 时删除了空白，并存储 x 位置 0。
             * 此时调整要返回和要收集数据的位置。 */
            let original_x = if head_x == 0 { tail_x } else { head_x };
            let regain_from_x = if head_x == 0 { 0 } else { tail_x };
            if let Some(l) = &line {
                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let tail = strdata[regain_from_x..].to_string();
                let mut data = l.borrow().data.clone();
                data.push_str(&tail);
                l.borrow_mut().data = data;

                /* 合并锚点。 */
                let next = { let r = l.borrow(); r.next.clone() }.unwrap();
                let next_anchor = next.borrow().has_anchor;
                let cur_anchor = l.borrow().has_anchor;
                l.borrow_mut().has_anchor = cur_anchor || next_anchor;
                files::unlink_node(&next);
                files::renumber_from(l);
                let of = openfile_ref();
                of.borrow_mut().current = Some(l.clone());
            }
            crate::search::goto_line_posx(head_lineno, original_x);
        }
        UndoType::Back | UndoType::Del => {
            undidmsg = Some("deletion");
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                let strdata = u.borrow().strdata.clone().unwrap_or_default().into_bytes();
                data.splice(head_x..head_x, strdata.iter().cloned());
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();
            }
            crate::search::goto_line_posx(tail_lineno, tail_x);
        }
        UndoType::Join => {
            undidmsg = Some("line join");
            /* 当连接是文件末尾的 Backspace 且 nonewlines 未设置时，
             * 不重新添加实际未删除的换行；仅定位光标。 */
            if xflags & WAS_BACKSPACE_AT_EOF != 0 && !ISSET(NO_NEWLINES) {
                let of = openfile_ref();
                let fb_lineno = of.borrow().filebot.as_ref().map(|b| b.borrow().lineno).unwrap_or(1);
                crate::search::goto_line_posx(fb_lineno, 0);
                with_global_mut(|g| g.focusing = false);
                break_case(&u, &mut undidmsg);
                finalize_undo(&u, undidmsg, true);
                return;
            }
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                data.truncate(tail_x);
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();

                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let intruder = make_new_node(Some(&*l.borrow()));
                intruder.borrow_mut().data = strdata;
                files::splice_node(l, &intruder);
                files::renumber_from(&intruder);
            }
            crate::search::goto_line_posx(head_lineno, head_x);
        }
        UndoType::Replace => {
            undidmsg = Some("replacement");
            if let Some(l) = &line {
                let data = u.borrow().strdata.clone();
                let line_data = l.borrow().data.clone();
                u.borrow_mut().strdata = Some(line_data);
                l.borrow_mut().data = data.unwrap_or_default();
            }
            crate::search::goto_line_posx(head_lineno, head_x);
        }
        UndoType::SplitBegin => {
            undidmsg = Some("addition");
        }
        UndoType::SplitEnd => {
            /* 跳过多行 undo 项之间的拆分项。 */
            let of = openfile_ref();
            let next = {
                let of_ref = of.borrow();
                of_ref.current_undo.as_ref().and_then(|cu| cu.borrow().next.clone())
            };
            if let Some(n) = next {
                of.borrow_mut().current_undo = Some(n);
            }
            drop(of);
            do_undo();
            let of = openfile_ref();
            let u2 = of.borrow().current_undo.clone();
            if let Some(u2) = u2 {
                let _of = openfile_ref();
                let head = { let r = u2.borrow(); (r.head_lineno, r.head_x) };
                crate::search::goto_line_posx(head.0, head.1);
            }
            return;
        }
        UndoType::Zap => {
            undidmsg = Some("erasure");
            undo_cut(&u);
        }
        UndoType::CutToEof | UndoType::Cut => {
            undidmsg = Some("cut");
            undo_cut(&u);
        }
        UndoType::Paste => {
            undidmsg = Some("paste");
            redo_cut(&u);
            if xflags & INCLUDED_LAST_LINE != 0 && !ISSET(NO_NEWLINES) {
                let of = openfile_ref();
                let (fb_ne_cur, _) = {
                    let of_ref = of.borrow();
                    let filebot = of_ref.filebot.clone().unwrap();
                    let not_current = of_ref.current.as_ref().map(|c| !Rc::ptr_eq(&filebot, c)).unwrap_or(false);
                    (not_current, ())
                };
                if fb_ne_cur {
                    utils::remove_magicline();
                }
            }
        }
        UndoType::Insert => {
            undidmsg = Some("insertion");
            let oldcutbuffer = with_global(|g| g.cutbuffer.clone());
            with_global_mut(|g| g.cutbuffer = None);
            crate::search::goto_line_posx(head_lineno, head_x);
            let of = openfile_ref();
            {
                let mut of_ref = of.borrow_mut();
                of_ref.mark = Some(utils::line_from_number(tail_lineno));
                of_ref.mark_x = tail_x;
            }
            cut::cut_marked_region();
            let cb = with_global(|g| g.cutbuffer.clone());
            u.borrow_mut().cutbuffer = cb;
            with_global_mut(|g| g.cutbuffer = oldcutbuffer);
            if xflags & INCLUDED_LAST_LINE != 0 && !ISSET(NO_NEWLINES) {
                let of = openfile_ref();
                let fb_ne_cur = {
                    let of_ref = of.borrow();
                    let filebot = of_ref.filebot.clone().unwrap();
                    of_ref.current.as_ref().map(|c| !Rc::ptr_eq(&filebot, c)).unwrap_or(false)
                };
                if fb_ne_cur {
                    utils::remove_magicline();
                }
            }
        }
        UndoType::CoupleBegin => {
            undidmsg = Some(u.borrow().strdata.clone().map(|s| leak_string(s)).unwrap_or("operation"));
            crate::search::goto_line_posx(head_lineno, head_x);
            let of = openfile_ref();
            let cursor_row = of.borrow().current_undo.as_ref().map(|cu| cu.borrow().tail_lineno).unwrap_or(0);
            of.borrow_mut().cursor_row = cursor_row;
            drop(of);
            winio::adjust_viewport(UpdateType::Stationary);
        }
        UndoType::CoupleEnd => {
            /* 为可能的 redo 记住光标所在行。 */
            let of = openfile_ref();
            let cursor_row = of.borrow().cursor_row;
            of.borrow_mut().current_undo.as_mut().map(|cu| cu.borrow_mut().head_lineno = cursor_row);
            let next = {
                let of_ref = of.borrow();
                of_ref.current_undo.as_ref().and_then(|cu| cu.borrow().next.clone())
            };
            of.borrow_mut().current_undo = next;
            drop(of);
            do_undo();
            do_undo();
            do_undo();
            return;
        }
        UndoType::Indent => {
            handle_indent_action(&u, true, true);
            undidmsg = Some("indent");
        }
        UndoType::Unindent => {
            handle_indent_action(&u, true, false);
            undidmsg = Some("unindent");
        }
        UndoType::Comment => {
            handle_comment_action(&u, true, true);
            undidmsg = Some("comment");
        }
        UndoType::Uncomment => {
            handle_comment_action(&u, true, false);
            undidmsg = Some("uncomment");
        }
        _ => {}
    }

    finalize_undo(&u, undidmsg, true);
}

/// do_undo/do_redo 共用的收尾逻辑。
fn finalize_undo(u: &UndoRef, undidmsg: Option<&str>, undoing: bool) {
    let is_zero = ISSET(ZERO);
    let pletion_active = with_global(|g| g.pletion_line.is_some());
    if let Some(msg) = undidmsg {
        if !is_zero && !pletion_active {
            let text = if undoing { crate::t!("text-undid", action = msg) } else { crate::t!("text-redid", action = msg) };
            winio::statusline(MessageType::Hush, &text);
        }
    }

    let of = openfile_ref();
    let next_undo = {
        let of_ref = of.borrow();
        of_ref.current_undo.as_ref().and_then(|cu| cu.borrow().next.clone())
    };
    of.borrow_mut().current_undo = next_undo;
    of.borrow_mut().last_action = UndoType::Other;
    of.borrow_mut().mark = None;

    let (wassize, newsize) = {
        let r = u.borrow();
        (r.wassize, r.newsize)
    };
    of.borrow_mut().totsize = if undoing { wassize } else { newsize };

    /* 颜色重算。 */
    let type_ = u.borrow().type_;
    if type_ as i32 <= UndoType::Replace as i32 {
        let of = openfile_ref();
        let current = of.borrow().current.clone().unwrap();
        let of = openfile_ref();
        of.borrow_mut().placewewant = utils::xplustabs();
        let _ = current;
    } else if type_ == UndoType::Insert || type_ == UndoType::CoupleBegin {
        with_global_mut(|g| g.recook = true);
    } else {
        let of = openfile_ref();
        of.borrow_mut().placewewant = utils::xplustabs();
    }

    /* 位于缓冲区最后保存处时，取消 "Modified" 标记。 */
    let at_last_saved = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            match (&of.current_undo, &of.last_saved) {
                (Some(cu), Some(ls)) => Rc::ptr_eq(cu, ls),
                (None, None) => true,
                _ => false,
            }
        }).unwrap_or(false)
    });
    if at_last_saved {
        let of = openfile_ref();
        of.borrow_mut().modified = false;
        winio::titlebar(None);
    } else {
        files::set_modified();
    }
}

/// 重做最后撤销的（若干）事情（对应 `do_redo`）。
pub fn do_redo() {
    let of = openfile_ref();
    let u = {
        let of_ref = of.borrow();
        of_ref.undotop.clone()
    };
    let Some(mut u) = u else {
        drop(of);
        winio::statusline(MessageType::Ahem, &crate::t!("text-nothing_to_redo"));
        return;
    };

    /* 找到当前 undo 项之前的那一项。 */
    loop {
        let is_next_current = {
            let of_ref = of.borrow();
            let next = u.borrow().next.clone();
            match (&next, &of_ref.current_undo) {
                /* C 语义：u->next == current_undo（NULL == NULL 也匹配）。 */
                (None, None) => true,
                (Some(n), Some(cu)) => Rc::ptr_eq(n, cu),
                _ => false,
            }
        };
        if is_next_current {
            break;
        }
        let next = { let r = u.borrow(); r.next.clone() };
        match next {
            Some(n) => u = n,
            None => {
                drop(of);
                winio::statusline(MessageType::Ahem, &crate::t!("text-nothing_to_redo"));
                return;
            }
        }
    }

    let (type_, head_lineno, head_x, tail_lineno, tail_x, xflags) = {
        let r = u.borrow();
        (r.type_, r.head_lineno, r.head_x, r.tail_lineno, r.tail_x, r.xflags)
    };

    let mut redidmsg: Option<&'static str> = None;

    let line = if type_ as i32 <= UndoType::Replace as i32 {
        Some(utils::line_from_number(tail_lineno))
    } else {
        None
    };

    match type_ {
        UndoType::Add => {
            redidmsg = Some("addition");
            if xflags & INCLUDED_LAST_LINE != 0 && !ISSET(NO_NEWLINES) {
                utils::new_magicline();
            }
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                let strdata = u.borrow().strdata.clone().unwrap_or_default().into_bytes();
                data.splice(head_x..head_x, strdata.iter().cloned());
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();
            }
            crate::search::goto_line_posx(tail_lineno, tail_x);
        }
        UndoType::Enter => {
            redidmsg = Some("line break");
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                data.truncate(head_x);
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();

                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let intruder = make_new_node(Some(&*l.borrow()));
                intruder.borrow_mut().data = strdata;
                files::splice_node(l, &intruder);
                files::renumber_from(&intruder);
            }
            crate::search::goto_line_posx(head_lineno + 1, tail_x);
        }
        UndoType::Back | UndoType::Del => {
            redidmsg = Some("deletion");
            if let Some(l) = &line {
                let mut data = l.borrow().data.clone().into_bytes();
                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let slen = strdata.len();
                data.drain(head_x..head_x + slen);
                l.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();
            }
            crate::search::goto_line_posx(head_lineno, head_x);
        }
        UndoType::Join => {
            redidmsg = Some("line join");
            if xflags & WAS_BACKSPACE_AT_EOF != 0 && !ISSET(NO_NEWLINES) {
                crate::search::goto_line_posx(tail_lineno, tail_x);
                finalize_redo(&u, redidmsg);
                return;
            }
            if let Some(l) = &line {
                let strdata = u.borrow().strdata.clone().unwrap_or_default();
                let mut data = l.borrow().data.clone();
                data.push_str(&strdata);
                l.borrow_mut().data = data;
                let next = { let r = l.borrow(); r.next.clone() }.unwrap();
                files::unlink_node(&next);
                files::renumber_from(l);
                let of = openfile_ref();
                of.borrow_mut().current = Some(l.clone());
            }
            crate::search::goto_line_posx(tail_lineno, tail_x);
        }
        UndoType::Replace => {
            redidmsg = Some("replacement");
            if let Some(l) = &line {
                let data = u.borrow().strdata.clone();
                let line_data = l.borrow().data.clone();
                u.borrow_mut().strdata = Some(line_data);
                l.borrow_mut().data = data.unwrap_or_default();
            }
            crate::search::goto_line_posx(head_lineno, head_x);
        }
        UndoType::SplitBegin => {
            let of = openfile_ref();
            of.borrow_mut().current_undo = Some(u.clone());
            drop(of);
            loop {
                let of = openfile_ref();
                let cu_type = of.borrow().current_undo.as_ref().map(|cu| cu.borrow().type_).unwrap_or(UndoType::Other);
                if cu_type == UndoType::SplitEnd {
                    break;
                }
                drop(of);
                do_redo();
            }
            let _of = openfile_ref();
            let head = { let r = u.borrow(); (r.head_lineno, r.head_x) };
            crate::search::goto_line_posx(head.0, head.1);
            ensure_firstcolumn_is_aligned();
        }
        UndoType::SplitEnd => {
            redidmsg = Some("addition");
        }
        UndoType::Zap => {
            redidmsg = Some("erasure");
            redo_cut(&u);
        }
        UndoType::CutToEof | UndoType::Cut => {
            redidmsg = Some("cut");
            redo_cut(&u);
        }
        UndoType::Paste => {
            redidmsg = Some("paste");
            redo_cut(&u);
        }
        UndoType::Insert => {
            redidmsg = Some("insertion");
            crate::search::goto_line_posx(head_lineno, head_x);
            let cutbuffer = u.borrow().cutbuffer.clone();
            if let Some(cb) = cutbuffer {
                cut::copy_from_buffer(&cb);
            }
            u.borrow_mut().cutbuffer = None;
        }
        UndoType::CoupleBegin => {
            let of = openfile_ref();
            of.borrow_mut().current_undo = Some(u.clone());
            drop(of);
            do_redo();
            do_redo();
            do_redo();
            return;
        }
        UndoType::CoupleEnd => {
            redidmsg = Some(u.borrow().strdata.clone().map(|s| leak_string(s)).unwrap_or("operation"));
            crate::search::goto_line_posx(tail_lineno, tail_x);
            let of = openfile_ref();
            let cursor_row = { let r = u.borrow(); r.head_lineno };
            of.borrow_mut().cursor_row = cursor_row;
            drop(of);
            winio::adjust_viewport(UpdateType::Stationary);
        }
        UndoType::Indent => {
            handle_indent_action(&u, false, true);
            redidmsg = Some("indent");
        }
        UndoType::Unindent => {
            handle_indent_action(&u, false, false);
            redidmsg = Some("unindent");
        }
        UndoType::Comment => {
            handle_comment_action(&u, false, true);
            redidmsg = Some("comment");
        }
        UndoType::Uncomment => {
            handle_comment_action(&u, false, false);
            redidmsg = Some("uncomment");
        }
        _ => {}
    }

    finalize_redo(&u, redidmsg);
}

/// do_redo 的收尾逻辑。
fn finalize_redo(u: &UndoRef, redidmsg: Option<&str>) {
    let is_zero = ISSET(ZERO);
    if let Some(msg) = redidmsg {
        if !is_zero {
            let text = crate::t!("text-redid", action = msg);
            winio::statusline(MessageType::Hush, &text);
        }
    }

    let of = openfile_ref();
    of.borrow_mut().current_undo = Some(u.clone());
    of.borrow_mut().last_action = UndoType::Other;
    of.borrow_mut().mark = None;

    let newsize = u.borrow().newsize;
    of.borrow_mut().totsize = newsize;
    of.borrow_mut().placewewant = utils::xplustabs();

    let type_ = u.borrow().type_;
    if type_ == UndoType::Insert || type_ == UndoType::CoupleEnd {
        with_global_mut(|g| g.recook = true);
    }

    /* 位于缓冲区最后保存处时，取消 "Modified" 标记。 */
    let at_last_saved = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            match (&of.current_undo, &of.last_saved) {
                (Some(cu), Some(ls)) => Rc::ptr_eq(cu, ls),
                (None, None) => true,
                _ => false,
            }
        }).unwrap_or(false)
    });
    if at_last_saved {
        let of = openfile_ref();
        of.borrow_mut().modified = false;
        winio::titlebar(None);
    } else {
        files::set_modified();
    }
}

/// 丢弃比给定项更新的 undo 项，或全部丢弃（NULL 时）
/// （对应 `discard_until`）。
pub fn discard_until(thisitem: Option<&UndoRef>) {
    let of = openfile_ref();
    let mut dropit = {
        let of_ref = of.borrow();
        of_ref.undotop.clone()
    };

    loop {
        let Some(d) = dropit.clone() else { break };
        let is_this = match thisitem {
            Some(t) => Rc::ptr_eq(&d, t),
            None => false,
        };
        if is_this {
            break;
        }

        /* 释放 strdata 与 cutbuffer（Rc 自动）。 */
        let next = { let r = d.borrow(); r.next.clone() };
        of.borrow_mut().undotop = next;
        dropit = of.borrow().undotop.clone();
    }

    /* 调整指向 undo 栈顶的指针。 */
    of.borrow_mut().current_undo = thisitem.cloned();

    /* 阻止连续编辑动作继续。 */
    of.borrow_mut().last_action = UndoType::Other;
}

/// 将一个新的指定类型 undo 项添加到当前堆栈顶部
/// （对应 `add_undo`）。
pub fn add_undo(action: UndoType, message: Option<&str>) {
    let of = openfile_ref();
    let thisline = {
        let of_ref = of.borrow();
        of_ref.current.clone().unwrap()
    };
    let (lineno, current_x, totsize, _undotop) = {
        let of_ref = of.borrow();
        (
            thisline.borrow().lineno,
            of_ref.current_x,
            of_ref.totsize,
            of_ref.undotop.clone(),
        )
    };

    /* 初始化新分配的 undo 项。 */
    let u = Rc::new(RefCell::new(UndoStruct {
        type_: action,
        strdata: None,
        cutbuffer: None,
        head_lineno: lineno,
        head_x: current_x,
        tail_lineno: lineno,
        tail_x: current_x,
        wassize: totsize,
        newsize: totsize,
        grouping: None,
        xflags: 0,
        next: None,
    }));

    /* 吹走任何已撤销的项。 */
    drop(of);
    discard_until(with_global(|g| g.openfile.as_ref().and_then(|o| o.borrow().current_undo.clone())).as_ref());

    /* 若某动作导致自动长行换行，在动作的 undo 项之下插入
     * SPLIT_BEGIN 项；否则直接把新项加到 undo 栈顶。 */
    let of = openfile_ref();
    let undotop = {
        let of_ref = of.borrow();
        of_ref.undotop.clone()
    };
    if action == UndoType::SplitBegin {
        if let Some(top) = &undotop {
            let top_wassize = top.borrow().wassize;
            u.borrow_mut().wassize = top_wassize;
            let top_next = { let r = top.borrow(); r.next.clone() };
            u.borrow_mut().next = top_next;
            top.borrow_mut().next = Some(u.clone());
        }
    } else {
        u.borrow_mut().next = undotop;
        let mut of_ref = of.borrow_mut();
        of_ref.undotop = Some(u.clone());
        of_ref.current_undo = Some(u.clone());
    }

    /* 记录撤销每种可能动作所需的信息。 */
    let mut thisaction = action;
    let filebot = {
        let of_ref = of.borrow();
        of_ref.filebot.clone().unwrap()
    };
    {
        let mut of_ref = of.borrow_mut();
        match action {
            UndoType::Add => {
                /* 若将添加新的魔法行，撤销应移除它。 */
                if Rc::ptr_eq(&thisline, &filebot) {
                    u.borrow_mut().xflags |= INCLUDED_LAST_LINE;
                }
            }
            UndoType::Enter => {}
            UndoType::Back => {
                /* 若下一行是魔法行，不要撤销这个退格。 */
                let next = { let r = thisline.borrow(); r.next.clone() };
                let is_magic = next.as_ref().map(|n| Rc::ptr_eq(n, &filebot)).unwrap_or(false);
                let has_text = !thisline.borrow().data.is_empty();
                if is_magic && has_text {
                    u.borrow_mut().xflags |= WAS_BACKSPACE_AT_EOF;
                }
                /* Fall-through。 */
                fallthrough_del(&u, &thisline, &mut thisaction, of_ref.current_x);
            }
            UndoType::Del => {
                fallthrough_del(&u, &thisline, &mut thisaction, of_ref.current_x);
            }
            UndoType::Replace => {
                let data = thisline.borrow().data.clone();
                u.borrow_mut().strdata = Some(data);
            }
            UndoType::SplitBegin | UndoType::SplitEnd => {}
            UndoType::CutToEof => {
                u.borrow_mut().xflags |= INCLUDED_LAST_LINE | CURSOR_WAS_AT_HEAD;
                if thisline.borrow().has_anchor {
                    u.borrow_mut().xflags |= HAD_ANCHOR_AT_START;
                }
            }
            UndoType::Zap | UndoType::Cut => {
                let mark_before = {
                    let mark = of_ref.mark.clone();
                    mark.map(|m| {
                        let m_line = m.borrow().lineno;
                        let c_line = thisline.borrow().lineno;
                        m_line < c_line || (Rc::ptr_eq(&m, &thisline) && of_ref.mark_x <= of_ref.current_x)
                    }).unwrap_or(false)
                };
                if of_ref.mark.is_some() {
                    if mark_before {
                        u.borrow_mut().head_lineno = of_ref.mark.as_ref().unwrap().borrow().lineno;
                        u.borrow_mut().head_x = of_ref.mark_x;
                        u.borrow_mut().xflags |= MARK_WAS_SET;
                    } else {
                        u.borrow_mut().tail_lineno = of_ref.mark.as_ref().unwrap().borrow().lineno;
                        u.borrow_mut().tail_x = of_ref.mark_x;
                        u.borrow_mut().xflags |= MARK_WAS_SET | CURSOR_WAS_AT_HEAD;
                    }
                    if u.borrow().tail_lineno == filebot.borrow().lineno {
                        u.borrow_mut().xflags |= INCLUDED_LAST_LINE;
                    }
                } else if !ISSET(CUT_FROM_CURSOR) {
                    u.borrow_mut().xflags |= WAS_WHOLE_LINE | CURSOR_WAS_AT_HEAD;
                    u.borrow_mut().tail_x = 0;
                } else {
                    u.borrow_mut().xflags |= CURSOR_WAS_AT_HEAD;
                }
                let anchor_at_start = (of_ref.mark.is_some() && mark_before
                    && of_ref.mark.as_ref().unwrap().borrow().has_anchor)
                    || ((of_ref.mark.is_none() || !mark_before) && thisline.borrow().has_anchor);
                if anchor_at_start {
                    u.borrow_mut().xflags |= HAD_ANCHOR_AT_START;
                }
            }
            UndoType::Paste => {
                let cb = with_global(|g| g.cutbuffer.clone());
                u.borrow_mut().cutbuffer = cb.map(|c| files::copy_buffer(&c));
                /* Fall-through。 */
                if Rc::ptr_eq(&thisline, &filebot) {
                    u.borrow_mut().xflags |= INCLUDED_LAST_LINE;
                }
            }
            UndoType::Insert => {
                if Rc::ptr_eq(&thisline, &filebot) {
                    u.borrow_mut().xflags |= INCLUDED_LAST_LINE;
                }
            }
            UndoType::CoupleBegin => {
                u.borrow_mut().tail_lineno = of_ref.cursor_row;
                /* Fall-through。 */
                u.borrow_mut().strdata = message.map(|m| m.to_string());
            }
            UndoType::CoupleEnd => {
                u.borrow_mut().strdata = message.map(|m| m.to_string());
            }
            UndoType::Indent | UndoType::Unindent | UndoType::Comment | UndoType::Uncomment => {}
            _ => {}
        }
        of_ref.last_action = thisaction;
    }
}

/// add_undo 中 BACK/DEL 共用的分支（删除字符或转为行连接）。
fn fallthrough_del(u: &UndoRef, thisline: &LineRef, action: &mut UndoType, current_x: usize) {
    let data = thisline.borrow().data.clone();
    let bytes = data.as_bytes();
    if bytes.get(current_x).copied().unwrap_or(0) != 0 {
        let charlen = chars::char_length(&bytes[current_x..]);
        let removed = String::from_utf8_lossy(&bytes[current_x..current_x + charlen]).into_owned();
        u.borrow_mut().strdata = Some(removed);
        if *action == UndoType::Back {
            let tail = u.borrow().tail_x;
            u.borrow_mut().tail_x = tail + charlen;
        }
    } else {
        *action = UndoType::Join;
        let next = { let r = thisline.borrow(); r.next.clone() };
        if let Some(n) = next {
            if *action == UndoType::Back {
                u.borrow_mut().head_lineno = n.borrow().lineno;
                u.borrow_mut().head_x = 0;
            }
            let ndata = n.borrow().data.clone();
            u.borrow_mut().strdata = Some(ndata);
        }
    }
}

/// 更新一个多行 undo 项。对多行改变功能的每一行调用一次；
/// 每行添加/移除的缩进在 undo 项中分别保存
/// （对应 `update_multiline_undo`）。
pub fn update_multiline_undo(lineno: isize, indentation: &str) {
    let of = openfile_ref();
    let u = {
        let of_ref = of.borrow();
        of_ref.current_undo.clone()
    };
    let Some(u) = u else { return };

    let grouping = u.borrow().grouping.clone();
    if let Some(gr) = &grouping {
        let contig = gr.borrow().bottom_line + 1 == lineno;
        if contig {
            let top_line = gr.borrow().top_line;
            let number_of_lines = (lineno - top_line + 1) as usize;
            gr.borrow_mut().bottom_line = lineno;
            let mut indents = gr.borrow_mut().indentations.clone();
            indents.resize(number_of_lines, None);
            indents[number_of_lines - 1] = Some(indentation.to_string());
            gr.borrow_mut().indentations = indents;
        } else {
            let born = Rc::new(RefCell::new(GroupStruct {
                top_line: lineno,
                bottom_line: lineno,
                indentations: vec![Some(indentation.to_string())],
                next: grouping,
            }));
            u.borrow_mut().grouping = Some(born);
        }
    } else {
        let born = Rc::new(RefCell::new(GroupStruct {
            top_line: lineno,
            bottom_line: lineno,
            indentations: vec![Some(indentation.to_string())],
            next: None,
        }));
        u.borrow_mut().grouping = Some(born);
    }

    /* 存储更改后的文件大小，供重做使用。 */
    let totsize = with_global(|g| g.openfile.as_ref().map(|o| o.borrow().totsize).unwrap_or(0));
    u.borrow_mut().newsize = totsize;
}

/// 用（除其他外）给定动作后的文件大小和光标位置更新 undo 项
/// （对应 `update_undo`）。
pub fn update_undo(_action: UndoType) {
    let of = openfile_ref();
    let u = {
        let of_ref = of.borrow();
        of_ref.undotop.clone()
    };
    let Some(u) = u else { return };

    let totsize = with_global(|g| g.openfile.as_ref().map(|o| o.borrow().totsize).unwrap_or(0));
    u.borrow_mut().newsize = totsize;

    let (current_x, current_lineno) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.current_x, of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(0))
    });

    let u_type = u.borrow().type_;
    match u_type {
        UndoType::Add => {
            let of = openfile_ref();
            let current = of.borrow().current.clone().unwrap();
            let data = current.borrow().data.clone();
            let head_x = u.borrow().head_x;
            let newlen = current_x - head_x;
            let strdata = String::from_utf8_lossy(&data.as_bytes()[head_x..head_x + newlen]).into_owned();
            u.borrow_mut().strdata = Some(strdata);
            u.borrow_mut().tail_x = current_x;
        }
        UndoType::Enter => {
            let of = openfile_ref();
            let current = of.borrow().current.clone().unwrap();
            let data = current.borrow().data.clone();
            u.borrow_mut().strdata = Some(data);
            u.borrow_mut().tail_x = current_x;
        }
        UndoType::Back | UndoType::Del => {
            let of = openfile_ref();
            let current = of.borrow().current.clone().unwrap();
            let data = current.borrow().data.clone();
            let bytes = data.as_bytes();
            let textposition = current_x;
            let charlen = chars::char_length(&bytes[textposition..]);
            let head_x = u.borrow().head_x;
            let datalen = u.borrow().strdata.clone().map(|s| s.len()).unwrap_or(0);
            if current_x == head_x {
                let removed = String::from_utf8_lossy(&bytes[textposition..textposition + charlen]).into_owned();
                let mut sd = u.borrow().strdata.clone().unwrap_or_default();
                sd.push_str(&removed);
                u.borrow_mut().strdata = Some(sd);
                u.borrow_mut().tail_x = current_x;
            } else if current_x == head_x - charlen {
                let removed = String::from_utf8_lossy(&bytes[textposition..textposition + charlen]).into_owned();
                let mut sd = u.borrow().strdata.clone().unwrap_or_default();
                sd.insert_str(0, &removed);
                u.borrow_mut().strdata = Some(sd);
                u.borrow_mut().head_x = current_x;
            } else {
                /* 他们在行的*其他*位置删除了：开始新的 undo 项。 */
                let _ = datalen;
                let u_type = u.borrow().type_;
                drop(u);
                add_undo(u_type, None);
            }
        }
        UndoType::Zap | UndoType::CutToEof | UndoType::Cut => {
            let u_type = u.borrow().type_;
            let xflags = u.borrow().xflags;
            if u_type == UndoType::Zap {
                let cb = with_global(|g| g.cutbuffer.clone());
                u.borrow_mut().cutbuffer = cb;
            } else {
                let cb = with_global(|g| g.cutbuffer.clone());
                match cb {
                    Some(c) => {
                        u.borrow_mut().cutbuffer = Some(files::copy_buffer(&c));
                    }
                    None => {}
                }
            }
            if xflags & MARK_WAS_SET == 0 {
                let cb = u.borrow().cutbuffer.clone();
                if let Some(cutbuffer) = cb {
                    let mut bottomline = cutbuffer.clone();
                    let mut count: isize = 0;
                    loop {
                        let next = { let r = bottomline.borrow(); r.next.clone() };
                        match next {
                            Some(n) => {
                                bottomline = n;
                                count += 1;
                            }
                            None => break,
                        }
                    }
                    let head_lineno = u.borrow().head_lineno;
                    u.borrow_mut().tail_lineno = head_lineno + count;
                    let cut_from_cursor = ISSET(CUT_FROM_CURSOR) || u_type == UndoType::CutToEof;
                    if cut_from_cursor {
                        let bl = bottomline.borrow().data.len();
                        u.borrow_mut().tail_x = bl;
                        if count == 0 {
                            let hx = u.borrow().head_x;
                            u.borrow_mut().tail_x = bl + hx;
                        }
                    } else {
                        let (is_filebot, no_newlines) = with_global(|g| {
                            let of = g.openfile.as_ref().unwrap().borrow();
                            let cur = of.current.clone().unwrap();
                            let is_fb = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, &cur)).unwrap_or(false);
                            (is_fb, ISSET(NO_NEWLINES))
                        });
                        if is_filebot && no_newlines {
                            let bl = bottomline.borrow().data.len();
                            u.borrow_mut().tail_x = bl;
                        }
                    }
                }
            }
        }
        UndoType::CoupleBegin => {}
        UndoType::CoupleEnd | UndoType::Paste | UndoType::Insert => {
            u.borrow_mut().tail_lineno = current_lineno;
            u.borrow_mut().tail_x = current_x;
        }
        _ => {}
    }
}

// ======================== 行断开（对应 do_enter） ========================

/// 在光标位置断开当前行（对应 `do_enter`）。
pub fn do_enter() {
    let of = openfile_ref();
    let (current, current_x, _autoindent) = {
        let of_ref = of.borrow();
        (
            of_ref.current.clone().unwrap(),
            of_ref.current_x,
            of_ref.last_action, // 占位（autoindent 由 ISSET 判断）
        )
    };

    let newnode = make_new_node(Some(&*current.borrow()));
    let mut extra = 0;
    let mut allblanks = false;

    let mut sampleline = current.clone();
    let autoindent = ISSET(AUTOINDENT);

    if autoindent {
        /* 自动长行换行且下一行在同一段落时，用它的缩进作样板。 */
        if ISSET(BREAK_LONG_LINES) {
            let next = { let r = sampleline.borrow(); r.next.clone() };
            if let Some(n) = next {
                if inpar(&n) && !begpar(&n, 0) {
                    sampleline = n;
                }
            }
        }
        extra = indent_length(sampleline.borrow().data.as_bytes());

        /* 在缩进处断开时，限制自动缩进。 */
        if extra > current_x {
            extra = current_x;
        } else if extra == current_x {
            allblanks = indent_length(current.borrow().data.as_bytes()) == extra;
        }
    }

    /* 新行包含光标之后的部分加上自动缩进。
     * 原版 C 用字节拼接；这里一次性 clone 当前行字节，供后续"构造新行"与
     * "截断当前行"两处复用，避免对 current.data 做两次 clone()+into_bytes()。 */
    let cur_bytes = current.borrow().data.clone().into_bytes();

    {
        let mut ndata = Vec::with_capacity(extra + (cur_bytes.len() - current_x));
        ndata.resize(extra, b' ');
        ndata.extend_from_slice(&cur_bytes[current_x..]);
        newnode.borrow_mut().data = String::from_utf8_lossy(&ndata).into_owned();
    }

    /* 若标记位于光标之后的当前行上，调整标记。 */
    {
        let mut of_ref = of.borrow_mut();
        if of_ref.mark.as_ref().map(|m| Rc::ptr_eq(m, &current)).unwrap_or(false)
            && of_ref.mark_x > current_x
        {
            of_ref.mark = Some(newnode.clone());
            of_ref.mark_x += extra - current_x;
        }

        if autoindent {
            /* 把样板行的空白复制到新行。原实现 clone 新行 data 再逐字节赋值；
             * 这里改为直接在 String 的字节切片上 copy，省一次 clone()+into_bytes()。 */
            let sdata = sampleline.borrow().data.clone().into_bytes();
            {
                let mut nd_ref = newnode.borrow_mut();
                let nd_bytes = unsafe { nd_ref.data.as_bytes_mut() };
                let copy_len = extra.min(sdata.len()).min(nd_bytes.len());
                nd_bytes[..copy_len].copy_from_slice(&sdata[..copy_len]);
                /* 不足部分保持原样（resize 的空格）。 */
            }

            /* 若光标前只有空白，修剪它们。 */
            if allblanks {
                of_ref.current_x = 0;
                if of_ref.mark.as_ref().map(|m| Rc::ptr_eq(m, &current)).unwrap_or(false) {
                    of_ref.mark_x = 0;
                }
            }
        }
    }

    /* 让当前行在光标处结束。复用已 clone 的 cur_bytes，避免再次 clone。 */
    {
        let mut cdata = cur_bytes;
        cdata.truncate(current_x);
        current.borrow_mut().data = String::from_utf8_lossy(&cdata).into_owned();
    }

    add_undo(UndoType::Enter, None);

    /* 在当前行之后插入新创建的行并重新编号。 */
    files::splice_node(&current, &newnode);
    files::renumber_from(&newnode);

    /* 把光标放到新行上，在自动空白之后。 */
    {
        let mut of_ref = of.borrow_mut();
        of_ref.current = Some(newnode.clone());
        of_ref.current_x = extra;
        /* xplustabs 内联：避免在持有 of 借用时访问 openfile。 */
        let cur = of_ref.current.clone().unwrap();
        of_ref.placewewant = utils::wideness(cur.borrow().data.as_bytes(), of_ref.current_x);
        of_ref.totsize += 1;
    }
    files::set_modified();

    if autoindent && !allblanks {
        let mut of_ref = of.borrow_mut();
        of_ref.totsize += extra;
    }
    update_undo(UndoType::Enter);

    with_global_mut(|g| {
        g.refresh_needed = true;
        g.focusing = false;
    });
}

// ======================== 自动换行（对应 do_wrap / break_line） ========================

/// 在给定文本中查找最后一个空白，使得到该点的显示宽度至多为
/// (goal + 1)。若无此类空白，则查找第一个空白。
/// 返回该组空白中最后一个空白的索引。snap_at_nl 为 TRUE 时换行也算空白
/// （对应 `break_line`）。
pub fn break_line(textstart: &[u8], goal: isize, snap_at_nl: bool) -> isize {
    let mut lastblank: Option<usize> = None;
    let mut pos = 0;
    let mut column = 0;

    /* 跳过开头的空白，行不应在此断开。 */
    while chars::byte_at(textstart, pos) != 0 && chars::is_blank_char(&textstart[pos..]) {
        pos += chars::advance_over(&textstart[pos..], &mut column);
    }

    /* 查找不超过目标列的最后空白。 */
    let inhelp = with_global(|g| g.inhelp);
    while chars::byte_at(textstart, pos) != 0 && (column as isize) <= goal {
        if chars::is_blank_char(&textstart[pos..]) && (!inhelp || column > 17 || goal < 40) {
            lastblank = Some(pos);
        } else if snap_at_nl && chars::byte_at(textstart, pos) == b'\n' {
            lastblank = Some(pos);
            break;
        }
        pos += chars::advance_over(&textstart[pos..], &mut column);
    }

    /* 若整行显示比 goal 短，完成。 */
    if (column as isize) <= goal {
        return pos as isize;
    }

    /* 处理帮助文本时未找到空白，强制换行。 */
    if snap_at_nl && lastblank.is_none() {
        return chars::step_left(textstart, pos) as isize;
    }

    /* 若在 goal 宽度内未找到空白，在其后寻找。 */
    let mut lastblank = match lastblank {
        Some(lb) => lb,
        None => {
            loop {
                if chars::byte_at(textstart, pos) == 0 {
                    /* 文本已结束仍无空白：返回行长度（对应 C 的 pos - textstart）。 */
                    return pos as isize;
                }
                if chars::is_blank_char(&textstart[pos..]) {
                    break;
                }
                pos += chars::char_length(&textstart[pos..]);
            }
            pos
        }
    };

    pos = lastblank + chars::char_length(&textstart[lastblank..]);

    /* 跳过最后空白之后的连续空白。 */
    while chars::byte_at(textstart, pos) != 0 && chars::is_blank_char(&textstart[pos..]) {
        lastblank = pos;
        pos += chars::char_length(&textstart[pos..]);
    }

    lastblank as isize
}

/// 当当前行过长时，在可能的最远空白处硬换行，
/// 并把多余部分前置到"溢出"行（对应 `do_wrap`）。
pub fn do_wrap() {
    let of = openfile_ref();
    let (line, current_x, wrap_at) = {
        let of_ref = of.borrow();
        (of_ref.current.clone().unwrap(), of_ref.current_x, with_global(|g| g.wrap_at))
    };

    let line_data = line.borrow().data.clone();
    let line_len = line_data.len();
    let quot_len = quote_length(&line_data);
    let lead_len = quot_len + indent_length(&line_data.as_bytes()[quot_len..]);
    let cursor_x = current_x;

    /* 先找到可以断行的最后一个空白字符。 */
    let wrap_loc = break_line(
        &line_data.as_bytes()[lead_len..],
        wrap_at as isize - utils::wideness(line_data.as_bytes(), lead_len) as isize,
        false,
    );

    /* 若在行尾前未找到换行点，不换行。 */
    if wrap_loc < 0 || lead_len + wrap_loc as usize == line_len {
        return;
    }

    /* 把换行位置调整到整行中的位置，并前进到空白之后。 */
    let wrap_loc = lead_len + chars::step_right(&line_data.as_bytes()[lead_len..], wrap_loc as usize);

    /* 现在位于行尾时，无需换行。 */
    if line_data.as_bytes().get(wrap_loc).copied().unwrap_or(0) == 0 {
        return;
    }

    add_undo(UndoType::SplitBegin, None);

    let autowhite = ISSET(AUTOINDENT);
    if quot_len > 0 {
        UNSET(AUTOINDENT);
    }

    /* 其余部分是换行到下一行的文本。 */
    let rest_length = line_len - wrap_loc;

    /* 前置时，若本行的剩余部分不会使下一行过长，则连接两行。 */
    let spillage_is_next = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.spillage_line.as_ref().map(|s| {
                line.borrow().next.as_ref().map(|n| Rc::ptr_eq(s, n)).unwrap_or(false)
            }).unwrap_or(false)
        }).unwrap_or(false)
    });

    let next_data = {
        let next = line.borrow().next.clone();
        next.map(|n| n.borrow().data.clone())
    };

    let can_join = spillage_is_next
        && rest_length + next_data.as_ref().map(|d| utils::breadth(d.as_bytes())).unwrap_or(0) <= wrap_at;

    if can_join {
        /* 转到本行末尾。 */
        let mut of_ref = of.borrow_mut();
        of_ref.current_x = line_len;
        drop(of_ref);

        /* 若剩余部分不以空白结尾，添加一个空格。 */
        let remainder = &line_data.as_bytes()[wrap_loc..];
        let last_rem = chars::step_left(remainder, rest_length);
        if !chars::is_blank_char(&remainder[last_rem..]) {
            add_undo(UndoType::Add, None);
            let mut data = line.borrow().data.clone();
            data.push(' ');
            line.borrow_mut().data = data;
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    let mut of = of.borrow_mut();
                    of.totsize += 1;
                    of.current_x += 1;
                }
            });
            update_undo(UndoType::Add);
        }

        /* 把下一行连接到本行。 */
        cut::expunge(UndoType::Del);

        /* 若本行的前置部分等于原下一行的前置部分，则剥除第二个。 */
        let of_ref = of.borrow_mut();
        let cur_x = of_ref.current_x;
        let lead_ok = line.borrow().data.as_bytes().get(..lead_len).unwrap_or(&[])
            == line.borrow().data.as_bytes().get(cur_x..cur_x + lead_len).unwrap_or(&[]);
        drop(of_ref);
        if lead_ok {
            for _ in 0..lead_len {
                cut::expunge(UndoType::Del);
            }
        }

        /* 移除多余空白。 */
        loop {
            let is_blank = with_global(|g| {
                g.openfile.as_ref().map(|of| {
                    let of = of.borrow();
                    let d = of.current.as_ref().unwrap().borrow().data.clone();
                    let x = of.current_x;
                    chars::is_blank_char(&d.as_bytes()[x..])
                }).unwrap_or(false)
            });
            if !is_blank {
                break;
            }
            cut::expunge(UndoType::Del);
        }
    }

    /* 转到换行位置。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().current_x = wrap_loc;
        }
    });

    /* 请求时，剪掉换行行尾的空白。 */
    if ISSET(TRIM_BLANKS) {
        let (rear_x0, typed_x0, wrap_loc0) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            let d = of.current.as_ref().unwrap().borrow().data.clone();
            (
                chars::step_left(d.as_bytes(), wrap_loc),
                chars::step_left(d.as_bytes(), cursor_x),
                wrap_loc,
            )
        });
        let mut rear_x = rear_x0;
        let typed_x = typed_x0;
        loop {
            let is_blank = {
                let of = openfile_ref();
                let d = of.borrow().current.as_ref().unwrap().borrow().data.clone();
                chars::is_blank_char(&d.as_bytes()[rear_x..])
            };
            let cond = (rear_x != typed_x || cursor_x >= wrap_loc0) && is_blank;
            if !cond {
                break;
            }
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    of.borrow_mut().current_x = rear_x;
                }
            });
            cut::expunge(UndoType::Del);
            let of = openfile_ref();
            let d = of.borrow().current.as_ref().unwrap().borrow().data.clone();
            rear_x = chars::step_left(d.as_bytes(), rear_x);
        }
    }

    /* 现在断开该行。 */
    do_enter();

    /* 换行部分可见的行时，调整屏幕起点。 */
    let (edittop_is_line, firstcolumn_gt_0, cursor_ge_wrap) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let is_edittop = of.edittop.as_ref().map(|e| Rc::ptr_eq(e, &line)).unwrap_or(false);
        (is_edittop, of.firstcolumn > 0, cursor_x >= wrap_loc)
    });
    if edittop_is_line && firstcolumn_gt_0 && cursor_ge_wrap {
        let (et0, fc0) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.edittop.clone().unwrap(), of.firstcolumn)
        });
        let mut et = et0;
        let mut fc = fc0;
        winio::go_forward_chunks(1, &mut et, &mut fc);
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut r = of.borrow_mut();
                r.edittop = Some(et);
                r.firstcolumn = fc;
            }
        });
    }

    /* 若原行有引用，把它复制到溢出行。 */
    if quot_len > 0 {
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                let line = of.current.clone().unwrap();
                let prev = line.borrow().prev.clone();
                let prev_data = prev.as_ref().and_then(|w| w.upgrade()).map(|p| p.borrow().data.clone()).unwrap_or_default();
                let lead = prev_data.as_bytes().get(..lead_len).unwrap_or(&[]).to_vec();

                let nd = line.borrow().data.clone().into_bytes();
                let mut combined = lead.clone();
                combined.extend_from_slice(&nd);
                line.borrow_mut().data = String::from_utf8_lossy(&combined).into_owned();

                of.current_x += lead_len;
                of.totsize += lead_len;
            }
        });

        /* 用新的引号长度更新 undo 项。 */
        let of = openfile_ref();
        if let Some(u) = of.borrow().undotop.clone() {
            u.borrow_mut().strdata = None;
        }
        update_undo(UndoType::Enter);

        if autowhite {
            SET(AUTOINDENT);
        }
    }

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.spillage_line = of.current.clone();
        }
    });

    if cursor_x < wrap_loc {
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                let cur = of.current.clone().unwrap();
                let prev = cur.borrow().prev.clone().and_then(|w| w.upgrade());
                if let Some(p) = prev {
                    of.current = Some(p);
                }
                of.current_x = cursor_x;
            }
        });
    } else {
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.current_x += cursor_x - wrap_loc;
            }
        });
    }

    let pww = utils::xplustabs();
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().placewewant = pww;
        }
    });

    add_undo(UndoType::SplitEnd, None);
    with_global_mut(|g| g.refresh_needed = true);
}

// ======================== 引用与段落（对应 quote_length / begpar / inpar） ========================

/// 返回给定行中引用部分的长度（对应 `quote_length`）。
pub fn quote_length(line: &str) -> usize {
    let quotereg = with_global(|g| g.quotereg.clone());
    let Some(qr) = quotereg else { return 0 };
    match qr.find_match(line) {
        Some((so, eo)) => {
            if so == 0 {
                eo
            } else {
                0
            }
        }
        None => 0,
    }
}

/// 返回 TRUE 当给定行是段落开头（BOP）（对应 `begpar`）。
pub fn begpar(line: &LineRef, depth: usize) -> bool {
    const RECURSION_LIMIT: usize = 222;

    /* 第一行即使没有文本也算 BOP。 */
    let prev = { let r = line.borrow(); r.prev.clone() };
    if prev.is_none() {
        return true;
    }
    let Some(prev) = prev.and_then(|w| w.upgrade()) else { return true };

    /* 递归太深时，直接说不是 BOP。 */
    if depth > RECURSION_LIMIT {
        return false;
    }

    let data = line.borrow().data.clone();
    let quot_len = quote_length(&data);
    let indent_len = indent_length(&data.as_bytes()[quot_len..]);

    /* 若该行没有文本，则不是 BOP。 */
    if data.as_bytes().get(quot_len + indent_len).copied().unwrap_or(0) == 0 {
        return false;
    }

    /* 请求时，把以空白开头的行当作 BOP。 */
    if ISSET(BOOKSTYLE) && !ISSET(AUTOINDENT) && chars::is_blank_char(data.as_bytes()) {
        return true;
    }

    /* 若前一行的引用部分不同，则是 BOP。 */
    let prev_data = prev.borrow().data.clone();
    let prev_quot_len = quote_length(&prev_data);
    let same_quote = quot_len == prev_quot_len
        && data.as_bytes().get(..quot_len).unwrap_or(&[]) == prev_data.as_bytes().get(..quot_len).unwrap_or(&[]);
    if !same_quote {
        return true;
    }

    let prev_dent_len = indent_length(&prev_data.as_bytes()[prev_quot_len..]);

    /* 若前一行没有文本，则是 BOP。 */
    if prev_data.as_bytes().get(prev_quot_len + prev_dent_len).copied().unwrap_or(0) == 0 {
        return true;
    }

    /* 若本行与前一行的缩进相同，则不是 BOP。 */
    if utils::wideness(prev_data.as_bytes(), prev_quot_len + prev_dent_len)
        == utils::wideness(data.as_bytes(), quot_len + indent_len)
    {
        return false;
    }

    /* 否则，前一行不是 BOP 时这才是 BOP。 */
    !begpar(&prev, depth + 1)
}

/// 返回 TRUE 当给定行是段落的一部分：含有引用与开头的空白之外的内容
/// （对应 `inpar`）。
pub fn inpar(line: &LineRef) -> bool {
    let data = line.borrow().data.clone();
    let quot_len = quote_length(&data);
    let indent_len = indent_length(&data.as_bytes()[quot_len..]);
    data.as_bytes().get(quot_len + indent_len).copied().unwrap_or(0) != 0
}

// ======================== 段落对齐（对应 justify_paragraph / justify_text） ========================

/// 从给定行开始查找下一个段落，返回其首行与行数；找不到返回 None
/// （对应 `find_paragraph`）。
fn find_paragraph(firstline: &LineRef) -> Option<(LineRef, usize)> {
    let mut line = firstline.clone();

    /* 不在段落中时，前进到段落的行。 */
    while !inpar(&line) {
        let next = { let r = line.borrow(); r.next.clone() };
        match next {
            Some(n) => line = n,
            None => break,
        }
    }

    /* 前进到段落的最后一行。 */
    let mut last = line.clone();
    movement_para_end(&mut last);

    /* 仍不在段落中时，没有剩余段落。 */
    if !inpar(&last) {
        return None;
    }

    let count = {
        let last_lineno = last.borrow().lineno;
        let first_lineno = line.borrow().lineno;
        (last_lineno - first_lineno + 1) as usize
    };
    Some((line, count))
}

/// 前进到段落末尾（对应 C 的 do_para_end；供对齐使用）。
fn movement_para_end(line: &mut LineRef) {
    loop {
        let next = { let r = line.borrow(); r.next.clone() };
        match next {
            Some(n) => {
                if begpar(&n, 0) {
                    break;
                }
                *line = n;
            }
            None => break,
        }
    }
}

/// 把以 *line 开头、共 count 行的段落拼接为一行，跳过其后各行的
/// 引用与缩进（对应 `concat_paragraph`）。
fn concat_paragraph(line: &LineRef, count: usize) {
    let mut count = count;
    while count > 1 {
        let next_line = { let r = line.borrow(); r.next.clone() }.unwrap();
        let next_data = next_line.borrow().data.clone();
        let next_quot_len = quote_length(&next_data);
        let next_lead_len =
            next_quot_len + indent_length(&next_data.as_bytes()[next_quot_len..]);
        let mut line_data = line.borrow().data.clone();

        /* 把下一行接到本行后：本行非空且不以空格结尾时补一个空格。 */
        if !line_data.is_empty() && !line_data.ends_with(' ') {
            line_data.push(' ');
        }
        line_data.push_str(&next_data[next_lead_len..]);

        /* 合并锚点。 */
        let anchor = next_line.borrow().has_anchor;
        if anchor {
            line.borrow_mut().has_anchor = true;
        }

        line.borrow_mut().data = line_data;
        files::unlink_node(&next_line);
        files::renumber_from(line);
        count -= 1;
    }
}

/// 在给定行中把连续空白替换为单个空格，但任何结尾标点后保留两个空格
/// （若有两个），并去掉行尾全部空白。前 skip 个字符不处理
/// （对应 `squeeze`）。
fn squeeze(line: &LineRef, skip: usize) {
    let data = line.borrow().data.clone().into_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..skip.min(data.len())]);

    let mut i = skip.min(data.len());
    let n = data.len();
    while i < n {
        if chars::is_blank_char(&data[i..]) {
            /* 空白 → 单个空格。 */
            out.push(b' ');
            i += chars::char_length(&data[i..]);
            while i < n && chars::is_blank_char(&data[i..]) {
                i += chars::char_length(&data[i..]);
            }
        } else if is_punctuation(&data[i..]) {
            /* 标点：复制它及可能的后续括号，最多两个后续空白变空格。 */
            let clen = chars::char_length(&data[i..]);
            out.extend_from_slice(&data[i..i + clen]);
            i += clen;
            if i < n && is_bracket(&data[i..]) {
                let blen = chars::char_length(&data[i..]);
                out.extend_from_slice(&data[i..i + blen]);
                i += blen;
            }
            let mut spaces = 0;
            while i < n && chars::is_blank_char(&data[i..]) && spaces < 2 {
                out.push(b' ');
                i += chars::char_length(&data[i..]);
                spaces += 1;
            }
            while i < n && chars::is_blank_char(&data[i..]) {
                i += chars::char_length(&data[i..]);
            }
        } else {
            let clen = chars::char_length(&data[i..]);
            out.extend_from_slice(&data[i..i + clen]);
            i += clen;
        }
    }

    /* 去掉行尾空格。 */
    while out.len() > skip && out.last() == Some(&b' ') {
        out.pop();
    }

    line.borrow_mut().data = String::from_utf8_lossy(&out).into_owned();
}

/// 是否为标点字符（对应 C 的 punct 集合）。
fn is_punctuation(data: &[u8]) -> bool {
    let c = data.first().copied().unwrap_or(0);
    matches!(
        c,
        b'.' | b',' | b';' | b':' | b'!' | b'?' | b'\'' | b'"' | b')'
    )
}

/// 是否为括号字符（对应 C 的 brackets 集合）。
fn is_bracket(data: &[u8]) -> bool {
    let c = data.first().copied().unwrap_or(0);
    matches!(c, b')' | b']' | b'}' | b'>')
}

/// 把给定行（以 lead_string/lead_len 开头）重排为不超过 wrap_at 宽度的
/// 多行（对应 `rewrap_paragraph`）。
fn rewrap_paragraph(line: &mut LineRef, lead_string: &str, lead_len: usize) {
    let wrap_at = with_global(|g| g.wrap_at);

    while utils::breadth(line.borrow().data.as_bytes()) > wrap_at {
        let line_data = line.borrow().data.clone();
        let line_len = line_data.len();

        /* 在行中找可断点。 */
        let break_pos = break_line(
            line_data.as_bytes(),
            wrap_at as isize - utils::wideness(line_data.as_bytes(), lead_len) as isize,
            false,
        );

        /* 无法断开或不需要断开时结束。 */
        if break_pos < 0 || lead_len + break_pos as usize == line_len {
            break;
        }

        let mut break_pos = lead_len + break_pos as usize + 1;

        /* 在当前行后插入新行，把前导部分与断点后的文本复制进去。 */
        let newnode = make_new_node(Some(&*line.borrow()));
        let mut ndata = String::new();
        ndata.push_str(lead_string);
        ndata.push_str(&line_data[break_pos..]);
        newnode.borrow_mut().data = ndata;
        files::splice_node(line, &newnode);

        /* 请求时剪掉一或两个尾随空格。 */
        if ISSET(TRIM_BLANKS) {
            while break_pos > 0 && line.borrow().data.as_bytes().get(break_pos - 1) == Some(&b' ') {
                break_pos -= 1;
            }
        }

        /* 实际断开当前行并移到下一行。 */
        {
            let mut d = line.borrow().data.clone().into_bytes();
            d.truncate(break_pos);
            line.borrow_mut().data = String::from_utf8_lossy(&d).into_owned();
        }
        *line = newnode;
    }

    files::renumber_from(line);

    /* 可能时，移到重排段落之后的行。 */
    let next = { let r = line.borrow(); r.next.clone() };
    if let Some(n) = next {
        *line = n;
    }
}

/// 对齐以 *line 开头、共 count 行的段落，使各行适配 wrap_at 宽度并
/// 规范化空白（对应 `justify_paragraph`）。
fn justify_paragraph(line: &mut LineRef, count: usize) {
    /* 样板行是唯一一行或第二行。 */
    let sampleline = if count == 1 {
        line.clone()
    } else {
        { let r = line.borrow(); r.next.clone() }.unwrap()
    };

    /* 复制样板行的前导部分（引用 + 缩进）。 */
    let sample_data = sampleline.borrow().data.clone();
    let quot_len = quote_length(&sample_data);
    let lead_len = quot_len + indent_length(&sample_data.as_bytes()[quot_len..]);
    let lead_string: String = sample_data.chars().take(lead_len).collect();

    /* 把段落所有行拼接为一行。 */
    concat_paragraph(line, count);

    /* 规范化空白。 */
    let line_data = line.borrow().data.clone();
    let lq = quote_length(&line_data);
    let li = indent_length(&line_data.as_bytes()[lq..]);
    squeeze(line, lq + li);

    /* 按前导部分重排。 */
    rewrap_paragraph(line, &lead_string, lead_len);
}

/// 对齐当前段落（对应 `do_justify`）。
pub fn do_justify() {
    justify_text(false);
}

/// 对齐整个文件（对应 `do_full_justify`）。
pub fn do_full_justify() {
    justify_text(true);
    with_global_mut(|g| {
        g.ran_a_tool = true;
        g.recook = true;
    });
}

/// 对齐当前段落（whole_buffer=FALSE）或整个缓冲区（whole_buffer=TRUE）；
/// 标记存在时只对齐标记区域（对应 `justify_text`）。
pub fn justify_text(whole_buffer: bool) {
    let was_cutbuffer = with_global(|g| g.cutbuffer.clone());
    let was_the_linenumber = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let r = of.borrow();
            r.current.as_ref().map(|c| c.borrow().lineno)
        }).unwrap_or(1)
    });

    add_undo(UndoType::CoupleBegin, Some("justification"));

    let has_mark = with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false)
    });

    let (startline, start_x, endline, end_x) = if has_mark {
        /* 标记区域当作一个段落。 */
        let (s, sx, e, ex) = utils::get_region();
        /* 空区域不做任何事。 */
        if std::rc::Rc::ptr_eq(&s, &e) && sx == ex {
            winio::statusline(MessageType::Ahem, &crate::t!("text-selection_is_empty"));
            discard_until(with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.undotop.clone()
                })
            }).as_ref().and_then(|u| u.borrow().next.clone()).as_ref());
            return;
        }
        (s, sx, e, ex)
    } else {
        /* 对齐整个缓冲区时从顶部开始；否则在段落中时回到段落开头。 */
        let mut current = with_global(|g| {
            g.openfile.as_ref().and_then(|of| {
                let r = of.borrow();
                r.current.clone()
            }).unwrap()
        });
        if whole_buffer {
            let top = with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.filetop.clone()
                }).unwrap()
            });
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    of.borrow_mut().current = Some(top.clone());
                }
            });
            current = top;
        } else if inpar(&current) && !begpar(&current, 0) {
            crate::movement::do_para_begin(&mut current);
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    of.borrow_mut().current = Some(current.clone());
                }
            });
        }

        /* 找第一个要对齐的段落。 */
        let Some((first, linecount)) = find_paragraph(&current) else {
            /* 没有可对齐的段落：光标移到文件末尾。 */
            let bot = with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.filebot.clone()
                }).unwrap()
            });
            let bot_len = bot.borrow().data.len();
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    let mut of = of.borrow_mut();
                    of.current = Some(bot);
                    of.current_x = bot_len;
                }
            });
            discard_until(with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.undotop.clone()
                })
            }).as_ref().and_then(|u| u.borrow().next.clone()).as_ref());
            with_global_mut(|g| g.refresh_needed = true);
            return;
        };
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.current = Some(first.clone());
                of.current_x = 0;
            }
        });

        let start = first.clone();
        let start_x = 0;

        /* 段落末尾。 */
        let (end, end_x) = if whole_buffer {
            let bot = with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.filebot.clone()
                }).unwrap()
            });
            let bx = bot.borrow().data.len();
            (bot, bx)
        } else {
            let mut e = start.clone();
            for _ in 1..linecount {
                let next = { let r = e.borrow(); r.next.clone() }.unwrap();
                e = next;
            }
            match { let r = e.borrow(); r.next.clone() } {
                Some(n) => (n, 0),
                None => {
                    let len = e.borrow().data.len();
                    (e, len)
                }
            }
        };
        (start, start_x, end, end_x)
    };

    /* 剪出段落区域。 */
    add_undo(UndoType::Cut, None);
    with_global_mut(|g| {
        g.cutbuffer = None;
        g.cutbottom = None;
    });
    cut::extract_segment(&startline, start_x, &endline, end_x);
    update_undo(UndoType::Cut);

    /* 对齐剪出的文本。 */
    let cutbuffer = with_global(|g| g.cutbuffer.clone()).unwrap();
    let mut jusline = cutbuffer.clone();
    let count = {
        let start_lineno = startline.borrow().lineno;
        let end_lineno = endline.borrow().lineno;
        (end_lineno - start_lineno) as usize + if end_x > 0 { 1 } else { 0 }
    };
    justify_paragraph(&mut jusline, count.max(1));

    if whole_buffer && !has_mark {
        /* 对齐整个文件：继续对齐后续段落。 */
        let mut first = jusline.clone();
        loop {
            let next = { let r = first.borrow(); r.next.clone() };
            match next {
                Some(n) => {
                    if let Some((f, lc)) = find_paragraph(&n) {
                        first = f.clone();
                        let mut jl = f;
                        justify_paragraph(&mut jl, lc);
                        if { let r = jl.borrow(); r.next.is_none() } {
                            break;
                        }
                        continue;
                    }
                    break;
                }
                None => break,
            }
        }
    }

    /* 把对齐后的文本嫁回缓冲区。 */
    add_undo(UndoType::Paste, None);
    cut::ingraft_buffer(&cutbuffer);
    update_undo(UndoType::Paste);

    /* 整段对齐后回到原来的行。 */
    if whole_buffer && !has_mark {
        crate::search::goto_line_posx(was_the_linenumber as isize, 0);
    }

    add_undo(UndoType::CoupleEnd, Some("justification"));

    /* 报告对齐结果。 */
    let msg = if has_mark {
        crate::t!("text-justified_selection")
    } else if whole_buffer {
        crate::t!("text-justified_file")
    } else {
        crate::t!("text-justified_paragraph")
    };
    winio::statusline(MessageType::Remark, &msg);

    /* 恢复剪贴板（注意：free_lines 内部会访问全局，须在闭包外调用）。 */
    let old_cb = with_global(|g| g.cutbuffer.clone());
    files::free_lines(old_cb);
    with_global_mut(|g| g.cutbuffer = was_cutbuffer);

    files::set_modified();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.focusing = false;
    });
}

// ======================== 单词补全（对应 complete_a_word） ========================

/// 返回给定位置开始的补全候选词（复制到下一个非单词字符前）
/// （对应 `copy_completion`）。
fn copy_completion(text: &[u8]) -> String {
    let mut length = 0;
    while length < text.len() && chars::is_word_char(&text[length..], false) {
        let step = chars::step_right(text, length).min(text.len());
        if step <= length {
            break;
        }
        length = step;
    }
    String::from_utf8_lossy(&text[..length.min(text.len())]).into_owned()
}

/// 查看用户输入的单词片段，然后搜索当前缓冲区中以此片段开头的单词，
/// 并暂时代补全该片段。再次按补全键则撤销上次补全并搜索下一个候选
/// （对应 `complete_a_word`）。
pub fn complete_a_word() {
    let was_set_wrapping = ISSET(BREAK_LONG_LINES);

    /* 全新补全尝试。 */
    let fresh = with_global(|g| g.pletion_line.is_none());
    if fresh {
        /* 清除上次补全运行的候选列表。 */
        with_global_mut(|g| {
            let mut cur = g.completion_list.take();
            while let Some(c) = cur {
                let next = { let r = c.borrow(); r.next.clone() };
                cur = next;
            }
            /* 防止补全被并入已输入的文本。 */
            if let Some(of) = &g.openfile {
                of.borrow_mut().last_action = UndoType::Other;
            }
            /* 从缓冲区顶部开始搜索。 */
            g.pletion_line = g.openfile.as_ref().and_then(|of| {
                let r = of.borrow();
                r.filetop.clone()
            });
            g.pletion_x = 0;
        });
        winio::wipe_statusbar();
    } else {
        /* 撤销上次尝试的补全。 */
        do_undo();
    }

    /* 找到用户输入的片段起点。 */
    let (mut start_of_shard, current_x, current_data) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let cur = of.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        (of.current_x, of.current_x, data)
    });
    while start_of_shard > 0 {
        let oneleft = chars::step_left(current_data.as_bytes(), start_of_shard);
        if !chars::is_word_char(&current_data.as_bytes()[oneleft..], false) {
            break;
        }
        start_of_shard = oneleft;
    }

    /* 光标前没有单词片段时不做任何事。 */
    if start_of_shard == current_x {
        winio::statusline(MessageType::Ahem, &crate::t!("text-no_word_fragment"));
        with_global_mut(|g| g.pletion_line = None);
        return;
    }

    /* 复制要搜索的片段。 */
    let shard: String = current_data[start_of_shard..current_x].to_string();
    let shard_length = shard.len();

    /* 搜索整个缓冲区，寻找 shard。 */
    loop {
        let (pletion_line, pletion_x) = with_global(|g| {
            (g.pletion_line.clone(), g.pletion_x)
        });
        let Some(pletion_line) = pletion_line else { break };

        let pletion_data = pletion_line.borrow().data.clone();
        let threshold = pletion_data.len().saturating_sub(shard_length);

        let mut i = pletion_x;
        while i < threshold {
            /* 首字节不匹配则继续。 */
            if pletion_data.as_bytes()[i] != shard.as_bytes()[0] {
                i += 1;
                continue;
            }

            /* 比较 shard 的其余字节。 */
            let mut j = 1;
            while j < shard_length
                && pletion_data.as_bytes().get(i + j) == shard.as_bytes().get(j)
            {
                j += 1;
            }

            /* 未完全匹配则继续搜索。 */
            if j < shard_length {
                i += 1;
                continue;
            }

            /* 匹配不比 shard 长时跳过。 */
            if !chars::is_word_char(&pletion_data.as_bytes()[i + j..], false) {
                i += 1;
                continue;
            }

            /* 匹配不是独立单词时跳过。 */
            if i > 0 && chars::is_word_char(
                &pletion_data.as_bytes()[chars::step_left(pletion_data.as_bytes(), i)..],
                false,
            ) {
                i += 1;
                continue;
            }

            /* 该匹配就是 shard 本身时忽略。 */
            if with_global(|g| {
                g.openfile.as_ref().map(|of| {
                    let r = of.borrow();
                    r.current.as_ref().map(|c| {
                        Rc::ptr_eq(&pletion_line, c) && i == r.current_x - shard_length
                    }).unwrap_or(false)
                }).unwrap_or(false)
            }) {
                i += 1;
                continue;
            }

            let completion = copy_completion(&pletion_data.as_bytes()[i..]);

            /* 在之前的候选列表中查找重复。 */
            let dup = with_global(|g| {
                let mut some_word = g.completion_list.clone();
                let mut is_dup = false;
                while let Some(w) = some_word {
                    if w.borrow().word.as_deref() == Some(completion.as_str()) {
                        is_dup = true;
                        break;
                    }
                    let next = { let r = w.borrow(); r.next.clone() };
                    some_word = next;
                }
                is_dup
            });

            /* 已尝试过这个词则跳过。 */
            if dup {
                i += 1;
                continue;
            }

            /* 把找到的词加入候选列表。 */
            with_global_mut(|g| {
                let node = Rc::new(RefCell::new(CompletionStruct {
                    word: Some(completion.clone()),
                    next: g.completion_list.clone(),
                }));
                g.completion_list = Some(node);
            });

            /* 临时禁用换行，使只添加一个撤销项。 */
            if was_set_wrapping {
                UNSET(BREAK_LONG_LINES);
            }
            /* 把补全注入缓冲区。 */
            let extra = &completion.as_bytes()[shard_length..];
            inject(extra, extra.len());
            /* 需要时重新启用换行并换行。 */
            if was_set_wrapping {
                SET(BREAK_LONG_LINES);
                do_wrap();
            }

            /* 为下次搜索设置位置。 */
            with_global_mut(|g| g.pletion_x = i + 1);
            return;
        }

        /* 移到下一行。 */
        with_global_mut(|g| {
            let next = { let r = pletion_line.borrow(); r.next.clone() };
            g.pletion_line = next;
            g.pletion_x = 0;
        });
    }

    /* 搜索遍历了所有行。 */
    let tried = with_global(|g| g.completion_list.is_some());
    if tried {
        winio::edit_refresh();
        winio::statusline(MessageType::Ahem, &crate::t!("text-no_further_matches"));
    } else {
        winio::statusline(MessageType::Ahem, &crate::t!("text-no_matches"));
    }
}

// ======================== 界面辅助（保留接口） ========================

/// 确保 firstcolumn 对齐（对应 winio.c；由渲染层处理）。
pub fn ensure_firstcolumn_is_aligned() {
    // 渲染层负责
}

/// 在当前位置设置锚点（对应 `do_anchor`）。
pub fn do_anchor() {
    let of = openfile_ref();
    let of_ref = of.borrow_mut();
    if let Some(cur) = &of_ref.current {
        let mut data = cur.borrow_mut();
        data.has_anchor = !data.has_anchor;
    }
}

/// 清除所有剪贴板内容（对应 C 版在剪切操作前 `free_lines(cutbuffer);
/// cutbuffer = NULL` 的集中清理）。
pub fn zap_all_cutbuffer() {
    /* free_lines 内部会访问全局，须在闭包外调用。 */
    let old_cb = with_global(|g| g.cutbuffer.clone());
    files::free_lines(old_cb);
    with_global_mut(|g| {
        g.cutbuffer = None;
        g.cutbottom = None;
    });
}

/// 拼写检查：把（标记区域或整个）缓冲区写入临时文件，调用拼写器处理，
/// 再把结果读回替换原文本（对应 text.c 的 `do_spell`）。
pub fn do_spell() {
    if files::in_restricted_mode() {
        return;
    }
    with_global_mut(|g| g.ran_a_tool = true);

    /* 拼写器：-s/--speller 指定的值，其次环境变量 SPELL，最后系统 spell。 */
    let speller = with_global(|g| g.speller.clone())
        .or_else(|| std::env::var("SPELL").ok())
        .unwrap_or_else(|| "spell".to_string());

    let temp_name = safe_tempfile_name();
    let Some(temp_name) = temp_name else {
        return;
    };

    let okay = write_buffer_to_file(&temp_name);
    if !okay {
        winio::statusline(
            MessageType::Alert,
            &crate::t!("files-error_writing_temp", err = "write failed"),
        );
        let _ = std::fs::remove_file(&temp_name);
        return;
    }

    treat(&temp_name, &speller, true);

    let _ = std::fs::remove_file(&temp_name);
}

/// 格式化：运行当前语法定义的 formatter 命令处理整个缓冲区，
/// 并把输出读回替换（对应 text.c 的 `do_formatter`）。
pub fn do_formatter() {
    if files::in_restricted_mode() {
        return;
    }
    with_global_mut(|g| g.ran_a_tool = true);

    let formatter = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let r = of.borrow();
            r.syntax.as_ref().and_then(|s| s.borrow().formatter.clone())
        })
    });

    let Some(formatter) = formatter else {
        winio::statusline(MessageType::Ahem, &crate::t!("files-no_formatter_defined"));
        return;
    };

    /* 格式化整个缓冲区，清除标记。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().mark = None;
        }
    });

    let temp_name = safe_tempfile_name();
    let Some(temp_name) = temp_name else {
        return;
    };

    let okay = write_buffer_to_file(&temp_name);
    if !okay {
        winio::statusline(
            MessageType::Alert,
            &crate::t!("files-error_writing_temp", err = "write failed"),
        );
        let _ = std::fs::remove_file(&temp_name);
        return;
    }

    treat(&temp_name, &formatter, false);

    let _ = std::fs::remove_file(&temp_name);
}

/// 获取逐字输入并插入缓冲区（对应 text.c 的 `do_verbatim_input`）。
pub fn do_verbatim_input() {
    /* 无状态栏且光标在底行时，先滚动一行腾出反馈空间。 */
    if ISSET(ZERO) {
        let (cursor_row, editwinrows) = with_global(|g| {
            let row = g.openfile.as_ref().map(|of| of.borrow().cursor_row).unwrap_or(0);
            (row, g.editwinrows)
        });
        if cursor_row == (editwinrows - 1) as isize && with_global(|g| g.LINES) > 1 {
            winio::edit_scroll(winio::ScrollDirection::Forward);
            winio::edit_refresh();
        }
    }

    winio::statusline(MessageType::Info, &crate::t!("text-verbatim_input"));
    winio::place_the_cursor();

    let mut count = 0usize;
    let bytes = winio::get_verbatim_kbinput(&mut count);

    /* 获得有效输入时，插入缓冲区并清空状态栏。 */
    if count > 0 {
        if ISSET(CONSTANT_SHOW) || ISSET(MINIBAR) {
            with_global_mut(|g| g.lastmessage = MessageType::Vacuum);
        }
        inject(&bytes, count);
        winio::wipe_statusbar();
    } else {
        winio::statusline(MessageType::Ahem, &crate::t!("text-invalid_code"));
    }
}

// ======================== 内部辅助 ========================

/// 创建唯一的临时文件名（对应 C 版的 `safe_tempfile`，但只返回文件名；
/// 文件由调用者用 `write_buffer_to_file` 写入）。
fn safe_tempfile_name() -> Option<String> {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    for attempt in 0..100u32 {
        let name = dir.join(format!("nano.{}.{}.tmp", pid, attempt));
        let path = name.to_string_lossy().into_owned();
        /* 用 create_new 确保唯一性。 */
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Some(path),
            Err(_) => continue,
        }
    }
    winio::statusline(
        MessageType::Alert,
        &crate::t!("files-no_temp_file", err = "cannot create unique file"),
    );
    None
}

/// 把当前缓冲区内容写入给定文件（对应 C 版 `write_file(..., SPECIAL, NONOTES)`，
/// 不显示状态栏消息）。
fn write_buffer_to_file(path: &str) -> bool {
    let mut content = String::new();
    with_global(|g| {
        let of = g.openfile.clone();
        if let Some(of) = of {
            let r = of.borrow();
            let mut cur = r.filetop.clone();
            let mut first = true;
            while let Some(c) = cur {
                let (data, next) = {
                    let r = c.borrow();
                    (r.data.clone(), r.next.clone())
                };
                if first {
                    content.push_str(&data);
                    first = false;
                } else {
                    content.push('\n');
                    content.push_str(&data);
                }
                cur = next;
            }
        }
    });
    std::fs::write(path, content).is_ok()
}

/// 执行给定程序（拼写器或格式化器）处理临时文件，然后把处理结果
/// 读回替换（标记区域或整个）缓冲区（对应 text.c 的 `treat`）。
fn treat(tempfile_name: &str, theprogram: &str, spelling: bool) {
    /* 空缓冲区时无事可做。 */
    let (size, marked) = with_global(|g| {
        let of = g.openfile.as_ref().map(|o| o.borrow());
        let sz = of.as_ref().map(|r| {
            r.filebot.as_ref().map(|b| b.borrow().data.len()).unwrap_or(0)
        }).unwrap_or(0);
        let marked = of.as_ref().map(|r| r.mark.is_some()).unwrap_or(false);
        (sz, marked)
    });
    if size == 0 && !marked {
        let msg = if marked {
            crate::t!("text-selection_is_empty")
        } else {
            crate::t!("text-buffer_is_empty")
        };
        winio::statusline(MessageType::Ahem, &msg);
        return;
    }

    if spelling {
        winio::leave_terminal();
    } else {
        winio::statusbar(&crate::t!("text-invoking_formatter"));
    }

    /* 运行 program tempfile_name。 */
    let program = theprogram.to_string();
    let result = run_program_with_file(&program, tempfile_name);

    if spelling {
        winio::enter_terminal();
        winio::full_refresh();
    } else {
        winio::full_refresh();
    }

    match result {
        Err(_e) => {
            winio::statusline(
                MessageType::Alert,
                &crate::t!("text-error_invoking", program = theprogram),
            );
            return;
        }
        Ok(exit_code) => {
            if exit_code > 2 {
                winio::statusline(
                    MessageType::Alert,
                    &crate::t!("text-error_invoking", program = theprogram),
                );
                return;
            }
            if exit_code != 0 {
                winio::statusline(
                    MessageType::Alert,
                    &crate::t!("text-error_invoking", program = theprogram),
                );
            }
        }
    }

    /* 读回处理后的临时文件并替换（标记区域或整个）缓冲区。 */
    let Ok(new_text) = std::fs::read_to_string(tempfile_name) else {
        winio::statusline(MessageType::Alert, &crate::t!("files-error_reading", filename = tempfile_name, err = "read back"));
        return;
    };

    if new_text.is_empty() && size > 0 {
        /* 程序清空了文件——按 C 版语义，空输出不替换。 */
        winio::statusline(MessageType::Remark, &crate::t!("text-nothing_changed"));
        return;
    }

    replace_buffer_with_text(&new_text);

    if spelling {
        winio::statusline(MessageType::Remark, &crate::t!("text-finished_checking_spelling"));
    } else {
        winio::statusline(MessageType::Remark, &crate::t!("text-justified"));
    }
}

/// 在 shell 中运行 "program filename"（程序可能带参数，按空格拆分）。
fn run_program_with_file(program: &str, filename: &str) -> Result<i32, String> {
    use std::process::Command;

    let mut parts = program.split_whitespace();
    let Some(prog) = parts.next() else {
        return Err("empty program".to_string());
    };
    let mut cmd = Command::new(prog);
    cmd.args(parts);
    cmd.arg(filename);

    let status = cmd.status().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

/// 用给定文本替换（标记区域或整个）缓冲区（对应 C 版 `replace_buffer`）：
/// 把原内容剪掉丢弃，再把新文本插入光标处。
fn replace_buffer_with_text(new_text: &str) {
    /* 保存剪贴板并临时清空。 */
    let was_cutbuffer = with_global(|g| g.cutbuffer.clone());
    with_global_mut(|g| {
        g.cutbuffer = None;
        g.cutbottom = None;
    });

    add_undo(UndoType::CoupleBegin, Some("spelling correction"));

    /* 从顶部开始剪（整个缓冲区）。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.current = of.filetop.clone();
            of.current_x = 0;
        }
    });

    add_undo(UndoType::Cut, None);
    cut::do_snip(false, true, false);
    update_undo(UndoType::Cut);

    /* 丢弃剪下的内容，恢复原剪贴板。 */
    let old_cb = with_global(|g| g.cutbuffer.clone());
    files::free_lines(old_cb);
    with_global_mut(|g| g.cutbuffer = was_cutbuffer);

    /* 把新文本插入到已清空的缓冲区。 */
    files::insert_text_into_buffer(new_text);

    add_undo(UndoType::CoupleEnd, Some("spelling correction"));
}

/// 将 String 泄漏为 &'static str（用于 undo 消息；等价于 C 中复制后不释放）。
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// do_undo 中 JOIN 特殊情况分支的辅助。
/// 原版 C 代码在该分支只做 `break`（不恢复文本，仅定位光标），
/// 因此这里保持空实现，与 C 版行为一致。
fn break_case(_u: &UndoRef, _undidmsg: &mut Option<&'static str>) {}

// ======================== 文本注入（对应 nano.c 的 inject） ========================

/// 将给定的短字节串插入编辑缓冲区（对应 `inject`）。
pub fn inject(burst: &[u8], count: usize) {
    let of = openfile_ref();
    let thisline = of.borrow().current.clone().unwrap();
    let mut original_row = 0;
    let mut old_amount = 0;

    let (cursor_row, editwinrows) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.cursor_row, g.editwinrows)
    });

    if ISSET(SOFTWRAP) {
        if cursor_row == (editwinrows - 1) as isize {
            original_row = winio::chunk_for(utils::xplustabs(), &thisline);
        }
        old_amount = winio::extra_chunks_in(&thisline);
    }

    /* 把内嵌 NUL 字节编码为 0x0A。 */
    let mut burst_vec = burst[..count.min(burst.len())].to_vec();
    for b in &mut burst_vec {
        if *b == 0 {
            *b = b'\n';
        }
    }

    /* 仅当当前项不是 ADD 或当前输入不与上次输入连续时添加新 undo 项。 */
    let need_new = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let this_lineno = thisline.borrow().lineno;
            let this_x = of.current_x;
            let contig = of.last_action == UndoType::Add
                && of.current_undo.as_ref().map(|cu| cu.borrow().tail_lineno).unwrap_or(-1) == this_lineno
                && of.current_undo.as_ref().map(|cu| cu.borrow().tail_x).unwrap_or(usize::MAX) == this_x;
            !contig
        }).unwrap_or(true)
    });
    if need_new {
        add_undo(UndoType::Add, None);
    }

    /* 为新字节腾出空间并复制到行中。
     * 原版 C 用 memmove + memcpy 在字节缓冲区插入；这里用 String::insert_str
     * 直接在行数据上插入，避免整行 clone()+into_bytes()+from_utf8_lossy() 往返
     * （原实现每次插入都拷贝整行两次；现仅对 burst 做一次 lossy 转换）。 */
    let current_x = of.borrow().current_x;
    {
        let insert_str = String::from_utf8_lossy(&burst_vec).into_owned();
        thisline.borrow_mut().data.insert_str(current_x, &insert_str);
    }

    /* 光标在顶行且不在行首块时，添加文本可能改变前一块。 */
    let (is_edittop, firstcolumn) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let is_et = of.edittop.as_ref().map(|e| Rc::ptr_eq(e, &thisline)).unwrap_or(false);
        (is_et, of.firstcolumn)
    });
    if is_edittop && firstcolumn > 0 {
        ensure_firstcolumn_is_aligned();
        with_global_mut(|g| g.refresh_needed = true);
    }

    /* 标记在光标右侧时补偿其位置。 */
    {
        let mut of_ref = of.borrow_mut();
        if of_ref.mark.as_ref().map(|m| Rc::ptr_eq(m, &thisline)).unwrap_or(false)
            && of_ref.current_x < of_ref.mark_x
        {
            of_ref.mark_x += count;
        }
        of_ref.current_x += count;
        of_ref.totsize += chars::mbstrlen(&burst_vec);
    }
    files::set_modified();

    /* 若文本添加到魔法行，创建新的魔法行。 */
    let is_filebot = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.filebot.as_ref().map(|b| Rc::ptr_eq(b, &thisline)).unwrap_or(false)
        }).unwrap_or(false)
    });
    if is_filebot && !ISSET(NO_NEWLINES) {
        utils::new_magicline();
    }

    update_undo(UndoType::Add);

    /* 请求硬换行时进行自动换行。 */
    if ISSET(BREAK_LONG_LINES) {
        do_wrap();
    }

    let placewewant = utils::xplustabs();
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().placewewant = placewewant;
        }
    });

    /* 平移时接近视口边缘需要刷新。 */
    let (united_sidescroll, placewewant, brink) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (g.united_sidescroll, of.placewewant, of.brink)
    });
    let editwincols = with_global(|g| g.editwincols);
    if united_sidescroll && placewewant > brink + editwincols.saturating_sub(CUSHION + 1) {
        with_global_mut(|g| g.refresh_needed = true);
    }

    /* 软换行且当前行块数改变，或位于编辑窗口最后一行并移到新块。 */
    if ISSET(SOFTWRAP) {
        let (current, placewewant, cursor_row, editwinrows) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (
                of.current.clone().unwrap(),
                of.placewewant,
                of.cursor_row,
                g.editwinrows,
            )
        });
        let chunks_changed = winio::extra_chunks_in(&current) != old_amount;
        let moved_to_new_chunk = cursor_row == (editwinrows - 1) as isize
            && winio::chunk_for(placewewant, &current) > original_row;
        if chunks_changed || moved_to_new_chunk {
            with_global_mut(|g| {
                g.refresh_needed = true;
                g.focusing = false;
            });
        }
    }
}
