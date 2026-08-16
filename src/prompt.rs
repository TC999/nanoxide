/**************************************************************************
 * prompt.rs  --  GNU nano 状态栏提示（对应 prompt.c）
 * 版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 状态栏提示：获取用户输入、编辑回答、历史导航、Yes/No 询问。
//!
//! 转换说明：
//! - `answer`/`typing_x`/`prompt` 全局放入 [`GlobalState`]；
//! - C 的函数指针（`functionptrtype`）用 [`FunctionId`] 枚举替代，
//!   比较与调用均按枚举匹配；
//! - `absorb_character` 的静态输入缓冲用 `thread_local!`；
//! - 文件补全 `input_tab` 暂以简化实现（`files.rs` 完整翻译时完善）；
//! - 提示栏渲染委托给 [`crate::winio`] 的状态栏显示。

use crate::definitions::*;
use crate::chars;
use crate::history;
use crate::utils;
use crate::winio;
use std::cell::RefCell;
use std::rc::Rc;

// ======================== 回答与提示的全局访问 ========================

pub fn get_answer() -> String {
    with_global(|g| g.answer.clone().unwrap_or_default())
}

pub fn set_answer(s: &str) {
    with_global_mut(|g| g.answer = Some(s.to_string()));
}

fn get_typing_x() -> usize {
    with_global(|g| g.typing_x)
}

fn set_typing_x(x: usize) {
    with_global_mut(|g| g.typing_x = x);
}

fn get_prompt() -> String {
    with_global(|g| g.prompt.clone().unwrap_or_default())
}

// ======================== 回答编辑（对应 prompt.c） ========================

/// 移动到回答的开头（对应 `do_statusbar_home`）。
pub fn do_statusbar_home() {
    set_typing_x(0);
}

/// 移动到回答的末尾（对应 `do_statusbar_end`）。
pub fn do_statusbar_end() {
    let len = get_answer().len();
    set_typing_x(len);
}

/// 移动到回答中的上一个单词（对应 `do_statusbar_prev_word`）。
pub fn do_statusbar_prev_word() {
    let mut seen_a_word = false;
    let mut step_forward = false;
    let answer = get_answer();
    let mut typing_x = get_typing_x();

    /* 向后移动直到越过一个单词的开头。 */
    while typing_x != 0 {
        typing_x = chars::step_left(answer.as_bytes(), typing_x);

        if chars::is_word_char(&answer.as_bytes()[typing_x..], false) {
            seen_a_word = true;
        } else if chars::is_zerowidth(&answer.as_bytes()[typing_x..]) {
            /* 跳过零宽字符。 */
        } else if seen_a_word {
            /* 这是空白：已越过单词开头。 */
            step_forward = true;
            break;
        }
    }

    if step_forward {
        /* 再前进一个字符以停在单词开头。 */
        typing_x = chars::step_right(answer.as_bytes(), typing_x);
    }

    set_typing_x(typing_x);
}

/// 移动到回答中的下一个单词（对应 `do_statusbar_next_word`）。
pub fn do_statusbar_next_word() {
    let answer = get_answer();
    let mut typing_x = get_typing_x();
    let mut seen_space = !chars::is_word_char(&answer.as_bytes()[typing_x..], false);
    let mut seen_word = !seen_space;

    /* 向前移动直到到达单词结尾或开头。 */
    while chars::byte_at(answer.as_bytes(), typing_x) != 0 {
        typing_x = chars::step_right(answer.as_bytes(), typing_x);

        if ISSET(AFTER_ENDS) {
            if chars::is_word_char(&answer.as_bytes()[typing_x..], false) {
                seen_word = true;
            } else if chars::is_zerowidth(&answer.as_bytes()[typing_x..]) {
                /* 跳过零宽字符。 */
            } else if seen_word {
                break;
            }
        } else {
            if chars::is_zerowidth(&answer.as_bytes()[typing_x..]) {
                /* 跳过零宽字符。 */
            } else if !chars::is_word_char(&answer.as_bytes()[typing_x..], false) {
                seen_space = true;
            } else if seen_space {
                break;
            }
        }
    }

    set_typing_x(typing_x);
}

/// 在回答中向左移动一个字符（对应 `do_statusbar_left`）。
pub fn do_statusbar_left() {
    let answer = get_answer();
    let mut typing_x = get_typing_x();
    if typing_x > 0 {
        typing_x = chars::step_left(answer.as_bytes(), typing_x);
        while typing_x > 0 && chars::is_zerowidth(&answer.as_bytes()[typing_x..]) {
            typing_x = chars::step_left(answer.as_bytes(), typing_x);
        }
    }
    set_typing_x(typing_x);
}

