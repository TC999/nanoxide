/**************************************************************************
 *   cut.rs  --  GNU nano 的 cut.c 的 Rust 翻译版本。                     *
 **************************************************************************/

use crate::definitions::*;
use crate::gettext;
use crate::chars;
use crate::utils;
use crate::global::*;
use crate::files::{statusline, statusbar, wipe_statusbar, free_lines, set_modified};
use crate::text::{add_undo, update_undo};
use crate::text::{
    copy_buffer, new_magicline, inject, splice_node, unlink_node, renumber_from,
};
use crate::r#move as mv;
use crate::r#move::{adjust_viewport, edit_redraw, do_left};

/* 尚未翻译的外部依赖，先以桩函数占位；对应模块落地后移除桩并改用 use。 */
#[allow(dead_code)]
pub unsafe fn update_line(_line: *mut linestruct, _x: usize) {
    refresh_needed = true;
}
#[allow(dead_code)]
pub unsafe fn check_the_multis(_line: *mut linestruct) {}
#[allow(dead_code)]
pub unsafe fn do_prev_word() {}
#[allow(dead_code)]
pub unsafe fn do_next_word(_after_ends: bool) -> bool { false }
#[allow(dead_code)]
pub unsafe fn do_wrap() {}
#[allow(dead_code)]
pub unsafe fn extra_chunks_in(_line: *mut linestruct) -> usize { 0 }
#[allow(dead_code)]
pub unsafe fn leftedge_for(_want: isize, _line: *mut linestruct) -> usize { 0 }
#[allow(dead_code)]
pub unsafe fn less_than_a_screenful(_lineno: isize, _leftedge: usize) -> bool { false }
#[allow(dead_code)]
pub unsafe fn precalc_multicolorinfo() {}
#[allow(dead_code)]
pub unsafe fn number_of_characters_in(_begin: *const linestruct, _end: *const linestruct) -> usize { 0 }

/* 删除光标处的字符，并为给定动作添加或更新一个撤销项。 */
pub unsafe fn expunge(action: undo_type) {
    (*openfile).placewewant = utils::xplustabs();

    /* 当处于一行中间时，删除当前字符。 */
    if !(*(*openfile).current).data.as_bytes()[(*openfile).current_x..].is_empty() {
        let charlen = chars::char_length(&(*(*openfile).current).data.as_bytes()[(*openfile).current_x..]);

        let old_amount = if ISSET(SOFTWRAP) { extra_chunks_in((*openfile).current) } else { 0 };

        /* 若动作类型改变或光标移动到另一行，则新建撤销项，否则更新现有项。 */
        if action != (*openfile).last_action
            || (*(*openfile).current).lineno != (*(*openfile).current_undo).head_lineno
        {
            add_undo(action, std::ptr::null_mut());
        } else {
            update_undo(action);
        }

        /* 将本行剩余部分“向内”移动，覆盖当前字符。 */
        let cur = &mut (*(*openfile).current).data;
        let cx = (*openfile).current_x;
        let total = cur.len();
        cur.as_bytes_mut().copy_within(cx + charlen..total, cx);
        cur.truncate(total - charlen);

        /* 软换行时，块数变化需要刷新。 */
        if ISSET(SOFTWRAP) && extra_chunks_in((*openfile).current) != old_amount {
            refresh_needed = true;
        } else if united_sidescroll && (*openfile).placewewant < (*openfile).brink + CUSHION {
            refresh_needed = true;
        }

        /* 若标记在当前行且位于光标之后，则调整标记位置。 */
        if (*openfile).mark == (*openfile).current && (*openfile).mark_x > (*openfile).current_x {
            (*openfile).mark_x -= charlen;
        }
    /* 否则，若不在缓冲区末尾，则将本行与下一行合并。 */
    } else if (*openfile).current != (*openfile).filebot {
        let joining = (*(*openfile).current).next;

        /* 若有一个魔法行且我们位于它之前：不要吃掉它。 */
        if joining == (*openfile).filebot
            && (*openfile).current_x != 0
            && !ISSET(NO_NEWLINES)
        {
            if action == undo_type::BACK {
                add_undo(undo_type::BACK, std::ptr::null_mut());
            }
            return;
        }

        add_undo(action, std::ptr::null_mut());

        /* 若标记位于将被“吃掉”的那一行，则调整标记。 */
        if (*openfile).mark == joining {
            (*openfile).mark = (*openfile).current;
            (*openfile).mark_x += (*openfile).current_x;
        }

        (*(*openfile).current).has_anchor |= (*joining).has_anchor;

        /* 将下一行内容追加到当前行之后。 */
        let newdata = format!("{}{}", (*(*openfile).current).data, (*joining).data);
        (*(*openfile).current).data = newdata;

        unlink_node(joining);

        /* 若被合并的行是缓冲区末行，更新 filebot。 */
        if (*openfile).filebot == joining {
            (*openfile).filebot = (*openfile).current;
        }

        /* 两行已合并，重新编号并刷新屏幕。 */
        renumber_from((*openfile).current);
        refresh_needed = true;
    } else {
        /* 已到文件末尾：无事可做。 */
        return;
    }

    if !refresh_needed {
        check_the_multis((*openfile).current);
    }
    if !refresh_needed {
        update_line((*openfile).current, (*openfile).current_x);
    }

    /* 调整文件大小，并为可能的重做记下它。 */
    (*openfile).totsize -= 1;
    (*(*openfile).current_undo).newsize = (*openfile).totsize;

    set_modified();
}

