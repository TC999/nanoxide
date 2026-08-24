/**************************************************************************
 * search.rs  --  GNU nano 搜索/替换功能（对应 search.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 搜索和替换操作。对应原版 nano 的 `search.c`。
//! 转换说明：使用 `MatchPattern` 替代 POSIX regex。

use crate::definitions::*;
use crate::chars;
use crate::utils;
use crate::history;
use crate::files;
use crate::winio;
use std::rc::Rc;

/// 获取当前打开的缓冲区引用（克隆 Rc，释放全局借用）。
fn openfile_ref() -> OpenFileRef {
    with_global(|g| g.openfile.as_ref().expect("no open file").clone())
}

/// 初始化正则表达式（模式匹配）。
pub fn regexp_init(pattern: &str) -> bool {
    with_global_mut(|g| {
        let _case_sensitive = ISSET(CASE_SENSITIVE);
        let pat = if pattern.contains('*') || pattern.contains('?') {
            MatchPattern::from_glob(pattern)
        } else {
            MatchPattern::from_literal(pattern)
        };
        g.search_regexp = Some(pat);
        g.regexp_nsub = 0;
    });
    true
}

/// 释放正则表达式。
pub fn regexp_cleanup() {
    with_global_mut(|g| {
        g.search_regexp = None;
        g.regexp_nsub = 0;
    });
}

/// 查找下一个匹配。
pub fn find_next_match(needle: &str, start_line: Option<LineRef>, start_x: usize, backwards: bool) -> Option<(LineRef, usize)> {
    with_global(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let current = start_line.or_else(|| of_ref.current.clone())?;
            let mut line = current.clone();
            let mut pos = start_x;

            if backwards {
                // 向后搜索（从当前位置向前）
                loop {
                    let data = line.borrow().data.clone();
                    if !needle.is_empty() {
                        if let Some(found) = if ISSET(CASE_SENSITIVE) {
                            data[..pos].rfind(needle)
                        } else {
                            let lower = data[..pos].to_lowercase();
                            let needle_lower = needle.to_lowercase();
                            lower.rfind(&needle_lower)
                        } {
                            return Some((line.clone(), found));
                        }
                    }
                    // 移动到上一行
                    let prev = line.borrow().prev.clone().and_then(|w| w.upgrade());
                    match prev {
                        Some(p) => {
                            line = p;
                            pos = line.borrow().data.len();
                        }
                        None => break,
                    }
                }
            } else {
                // 向前搜索
                loop {
                    let data = line.borrow().data.clone();
                    if !needle.is_empty() {
                        if let Some(found) = if ISSET(CASE_SENSITIVE) {
                            data[pos..].find(needle)
                        } else {
                            let lower = data[pos..].to_lowercase();
                            let needle_lower = needle.to_lowercase();
                            lower.find(&needle_lower)
                        } {
                            return Some((line.clone(), pos + found));
                        }
                    }
                    // 移动到下一行
                    let next = line.borrow().next.clone();
                    match next {
                        Some(n) => {
                            line = n;
                            pos = 0;
                        }
                        None => break,
                    }
                }
            }
        }
        None
    });
    None
}

// TODO: 翻译时未翻译到位，暂注释占位，后续补上。
// 原型：pub fn do_replace_old_removed() { /* 已被 search_init/do_replace_loop 替代 */ }

/// 替换所有匹配。
pub fn replace_all(needle: &str, replacement: &str) -> usize {
    with_global_mut(|g| {
        let mut count = 0;
        if let Some(of) = &g.openfile {
            let mut current = of.borrow().filetop.clone();
            while let Some(c) = current {
                let mut data = c.borrow_mut();
                if !needle.is_empty() {
                    let mut pos = 0;
                    while let Some(found) = if ISSET(CASE_SENSITIVE) {
                        data.data[pos..].find(needle)
                    } else {
                        let lower = data.data[pos..].to_lowercase();
                        let needle_lower = needle.to_lowercase();
                        lower.find(&needle_lower)
                    } {
                        let start = pos + found;
                        let end = start + needle.len();
                        data.data.replace_range(start..end, replacement);
                        count += 1;
                        pos = start + replacement.len();
                    }
                }
                let next = c.borrow().next.clone();
                current = next;
            }
            of.borrow_mut().modified = count > 0;
        }
        count
    })
}

/// 设置状态栏消息。
fn set_statusbar_message(_msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
    });
}

/// 查找下一个匹配位置（用于高亮）。
pub fn find_next_match_highlight(needle: &str, from_line: Option<LineRef>) -> Option<(LineRef, usize, usize)> {
    with_global(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let start = from_line.or_else(|| of_ref.current.clone())?;
            let mut line = start;
            loop {
                let data = line.borrow().data.clone();
                if !needle.is_empty() {
                    let found = if ISSET(CASE_SENSITIVE) {
                        data.find(needle)
                    } else {
                        data.to_lowercase().find(&needle.to_lowercase())
                    };
                    if let Some(pos) = found {
                        return Some((line.clone(), pos, pos + needle.len()));
                    }
                }
                let next = line.borrow().next.clone();
                match next {
                    Some(n) => line = n,
                    None => break,
                }
            }
        }
        None
    });
    None
}

