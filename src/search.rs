/**************************************************************************
 *   search.rs  --  GNU nano 的 search.c 的 Rust 翻译版本。                *
 **************************************************************************/

use crate::definitions::*;
use crate::gettext;
use crate::chars;
use crate::utils;
use crate::global;
use crate::history;
use crate::files;
use crate::text;
use crate::winio;
use crate::r#move as mv;

use crate::global::{refresh_needed, recook, perturbed, last_search, search_history, answer, inhelp, currmenu, matchbrackets, searchbot, midwin, spotlighted, light_from_col, light_to_col, didfind, replace_history, editwinrows};
use crate::files::{statusline, statusbar, wipe_statusbar, COLS, LINES, leftedge_for};
use crate::definitions::editwincols;
use crate::text::P_;

static mut came_full_circle: bool = false;
/* 搜索时是否已经绕回到起始行？ */
static mut have_compiled_regexp: bool = false;
/* 是否已经为正则搜索编译过正则表达式？ */

/* 编译给定的正则表达式并存入 search_regexp。
 * 表达式合法返回 true，否则返回 false。 */
pub unsafe fn regexp_init(regexp: &str) -> bool {
    let mut builder = regex::RegexBuilder::new(regexp);
    if ISSET(CASE_SENSITIVE) {
        builder.case_insensitive(false);
    } else {
        builder.case_insensitive(true);
    }
    match builder.build() {
        Ok(rx) => {
            regexp_nsub = rx.captures_len();
            search_regexp = Some(Box::new(rx));
            have_compiled_regexp = true;
            true
        }
        Err(e) => {
            statusline(message_type::AHEM, &format!("Bad regex \"{}\": {}", regexp, e));
            false
        }
    }
}

/* 释放已编译的正则表达式（若有）；当标记开启时安排整屏刷新，
 * 以防光标已经移动。 */
pub unsafe fn tidy_up_after_search() {
    if have_compiled_regexp {
        search_regexp = None;
        have_compiled_regexp = false;
    }
    if !(*openfile).mark.is_null() {
        refresh_needed = true;
    }
    recook |= perturbed;
}

/* 准备提示并询问用户要搜索什么。只要用户按下切换键就继续循环，
 * 仅在按下 <Enter> 或执行了非切换快捷键时才采取行动并退出。 */
pub unsafe fn search_init(replacing: bool, retain_answer: bool) {
    let thedefault: String;

    /* 如果之前搜索过内容，把它放进提示里。 */
    if let Some(ls) = &last_search {
        if !ls.is_empty() {
            let disp = files::display_string(ls.as_bytes(), 0, (COLS / 3) as usize, false, false);
            let mut s = String::from(" [");
            s.push_str(&disp);
            if utils::breadth(ls.as_bytes()) > (COLS / 3) as usize {
                s.push_str("...");
            }
            s.push(']');
            thedefault = s;
        } else {
            thedefault = String::new();
        }
    } else {
        thedefault = String::new();
    }

    let mut retain = retain_answer;
    let mut repl = replacing;

    loop {
        let response = files::do_prompt(
            if inhelp { MFINDINHELP } else if repl { MREPLACE } else { MWHEREIS },
            &mut answer,
            search_history,
            winio::edit_refresh,
            &format!(
                "{}{}{}{}{}",
                gettext!("Search"),
                if ISSET(CASE_SENSITIVE) { " [Case sensitive]" } else { "" },
                if ISSET(USE_REGEXP) { " [Reg.exp.]" } else { "" },
                if ISSET(BACKWARDS_SEARCH) { " [Backwards]" } else { "" },
                if repl {
                    if !(*openfile).mark.is_null() {
                        " (to replace) in selection"
                    } else {
                        " (to replace)"
                    }
                } else {
                    ""
                }
            ),
        );

        /* 搜索被取消，或得到空答案且本次会话尚未搜索过内容，退出。 */
        if response == -1
            || (response == -2 && last_search.as_deref().map_or(true, |s| s.is_empty()))
        {
            statusbar(gettext!("Cancelled"));
            break;
        }

        /* 按下了 Enter，准备做替换或搜索。 */
        if response == 0 || response == -2 {
            /* 如果确实输入了答案，记住它。 */
            if let Some(a) = &answer {
                if ! a.is_empty() {
                    last_search = Some(a.clone());
                    history::update_history(&mut search_history, a, PRUNE_DUPLICATE);
                }
            }

            if ISSET(USE_REGEXP) && !regexp_init(last_search.as_deref().unwrap_or("")) {
                break;
            }

            if repl {
                ask_for_and_do_replacements();
            } else {
                go_looking();
            }

            break;
        }

        retain = true;

        let function = global::func_from_key(response);

        /* 走到这里，说明按下了五个切换键之一，或执行了某快捷键。 */
        if function == Some(global::case_sens_void as fn()) {
            TOGGLE(CASE_SENSITIVE);
        } else if function == Some(global::backwards_void as fn()) {
            TOGGLE(BACKWARDS_SEARCH);
        } else if function == Some(global::regexp_void as fn()) {
            TOGGLE(USE_REGEXP);
        } else if function == Some(global::flip_replace as fn()) {
            if ISSET(VIEW_MODE) {
                print_view_warning();
                napms(600);
            } else {
                repl = !repl;
            }
        } else if function == Some(global::flip_goto as fn()) {
            ask_for_line_and_column();
            break;
        } else {
            break;
        }
    }

    if !inhelp {
        tidy_up_after_search();
    }
}