/* 删除光标下的字符以及其后所有零宽字符，或在开启了 --zap 且启用了标记时
 * 删除被标记的区域。 */
pub unsafe fn do_delete() {
    if !(*openfile).mark.is_null() && ISSET(LET_THEM_ZAP) {
        zap_text();
    } else {
        expunge(undo_type::DEL);
        while !(*(*openfile).current).data.as_bytes()[(*openfile).current_x..].is_empty()
            && chars::is_zerowidth(&(*(*openfile).current).data.as_bytes()[(*openfile).current_x..])
        {
            expunge(undo_type::DEL);
        }
    }
}

/* 退格删除一个字符。即先将光标左移一个字符，再删除光标下的字符。
 * 或在开启了 --zap 且启用了标记时删除被标记的区域。 */
pub unsafe fn do_backspace() {
    if !(*openfile).mark.is_null() && ISSET(LET_THEM_ZAP) {
        zap_text();
    } else if (*openfile).current_x > 0 {
        (*openfile).current_x = chars::step_left((*(*openfile).current).data.as_bytes(), (*openfile).current_x);
        expunge(undo_type::BACK);
    } else if (*openfile).current != (*openfile).filetop {
        do_left();
        expunge(undo_type::BACK);
    }
}

/* 当剪切命令实际上切不到任何内容时返回 false：处于 EOF 的空行上，
 * 或标记覆盖零个字符，或（当 test_cliff 为 true 时）将切到魔法行。 */
pub unsafe fn is_cuttable(test_cliff: bool) -> bool {
    let from = if test_cliff { (*openfile).current_x } else { 0 };

    if (*(*openfile).current).next.is_null()
        && (*(*openfile).current).data.as_bytes()[from..].is_empty()
        && (*openfile).mark.is_null()
        || (!(*openfile).mark.is_null()
            && (*openfile).mark == (*openfile).current
            && (*openfile).mark_x == (*openfile).current_x)
        || (from > 0
            && !ISSET(NO_NEWLINES)
            && (*(*openfile).current).data.as_bytes()[from..].is_empty()
            && (*(*openfile).current).next == (*openfile).filebot)
    {
        statusbar(gettext!("Nothing was cut"));
        (*openfile).mark = std::ptr::null_mut();
        return false;
    } else {
        return true;
    }
}

/* 从光标处删除文本，直到左方（forward 为 false）或右方（forward 为 true）
 * 第一个单词的起点。 */
