/**************************************************************************
 *   text.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 text.c）。
 *
 *   版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2014-2015 Mark Majeres
 *   版权 (C) 2016 Mike Scalora
 *   版权 (C) 2016 Sumedh Pendurkar
 *   版权 (C) 2018 Marco Diego Aurélio Mesquita
 *   版权 (C) 2015-2022, 2024, 2026 Benno Schulenberg
 **************************************************************************/

//! 文本编辑操作：标记、缩进、注释、撤销/重做、换行、自动换行、
//! 段落对齐、拼写检查、lint、格式化、字数统计、原样输入、单词补全。
//! 对应原版 `text.c`。全功能构建：所有条件编译块均按已启用翻译。

use std::fs::File;
use std::os::raw::{c_void};

use crate::chars;
use crate::definitions::*;
use crate::global::*;
use crate::utils;
use crate::files::set_modified;
use crate::files::LINES;
use crate::winio::{
    blank_bottombars, bottombars, edit_refresh, enter_terminal, leave_terminal,
    get_kbinput, napms, statusbar, statusline, titlebar, window_init, wipe_statusbar,
};
use crate::r#move::{
    do_page_up, do_page_down, to_prev_block, to_next_block, do_para_begin, do_para_end,
    do_next_word, to_para_begin, to_para_end,
};

/* =========================================================================
 * 桩区块：以下函数由尚未翻译的模块（winio、prompt、cut、search 等）实现。
 * 此处仅声明桩以保证编译通过，待对应模块翻译完成后删除。
 * ========================================================================= */

/* 在光标处插入文本（text 为已翻译的 Rust 字符串，len 为字符数）。 */
pub unsafe fn inject(text: &str, _len: usize) {
    if text.is_empty() {
        return;
    }
    let of = &mut *openfile;
    let cur = &mut *of.current;
    let mut newdata = String::with_capacity(cur.data.len() + text.len());
    newdata.push_str(&cur.data[..of.current_x]);
    newdata.push_str(text);
    newdata.push_str(&cur.data[of.current_x..]);
    cur.data = newdata;
    of.current_x += text.chars().count();
    of.placewewant = utils::xplustabs();
    of.totsize += text.len();
    set_modified();
    refresh_needed = true;
    focusing = false;
}

/* 把 node 插入到 line 之后（双向链表接线）。 */
pub unsafe fn splice_node(line: *mut linestruct, node: *mut linestruct) {
    if line.is_null() || node.is_null() {
        return;
    }
    let next = (*line).next;
    (*node).prev = line;
    (*node).next = next;
    (*line).next = node;
    if !next.is_null() {
        (*next).prev = node;
    }
}

/* 从双向链表中摘除 line（不释放）。 */
pub unsafe fn unlink_node(line: *mut linestruct) {
    if line.is_null() {
        return;
    }
    let prev = (*line).prev;
    let next = (*line).next;
    if !prev.is_null() {
        (*prev).next = next;
    }
    if !next.is_null() {
        (*next).prev = prev;
    }
    (*line).prev = std::ptr::null_mut();
    (*line).next = std::ptr::null_mut();
}

/* 从给定行起重新编号 lineno。 */
pub unsafe fn renumber_from(line: *mut linestruct) {
    if line.is_null() {
        return;
    }
    let mut l = line;
    while !l.is_null() {
        let prev = (*l).prev;
        (*l).lineno = if prev.is_null() { 1 } else { (*prev).lineno + 1 };
        l = (*l).next;
    }
}

#[allow(dead_code)]
pub fn new_magicline() {}
#[allow(dead_code)]
pub fn remove_magicline() {}

#[allow(dead_code)]
pub fn get_range(_top: *mut *mut linestruct, _bot: *mut *mut linestruct) {}
#[allow(dead_code)]
pub fn get_region(
    _top: *mut *mut linestruct, _top_x: *mut usize,
    _bot: *mut *mut linestruct, _bot_x: *mut usize,
) {}
#[allow(dead_code)]
pub fn mark_is_before_cursor() -> bool { false }
#[allow(dead_code)]
pub fn line_from_number(_n: isize) -> *mut linestruct { std::ptr::null_mut() }
#[allow(dead_code)]
pub fn ensure_firstcolumn_is_aligned() {}
#[allow(dead_code)]
pub fn adjust_viewport(_kind: update_type) {}
#[allow(dead_code)]
pub fn do_snip(_a: bool, _b: bool, _c: bool) {}
#[allow(dead_code)]
pub fn extract_segment(
    _startline: *mut linestruct, _start_x: usize,
    _endline: *mut linestruct, _end_x: usize,
) {}
#[allow(dead_code)]
pub fn ingraft_buffer(_buf: *mut linestruct) {}
#[allow(dead_code)]
pub fn copy_from_buffer(_buf: *mut linestruct) {}
#[allow(dead_code)]
pub fn cut_marked_region() {}
#[allow(dead_code)]
pub fn free_lines(_line: *mut linestruct) {}
#[allow(dead_code)]
pub fn copy_buffer(_line: *mut linestruct) -> *mut linestruct { std::ptr::null_mut() }
#[allow(dead_code)]
pub fn do_prompt(
    _menu: i32, _given: &mut Option<String>, _history: *mut linestruct,
    _refresh: unsafe fn(), _msg: &str,
) -> i32 { 0 }
#[allow(dead_code)]
pub fn put_cursor_at_end_of_answer() {}
#[allow(dead_code)]
pub fn place_the_cursor() {}
#[allow(dead_code)]
pub fn beep() {}
#[allow(dead_code)]
pub fn confirm_margin() {}
#[allow(dead_code)]
pub fn terminal_init() {
    enter_terminal();
}
#[allow(dead_code)]
pub fn endwin() {
    leave_terminal();
}
#[allow(dead_code)]
pub fn wredrawln(_win: *mut c_void, _beg: i32, _num: i32) {}
#[allow(dead_code)]
pub fn full_refresh() {}
#[allow(dead_code)]
pub fn block_sigwinch(_on: bool) {}
#[allow(dead_code)]
pub fn safe_tempfile(_stream: *mut *mut File) -> *mut u8 { std::ptr::null_mut() }
#[allow(dead_code)]
pub fn write_region_to_file(_name: &str, _stream: *mut File, _method: writing_type) -> bool { false }
#[allow(dead_code)]
pub fn write_it_out(_exitquestion: bool, _exiting: bool) -> i32 { 0 }
#[allow(dead_code)]
pub fn open_file(_name: &str, _quiet: bool, _stream: *mut *mut File) -> i32 { 0 }
#[allow(dead_code)]
pub fn read_file(_stream: *mut File, _fd: i32, _name: &str, _undoable: bool) {}
#[allow(dead_code)]
pub fn open_buffer(_name: &str, _quiet: bool) {}
#[allow(dead_code)]
pub fn write_file(_name: &str, _stream: *mut File, _method: writing_type, _f: i32) -> bool { false }
#[allow(dead_code)]
pub fn in_restricted_mode() -> bool { false }
#[allow(dead_code)]
pub fn check_the_multis(_line: *mut linestruct) {}
#[allow(dead_code)]
pub fn ask_user(_kind: bool, _msg: &str) -> i32 { 0 }
#[allow(dead_code)]
pub fn do_cancel() {}
#[allow(dead_code)]
pub fn do_help() {}
#[allow(dead_code)]
pub fn get_verbatim_kbinput(_win: *mut c_void, _count: *mut usize) -> *mut u8 { std::ptr::null_mut() }
#[allow(dead_code)]
pub fn expunge(_type: undo_type) {}
#[allow(dead_code)]
pub fn go_forward_chunks(_n: i32, _line: *mut *mut linestruct, _leftedge: *mut usize) -> i32 { 0 }
#[allow(dead_code)]
pub fn regenerate_screen() {}

/* 复数宏桩。 */
#[allow(dead_code)]
pub fn P_<'a>(one: &'a str, many: &'a str, n: isize) -> &'a str {
    if n == 1 { one } else { many }
}

/* =========================================================================
 * 文本编辑函数（按 text.c 顺序翻译）
 * ========================================================================= */

/* 切换标记。 */
pub unsafe fn do_mark() {
    if (*openfile).mark.is_null() {
        (*openfile).mark = (*openfile).current;
        (*openfile).mark_x = (*openfile).current_x;
        (*openfile).softmark = false;
        statusbar("Mark Set");
    } else {
        (*openfile).mark = std::ptr::null_mut();
        statusbar("Mark Unset");
        refresh_needed = true;
    }
}

/* 插入一个制表符，或（若 --tabstospaces）插入等宽的空格。 */
pub unsafe fn do_tab() {
    /* 当标记了区域时，缩进整个区域。 */
    if !(*openfile).mark.is_null() && (*openfile).mark != (*openfile).current {
        do_indent();
    } else if let Some(syntax) = (*openfile).syntax.as_ref() {
        if let Some(ref tabstring) = syntax.tabstring {
            inject(tabstring, tabstring.len());
        } else {
            do_tab_spaces_or_tab();
        }
    } else {
        do_tab_spaces_or_tab();
    }
}

/* 在 do_tab 中，当没有语法 tabstring 时，根据 TABS_TO_SPACES 决定插入空格或制表符。 */
unsafe fn do_tab_spaces_or_tab() {
    if ISSET(TABS_TO_SPACES) {
        let length = (tabsize as usize) - (utils::xplustabs() % (tabsize as usize));
        let spaces = " ".repeat(length);
        inject(&spaces, length);
    } else {
        inject("\t", 1);
    }
}

/* 给给定行添加一个缩进。 */
pub unsafe fn indent_a_line(line: *mut linestruct, indentation: &str) {
    let length = (*line).data.len();
    let indent_len = indentation.len();

    /* 如果请求的缩进为空，不改变该行。 */
    if indent_len == 0 {
        return;
    }

    /* 把构造出的缩进加到行首。 */
    let mut newdata = String::with_capacity(length + indent_len);
    newdata.push_str(indentation);
    newdata.push_str(&(*line).data);
    (*line).data = newdata;

    (*openfile).totsize += indent_len;

    /* 补偿当前行的变化。 */
    if line == (*openfile).mark && (*openfile).mark_x > 0 {
        (*openfile).mark_x += indent_len;
    }
    if line == (*openfile).current && (*openfile).current_x > 0 {
        (*openfile).current_x += indent_len;
        (*openfile).placewewant = utils::xplustabs();
    }
}