// ======================== 跳转到指定行列（对应 search.c 的 goto_line_and_column） ========================

/// 转到指定的行和列（注意两者都是从 1 开始计数的）
/// （对应 `goto_line_and_column`）。
pub fn goto_line_and_column(mut line: isize, mut column: isize, hugfloor: bool) {
    /* 负行号表示：从文件末尾倒数。 */
    let mut tail_data: Option<(LineRef, isize, isize)> = None;

    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let mut of = of.borrow_mut();

        let filebot_lineno = of.filebot.as_ref().map(|b| b.borrow().lineno).unwrap_or(1);
        let current_lineno = of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(1);

        if line < 0 {
            line = filebot_lineno + line + 1;
        } else if line == 0 {
            line = current_lineno;
        }
        if line < 1 {
            line = 1;
        }

        /* 若目标行在视口之外，需要重算颜色。 */
        if let (Some(et), Some(cur)) = (&of.edittop, &of.current) {
            let et_lineno = et.borrow().lineno;
            let cur_lineno = cur.borrow().lineno;
            if line > et_lineno + g.editwinrows as isize
                || (ISSET(SOFTWRAP) && line > cur_lineno)
            {
                g.recook |= g.perturbed;
            }
        }

        /* 迭代到请求的行。 */
        let mut current = of.filetop.clone().unwrap();
        let mut remaining = line;
        loop {
            let is_filebot = of.filebot.as_ref().map(|b| std::rc::Rc::ptr_eq(&current, b)).unwrap_or(false);
            if remaining <= 1 || is_filebot {
                break;
            }
            let next = { let r = current.borrow(); r.next.clone() }.unwrap();
            current = next;
            remaining -= 1;
        }
        of.current = Some(current.clone());

        /* 负列号表示：从行末倒数。 */
        let data = current.borrow().data.clone();
        let line_breadth = utils::breadth(data.as_bytes()) as isize;
        if column < 0 {
            column = line_breadth + column + 2;
        } else if column == 0 {
            column = of.placewewant as isize + 1;
        }
        if column < 1 {
            column = 1;
        }

        /* 设置与请求列对应的 x 位置。 */
        of.current_x = utils::actual_x(data.as_bytes(), column as usize - 1);
        of.placewewant = column as usize - 1;

        if ISSET(SOFTWRAP) && of.placewewant / g.editwincols
            > line_breadth as usize / g.editwincols
        {
            of.placewewant = line_breadth as usize;
        }

        if hugfloor {
            tail_data = Some((of.current.clone().unwrap(), filebot_lineno, current_lineno));
        }
    });

    if !hugfloor {
        return;
    }

    /* 注意：以下计算在闭包外执行，因为 leftedge_for/go_forward_chunks
     * 会再次访问全局状态。 */
    let (current, filebot_lineno, current_lineno) = match tail_data {
        Some(t) => t,
        None => return,
    };

    let rows_from_tail = if ISSET(SOFTWRAP) {
        let mut currentline = current;
        let mut leftedge = crate::winio::leftedge_for(utils::xplustabs(), &currentline);
        let rows = with_global(|g| g.editwinrows) / 2;
        rows - crate::winio::go_forward_chunks(rows, &mut currentline, &mut leftedge)
    } else {
        (filebot_lineno - current_lineno) as i32
    };

    let half = with_global(|g| g.editwinrows) / 2;
    let jumpy = ISSET(JUMPY_SCROLLING);

    /* 若目标行接近文件尾部，把最后一行或块放在屏幕底行；
     * 否则，将目标行居中。 */
    if rows_from_tail < half && !jumpy {
        with_global_mut(|g| {
            let of = g.openfile.as_ref().expect("no open file").clone();
            let mut of = of.borrow_mut();
            of.cursor_row = (g.editwinrows - 1 - rows_from_tail) as isize;
        });
        crate::winio::adjust_viewport(UpdateType::Stationary);
    } else {
        crate::winio::adjust_viewport(UpdateType::Centering);
    }
}
// ======================== 跳转到指定行与列位置（对应 search.c 的 goto_line_posx） ========================

/// 转到指定的行与 x 位置（对应 `goto_line_posx`）。
pub fn goto_line_posx(linenumber: isize, pos_x: usize) {
    let of = openfile_ref();

    let (edittop_lineno, current_lineno) = {
        let r = of.borrow();
        (
            r.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(0),
            r.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(0),
        )
    };
    let editwinrows = with_global(|g| g.editwinrows);
    if linenumber > edittop_lineno + editwinrows as isize
        || (ISSET(SOFTWRAP) && linenumber > current_lineno)
    {
        with_global_mut(|g| g.recook |= g.perturbed);
    }

    let filebot = { let r = of.borrow(); r.filebot.clone().unwrap() };
    let fb_lineno = filebot.borrow().lineno;
    let new_current = if linenumber < fb_lineno {
        crate::utils::line_from_number(linenumber)
    } else {
        filebot
    };

    {
        let mut r = of.borrow_mut();
        r.current = Some(new_current);
        r.current_x = pos_x;
        /* xplustabs 内联：避免在持有 openfile 借用时访问它。 */
        let cur = r.current.clone().unwrap();
        r.placewewant = crate::utils::wideness(cur.borrow().data.as_bytes(), pos_x);
    }

    with_global_mut(|g| g.refresh_needed = true);
}