pub unsafe fn chop_word(forward: bool) {
    /* 记住当前光标位置。 */
    let was_current = (*openfile).current;
    let was_x = (*openfile).current_x;
    /* 记住 cutbuffer 在哪里，然后让它看起来是空的。 */
    let is_cutbuffer = cutbuffer;

    cutbuffer = std::ptr::null_mut();

    /* 将光标移到一个单词的起点，向左或向右。若该单词在另一行上且光标
     * 原本不在原行的边缘，则将光标放到该边缘，以免意外合并行。 */
    if !forward {
        do_prev_word();
        if (*openfile).current != was_current {
            if was_x > 0 {
                (*openfile).current = was_current;
                (*openfile).current_x = 0;
            } else {
                (*openfile).current_x = (*(*openfile).current).data.len();
            }
        }
    } else {
        do_next_word(ISSET(AFTER_ENDS));
        if (*openfile).current != was_current && !(*was_current).data.as_bytes()[was_x..].is_empty() {
            (*openfile).current = was_current;
            (*openfile).current_x = (*was_current).data.len();
        }
    }

    /* 在该单词起点设置标记。 */
    (*openfile).mark = (*openfile).current;
    (*openfile).mark_x = (*openfile).current_x;

    /* 把光标放回原处，以便撤销时也把它放回那里。 */
    (*openfile).current = was_current;
    (*openfile).current_x = was_x;

    /* 现在删除被标记的区域，一个单词就消失了。 */
    add_undo(undo_type::CUT, std::ptr::null_mut());
    do_snip(true, false, false);
    update_undo(undo_type::CUT);

    /* 丢弃被切下的单词并恢复 cutbuffer。 */
    free_lines(cutbuffer);
    cutbuffer = is_cutbuffer;
}

/* 向左删除一个单词。 */
pub unsafe fn chop_previous_word() {
    if (*(*openfile).current).prev.is_null() && (*openfile).current_x == 0 {
        statusbar(gettext!("Nothing was cut"));
    } else {
        chop_word(BACKWARD);
    }
}

/* 向右删除一个单词。 */
pub unsafe fn chop_next_word() {
    (*openfile).mark = std::ptr::null_mut();

    if is_cuttable(true) {
        chop_word(FORWARD);
    }
}

/* 切除给定两点之间的文本，并将其加入 cutbuffer。 */
pub unsafe fn extract_segment(top: *mut linestruct, top_x: usize, bot: *mut linestruct, bot_x: usize) {
    let mut taken: *mut linestruct;
    let mut last: *mut linestruct;
    let edittop_inside = (*(*openfile).edittop).lineno >= (*top).lineno
        && (*(*openfile).edittop).lineno <= (*bot).lineno;

    let same_line = (*openfile).mark == top;
    let post_marked = !(*openfile).mark.is_null()
        && ((*(*openfile).mark).lineno > (*top).lineno
            || (same_line && (*openfile).mark_x > top_x));
    let mut inherited_anchor = false;
    let mut had_anchor = (*top).has_anchor;

    if top == bot && top_x == bot_x {
        return;
    }

    if top != bot {
        let mut line = (*top).next;
        while line != (*bot).next {
            had_anchor |= (*line).has_anchor;
            line = (*line).next;
        }
    }

    if top == bot {
        taken = Box::into_raw(make_new_node(&*top));
        (*taken).data = String::from_utf8_lossy(
            &(*top).data.as_bytes()[top_x..top_x + (bot_x - top_x)],
        ).into_owned();
        let cur = &mut (*top).data;
        let bytes = unsafe { cur.as_bytes_mut() };
        bytes.copy_within(top_x + (bot_x - top_x).., top_x);
        last = taken;
    } else if top_x == 0 && bot_x == 0 {
        taken = top;
        last = Box::into_raw(make_new_node(&*top));
        (*last).data = copy_of("");
        (*last).has_anchor = (*bot).has_anchor;
        (*last).prev = (*bot).prev;
        (*(*bot).prev).next = last;
        (*last).next = std::ptr::null_mut();

        (*bot).prev = (*top).prev;
        if !(*top).prev.is_null() {
            (*(*top).prev).next = bot;
        } else {
            (*openfile).filetop = bot;
        }

        (*openfile).current = bot;
    } else {
        taken = Box::into_raw(make_new_node(&*top));
        (*taken).data = (*top).data[top_x..].to_string();
        (*taken).next = (*top).next;
        (*(*top).next).prev = taken;

        (*top).next = (*bot).next;
        if !(*bot).next.is_null() {
            (*(*bot).next).prev = top;
        }

        let newdata = format!(
            "{}{}",
            &(*top).data[..top_x],
            &(*bot).data[bot_x..]
        );
        (*top).data = newdata;

        last = bot;
        (*last).data.truncate(bot_x);
        (*last).next = std::ptr::null_mut();

        (*openfile).current = top;
    }

    /* 从缓冲区大小中减去被切除文本的大小。 */
    (*openfile).totsize -= number_of_characters_in(taken, last);

    /* 若 cutbuffer 当前为空，则直接把所有文本移入其中；否则追加到已有内容之后。 */
    if cutbuffer.is_null() {
        cutbuffer = taken;
        cutbottom = last;
        inherited_anchor = (*taken).has_anchor;
    } else {
        let cb = cutbuffer;
        let cb_bytes = (*cb).data.as_bytes().to_vec();
        let taken_bytes = (*taken).data.as_bytes().to_vec();
        let mut merged = cb_bytes;
        merged.extend_from_slice(&taken_bytes);
        merged.push(0);
        (*cb).data = String::from_utf8_lossy(&merged[..merged.len() - 1]).into_owned();
        (*cb).has_anchor = (*taken).has_anchor && !inherited_anchor;
        inherited_anchor |= (*taken).has_anchor;
        (*cb).next = (*taken).next;
        delete_node(Box::from_raw(taken));

        if !(*cb).next.is_null() {
            (*(*cb).next).prev = cb;
            cutbottom = last;
        }
    }

    (*openfile).current_x = top_x;

    (*(*openfile).current).has_anchor = had_anchor;

    if post_marked || same_line {
        (*openfile).mark = (*openfile).current;
    }
    if post_marked {
        (*openfile).mark_x = (*openfile).current_x;
    }
    if (*openfile).filebot == bot {
        (*openfile).filebot = (*openfile).current;
    }

    renumber_from((*openfile).current);

    /* 当视口起点在被切除范围内时，调整视口。 */
    if edittop_inside {
        adjust_viewport(update_type::STATIONARY);
        refresh_needed = true;
    }

    /* 若文本不以换行符结尾而本应如此，则补上一个。 */
    if !ISSET(NO_NEWLINES) && !(*(*openfile).filebot).data.is_empty() {
        new_magicline();
    }
}

