/**************************************************************************
 *   move.rs  --  杩欐槸 GNU nano 鐨?Rust 缈昏瘧鐗堟湰鐨勪竴閮ㄥ垎锛堝搴?move.c锛夈€? *
 *   鐗堟潈 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   鐗堟潈 (C) 2014-2018, 2020, 2024, 2026 Benno Schulenberg
 **************************************************************************/

//! 鍏夋爣绉诲姩銆佹粴鍔ㄣ€佺炕椤点€佸崟璇?娈佃惤璺宠浆绛夈€傚搴斿師鐗?`move.c`銆?
use crate::chars;
use crate::chars::white_string;
use crate::definitions::*;
use crate::files::leftedge_for;
use crate::global::*;
use crate::utils;
use crate::utils::xplustabs;
use crate::winio::statusline;

/* =========================================================================
 * 鍗犱綅鍑芥暟妗╋細浠ヤ笅鍑芥暟鐢卞悗缁炕璇戠殑妯″潡锛坵inio銆乼ext銆乧olor 绛夛級瀹炵幇銆? * 姝ゅ浠呭０鏄庢々浠ヤ繚璇?cargo check 閫氳繃锛屽緟瀵瑰簲妯″潡缈昏瘧瀹屾垚鍚庡垹闄ゅ搴旀々銆? * ========================================================================= */

#[allow(dead_code)]
pub fn full_refresh() {}
#[allow(dead_code)]
pub fn actual_last_column(_leftedge: usize, _target: usize) -> usize {
    0
}
#[allow(dead_code)]
pub fn go_back_chunks(_n: i32, _line: *mut *mut linestruct, _leftedge: *mut usize) -> i32 {
    0
}
#[allow(dead_code)]
pub fn go_forward_chunks(_n: i32, _line: *mut *mut linestruct, _leftedge: *mut usize) -> i32 {
    0
}
/* 调整视口，使 current 保持在可见区域内（简化：无软换行）。 */
pub unsafe fn adjust_viewport(_kind: update_type) {
    let of = &mut *openfile;
    if of.edittop.is_null() {
        of.edittop = of.current;
        return;
    }
    /* current 在 edittop 之前 → 滚动到 current。 */
    let mut line = of.edittop;
    let mut count: isize = 0;
    while !line.is_null() && line != of.current {
        line = (*line).next;
        count += 1;
    }
    if line.is_null() {
        /* current 在 edittop 之前，把 edittop 移到 current。 */
        of.edittop = of.current;
    } else if count >= editwinrows as isize {
        /* current 超出底部，把 edittop 下移使其可见。 */
        let mut top = of.edittop;
        for _ in 0..(count - editwinrows as isize + 1) {
            if (*top).next.is_null() { break; }
            top = (*top).next;
        }
        of.edittop = top;
    }
    refresh_needed = true;
}

#[allow(dead_code)]
pub fn draw_all_subwindows() {}
#[allow(dead_code)]
pub fn begpar(_line: *mut linestruct, _n: i32) -> bool {
    false
}
#[allow(dead_code)]
pub fn inpar(_line: *mut linestruct) -> bool {
    false
}
/* 重新绘制编辑区：调整视口后请求整屏刷新。 */
pub unsafe fn edit_redraw(_was: *mut linestruct, _kind: update_type) {
    adjust_viewport(_kind);
    refresh_needed = true;
}
#[allow(dead_code)]
pub fn indent_length(_data: &str) -> usize {
    0
}
#[allow(dead_code)]
pub fn get_softwrap_breakpoint(_data: &str, _leftedge: usize, _kickoff: *mut bool, _last_chunk: *mut bool) -> usize {
    0
}
#[allow(dead_code)]
pub fn line_needs_update(_was: usize, _now: usize) -> bool {
    false
}
#[allow(dead_code)]
pub fn update_line(_line: *mut linestruct, _x: usize) {}
#[allow(dead_code)]
pub fn edit_scroll(_dir: bool) {}
#[allow(dead_code)]
pub fn extra_chunks_in(_line: *mut linestruct) -> usize {
    0
}
#[allow(dead_code)]
pub fn chunk_for(_leftedge: usize, _line: *mut linestruct) -> usize {
    0
}

/* ===== 绉诲姩鐩稿叧鍑芥暟 ===== */

