/**************************************************************************
 * movement.rs  --  GNU nano 光标移动（对应 move.c）
 * 版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 光标移动与滚动，完整移植自 `move.c`。
//!
//! 转换说明：
//! - `linestruct *` → `Rc<RefCell<LineStruct>>`；
//! - 所有跨函数调用遵循"先取数据、释放借用再调用"的模式；
//! - `edit_redraw`/`edit_scroll`/`update_line` 等在 crossterm 架构下
//!   以等价行为实现（见 [`crate::winio`]）；
//! - `do_next_word` 的 `after_ends` 参数保留 C 语义。

use crate::definitions::*;
use crate::chars;
use crate::text;
use crate::utils;
use crate::winio;
use std::rc::Rc;

/// 获取当前打开的缓冲区引用（克隆 Rc，释放全局借用）。
fn openfile_ref() -> OpenFileRef {
    with_global(|g| g.openfile.as_ref().expect("no open file").clone())
}

/// 获取 tabsize 全局。
fn tabsize_value() -> usize {
    with_global(|g| g.tabsize)
}

/// 获取 editwincols 全局。
fn editwincols_value() -> usize {
    with_global(|g| g.editwincols)
}

/// 获取 editwinrows 全局。
fn editwinrows_value() -> i32 {
    with_global(|g| g.editwinrows)
}

// ======================== 首行/末行（对应 move.c） ========================

/// 移动到文件首行（对应 `to_first_line`）。
pub fn to_first_line() {
    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();
    of_ref.current = of_ref.filetop.clone();
    of_ref.current_x = 0;
    of_ref.placewewant = 0;
    with_global_mut(|g| g.refresh_needed = true);
}

/// 移动到文件末行（对应 `to_last_line`）。
pub fn to_last_line() {
    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();
    of_ref.current = of_ref.filebot.clone();
    let inhelp = with_global(|g| g.inhelp);
    of_ref.current_x = if inhelp {
        0
    } else {
        of_ref.current.as_ref().map(|c| c.borrow().data.len()).unwrap_or(0)
    };
    of_ref.placewewant = utils::xplustabs();

    /* 把屏幕最后一行设为光标的行目标。 */
    let rows = editwinrows_value();
    of_ref.cursor_row = (rows - 1) as isize;

    with_global_mut(|g| {
        g.refresh_needed = true;
        g.recook |= g.perturbed;
        g.focusing = false;
    });
}

// ======================== 页与目标列（对应 move.c） ========================

/// 确定当前的软换行块与实际目标列（对应 `get_edge_and_target`）。
fn get_edge_and_target(leftedge: &mut usize, target_column: &mut usize) {
    if ISSET(SOFTWRAP) {
        let shim = editwincols_value() * (1 + (tabsize_value() / editwincols_value()));
        let of = openfile_ref();
        let current = of.borrow().current.clone().unwrap();
        *leftedge = winio::leftedge_for(utils::xplustabs(), &current);
        let placewewant = of.borrow().placewewant;
        *target_column = (placewewant + shim - *leftedge) % editwincols_value();
    } else {
        *leftedge = 0;
        let of = openfile_ref();
        *target_column = of.borrow().placewewant;
    }
}

/// 返回 line->data 中与给定列（在给定 leftedge 开始的块上）对应的索引。
/// 若目标列落在制表符上，向前移动时防止光标回落一行，
/// 向后移动时防止跳过一行：递增索引（对应 `proper_x`）。
fn proper_x(line: &LineRef, leftedge: &mut usize, forward: bool, column: usize, shifted: &mut bool) -> usize {
    let data = line.borrow().data.clone();
    let mut index = utils::actual_x(data.as_bytes(), column);

    if ISSET(SOFTWRAP) {
        let byte = data.as_bytes().get(index).copied().unwrap_or(0);
        if byte == b'\t'
            && ((forward && utils::wideness(data.as_bytes(), index) < *leftedge)
                || (!forward
                    && column / tabsize_value() == ((*leftedge).saturating_sub(1)) / tabsize_value()
                    && column / tabsize_value()
                        < (*leftedge + editwincols_value().saturating_sub(1)) / tabsize_value()))
        {
            index += 1;
            *shifted = true;
        }
    }

    if ISSET(SOFTWRAP) {
        *leftedge = winio::leftedge_for(utils::wideness(data.as_bytes(), index), line);
    }

    index
}