/* 从 (current, current_x) 处开始查找 needle。begin 是首次开始搜索的行，
 * 列坐标为 begin_x。找到返回 1，未找到返回 0，取消返回 -2。
 * 当 match_len 非空时，把匹配长度写入其中。 */
pub unsafe fn findnextstr(
    needle: &str,
    whole_word_only: bool,
    modus: i32,
    match_len: *mut usize,
    mut skipone: bool,
    begin: *const linestruct,
    begin_x: usize,
) -> i32 {
    let mut found_len = needle.len();
    /* 匹配长度——正则搜索时会重新计算。 */
    let mut feedback = 0;
    /* 大于零时显示并清除 "Searching..." 消息。 */
    let mut line = (*openfile).current;
    /* 当前要搜索的行。 */
    let mut from: usize = 0;
    /* 当前行中开始搜索的位置。 */
    let mut found: Option<usize> = None;
    /* 匹配位置（若有）。 */
    let mut found_x: usize = 0;
    /* 找到的匹配项的 x 坐标。 */

    /* 设置非阻塞输入以便窥探取消键。 */
    nodelay(midwin, true);

    if begin.is_null() {
        came_full_circle = false;
    }

    loop {
        /* 开始新搜索时跳过第一个字符，然后（两种情况下）在当前行搜索 needle。 */
        if skipone {
            skipone = false;
            let ld = (*line).data.clone();
            if ISSET(BACKWARDS_SEARCH) && from != 0 {
                from = chars::step_left(ld.as_bytes(), from);
                found = utils::strstrwrapper(ld.as_bytes(), needle.as_bytes(), from);
            } else if !ISSET(BACKWARDS_SEARCH) && from < ld.len() {
                from += chars::char_length(&ld.as_bytes()[from..]);
                found = utils::strstrwrapper(ld.as_bytes(), needle.as_bytes(), from);
            }
        } else {
            let ld = (*line).data.clone();
            found = utils::strstrwrapper(ld.as_bytes(), needle.as_bytes(), from);
        }

        if found.is_some() {
            /* 正则搜索时计算匹配长度。 */
            if ISSET(USE_REGEXP) {
                found_len = regmatches[0].1.unwrap_or(0) - regmatches[0].0.unwrap_or(0);
            }
            /* 拼写检查时，匹配必须是独立单词；否则继续在行内查找。 */
            if whole_word_only
                && !utils::is_separate_word(found.unwrap(), found_len, (*line).data.as_bytes())
            {
                from = found.unwrap() + chars::char_length(&(*line).data.as_bytes()[found.unwrap()..]);
                continue;
            }
            /* 不在魔法行上时，匹配有效。 */
            if (*line).next.is_null() || !(*line).data.is_empty() {
                break;
            }
        }

        /* 回到起点，说明没有 needle。 */
        if came_full_circle {
            nodelay(midwin, false);
            return 0;
        }

        /* 移动到文件中的上一行或下一行。 */
        line = if ISSET(BACKWARDS_SEARCH) {
            (*line).prev
        } else {
            (*line).next
        };

        /* 到达缓冲区开头或结尾时绕回；但拼写检查或在区域内替换时停止。 */
        if line.is_null() {
            if whole_word_only || modus == INREGION as i32 {
                nodelay(midwin, false);
                return 0;
            }

            line = if ISSET(BACKWARDS_SEARCH) {
                (*openfile).filebot
            } else {
                (*openfile).filetop
            };

            if modus == JUSTFIND as i32 {
                statusline(message_type::REMARK, gettext!("Search Wrapped"));
                /* 把 "Searching..." 消息至少延迟两秒。 */
                feedback = -2;
            }
        }

        /* 若已到达原始起始行，做记录。 */
        if line == begin as *mut linestruct {
            came_full_circle = true;
        }

        /* 把起始 x 设为行首或行尾。 */
        from = 0;
        if ISSET(BACKWARDS_SEARCH) {
            from = (*line).data.len();
        }
    }

    found_x = found.unwrap();

    nodelay(midwin, false);

    /* 确保找到的匹配不在起始 x 之后。 */
    if came_full_circle
        && ((!ISSET(BACKWARDS_SEARCH)
            && (found_x > begin_x || (modus == REPLACING as i32 && found_x == begin_x)))
            || (ISSET(BACKWARDS_SEARCH) && found_x < begin_x))
    {
        return 0;
    }

    /* 把当前位置指向找到的内容。 */
    (*openfile).current = line;
    (*openfile).current_x = found_x;

    /* 需要时回传匹配长度。 */
    if !match_len.is_null() {
        let fl = if ISSET(USE_REGEXP) {
            regmatches[0].1.unwrap_or(0) - regmatches[0].0.unwrap_or(0)
        } else {
            found_len
        };
        *match_len = fl;
    }

    if modus == JUSTFIND as i32 && ((*openfile).mark.is_null() || (*openfile).softmark) {
        spotlighted = true;
        light_from_col = utils::xplustabs();
        light_to_col = utils::wideness((*line).data.as_bytes(), found_x + found_len);

        /* 平移时，若匹配能容纳在未平移视口内则取消平移，否则确保匹配末尾也可见。 */
        if united_sidescroll && light_to_col < editwincols - CUSHION {
            (*openfile).brink = 0;
        } else if united_sidescroll {
            (*openfile).brink = utils::get_page_start(light_to_col);
        }

        refresh_needed = true;
    }

    if feedback > 0 {
        wipe_statusbar();
    }

    1
}