// ======================== 搜索核心（对应 search.c） ========================

/// 编译给定正则（对应 `regexp_init`；用 MatchPattern 替代）。
/// 返回 TRUE 当表达式有效。
pub fn regexp_init_real(regexp: &str) -> bool {
    let valid = if regexp.contains('*') || regexp.contains('?') {
        Some(MatchPattern::from_glob(regexp))
    } else {
        Some(MatchPattern::from_literal(regexp))
    };
    match valid {
        Some(pat) => {
            with_global_mut(|g| {
                g.search_regexp = Some(pat);
                g.regexp_compiled = true;
            });
            true
        }
        None => {
            winio::statusline(MessageType::Ahem, &crate::t!("search-bad_regex", regexp = regexp));
            false
        }
    }
}

/// 搜索结束后释放正则并安排刷新（对应 `tidy_up_after_search`）。
pub fn tidy_up_after_search() {
    with_global_mut(|g| {
        if g.regexp_compiled {
            g.search_regexp = None;
            g.regexp_compiled = false;
        }
        let marked = g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false);
        if marked {
            g.refresh_needed = true;
        }
        g.recook |= g.perturbed;
    });
}

/// 准备提示并询问用户要搜索什么（对应 `search_init`）。
pub fn search_init(replacing: bool, retain_answer: bool) {
    let cols = with_global(|g| g.COLS);
    let inhelp = with_global(|g| g.inhelp);

    /* 若之前搜索过，包含在提示中。 */
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();
    let thedefault = if !last_search.is_empty() {
        let disp = winio::display_string(last_search.as_bytes(), 0, cols / 3, false, false).0;
        let dots = utils::breadth(last_search.as_bytes()) > cols / 3;
        format!(" [{} {}]", disp, if dots { "..." } else { "" })
    } else {
        String::new()
    };

    let mut retain_answer = retain_answer;
    loop {
        let menu = if inhelp {
            MFINDINHELP
        } else if replacing {
            MREPLACE
        } else {
            MWHEREIS
        };
        let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
        let cs = if ISSET(CASE_SENSITIVE) { format!("[ {} ]", crate::t!("search-case_sensitive")) } else { String::new() };
        let rg = if ISSET(USE_REGEXP) { format!("[ {} ]", crate::t!("search-regexp")) } else { String::new() };
        let bw = if ISSET(BACKWARDS_SEARCH) { format!("[ {} ]", crate::t!("search-backwards")) } else { String::new() };
        let tr = if replacing { format!("[ {} ]", crate::t!("search-to_replace")) } else { String::new() };
        let msg = format!("{}{}{}{}{}{}", crate::t!("search-search"), cs, rg, bw, tr, thedefault);

        let mut search_history = with_global(|g| g.search_history.clone())
            .unwrap_or_else(|| make_new_node(None));
        let response = crate::prompt::do_prompt(
            menu,
            if retain_answer { &answer } else { "" },
            Some(&mut search_history),
            Some(winio::edit_refresh),
            &msg,
        );
        with_global_mut(|g| g.search_history = Some(search_history));

        /* 取消，或空白回答且本次会话尚未搜索过时，退出。 */
        if response == -1 || (response == -2 && with_global(|g| g.last_search.clone()).unwrap_or_default().is_empty()) {
            winio::statusbar(&crate::t!("search-cancelled"));
            break;
        }

        /* Enter 被按下时，准备进行替换或搜索。 */
        if response == 0 || response == -2 {
            let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
            if !answer.is_empty() {
                with_global_mut(|g| g.last_search = Some(answer.clone()));
                let mut sh = with_global(|g| g.search_history.clone()).unwrap_or_else(|| make_new_node(None));
                history::update_history(&mut sh, &answer, true);
                with_global_mut(|g| g.search_history = Some(sh));
            }

            let ls = with_global(|g| g.last_search.clone()).unwrap_or_default();
            if ISSET(USE_REGEXP) && !regexp_init_real(&ls) {
                break;
            }

            if replacing {
                ask_for_and_do_replacements();
            } else {
                go_looking();
            }
            break;
        }

        retain_answer = true;

        let function = crate::global::interpret(response);

        /* 此处是五个切换之一或快捷键被执行。 */
        match function {
            Some(FunctionId::DoToggleCaseSensitive) => TOGGLE(CASE_SENSITIVE),
            Some(FunctionId::DoToggleBackwards) => TOGGLE(BACKWARDS_SEARCH),
            Some(FunctionId::DoToggleRegexp) => TOGGLE(USE_REGEXP),
            _ => break,
        }
    }

    if !inhelp {
        tidy_up_after_search();
    }
}