/// 落在跨行边界的制表符中间时，调整 current_x 与 placewewant
/// （对应 `set_proper_index_and_pww`）。
fn set_proper_index_and_pww(leftedge: &mut usize, target: usize, forward: bool) {
    let was_edge = *leftedge;
    let mut shifted = false;

    let of = openfile_ref();
    let current = of.borrow().current.clone().unwrap();
    let mut le = *leftedge;
    let mut sh = false;
    let idx = proper_x(&current, &mut le, forward, winio::actual_last_column(*leftedge, target), &mut sh);
    of.borrow_mut().current_x = idx;
    *leftedge = le;
    shifted = sh;

    /* 若索引已递增，尝试转到目标列。 */
    if shifted || *leftedge < was_edge {
        let mut le = *leftedge;
        let mut sh = false;
        let idx = proper_x(&current, &mut le, forward, winio::actual_last_column(*leftedge, target), &mut sh);
        of.borrow_mut().current_x = idx;
        *leftedge = le;
    }

    of.borrow_mut().placewewant = *leftedge + target;
}

// ======================== 翻页（对应 move.c） ========================

/// 向上移动几乎一屏（对应 `do_page_up`）。
pub fn do_page_up() {
    let mustmove = if editwinrows_value() < 3 { 1 } else { editwinrows_value() - 2 };
    let mut leftedge = 0;
    let mut target_column = 0;

    /* 非平滑滚动模式下，把光标放到编辑窗口顶行的开头。 */
    if ISSET(JUMPY_SCROLLING) {
        let of = openfile_ref();
        let mut of_ref = of.borrow_mut();
        of_ref.current = of_ref.edittop.clone();
        leftedge = of_ref.firstcolumn;
        of_ref.cursor_row = 0;
        target_column = 0;
    } else {
        get_edge_and_target(&mut leftedge, &mut target_column);
    }

    /* 上移所需行数或块数；若不能，则位于文件顶部。 */
    let of = openfile_ref();
    let mut current = of.borrow().current.clone().unwrap();
    if winio::go_back_chunks(mustmove, &mut current, &mut leftedge) > 0 {
        to_first_line();
        return;
    }
    of.borrow_mut().current = Some(current);

    set_proper_index_and_pww(&mut leftedge, target_column, false);

    /* 移动视口使光标保持不动（如可能）。 */
    winio::adjust_viewport(UpdateType::Stationary);
    with_global_mut(|g| g.refresh_needed = true);
}

/// 向下移动几乎一屏（对应 `do_page_down`）。
pub fn do_page_down() {
    let mustmove = if editwinrows_value() < 3 { 1 } else { editwinrows_value() - 2 };
    let mut leftedge = 0;
    let mut target_column = 0;

    if ISSET(JUMPY_SCROLLING) {
        let of = openfile_ref();
        let mut of_ref = of.borrow_mut();
        of_ref.current = of_ref.edittop.clone();
        leftedge = of_ref.firstcolumn;
        of_ref.cursor_row = 0;
        target_column = 0;
    } else {
        get_edge_and_target(&mut leftedge, &mut target_column);
    }

    let of = openfile_ref();
    let mut current = of.borrow().current.clone().unwrap();
    if winio::go_forward_chunks(mustmove, &mut current, &mut leftedge) > 0 {
        to_last_line();
        return;
    }
    of.borrow_mut().current = Some(current);

    set_proper_index_and_pww(&mut leftedge, target_column, true);

    winio::adjust_viewport(UpdateType::Stationary);
    with_global_mut(|g| g.refresh_needed = true);
}

/// 把光标放到视口第一行（对应 `to_top_row`）。
pub fn to_top_row() {
    let mut leftedge = 0;
    let mut offset = 0;

    get_edge_and_target(&mut leftedge, &mut offset);

    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();
    of_ref.current = of_ref.edittop.clone();
    leftedge = of_ref.firstcolumn;

    drop(of_ref);
    set_proper_index_and_pww(&mut leftedge, offset, false);

    let of = openfile_ref();
    let marked = of.borrow().mark.is_some();
    with_global_mut(|g| g.refresh_needed = marked);
}