/// 在回答中向右移动一个字符（对应 `do_statusbar_right`）。
pub fn do_statusbar_right() {
    let answer = get_answer();
    let mut typing_x = get_typing_x();
    if chars::byte_at(answer.as_bytes(), typing_x) != 0 {
        typing_x = chars::step_right(answer.as_bytes(), typing_x);
        while chars::byte_at(answer.as_bytes(), typing_x) != 0
            && chars::is_zerowidth(&answer.as_bytes()[typing_x..])
        {
            typing_x = chars::step_right(answer.as_bytes(), typing_x);
        }
    }
    set_typing_x(typing_x);
}

/// 在回答中退格删除一个字符（对应 `do_statusbar_backspace`）。
pub fn do_statusbar_backspace() {
    let mut answer = get_answer();
    let mut typing_x = get_typing_x();
    if typing_x > 0 {
        let was_x = typing_x;
        typing_x = chars::step_left(answer.as_bytes(), typing_x);
        answer.drain(typing_x..was_x);
    }
    set_answer(&answer);
    set_typing_x(typing_x);
}

/// 在回答中删除一个字符（对应 `do_statusbar_delete`）。
pub fn do_statusbar_delete() {
    let mut answer = get_answer();
    let typing_x = get_typing_x();
    if chars::byte_at(answer.as_bytes(), typing_x) != 0 {
        let charlen = chars::char_length(&answer.as_bytes()[typing_x..]);
        answer.drain(typing_x..typing_x + charlen);
        set_answer(&answer);
        /* 继续删除零宽字符。 */
        if chars::is_zerowidth(&answer.as_bytes()[typing_x..]) {
            do_statusbar_delete();
        }
    }
}

/// 删除光标之后的回答部分，或整个回答（对应 `lop_the_answer`）。
pub fn lop_the_answer() {
    let mut answer = get_answer();
    let mut typing_x = get_typing_x();
    if chars::byte_at(answer.as_bytes(), typing_x) == 0 {
        typing_x = 0;
    }
    answer.truncate(typing_x);
    set_answer(&answer);
    set_typing_x(typing_x);
}

/// 把当前回答（若有）复制到 cutbuffer（对应 `copy_the_answer`）。
pub fn copy_the_answer() {
    let answer = get_answer();
    if !answer.is_empty() {
        nano_free_lines(with_global(|g| g.cutbuffer.clone()));
        let newnode = make_new_node(None);
        newnode.borrow_mut().data = answer.clone();
        with_global_mut(|g| g.cutbuffer = Some(newnode));
        set_typing_x(0);
    }
}

fn nano_free_lines(src: Option<LineRef>) {
    crate::nano::free_lines(src);
}

/// 把 cutbuffer 的第一行粘贴到当前回答（对应 `paste_into_answer`）。
pub fn paste_into_answer() {
    let cutbuffer = with_global(|g| g.cutbuffer.clone());
    if let Some(cb) = cutbuffer {
        let pastelen = cb.borrow().data.len();
        let mut answer = get_answer();
        let typing_x = get_typing_x();
        answer.insert_str(typing_x, &cb.borrow().data);
        set_answer(&answer);
        set_typing_x(typing_x + pastelen);
    }
}

/// 把给定的短字节串插入回答（对应 `inject_into_answer`）。
pub fn inject_into_answer(burst: &[u8], count: usize) {
    /* 先把内嵌 NUL 编码为 0x0A。 */
    let mut burst_vec = burst[..count.min(burst.len())].to_vec();
    for b in &mut burst_vec {
        if *b == 0 {
            *b = b'\n';
        }
    }

    let mut answer = get_answer();
    let typing_x = get_typing_x();
    let s = String::from_utf8_lossy(&burst_vec).into_owned();
    answer.insert_str(typing_x, &s);
    set_answer(&answer);
    set_typing_x(typing_x + count);
}

/// 获取一个逐字按键并插入回答（对应 `do_statusbar_verbatim_input`）。
pub fn do_statusbar_verbatim_input() {
    let mut count = 1;
    let bytes = winio::get_verbatim_kbinput(&mut count);

    if 0 < count && count < 999 {
        inject_into_answer(&bytes, count);
    } else if count == 0 {
        winio::beep();
    }
}