/* 询问字符串并向前搜索。 */
pub unsafe fn do_search_forward() {
    UNSET(BACKWARDS_SEARCH);
    search_init(false, false);
}

/* 询问字符串并向后搜索。 */
pub unsafe fn do_search_backward() {
    SET(BACKWARDS_SEARCH);
    search_init(false, false);
}

/* 不提示，直接搜索上次的字符串。 */
pub unsafe fn do_research() {
    /* 若本次运行尚未搜索过，但有搜索历史，取最近的一项。 */
    if last_search.as_deref().map_or(true, |s| s.is_empty())
        && !search_history.is_null()
        && !(*search_history).prev.is_null()
    {
        last_search = Some((*(*search_history).prev).data.clone());
    }

    if last_search.as_deref().map_or(true, |s| s.is_empty()) {
        statusline(message_type::AHEM, gettext!("No current search pattern"));
        return;
    }

    if ISSET(USE_REGEXP) && !regexp_init(last_search.as_deref().unwrap_or("")) {
        return;
    }

    /* 使用搜索菜单的按键绑定，以便取消。 */
    currmenu = MWHEREIS;

    if LINES > 1 {
        wipe_statusbar();
    }

    go_looking();

    if !inhelp {
        tidy_up_after_search();
    }
}

/* 向后搜索下一个匹配。 */
pub unsafe fn do_findprevious() {
    SET(BACKWARDS_SEARCH);
    do_research();
}

/* 向前搜索下一个匹配。 */
pub unsafe fn do_findnext() {
    UNSET(BACKWARDS_SEARCH);
    do_research();
}

/* 在状态栏报告未找到给定字符串。 */
pub unsafe fn not_found_msg(str: &str) {
    let disp = files::display_string(str.as_bytes(), 0, (COLS / 2) as usize + 1, false, false);
    let numchars = utils::actual_x(disp.as_bytes(), utils::wideness(disp.as_bytes(), (COLS / 2) as usize));
    let truncated = if disp.as_bytes().get(numchars).map_or(true, |c| *c == 0) {
        ""
    } else {
        "..."
    };
    statusline(message_type::AHEM, &format!("\"{:.width$}{}\" not found", disp, truncated, width = numchars));
}

/* 搜索全局字符串 'last_search'。当字符串只出现一次时告知用户。 */
pub unsafe fn go_looking() {
    let was_current = (*openfile).current;
    let was_x = (*openfile).current_x;

    came_full_circle = false;

    didfind = findnextstr(
        last_search.as_deref().unwrap_or(""),
        false,
        JUSTFIND as i32,
        std::ptr::null_mut(),
        true,
        (*openfile).current,
        (*openfile).current_x,
    );

    /* 若找到，且正好回到开始搜索的同一位置，说明这是唯一出现。 */
    if didfind == 1 && (*openfile).current == was_current && (*openfile).current_x == was_x {
        statusline(message_type::REMARK, gettext!("This is the only occurrence"));
    } else if didfind == 0 {
        not_found_msg(last_search.as_deref().unwrap_or(""));
    }

    mv::edit_redraw(was_current, update_type::CENTERING);
}

/* 计算针对找到的正则表达式的替换文本大小，考虑对子表达式（\1 到 \9）的引用。
 * 当 string 参数非空时，把替换文本复制进 `string`。 */