/// 把光标放到视口最后一行（如可能）（对应 `to_bottom_row`）。
pub fn to_bottom_row() {
    let mut leftedge = 0;
    let mut offset = 0;

    get_edge_and_target(&mut leftedge, &mut offset);

    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();
    of_ref.current = of_ref.edittop.clone();
    leftedge = of_ref.firstcolumn;

    let mut cur = of_ref.current.clone().unwrap();
    let rows = editwinrows_value();
    winio::go_forward_chunks(rows - 1, &mut cur, &mut leftedge);
    of_ref.current = Some(cur);
    drop(of_ref);
    set_proper_index_and_pww(&mut leftedge, offset, true);

    let of = openfile_ref();
    let marked = of.borrow().mark.is_some();
    with_global_mut(|g| g.refresh_needed = marked);
}

/// 依次把光标行居中、置顶、置底（对应 `do_cycle`）。
pub fn do_cycle() {
    let aim = with_global(|g| g.cycling_aim);
    if aim == 0 {
        winio::adjust_viewport(UpdateType::Centering);
    } else {
        let of = openfile_ref();
        let mut of_ref = of.borrow_mut();
        let rows = editwinrows_value();
        of_ref.cursor_row = if aim == 1 { 0 } else { (rows - 1) as isize };
        drop(of_ref);
        winio::adjust_viewport(UpdateType::Stationary);
    }

    with_global_mut(|g| g.cycling_aim = (aim + 1) % 3);

    winio::draw_all_subwindows();
    winio::full_refresh();
}

/// 把光标行滚动到屏幕中央（对应 `do_center`）。
pub fn do_center() {
    winio::adjust_viewport(UpdateType::Centering);
    winio::draw_all_subwindows();
    winio::full_refresh();
}

// ======================== 段落移动（对应 move.c） ========================

/// 移动到当前行之前第一个段落的开头（对应 `do_para_begin`）。
pub fn do_para_begin(line: &mut LineRef) {
    let prev = { let r = line.borrow(); r.prev.clone() };
    if prev.is_some() {
        *line = prev.and_then(|w| w.upgrade()).unwrap();
    }
    while !text::begpar(line, 0) {
        let prev = { let r = line.borrow(); r.prev.clone() }.and_then(|w| w.upgrade()).unwrap();
        *line = prev;
    }
}

/// 向下移动到找到的第一个段落的最后一行（对应 `do_para_end`）。
pub fn do_para_end(line: &mut LineRef) {
    loop {
        let next = { let r = line.borrow(); r.next.clone() };
        let inpar = text::inpar(line);
        match next {
            Some(n) if !inpar => *line = n,
            _ => break,
        }
    }

    loop {
        let (next, inpar_next, begpar_next) = {
            let r = line.borrow();
            let n = r.next.clone();
            let inp = n.as_ref().map(|n| text::inpar(n)).unwrap_or(false);
            let bgp = n.as_ref().map(|n| text::begpar(n, 0)).unwrap_or(false);
            (n, inp, bgp)
        };
        match next {
            Some(n) if inpar_next && !begpar_next => *line = n,
            _ => break,
        }
    }
}

/// 移动到当前行之前的第一个段落开头（对应 `to_para_begin`）。
pub fn to_para_begin() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut cur = was_current.clone();
    do_para_begin(&mut cur);
    {
        let mut of_ref = of.borrow_mut();
        of_ref.current = Some(cur);
        of_ref.current_x = 0;
    }
    winio::edit_redraw(&was_current, UpdateType::Centering);
}

/// 移动到找到的段落末尾之后（对应 `to_para_end`）。
pub fn to_para_end() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut cur = was_current.clone();
    do_para_end(&mut cur);

    /* 可能时越过段落的最后一行；否则移到行尾。 */
    {
        let mut of_ref = of.borrow_mut();
        let next = { let r = cur.borrow(); r.next.clone() };
        match next {
            Some(n) => {
                of_ref.current = Some(n);
                of_ref.current_x = 0;
            }
            None => {
                let len = cur.borrow().data.len();
                of_ref.current = Some(cur);
                of_ref.current_x = len;
            }
        }
    }

    winio::edit_redraw(&was_current, UpdateType::Centering);
    with_global_mut(|g| g.recook |= g.perturbed);
}