/// 当输入是普通字节时加入输入缓冲，就绪时把收集的字节注入回答
/// （对应 `absorb_character`）。
pub fn absorb_character(input: i32, function: Option<FunctionId>) {
    thread_local! {
        static PUDDLE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    let currmenu = with_global(|g| g.currmenu);
    let filename_blank = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            of.borrow().filename.as_ref().map(|f| f.is_empty()).unwrap_or(true)
        }).unwrap_or(true)
    });

    /* 若不是命令，丢弃任何非普通字符字节。 */
    if function.is_none() {
        let meta_key = with_global(|g| g.meta_key);
        if (input < 0x20 && input != b'\t' as i32) || meta_key || input > 0xFF {
            winio::beep();
        } else if !ISSET(RESTRICTED) || currmenu != MWRITEFILE || filename_blank {
            PUDDLE.with(|p| p.borrow_mut().push(input as u8));
        }
    }

    /* 若有收集的字节且遇到命令或没有其他键码等待，注入回答。 */
    let waiting = winio::waiting_keycodes();
    let ready = PUDDLE.with(|p| !p.borrow().is_empty()) && (function.is_some() || waiting == 0);
    if ready {
        let bytes = PUDDLE.with(|p| std::mem::replace(&mut *p.borrow_mut(), Vec::new()));
        inject_into_answer(&bytes, bytes.len());
    }
}

/// 处理任何编辑快捷键，处理过返回 TRUE（对应 `handle_editing`）。
pub fn handle_editing(function: FunctionId) -> bool {
    let currmenu = with_global(|g| g.currmenu);
    let filename_blank = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            of.borrow().filename.as_ref().map(|f| f.is_empty()).unwrap_or(true)
        }).unwrap_or(true)
    });

    match function {
        FunctionId::DoLeft => do_statusbar_left(),
        FunctionId::DoRight => do_statusbar_right(),
        FunctionId::DoPrevWord => do_statusbar_prev_word(),
        FunctionId::DoNextWord => do_statusbar_next_word(),
        FunctionId::DoHome => do_statusbar_home(),
        FunctionId::DoEnd => do_statusbar_end(),
        /* 受限模式下"Write File"提示且文件名非空时，禁止输入和删除。 */
        _ if ISSET(RESTRICTED) && currmenu == MWRITEFILE && !filename_blank
            && (function == FunctionId::DoVerbatimInput
                || function == FunctionId::DoDelete
                || function == FunctionId::DoBackspace
                || function == FunctionId::DoCut
                || function == FunctionId::DoPaste) => {}
        FunctionId::DoVerbatimInput => do_statusbar_verbatim_input(),
        FunctionId::DoDelete => do_statusbar_delete(),
        FunctionId::DoBackspace => do_statusbar_backspace(),
        FunctionId::DoCut => lop_the_answer(),
        FunctionId::DoCopy => copy_the_answer(),
        FunctionId::DoPaste => {
            if with_global(|g| g.cutbuffer.is_some()) {
                paste_into_answer();
            }
        }
        _ => return false,
    }

    true
}

// ======================== 提示栏绘制（对应 prompt.c） ========================

/// 返回在状态栏显示的回答首字符的列号（对应 `get_statusbar_page_start`）。
pub fn get_statusbar_page_start(base: usize, column: usize) -> usize {
    let cols = with_global(|g| g.COLS);
    if column == base || column < cols - 1 {
        0
    } else if cols > base + 2 {
        column - base - 1 - (column - base - 1) % (cols - base - 2)
    } else {
        column - 2
    }
}

/// 重新初始化回答中的光标位置（对应 `put_cursor_at_end_of_answer`）。
pub fn put_cursor_at_end_of_answer() {
    set_typing_x(usize::MAX >> 1);
}