pub unsafe fn replace_regexp(mut string: *mut u8) -> usize {
    let mut replacement_size = 0;
    let given = answer.as_deref().unwrap_or("").as_bytes();
    let mut i = 0;

    while i < given.len() {
        let c = given[i];
        let digit = (c as i32) - ('0' as i32);

        /* 若有有效的反向引用，使用相应子表达式；否则使用字面的给定答案。 */
        if c == b'\\' && i + 1 < given.len() && 0 < digit && digit < 10 && (digit as usize) <= regexp_nsub {
            let extent = regmatches[digit as usize].1.unwrap_or(0) - regmatches[digit as usize].0.unwrap_or(0);

            if !string.is_null() {
                let so = regmatches[digit as usize].0.unwrap_or(0);
                let src = (*openfile).current;
                let data = (*src).data.as_bytes();
                std::ptr::copy_nonoverlapping(data.as_ptr().add(so), string, extent);
                string = string.add(extent);
            }
            replacement_size += extent;
            i += 2;
        } else {
            if !string.is_null() {
                *string = c;
                string = string.add(1);
            }
            replacement_size += 1;
            i += 1;
        }
    }

    if !string.is_null() {
        *string = 0;
    }

    replacement_size
}

/* 返回当前行被替换一处 needle 后的副本。 */
pub unsafe fn replace_line(needle: &str) -> String {
    let cur = (*openfile).current;
    let cur_data = (*cur).data.clone();
    let mut new_size = cur_data.len() + 1;
    let match_len: usize;

    /* 先为正则模式调整新行大小。 */
    if ISSET(USE_REGEXP) {
        match_len = regmatches[0].1.unwrap_or(0) - regmatches[0].0.unwrap_or(0);
        new_size += replace_regexp(std::ptr::null_mut()) - match_len;
    } else {
        match_len = needle.len();
        new_size += answer.as_deref().map_or(0, |a| a.len()) - match_len;
    }

    let mut copy: Vec<u8> = vec![0u8; new_size];

    /* 复制原行头部。 */
    let head = &cur_data.as_bytes()[..(*openfile).current_x];
    copy[..head.len()].copy_from_slice(head);

    /* 添加替换文本。 */
    if ISSET(USE_REGEXP) {
        replace_regexp(copy.as_mut_ptr().add((*openfile).current_x));
    } else {
        let a = answer.as_deref().unwrap_or("");
        let dst = &mut copy[(*openfile).current_x..];
        dst[..a.len()].copy_from_slice(a.as_bytes());
    }

    /* 复制原行尾部。 */
    let tail_src = &cur_data.as_bytes()[(*openfile).current_x + match_len..];
    let tail_pos = (*openfile).current_x + answer.as_deref().map_or(0, |a| a.len());
    copy[tail_pos..tail_pos + tail_src.len()].copy_from_slice(tail_src);

    /* 去掉多余尾部零字节。 */
    let end = tail_pos + tail_src.len();
    String::from_utf8_lossy(&copy[..end]).into_owned()
}

/* 逐处遍历搜索字符串的出现并替换前询问用户。寻找 needle 并以 answer 替换。
 * real_current 与 real_current_x 用于允许在光标前的单词被更短单词替换时更新光标位置。
 * 若找不到 needle 返回 -1，若搜索被中止返回 -2，否则返回执行的替换次数。 */