/// 移动到前一个文本块（对应 `to_prev_block`）。
pub fn to_prev_block() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut is_text = false;
    let mut seen_text = false;

    /* 在若干非空行之后向后跳过直到第一个空行。 */
    loop {
        let has_prev = {
            let cur = of.borrow().current.clone().unwrap();
            let r = cur.borrow();
            r.prev.is_some()
        };
        if !has_prev || (seen_text && !is_text) {
            break;
        }
        let mut of_ref = of.borrow_mut();
        let cur = of_ref.current.clone().unwrap();
        let prev = { let r = cur.borrow(); r.prev.clone() }.and_then(|w| w.upgrade()).unwrap();
        of_ref.current = Some(prev.clone());
        let data = prev.borrow().data.clone();
        is_text = !chars::white_string(data.as_bytes());
        seen_text = seen_text || is_text;
    }

    /* 若越过了文本但本行是空行，再前进一步。 */
    if seen_text {
        let of = openfile_ref();
        let mut of_ref = of.borrow_mut();
        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        let next = { let r = cur.borrow(); r.next.clone() };
        if let Some(n) = next {
            if chars::white_string(data.as_bytes()) {
                of_ref.current = Some(n);
            }
        }
    }

    of.borrow_mut().current_x = 0;
    winio::edit_redraw(&was_current, UpdateType::Centering);
}

/// 移动到下一个文本块（对应 `to_next_block`）。
pub fn to_next_block() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut is_white = {
        let data = was_current.borrow().data.clone();
        chars::white_string(data.as_bytes())
    };
    let mut seen_white = is_white;

    /* 在若干空行之后向前跳过直到第一个非空行。 */
    loop {
        let has_next = {
            let cur = of.borrow().current.clone().unwrap();
            let r = cur.borrow();
            r.next.is_some()
        };
        if !has_next || (seen_white && !is_white) {
            break;
        }
        let mut of_ref = of.borrow_mut();
        let cur = of_ref.current.clone().unwrap();
        let next = { let r = cur.borrow(); r.next.clone() }.unwrap();
        of_ref.current = Some(next.clone());
        let data = next.borrow().data.clone();
        is_white = chars::white_string(data.as_bytes());
        seen_white = seen_white || is_white;
    }

    of.borrow_mut().current_x = 0;
    winio::edit_redraw(&was_current, UpdateType::Centering);
    with_global_mut(|g| g.recook |= g.perturbed);
}

// ======================== 单词移动（对应 move.c） ========================

/// 移动到上一个单词（对应 `do_prev_word`）。
pub fn do_prev_word() {
    let punctuation_as_letters = ISSET(WORD_BOUNDS);
    let mut seen_a_word = false;
    let mut step_forward = false;

    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();

    /* 向后移动直到越过一个单词的开头。 */
    loop {
        /* 若在行首，移动到前一行的末尾。 */
        if of_ref.current_x == 0 {
            let cur = of_ref.current.clone().unwrap();
            let prev = { let r = cur.borrow(); r.prev.clone() };
            match prev.and_then(|w| w.upgrade()) {
                None => break,
                Some(p) => {
                    of_ref.current = Some(p.clone());
                    let len = p.borrow().data.len();
                    of_ref.current_x = len;
                }
            }
        }

        /* 后退一个字符。 */
        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        of_ref.current_x = chars::step_left(data.as_bytes(), of_ref.current_x);

        let is_word = chars::is_word_char(&data.as_bytes()[of_ref.current_x..], punctuation_as_letters);
        if is_word {
            seen_a_word = true;
            /* 若现在位于行首，这肯定是单词开头。 */
            if of_ref.current_x == 0 {
                break;
            }
        } else if chars::is_zerowidth(&data.as_bytes()[of_ref.current_x..]) {
            /* 跳过零宽字符。 */
        } else if seen_a_word {
            /* 这是空白：已越过单词开头。 */
            step_forward = true;
            break;
        }
    }

    if step_forward {
        /* 再前进一个字符以停在单词开头。 */
        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        of_ref.current_x = chars::step_right(data.as_bytes(), of_ref.current_x);
    }
}