/* Move to the first line of the file. */
pub unsafe fn to_first_line() {
    (*openfile).current = (*openfile).filetop;
    (*openfile).current_x = 0;
    (*openfile).placewewant = 0;

    refresh_needed = true;
}

/* Move to the last line of the file. */
pub unsafe fn to_last_line() {
    let of = &mut *openfile;
    of.current = of.filebot;
    of.current_x = if inhelp { 0 } else { (*of.filebot).data.len() };
    of.placewewant = xplustabs();

    of.cursor_row = (editwinrows - 1) as isize;

    refresh_needed = true;
    focusing = false;
}

/* Determine the actual current chunk and the target column. */
pub unsafe fn get_edge_and_target(leftedge: *mut usize, target_column: *mut usize) {
    if ISSET(SOFTWRAP) {
        let shim = editwincols * (1 + (tabsize as usize / editwincols));

        *leftedge = leftedge_for(xplustabs() as isize, (*openfile).current);
        *target_column = (openfile.as_ref().unwrap().placewewant + shim - *leftedge) % editwincols;
    } else {
        *leftedge = 0;
        *target_column = openfile.as_ref().unwrap().placewewant;
    }
}

/* Return the index in line->data that corresponds to the given column on the
 * chunk that starts at the given leftedge. */
pub unsafe fn proper_x(line: *mut linestruct, leftedge: *mut usize, forward: bool, column: usize, shifted: *mut bool) -> usize {
    let index = utils::actual_x((*line).data.as_bytes(), column);

    if ISSET(SOFTWRAP) && (*line).data.as_bytes()[index] == b'\t' && ((forward && utils::wideness((*line).data.as_bytes(), index) < *leftedge) || (!forward && column / (tabsize as usize) == (*leftedge - 1) / (tabsize as usize) && column / (tabsize as usize) < (*leftedge + editwincols - 1) / (tabsize as usize))) {
        let newindex = index + 1;
        if !shifted.is_null() {
            *shifted = true;
        }
        if ISSET(SOFTWRAP) {
            *leftedge = leftedge_for(utils::wideness((*line).data.as_bytes(), newindex) as isize, line);
        }
        return newindex;
    }

    if ISSET(SOFTWRAP) {
        *leftedge = leftedge_for(utils::wideness((*line).data.as_bytes(), index) as isize, line);
    }

    index
}

/* Adjust the values for current_x and placewewant in case we have landed in
 * the middle of a tab that crosses a row boundary. */
pub unsafe fn set_proper_index_and_pww(leftedge: *mut usize, target: usize, forward: bool) {
    let was_edge = *leftedge;
    let mut shifted = false;

    (*openfile).current_x = proper_x((*openfile).current, leftedge, forward, actual_last_column(*leftedge, target), &mut shifted);

    if shifted || *leftedge < was_edge {
        (*openfile).current_x = proper_x((*openfile).current, leftedge, forward, actual_last_column(*leftedge, target), &mut shifted);
    }

    (*openfile).placewewant = *leftedge + target;
}

/* Move up almost one screenful. */
pub unsafe fn do_page_up() {
    let mustmove = if editwinrows < 3 { 1 } else { editwinrows - 2 };
    let mut leftedge: usize = 0;
    let mut target_column: usize = 0;

    if ISSET(JUMPY_SCROLLING) {
        let of = &mut *openfile;
        of.current = of.edittop;
        leftedge = of.firstcolumn;
        of.cursor_row = 0;
        target_column = 0;
    } else {
        get_edge_and_target(&mut leftedge, &mut target_column);
    }

    if go_back_chunks(mustmove as i32, &mut (*openfile).current, &mut leftedge) > 0 {
        to_first_line();
        return;
    }

    set_proper_index_and_pww(&mut leftedge, target_column, false);

    adjust_viewport(update_type::STATIONARY);
    refresh_needed = true;
}