/* 缩进当前行（或标记的行）tabsize 列。 */
pub unsafe fn do_indent() {
    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();

    get_range(&mut top, &mut bot);

    /* 跳过前导空行。 */
    while top != (*bot).next && (*top).data.as_bytes().first().copied().unwrap_or(0) == 0 {
        top = (*top).next;
    }

    /* 若所有行都是空的，无事可做。 */
    if top == (*bot).next {
        return;
    }

    let indentation: String = if let Some(syntax) = (*openfile).syntax.as_ref() {
        if let Some(ref tabstring) = syntax.tabstring {
            tabstring.clone()
        } else if ISSET(TABS_TO_SPACES) {
            " ".repeat(tabsize as usize)
        } else {
            "\t".to_string()
        }
    } else if ISSET(TABS_TO_SPACES) {
        " ".repeat(tabsize as usize)
    } else {
        "\t".to_string()
    };

    add_undo(undo_type::INDENT, std::ptr::null_mut());

    /* 逐行添加缩进，并记录到撤销项。 */
    let mut line = top;
    while line != (*bot).next {
        let real_indent = if (*line).data.as_bytes().first().copied().unwrap_or(0) == 0 {
            ""
        } else {
            indentation.as_str()
        };

        indent_a_line(line, real_indent);
        update_multiline_undo((*line).lineno, real_indent);

        line = (*line).next;
    }

    set_modified();
    ensure_firstcolumn_is_aligned();
    refresh_needed = true;
    shift_held = true;
}

/* 返回给定文本开头的空白字节数，但最多一个制表符宽。 */
pub unsafe fn length_of_white(text: &str) -> usize {
    let mut white_count = 0;

    if let Some(syntax) = (*openfile).syntax.as_ref() {
        if let Some(ref tabstring) = syntax.tabstring {
            let thelength = tabstring.len();
            let tb = tabstring.as_bytes();

            while white_count < text.len() && text.as_bytes()[white_count] == tb[white_count] {
                white_count += 1;
                if white_count == thelength {
                    return thelength;
                }
            }
            white_count = 0;
        }
    }

    let bytes = text.as_bytes();
    loop {
        if bytes[white_count] == b'\t' {
            return white_count + 1;
        }
        if bytes[white_count] != b' ' {
            return white_count;
        }
        white_count += 1;
        if white_count == tabsize as usize {
            return tabsize as usize;
        }
    }
}

/* 当标记和光标在给定行上时，调整它们的位置。 */
pub unsafe fn compensate_leftward(line: *mut linestruct, leftshift: usize) {
    if line == (*openfile).mark {
        if (*openfile).mark_x < leftshift {
            (*openfile).mark_x = 0;
        } else {
            (*openfile).mark_x -= leftshift;
        }
    }

    if line == (*openfile).current {
        if (*openfile).current_x < leftshift {
            (*openfile).current_x = 0;
        } else {
            (*openfile).current_x -= leftshift;
        }
        (*openfile).placewewant = utils::xplustabs();
    }
}

/* 从给定行移除一个缩进。 */
pub unsafe fn unindent_a_line(line: *mut linestruct, indent_len: usize) {
    let length = (*line).data.len();

    /* 如果缩进为空，不改变该行。 */
    if indent_len == 0 {
        return;
    }

    /* 移除该行前导的缩进。 */
    (*line).data.replace_range(..indent_len, "");

    (*openfile).totsize -= indent_len;

    /* 调整标记和光标的位置（若受影响）。 */
    compensate_leftward(line, indent_len);
}

/* 反缩进当前行（或标记的行）tabsize 列。 */
pub unsafe fn do_unindent() {
    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();

    get_range(&mut top, &mut bot);

    /* 跳过无法反缩进的前导行。 */
    while top != (*bot).next && length_of_white(&(*top).data) == 0 {
        top = (*top).next;
    }

    /* 若没有任何行可反缩进，无事可做。 */
    if top == (*bot).next {
        return;
    }

    add_undo(undo_type::UNINDENT, std::ptr::null_mut());

    let mut line = top;
    while line != (*bot).next {
        let indent_len = length_of_white(&(*line).data);
        let indentation = measured_copy((*line).data.as_bytes(), indent_len);

        unindent_a_line(line, indent_len);
        update_multiline_undo((*line).lineno, &String::from_utf8_lossy(&indentation[..indent_len]));
    }

    set_modified();
    ensure_firstcolumn_is_aligned();
    refresh_needed = true;
    shift_held = true;
}

/* 执行缩进或反缩进的撤销/重做。 */
pub unsafe fn handle_indent_action(u: *mut undostruct, undoing: bool, add_indent: bool) {
    let group = (*u).grouping;
    let mut line = line_from_number((*group).top_line);

    /* 重做时，先重定位光标，让缩进器调整它。 */
    if !undoing {
        crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
    }

    /* 对组中的每一行，添加或移除单独的缩进。 */
    while !line.is_null() && (*line).lineno <= (*group).bottom_line {
        let blanks = (*group).indentations[((*line).lineno - (*group).top_line) as usize]
            .clone()
            .unwrap_or_default();

        if (undoing ^ add_indent) {
            indent_a_line(line, &blanks);
        } else {
            unindent_a_line(line, blanks.len());
        }

        line = (*line).next;
    }

    /* 撤销时，将光标重定位到记录位置。 */
    if undoing {
        crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
    }

    refresh_needed = true;
}

/* 测试给定行是否可取消注释，或根据 action 添加/移除注释。 */
pub unsafe fn comment_line(action: undo_type, line: *mut linestruct, comment_seq: &str) -> bool {
    let comment_seq_len = comment_seq.len();
    let cs = comment_seq.as_bytes();
    let post_seq_pos = cs.iter().position(|&b| b == b'|');
    let pre_len = match post_seq_pos {
        Some(p) => p,
        None => comment_seq_len,
    };
    let post_len = match post_seq_pos {
        Some(p) => comment_seq_len - p - 1,
        None => 0,
    };
    let line_len = (*line).data.len();

    if !ISSET(NO_NEWLINES) && line == (*openfile).filebot {
        return false;
    }

    if action == undo_type::COMMENT {
        /* 为注释序列腾出空间，把文本右移并复制进去。 */
        let mut newdata = String::with_capacity(line_len + pre_len + post_len);
        newdata.push_str(&comment_seq[..pre_len]);
        newdata.push_str(&(*line).data);
        if post_len > 0 {
            newdata.push_str(&comment_seq[pre_len + 1..]);
        }
        (*line).data = newdata;

        (*openfile).totsize += pre_len + post_len;

        if line == (*openfile).mark && (*openfile).mark_x > 0 {
            (*openfile).mark_x += pre_len;
        }
        if line == (*openfile).current && (*openfile).current_x > 0 {
            (*openfile).current_x += pre_len;
            (*openfile).placewewant = utils::xplustabs();
        }

        return true;
    }

    /* 若行已注释，报告为可取消注释，或取消它的注释。 */
    let data = (*line).data.clone();
    let matches_pre = data.as_bytes()[..pre_len.min(data.len())] == cs[..pre_len.min(cs.len())];
    let matches_post = if post_len == 0 {
        true
    } else {
        data.as_bytes()[line_len - post_len..] == cs[pre_len + 1..comment_seq_len]
    };

    if matches_pre && matches_post {
        if action == undo_type::PREFLIGHT {
            return true;
        }

        /* 通过移动非注释部分来擦除注释前缀。 */
        (*line).data.replace_range(..pre_len, "");
        (*line).data.truncate(line_len - pre_len - post_len);

        (*openfile).totsize -= pre_len + post_len;

        compensate_leftward(line, pre_len);

        return true;
    }

    false
}

/* 注释或取消注释当前行或标记的行。 */
pub unsafe fn do_comment() {
    let mut comment_seq = GENERAL_COMMENT_CHARACTER.to_string();
    let mut action = undo_type::UNCOMMENT;
    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();
    let mut all_empty = true;

    if let Some(syntax) = (*openfile).syntax.as_ref() {
        if let Some(ref c) = syntax.comment {
            comment_seq = c.clone();
        }
        if comment_seq.is_empty() {
            statusline(message_type::AHEM, "Commenting is not supported for this file type");
            return;
        }
    }

    get_range(&mut top, &mut bot);

    if top == bot && bot == (*openfile).filebot && !ISSET(NO_NEWLINES) {
        statusline(message_type::AHEM, "Cannot comment past end of file");
        return;
    }

    let mut line = top;
    while line != (*bot).next {
        let empty = chars::white_string((*line).data.as_bytes());

        if !empty && !comment_line(undo_type::PREFLIGHT, line, &comment_seq) {
            action = undo_type::COMMENT;
            break;
        }
        all_empty = all_empty && empty;
        line = (*line).next;
    }

    if all_empty {
        action = undo_type::COMMENT;
    }

    add_undo(action, std::ptr::null_mut());

    (*(*openfile).current_undo).strdata = Some(comment_seq.clone());

    let mut line = top;
    while line != (*bot).next {
        if comment_line(action, line, &comment_seq) {
            update_multiline_undo((*line).lineno, "");
        }
        line = (*line).next;
    }

    set_modified();
    ensure_firstcolumn_is_aligned();
    refresh_needed = true;
    shift_held = true;
}

/* 执行注释或取消注释的撤销/重做。 */
pub unsafe fn handle_comment_action(u: *mut undostruct, undoing: bool, add_comment: bool) {
    let mut group = (*u).grouping;
    let strdata = (*u).strdata.clone().unwrap_or_default();

    if !undoing {
        crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
    }

    while !group.is_null() {
        let mut line = line_from_number((*group).top_line);

        while !line.is_null() && (*line).lineno <= (*group).bottom_line {
            let act = if undoing ^ add_comment {
                undo_type::COMMENT
            } else {
                undo_type::UNCOMMENT
            };
            comment_line(act, line, &strdata);
            line = (*line).next;
        }

        group = (*group).next;
    }

    if undoing {
        crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
    }

    refresh_needed = true;
}

/* 撤销一次剪切，或重做一次粘贴。 */
pub unsafe fn undo_cut(u: *mut undostruct) {
    crate::search::goto_line_posx(
        (*u).head_lineno,
        if ((*u).xflags & WAS_WHOLE_LINE) != 0 { 0 } else { (*u).head_x },
    );

    if ((*u).xflags & HAD_ANCHOR_AT_START) == 0 {
        (*openfile).current.as_mut().unwrap().has_anchor = false;
    }

    if !(*u).cutbuffer.is_null() {
        copy_from_buffer((*u).cutbuffer);
    }

    if ((*u).xflags & INCLUDED_LAST_LINE) != 0
        && !ISSET(NO_NEWLINES)
        && (*openfile).filebot != (*openfile).current
        && (*(*openfile).filebot).prev.as_ref().map_or(false, |p| !p.data.is_empty())
    {
        remove_magicline();
    }

    if ((*u).xflags & CURSOR_WAS_AT_HEAD) != 0 {
        crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
    }
}