/// 移动到下一个单词。after_ends 为 TRUE 时停在单词末尾而非开头。
/// 若从单词上开始移动则返回 TRUE（对应 `do_next_word`）。
pub fn do_next_word(after_ends: bool) -> bool {
    let punctuation_as_letters = ISSET(WORD_BOUNDS);
    let (started_on_word, current_x0) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let cur = of.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        (chars::is_word_char(&data.as_bytes()[of.current_x..], punctuation_as_letters), of.current_x)
    });
    let _ = current_x0;
    let mut seen_space = !started_on_word;
    let mut seen_word = started_on_word;

    let of = openfile_ref();
    let mut of_ref = of.borrow_mut();

    /* 向前移动直到到达单词开头。 */
    loop {
        /* 若在行尾，移动到下一行的开头。 */
        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        let at_eol = data.as_bytes().get(of_ref.current_x).copied().unwrap_or(0) == 0;
        if at_eol {
            /* 位于文件末尾时停止。 */
            let next = { let r = cur.borrow(); r.next.clone() };
            match next {
                None => break,
                Some(n) => {
                    of_ref.current = Some(n);
                    of_ref.current_x = 0;
                    seen_space = true;
                }
            }
        } else {
            of_ref.current_x = chars::step_right(data.as_bytes(), of_ref.current_x);
        }

        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();

        if after_ends {
            /* 若是单词字符继续；否则是分隔符，若已见单词则是单词末尾。 */
            if chars::is_word_char(&data.as_bytes()[of_ref.current_x..], punctuation_as_letters) {
                seen_word = true;
            } else if chars::is_zerowidth(&data.as_bytes()[of_ref.current_x..]) {
                /* 跳过零宽字符。 */
            } else if seen_word {
                break;
            }
        } else {
            if chars::is_zerowidth(&data.as_bytes()[of_ref.current_x..]) {
                /* 跳过零宽字符。 */
            } else if !chars::is_word_char(&data.as_bytes()[of_ref.current_x..], punctuation_as_letters) {
                seen_space = true;
            } else if seen_space {
                break;
            }
        }
    }

    started_on_word
}

/// 移动到文件中的上一个单词并刷新屏幕（对应 `to_prev_word`）。
pub fn to_prev_word() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    do_prev_word();
    winio::edit_redraw(&was_current, UpdateType::Flowing);
}

/// 移动到文件中的下一个单词并刷新屏幕（对应 `to_next_word`）。
pub fn to_next_word() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    do_next_word(ISSET(AFTER_ENDS));
    winio::edit_redraw(&was_current, UpdateType::Flowing);
}

// ======================== 行首/行尾（对应 move.c） ========================

/// 移动到当前行（或软换行块）的开头。启用时执行 smart home。
/// 软换行时若已在块首则移到整行开头（对应 `do_home`）。
pub fn do_home() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let was_column = utils::xplustabs();
    let mut moved_off_chunk = true;
    let mut moved = false;
    let mut leftedge = 0;
    let mut left_x = 0;

    if ISSET(SOFTWRAP) {
        leftedge = winio::leftedge_for(was_column, &was_current);
        let mut le = leftedge;
        let mut sh = false;
        left_x = proper_x(&was_current, &mut le, false, leftedge, &mut sh);
        leftedge = le;
    }

    if ISSET(SMART_HOME) {
        let data = was_current.borrow().data.clone();
        let indent_x = text::indent_length(data.as_bytes());
        let has_text = data.as_bytes().get(indent_x).copied().unwrap_or(0) != 0;

        if has_text {
            let of_ref = of.borrow();
            /* 若恰好位于缩进处，完全 home；否则不在软换行或不在首个
             * 非空块之后时，移到首个非空字符。 */
            if of_ref.current_x == indent_x {
                of.borrow_mut().current_x = 0;
                moved = true;
            } else if left_x <= indent_x {
                of.borrow_mut().current_x = indent_x;
                moved = true;
            }
        }
    }

    if !moved && ISSET(SOFTWRAP) {
        let of_ref = of.borrow();
        /* 若已在屏幕左边缘，完全 home；否则移到左边缘。 */
        if of_ref.current_x == left_x {
            of.borrow_mut().current_x = 0;
        } else {
            of.borrow_mut().current_x = left_x;
            of.borrow_mut().placewewant = leftedge;
            moved_off_chunk = false;
        }
    } else if !moved {
        of.borrow_mut().current_x = 0;
    }

    if moved_off_chunk {
        of.borrow_mut().placewewant = utils::xplustabs();
    }

    /* 若改变块可能离屏；否则在标记开启或"页"改变时更新当前行。 */
    if ISSET(SOFTWRAP) && moved_off_chunk {
        winio::edit_redraw(&was_current, UpdateType::Flowing);
    } else if winio::line_needs_update(was_column, {
        let of_ref = of.borrow();
        of_ref.placewewant
    }) {
        let of_ref = of.borrow();
        let cur = of_ref.current.clone().unwrap();
        let x = of_ref.current_x;
        winio::update_line(&cur, x);
    }
}