/* Move down almost one screenful. */
pub unsafe fn do_page_down() {
    let mustmove = if editwinrows < 3 { 1 } else { editwinrows - 2 };
    let mut leftedge: usize = 0;
    let mut target_column: usize = 0;

    if ISSET(JUMPY_SCROLLING) {
        let of = &mut *openfile;
        of.current = of.edittop;
        leftedge = of.firstcolumn;
        of.cursor_row = 0;
        target_column = 0;
    } else {
        get_edge_and_target(&mut leftedge, &mut target_column);
    }

    if go_forward_chunks(mustmove as i32, &mut (*openfile).current, &mut leftedge) > 0 {
        to_last_line();
        return;
    }

    set_proper_index_and_pww(&mut leftedge, target_column, true);

    adjust_viewport(update_type::STATIONARY);
    refresh_needed = true;
}

/* Place the cursor on the first row in the viewport. */
pub unsafe fn to_top_row() {
    let mut leftedge: usize = 0;
    let mut offset: usize = 0;

    get_edge_and_target(&mut leftedge, &mut offset);

    let of = &mut *openfile;
    of.current = of.edittop;
    leftedge = of.firstcolumn;

    set_proper_index_and_pww(&mut leftedge, offset, false);

    refresh_needed = !(*openfile).mark.is_null();
}

/* Place the cursor on the last row in the viewport, when possible. */
pub unsafe fn to_bottom_row() {
    let mut leftedge: usize = 0;
    let mut offset: usize = 0;

    get_edge_and_target(&mut leftedge, &mut offset);

    let of = &mut *openfile;
    of.current = of.edittop;
    leftedge = of.firstcolumn;

    go_forward_chunks((editwinrows - 1) as i32, &mut (*openfile).current, &mut leftedge);
    set_proper_index_and_pww(&mut leftedge, offset, true);

    refresh_needed = !(*openfile).mark.is_null();
}

/* Put the cursor line at the center, then the top, then the bottom. */
pub unsafe fn do_cycle() {
    if cycling_aim == 0 {
        adjust_viewport(update_type::CENTERING);
    } else {
        let of = &mut *openfile;
        of.cursor_row = if cycling_aim == 1 { 0 } else { (editwinrows - 1) as isize };
        adjust_viewport(update_type::STATIONARY);
    }

    cycling_aim = (cycling_aim + 1) % 3;

    draw_all_subwindows();
    full_refresh();
}

/* Scroll the line with the cursor to the center of the screen. */
pub unsafe fn do_center() {
    adjust_viewport(update_type::CENTERING);
    draw_all_subwindows();
    full_refresh();
}

/* Move to the first beginning of a paragraph before the current line. */
pub unsafe fn do_para_begin(line: *mut *mut linestruct) {
    if !(*(*line)).prev.is_null() {
        *line = (*(*line)).prev;
    }

    while !begpar(*line, 0) {
        *line = (*(*line)).prev;
    }
}

/* Move down to the last line of the first found paragraph. */
pub unsafe fn do_para_end(line: *mut *mut linestruct) {
    while !(*(*line)).next.is_null() && !inpar(*line) {
        *line = (*(*line)).next;
    }

    while !(*(*line)).next.is_null() && inpar((*(*line)).next) && !begpar((*(*line)).next, 0) {
        *line = (*(*line)).next;
    }
}

/* Move up to first start of a paragraph before the current line. */
pub unsafe fn to_para_begin() {
    let was_current = (*openfile).current;

    do_para_begin(&mut (*openfile).current);
    (*openfile).current_x = 0;

    edit_redraw(was_current, update_type::CENTERING);
}

/* Move down to just after the first found end of a paragraph. */
pub unsafe fn to_para_end() {
    let was_current = (*openfile).current;

    do_para_end(&mut (*openfile).current);

    let of = &mut *openfile;
    if !of.current.is_null() && !(*of.current).next.is_null() {
        of.current = (*of.current).next;
        of.current_x = 0;
    } else {
        of.current_x = (*of.current).data.len();
    }

    edit_redraw(was_current, update_type::CENTERING);
}

/* Move to the preceding block of text. */
pub unsafe fn to_prev_block() {
    let was_current = (*openfile).current;
    let mut is_text = false;
    let mut seen_text = false;

    while !(*openfile).current.is_null() && !(*(*openfile).current).prev.is_null() && (!seen_text || is_text) {
        let of = &mut *openfile;
        of.current = (*of.current).prev;
        is_text = !white_string((*of.current).data.as_bytes());
        seen_text = seen_text || is_text;
    }

    let of = &mut *openfile;
    if seen_text && !of.current.is_null() && !(*of.current).next.is_null() && white_string((*of.current).data.as_bytes()) {
        of.current = (*of.current).next;
    }

    (*openfile).current_x = 0;
    edit_redraw(was_current, update_type::CENTERING);
}