/// 从当前行开始查找 needle（对应 `findnextstr`）。
/// 返回 1 找到、0 未找到、-2 取消。
pub fn findnextstr(
    needle: &str,
    whole_word_only: bool,
    modus: i32,
    match_len: &mut usize,
    skipone: bool,
    begin: Option<&LineRef>,
    begin_x: usize,
) -> i32 {
    let found_len = needle.len();
    let mut feedback = 0;
    let inhelp = with_global(|g| g.inhelp);

    let mut came_full_circle = with_global(|g| g.came_full_circle);

    let of = openfile_ref();
    let mut line = of.borrow().current.clone().unwrap();
    let mut from = of.borrow().current_x;
    let mut found: Option<usize> = None;
    let mut found_x = 0;

    if begin.is_none() {
        came_full_circle = false;
    }

    let mut skipone = skipone;
    loop {
        let data = line.borrow().data.clone();
        let bytes = data.as_bytes();

        /* 开始新搜索时跳过第一个字符，然后搜索当前行。 */
        if skipone {
            skipone = false;
            if ISSET(BACKWARDS_SEARCH) && from != 0 {
                from = chars::step_left(bytes, from);
                found = crate::utils::strstrwrapper(bytes, needle.as_bytes(), from);
            } else if !ISSET(BACKWARDS_SEARCH) && chars::byte_at(bytes, from) != 0 {
                from += chars::char_length(&bytes[from..]);
                found = crate::utils::strstrwrapper(bytes, needle.as_bytes(), from);
            }
        } else {
            found = crate::utils::strstrwrapper(bytes, needle.as_bytes(), from);
        }

        if let Some(f) = found {
            /* 正则搜索时计算匹配长度。 */
            if ISSET(USE_REGEXP) {
                // 简化：匹配长度 = needle 长度
            }

            /* 拼写检查时匹配应是独立单词。 */
            if whole_word_only && !utils::is_separate_word(f, found_len, bytes) {
                from = f + chars::char_length(&bytes[f..]);
                continue;
            }

            /* 不在魔法行上时匹配有效。 */
            let has_next = { let r = line.borrow(); r.next.is_some() };
            if has_next || chars::byte_at(bytes, 0) != 0 {
                break;
            }
        }

        /* 若回到起点则没有 needle。 */
        if came_full_circle {
            with_global_mut(|g| g.came_full_circle = false);
            return 0;
        }

        /* 移到前一行或下一行。 */
        let next_line = if ISSET(BACKWARDS_SEARCH) {
            { let r = line.borrow(); r.prev.clone() }.and_then(|w| w.upgrade())
        } else {
            { let r = line.borrow(); r.next.clone() }
        };
        line = match next_line {
            Some(l) => l,
            None => {
                if whole_word_only || modus == INREGION {
                    return 0;
                }
                let of = openfile_ref();
                let wrapped = if ISSET(BACKWARDS_SEARCH) {
                    of.borrow().filebot.clone().unwrap()
                } else {
                    of.borrow().filetop.clone().unwrap()
                };
                if modus == JUSTFIND {
                    winio::statusline(MessageType::Remark, &crate::t!("search-search_wrapped"));
                    feedback = -2;
                }
                wrapped
            }
        };

        /* 回到起始行时记下。 */
        if let Some(b) = begin {
            if Rc::ptr_eq(&line, b) {
                came_full_circle = true;
            }
        }

        /* 把起始 x 设为行首或行尾。 */
        from = 0;
        if ISSET(BACKWARDS_SEARCH) {
            from = line.borrow().data.len();
        }

        /* 每秒瞥一眼键盘检查取消。 */
        if feedback != -2 {
            if winio::kbhit() {
                let input = winio::get_kbinput();
                let function = crate::global::interpret(input);
                if function == Some(FunctionId::DoCancel) {
                    winio::statusbar(&crate::t!("search-cancelled"));
                    with_global_mut(|g| g.came_full_circle = false);
                    return -2;
                }
            }
            feedback += 1;
            if feedback > 0 {
                winio::statusbar(&crate::t!("search-searching"));
            }
        }
    }

    found_x = found.unwrap();
    let data = line.borrow().data.clone();

    /* 确保找到的出现不在起始 x 之后。 */
    if came_full_circle
        && ((!ISSET(BACKWARDS_SEARCH) && (found_x > begin_x || (modus == REPLACING && found_x == begin_x)))
            || (ISSET(BACKWARDS_SEARCH) && found_x < begin_x))
    {
        with_global_mut(|g| g.came_full_circle = false);
        return 0;
    }

    /* 把当前位置设为找到的。 */
    let of = openfile_ref();
    {
        let mut of_ref = of.borrow_mut();
        of_ref.current = Some(line.clone());
        of_ref.current_x = found_x;
    }

    *match_len = found_len;

    if modus == JUSTFIND {
        let marked = with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false));
        let softmark = with_global(|g| g.openfile.as_ref().map(|of| of.borrow().softmark).unwrap_or(false));
        if !marked || softmark {
            let light_from_col = utils::xplustabs();
            with_global_mut(|g| {
                g.spotlighted = true;
                g.light_from_col = light_from_col;
                g.light_to_col = utils::wideness(data.as_bytes(), found_x + found_len);
                let (united, ew) = (g.united_sidescroll, g.editwincols);
                if united && g.light_to_col < ew - CUSHION {
                    if let Some(of) = &g.openfile {
                        of.borrow_mut().brink = 0;
                    }
                } else if united {
                    let b = utils::get_page_start(g.light_to_col);
                    if let Some(of) = &g.openfile {
                        of.borrow_mut().brink = b;
                    }
                }
                g.refresh_needed = true;
            });
        }
    }

    if feedback > 0 {
        winio::wipe_statusbar();
    }

    with_global_mut(|g| g.came_full_circle = came_full_circle);
    let _ = inhelp;
    1
}