/// 移动到当前行（或软换行块）的末尾。软换行时若已在块末则移到整行末尾
/// （对应 `do_end`）。
pub fn do_end() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let was_column = utils::xplustabs();
    let line_len = was_current.borrow().data.len();
    let mut moved_off_chunk = true;

    if ISSET(SOFTWRAP) {
        let mut kickoff = true;
        let mut last_chunk = false;
        let mut leftedge = winio::leftedge_for(was_column, &was_current);
        let data = was_current.borrow().data.clone();
        let mut rightedge = winio::get_softwrap_breakpoint(data.as_bytes(), leftedge, &mut kickoff, &mut last_chunk);

        /* 若在最后一块上，已在行尾；否则在行末后一列。
         * 后退一列可能落在多列字符中间，但 actual_x() 会修正。 */
        if !last_chunk {
            rightedge = rightedge.saturating_sub(1);
        }

        let right_x = utils::actual_x(data.as_bytes(), rightedge);
        let of_ref = of.borrow();
        /* 若已在屏幕右边缘，完全移到行尾；否则移到右边缘。 */
        if of_ref.current_x == right_x {
            of.borrow_mut().current_x = line_len;
        } else {
            of.borrow_mut().current_x = right_x;
            of.borrow_mut().placewewant = rightedge;
            moved_off_chunk = false;
        }
    } else {
        of.borrow_mut().current_x = line_len;
    }

    if moved_off_chunk {
        of.borrow_mut().placewewant = utils::xplustabs();
    }

    if ISSET(SOFTWRAP) && moved_off_chunk {
        winio::edit_redraw(&was_current, UpdateType::Flowing);
    } else if winio::line_needs_update(was_column, {
        let of_ref = of.borrow();
        of_ref.placewewant
    }) {
        let of_ref = of.borrow();
        let cur = of_ref.current.clone().unwrap();
        let x = of_ref.current_x;
        winio::update_line(&cur, x);
    }
}

// ======================== 上下移动（对应 move.c） ========================

/// 把光标移动到前一行或前一块（对应 `do_up`）。
pub fn do_up() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut leftedge = 0;
    let mut target_column = 0;

    get_edge_and_target(&mut leftedge, &mut target_column);

    /* 若不能上移一行或一块，则位于文件顶部。 */
    let of = openfile_ref();
    let mut cur = of.borrow().current.clone().unwrap();
    if winio::go_back_chunks(1, &mut cur, &mut leftedge) > 0 {
        return;
    }
    of.borrow_mut().current = Some(cur);

    set_proper_index_and_pww(&mut leftedge, target_column, false);

    let (cursor_row, jumpy) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.cursor_row, g.flags.isset(JUMPY_SCROLLING))
    });
    let tabsize = tabsize_value();
    let editwincols = editwincols_value();
    if cursor_row == 0 && !jumpy && (tabsize < editwincols || !ISSET(SOFTWRAP)) {
        winio::edit_scroll(winio::ScrollDirection::Backward);
    } else {
        winio::edit_redraw(&was_current, UpdateType::Flowing);
    }

    /* <Up> 不应改变 placewewant，恢复它。 */
    let of = openfile_ref();
    of.borrow_mut().placewewant = leftedge + target_column;
}