pub unsafe fn do_replace_loop(
    needle: &str,
    whole_word_only: bool,
    real_current: *const linestruct,
    real_current_x: *mut usize,
) -> isize {
    let mut skipone = ISSET(BACKWARDS_SEARCH);
    let mut replaceall = false;
    let mut modus = REPLACING as i32;
    let mut numreplaced: isize = -1;
    let mut match_len: usize = 0;

    let was_mark = (*openfile).mark;
    let mut top: *mut linestruct = std::ptr::null_mut();
    let mut bot: *mut linestruct = std::ptr::null_mut();
    let mut top_x: usize = 0;
    let mut bot_x: usize = 0;
    let right_side_up = !(*openfile).mark.is_null() && crate::text::mark_is_before_cursor();

    /* 若标记开启，框定区域并关闭标记。 */
    if !(*openfile).mark.is_null() {
        utils::get_region(&mut top, &mut top_x, &mut bot, &mut bot_x);
        (*openfile).mark = std::ptr::null_mut();
        modus = INREGION as i32;

        /* 从标记区域的顶部或底部开始。 */
        if !ISSET(BACKWARDS_SEARCH) {
            (*openfile).current = top;
            (*openfile).current_x = top_x;
        } else {
            (*openfile).current = bot;
            (*openfile).current_x = bot_x;
        }
    }

    came_full_circle = false;

    loop {
        let mut choice = 0;
        let result = findnextstr(needle, whole_word_only, modus, &mut match_len, skipone, real_current, *real_current_x);

        /* 若未找到更多，或用户中止，停止循环。 */
        if result < 1 {
            if result < 0 {
                numreplaced = -2;
                /* 是取消而非未找到。 */
            }
            break;
        }

        /* 标记区域外的出现意味着完成。 */
        if !was_mark.is_null()
            && ((*(*openfile).current).lineno > (*bot).lineno
                || (*(*openfile).current).lineno < (*top).lineno
                || ((*openfile).current == bot && (*openfile).current_x + match_len > bot_x)
                || ((*openfile).current == top && (*openfile).current_x < top_x))
        {
            break;
        }

        /* 标记已找到搜索字符串。 */
        if numreplaced == -1 {
            numreplaced = 0;
        }

        if !replaceall {
            spotlighted = true;
            light_from_col = utils::xplustabs();
            light_to_col = utils::wideness((*(*openfile).current).data.as_bytes(), (*openfile).current_x + match_len);

            if united_sidescroll && light_to_col < editwincols - CUSHION {
                (*openfile).brink = 0;
            } else if united_sidescroll {
                (*openfile).brink = utils::get_page_start(light_to_col);
            }
            /* 刷新编辑窗口，必要时滚动。 */
            winio::edit_refresh();

            choice = files::ask_user(YESORALLORNO, gettext!("Replace this instance?"));

            spotlighted = false;

            if choice == CANCEL {
                break;
            }

            replaceall = (choice == ALL);

            /* 当选择“否”或向后移动时，搜索例程应继续前先再移动一个字符。 */
            skipone = (choice == 0 || ISSET(BACKWARDS_SEARCH));
        }

        if choice == YES || replaceall {
            let length_change: isize;
            let altered: String;

            altered = replace_line(needle);

            length_change = altered.len() as isize - (*(*openfile).current).data.len() as isize;

            /* 若标记曾开启且位于光标之后，则针对文本长度变化调整其 x 位置。 */
            if !was_mark.is_null() && !right_side_up {
                if (*openfile).current == was_mark && (*openfile).mark_x > (*openfile).current_x {
                    if (*openfile).mark_x < (*openfile).current_x + match_len {
                        (*openfile).mark_x = (*openfile).current_x;
                    } else {
                        (*openfile).mark_x = ((*openfile).mark_x as isize + length_change) as usize;
                    }
                    bot_x = (*openfile).mark_x;
                }
            }

            /* 若标记未开启或位于光标之前，则针对文本长度变化调整光标的 x 位置。 */
            if was_mark.is_null() || right_side_up {
                if (*openfile).current == real_current as *mut linestruct && (*openfile).current_x < *real_current_x {
                    if *real_current_x < (*openfile).current_x + match_len {
                        *real_current_x = (*openfile).current_x + match_len;
                    }
                    *real_current_x = ((*real_current_x as isize) + length_change) as usize;
                    bot_x = *real_current_x;
                }
            }

            /* 不要再找到同一个零长度或行首匹配。 */
            if match_len == 0 || (needle.as_bytes().first().copied() == Some(b'^') && ISSET(USE_REGEXP)) {
                skipone = true;
            }

            /* 向前移动时，把光标放在替换文本之后，以便继续搜索。 */
            if !ISSET(BACKWARDS_SEARCH) {
                (*openfile).current_x += match_len + length_change as usize;
            }

            /* 更新文件大小，并放入改动后的行。 */
            (*openfile).totsize = (*openfile).totsize
                + chars::mbstrlen(altered.as_bytes())
                - chars::mbstrlen((*(*openfile).current).data.as_bytes());
            (*(*openfile).current).data = altered;

            crate::text::check_the_multis((*openfile).current);
            refresh_needed = false;

            files::set_modified();
            chars::as_an_at = true;
            numreplaced += 1;
        }
    }

    if numreplaced == -1 {
        not_found_msg(needle);
    }

    (*openfile).mark = was_mark;

    numreplaced
}

/* 替换字符串。 */
pub unsafe fn do_replace() {
    if ISSET(VIEW_MODE) {
        print_view_warning();
    } else {
        UNSET(BACKWARDS_SEARCH);
        search_init(true, false);
    }
}