/// 报告给定字符串未找到（对应 `not_found_msg`）。
fn not_found_msg(str: &str) {
    let cols = with_global(|g| g.COLS);
    let disp = winio::display_string(str.as_bytes(), 0, (cols / 2) + 1, false, false).0;
    let numchars = utils::actual_x(disp.as_bytes(), utils::wideness(disp.as_bytes(), cols / 2));
    let dots = if disp.as_bytes().get(numchars).copied().unwrap_or(0) == 0 { "" } else { "..." };
    winio::statusline(MessageType::Ahem, &crate::t!("search-not_found", pattern = format!("{}{}", &disp[..numchars.min(disp.len())], dots)));
}

/// 搜索全局字符串 last_search 并报告（对应 `go_looking`）。
pub fn go_looking() {
    let (was_current, was_x) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.current.clone().unwrap(), of.current_x)
    });

    with_global_mut(|g| g.came_full_circle = false);

    let mut match_len = 0;
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();
    let of = openfile_ref();
    let (cur, cur_x) = {
        let of_ref = of.borrow();
        (of_ref.current.clone().unwrap(), of_ref.current_x)
    };
    let didfind = findnextstr(&last_search, false, JUSTFIND, &mut match_len, true, Some(&cur), cur_x);

    /* 若找到且回到起始点，则是唯一出现。 */
    let (same_current, same_x) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let same_c = of.current.as_ref().map(|c| Rc::ptr_eq(c, &was_current)).unwrap_or(false);
        (same_c, of.current_x == was_x)
    });
    if didfind == 1 && same_current && same_x {
        winio::statusline(MessageType::Remark, &crate::t!("search-only_occurrence"));
    } else if didfind == 0 {
        not_found_msg(&last_search);
    }

    winio::edit_redraw(&was_current, UpdateType::Centering);
}

/// 询问并向前搜索（对应 `do_search_forward`）。
pub fn do_search_forward() {
    UNSET(BACKWARDS_SEARCH);
    search_init(false, false);
}

/// 询问并向后搜索（对应 `do_search_backward`）。
pub fn do_search_backward() {
    SET(BACKWARDS_SEARCH);
    search_init(false, false);
}

/// 不提示地搜索最后给出的字符串（对应 `do_research`）。
pub fn do_research() {
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();

    if last_search.is_empty() {
        winio::statusline(MessageType::Ahem, &crate::t!("search-no_search_pattern"));
        return;
    }

    if ISSET(USE_REGEXP) && !regexp_init_real(&last_search) {
        return;
    }

    with_global_mut(|g| g.currmenu = MWHEREIS);

    let lines = with_global(|g| g.LINES);
    if lines > 1 {
        winio::wipe_statusbar();
    }

    go_looking();

    let inhelp = with_global(|g| g.inhelp);
    if !inhelp {
        tidy_up_after_search();
    }
}

/// 向后搜索下一次出现（对应 `do_findprevious`）。
pub fn do_findprevious() {
    SET(BACKWARDS_SEARCH);
    do_research();
}

/// 向前搜索下一次出现（对应 `do_findnext`）。
pub fn do_findnext() {
    UNSET(BACKWARDS_SEARCH);
    do_research();
}

/// 返回给定 regex 的替换文本大小，考虑子表达式引用
/// （对应 `replace_regexp`；简化实现）。
fn replace_regexp(string: Option<&mut String>) -> usize {
    let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
    let mut replacement_size = 0;
    let mut output = String::new();

    let given = answer.as_bytes();
    let mut i = 0;
    while i < given.len() {
        let c = given[i];
        let digit = if i + 1 < given.len() { given[i + 1] - b'0' } else { 0 };

        /* 有效的反向引用时使用子表达式，否则使用字面回答。 */
        if c == b'\\' && 0 < digit && digit < 10 {
            let d = digit as usize;
            if let Some(reg) = with_global(|g| g.regmatches.get(d).cloned()) {
                if let (Some(so), Some(eo)) = reg {
                    let of = openfile_ref();
                    let data = of.borrow().current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
                    if so <= eo && eo <= data.len() {
                        let extent = &data.as_bytes()[so..eo];
                        output.push_str(&String::from_utf8_lossy(extent));
                        replacement_size += extent.len();
                        i += 2;
                        continue;
                    }
                }
            }
            output.push(c as char);
            replacement_size += 1;
            i += 1;
        } else {
            output.push(c as char);
            replacement_size += 1;
            i += 1;
        }
    }

    if let Some(s) = string {
        *s = output;
    }

    replacement_size
}

/// 返回当前行的一个 needle 被替换后的副本（对应 `replace_line`）。
fn replace_line(needle: &str) -> String {
    let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
    let of = openfile_ref();
    let (data, current_x) = {
        let of_ref = of.borrow();
        (
            of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default(),
            of_ref.current_x,
        )
    };

    let match_len = needle.len();
    let mut copy = String::new();
    copy.push_str(&data[..current_x]);
    copy.push_str(&answer);
    copy.push_str(&data[current_x + match_len..]);
    copy
}