/// 把光标移动到下一行或下一块（对应 `do_down`）。
pub fn do_down() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();
    let mut leftedge = 0;
    let mut target_column = 0;

    get_edge_and_target(&mut leftedge, &mut target_column);

    let of = openfile_ref();
    let mut cur = of.borrow().current.clone().unwrap();
    if winio::go_forward_chunks(1, &mut cur, &mut leftedge) > 0 {
        return;
    }
    of.borrow_mut().current = Some(cur);

    set_proper_index_and_pww(&mut leftedge, target_column, true);

    let (cursor_row, jumpy) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.cursor_row, g.flags.isset(JUMPY_SCROLLING))
    });
    let rows = editwinrows_value();
    let tabsize = tabsize_value();
    let editwincols = editwincols_value();
    if cursor_row == (rows - 1) as isize && !jumpy && (tabsize < editwincols || !ISSET(SOFTWRAP)) {
        winio::edit_scroll(winio::ScrollDirection::Forward);
    } else {
        winio::edit_redraw(&was_current, UpdateType::Flowing);
    }

    let of = openfile_ref();
    of.borrow_mut().placewewant = leftedge + target_column;
}

// ======================== 滚动（对应 move.c） ========================

/// 不移动光标文本位置地向上滚动一行或一块（对应 `do_scroll_up`）。
pub fn do_scroll_up() {
    let (top_at_filetop, firstcolumn_zero) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let no_prev = of.edittop.as_ref().map(|e| e.borrow().prev.is_none()).unwrap_or(false);
        (no_prev, of.firstcolumn == 0)
    });

    /* 当文件顶部在屏幕上时，无法滚动。 */
    if top_at_filetop && firstcolumn_zero {
        return;
    }

    let cursor_row = with_global(|g| g.openfile.as_ref().unwrap().borrow().cursor_row);
    let rows = editwinrows_value();
    if cursor_row == (rows - 1) as isize {
        do_up();
    }

    if rows > 1 {
        winio::edit_scroll(winio::ScrollDirection::Backward);
    }
}

/// 不移动光标文本位置地向下滚动一行或一块（对应 `do_scroll_down`）。
pub fn do_scroll_down() {
    let cursor_row = with_global(|g| g.openfile.as_ref().unwrap().borrow().cursor_row);
    if cursor_row == 0 {
        do_down();
    }

    let rows = editwinrows_value();
    let can_scroll = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let edittop = of.edittop.clone().unwrap();
        let has_next = edittop.borrow().next.is_some();
        let extra = if ISSET(SOFTWRAP) {
            winio::extra_chunks_in(&edittop)
                > winio::chunk_for(of.firstcolumn, &edittop)
        } else {
            false
        };
        has_next || extra
    });

    if rows > 1 && can_scroll {
        winio::edit_scroll(winio::ScrollDirection::Forward);
    }
}

// ======================== 左右移动（对应 move.c） ========================

/// 向左移动一个字符（对应 `do_left`）。
pub fn do_left() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();

    {
        let mut of_ref = of.borrow_mut();
        if of_ref.current_x > 0 {
            let cur = of_ref.current.clone().unwrap();
            let data = cur.borrow().data.clone();
            of_ref.current_x = chars::step_left(data.as_bytes(), of_ref.current_x);
            /* 跳过零宽字符。 */
            while of_ref.current_x > 0 {
                let cur = of_ref.current.clone().unwrap();
                let data = cur.borrow().data.clone();
                if !chars::is_zerowidth(&data.as_bytes()[of_ref.current_x..]) {
                    break;
                }
                of_ref.current_x = chars::step_left(data.as_bytes(), of_ref.current_x);
            }
        } else {
            let is_filetop = {
                let of_ref2 = &of_ref;
                of_ref2.filetop.as_ref().map(|t| {
                    of_ref2.current.as_ref().map(|c| Rc::ptr_eq(t, c)).unwrap_or(false)
                }).unwrap_or(false)
            };
            if !is_filetop {
                let cur = of_ref.current.clone().unwrap();
                let prev = { let r = cur.borrow(); r.prev.clone() }.and_then(|w| w.upgrade()).unwrap();
                of_ref.current = Some(prev.clone());
                let len = prev.borrow().data.len();
                of_ref.current_x = len;
            }
        }
    }

    winio::edit_redraw(&was_current, UpdateType::Flowing);
}