/* 将起始于 topline 的缓冲区合并进当前文件缓冲区的当前光标位置。 */
pub unsafe fn ingraft_buffer(topline: *mut linestruct) {
    let line = (*openfile).current;
    let length = (*line).data.len();
    let extralen = (*topline).data.len();
    let xpos = (*openfile).current_x;
    let tailtext = (*line).data[xpos..].to_string();
    let mark_follows = (*openfile).mark == line && !utils::mark_is_before_cursor();
    let mut botline = topline;

    while !(*botline).next.is_null() {
        botline = (*botline).next;
    }

    /* 将被嫁接文本的大小加入缓冲区大小。 */
    (*openfile).totsize += number_of_characters_in(topline, botline);

    let mut length = length;
    if topline != botline {
        length = xpos;
    }

    if extralen > 0 {
        /* 在光标当前位置插入 topline 的文本。 */
        let mut line_bytes = (*line).data.as_bytes().to_vec();
        /* 模拟 C 字符串的结尾 '\0'，确保 [xpos..length+1] 切片合法。 */
        if line_bytes.len() < length + 1 {
            line_bytes.resize(length + 1, 0);
        }
        let top_bytes = (*topline).data.as_bytes().to_vec();
        let mut newbytes = line_bytes[..xpos].to_vec();
        newbytes.extend_from_slice(&top_bytes[..extralen]);
        newbytes.extend_from_slice(&line_bytes[xpos..length + 1]);
        (*line).data = String::from_utf8_lossy(&newbytes[..newbytes.len() - 1]).into_owned();
    }

    if topline != botline {
        /* 当插入到缓冲区末尾时，更新相关指针。 */
        if (*line).next.is_null() {
            (*openfile).filebot = botline;
        }

        {
            let need = xpos + extralen + 1;
            let mut bytes = (*line).data.as_bytes().to_vec();
            if bytes.len() < need {
                bytes.resize(need, 0);
            }
            bytes[xpos + extralen] = 0;
            (*line).data = String::from_utf8_lossy(&bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]).into_owned();
        }

        /* 将被嫁接的行挂到当前行之后。 */
        (*botline).next = (*(*openfile).current).next;
        if !(*botline).next.is_null() {
            (*(*botline).next).prev = botline;
        }
        (*(*openfile).current).next = (*topline).next;
        (*(*topline).next).prev = (*openfile).current;

        /* 把光标后的文本加到 botline 末尾。 */
        let blen = (*botline).data.len();
        let extralen2 = tailtext.len();
        let bl_bytes = (*botline).data.as_bytes().to_vec();
        let tt_bytes = tailtext.as_bytes().to_vec();
        let mut merged = bl_bytes;
        merged.extend_from_slice(&tt_bytes);
        merged.push(0);
        (*botline).data = String::from_utf8_lossy(&merged[..merged.len() - 1]).into_owned();

        /* 把光标放到被嫁接文本的末尾。 */
        (*openfile).current = botline;
        (*openfile).current_x = blen;
    } else {
        (*openfile).current_x += extralen;
    }

    /* 需要时，更新标记的指针与位置。 */
    if mark_follows && topline != botline {
        (*openfile).mark = botline;
        (*openfile).mark_x += length - xpos;
    } else if mark_follows {
        (*openfile).mark_x += extralen;
    }

    delete_node(Box::from_raw(topline));

    renumber_from(line);

    /* 若文本不以换行符结尾而本应如此，则补上一个。 */
    if !ISSET(NO_NEWLINES) && !(*(*openfile).filebot).data.is_empty() {
        new_magicline();
    }
}