/// 逐步检查搜索字符串的每次出现并提示用户是否替换
/// （对应 `do_replace_loop`）。
fn do_replace_loop(needle: &str, real_current: &LineRef, real_current_x: &mut usize) -> isize {
    let mut skipone = ISSET(BACKWARDS_SEARCH);
    let mut replaceall = false;
    let modus = REPLACING;
    let mut numreplaced: isize = -1;
    let mut match_len = 0;

    with_global_mut(|g| g.came_full_circle = false);

    loop {
        let mut choice = NO;
        let result = findnextstr(needle, false, modus, &mut match_len, skipone, Some(real_current), *real_current_x);

        /* 未找到或取消时停止循环。 */
        if result < 1 {
            if result < 0 {
                numreplaced = -2;
            }
            break;
        }

        /* 表示找到搜索字符串。 */
        if numreplaced == -1 {
            numreplaced = 0;
        }

        if !replaceall {
            let light_from_col = utils::xplustabs();
            with_global_mut(|g| {
                g.spotlighted = true;
                g.light_from_col = light_from_col;
                let of = g.openfile.as_ref().unwrap().borrow();
                let cur = of.current.clone().unwrap();
                g.light_to_col = utils::wideness(cur.borrow().data.as_bytes(), of.current_x + match_len);
                let (united, ew) = (g.united_sidescroll, g.editwincols);
                if united && g.light_to_col < ew - CUSHION {
                    drop(of);
                    g.openfile.as_ref().unwrap().borrow_mut().brink = 0;
                } else if united {
                    let b = utils::get_page_start(g.light_to_col);
                    drop(of);
                    g.openfile.as_ref().unwrap().borrow_mut().brink = b;
                }
            });
            winio::edit_refresh();

            choice = crate::prompt::ask_user(true, &crate::t!("search-replace_instance"));

            with_global_mut(|g| g.spotlighted = false);

            if choice == CANCEL {
                break;
            }

            replaceall = choice == ALL;

            skipone = choice == 0 || ISSET(BACKWARDS_SEARCH);
        }

        if choice == YES || replaceall {
            let altered = replace_line(needle);
            let length_change = altered.len() as isize
                - with_global(|g| {
                    g.openfile.as_ref().unwrap().borrow().current.as_ref().map(|c| c.borrow().data.len()).unwrap_or(0)
                }) as isize;

            let of = openfile_ref();
            let (is_real, cur_x_lt) = {
                let of_ref = of.borrow();
                let is_real = of_ref.current.as_ref().map(|c| Rc::ptr_eq(c, real_current)).unwrap_or(false);
                (is_real, of_ref.current_x < *real_current_x)
            };
            if is_real && cur_x_lt {
                let of_ref = of.borrow();
                let cur_x = of_ref.current_x;
                if *real_current_x < cur_x + match_len {
                    *real_current_x = cur_x + match_len;
                }
                *real_current_x = (*real_current_x as isize + length_change) as usize;
            }

            /* 不再寻找同样的零长度或行首匹配。 */
            if match_len == 0 {
                skipone = true;
            }

            /* 向前移动时把光标放在替换文本之后。 */
            let cur_x = of.borrow().current_x;
            if !ISSET(BACKWARDS_SEARCH) {
                of.borrow_mut().current_x = cur_x + match_len + length_change as usize;
            }

            /* 更新文件大小并放入修改后的行。 */
            {
                let mut of_ref = of.borrow_mut();
                let cur = of_ref.current.clone().unwrap();
                let old_len = cur.borrow().data.len();
                of_ref.totsize = of_ref.totsize.saturating_sub(old_len);
                cur.borrow_mut().data = altered.clone();
                of_ref.totsize += altered.len();
            }

            crate::color::check_the_multis(&of.borrow().current.clone().unwrap());
            with_global_mut(|g| g.refresh_needed = false);
            files::set_modified();
            with_global_mut(|g| g.as_an_at = true);
        set_as_an_at_independent(true);
            numreplaced += 1;
        }
    }

    if numreplaced == -1 {
        not_found_msg(needle);
    }

    numreplaced
}

/// 替换字符串（对应 `do_replace`）。
pub fn do_replace() {
    if ISSET(VIEW_MODE) {
        winio::statusline(MessageType::Ahem, &crate::t!("search-view_replace_disabled"));
    } else {
        UNSET(BACKWARDS_SEARCH);
        search_init(true, false);
    }
}