/// 向右移动一个字符（对应 `do_right`）。
pub fn do_right() {
    let of = openfile_ref();
    let was_current = of.borrow().current.clone().unwrap();

    {
        let mut of_ref = of.borrow_mut();
        let cur = of_ref.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        let at_eol = data.as_bytes().get(of_ref.current_x).copied().unwrap_or(0) == 0;
        if !at_eol {
            of_ref.current_x = chars::step_right(data.as_bytes(), of_ref.current_x);
            /* 跳过零宽字符。 */
            loop {
                let cur = of_ref.current.clone().unwrap();
                let data = cur.borrow().data.clone();
                let has_next_char = data.as_bytes().get(of_ref.current_x).copied().unwrap_or(0) != 0;
                if !has_next_char || !chars::is_zerowidth(&data.as_bytes()[of_ref.current_x..]) {
                    break;
                }
                of_ref.current_x = chars::step_right(data.as_bytes(), of_ref.current_x);
            }
        } else {
            let is_filebot = {
                of_ref.filebot.as_ref().map(|b| {
                    of_ref.current.as_ref().map(|c| Rc::ptr_eq(b, c)).unwrap_or(false)
                }).unwrap_or(false)
            };
            if !is_filebot {
                let cur = of_ref.current.clone().unwrap();
                let next = { let r = cur.borrow(); r.next.clone() }.unwrap();
                of_ref.current = Some(next);
                of_ref.current_x = 0;
            }
        }
    }

    winio::edit_redraw(&was_current, UpdateType::Flowing);
}

/// 水平向左滚动视口（对应 `do_scroll_left`）。
pub fn do_scroll_left() {
    if ISSET(SOFTWRAP) || ISSET(SOLO_SIDESCROLL) {
        let opt = if ISSET(SOFTWRAP) { "--softwrap" } else { "--solo" };
        winio::statusline(MessageType::Ahem, &format!("Not possible with '{}'", opt));
        return;
    }

    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        let tabsize = g.tabsize;
        of.brink -= if of.brink < tabsize {
            of.brink
        } else if tabsize < 2 {
            2
        } else {
            tabsize
        };

        let cur = of.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        let frame_x = utils::actual_x(data.as_bytes(), of.brink + g.editwincols - CUSHION - 1);

        if of.current_x > frame_x {
            of.current_x = frame_x;
            of.placewewant = utils::xplustabs();
        }
        g.refresh_needed = true;
    });
}

/// 水平向右滚动视口（对应 `do_scroll_right`）。
pub fn do_scroll_right() {
    if ISSET(SOFTWRAP) || ISSET(SOLO_SIDESCROLL) {
        let opt = if ISSET(SOFTWRAP) { "--softwrap" } else { "--solo" };
        winio::statusline(MessageType::Ahem, &format!("Not possible with '{}'", opt));
        return;
    }

    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        let tabsize = g.tabsize;
        of.brink += if tabsize < 2 { 2 } else { tabsize };

        let sill = of.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(0) + g.editwinrows as isize;
        let mut line = of.current.clone().unwrap();

        /* 若当前行不允许进一步滚动，在视口中寻找更早或更晚的行。 */
        while {
            let is_edittop = of.edittop.as_ref().map(|e| Rc::ptr_eq(e, &line)).unwrap_or(false);
            !is_edittop
        } && utils::breadth(line.borrow().data.as_bytes()) < of.brink + CUSHION
        {
            let prev = { let r = line.borrow(); r.prev.clone() }.and_then(|w| w.upgrade()).unwrap();
            line = prev;
        }
        loop {
            let lineno = line.borrow().lineno;
            let has_next = { let r = line.borrow(); r.next.is_some() };
            if lineno >= sill || utils::breadth(line.borrow().data.as_bytes()) >= of.brink + CUSHION || !has_next {
                break;
            }
            let next = { let r = line.borrow(); r.next.clone() }.unwrap();
            line = next;
        }
        let lineno = line.borrow().lineno;
        if lineno < sill && utils::breadth(line.borrow().data.as_bytes()) >= of.brink + CUSHION {
            of.current = Some(line.clone());
        }

        let cur = of.current.clone().unwrap();
        let data = cur.borrow().data.clone();
        let frame_x = utils::actual_x(data.as_bytes(), of.brink + CUSHION);

        if of.current_x < frame_x {
            of.current_x = frame_x;
            of.placewewant = utils::xplustabs();
        }
        g.refresh_needed = true;
    });
}

// ======================== 兼容别名 ========================

/// 移动到文件首行（对应 global.c 绑定的 do_first_line）。
pub fn do_first_line() {
    to_first_line();
}

/// 移动到文件末行（对应 global.c 绑定的 do_last_line）。
pub fn do_last_line() {
    to_last_line();
}