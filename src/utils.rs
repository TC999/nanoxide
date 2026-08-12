/**************************************************************************
 *   utils.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 utils.c）。
 *
 *   版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2016, 2017, 2019, 2020, 2026 Benno Schulenberg
 **************************************************************************/

//! 通用工具函数。对应原版 nano 的 `utils.c`。

use std::env;

use crate::chars;
use crate::definitions::*;

/* Set global variable `homedir` to the user's home directory.  First try
 * $HOME, otherwise consult the password database for the current UID. */
pub fn get_homedir() {
    unsafe {
        if homedir.is_none() {
            let mut homenv = env::var("HOME").ok();

            if homenv.is_none() || geteuid() == ROOT_UID {
                if let Some(home) = dirs::home_dir() {
                    let s = home.to_string_lossy().to_string();
                    if !s.is_empty() {
                        homenv = Some(s);
                    }
                }
            }

            if let Some(ref h) = homenv {
                if !h.is_empty() {
                    homedir = Some(h.clone());
                }
            }
        }
    }
}

/* 返回当前有效用户 ID（对应 geteuid）。 */
pub unsafe fn geteuid() -> u32 {
    0
}

/* Return the filename part of the given path. */
pub fn tail(path: &str) -> &str {
    match path.rfind('/') {
        None => path,
        Some(slash) => &path[slash + 1..],
    }
}

/* Return a copy of the two given strings, welded together. */
pub fn concatenate(path: &str, name: &str) -> String {
    let mut joined = String::with_capacity(path.len() + name.len() + 1);
    joined.push_str(path);
    joined.push_str(name);
    joined
}

/* Return the number of digits that the given integer n takes up. */
pub fn digits(n: isize) -> i32 {
    let n = if n < 0 { -n } else { n };
    if n < 100000 {
        if n < 1000 {
            if n < 100 {
                2
            } else {
                3
            }
        } else {
            if n < 10000 {
                4
            } else {
                5
            }
        }
    } else {
        if n < 10000000 {
            if n < 1000000 {
                6
            } else {
                7
            }
        } else {
            if n < 100000000 {
                8
            } else {
                9
            }
        }
    }
}

/* Read an integer from the given string.  If it parses okay,
 * store it in *result and return TRUE; otherwise, return FALSE. */
pub fn parse_num(string: &str, result: &mut isize) -> bool {
    match string.trim().parse::<isize>() {
        Ok(v) => {
            *result = v;
            true
        }
        Err(_) => false,
    }
}

/* Read one number (or two numbers separated by comma, period, or colon)
 * from the given string and store the number(s) in *line (and *column). */
pub fn parse_line_column(string: &str, line: &mut isize, column: &mut isize) -> bool {
    let mut s = string;
    while s.starts_with(' ') {
        s = &s[1..];
    }

    let comma = s.find(|c| c == ',' || c == '.' || c == ':');

    if comma.is_none() {
        return parse_num(s, line);
    }

    let comma = comma.unwrap();
    let mut retval = parse_num(&s[comma + 1..], column);

    if comma == 0 {
        return retval;
    }

    let firstpart = &s[..comma];
    retval = parse_num(firstpart, line) && retval;

    retval
}

/* In the given string, recode each embedded NUL as a newline. */
pub fn recode_NUL_to_LF(string: &mut [u8], length: usize) {
    let mut i = 0;
    while i < length && i < string.len() {
        if string[i] == 0 {
            string[i] = b'\n';
        }
        i += 1;
    }
}

/* In the given string, recode each embedded newline as a NUL,
 * and return the number of bytes in the string. */
pub fn recode_LF_to_NUL(string: &mut [u8]) -> usize {
    let mut i = 0;
    while i < string.len() && string[i] != 0 {
        if string[i] == b'\n' {
            string[i] = 0;
        }
        i += 1;
    }
    i
}

/* Free the memory of the given array, which should contain len elements. */
pub fn free_chararray(array: Vec<String>) {
    drop(array);
}

/* Is the word starting at the given position in `text` and of the given
 * length a separate word?  That is: is it not part of a longer word? */
pub fn is_separate_word(position: usize, length: usize, text: &[u8]) -> bool {
    let before_idx = chars::step_left(text, position);
    let after = position + length;

    let before_is_alpha = if position == 0 {
        false
    } else {
        chars::is_alnum_char(&text[before_idx..])
    };
    let after_is_alpha = if after >= text.len() {
        false
    } else {
        chars::is_alnum_char(&text[after..])
    };

    ((position == 0 || !before_is_alpha) && (after >= text.len() || !after_is_alpha))
}