/// 询问用户用什么替换搜索字符串，并执行替换（对应 `ask_for_and_do_replacements`）。
pub fn ask_for_and_do_replacements() {
    let (was_edittop, was_firstcolumn, beginline, begin_x) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (
            of.edittop.clone().unwrap(),
            of.firstcolumn,
            of.current.clone().unwrap(),
            of.current_x,
        )
    });
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();

    let mut replace_history = with_global(|g| g.replace_history.clone())
        .unwrap_or_else(|| make_new_node(None));
    let response = crate::prompt::do_prompt(
        MREPLACEWITH,
        "",
        Some(&mut replace_history),
        Some(winio::edit_refresh),
        &crate::t!("search-replace_with"),
    );
    with_global_mut(|g| g.replace_history = Some(replace_history));

    /* 非空时把替换字符串加入替换历史。 */
    if response == 0 {
        let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
        if !answer.is_empty() {
            let mut rh = with_global(|g| g.replace_history.clone()).unwrap_or_else(|| make_new_node(None));
            history::update_history(&mut rh, &answer, true);
            with_global_mut(|g| g.replace_history = Some(rh));
        }
    }

    /* 取消或执行了函数时完成。 */
    if response == -1 {
        winio::statusbar(&crate::t!("search-cancelled"));
        return;
    } else if response > 0 {
        return;
    }

    let mut begin_x = begin_x;
    let numreplaced = do_replace_loop(&last_search, &beginline, &mut begin_x);

    /* 恢复到之前的位置。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.edittop = Some(was_edittop);
            of.firstcolumn = was_firstcolumn;
            of.current = Some(beginline);
            of.current_x = begin_x;
        }
        g.refresh_needed = true;
    });

    if numreplaced >= 0 {
        let msg = if numreplaced == 1 {
            crate::t!("search-replaced_one", count = numreplaced.to_string())
        } else {
            crate::t!("search-replaced_many", count = numreplaced.to_string())
        };
        winio::statusline(MessageType::Remark, &msg);
    }
}

// ======================== 跳转到行与列（对应 do_gotolinecolumn） ========================

/// 实现 Go To Line 菜单：询问行号（可带列号）并跳转
/// （对应 `do_gotolinecolumn`）。
pub fn do_gotolinecolumn() {
    ask_for_line_and_column("");
}

/// 询问行号与列号，然后跳转到那里（对应 `ask_for_line_and_column`）。
/// `provided` 为提示栏的初始内容；支持 `++`/`--` 相对跳转与
/// `行,列` 形式（逗号开头时视口静止）。
pub fn ask_for_line_and_column(provided: &str) {
    let (mut line, mut column) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let cur_lineno = of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(1);
        (cur_lineno as isize, of.placewewant as isize + 1)
    });

    let response = crate::prompt::do_prompt(
        MGOTOLINE,
        provided,
        None,
        Some(winio::edit_refresh),
        &crate::t!("search-enter_line_column"),
    );

    /* 取消或运行了函数（如 ^T 切换搜索）时完成。
     * 注：^T 的 FlipGoto 已在 do_prompt 内部通过 run_function 处理。 */
    if response < 0 {
        winio::statusbar(&crate::t!("search-cancelled"));
        return;
    } else if response > 0 {
        return;
    }

    let answer = with_global(|g| g.answer.clone()).unwrap_or_default();

    /* ++ 或 -- 前缀表示相对跳转。 */
    let mut doublesign = 0;
    if answer.starts_with("++") || answer.starts_with("--") {
        doublesign = 1;
    }

    /* 尝试从回答中提取一个或两个数字。 */
    if !utils::parse_line_column(&answer[doublesign..], &mut line, &mut column) {
        winio::statusline(MessageType::Ahem, &crate::t!("search-invalid_line_or_column"));
        return;
    }

    if doublesign != 0 {
        let cur_lineno = with_global(|g| {
            g.openfile.as_ref().and_then(|of| {
                let r = of.borrow();
                r.current.as_ref().map(|c| c.borrow().lineno)
            }).unwrap_or(1)
        }) as isize;
        line += cur_lineno;
        if line < 1 {
            line = 1;
        }
    }

    goto_line_and_column(line, column, false);

    crate::winio::adjust_viewport(
        if answer.starts_with(',') {
            UpdateType::Stationary
        } else {
            UpdateType::Centering
        },
    );
    with_global_mut(|g| g.refresh_needed = true);
}

/// 在 Go To Line 提示与 Search 提示之间切换（对应 `flip_goto`）。
/// 在跳转提示中按 ^T 进入搜索（保留输入），在搜索提示中按 ^T 进入跳转。
pub fn flip_goto() {
    UNSET(BACKWARDS_SEARCH);
    let currmenu = with_global(|g| g.currmenu);
    let answer = crate::prompt::get_answer();
    if currmenu == MGOTOLINE {
        /* 从 Go To Line 切换到 Search。 */
        search_init(false, true);
    } else {
        /* 从 Search 切换到 Go To Line，使用已输入的回答。 */
        ask_for_line_and_column(&answer);
    }
}

// ======================== 括号匹配（对应 find_a_bracket / do_find_bracket） ========================