/* 将给定缓冲区的一份副本合并进当前文件缓冲区。 */
pub unsafe fn copy_from_buffer(somebuffer: *mut linestruct) {
    let threshold = (*(*openfile).edittop).lineno + editwinrows as isize - 1;

    let the_copy = copy_buffer(somebuffer);

    ingraft_buffer(the_copy);

    if (*(*openfile).current).lineno > threshold || ISSET(SOFTWRAP) {
        recook = true;
    } else {
        perturbed = true;
    }
}

/* 将当前缓冲区中所有被标记的文本移入 cutbuffer。 */
pub unsafe fn cut_marked_region() {
    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();
    let mut top_x: usize = 0;
    let mut bot_x: usize = 0;

    utils::get_region(&mut top, &mut top_x, &mut bot, &mut bot_x);

    extract_segment(top, top_x, bot, bot_x);

    (*openfile).placewewant = utils::xplustabs();
}

/* 将当前缓冲区的文本移入 cutbuffer。
 * 若 until_eof 为 true，则把从当前光标位置到文件末尾的所有文本移入 cutbuffer。
 * 若 append 为 true（zap 时），总是把切除内容追加到 cutbuffer。 */
pub unsafe fn do_snip(marked: bool, until_eof: bool, append: bool) {
    let line = (*openfile).current;

    keep_cutbuffer &= (*openfile).last_action != undo_type::COPY;

    /* 若剪切不是连续的，或正在剪切一个区域，则清空。 */
    if (marked || until_eof || !keep_cutbuffer) && !append {
        free_lines(cutbuffer);
        cutbuffer = std::ptr::null_mut();
    }

    /* 现在把相关文本移入 cutbuffer。 */
    if until_eof {
        extract_segment(
            (*openfile).current,
            (*openfile).current_x,
            (*openfile).filebot,
            (*(*openfile).filebot).data.len(),
        );
    } else if !(*openfile).mark.is_null() {
        cut_marked_region();
        (*openfile).mark = std::ptr::null_mut();
    } else if ISSET(CUT_FROM_CURSOR) {
        /* 当不在行末时，把本行剩余部分移入 cutbuffer。否则，若不在缓冲区
         * 末尾，则只移入“行分隔符”。 */
        if !(*line).data.as_bytes()[(*openfile).current_x..].is_empty() {
            extract_segment(line, (*openfile).current_x, line, (*line).data.len());
        } else if (*openfile).current != (*openfile).filebot {
            extract_segment(line, (*openfile).current_x, (*line).next, 0);
            (*openfile).placewewant = utils::xplustabs();
        }
    } else {
        /* 当不在缓冲区末尾时，移入一整行；否则移入到行末的所有文本。 */
        if (*openfile).current != (*openfile).filebot {
            extract_segment(line, 0, (*line).next, 0);
        } else {
            extract_segment(line, 0, line, (*line).data.len());
        }

        (*openfile).placewewant = 0;
    }

    /* 行操作之后，后续的行操作应追加到 cutbuffer。 */
    keep_cutbuffer = !marked && !until_eof;

    set_modified();
    refresh_needed = true;
    perturbed = true;
}