/* Move to the next block of text. */
pub unsafe fn to_next_block() {
    let was_current = (*openfile).current;
    let mut is_white = white_string((*(*openfile).current).data.as_bytes());
    let mut seen_white = is_white;

    while !(*openfile).current.is_null() && !(*(*openfile).current).next.is_null() && (!seen_white || is_white) {
        let of = &mut *openfile;
        of.current = (*of.current).next;
        is_white = white_string((*of.current).data.as_bytes());
        seen_white = seen_white || is_white;
    }

    (*openfile).current_x = 0;
    edit_redraw(was_current, update_type::CENTERING);
}

/* Move to the previous word. */
pub unsafe fn do_prev_word() {
    let punctuation_as_letters = ISSET(WORD_BOUNDS);
    let mut seen_a_word = false;
    let mut step_forward = false;

    loop {
        let of = &mut *openfile;
        if of.current_x == 0 {
            if (*of.current).prev.is_null() {
                break;
            }
            of.current = (*of.current).prev;
            of.current_x = (*of.current).data.len();
        }

        of.current_x = chars::step_left((*of.current).data.as_bytes(), of.current_x);

        let data = (*of.current).data.clone();
        if chars::is_word_char(&data.as_bytes()[of.current_x..], punctuation_as_letters) {
            seen_a_word = true;
            if of.current_x == 0 {
                break;
            }
        } else if chars::is_zerowidth(&data.as_bytes()[of.current_x..]) {
            /* skip */
        } else if seen_a_word {
            step_forward = true;
            break;
        }
    }

    if step_forward {
        let of = &mut *openfile;
        of.current_x = chars::step_right((*of.current).data.as_bytes(), of.current_x);
    }
}

/* Move to the next word.  If after_ends is TRUE, stop at the ends of words
 * instead of at their beginnings.  Return TRUE if we started on a word. */
pub unsafe fn do_next_word(after_ends: bool) -> bool {
    let of = &mut *openfile;
    let data = (*of.current).data.clone();
    let started_on_word = chars::is_word_char(&data.as_bytes()[of.current_x..], ISSET(WORD_BOUNDS));
    let mut seen_space = !started_on_word;
    let mut seen_word = started_on_word;

    loop {
        let of = &mut *openfile;
        if of.current_x >= (*of.current).data.len() {
            if (*of.current).next.is_null() {
                break;
            }
            of.current = (*of.current).next;
            of.current_x = 0;
            seen_space = true;
        } else {
            of.current_x = chars::step_right((*of.current).data.as_bytes(), of.current_x);
        }

        let data = (*of.current).data.clone();
        if after_ends {
            if chars::is_word_char(&data.as_bytes()[of.current_x..], ISSET(WORD_BOUNDS)) {
                seen_word = true;
            } else if chars::is_zerowidth(&data.as_bytes()[of.current_x..]) {
                /* skip */
            } else if seen_word {
                break;
            }
        } else {
            if chars::is_zerowidth(&data.as_bytes()[of.current_x..]) {
                /* skip */
            } else if !chars::is_word_char(&data.as_bytes()[of.current_x..], ISSET(WORD_BOUNDS)) {
                seen_space = true;
            } else if seen_space {
                break;
            }
        }
    }

    started_on_word
}

/* Move to the previous word in the file, and update the screen afterwards. */
pub unsafe fn to_prev_word() {
    let was_current = (*openfile).current;

    do_prev_word();

    edit_redraw(was_current, update_type::FLOWING);
}

/* Move to the next word in the file.  Update the screen afterwards. */
pub unsafe fn to_next_word() {
    let was_current = (*openfile).current;

    do_next_word(ISSET(AFTER_ENDS));

    edit_redraw(was_current, update_type::FLOWING);
}