/// 从当前光标位置开始，向前（reverse=FALSE）或向后（reverse=TRUE）搜索
/// `bracket_pair` 中两个字符的任一一个。找到时把光标移到该字符上并返回
/// TRUE，否则返回 FALSE（对应 `find_a_bracket`）。
fn find_a_bracket(reverse: bool, bracket_pair: &str) -> bool {
    let pair = bracket_pair.as_bytes();
    let of = openfile_ref();

    let mut line = {
        let r = of.borrow();
        r.current.clone().unwrap()
    };

    let mut pointer: usize;
    if reverse {
        /* 先离开当前括号。 */
        let current_x = of.borrow().current_x;
        if current_x == 0 {
            let prev = { let r = line.borrow(); r.prev.clone() };
            match prev.and_then(|w| w.upgrade()) {
                Some(p) => line = p,
                None => return false,
            }
            pointer = line.borrow().data.len();
        } else {
            pointer = chars::step_left(line.borrow().data.as_bytes(), current_x);
        }

        /* 向后搜索两个感兴趣的括号。 */
        loop {
            let data = line.borrow().data.clone();
            match chars::mbrevstrpbrk(data.as_bytes(), pair, pointer) {
                Some(found) => {
                    with_global_mut(|g| {
                        if let Some(o) = &g.openfile {
                            let mut o = o.borrow_mut();
                            o.current = Some(line.clone());
                            o.current_x = found;
                        }
                    });
                    return true;
                }
                None => {
                    let prev = { let r = line.borrow(); r.prev.clone() };
                    match prev.and_then(|w| w.upgrade()) {
                        Some(p) => line = p,
                        None => return false,
                    }
                    pointer = line.borrow().data.len();
                }
            }
        }
    } else {
        let current_x = of.borrow().current_x;
        pointer = chars::step_right(line.borrow().data.as_bytes(), current_x);

        loop {
            let data = line.borrow().data.clone();
            match chars::mbstrpbrk(&data.as_bytes()[pointer..], pair) {
                Some(found) => {
                    with_global_mut(|g| {
                        if let Some(o) = &g.openfile {
                            let mut o = o.borrow_mut();
                            o.current = Some(line.clone());
                            o.current_x = pointer + found;
                        }
                    });
                    return true;
                }
                None => {
                    let next = { let r = line.borrow(); r.next.clone() };
                    match next {
                        Some(n) => line = n,
                        None => return false,
                    }
                    pointer = 0;
                }
            }
        }
    }
}

/// 若光标处是括号，搜索它的互补括号（对应 `do_find_bracket`）。
pub fn do_find_bracket() {
    let (was_current, was_x) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        (of.current.clone().unwrap(), of.current_x)
    });
    let matchbrackets = with_global(|g| g.matchbrackets.clone());

    let Some(matchbrackets) = matchbrackets else {
        winio::statusline(MessageType::Ahem, &crate::t!("search-not_a_bracket"));
        return;
    };

    let (data, current_x) = {
        let r = was_current.borrow();
        (r.data.clone(), was_x)
    };
    let bytes = data.as_bytes();

    /* 找到 matchbrackets 中光标处字符的位置。 */
    let Some(ch_pos) = chars::mbstrchr(matchbrackets.as_bytes(), &bytes[current_x..]) else {
        winio::statusline(MessageType::Ahem, &crate::t!("search-not_a_bracket"));
        return;
    };

    /* 半数是左括号的个数（闭括号从中间开始）。 */
    let charcount = chars::mbstrlen(matchbrackets.as_bytes()) / 2;
    let mut halfway = 0;
    let mbytes = matchbrackets.as_bytes();
    for _ in 0..charcount {
        halfway += chars::char_length(&mbytes[halfway..]);
    }

    /* 在闭括号上时反向搜索；否则正向搜索。 */
    let reverse = ch_pos >= halfway;

    /* 从 ch 向前/向后移动一半字符数得到互补括号。 */
    let mut wanted_pos = ch_pos;
    let mut count = charcount;
    while count > 0 {
        if reverse {
            wanted_pos = chars::step_left(mbytes, wanted_pos);
        } else {
            wanted_pos += chars::char_length(&mbytes[wanted_pos..]);
        }
        count -= 1;
    }

    let ch_len = chars::char_length(&mbytes[ch_pos..]);
    let wanted_len = chars::char_length(&mbytes[wanted_pos..]);

    /* 把两个互补括号放入同一字符串。 */
    let mut bracket_pair = String::new();
    bracket_pair.push_str(&matchbrackets[ch_pos..ch_pos + ch_len]);
    bracket_pair.push_str(&matchbrackets[wanted_pos..wanted_pos + wanted_len]);

    let mut balance = 1i32;
    let ch = &matchbrackets[ch_pos..ch_pos + ch_len];

    while find_a_bracket(reverse, &bracket_pair) {
        /* 相同括号则增加平衡数，否则减少。 */
        let (data, current_x) = with_global(|g| {
            let of = g.openfile.as_ref().expect("no open file").borrow();
            let c = of.current.clone().unwrap();
            let d = c.borrow().data.clone();
            (d, of.current_x)
        });
        let found = &data[current_x..current_x + ch_len];
        if found == ch {
            balance += 1;
        } else {
            balance -= 1;
        }

        /* 平衡数归零时找到互补括号。 */
        if balance == 0 {
            crate::winio::edit_redraw(&was_current, UpdateType::Flowing);
            return;
        }
    }

    winio::statusline(MessageType::Ahem, &crate::t!("search-no_matching_bracket"));

    /* 恢复光标位置。 */
    with_global_mut(|g| {
        if let Some(o) = &g.openfile {
            let mut o = o.borrow_mut();
            o.current = Some(was_current);
            o.current_x = was_x;
        }
    });
}