/* Return the position of the needle in the haystack, or None if not found. */
pub fn strstrwrapper(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    unsafe {
        if ISSET(USE_REGEXP) {
            let sr = match &search_regexp {
                Some(r) => r,
                None => return None,
            };
            if ISSET(BACKWARDS_SEARCH) {
                let mut last_find: usize = 0;
                let ceiling: usize = start;
                let mut floor: usize = 0;
                let mut next_rung: usize = 0;

                if sr.captures(&String::from_utf8_lossy(haystack)).is_none() {
                    return None;
                }

                if last_find > ceiling {
                    return None;
                }

                let mut pos = 0usize;
                loop {
                    let tmp = String::from_utf8_lossy(&haystack[pos..]);
                    let m = sr.captures(&tmp);
                    if m.is_none() {
                        break;
                    }
                    let m = m.unwrap();
                    let rm_so = pos + m.get(0).unwrap().start();
                    let rm_eo = pos + m.get(0).unwrap().end();
                    if rm_so > ceiling {
                        break;
                    }
                    floor = next_rung;
                    last_find = rm_so;
                    fill_regmatches(&m);
                    if last_find == ceiling {
                        break;
                    }
                    next_rung = chars::step_right(haystack, last_find);
                    pos = next_rung;
                }

                return Some(last_find);
            } else {
                let tmp = String::from_utf8_lossy(&haystack[start..]);
                let m = sr.captures(&tmp);
                return match m {
                    Some(c) => {
                        let rm_so = start + c.get(0).unwrap().start();
                        fill_regmatches(&c);
                        Some(rm_so)
                    }
                    None => None,
                };
            }
        }
    }

    if unsafe { ISSET(CASE_SENSITIVE) } {
        if unsafe { ISSET(BACKWARDS_SEARCH) } {
            return chars::revstrstr(haystack, needle, start);
        } else {
            return find_substring(&haystack[start..], needle).map(|x| start + x);
        }
    }

    if unsafe { ISSET(BACKWARDS_SEARCH) } {
        return chars::mbrevstrcasestr(haystack, needle, start);
    } else {
        return chars::mbstrcasestr(&haystack[start..], needle).map(|x| start + x);
    }
}

/* 把一次正则匹配的全部捕获组（含第 0 组）写入全局 regmatches，
 * 供 findnextstr 计算匹配长度以及 replace_regexp 反向引用使用。 */
pub fn fill_regmatches(caps: &regex::Captures) {
    unsafe {
        for i in 0..10 {
            regmatches[i] = (None, None);
        }
        for i in 0..caps.len() {
            if i >= 10 {
                break;
            }
            if let Some(m) = caps.get(i) {
                regmatches[i] = (Some(m.start()), Some(m.end()));
            }
        }
    }
}