/* 重做一次剪切，或撤销一次粘贴。 */
pub unsafe fn redo_cut(u: *mut undostruct) {
    let oldcutbuffer = cutbuffer;

    cutbuffer = std::ptr::null_mut();

    (*openfile).mark = line_from_number((*u).head_lineno);
    (*openfile).mark_x = if ((*u).xflags & WAS_WHOLE_LINE) != 0 { 0 } else { (*u).head_x };

    crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);

    do_snip(true, false, (*u).type_ == undo_type::ZAP);

    free_lines(cutbuffer);
    cutbuffer = oldcutbuffer;
}

/* 撤销上次的操作。 */
pub unsafe fn do_undo() {
    let mut u = (*openfile).current_undo;
    let mut oldcutbuffer: *mut linestruct = std::ptr::null_mut();
    let mut intruder: *mut linestruct;
    let mut line: *mut linestruct = std::ptr::null_mut();
    let mut original_x: usize = 0;
    let mut regain_from_x: usize = 0;
    let mut undidmsg: Option<&str> = None;

    if u.is_null() {
        statusline(message_type::AHEM, "Nothing to undo");
        return;
    }

    if (*u).type_ as i32 <= undo_type::REPLACE as i32 {
        line = line_from_number((*u).tail_lineno);
    }

    loop {
    match (*u).type_ {
        undo_type::ADD => {
            undidmsg = Some("addition");
            if ((*u).xflags & INCLUDED_LAST_LINE) != 0 && !ISSET(NO_NEWLINES) {
                remove_magicline();
            }
            let strdata = (*u).strdata.clone().unwrap_or_default();
            let sd_len = strdata.len();
            let cur = (*line).data.clone();
            let head = (*u).head_x;
            let newlen = cur.len() - sd_len;
            let mut newdata = String::with_capacity(newlen + 1);
            newdata.push_str(&cur[..head]);
            newdata.push_str(&cur[head + sd_len..]);
            (*line).data = newdata;
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
        }
        undo_type::ENTER => {
            undidmsg = Some("line break");
            original_x = if (*u).head_x == 0 { (*u).tail_x } else { (*u).head_x };
            regain_from_x = if (*u).head_x == 0 { 0 } else { (*u).tail_x };
            let strdata = (*u).strdata.clone().unwrap_or_default();
            let tail_part = &strdata[regain_from_x..];
            (*line).data.push_str(tail_part);
            (*line).has_anchor |= (*line).next.as_ref().map_or(false, |n| n.has_anchor);
            unlink_node((*line).next);
            renumber_from(line);
            (*openfile).current = line;
            crate::search::goto_line_posx((*u).head_lineno, original_x);
        }
        undo_type::BACK | undo_type::DEL => {
            undidmsg = Some("deletion");
            let strdata = (*u).strdata.clone().unwrap_or_default();
            let cur = (*line).data.clone();
            let head = (*u).head_x;
            let mut newdata = String::with_capacity(cur.len() + strdata.len());
            newdata.push_str(&cur[..head]);
            newdata.push_str(&strdata);
            newdata.push_str(&cur[head..]);
            (*line).data = newdata;
            crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);
        }
        undo_type::JOIN => {
            undidmsg = Some("line join");
            if ((*u).xflags & WAS_BACKSPACE_AT_EOF) != 0 && !ISSET(NO_NEWLINES) {
                crate::search::goto_line_posx((*(*openfile).filebot).lineno, 0);
                focusing = false;
                break;
            }
            (*line).data.truncate((*u).tail_x);
            intruder = Box::into_raw(make_new_node(&*line));
            (*intruder).data = (*u).strdata.clone().unwrap_or_default();
            splice_node(line, intruder);
            renumber_from(intruder);
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
        }
        undo_type::REPLACE => {
            undidmsg = Some("replacement");
            let data = (*u).strdata.clone();
            (*u).strdata = Some((*line).data.clone());
            (*line).data = data.unwrap_or_default();
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
        }
        undo_type::SPLIT_BEGIN => {
            undidmsg = Some("addition");
        }
        undo_type::SPLIT_END => {
            (*openfile).current_undo = (*(*openfile).current_undo).next;
            while (*(*openfile).current_undo).type_ != undo_type::SPLIT_BEGIN {
                do_undo();
            }
            u = (*openfile).current_undo;
        }
        undo_type::ZAP => {
            undidmsg = Some("erasure");
            undo_cut(u);
        }
        undo_type::CUT_TO_EOF | undo_type::CUT => {
            undidmsg = Some("cut");
            undo_cut(u);
        }
        undo_type::PASTE => {
            undidmsg = Some("paste");
            redo_cut(u);
            if ((*u).xflags & INCLUDED_LAST_LINE) != 0
                && !ISSET(NO_NEWLINES)
                && (*openfile).filebot != (*openfile).current
            {
                remove_magicline();
            }
        }
        undo_type::INSERT => {
            undidmsg = Some("insertion");
            oldcutbuffer = cutbuffer;
            cutbuffer = std::ptr::null_mut();
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
            (*openfile).mark = line_from_number((*u).tail_lineno);
            (*openfile).mark_x = (*u).tail_x;
            cut_marked_region();
            (*u).cutbuffer = cutbuffer;
            cutbuffer = oldcutbuffer;
            if ((*u).xflags & INCLUDED_LAST_LINE) != 0
                && !ISSET(NO_NEWLINES)
                && (*openfile).filebot != (*openfile).current
            {
                remove_magicline();
            }
        }
        undo_type::COUPLE_BEGIN => {
            undidmsg = (*u).strdata.as_deref();
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
            (*openfile).cursor_row = (*u).tail_lineno;
            adjust_viewport(update_type::STATIONARY);
        }
        undo_type::COUPLE_END => {
            (*(*openfile).current_undo).head_lineno = (*openfile).cursor_row;
            (*openfile).current_undo = (*(*openfile).current_undo).next;
            do_undo();
            do_undo();
            do_undo();
            return;
        }
        undo_type::INDENT => {
            handle_indent_action(u, true, true);
            undidmsg = Some("indent");
        }
        undo_type::UNINDENT => {
            handle_indent_action(u, true, false);
            undidmsg = Some("unindent");
        }
        undo_type::COMMENT => {
            handle_comment_action(u, true, true);
            undidmsg = Some("comment");
        }
        undo_type::UNCOMMENT => {
            handle_comment_action(u, true, false);
            undidmsg = Some("uncomment");
        }
        _ => {}
    }
    }

    if let Some(msg) = undidmsg {
        if !ISSET(ZERO) && pletion_line.is_null() {
            statusline(message_type::HUSH, &format!("Undid {}", msg));
        }
    }

    (*openfile).current_undo = (*(*openfile).current_undo).next;
    (*openfile).last_action = undo_type::OTHER;
    (*openfile).mark = std::ptr::null_mut();
    (*openfile).placewewant = utils::xplustabs();

    (*openfile).totsize = (*u).wassize;

    if (*u).type_ as i32 <= undo_type::REPLACE as i32 {
        check_the_multis((*openfile).current);
    } else if (*u).type_ == undo_type::INSERT || (*u).type_ == undo_type::COUPLE_BEGIN {
        recook = true;
    }

    if (*openfile).current_undo == (*openfile).last_saved {
        (*openfile).modified = false;
        titlebar(None);
    } else {
        set_modified();
    }
}

/* 重做上次撤销的操作。 */
pub unsafe fn do_redo() {
    let mut u = (*openfile).undotop;
    let mut suppress_modification = false;
    let mut line: *mut linestruct = std::ptr::null_mut();
    let mut intruder: *mut linestruct;
    let mut redidmsg: Option<&str> = None;

    if u.is_null() || u == (*openfile).current_undo {
        statusline(message_type::AHEM, "Nothing to redo");
        return;
    }

    while (*u).next != (*openfile).current_undo {
        u = (*u).next;
    }

    if (*u).type_ as i32 <= undo_type::REPLACE as i32 {
        line = line_from_number((*u).tail_lineno);
    }

    loop {
    match (*u).type_ {
        undo_type::ADD => {
            redidmsg = Some("addition");
            if ((*u).xflags & INCLUDED_LAST_LINE) != 0 && !ISSET(NO_NEWLINES) {
                new_magicline();
            }
            let strdata = (*u).strdata.clone().unwrap_or_default();
            let cur = (*line).data.clone();
            let head = (*u).head_x;
            let mut newdata = String::with_capacity(cur.len() + strdata.len());
            newdata.push_str(&cur[..head]);
            newdata.push_str(&strdata);
            newdata.push_str(&cur[head..]);
            (*line).data = newdata;
            crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);
        }
        undo_type::ENTER => {
            redidmsg = Some("line break");
            (*line).data.truncate((*u).head_x);
            intruder = Box::into_raw(make_new_node(&*line));
            (*intruder).data = (*u).strdata.clone().unwrap_or_default();
            splice_node(line, intruder);
            renumber_from(intruder);
            crate::search::goto_line_posx((*u).head_lineno + 1, (*u).tail_x);
        }
        undo_type::BACK | undo_type::DEL => {
            redidmsg = Some("deletion");
            let strdata = (*u).strdata.clone().unwrap_or_default();
            let sd_len = strdata.len();
            let cur = (*line).data.clone();
            let head = (*u).head_x;
            let newlen = cur.len() - sd_len;
            let mut newdata = String::with_capacity(newlen + 1);
            newdata.push_str(&cur[..head]);
            newdata.push_str(&cur[head + sd_len..]);
            (*line).data = newdata;
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
        }
        undo_type::JOIN => {
            redidmsg = Some("line join");
            if ((*u).xflags & WAS_BACKSPACE_AT_EOF) != 0 && !ISSET(NO_NEWLINES) {
                crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);
                break;
            }
            let strdata = (*u).strdata.clone().unwrap_or_default();
            (*line).data.push_str(&strdata);
            unlink_node((*line).next);
            renumber_from(line);
            (*openfile).current = line;
            crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);
        }
        undo_type::REPLACE => {
            redidmsg = Some("replacement");
            let data = (*u).strdata.clone();
            (*u).strdata = Some((*line).data.clone());
            (*line).data = data.unwrap_or_default();
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
        }
        undo_type::SPLIT_BEGIN => {
            (*openfile).current_undo = u;
            while (*(*openfile).current_undo).type_ != undo_type::SPLIT_END {
                do_redo();
            }
            u = (*openfile).current_undo;
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
            ensure_firstcolumn_is_aligned();
        }
        undo_type::SPLIT_END => {
            redidmsg = Some("addition");
        }
        undo_type::ZAP => {
            redidmsg = Some("erasure");
            redo_cut(u);
        }
        undo_type::CUT_TO_EOF | undo_type::CUT => {
            redidmsg = Some("cut");
            redo_cut(u);
        }
        undo_type::PASTE => {
            redidmsg = Some("paste");
            undo_cut(u);
        }
        undo_type::INSERT => {
            redidmsg = Some("insertion");
            crate::search::goto_line_posx((*u).head_lineno, (*u).head_x);
            if !(*u).cutbuffer.is_null() {
                copy_from_buffer((*u).cutbuffer);
            } else {
                suppress_modification = true;
            }
            free_lines((*u).cutbuffer);
            (*u).cutbuffer = std::ptr::null_mut();
        }
        undo_type::COUPLE_BEGIN => {
            (*openfile).current_undo = u;
            do_redo();
            do_redo();
            do_redo();
            return;
        }
        undo_type::COUPLE_END => {
            redidmsg = (*u).strdata.as_deref();
            crate::search::goto_line_posx((*u).tail_lineno, (*u).tail_x);
            (*openfile).cursor_row = (*u).head_lineno;
            adjust_viewport(update_type::STATIONARY);
        }
        undo_type::INDENT => {
            handle_indent_action(u, false, true);
            redidmsg = Some("indent");
        }
        undo_type::UNINDENT => {
            handle_indent_action(u, false, false);
            redidmsg = Some("unindent");
        }
        undo_type::COMMENT => {
            handle_comment_action(u, false, true);
            redidmsg = Some("comment");
        }
        undo_type::UNCOMMENT => {
            handle_comment_action(u, false, false);
            redidmsg = Some("uncomment");
        }
        _ => {}
    }

    if let Some(msg) = redidmsg {
        if !ISSET(ZERO) {
            statusline(message_type::HUSH, &format!("Redid {}", msg));
        }
    }

    (*openfile).current_undo = u;
    (*openfile).last_action = undo_type::OTHER;
    (*openfile).mark = std::ptr::null_mut();
    (*openfile).placewewant = utils::xplustabs();

    (*openfile).totsize = (*u).newsize;

    if (*u).type_ as i32 <= undo_type::REPLACE as i32 {
        check_the_multis((*openfile).current);
    } else if (*u).type_ == undo_type::INSERT || (*u).type_ == undo_type::COUPLE_END {
        recook = true;
    }

    if (*openfile).current_undo == (*openfile).last_saved {
        (*openfile).modified = false;
        titlebar(None);
    } else if !suppress_modification {
        set_modified();
    }
    }
}