/// 重绘提示栏并把光标放到正确位置（对应 `draw_the_promptbar`）。
/// crossterm 架构下：渲染提示与回答的可见部分到状态栏。
pub fn draw_the_promptbar() {
    let prompt = get_prompt();
    let answer = get_answer();
    let typing_x = get_typing_x();
    let cols = with_global(|g| g.COLS);

    let base = utils::breadth(prompt.as_bytes()) + 2;
    let column = base + utils::wideness(answer.as_bytes(), typing_x);

    let the_page = get_statusbar_page_start(base, column);
    let end_page = get_statusbar_page_start(base, base + utils::breadth(answer.as_bytes()).saturating_sub(1));

    /* 构造显示字符串：prompt: <答案（分页）> */
    let mut display = format!("{}:{}", prompt, if the_page == 0 { ' ' } else { '<' });
    let span = cols.saturating_sub(base);
    let expanded = winio::display_string(answer.as_bytes(), the_page, span, false, true);
    display.push_str(&expanded);
    if the_page < end_page && base + utils::breadth(answer.as_bytes()) - the_page > cols {
        display.push('>');
    }

    /* 把提示行写到状态栏位置。 */
    with_global_mut(|g| g.lastmessage = MessageType::Vacuum);
    let mut stdout = std::io::stdout();
    let lines = with_global(|g| g.LINES);
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = crossterm::execute!(stdout, crossterm::cursor::MoveTo(0, status_row));
    use std::io::Write;
    let _ = write!(stdout, "{:width$}", display, width = cols);
    /* 把光标放到回答的输入位置（冒号之后，对应 C 的 wmove(footwin, 0, ...)）。 */
    let cursor_col = (base + utils::wideness(answer.as_bytes(), typing_x) - the_page) as u16;
    let _ = crossterm::execute!(stdout, crossterm::cursor::MoveTo(cursor_col, status_row));
    let _ = stdout.flush();
}

// ======================== 获取回答（对应 acquire_an_answer） ========================

/// 在状态栏提示处获取输入字符串（对应 `acquire_an_answer`）。
pub fn acquire_an_answer(
    actual: &mut i32,
    listed: &mut bool,
    mut history_list: Option<&mut LineRef>,
    refresh_func: Option<fn()>,
) -> Option<FunctionId> {
    let mut stored_string: Option<String> = None;
    let mut previous_was_tab = false;
    let mut fragment_length = 0;

    let mut function: Option<FunctionId> = None;
    let mut input: i32 = 0;

    if get_typing_x() > get_answer().len() {
        set_typing_x(get_answer().len());
    }

    loop {
        draw_the_promptbar();

        /* 读取一个按键。 */
        let input = winio::get_kbinput();

        /* 窗口大小改变时重新格式化提示。 */
        if input == THE_WINDOW_RESIZED {
            *actual = THE_WINDOW_RESIZED;
            return None;
        }

        /* 检查当前列表中的快捷键。 */
        let shortcut = global_find_shortcut(input);
        function = shortcut.map(|s| s.borrow().func);

        /* 当它是普通字符时，加入回答。 */
        absorb_character(input, function);

        if function == Some(FunctionId::DoCancel) || function == Some(FunctionId::DoEnter) {
            break;
        }

        if function == Some(FunctionId::DoTab) {
            if let Some(hl) = history_list.as_deref_mut() {
                if !previous_was_tab {
                    fragment_length = get_answer().len();
                }
                if fragment_length > 0 {
                    let mut answer = get_answer();
                    let mut here = hl.clone();
                    let completed = history::get_history_completion(&mut here, &mut answer, fragment_length);
                    *hl = here;
                    set_answer(&completed);
                    set_typing_x(get_answer().len());
                }
            } else {
                /* 允许文件名补全，但受限模式除外。 */
                let currmenu = with_global(|g| g.currmenu);
                if (currmenu & (MINSERTFILE | MWRITEFILE | MGOTODIR)) != 0 && !ISSET(RESTRICTED) {
                    input_tab(refresh_func, listed);
                }
            }
        } else if function == Some(FunctionId::GetOlderItem) && history_list.is_some() {
            let hl = history_list.as_ref().unwrap();
            /* 若这是第一次进入历史，从底部开始。 */
            if stored_string.is_none() {
                history::reset_history_pointer_for(hl);
            }
            /* 从底部上移时，记住当前回答。 */
            let is_at_bottom = {
                let next = { let r = hl.borrow(); r.next.clone() };
                next.is_none()
            };
            if is_at_bottom {
                stored_string = Some(get_answer());
            }
            /* 若有更旧项，移到它并复制其字符串。 */
            let prev = { let r = hl.borrow(); r.prev.clone() }.and_then(|w| w.upgrade());
            if let Some(p) = prev {
                **history_list.as_mut().unwrap() = p.clone();
                let data = p.borrow().data.clone();
                set_answer(&data);
                set_typing_x(get_answer().len());
            }
        } else if function == Some(FunctionId::GetNewerItem) && history_list.is_some() {
            let hl = history_list.as_mut().unwrap();
            /* 若有更新项，移到它并复制其字符串。 */
            let next = { let r = hl.borrow(); r.next.clone() };
            if let Some(n) = next {
                **hl = n.clone();
                let data = n.borrow().data.clone();
                set_answer(&data);
                set_typing_x(get_answer().len());
            }
            /* 位于历史列表底部时，恢复旧回答。 */
            let is_at_bottom = { let r = hl.borrow(); r.next.is_none() };
            if is_at_bottom && stored_string.is_some() && get_answer().is_empty() {
                if let Some(s) = &stored_string {
                    set_answer(s);
                    set_typing_x(get_answer().len());
                }
            }
        } else if function == Some(FunctionId::DoHelp) || function == Some(FunctionId::DoFullRefresh) {
            match function {
                Some(FunctionId::DoFullRefresh) => winio::full_refresh(),
                _ => crate::help::do_help(),
            }
        } else if function == Some(FunctionId::DoToggle) {
            /* 切换 NO_HELP。 */
            let shortcut = global_find_shortcut(input);
            let is_nohelp_toggle = shortcut.map(|s| s.borrow().toggle == NO_HELP as i32).unwrap_or(false);
            if is_nohelp_toggle {
                TOGGLE(NO_HELP);
                winio::window_init();
                with_global_mut(|g| g.focusing = false);
                if let Some(rf) = refresh_func {
                    rf();
                }
                winio::bottombars();
            }
        } else if function == Some(FunctionId::DoNothing) {
            /* 忽略。 */
        } else if function == Some(FunctionId::Implant) {
            /* 简化：不处理宏植入。 */
        } else if let Some(f) = function {
            if !handle_editing(f) {
                /* 允许的快捷键：运行它并完成。 */
                if !ISSET(VIEW_MODE) || !changes_something(f) {
                    /* 在 Execute 提示处运行工具时，暂存"回答"。 */
                    let currmenu = with_global(|g| g.currmenu);
                    if currmenu == MEXECUTE {
                        with_global_mut(|g| g.foretext = Some(get_answer()));
                    }
                    run_function(f);
                    break;
                } else {
                    winio::beep();
                }
            }
        }

        previous_was_tab = function == Some(FunctionId::DoTab);
    }

    /* 执行外部命令后，清除可能暂存的回答。 */
    let currmenu = with_global(|g| g.currmenu);
    if currmenu == MEXECUTE && function == Some(FunctionId::DoEnter) {
        with_global_mut(|g| g.foretext = None);
    }

    /* 若历史指针被移动，把它指回底部。 */
    if stored_string.is_some() {
        if let Some(hl) = history_list.as_deref_mut() {
            history::reset_history_pointer_for(hl);
        }
    }

    *actual = input;
    function
}