/* 在字节切片中做区分大小写的子串查找。 */
pub fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let hlen = haystack.len();
    let nlen = needle.len();
    if nlen > hlen {
        return None;
    }
    let mut i = 0;
    while i + nlen <= hlen {
        if &haystack[i..i + nlen] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/* Allocate the given amount of memory and return a pointer to it. */
pub fn nmalloc(howmuch: usize) -> Vec<u8> {
    vec![0u8; howmuch]
}

/* Reallocate the given section of memory to have the given size. */
pub fn nrealloc(mut section: Vec<u8>, howmuch: usize) -> Vec<u8> {
    section.resize(howmuch, 0);
    section
}

/* Return an appropriately reallocated dest string holding a copy of src. */
pub fn mallocstrcpy(_dest: Option<Vec<u8>>, src: &[u8]) -> Vec<u8> {
    src.to_vec()
}

/* Free the string at dest and return the string at src. */
pub fn free_and_assign(_dest: Option<Vec<u8>>, src: Vec<u8>) -> Vec<u8> {
    src
}

/* When not softwrapping, nano scrolls the current line horizontally by
 * chunks ("pages").  Return the column number of the first character
 * displayed in the edit window when the cursor is at the given column. */
pub fn get_page_start(column: usize) -> usize {
    unsafe {
        if united_sidescroll {
            let of = &*openfile;
            if column < CUSHION {
                return 0;
            } else if column < of.brink + CUSHION {
                if ISSET(JUMPY_SCROLLING) {
                    return if column > editwincols / 2 {
                        column - editwincols / 2
                    } else {
                        0
                    };
                } else {
                    return column - CUSHION;
                }
            } else if column > of.brink + editwincols - CUSHION - 1 {
                return column - editwincols
                    + (if ISSET(JUMPY_SCROLLING) {
                        editwincols / 2
                    } else {
                        CUSHION
                    })
                    + 1;
            } else {
                return of.brink;
            }
        }
    }

    if column == 0 || column + 2 < unsafe { editwincols } || unsafe { ISSET(SOFTWRAP) } {
        return 0;
    } else if unsafe { editwincols } > 8 {
        return column - 6 - (column - 6) % (unsafe { editwincols } - 8);
    } else {
        return column - (unsafe { editwincols } - 2);
    }
}

/* Return the index in the given text of the character that (when displayed)
 * will not overshoot the given column. */
pub fn actual_x(text: &[u8], column: usize) -> usize {
    let mut width: usize = 0;
    let mut i = 0usize;

    while i < text.len() && text[i] != 0 {
        let charlen = chars::advance_over(&text[i..], &mut width);

        if width > column {
            break;
        }

        i += charlen;
    }

    i
}

/* Return the number of columns that the first count bytes of text occupy. */
pub fn wideness(text: &[u8], count: usize) -> usize {
    let mut width: usize = 0;
    let mut remaining = count;
    let mut i = 0usize;

    if count == 0 {
        return 0;
    }

    while i < text.len() && text[i] != 0 {
        let charlen = chars::advance_over(&text[i..], &mut width);

        if remaining <= charlen {
            break;
        }

        remaining -= charlen;
        i += charlen;
    }

    width
}

/* Return the number of columns that the given text occupies. */
pub fn breadth(text: &[u8]) -> usize {
    let mut span: usize = 0;
    let mut i = 0usize;

    while i < text.len() && text[i] != 0 {
        let charlen = chars::advance_over(&text[i..], &mut span);
        i += charlen;
    }

    span
}

/* Return the (zero-based) column position of the cursor. */
pub fn xplustabs() -> usize {
    unsafe {
        let of = &*openfile;
        let cur = &*of.current;
        wideness(cur.data.as_bytes(), of.current_x)
    }
}

/* Append a new magic line to the end of the buffer. */
pub fn new_magicline() {
    unsafe {
        let of = &mut *openfile;
        let bot = &mut *of.filebot;
        let mut newnode = make_new_node(bot);
        newnode.data = String::new();
        newnode.prev = of.filebot;
        bot.next = Box::into_raw(newnode);
        of.filebot = bot.next;
        of.totsize += 1;
    }
}

/* Remove the magic line from the end of the buffer, if there is one and
 * it isn't the only line in the file. */
pub fn remove_magicline() {
    unsafe {
        let of = &mut *openfile;
        let bot = &*of.filebot;
        let is_only = of.filebot == of.filetop;
        if bot.data.as_bytes().first().copied().unwrap_or(0) == 0 && !is_only {
            if of.current == of.filebot {
                of.current = (*of.filebot).prev;
            }
            let newbot = (*of.filebot).prev;
            of.filebot = newbot;
            let _ = Box::from_raw((*of.filebot).next);
            (*of.filebot).next = std::ptr::null_mut();
            of.totsize -= 1;
        }
    }
}

/* Return TRUE when the mark is before or at the cursor, and FALSE otherwise. */
pub fn mark_is_before_cursor() -> bool {
    unsafe {
        let of = &*openfile;
        let mark = &*of.mark;
        let cur = &*of.current;
        (mark.lineno < cur.lineno)
            || (of.mark == of.current && of.mark_x <= of.current_x)
    }
}

/* Return in (top, top_x) and (bot, bot_x) the start and end "coordinates"
 * of the marked region. */
pub unsafe fn get_region(
    top: *mut *mut linestruct,
    top_x: *mut usize,
    bot: *mut *mut linestruct,
    bot_x: *mut usize,
) {
    if mark_is_before_cursor() {
        let of = &*openfile;
        *top = of.mark;
        *top_x = of.mark_x;
        *bot = of.current;
        *bot_x = of.current_x;
    } else {
        let of = &*openfile;
        *bot = of.mark;
        *bot_x = of.mark_x;
        *top = of.current;
        *top_x = of.current_x;
    }
}

/* Get the set of lines to work on -- either just the current line, or the
 * first to last lines of the marked region. */
pub unsafe fn get_range(top: *mut *mut linestruct, bot: *mut *mut linestruct) {
    if (*openfile).mark.is_null() {
        let of = &*openfile;
        *top = of.current;
        *bot = of.current;
    } else {
        let mut top_x: usize = 0;
        let mut bot_x: usize = 0;
        get_region(top, &mut top_x, bot, &mut bot_x);

        if bot_x == 0 && *bot != *top && !also_the_last {
            *bot = (**bot).prev;
        } else {
            also_the_last = true;
        }
    }
}

/* Return a pointer to the line that has the given line number. */
pub unsafe fn line_from_number(number: isize) -> *mut linestruct {
    let of = &*openfile;
    let mut line = of.current;
    if (*line).lineno > number {
        while (*line).lineno != number {
            line = (*line).prev;
        }
    } else {
        while (*line).lineno != number {
            line = (*line).next;
        }
    }
    line
}

/* Count the number of characters from begin to end, and return it. */
pub unsafe fn number_of_characters_in(begin: *const linestruct, end: *const linestruct) -> usize {
    let mut count: usize = 0;
    let mut line: *const linestruct = begin;

    loop {
        count += chars::mbstrlen((*line).data.as_bytes()) + 1;
        if line == end {
            break;
        }
        line = (*line).next;
    }

    count - 1
}