/* 询问用户用什么替换搜索字符串，并执行替换。 */
pub unsafe fn ask_for_and_do_replacements() {
    let was_edittop = (*openfile).edittop;
    let was_firstcolumn = (*openfile).firstcolumn;
    let beginline = (*openfile).current;
    let begin_x = (*openfile).current_x;
    let replacee = last_search.clone().unwrap_or_default();
    let mut numreplaced: isize;

    let response = files::do_prompt(
        MREPLACEWITH,
        &mut None,
        replace_history,
        winio::edit_refresh,
        gettext!("Replace with"),
    );

    /* 设置要搜索的字符串，因为它可能在提示时发生了变化。 */
    last_search = Some(replacee);

    /* 当不是 "" 时，把替换字符串加入替换历史列表。 */
    if response == 0 {
        if let Some(a) = &answer {
            history::update_history(&mut replace_history, a, PRUNE_DUPLICATE);
        }
    }

    /* 取消，或运行了某函数，则结束。 */
    if response == -1 {
        statusbar(gettext!("Cancelled"));
        return;
    } else if response > 0 {
        return;
    }

    let mut bx = begin_x;
    numreplaced = do_replace_loop(last_search.as_deref().unwrap_or(""), false, beginline, &mut bx);

    /* 恢复原来的位置。 */
    (*openfile).edittop = was_edittop;
    (*openfile).firstcolumn = was_firstcolumn;
    (*openfile).current = beginline;
    (*openfile).current_x = begin_x;

    refresh_needed = true;

    if numreplaced >= 0 {
        let msg = P_(
            "Replaced %zd occurrence",
            "Replaced %zd occurrences",
            numreplaced,
        );
        statusline(message_type::REMARK, &msg.replace("%zd", &numreplaced.to_string()));
    }
}

/* 跳到指定的行和 x 位置。 */
pub unsafe fn goto_line_posx(linenumber: isize, pos_x: usize) {
    if linenumber > (*(*openfile).edittop).lineno + editwinrows as isize
        || (ISSET(SOFTWRAP) && linenumber > (*(*openfile).current).lineno)
    {
        recook |= perturbed;
    }

    if linenumber < (*(*openfile).filebot).lineno {
        (*openfile).current = crate::text::line_from_number(linenumber);
    } else {
        (*openfile).current = (*openfile).filebot;
    }

    (*openfile).current_x = pos_x;
    (*openfile).placewewant = utils::xplustabs();

    refresh_needed = true;
}

/* 实现“跳到行”菜单。 */
pub unsafe fn do_gotolinecolumn() {
    ask_for_line_and_column();
}

/* 询问行号与（可选的）列号，然后跳转。 */
pub unsafe fn ask_for_line_and_column() {
    let mut line = (*(*openfile).current).lineno;
    let mut column = (*openfile).placewewant as isize + 1;
    let response = files::do_prompt(
        MGOTOLINE,
        &mut answer,
        std::ptr::null_mut(),
        winio::edit_refresh,
        gettext!("Enter line number, column number"),
    );
    let mut doublesign = 0;

    /* 切换到搜索时，保留用户已输入的内容。 */
    if global::func_from_key(response) == Some(global::flip_goto as fn()) {
        UNSET(BACKWARDS_SEARCH);
        search_init(false, true);
        return;
    }

    /* 取消或空白，或运行了某函数，则结束。 */
    if response < 0 {
        statusbar(gettext!("Cancelled"));
        return;
    } else if response > 0 {
        return;
    }

    let ans = answer.clone().unwrap_or_default();
    /* ++ 或 -- 在数字前表示相对跳转。 */
    if ans.starts_with("++") || ans.starts_with("--") {
        doublesign = 1;
    }

    /* 尝试从用户响应中提取一个或两个数字。 */
    if !utils::parse_line_column(&ans[doublesign..], &mut line, &mut column) {
        statusline(message_type::AHEM, gettext!("Invalid line or column number"));
        return;
    }

    if doublesign == 1 {
        line += (*(*openfile).current).lineno;
    }
    if doublesign == 1 && line < 1 {
        line = 1;
    }

    goto_line_and_column(line, column, false);

    mv::adjust_viewport(if ans.starts_with(',') {
        update_type::STATIONARY
    } else {
        update_type::CENTERING
    });
    refresh_needed = true;
}