/// 按 keycode 在当前菜单查找快捷键（对应 `get_shortcut`）。
fn global_find_shortcut(keycode: i32) -> Option<KeyRef> {
    let currmenu = with_global(|g| g.currmenu);
    crate::global::find_shortcut(keycode, currmenu)
}

/// 函数是否改变内容（对应 global.c 的 `changes_something`）。
pub fn changes_something(_func: FunctionId) -> bool {
    /* 简化：多数编辑函数都改变内容。 */
    matches!(
        _func,
        FunctionId::DoCut | FunctionId::DoCopy | FunctionId::DoPaste
            | FunctionId::DoDelete | FunctionId::DoBackspace | FunctionId::DoEnter
            | FunctionId::DoCutToEof | FunctionId::DoIndent | FunctionId::DoUnindent
            | FunctionId::DoComment | FunctionId::DoUncomment
            | FunctionId::DoUndo | FunctionId::DoRedo
    )
}

/// 运行一个函数（对应 C 中 `function()` 调用）。
pub fn run_function(func: FunctionId) {
    use crate::cut;
    use crate::movement;
    use crate::nano;
    use crate::search;
    use crate::text;
    match func {
        FunctionId::DoCancel => text::do_cancel(),
        FunctionId::DoExit => text::do_exit(),
        FunctionId::DoHelp => crate::help::do_help(),
        FunctionId::DoLeft => movement::do_left(),
        FunctionId::DoRight => movement::do_right(),
        FunctionId::DoUp => movement::do_up(),
        FunctionId::DoDown => movement::do_down(),
        FunctionId::DoHome => movement::do_home(),
        FunctionId::DoEnd => movement::do_end(),
        FunctionId::DoPageUp => movement::do_page_up(),
        FunctionId::DoPageDown => movement::do_page_down(),
        FunctionId::DoDelete => cut::do_delete(),
        FunctionId::DoBackspace => cut::do_backspace(),
        FunctionId::DoEnter => text::do_enter(),
        FunctionId::DoTab => text::do_tab(),
        FunctionId::DoCut => cut::cut_text(),
        FunctionId::DoCopy => cut::copy_text(),
        FunctionId::DoPaste => cut::paste_text(),
        FunctionId::DoCutToEof => cut::cut_till_eof(),
        FunctionId::DoSearchForward | FunctionId::DoSearchBackward | FunctionId::DoFindNext
        | FunctionId::DoFindPrevious | FunctionId::DoReplace | FunctionId::DoGoToLine => {
            /* 搜索/替换/跳转在 search.rs 完整翻译后接入。 */
        }
        FunctionId::DoWriteOut | FunctionId::DoInsertFile | FunctionId::DoExecute => {}
        FunctionId::DoSpell => text::do_spell(),
        FunctionId::DoLinter => {}
        FunctionId::DoFormatter => text::do_formatter(),
        FunctionId::DoIndent => text::do_indent(),
        FunctionId::DoUnindent => text::do_unindent(),
        FunctionId::DoComment => text::do_comment(),
        FunctionId::DoUncomment => text::do_comment(),
        FunctionId::DoUndo => text::do_undo(),
        FunctionId::DoRedo => text::do_redo(),
        FunctionId::DoRefresh => text::do_refresh(),
        FunctionId::DoSuspend => text::do_suspend(),
        FunctionId::DoScrollUp => movement::do_scroll_up(),
        FunctionId::DoScrollDown => movement::do_scroll_down(),
        FunctionId::DoPrevBlock => movement::to_prev_block(),
        FunctionId::DoNextBlock => movement::to_next_block(),
        FunctionId::DoParaBegin => movement::to_para_begin(),
        FunctionId::DoParaEnd => movement::to_para_end(),
        FunctionId::DoFirstLine => movement::do_first_line(),
        FunctionId::DoLastLine => movement::do_last_line(),
        FunctionId::DoNextWord => movement::to_next_word(),
        FunctionId::DoPrevWord => movement::to_prev_word(),
        FunctionId::DoMark => text::do_mark(),
        FunctionId::DoAnchor => text::do_anchor(),
        FunctionId::DoFullRefresh => winio::full_refresh(),
        FunctionId::DoJustify => {}
        _ => {}
    }
    let _ = nano::set_modified;
}