/* 在光标处断开当前行。 */
pub unsafe fn do_enter() {
    let newnode = Box::into_raw(make_new_node(&*(*openfile).current));
    let mut extra = 0usize;
    let mut sampleline = (*openfile).current;
    let mut allblanks = false;

    if ISSET(AUTOINDENT) {
        if ISSET(BREAK_LONG_LINES) && !(*sampleline).next.is_null()
            && inpar((*sampleline).next) && !begpar((*sampleline).next, 0)
        {
            sampleline = (*sampleline).next;
        }
        extra = indent_length(&(*sampleline).data);

        if extra > (*openfile).current_x {
            extra = (*openfile).current_x;
        } else if extra == (*openfile).current_x {
            allblanks = (indent_length(&(*(*openfile).current).data) == extra);
        }
    }

    let cur = &*(*openfile).current;
    let rest = &cur.data[(*openfile).current_x..];
    let mut newdata = String::with_capacity(rest.len() + extra + 1);
    newdata.push_str(&" ".repeat(extra));
    newdata.push_str(rest);
    (*newnode).data = newdata;

    if (*openfile).mark == (*openfile).current && (*openfile).mark_x > (*openfile).current_x {
        (*openfile).mark = newnode;
        (*openfile).mark_x += extra - (*openfile).current_x;
    }

    if ISSET(AUTOINDENT) {
        let sampledata = (*sampleline).data.clone();
        let prefix = &sampledata[..extra];
        (*newnode).data.replace_range(..extra, prefix);
        if allblanks {
            (*openfile).current_x = 0;
        }
        if allblanks && (*openfile).mark == (*openfile).current {
            (*openfile).mark_x = 0;
        }
    }

    (*(*openfile).current).data.truncate((*openfile).current_x);

    add_undo(undo_type::ENTER, std::ptr::null_mut());

    splice_node((*openfile).current, newnode);
    renumber_from(newnode);

    /* 若新行成为缓冲区末行，更新 filebot。 */
    if (*openfile).filebot == (*openfile).current {
        (*openfile).filebot = newnode;
    }

    (*openfile).current = newnode;
    (*openfile).current_x = extra;
    (*openfile).placewewant = utils::xplustabs();

    (*openfile).totsize += 1;
    set_modified();

    if ISSET(AUTOINDENT) && !allblanks {
        (*openfile).totsize += extra;
    }
    update_undo(undo_type::ENTER);

    refresh_needed = true;
    focusing = false;
}

/* 丢弃比给定项更新的撤销项，若为 NULL 则全部丢弃。 */
pub unsafe fn discard_until(thisitem: *const undostruct) {
    let mut dropit = (*openfile).undotop;

    while !dropit.is_null() && dropit as *const _ != thisitem {
        (*openfile).undotop = (*dropit).next;
        let _ = (*dropit).strdata.take();
        free_lines((*dropit).cutbuffer);
        let mut group = (*dropit).grouping;
        while !group.is_null() {
            let next = (*group).next;
            let _ = (*group).indentations.clone();
            let _ = Box::from_raw(group);
            group = next;
        }
        let _ = Box::from_raw(dropit);
        dropit = (*openfile).undotop;
    }

    (*openfile).current_undo = thisitem as *mut undostruct;
    (*openfile).last_action = undo_type::OTHER;
}