/* 将当前缓冲区的文本移入 cutbuffer。 */
pub unsafe fn cut_text() {
    if !is_cuttable(ISSET(CUT_FROM_CURSOR) && (*openfile).mark.is_null()) {
        return;
    }

    /* 仅当当前项不是 CUT 或当前剪切与上次不连续时才新建撤销项。 */
    if (*openfile).last_action != undo_type::CUT || !keep_cutbuffer {
        keep_cutbuffer = false;
        add_undo(undo_type::CUT, std::ptr::null_mut());
    }

    do_snip(!(*openfile).mark.is_null(), false, false);

    update_undo(undo_type::CUT);

    wipe_statusbar();
}

/* 从当前光标位置剪切到文件末尾。 */
pub unsafe fn cut_till_eof() {
    ran_a_tool = true;

    if (*(*openfile).current).data.as_bytes()[(*openfile).current_x..].is_empty()
        && ((*(*openfile).current).next.is_null()
            || (!ISSET(NO_NEWLINES)
                && (*openfile).current_x > 0
                && (*(*openfile).current).next == (*openfile).filebot))
    {
        statusbar(gettext!("Nothing was cut"));
        return;
    }

    add_undo(undo_type::CUT_TO_EOF, std::ptr::null_mut());
    do_snip(false, true, false);
    update_undo(undo_type::CUT_TO_EOF);
    wipe_statusbar();
}

/* 擦除文本（当前行或标记区域），使其消失无踪。 */
pub unsafe fn zap_text() {
    /* 记住当前 cutbuffer，以便 zap 之后恢复。 */
    let was_cutbuffer = cutbuffer;

    if !is_cuttable(ISSET(CUT_FROM_CURSOR) && (*openfile).mark.is_null()) {
        return;
    }

    /* 仅当当前项不是 ZAP 或当前 zap 与上次不连续时才新建撤销项。 */
    if (*openfile).last_action != undo_type::ZAP || !keep_cutbuffer {
        add_undo(undo_type::ZAP, std::ptr::null_mut());
    }

    /* 使用 ZAP 撤销项中的 cutbuffer，以便此剪切可被撤销。 */
    cutbuffer = (*(*openfile).current_undo).cutbuffer;

    do_snip(!(*openfile).mark.is_null(), false, true);

    update_undo(undo_type::ZAP);
    wipe_statusbar();

    cutbuffer = was_cutbuffer;
}

/* 制作被标记区域的一份副本，放入 cutbuffer。 */
pub unsafe fn copy_marked_region() {
    let mut topline: *mut linestruct = std::ptr::null_mut();
    let mut botline: *mut linestruct = std::ptr::null_mut();
    let mut afterline: *mut linestruct = std::ptr::null_mut();
    let saved_byte: u8;
    let mut top_x: usize = 0;
    let mut bot_x: usize = 0;

    utils::get_region(&mut topline, &mut top_x, &mut botline, &mut bot_x);

    (*openfile).last_action = undo_type::OTHER;
    keep_cutbuffer = false;
    (*openfile).mark = std::ptr::null_mut();
    refresh_needed = true;

    if topline == botline && top_x == bot_x {
        statusbar(gettext!("Copied nothing"));
        return;
    }

    /* 让被标记的区域看起来像一个独立的缓冲区。 */
    afterline = (*botline).next;
    (*botline).next = std::ptr::null_mut();
    saved_byte = (*botline).data.as_bytes()[bot_x];
    {
        let bd = &mut (*botline).data;
        let bytes = unsafe { bd.as_bytes_mut() };
        bytes[bot_x] = 0;
    }
    let was_datastart = (*topline).data.clone();
    (*topline).data = (*topline).data[top_x..].to_string();

    cutbuffer = copy_buffer(topline);

    /* 恢复缓冲区的正确状态。 */
    (*topline).data = was_datastart;
    {
        let bd = &mut (*botline).data;
        let bytes = unsafe { bd.as_bytes_mut() };
        bytes[bot_x] = saved_byte;
    }
    (*botline).next = afterline;
}

/* 将当前缓冲区的文本复制进 cutbuffer。文本可以是被标记区域、整行、
 * 从光标到行末的文本、仅行分隔符，或什么都没有，取决于模式与光标位置。 */