// ======================== 提示问题（对应 do_prompt） ========================

/// 在状态栏上提问。返回 0 表示输入了文本，-1 表示取消，
/// -2 表示空白字符串，有效快捷键键码表示按下了相应快捷键
/// （对应 `do_prompt`）。
pub fn do_prompt(
    menu: i32,
    provided: &str,
    mut history_list: Option<&mut LineRef>,
    refresh_func: Option<fn()>,
    msg: &str,
) -> i32 {
    let mut function: Option<FunctionId> = None;
    let mut listed = false;
    let mut retval: i32 = 0;

    /* 保存当前状态栏 x 位置和提示。 */
    let was_typing_x = get_typing_x();
    let saved_prompt = with_global(|g| g.prompt.clone());

    winio::bottombars();
    with_global_mut(|g| g.currmenu = menu);

    if get_answer() != provided {
        set_answer(provided);
    }

    /* 重新格式化提示（窗口大小改变时重试）。 */
    loop {
        let cols = with_global(|g| g.COLS);
        let prompt_text = msg.to_string();
        /* 保留五列给冒号、尖括号与回答。 */
        let prompt_cut = utils::actual_x(prompt_text.as_bytes(), if cols < 5 { 0 } else { cols - 5 });
        let display_prompt = prompt_text[..prompt_cut].to_string();
        with_global_mut(|g| g.prompt = Some(display_prompt));
        with_global_mut(|g| g.lastmessage = MessageType::Vacuum);

        function = acquire_an_answer(&mut retval, &mut listed, history_list.as_deref_mut(), refresh_func);

        if retval != THE_WINDOW_RESIZED {
            break;
        }
    }

    /* 恢复之前的提示和可能的输入位置。 */
    with_global_mut(|g| g.prompt = saved_prompt);
    if function == Some(FunctionId::DoCancel) || function == Some(FunctionId::DoEnter)
        || function == Some(FunctionId::ToFirstFile) || function == Some(FunctionId::ToLastFile)
        || function == Some(FunctionId::DoFirstLine) || function == Some(FunctionId::DoLastLine)
    {
        set_typing_x(was_typing_x);
    }

    /* 为 Cancel 和 Enter 设置正确的返回值。 */
    if function == Some(FunctionId::DoCancel) {
        retval = -1;
    } else if function == Some(FunctionId::DoEnter) {
        retval = if get_answer().is_empty() { -2 } else { 0 };
    }

    if with_global(|g| g.lastmessage == MessageType::Vacuum) {
        winio::wipe_statusbar();
    }

    /* 若仍列出可能的文件名补全，清除它们。 */
    if listed {
        if let Some(rf) = refresh_func {
            rf();
        }
    }

    retval
}