/* Move to the beginning of the current line (or softwrapped chunk). */
pub unsafe fn do_home() {
    let was_current = (*openfile).current;
    let was_column = xplustabs();
    let mut moved_off_chunk = true;
    let mut moved = false;
    let mut leftedge: usize = 0;
    let mut left_x: usize = 0;

    if ISSET(SOFTWRAP) {
        leftedge = leftedge_for(was_column as isize, (*openfile).current);
        left_x = proper_x((*openfile).current, &mut leftedge, false, leftedge, std::ptr::null_mut());
    }

    if ISSET(SMART_HOME) {
        let indent_x = indent_length(&(*(*openfile).current).data);

        let of = &mut *openfile;
        if !(*of.current).data.as_bytes()[indent_x..].is_empty() {
            if of.current_x == indent_x {
                of.current_x = 0;
                moved = true;
            } else if left_x <= indent_x {
                of.current_x = indent_x;
                moved = true;
            }
        }
    }

    if !moved && ISSET(SOFTWRAP) {
        let of = &mut *openfile;
        if of.current_x == left_x {
            of.current_x = 0;
        } else {
            of.current_x = left_x;
            of.placewewant = leftedge;
            moved_off_chunk = false;
        }
    } else if !moved {
        (*openfile).current_x = 0;
    }

    if moved_off_chunk {
        (*openfile).placewewant = xplustabs();
    }

    if ISSET(SOFTWRAP) && moved_off_chunk {
        edit_redraw(was_current, update_type::FLOWING);
    } else if line_needs_update(was_column, (*openfile).placewewant) {
        update_line((*openfile).current, (*openfile).current_x);
    }
}

/* Move to the end of the current line (or softwrapped chunk). */
pub unsafe fn do_end() {
    let was_current = (*openfile).current;
    let was_column = xplustabs();
    let line_len = (*(*openfile).current).data.len();
    let mut moved_off_chunk = true;

    if ISSET(SOFTWRAP) {
        let mut kickoff = true;
        let mut last_chunk = false;
        let leftedge = leftedge_for(was_column as isize, (*openfile).current);
        let mut rightedge = get_softwrap_breakpoint((*(*openfile).current).data.as_str(), leftedge, &mut kickoff, &mut last_chunk);

        if !last_chunk {
            rightedge -= 1;
        }

        let right_x = utils::actual_x((*(*openfile).current).data.as_bytes(), rightedge);

        let of = &mut *openfile;
        if of.current_x == right_x {
            of.current_x = line_len;
        } else {
            of.current_x = right_x;
            of.placewewant = rightedge;
            moved_off_chunk = false;
        }
    } else {
        (*openfile).current_x = line_len;
    }

    if moved_off_chunk {
        (*openfile).placewewant = xplustabs();
    }

    if ISSET(SOFTWRAP) && moved_off_chunk {
        edit_redraw(was_current, update_type::FLOWING);
    } else if line_needs_update(was_column, (*openfile).placewewant) {
        update_line((*openfile).current, (*openfile).current_x);
    }
}

/* Move the cursor to the preceding line or chunk. */
pub unsafe fn do_up() {
    let was_current = (*openfile).current;
    let mut leftedge: usize = 0;
    let mut target_column: usize = 0;

    get_edge_and_target(&mut leftedge, &mut target_column);

    if go_back_chunks(1, &mut (*openfile).current, &mut leftedge) > 0 {
        return;
    }

    set_proper_index_and_pww(&mut leftedge, target_column, false);

    if (*openfile).cursor_row == 0 && !ISSET(JUMPY_SCROLLING) && ((tabsize as usize) >= editwincols || !ISSET(SOFTWRAP)) {
        edit_scroll(BACKWARD);
    } else {
        edit_redraw(was_current, update_type::FLOWING);
    }

    (*openfile).placewewant = leftedge + target_column;
}

/* Move the cursor to next line or chunk. */
pub unsafe fn do_down() {
    let was_current = (*openfile).current;
    let mut leftedge: usize = 0;
    let mut target_column: usize = 0;

    get_edge_and_target(&mut leftedge, &mut target_column);

    if go_forward_chunks(1, &mut (*openfile).current, &mut leftedge) > 0 {
        return;
    }

    set_proper_index_and_pww(&mut leftedge, target_column, true);

    if (*openfile).cursor_row == (editwinrows - 1) as isize && !ISSET(JUMPY_SCROLLING) && ((tabsize as usize) >= editwincols || !ISSET(SOFTWRAP)) {
        edit_scroll(FORWARD);
    } else {
        edit_redraw(was_current, update_type::FLOWING);
    }

    (*openfile).placewewant = leftedge + target_column;
}