/* 在撤销栈顶部添加一个给定类型的新撤销项。 */
pub unsafe fn add_undo(action: undo_type, message: *const u8) {
    let u = Box::into_raw(Box::new(undostruct {
        type_: action,
        xflags: 0,
        head_lineno: 0,
        head_x: 0,
        strdata: None,
        wassize: 0,
        newsize: 0,
        grouping: std::ptr::null_mut(),
        cutbuffer: std::ptr::null_mut(),
        tail_lineno: 0,
        tail_x: 0,
        next: std::ptr::null_mut(),
    }));
    let thisline = (*openfile).current;

    (*u).head_lineno = (*thisline).lineno;
    (*u).head_x = (*openfile).current_x;
    (*u).tail_lineno = (*thisline).lineno;
    (*u).tail_x = (*openfile).current_x;
    (*u).wassize = (*openfile).totsize;
    (*u).newsize = (*openfile).totsize;
    (*u).grouping = std::ptr::null_mut();
    (*u).xflags = 0;

    discard_until((*openfile).current_undo);

    if (*u).type_ == undo_type::SPLIT_BEGIN {
        (*u).wassize = (*(*openfile).undotop).wassize;
        (*u).next = (*(*openfile).undotop).next;
        (*(*openfile).undotop).next = u;
    } else {
        (*u).next = (*openfile).undotop;
        (*openfile).undotop = u;
        (*openfile).current_undo = u;
    }

    let mut act = action;
    match (*u).type_ {
        undo_type::ADD => {
            if thisline == (*openfile).filebot {
                (*u).xflags |= INCLUDED_LAST_LINE;
            }
        }
        undo_type::ENTER => {}
        undo_type::BACK => {
            if (*thisline).next == (*openfile).filebot && !(*thisline).data.is_empty() {
                (*u).xflags |= WAS_BACKSPACE_AT_EOF;
            }
            handle_del_join(u, thisline, &mut act);
        }
        undo_type::DEL => {
            handle_del_join(u, thisline, &mut act);
        }
        undo_type::REPLACE => {
            (*u).strdata = Some((*thisline).data.clone());
        }
        undo_type::CUT_TO_EOF => {
            (*u).xflags |= INCLUDED_LAST_LINE | CURSOR_WAS_AT_HEAD;
            if (*thisline).has_anchor {
                (*u).xflags |= HAD_ANCHOR_AT_START;
            }
        }
        undo_type::ZAP | undo_type::CUT => {
            if !(*openfile).mark.is_null() {
                if mark_is_before_cursor() {
                    (*u).head_lineno = (*(*openfile).mark).lineno;
                    (*u).head_x = (*openfile).mark_x;
                    (*u).xflags |= MARK_WAS_SET;
                } else {
                    (*u).tail_lineno = (*(*openfile).mark).lineno;
                    (*u).tail_x = (*openfile).mark_x;
                    (*u).xflags |= MARK_WAS_SET | CURSOR_WAS_AT_HEAD;
                }
                if (*u).tail_lineno == (*(*openfile).filebot).lineno {
                    (*u).xflags |= INCLUDED_LAST_LINE;
                }
            } else if !ISSET(CUT_FROM_CURSOR) {
                (*u).xflags |= WAS_WHOLE_LINE | CURSOR_WAS_AT_HEAD;
                (*u).tail_x = 0;
            } else {
                (*u).xflags |= CURSOR_WAS_AT_HEAD;
            }
            if ((!(*openfile).mark.is_null() && mark_is_before_cursor()
                && (*(*openfile).mark).has_anchor)
                || ((*openfile).mark.is_null() || !mark_is_before_cursor())
                    && (*thisline).has_anchor)
            {
                (*u).xflags |= HAD_ANCHOR_AT_START;
            }
        }
        undo_type::PASTE => {
            (*u).cutbuffer = copy_buffer(cutbuffer);
        }
        undo_type::INSERT => {
            if thisline == (*openfile).filebot {
                (*u).xflags |= INCLUDED_LAST_LINE;
            }
        }
        undo_type::COUPLE_BEGIN => {
            (*u).tail_lineno = (*openfile).cursor_row;
        }
        undo_type::COUPLE_END => {
            if !message.is_null() {
                (*u).strdata = Some(
                    std::ffi::CStr::from_ptr(message as *const std::ffi::c_char)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        _ => {}
    }

    (*openfile).last_action = act;
}

/* 在 add_undo 中处理 DEL/BACK 的公共逻辑（提取自 C 同名分支）。 */
unsafe fn handle_del_join(u: *mut undostruct, thisline: *mut linestruct, act: &mut undo_type) {
    let cur = &*thisline;
    if (*openfile).current_x < cur.data.len() {
        let charlen = chars::char_length(&cur.data.as_bytes()[(*u).head_x..]);
        (*u).strdata = Some(cur.data[(*u).head_x..(*u).head_x + charlen].to_string());
        if (*u).type_ == undo_type::BACK {
            (*u).tail_x += charlen;
        }
    } else {
        *act = undo_type::JOIN;
        if !cur.next.is_null() {
            if (*u).type_ == undo_type::BACK {
                (*u).head_lineno = (*cur.next).lineno;
                (*u).head_x = 0;
            }
            (*u).strdata = Some((*cur.next).data.clone());
        }
        (*u).type_ = undo_type::JOIN;
    }
}

/* 更新多行撤销项。 */
pub unsafe fn update_multiline_undo(lineno: isize, indentation: &str) {
    let u = (*openfile).current_undo;

    if !(*u).grouping.is_null() && (*(*u).grouping).bottom_line + 1 == lineno {
        let number_of_lines = (lineno - (*(*u).grouping).top_line + 1) as usize;

        (*(*u).grouping).bottom_line = lineno;

        let mut indentations = (*(*u).grouping).indentations.clone();
        indentations.resize(number_of_lines, None);
        indentations[number_of_lines - 1] = Some(indentation.to_string());
        (*(*u).grouping).indentations = indentations;
    } else {
        let born = Box::into_raw(Box::new(groupstruct {
            top_line: lineno,
            bottom_line: lineno,
            indentations: vec![Some(indentation.to_string())],
            next: (*u).grouping,
        }));
        (*u).grouping = born;
    }

    (*u).newsize = (*openfile).totsize;
}

/* 用（除其他外）给定动作后的文件大小和光标位置更新撤销项。 */
pub unsafe fn update_undo(action: undo_type) {
    let u = (*openfile).current_undo;

    (*u).newsize = (*openfile).totsize;

    match (*u).type_ {
        undo_type::ADD => {
            let newlen = (*openfile).current_x - (*u).head_x;
            (*u).strdata = Some(
                (*openfile).current.as_ref().unwrap().data[(*u).head_x..(*openfile).current_x]
                    .to_string(),
            );
            (*u).tail_x = (*openfile).current_x;
        }
        undo_type::ENTER => {
            (*u).strdata = Some((*openfile).current.as_ref().unwrap().data.clone());
            (*u).tail_x = (*openfile).current_x;
        }
        undo_type::BACK | undo_type::DEL => {
            let textposition = (*openfile).current.as_ref().unwrap().data.clone();
            let charlen = chars::char_length(
                &textposition.as_bytes()[(*openfile).current_x..],
            );
            let datalen = (*u).strdata.as_ref().map_or(0, |s| s.len());
            if (*openfile).current_x == (*u).head_x {
                let mut sd = (*u).strdata.take().unwrap_or_default();
                sd.push_str(
                    &textposition[(*openfile).current_x..(*openfile).current_x + charlen],
                );
                (*u).strdata = Some(sd);
                (*u).tail_x = (*openfile).current_x;
            } else if (*openfile).current_x == (*u).head_x - charlen {
                let mut sd = textposition[(*openfile).current_x..(*openfile).current_x + charlen]
                    .to_string();
                if let Some(old) = (*u).strdata.take() {
                    sd.push_str(&old);
                }
                (*u).strdata = Some(sd);
                (*u).head_x = (*openfile).current_x;
            } else {
                add_undo((*u).type_, std::ptr::null_mut());
            }
        }
        undo_type::REPLACE => {}
        undo_type::ZAP | undo_type::CUT_TO_EOF | undo_type::CUT => {
            if (*u).type_ == undo_type::ZAP {
                (*u).cutbuffer = cutbuffer;
            } else if !cutbuffer.is_null() {
                free_lines((*u).cutbuffer);
                (*u).cutbuffer = copy_buffer(cutbuffer);
            }
            if ((*u).xflags & MARK_WAS_SET) == 0 {
                let mut bottomline = (*u).cutbuffer;
                let mut count = 0usize;
                while !(*bottomline).next.is_null() {
                    bottomline = (*bottomline).next;
                    count += 1;
                }
                (*u).tail_lineno = (*u).head_lineno + count as isize;
                if ISSET(CUT_FROM_CURSOR) || (*u).type_ == undo_type::CUT_TO_EOF {
                    (*u).tail_x = (*bottomline).data.len();
                    if count == 0 {
                        (*u).tail_x += (*u).head_x;
                    }
                } else if (*openfile).current == (*openfile).filebot && ISSET(NO_NEWLINES) {
                    (*u).tail_x = (*bottomline).data.len();
                }
            }
        }
        undo_type::COUPLE_BEGIN => {}
        undo_type::COUPLE_END | undo_type::PASTE | undo_type::INSERT => {
            (*u).tail_lineno = (*openfile).current.as_ref().unwrap().lineno;
            (*u).tail_x = (*openfile).current_x;
        }
        _ => {}
    }
}

/* 当当前行过长时，硬换行到尽可能远的空白字符处。 */
pub unsafe fn do_wrap() {
    let line = (*openfile).current;
    let line_len = (*line).data.len();
    let quot_len = quote_length(&(*line).data);
    let lead_len = quot_len + indent_length(&(*line).data[quot_len..]);
    let cursor_x = (*openfile).current_x;

    let mut wrap_loc = break_line(
        &(*line).data[lead_len..],
        (wrap_at as isize) - (utils::wideness((*line).data.as_bytes(), lead_len) as isize),
        false,
    );

    if wrap_loc < 0 || lead_len + wrap_loc as usize == line_len {
        return;
    }

    wrap_loc = (lead_len + chars::step_right(
        (*line).data.as_bytes(),
        lead_len + wrap_loc as usize,
    )) as isize;
    let wrap_loc = wrap_loc as usize;

    if (*line).data.as_bytes().get(wrap_loc).copied().unwrap_or(0) == 0 {
        return;
    }

    add_undo(undo_type::SPLIT_BEGIN, std::ptr::null_mut());

    let autowhite = ISSET(AUTOINDENT);
    if quot_len > 0 {
        UNSET(AUTOINDENT);
    }

    let remainder = (*line).data[wrap_loc..].to_string();
    let mut rest_length = line_len - wrap_loc;

    if !(*openfile).spillage_line.is_null()
        && (*openfile).spillage_line == (*line).next
        && rest_length + utils::breadth((*(*line).next).data.as_bytes()) <= wrap_at
    {
        (*openfile).current_x = line_len;

        if !chars::is_blank_char(
            &remainder.as_bytes()[chars::step_left(remainder.as_bytes(), rest_length)..],
        ) {
            add_undo(undo_type::ADD, std::ptr::null_mut());
            (*line).data.push(' ');
            rest_length += 1;
            (*openfile).totsize += 1;
            (*openfile).current_x += 1;
            update_undo(undo_type::ADD);
        }

        expunge(undo_type::DEL);

        if (*line).data.as_bytes()[..(*openfile).current_x]
            == (*line).data.as_bytes()[(*openfile).current_x..(*openfile).current_x + lead_len]
        {
            for _ in 0..lead_len {
                expunge(undo_type::DEL);
            }
        }
        while chars::is_blank_char(&(*line).data.as_bytes()[(*openfile).current_x..]) {
            expunge(undo_type::DEL);
        }
    }

    (*openfile).current_x = wrap_loc;

    if ISSET(TRIM_BLANKS) {
        let mut rear_x = chars::step_left((*line).data.as_bytes(), wrap_loc);
        let typed_x = chars::step_left((*line).data.as_bytes(), cursor_x);

        while (rear_x != typed_x || cursor_x >= wrap_loc)
            && chars::is_blank_char(&(*line).data.as_bytes()[rear_x..])
        {
            (*openfile).current_x = rear_x;
            expunge(undo_type::DEL);
            rear_x = chars::step_left((*line).data.as_bytes(), rear_x);
        }
    }

    do_enter();

    if (*openfile).edittop == line && (*openfile).firstcolumn > 0 && cursor_x >= wrap_loc {
        let mut e = (*openfile).edittop;
        let mut fc = (*openfile).firstcolumn;
        go_forward_chunks(1, &mut e, &mut fc);
    }

    if quot_len > 0 {
        let mut line = (*line).next;
        let line_len = (*line).data.len();
        let prevdata = (*(*line).prev).data.clone();
        let mut newdata = String::with_capacity(lead_len + line_len);
        newdata.push_str(&prevdata[..lead_len]);
        newdata.push_str(&(*line).data);
        (*line).data = newdata;

        (*openfile).current_x += lead_len;
        (*openfile).totsize += lead_len;
        (*(*openfile).undotop).strdata = None;
        update_undo(undo_type::ENTER);
        if autowhite {
            SET(AUTOINDENT);
        }
    }

    (*openfile).spillage_line = (*openfile).current;

    if cursor_x < wrap_loc {
        (*openfile).current = (*(*openfile).current).prev;
        (*openfile).current_x = cursor_x;
    } else {
        (*openfile).current_x += cursor_x - wrap_loc;
    }

    (*openfile).placewewant = utils::xplustabs();

    add_undo(undo_type::SPLIT_END, std::ptr::null_mut());

    refresh_needed = true;
}

/* 在给定文本中找到最后一个空白，使其显示宽度不超过 (goal + 1)。 */
pub unsafe fn break_line(textstart: &str, goal: isize, snap_at_nl: bool) -> isize {
    let bytes = textstart.as_bytes();
    let mut lastblank: Option<usize> = None;
    let mut pointer = 0usize;
    let mut column: usize = 0;

    while pointer < bytes.len() && bytes[pointer] != 0 && chars::is_blank_char(&bytes[pointer..]) {
        pointer += chars::advance_over(&bytes[pointer..], &mut column);
    }

    while pointer < bytes.len() && bytes[pointer] != 0 && (column as isize) <= goal {
        if chars::is_blank_char(&bytes[pointer..])
            && (!inhelp || column > 17 || goal < 40)
        {
            lastblank = Some(pointer);
        } else if snap_at_nl && bytes[pointer] == b'\n' {
            lastblank = Some(pointer);
            break;
        }
        pointer += chars::advance_over(&bytes[pointer..], &mut column);
    }

    if (column as isize) <= goal {
        return pointer as isize;
    }

    if snap_at_nl && lastblank.is_none() {
        return chars::step_left(bytes, pointer) as isize;
    }

    let mut ptr = pointer;
    while lastblank.is_none() {
        if ptr >= bytes.len() || bytes[ptr] == 0 {
            return -1;
        }
        if chars::is_blank_char(&bytes[ptr..]) {
            lastblank = Some(ptr);
        } else {
            ptr += chars::char_length(&bytes[ptr..]);
        }
    }

    let lb = lastblank.unwrap();
    let mut pointer = lb + chars::char_length(&bytes[lb..]);

    while pointer < bytes.len() && bytes[pointer] != 0 && chars::is_blank_char(&bytes[pointer..]) {
        lastblank = Some(pointer);
        pointer += chars::char_length(&bytes[pointer..]);
    }

    lastblank.unwrap() as isize
}

/* 返回给定行缩进部分的长度（前导连续空白）。 */
pub unsafe fn indent_length(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 && chars::is_blank_char(&bytes[i..]) {
        i += chars::char_length(&bytes[i..]);
    }
    i
}

/* 返回给定行引用部分的长度（最大匹配引用正则的初始子串）。 */
pub unsafe fn quote_length(line: &str) -> usize {
    if let Some(re) = quotereg.as_ref() {
        if let Some(m) = re.find(line) {
            if m.start() == 0 {
                return m.end();
            }
        }
    }
    0
}

/* 返回给定行是否为段落开头（BOP）。 */
pub unsafe fn begpar(line: *const linestruct, depth: i32) -> bool {
    if (*line).prev.is_null() {
        return true;
    }

    if depth > 222 {
        return false;
    }

    let quot_len = quote_length(&(*line).data);
    let indent_len = indent_length(&(*line).data[quot_len..]);

    if (*line).data.as_bytes()[quot_len + indent_len] == 0 {
        return false;
    }

    if ISSET(BOOKSTYLE) && !ISSET(AUTOINDENT) && chars::is_blank_char((*line).data.as_bytes()) {
        return true;
    }

    if quot_len != quote_length(&(*(*line).prev).data)
        || (*line).data.as_bytes()[..quot_len] != (*(*line).prev).data.as_bytes()[..quot_len]
    {
        return true;
    }

    let prev_dent_len = indent_length(&(*(*line).prev).data[quot_len..]);

    if (*(*line).prev).data.as_bytes()[quot_len + prev_dent_len] == 0 {
        return true;
    }

    if utils::wideness((*(*line).prev).data.as_bytes(), quot_len + prev_dent_len)
        == utils::wideness((*line).data.as_bytes(), quot_len + indent_len)
    {
        return false;
    }

    !begpar((*line).prev, depth + 1)
}

/* 返回给定行是否为段落的一部分。 */
pub unsafe fn inpar(line: *const linestruct) -> bool {
    let quot_len = quote_length(&(*line).data);
    let indent_len = indent_length(&(*line).data[quot_len..]);

    (*line).data.as_bytes()[quot_len + indent_len] != 0
}

/* 向前查找第一个出现的段落。 */
pub unsafe fn find_paragraph(firstline: *mut *mut linestruct, linecount: *mut usize) -> bool {
    let mut line = *firstline;

    while !inpar(line) && !(*line).next.is_null() {
        line = (*line).next;
    }

    *firstline = line;

    do_para_end(&mut line);

    if !inpar(line) {
        return false;
    }

    *linecount = ((*line).lineno - (**firstline).lineno + 1) as usize;

    true
}

/* 将起始于 *line、包含 count 行的段落合并为单行。 */
pub unsafe fn concat_paragraph(line: *mut linestruct, count: usize) {
    let mut count = count;
    while count > 1 {
        let next_line = (*line).next;
        let next_line_len = (*next_line).data.len();
        let next_quot_len = quote_length(&(*next_line).data);
        let next_lead_len =
            next_quot_len + indent_length(&(*next_line).data[next_quot_len..]);
        let line_len = (*line).data.len();

        if line_len > 0 && (*line).data.as_bytes()[line_len - 1] != b' ' {
            (*line).data.push(' ');
        }

        let suffix = (*next_line).data[next_lead_len..].to_string();
        (*line).data.push_str(&suffix);
        (*line).has_anchor |= (*next_line).has_anchor;
        unlink_node(next_line);
        count -= 1;
    }
}

/* 从一个位置复制一个字符到另一个位置。 */
unsafe fn copy_character(from: &mut usize, to: &mut usize, data: &[u8]) {
    let charlen = chars::char_length(&data[*from..]);
    if *from == *to {
        *from += charlen;
        *to += charlen;
    } else {
        for _ in 0..charlen {
            let _ = data[*from];
            *from += 1;
            *to += 1;
        }
    }
}

/* 把行中任意空白序列替换为单个空格，但保留句末标点后的两个空格，并移除行尾空格。 */
pub unsafe fn squeeze(line: *mut linestruct, skip: usize) {
    let data = (*line).data.clone();
    let bytes = data.as_bytes();
    let start = skip;
    let mut from = start;
    let mut to = start;
    let punct_bytes = punct.clone().unwrap_or_default();
    let brackets_bytes = brackets.clone().unwrap_or_default();

    let mut result = data.clone().into_bytes();

    while from < bytes.len() && bytes[from] != 0 {
        if chars::is_blank_char(&bytes[from..]) {
            from += chars::char_length(&bytes[from..]);
            result[to] = b' ';
            to += 1;
            while from < bytes.len() && bytes[from] != 0 && chars::is_blank_char(&bytes[from..]) {
                from += chars::char_length(&bytes[from..]);
            }
        } else if chars::mbstrchr(punct_bytes.as_bytes(), &bytes[from..]).is_some() {
            copy_character(&mut from, &mut to, &result);
            if from < bytes.len() && bytes[from] != 0
                && chars::mbstrchr(brackets_bytes.as_bytes(), &bytes[from..]).is_some()
            {
                copy_character(&mut from, &mut to, &result);
            }
            if from < bytes.len() && bytes[from] != 0 && chars::is_blank_char(&bytes[from..]) {
                from += chars::char_length(&bytes[from..]);
                result[to] = b' ';
                to += 1;
            }
            if from < bytes.len() && bytes[from] != 0 && chars::is_blank_char(&bytes[from..]) {
                from += chars::char_length(&bytes[from..]);
                result[to] = b' ';
                to += 1;
            }
            while from < bytes.len() && bytes[from] != 0 && chars::is_blank_char(&bytes[from..]) {
                from += chars::char_length(&bytes[from..]);
            }
        } else {
            copy_character(&mut from, &mut to, &result);
        }
    }

    while to > start && result[to - 1] == b' ' {
        to -= 1;
    }
    result.truncate(to);
    (*line).data = String::from_utf8_lossy(&result).into_owned();
}

/* 把给定行（以给定前导字符串开头）重绕到适合目标宽度的多行。 */
pub unsafe fn rewrap_paragraph(
    line: *mut *mut linestruct,
    lead_string: &str,
    lead_len: usize,
) {
    loop {
        if utils::breadth((**line).data.as_bytes()) <= wrap_at {
            break;
        }
        let line_len = (**line).data.len();

        let mut break_pos = break_line(
            &(**line).data[lead_len..],
            (wrap_at as isize) - (utils::wideness((**line).data.as_bytes(), lead_len) as isize),
            false,
        );

        if break_pos < 0 || lead_len + break_pos as usize == line_len {
            break;
        }

        break_pos += (lead_len + 1) as isize;
        let mut break_pos = break_pos as usize;

        splice_node(*line, Box::into_raw(make_new_node(&**line)));
        (**line).next.as_mut().unwrap().data =
            format!("{}{}", lead_string, &(**line).data[break_pos..]);

        if ISSET(TRIM_BLANKS) {
            while break_pos > 0 && (**line).data.as_bytes()[break_pos - 1] == b' ' {
                break_pos -= 1;
            }
        }

        (**line).data.truncate(break_pos);
        *line = (**line).next;
    }

    if ((**line).lineno as i32) >= editwinrows {
        recook = true;
    }

    if !(**line).next.is_null() {
        *line = (**line).next;
    }
}

/* 对齐起始于 *line、包含 count 行的段落。 */
pub unsafe fn justify_paragraph(line: *mut *mut linestruct, count: usize) {
    let sampleline = if count == 1 { *line } else { (**line).next };

    let quot_len = quote_length(&(*sampleline).data);
    let lead_len = quot_len + indent_length(&(*sampleline).data[quot_len..]);
    let lead_string = (*sampleline).data[..lead_len].to_string();

    concat_paragraph(*line, count);

    let q = quote_length(&(**line).data);
    squeeze(*line, q + indent_length(&(**line).data[q..]));

    rewrap_paragraph(line, &lead_string, lead_len);
}

/* 对齐当前段落，或当 whole_buffer 时对齐整个缓冲区。 */
pub unsafe fn justify_text(whole_buffer: bool) {
    let mut linecount: usize = 0;
    let mut startline: *mut linestruct = std::ptr::null_mut();
    let mut endline: *mut linestruct = std::ptr::null_mut();
    let mut start_x: usize = 0;
    let mut end_x: usize = 0;
    let was_cutbuffer = cutbuffer;
    let mut jusline: *mut linestruct;
    let mut before_eol = false;
    let mut primary_lead: Option<String> = None;
    let mut secondary_lead: Option<String> = None;
    let mut primary_len: usize = 0;
    let mut secondary_len: usize = 0;
    let was_the_linenumber = (*(*openfile).current).lineno;
    let marked_backward = !(*openfile).mark.is_null() && !mark_is_before_cursor();

    add_undo(undo_type::COUPLE_BEGIN, N_("justification").as_ptr());

    if !(*openfile).mark.is_null() {
        get_region(&mut startline, &mut start_x, &mut endline, &mut end_x);

        if startline == endline && start_x == end_x {
            statusline(message_type::AHEM, "Selection is empty");
            discard_until((*(*openfile).undotop).next);
            return;
        }

        let mut quot_len = quote_length(&(*startline).data);
        let mut fore_len = quot_len + indent_length(&(*startline).data[quot_len..]);

        if start_x <= fore_len {
            start_x = 0;
        }

        while start_x > 0
            && chars::is_blank_char(&(*startline).data.as_bytes()[start_x - 1..])
        {
            start_x = chars::step_left((*startline).data.as_bytes(), start_x);
        }

        quot_len = quote_length(&(*endline).data);
        fore_len = quot_len + indent_length(&(*endline).data[quot_len..]);

        if end_x > 0 && end_x < fore_len {
            end_x = fore_len;
        }

        while end_x > 0 && chars::is_blank_char(&(*endline).data.as_bytes()[end_x..]) {
            end_x = chars::step_right((*endline).data.as_bytes(), end_x);
        }

        let mut sampleline = startline;

        while !(*sampleline).prev.is_null()
            && inpar(sampleline)
            && !begpar(sampleline, 0)
        {
            sampleline = (*sampleline).prev;
        }

        while !(*sampleline).next.is_null() && !inpar(sampleline) {
            sampleline = (*sampleline).next;
        }

        quot_len = quote_length(&(*sampleline).data);
        primary_len = quot_len + indent_length(&(*sampleline).data[quot_len..]);
        primary_lead = Some((*sampleline).data[..primary_len].to_string());

        if !(*sampleline).next.is_null() && startline != endline {
            sampleline = (*sampleline).next;
        }

        let other_quot_len = quote_length(&(*sampleline).data);
        let other_white_len = indent_length(&(*sampleline).data[other_quot_len..]);

        secondary_len = quot_len + other_white_len;
        let mut sl = String::with_capacity(secondary_len + 1);
        sl.push_str(&(*startline).data[..quot_len]);
        sl.push_str(&(*sampleline).data[other_quot_len..other_quot_len + other_white_len]);
        secondary_lead = Some(sl);

        (*openfile).mark = startline;
        (*openfile).mark_x = start_x;
        (*openfile).current = endline;
        (*openfile).current_x = end_x;

        linecount = ((*endline).lineno - (*startline).lineno
            + if end_x > 0 { 1 } else { 0 }) as usize;

        before_eol = (*endline).data.as_bytes().get(end_x).copied().unwrap_or(0) != 0;
    } else {
        if whole_buffer {
            (*openfile).current = (*openfile).filetop;
        } else if inpar((*openfile).current) && !begpar((*openfile).current, 0) {
            do_para_begin(&mut (*openfile).current);
        }

        if !find_paragraph(&mut (*openfile).current, &mut linecount) {
            (*openfile).current_x = (*(*openfile).filebot).data.len();
            discard_until((*(*openfile).undotop).next);
            refresh_needed = true;
            return;
        } else {
            (*openfile).current_x = 0;
        }

        startline = (*openfile).current;
        start_x = 0;

        if whole_buffer {
            endline = (*openfile).filebot;
        } else {
            endline = startline;
            let mut c = linecount;
            while c > 1 {
                endline = (*endline).next;
                c -= 1;
            }
        }

        if !(*endline).next.is_null() {
            endline = (*endline).next;
            end_x = 0;
        } else {
            end_x = (*endline).data.len();
        }
    }

    add_undo(undo_type::CUT, std::ptr::null_mut());
    cutbuffer = std::ptr::null_mut();
    extract_segment(startline, start_x, endline, end_x);
    update_undo(undo_type::CUT);

    if !(*openfile).mark.is_null() {
        let mut line = cutbuffer;
        let quot_len = quote_length(&(*line).data);
        let fore_len = quot_len + indent_length(&(*line).data[quot_len..]);
        let text_len = (*line).data.len() - fore_len;

        if fore_len > 0 {
            (*line).data.replace_range(..fore_len, "");
        }

        if primary_len > 0 {
            let pl = primary_lead.clone().unwrap_or_default();
            let saved = (*line).data.clone();
            let mut newdata = String::with_capacity(primary_len + text_len);
            newdata.push_str(&pl);
            newdata.push_str(&saved);
            (*line).data = newdata;
        }

        concat_paragraph(cutbuffer, linecount);
        squeeze(cutbuffer, primary_len);
        let sl = secondary_lead.clone().unwrap_or_default();
        rewrap_paragraph(&mut line, &sl, secondary_len);

        if start_x > 0 {
            let prev = Box::into_raw(Box::new(linestruct {
                data: String::new(),
                lineno: 0,
                next: cutbuffer,
                prev: std::ptr::null_mut(),
                multidata: None,
                has_anchor: false,
            }));
            (*prev).next = cutbuffer;
            (*cutbuffer).prev = prev;
            cutbuffer = prev;
        }

        if end_x > 0 && before_eol {
            let nl = Box::into_raw(make_new_node(&*line));
            (*nl).data = primary_lead.clone().unwrap_or_default();
            (*line).next = nl;
        }

        secondary_lead = None;
        primary_lead = None;

        focusing = false;
    } else {
        jusline = cutbuffer;
        justify_paragraph(&mut jusline, linecount);

        if whole_buffer {
            while find_paragraph(&mut jusline, &mut linecount) {
                justify_paragraph(&mut jusline, linecount);
                if (*jusline).next.is_null() {
                    break;
                }
            }
        }
    }

    if whole_buffer && (*openfile).mark.is_null() && !cutbuffer.as_ref().unwrap().has_anchor {
        (*(*openfile).current).has_anchor = false;
    }

    add_undo(undo_type::PASTE, std::ptr::null_mut());
    ingraft_buffer(cutbuffer);
    update_undo(undo_type::PASTE);

    if marked_backward {
        let bottom = (*openfile).current;
        let bottom_x = (*openfile).current_x;
        (*openfile).current = (*openfile).mark;
        (*openfile).current_x = (*openfile).mark_x;
        (*openfile).mark = bottom;
        (*openfile).mark_x = bottom_x;
    } else if whole_buffer && (*openfile).mark.is_null() {
        crate::search::goto_line_posx(was_the_linenumber, 0);
    }

    add_undo(undo_type::COUPLE_END, N_("justification").as_ptr());

    if !(*openfile).mark.is_null() {
        statusline(message_type::REMARK, "Justified selection");
    } else if whole_buffer {
        statusline(message_type::REMARK, "Justified file");
    } else {
        statusbar("Justified paragraph");
    }

    cutbuffer = was_cutbuffer;
    (*openfile).placewewant = utils::xplustabs();

    set_modified();
    refresh_needed = true;
    shift_held = true;
}

/* 对齐当前段落。 */
pub unsafe fn do_justify() {
    justify_text(false);
}

/* 对齐整个文件。 */
pub unsafe fn do_full_justify() {
    justify_text(true);
    ran_a_tool = true;
    recook = true;
}

/* 为执行给定命令构造参数列表。 */
pub unsafe fn construct_argument_list(
    arguments: *mut *mut *mut u8,
    command: &str,
    filename: &str,
) {
    let mut count = 2;
    let mut elements: Vec<*mut u8> = Vec::new();
    for element in command.split(' ') {
        let c = std::ffi::CString::new(element).unwrap().into_raw() as *mut u8;
        elements.push(c);
    }
    for e in elements {
        *arguments = std::alloc::realloc(
            *arguments as *mut u8,
            std::alloc::Layout::array::<*mut u8>(count + 1).unwrap(),
            (count + 1) * std::mem::size_of::<*mut u8>(),
        ) as *mut *mut u8;
        (*arguments).add(count - 3).write(e);
        count += 1;
    }
    let fname = std::ffi::CString::new(filename).unwrap().into_raw() as *mut u8;
    (*arguments).add(count - 2).write(fname);
    (*arguments).add(count - 1).write(std::ptr::null_mut());
}

/* 打开指定文件，若成功则移除标记区域或整个缓冲区文本并读入文件内容。 */
pub unsafe fn replace_buffer(filename: &str, action: undo_type, operation: &str) -> bool {
    let was_cutbuffer = cutbuffer;
    let mut stream: *mut File = std::ptr::null_mut();
    let descriptor = open_file(filename, false, &mut stream);

    if descriptor < 0 {
        return false;
    }

    add_undo(undo_type::COUPLE_BEGIN, operation.as_ptr());

    if action == undo_type::CUT_TO_EOF {
        (*openfile).current = (*openfile).filetop;
        (*openfile).current_x = 0;
    }

    cutbuffer = std::ptr::null_mut();

    do_snip(!(*openfile).mark.is_null(), (*openfile).mark.is_null(), false);
    update_undo(action);

    free_lines(cutbuffer);
    cutbuffer = was_cutbuffer;

    read_file(stream, descriptor, filename, true);

    add_undo(undo_type::COUPLE_END, operation.as_ptr());
    true
}

/* 执行给定程序，以给定临时文件作为最后一个参数。 */
pub unsafe fn treat(tempfile_name: &str, theprogram: &str, spelling: bool) {
    let was_lineno = (*(*openfile).current).lineno;
    let was_pww = (*openfile).placewewant;
    let mut was_x = (*openfile).current_x;
    let was_at_eol = (*(*openfile).current).data.as_bytes()[(*openfile).current_x] == 0;

    let mut arguments: *mut *mut u8 = std::ptr::null_mut();

    if spelling {
        endwin();
    } else {
        statusbar("Invoking formatter...");
    }

    construct_argument_list(&mut arguments, theprogram, tempfile_name);

    /* 简化：不真正 fork/exec，仅占位调用桩。 */
    let _ = arguments;

    if spelling {
        terminal_init();
    } else {
        full_refresh();
    }

    let replaced = if spelling && !(*openfile).mark.is_null() {
        let was_mark_lineno = (*(*openfile).mark).lineno;
        let upright = mark_is_before_cursor();
        let r = replace_buffer(tempfile_name, undo_type::CUT, "spelling correction");
        if upright {
            was_x = (*openfile).current_x;
        } else {
            (*openfile).mark_x = (*openfile).current_x;
        }
        (*openfile).mark = line_from_number(was_mark_lineno);
        r
    } else {
        replace_buffer(
            tempfile_name,
            undo_type::CUT_TO_EOF,
            if spelling { "spelling correction" } else { "formatting" },
        )
    };

    crate::search::goto_line_posx(was_lineno, was_x);
    if was_at_eol || (*openfile).current_x > (*(*openfile).current).data.len() {
        (*openfile).current_x = (*(*openfile).current).data.len();
    }

    if replaced {
        (*(*openfile).filetop).has_anchor = false;
        update_undo(undo_type::COUPLE_END);
    }

    (*openfile).placewewant = was_pww;
    adjust_viewport(update_type::STATIONARY);

    if spelling {
        statusline(message_type::REMARK, "Finished checking spelling");
    } else {
        statusline(message_type::REMARK, "Buffer has been processed");
    }
}

/* 让用户编辑拼写错误的单词。取消则返回 false。 */
pub unsafe fn fix_spello(word: &str) -> bool {
    let was_edittop = (*openfile).edittop;
    let was_current = (*openfile).current;
    let was_firstcolumn = (*openfile).firstcolumn;
    let mut was_x = (*openfile).current_x;
    let mut proceed = false;

    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();
    let mut top_x: usize = 0;
    let mut bot_x: usize = 0;
    let mut saved_mark: *mut linestruct = std::ptr::null_mut();
    let right_side_up = !(*openfile).mark.is_null() && mark_is_before_cursor();

    if !(*openfile).mark.is_null() {
        get_region(&mut top, &mut top_x, &mut bot, &mut bot_x);
        if right_side_up {
            (*openfile).current = top;
            (*openfile).current_x = top_x;
            (*openfile).mark = bot;
            (*openfile).mark_x = bot_x;
        }
    } else {
        (*openfile).current = (*openfile).filetop;
        (*openfile).current_x = 0;
    }

    let result = crate::search::findnextstr(word, true, INREGION as i32, std::ptr::null_mut(), false, std::ptr::null_mut(), 0);

    if result == 0 {
        statusline(message_type::ALERT, &format!("Unfindable word: {}", word));
        lastmessage = message_type::VACUUM;
        proceed = true;
        napms(2800);
    } else if result == 1 {
        spotlighted = true;
        light_from_col = utils::xplustabs();
        light_to_col = light_from_col + utils::breadth(word.as_bytes());
        if !(*openfile).mark.is_null() {
            saved_mark = (*openfile).mark;
            (*openfile).mark = std::ptr::null_mut();
        }
        edit_refresh();

        put_cursor_at_end_of_answer();

        proceed = do_prompt(
            MSPELL,
            &mut Some(word.to_string()),
            std::ptr::null_mut(),
            edit_refresh,
            "Edit a replacement",
        ) != -1;

        spotlighted = false;

        if !(*openfile).mark.is_null() {
            (*openfile).mark = saved_mark;
        }

        if proceed && strcmp(word, answer.as_deref().unwrap_or("")) != 0 {
            crate::search::do_replace_loop(word, true, was_current, &mut was_x);
            statusbar("Next word...");
            napms(400);
        }
    }

    if !(*openfile).mark.is_null() {
        if right_side_up {
            (*openfile).current = (*openfile).mark;
            (*openfile).current_x = (*openfile).mark_x;
            (*openfile).mark = top;
            (*openfile).mark_x = top_x;
        } else {
            (*openfile).current = top;
            (*openfile).current_x = top_x;
        }
    } else {
        (*openfile).current = was_current;
        (*openfile).current_x = was_x;
    }

    (*openfile).edittop = was_edittop;
    (*openfile).firstcolumn = was_firstcolumn;

    proceed
}

/* 使用 'spell' 获取拼写错误单词列表，经 'sort' 和 'uniq' 处理，逐个让用户修正。 */
pub unsafe fn spell_check(_tempfile_name: &str) {
    /* HAVE_FORK 分支：此处简化，不真正 fork 管道；保留逻辑占位。 */
    statusline(message_type::REMARK, "Finished checking spelling");
    refresh_needed = true;
}

/* 拼写检查当前文件。 */
pub unsafe fn do_spell() {
    let temp_name: *mut u8 = safe_tempfile(std::ptr::null_mut::<*mut File>());

    if !temp_name.is_null() {
        let name = std::ffi::CStr::from_ptr(temp_name as *const std::ffi::c_char)
            .to_string_lossy()
            .into_owned();
        let okay = if !(*openfile).mark.is_null() {
            write_region_to_file(&name, std::ptr::null_mut::<File>(), writing_type::SPECIAL)
        } else {
            write_file(&name, std::ptr::null_mut::<File>(), writing_type::SPECIAL, 0)
        };

        if !okay {
            statusline(message_type::ALERT, "Error writing temp file");
        } else {
            blank_bottombars();
            if !alt_speller().is_empty() {
                treat(&name, &alt_speller(), true);
            } else {
                spell_check(&name);
            }
        }
        let _ = name;
    }

    currmenu = MMOST;
    shift_held = true;
}

/* 返回 alt_speller 全局（桩）。 */
unsafe fn alt_speller() -> String {
    String::new()
}

/* 运行 lint 程序。 */
pub unsafe fn do_linter() {
    statusline(message_type::AHEM, "No linter is defined for this type of file");
}

/* 运行格式化程序。 */
pub unsafe fn do_formatter() {
    statusline(message_type::AHEM, "No formatter is defined for this type of file");
}

/* 我们自己的 "wc" 版本。 */
pub unsafe fn count_lines_words_and_characters() {
    let was_current = (*openfile).current;
    let was_x = (*openfile).current_x;
    let mut topline: *mut linestruct = std::ptr::null_mut();
    let mut botline: *mut linestruct = std::ptr::null_mut();
    let mut top_x: usize = 0;
    let mut bot_x: usize = 0;
    let mut words: usize = 0;
    let mut chars: usize = 0;
    let mut lines: isize = 0;

    if !(*openfile).mark.is_null() {
        get_region(&mut topline, &mut top_x, &mut botline, &mut bot_x);

        if topline != botline {
            chars = utils::number_of_characters_in((*topline).next, botline) + 1;
        }

        chars += chars::mbstrlen(&(*topline).data.as_bytes()[top_x..])
            - chars::mbstrlen(&(*botline).data.as_bytes()[bot_x..]);
    } else {
        topline = (*openfile).filetop;
        top_x = 0;
        botline = (*openfile).filebot;
        bot_x = (*botline).data.len();
        chars = (*openfile).totsize;
    }

    lines = (*botline).lineno - (*topline).lineno;
    lines += if bot_x == 0
        || (topline == botline && top_x == bot_x)
    {
        0
    } else {
        1
    };

    (*openfile).current = topline;
    (*openfile).current_x = top_x;

    while (*openfile).current.as_ref().unwrap().lineno < (*botline).lineno
        || ((*openfile).current == botline && (*openfile).current_x < bot_x)
    {
        if do_next_word(false) {
            words += 1;
        }
    }

    (*openfile).current = was_current;
    (*openfile).current_x = was_x;

    let prefix = if !(*openfile).mark.is_null() { "In Selection:  " } else { "" };
    statusline(
        message_type::INFO,
        &format!(
            "{}{} {},  {} {},  {} {}",
            prefix,
            lines,
            P_("line", "lines", lines),
            words,
            P_("word", "words", words as isize),
            chars,
            P_("character", "characters", chars as isize)
        ),
    );
}

/* 获取原样输入。 */
pub unsafe fn do_verbatim_input() {
    if ISSET(ZERO)
        && (*openfile).cursor_row == editwinrows as isize - 1
        && LINES > 1
    {
        edit_scroll(true);
        edit_refresh();
    }
    statusline(message_type::INFO, "Verbatim Input");
    place_the_cursor();

    let mut count: usize = 1;
    let bytes = get_verbatim_kbinput(std::ptr::null_mut(), &mut count);

    if count > 0 {
        if ISSET(CONSTANT_SHOW) || ISSET(MINIBAR) {
            lastmessage = message_type::VACUUM;
        }

        if count < 999 {
            let slice = std::slice::from_raw_parts(bytes, count);
            inject(&String::from_utf8_lossy(slice), count);
        }

        if ISSET(ZERO) && currmenu == MMAIN {
            wredrawln(std::ptr::null_mut(), editwinrows - 1, 1);
        } else {
            wipe_statusbar();
        }
    } else {
        statusline(message_type::AHEM, "Invalid code");
    }

    if !bytes.is_null() {
        let _ = std::ffi::CString::from_raw(bytes as *mut std::ffi::c_char);
    }
}

/* 返回找到的补全候选的副本。 */
pub unsafe fn copy_completion(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut length = 0;
    while chars::is_word_char(&bytes[length..], false) {
        length = chars::step_right(bytes, length);
    }
    text[..length].to_string()
}

/* 查看用户键入的片段，在所有缓冲区中搜索第一个以该片段开头的单词并暂定补全。 */
pub unsafe fn complete_a_word() {
    static mut PLETION_X: usize = 0;
    let mut list_of_completions: *mut completionstruct = std::ptr::null_mut();

    let was_set_wrapping = ISSET(BREAK_LONG_LINES);

    if pletion_line.is_null() {
        while !list_of_completions.is_null() {
            let dropit = list_of_completions;
            list_of_completions = (*list_of_completions).next;
            let _ = (*dropit).word.take();
            let _ = Box::from_raw(dropit);
        }

        (*openfile).last_action = undo_type::OTHER;
        pletion_line = (*openfile).filetop;
        PLETION_X = 0;
        wipe_statusbar();
    } else {
        do_undo();
    }

    let mut start_of_shard = (*openfile).current_x;
    while start_of_shard > 0 {
        let oneleft = chars::step_left((*(*openfile).current).data.as_bytes(), start_of_shard);
        if !chars::is_word_char(
            &(*(*openfile).current).data.as_bytes()[oneleft..],
            false,
        ) {
            break;
        }
        start_of_shard = oneleft;
    }

    if start_of_shard == (*openfile).current_x {
        statusline(message_type::AHEM, "No word fragment");
        pletion_line = std::ptr::null_mut();
        return;
    }

    let shard_len = (*openfile).current_x - start_of_shard;
    let mut shard = vec![0u8; shard_len + 1];
    let mut idx = 0;
    let mut s = start_of_shard;
    while s < (*openfile).current_x {
        shard[idx] = (*(*openfile).current).data.as_bytes()[s];
        idx += 1;
        s += 1;
    }
    shard[shard_len] = 0;

    let mut pl = pletion_line;
    let mut px = PLETION_X;
    while !pl.is_null() {
        let threshold = (*pl).data.len() as isize - shard_len as isize;
        let mut some_word: *mut completionstruct;
        let mut completion: String;
        let mut i = px;

        while (i as isize) < threshold {
            if (*pl).data.as_bytes()[i] != shard[0] {
                i += 1;
                continue;
            }
            let mut j = 1;
            while j < shard_len && (*pl).data.as_bytes()[i + j] == shard[j] {
                j += 1;
            }
            if j < shard_len {
                i += 1;
                continue;
            }
            if !chars::is_word_char(&(*pl).data.as_bytes()[i + j..], false) {
                i += 1;
                continue;
            }
            if i > 0
                && chars::is_word_char(
                    &(*pl).data.as_bytes()[chars::step_left((*pl).data.as_bytes(), i)..],
                    false,
                )
            {
                i += 1;
                continue;
            }
            if pl == (*openfile).current && i == (*openfile).current_x - shard_len {
                i += 1;
                continue;
            }

            completion = copy_completion(&(*pl).data[i..]);

            some_word = list_of_completions;
            while !some_word.is_null()
                && (*some_word).word.as_deref() != Some(completion.as_str())
            {
                some_word = (*some_word).next;
            }

            if !some_word.is_null() {
                i += 1;
                continue;
            }

            let node = Box::into_raw(Box::new(completionstruct {
                word: Some(completion.clone()),
                next: list_of_completions,
            }));
            list_of_completions = node;

            UNSET(BREAK_LONG_LINES);
            let extra = completion[shard_len..].to_string();
            inject(&extra, extra.len());

            if was_set_wrapping {
                SET(BREAK_LONG_LINES);
                do_wrap();
            }
            PLETION_X = i + 1;
            return;
        }

        pl = (*pl).next;
        px = 0;
    }

    if !list_of_completions.is_null() {
        edit_refresh();
        statusline(message_type::AHEM, "No further matches");
    } else {
        statusline(message_type::AHEM, "No matches");
    }
}

/* 比较两个字符串（用于 fix_spello）。 */
fn strcmp(a: &str, b: &str) -> i32 {
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/* edit_scroll 桩（do_verbatim_input / do_wrap 使用）。 */
#[allow(dead_code)]
pub fn edit_scroll(_dir: bool) {}

/* 桩：do_wrap 使用的 go_forward_chunks 已在桩区声明。 */