pub unsafe fn copy_text() {
    let at_eol = (*(*openfile).current).data.as_bytes()[(*openfile).current_x..].is_empty();
    let mut sans_newline = ISSET(NO_NEWLINES) && (*(*openfile).current).next.is_null();
    let from_x = if ISSET(CUT_FROM_CURSOR) { (*openfile).current_x } else { 0 };
    let was_current = (*openfile).current;
    let addition: *mut linestruct;

    if !(*openfile).mark.is_null() || (*openfile).last_action != undo_type::COPY {
        keep_cutbuffer = false;
    }

    if !keep_cutbuffer {
        free_lines(cutbuffer);
        cutbuffer = std::ptr::null_mut();
    }

    wipe_statusbar();

    if !(*openfile).mark.is_null() {
        copy_marked_region();
        return;
    }

    /* 当位于缓冲区最末尾时，无事可做。 */
    if (*(*openfile).current).next.is_null()
        && at_eol
        && (ISSET(CUT_FROM_CURSOR) || (*openfile).current_x == 0 || !cutbuffer.is_null())
    {
        statusbar(gettext!("Copied nothing"));
        return;
    }

    addition = Box::into_raw(make_new_node(&*(*openfile).current));
    (*addition).data = (*(*openfile).current).data[from_x..].to_string();

    if ISSET(CUT_FROM_CURSOR) {
        sans_newline = !at_eol;
    }

    /* 根据模式、光标位置以及 cutbuffer 当前是否为空，创建或追加 cutbuffer。 */
    if cutbuffer.is_null() && sans_newline {
        cutbuffer = addition;
        cutbottom = addition;
    } else if cutbuffer.is_null() {
        cutbuffer = addition;
        cutbottom = Box::into_raw(make_new_node(&*cutbuffer));
        (*cutbottom).data = copy_of("");
        (*cutbuffer).next = cutbottom;
    } else if sans_newline {
        (*addition).prev = (*cutbottom).prev;
        (*(*addition).prev).next = addition;
        delete_node(Box::from_raw(cutbottom));
        cutbottom = addition;
    } else if ISSET(CUT_FROM_CURSOR) {
        (*addition).prev = cutbottom;
        (*cutbottom).next = addition;
        cutbottom = addition;
    } else {
        (*addition).prev = (*cutbottom).prev;
        (*(*addition).prev).next = addition;
        (*addition).next = cutbottom;
        (*cutbottom).prev = addition;
    }

    /* 需要时且可能时，将光标移到下一行。 */
    if (!ISSET(CUT_FROM_CURSOR) || at_eol) && !(*(*openfile).current).next.is_null() {
        (*openfile).current = (*(*openfile).current).next;
        (*openfile).current_x = 0;
    } else {
        (*openfile).current_x = (*(*openfile).current).data.len();
    }

    edit_redraw(was_current, update_type::FLOWING);

    (*openfile).last_action = undo_type::COPY;
    keep_cutbuffer = true;
}

/* 将 cutbuffer 中的文本复制进当前缓冲区。 */
pub unsafe fn paste_text() {
    /* 记住粘贴开始的位置。 */
    let was_current = (*openfile).current;
    let had_anchor = (*was_current).has_anchor;
    let was_lineno = (*(*openfile).current).lineno;
    let mut was_leftedge: usize = 0;

    if cutbuffer.is_null() {
        statusline(message_type::AHEM, gettext!("Cutbuffer is empty"));
        return;
    }

    add_undo(undo_type::PASTE, std::ptr::null_mut());

    if ISSET(SOFTWRAP) {
        was_leftedge = leftedge_for(utils::xplustabs() as isize, (*openfile).current);
    }

    /* 在当前光标位置把 cutbuffer 中文本的一份副本加入当前缓冲区。 */
    copy_from_buffer(cutbuffer);

    /* 清除被粘贴文本中的锚点，以免它们扩散。 */
    let mut line = was_current;
    while line != (*(*openfile).current).next {
        (*line).has_anchor = false;
        line = (*line).next;
    }

    (*was_current).has_anchor = had_anchor;

    update_undo(undo_type::PASTE);

    /* 若仍在同行且开启了硬换行，则限制宽度。 */
    if (*openfile).current == was_current && ISSET(BREAK_LONG_LINES) {
        do_wrap();
    }

    /* 若粘贴的内容不足一屏，则不居中光标。 */
    if less_than_a_screenful(was_lineno, was_leftedge) {
        focusing = false;
    } else {
        precalc_multicolorinfo();
    }

    /* 将期望的 x 位置设为被粘贴文本结束处。 */
    (*openfile).placewewant = utils::xplustabs();

    set_modified();
    wipe_statusbar();
    refresh_needed = true;
}