/* Scroll up one line or chunk without moving the cursor textwise. */
pub unsafe fn do_scroll_up() {
    let of = &mut *openfile;
    if (*of.edittop).prev.is_null() && of.firstcolumn == 0 {
        return;
    }

    if of.cursor_row == (editwinrows - 1) as isize {
        do_up();
    }

    if editwinrows > 1 {
        edit_scroll(BACKWARD);
    }
}

/* Scroll down one line or chunk without moving the cursor textwise. */
pub unsafe fn do_scroll_down() {
    let of = &mut *openfile;
    if of.cursor_row == 0 {
        do_down();
    }

    if editwinrows > 1 && ((*of.edittop).next.is_null() || (ISSET(SOFTWRAP) && (extra_chunks_in(of.edittop) > chunk_for(of.firstcolumn, of.edittop)))) {
        edit_scroll(FORWARD);
    }
}

/* Move left one character. */
pub unsafe fn do_left() {
    let was_current = (*openfile).current;
    let of = &mut *openfile;

    if of.current_x > 0 {
        of.current_x = chars::step_left((*of.current).data.as_bytes(), of.current_x);
        while of.current_x > 0 && chars::is_zerowidth(&(*of.current).data.as_bytes()[of.current_x..]) {
            of.current_x = chars::step_left((*of.current).data.as_bytes(), of.current_x);
        }
    } else if of.current != of.filetop {
        of.current = (*of.current).prev;
        of.current_x = (*of.current).data.len();
    }

    edit_redraw(was_current, update_type::FLOWING);
}

/* Move right one character. */
pub unsafe fn do_right() {
    let was_current = (*openfile).current;
    let of = &mut *openfile;

    if of.current_x < (*of.current).data.len() {
        of.current_x = chars::step_right((*of.current).data.as_bytes(), of.current_x);
        while of.current_x < (*of.current).data.len() && chars::is_zerowidth(&(*of.current).data.as_bytes()[of.current_x..]) {
            of.current_x = chars::step_right((*of.current).data.as_bytes(), of.current_x);
        }
    } else if of.current != of.filebot {
        of.current = (*of.current).next;
        of.current_x = 0;
    }

    edit_redraw(was_current, update_type::FLOWING);
}

/* Scroll the viewport horizontally to the left. */
pub unsafe fn do_scroll_left() {
    if ISSET(SOFTWRAP) || ISSET(SOLO_SIDESCROLL) {
        statusline(message_type::AHEM, "Not possible with option");
        return;
    }

    let of = &mut *openfile;
    of.brink -= if of.brink < (tabsize as usize) { of.brink } else if (tabsize as usize) < 2 { 2 } else { tabsize as usize };

    let frame_x = utils::actual_x((*of.current).data.as_bytes(), of.brink + editwincols - CUSHION - 1);

    if of.current_x > frame_x {
        of.current_x = frame_x;
        of.placewewant = xplustabs();
    }

    refresh_needed = true;
}

/* Scroll the viewport horizontally to the right. */
pub unsafe fn do_scroll_right() {
    if ISSET(SOFTWRAP) || ISSET(SOLO_SIDESCROLL) {
        statusline(message_type::AHEM, "Not possible with option");
        return;
    }

    let of = &mut *openfile;
    of.brink += if (tabsize as usize) < 2 { 2 } else { tabsize as usize };

    let sill = (*of.edittop).lineno + editwinrows as isize;
    let mut line = of.current;

    while line != of.edittop && utils::breadth((*line).data.as_bytes()) < of.brink + CUSHION {
        line = (*line).prev;
    }
    while (*line).lineno < sill && utils::breadth((*line).data.as_bytes()) < of.brink + CUSHION && !(*line).next.is_null() {
        line = (*line).next;
    }
    if (*line).lineno < sill && utils::breadth((*line).data.as_bytes()) >= of.brink + CUSHION {
        of.current = line;
    }

    let frame_x = utils::actual_x((*of.current).data.as_bytes(), of.brink + CUSHION);

    if of.current_x < frame_x {
        of.current_x = frame_x;
        of.placewewant = xplustabs();
    }

    refresh_needed = true;
}