// ======================== 文件补全（对应 files.c 的 input_tab） ========================

/// 文件名补全（对应 `input_tab`；当前为简化实现，browser 完善后扩展）。
fn input_tab(_refresh_func: Option<fn()>, _listed: &mut bool) {
    // 简化：不执行文件补全
}

// ======================== Yes/No 询问（对应 ask_user） ========================

const UNDECIDED: i32 = -2;

/// 在状态栏上询问简单的 Yes/No（可选 All）问题并返回选择
/// —— YES 或 NO 或 ALL 或 CANCEL（对应 `ask_user`）。
pub fn ask_user(withall: bool, question: &str) -> i32 {
    let mut choice = UNDECIDED;
    let mut _width = 16;
    let yesstr = "Yy";
    let nostr = "Nn";
    let allstr = "Aa";

    while choice == UNDECIDED {
        let kbinput: i32;

        if !ISSET(NO_HELP) {
            let cols = with_global(|g| g.COLS);
            if cols < 32 {
                _width = cols / 2;
            }
            /* 简化：快捷键列表的显示由 winio 渲染层处理。 */
        }

        /* 显示问题。 */
        let cols = with_global(|g| g.COLS);
        let truncated: String = question.chars().take(cols.saturating_sub(1)).collect();
        with_global_mut(|g| g.currmenu = MYESNO);
        winio::statusline(MessageType::Info, &format!("{:width$}", truncated, width = cols));

        /* 等待按键。 */
        kbinput = winio::get_kbinput();

        if kbinput == THE_WINDOW_RESIZED {
            continue;
        }

        /* 检查输入的字母是否在 Yes/No/All 字符串中。 */
        let letter = (kbinput & 0xFF) as u8 as char;
        if yesstr.contains(letter) {
            choice = YES;
        } else if nostr.contains(letter) {
            choice = NO;
        } else if withall && allstr.contains(letter) {
            choice = ALL;
        }

        if choice != UNDECIDED {
            break;
        }

        let shortcut = global_find_shortcut(kbinput);
        let function = shortcut.as_ref().map(|s| s.borrow().func);

        if function == Some(FunctionId::DoCancel) {
            choice = CANCEL;
        } else if function == Some(FunctionId::DoFullRefresh) {
            winio::full_refresh();
        } else if function == Some(FunctionId::DoToggle) {
            let is_nohelp = shortcut.map(|s| s.borrow().toggle == NO_HELP as i32).unwrap_or(false);
            if is_nohelp {
                TOGGLE(NO_HELP);
                winio::window_init();
                winio::titlebar(None);
                with_global_mut(|g| g.focusing = false);
                winio::edit_refresh();
                with_global_mut(|g| g.focusing = true);
            }
        }
        /* 把 ^N 解释为"No"，^Q 或 ^X 也解释为"No"。 */
        else if kbinput == 0x0E
            || (kbinput == 0x11 && !ISSET(MODERN_BINDINGS))
            || (kbinput == 0x18 && ISSET(MODERN_BINDINGS))
        {
            choice = NO;
            if kbinput != 0x0E {
                with_global_mut(|g| g.final_status = 2);
            }
        }
        /* 把 ^Y 解释为"Yes"，^A 解释为"All"。 */
        else if kbinput == 0x19 {
            choice = YES;
        } else if kbinput == 0x01 && withall {
            choice = ALL;
        } else {
            winio::beep();
        }
    }

    choice
}