/* 跳到指定的行和列。（注意两者都是基于 1 的。） */
pub unsafe fn goto_line_and_column(mut line: isize, mut column: isize, hugfloor: bool) {
    let rows_from_tail: isize;

    /* 负行号表示从文件末尾算起。 */
    if line < 0 {
        line = (*(*openfile).filebot).lineno + line + 1;
    } else if line == 0 {
        line = (*(*openfile).current).lineno;
    }
    if line < 1 {
        line = 1;
    }

    if line > (*(*openfile).edittop).lineno + editwinrows as isize
        || (ISSET(SOFTWRAP) && line > (*(*openfile).current).lineno)
    {
        recook |= perturbed;
    }

    /* 迭代到请求的行。 */
    (*openfile).current = (*openfile).filetop;
    let mut remaining = line - 1;
    while remaining > 0 && (*openfile).current != (*openfile).filebot {
        (*openfile).current = (*(*openfile).current).next;
        remaining -= 1;
    }

    /* 负列号表示从行尾算起。 */
    if column < 0 {
        column = utils::breadth((*(*openfile).current).data.as_bytes()) as isize + column + 2;
    } else if column == 0 {
        column = (*openfile).placewewant as isize + 1;
    }
    if column < 1 {
        column = 1;
    }

    /* 设置与请求列对应的 x 位置。 */
    (*openfile).current_x = utils::actual_x((*(*openfile).current).data.as_bytes(), (column - 1) as usize);
    (*openfile).placewewant = (column - 1) as usize;

    if ISSET(SOFTWRAP)
        && (*openfile).placewewant / editwincols
            > utils::breadth((*(*openfile).current).data.as_bytes()) / editwincols
    {
        (*openfile).placewewant = utils::breadth((*(*openfile).current).data.as_bytes());
    }

    if !hugfloor {
        return;
    }

    if ISSET(SOFTWRAP) {
        let mut currentline = (*openfile).current;
        let mut leftedge = files::leftedge_for(utils::xplustabs() as isize, (*openfile).current);
        rows_from_tail = (editwinrows / 2) as isize
            - mv::go_forward_chunks(editwinrows / 2, &mut currentline, &mut leftedge) as isize;
    } else {
        rows_from_tail = (*(*openfile).filebot).lineno - (*(*openfile).current).lineno;
    }

    /* 若目标行靠近文件尾部，把最后一行或块放在屏幕底行；否则居中目标行。 */
    if rows_from_tail < (editwinrows / 2) as isize && !ISSET(JUMPY_SCROLLING) {
        (*openfile).cursor_row = (editwinrows - 1) as isize - rows_from_tail;
        mv::adjust_viewport(update_type::STATIONARY);
    } else {
        mv::adjust_viewport(update_type::CENTERING);
    }
}

/* 从当前位置开始，在 bracket_pair 的两个字符中搜索任意一个。
 * 若 reverse 为 true 则向后搜索，否则向前。找到返回 true，否则 false。 */
pub unsafe fn find_a_bracket(reverse: bool, bracket_pair: &str) -> bool {
    let mut line = (*openfile).current;
    let mut pointer: usize;
    let mut found: Option<usize> = None;

    let bp = bracket_pair.as_bytes();

    if reverse {
        /* 先从当前括号处移开一步。 */
        if (*openfile).current_x == 0 {
            line = (*line).prev;
            if line.is_null() {
                return false;
            }
            pointer = (*line).data.len();
        } else {
            pointer = chars::step_left((*line).data.as_bytes(), (*openfile).current_x);
        }

        /* 现在寻找我们感兴趣的两种括号中的任意一个。 */
        loop {
            found = chars::mbrevstrpbrk((*line).data.as_bytes(), bp, pointer);
            if found.is_some() {
                break;
            }
            line = (*line).prev;
            if line.is_null() {
                return false;
            }
            pointer = (*line).data.len();
        }
    } else {
        pointer = chars::step_right((*line).data.as_bytes(), (*openfile).current_x);

        loop {
            found = chars::mbstrpbrk(&(*line).data.as_bytes()[pointer..], bp);
            if found.is_some() {
                break;
            }
            line = (*line).next;
            if line.is_null() {
                return false;
            }
            pointer = 0;
        }
    }

    /* 把当前位置设为找到的括号处。 */
    (*openfile).current = line;
    (*openfile).current_x = found.unwrap();

    true
}

/* 搜索与当前光标位置处括号匹配的括号（若存在）。 */
pub unsafe fn do_find_bracket() {
    let was_current = (*openfile).current;
    let was_x = (*openfile).current_x;
    /* 光标当前位置，以防找不到配对。 */
    let mb = matchbrackets.clone().unwrap_or_default();
    let ch: usize;
    /* matchbrackets 中光标下括号的位置。 */
    let ch_len: usize;
    /* ch 的字节长度。 */
    let mut wanted_ch: usize;
    /* matchbrackets 中互补括号的位置。 */
    let wanted_ch_len: usize;
    /* wanted_ch 的字节长度。 */
    let mut bracket_pair: [u8; MAXCHARLEN * 2 + 1] = [0u8; MAXCHARLEN * 2 + 1];
    /* ch 与 wanted_ch 中的一对字符。 */
    let mut halfway: usize = 0;
    /* matchbrackets 中闭括号开始处的索引。 */
    let charcount = chars::mbstrlen(mb.as_bytes()) / 2;
    /* matchbrackets 中字符数的一半。 */
    let mut balance: isize = 1;
    /* 初始括号计数。 */
    let reverse: bool;
    /* 搜索方向。 */

    ch = match chars::mbstrchr(mb.as_bytes(), &(*(*openfile).current).data.as_bytes()[(*openfile).current_x..]) {
        Some(p) => p,
        None => {
            statusline(message_type::AHEM, gettext!("Not a bracket"));
            return;
        }
    };

    /* 找到 matchbrackets 的中点，即闭括号开始处。 */
    for _ in 0..charcount {
        halfway += chars::char_length(&mb.as_bytes()[halfway..]);
    }

    /* 在闭括号上时，需向后搜索匹配的开括号；否则向前搜索匹配的闭括号。 */
    reverse = ch >= halfway;

    /* 通过 matchbrackets 向前或向后移动半个总字符数，找到想要的互补括号。 */
    wanted_ch = ch;
    let mut cc = charcount;
    while cc > 0 {
        if reverse {
            wanted_ch = chars::step_left(mb.as_bytes(), wanted_ch);
        } else {
            wanted_ch += chars::char_length(&mb.as_bytes()[wanted_ch..]);
        }
        cc -= 1;
    }

    ch_len = chars::char_length(&mb.as_bytes()[ch..]);
    wanted_ch_len = chars::char_length(&mb.as_bytes()[wanted_ch..]);

    /* 把两个互补括号复制进单个字符串。 */
    bracket_pair[..ch_len].copy_from_slice(&mb.as_bytes()[ch..ch + ch_len]);
    bracket_pair[ch_len..ch_len + wanted_ch_len].copy_from_slice(&mb.as_bytes()[wanted_ch..wanted_ch + wanted_ch_len]);
    bracket_pair[ch_len + wanted_ch_len] = 0;

    loop {
        let bp = std::str::from_utf8(&bracket_pair[..ch_len + wanted_ch_len]).unwrap_or("");
        if !find_a_bracket(reverse, bp) {
            break;
        }
        /* 对相同/其他括号增减 balance。 */
        let cur = (*(*openfile).current).data.as_bytes();
        let cx = (*openfile).current_x;
        balance += if &cur[cx..cx + ch_len] == &mb.as_bytes()[ch..ch + ch_len] {
            1
        } else {
            -1
        };

        /* 当 balance 归零，找到了互补括号。 */
        if balance == 0 {
            mv::edit_redraw(was_current, update_type::FLOWING);
            return;
        }
    }

    statusline(message_type::AHEM, gettext!("No matching bracket"));

    /* 恢复光标位置。 */
    (*openfile).current = was_current;
    (*openfile).current_x = was_x;
}

/* 在当前行没有锚点时放置锚点，否则移除它。 */
pub unsafe fn put_or_lift_anchor() {
    (*(*openfile).current).has_anchor = !(*(*openfile).current).has_anchor;

    if (*openfile).current != (*openfile).filetop {
        mv::update_line((*openfile).current, (*openfile).current_x);
    } else {
        refresh_needed = true;
    }

    if !ISSET(LINE_NUMBERS) && (!ISSET(MINIBAR) || ISSET(ZERO)) {
        if (*(*openfile).current).has_anchor {
            statusline(message_type::REMARK, gettext!("Placed anchor"));
        } else {
            statusline(message_type::HUSH, gettext!("Removed anchor"));
        }
    }
}

/* 把给定行设为当前行，或报告锚点情况。 */
pub unsafe fn go_to_and_confirm(line: *mut linestruct) {
    let was_current = (*openfile).current;

    if line != (*openfile).current {
        (*openfile).current = line;
        (*openfile).current_x = 0;
        if (*line).lineno > (*(*openfile).edittop).lineno + editwinrows as isize
            || (ISSET(SOFTWRAP) && (*line).lineno > (*was_current).lineno)
        {
            recook |= perturbed;
        }
        mv::edit_redraw(was_current, update_type::CENTERING);
        if !ISSET(LINE_NUMBERS) {
            statusbar(gettext!("Jumped to anchor"));
        }
    } else if (*(*openfile).current).has_anchor {
        statusline(message_type::REMARK, gettext!("This is the only anchor"));
    } else {
        statusline(message_type::AHEM, gettext!("There are no anchors"));
    }
}

/* 跳到当前行之前的第一个锚点；在顶部绕回。 */
pub unsafe fn to_prev_anchor() {
    let mut line = (*openfile).current;

    loop {
        line = if !(*line).prev.is_null() {
            (*line).prev
        } else {
            (*openfile).filebot
        };
        if (*line).has_anchor || line == (*openfile).current {
            break;
        }
    }

    go_to_and_confirm(line);
}

/* 跳到当前行之后的第一个锚点；在底部绕回。 */
pub unsafe fn to_next_anchor() {
    let mut line = (*openfile).current;

    loop {
        line = if !(*line).next.is_null() {
            (*line).next
        } else {
            (*openfile).filetop
        };
        if (*line).has_anchor || line == (*openfile).current {
            break;
        }
    }

    go_to_and_confirm(line);
}

/* 以下为 ncurses 相关桩函数（在 winio.rs 实现前提供）。 */
pub fn nodelay(_win: *mut std::ffi::c_void, _bf: bool) {}
pub fn wgetch(_win: *mut std::ffi::c_void) -> i32 { 0 }
pub fn get_input(_p: *mut i32) -> i32 { 0 }
pub fn print_view_warning() {}
pub fn napms(_ms: i32) {}
