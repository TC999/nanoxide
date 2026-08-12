/**************************************************************************
 *   chars.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 chars.c）。
 *
 *   版权 (C) 2001-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2016-2021 Benno Schulenberg
 **************************************************************************/

//! 字符处理函数。对应原版 nano 的 `chars.c`。

use crate::definitions::*;

/* 以下全局变量在原版中定义于 global.c，但 chars.c 依赖它们。
 * 在此先行声明，供本模块及后续模块使用。 */

/// 是否使用 UTF-8 编码。
pub static mut using_utf8: bool = true;

/// 额外的"构成单词"字符集合（None 表示无）。
pub static mut word_chars: Option<String> = None;

/// 是否把一个内嵌的换行当作编码后的 NUL 来处理。
pub static mut as_an_at: bool = false;

/// 制表符的宽度（以列计）。
pub static mut tabsize: usize = 8;

/* 把有符号的 char 当作 i8 处理。 */
#[inline]
fn schar(b: u8) -> i32 {
    b as i8 as i32
}

/* Return TRUE when the given character is some kind of letter or a digit. */
pub fn is_alnum_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    let wc = mbtowide(c);
    match wc {
        Some(w) => char::from_u32(w as u32).map_or(false, |ch| ch.is_alphanumeric()),
        None => (c[0] as u8).is_ascii_alphanumeric(),
    }
}

/* Return TRUE when the given character is space or tab or other whitespace. */
pub fn is_blank_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    if schar(c[0]) >= 0 {
        return c[0] == b' ' || c[0] == b'\t';
    }
    let wc = mbtowide(c);
    match wc {
        Some(w) => char::from_u32(w as u32).map_or(false, |ch| ch.is_whitespace()),
        None => false,
    }
}

/* Return TRUE when the given character is a control character. */
pub fn is_cntrl_char(c: &[u8]) -> bool {
    unsafe {
        if using_utf8 {
            return ((c[0] & 0xE0) == 0
                || c[0] == DEL_CODE
                || (schar(c[0]) == -62 && schar(c[1]) < -96));
        }
    }
    ((c[0] & 0x60) == 0 || c[0] == DEL_CODE)
}

/* Return TRUE when the given character is a punctuation character. */
pub fn is_punct_char(c: &[u8]) -> bool {
    let wc = mbtowide(c);
    match wc {
        Some(w) => {
            let ch = char::from_u32(w as u32);
            ch.map_or(false, |ch| {
                (!ch.is_alphanumeric() && !ch.is_whitespace() && !ch.is_control())
                    || ch.is_ascii_punctuation()
            })
        }
        None => (c[0] as u8).is_ascii_punctuation(),
    }
}

/* Return TRUE when the given character is word-forming. */
pub fn is_word_char(c: &[u8], allow_punct: bool) -> bool {
    if c.is_empty() || c[0] == 0 {
        return false;
    }
    if is_alnum_char(c) {
        return true;
    }
    if allow_punct && is_punct_char(c) {
        return true;
    }
    unsafe {
        if let Some(ref wc) = word_chars {
            if !wc.is_empty() {
                let mut symbol = [0u8; MAXCHARLEN + 1];
                let symlen = collect_char(c, &mut symbol);
                let symbol = &symbol[..symlen];
                return wc.as_bytes().windows(symlen).any(|w| w == symbol);
            }
        }
    }
    false
}

/* Return the visible representation of control character c. */
pub fn control_rep(c: i32) -> u8 {
    if c == DEL_CODE as i32 {
        b'?'
    } else if c == -97 {
        b'='
    } else if c < 0 {
        (c + 224) as u8
    } else {
        (c + 64) as u8
    }
}

/* Return the visible representation of multibyte control character c. */
pub fn control_mbrep(c: &[u8], isdata: bool) -> u8 {
    if c[0] == b'\n' && (isdata || unsafe { as_an_at }) {
        return b'@';
    }
    unsafe {
        if using_utf8 {
            if c[0] as u8 >= 128 {
                return control_rep(schar(c[0]));
            } else {
                return control_rep(schar(c[1]));
            }
        }
    }
    control_rep(c[0] as i32)
}

/* Convert the given multibyte sequence c to wide character, and return
 * the number of bytes in the sequence, or None for an invalid sequence. */
pub fn mbtowide(c: &[u8]) -> Option<u32> {
    unsafe {
        if schar(c[0]) < 0 && using_utf8 {
            let v1 = c[0];
            let v2 = c[1] ^ 0x80;
            if v2 > 0x3F || v1 < 0xC2 {
                return None;
            }
            if v1 < 0xE0 {
                return Some((((v1 & 0x1F) as u32) << 6) | (v2 as u32));
            }
            let v3 = c[2] ^ 0x80;
            if v3 > 0x3F {
                return None;
            }
            if v1 < 0xF0 {
                if (v1 > 0xE0 || v2 >= 0x20) && (v1 != 0xED || v2 < 0x20) {
                    return Some((((v1 & 0x0F) as u32) << 12) | ((v2 as u32) << 6) | (v3 as u32));
                } else {
                    return None;
                }
            }
            let v4 = c[3] ^ 0x80;
            if v4 > 0x3F || v1 > 0xF4 {
                return None;
            }
            if (v1 > 0xF0 || v2 >= 0x10) && (v1 != 0xF4 || v2 < 0x10) {
                return Some(
                    (((v1 & 0x07) as u32) << 18)
                        | ((v2 as u32) << 12)
                        | ((v3 as u32) << 6)
                        | (v4 as u32),
                );
            } else {
                return None;
            }
        }
    }
    Some(c[0] as u32)
}

/* Return TRUE when the given character occupies two cells. */
pub fn is_doublewidth(ch: &[u8]) -> bool {
    let b0 = ch[0] as u8;
    if b0 < 0xE1 || unsafe { !using_utf8 } {
        return false;
    }
    match mbtowide(ch) {
        Some(w) => {
            let c = char::from_u32(w).unwrap_or(' ');
            unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) == 2
        }
        None => false,
    }
}

/* Return TRUE when the given character occupies zero cells. */
pub fn is_zerowidth(ch: &[u8]) -> bool {
    let b0 = ch[0] as u8;
    if b0 < 0xCC || unsafe { !using_utf8 } {
        return false;
    }
    match mbtowide(ch) {
        Some(w) => {
            let c = char::from_u32(w).unwrap_or(' ');
            unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) == 0
        }
        None => false,
    }
}

/* Return the number of bytes in the character that starts at *pointer. */
pub fn char_length(pointer: &[u8]) -> usize {
    unsafe {
        if pointer[0] > 0xC1 && using_utf8 {
            let c1 = pointer[0];
            let c2 = pointer[1];
            if (c2 ^ 0x80) > 0x3F {
                return 1;
            }
            if c1 < 0xE0 {
                return 2;
            }
            if (pointer[2] ^ 0x80) > 0x3F {
                return 1;
            }
            if c1 < 0xF0 {
                if (c1 > 0xE0 || c2 >= 0xA0) && (c1 != 0xED || c2 < 0xA0) {
                    return 3;
                } else {
                    return 1;
                }
            }
            if (pointer[3] ^ 0x80) > 0x3F {
                return 1;
            }
            if c1 > 0xF4 {
                return 1;
            }
            if (c1 > 0xF0 || c2 >= 0x90) && (c1 != 0xF4 || c2 < 0x90) {
                return 4;
            }
        }
    }
    1
}

/* Return the number of (multibyte) characters in the given string. */
pub fn mbstrlen(pointer: &[u8]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < pointer.len() && pointer[i] != 0 {
        let len = char_length(&pointer[i..]);
        i += len;
        count += 1;
    }
    count
}

/* Return the length (in bytes) of the character at the start of the
 * given string, and return a copy of this character in *thechar. */
pub fn collect_char(string: &[u8], thechar: &mut [u8]) -> usize {
    let charlen = char_length(string);
    for i in 0..charlen {
        thechar[i] = string[i];
    }
    charlen
}

/* Return the length (in bytes) of the character at the start of
 * the given string, and add this character's width to *column. */
pub fn advance_over(string: &[u8], column: &mut usize) -> usize {
    unsafe {
        if schar(string[0]) < 0 && using_utf8 {
            if string[0] as u8 == 0xC2 && schar(string[1]) < -96 {
                *column += 2;
                return 2;
            } else {
                match mbtowide(string) {
                    Some(w) => {
                        let c = char::from_u32(w).unwrap_or(' ');
                        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                        *column += if width < 0 { 1 } else { width as usize };
                        return char_length(string);
                    }
                    None => {
                        *column += 1;
                        return 1;
                    }
                }
            }
        }
    }
    let b0 = string[0] as u8;
    if b0 < 0x20 {
        if string[0] == b'\t' {
            let ts = unsafe { tabsize };
            *column += ts - *column % ts;
        } else {
            *column += 2;
        }
    } else if 0x7E < b0 && b0 < 0xA0 {
        *column += 2;
    } else {
        *column += 1;
    }
    1
}

/* Return the index in buf of the beginning of the multibyte character
 * before the one at pos. */
pub fn step_left(buf: &[u8], pos: usize) -> usize {
    unsafe {
        if using_utf8 {
            let charlen = 0;
            let before = if pos < 4 {
                0
            } else {
                if schar(buf[pos - 1]) > -65 {
                    pos - 1
                } else if schar(buf[pos - 2]) > -65 {
                    pos - 2
                } else if schar(buf[pos - 3]) > -65 {
                    pos - 3
                } else if schar(buf[pos - 4]) > -65 {
                    pos - 4
                } else {
                    pos - 1
                }
            };
            let mut before = before;
            let mut charlen = charlen;
            while before < pos {
                charlen = char_length(&buf[before..]);
                before += charlen;
            }
            return before - charlen;
        }
    }
    if pos == 0 {
        0
    } else {
        pos - 1
    }
}

/* Return the index in buf of the beginning of the multibyte character
 * after the one at pos. */
pub fn step_right(buf: &[u8], pos: usize) -> usize {
    pos + char_length(&buf[pos..])
}

/* This function is equivalent to strcasecmp() for multibyte strings. */
pub fn mbstrcasecmp(s1: &[u8], s2: &[u8]) -> i32 {
    mbstrncasecmp(s1, s2, HIGHEST_POSITIVE)
}

/* This function is equivalent to strncasecmp() for multibyte strings. */
pub fn mbstrncasecmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    unsafe {
        if using_utf8 {
            let mut s1 = s1;
            let mut s2 = s2;
            let mut n = n;
            while !s1.is_empty() && !s2.is_empty() && n > 0 {
                if schar(s1[0]) >= 0 && schar(s2[0]) >= 0 {
                    let a = (s1[0] as u8) & 0x5F;
                    let b = (s2[0] as u8) & 0x5F;
                    if b'A' <= a && a <= b'Z' {
                        if b'A' <= b && b <= b'Z' {
                            if a != b {
                                return (a as i32) - (b as i32);
                            }
                        } else {
                            return ((s1[0] as u8 | 0x20) as i32) - (s2[0] as i32);
                        }
                    } else if b'A' <= b && b <= b'Z' {
                        return (s1[0] as i32) - ((s2[0] as u8 | 0x20) as i32);
                    } else if s1[0] != s2[0] {
                        return (s1[0] as i32) - (s2[0] as i32);
                    }
                    s1 = &s1[1..];
                    s2 = &s2[1..];
                    n -= 1;
                    continue;
                }
                let wc1 = mbtowide(s1);
                let wc2 = mbtowide(s2);
                let bad1 = wc1.is_none();
                let bad2 = wc2.is_none();
                if bad1 || bad2 {
                    if s1[0] != s2[0] {
                        return (s1[0] as i32) - (s2[0] as i32);
                    }
                    if bad1 != bad2 {
                        return if bad1 { 1 } else { -1 };
                    }
                } else {
                    let c1 = char::from_u32(wc1.unwrap()).unwrap_or('\0');
                    let c2 = char::from_u32(wc2.unwrap()).unwrap_or('\0');
                    let difference =
                        (c1.to_ascii_lowercase() as i32) - (c2.to_ascii_lowercase() as i32);
                    if difference != 0 {
                        return difference;
                    }
                }
                let l1 = char_length(s1);
                let l2 = char_length(s2);
                s1 = &s1[l1..];
                s2 = &s2[l2..];
                n -= 1;
            }
            if n > 0 {
                return (s1.first().copied().unwrap_or(0) as i32)
                    - (s2.first().copied().unwrap_or(0) as i32);
            } else {
                return 0;
            }
        } else {
            return strncasecmp(s1, s2, n);
        }
    }
    0
}

/* 对字节切片做不区分大小写的比较，最多比较 n 个字节。 */
pub fn strncasecmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = s1.get(i).copied().unwrap_or(0);
        let b = s2.get(i).copied().unwrap_or(0);
        let la = (a as char).to_ascii_lowercase() as u8;
        let lb = (b as char).to_ascii_lowercase() as u8;
        if la != lb {
            return (la as i32) - (lb as i32);
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/* This function is equivalent to strcasestr() for multibyte strings. */
pub fn mbstrcasestr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    unsafe {
        if using_utf8 {
            let needle_len = mbstrlen(needle);
            let mut h = 0;
            while h < haystack.len() && haystack[h] != 0 {
                if mbstrncasecmp(&haystack[h..], needle, needle_len) == 0 {
                    return Some(h);
                }
                h += char_length(&haystack[h..]);
            }
            return None;
        }
    }
    casestr(haystack, needle)
}

/* 在字节切片中做不区分大小写的子串查找。 */
pub fn casestr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
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
        if strncasecmp(&haystack[i..], needle, nlen) == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

/* This function is equivalent to strstr(), except in that it scans the
 * string in reverse, starting at pointer. */
pub fn revstrstr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    let needle_len = needle.len();
    let tail_len = haystack.len() - pointer.min(haystack.len());
    let mut pointer = if tail_len < needle_len {
        pointer - (needle_len - tail_len)
    } else {
        pointer
    };
    while pointer as isize >= 0 && pointer + needle_len <= haystack.len() {
        if &haystack[pointer..pointer + needle_len] == needle {
            return Some(pointer);
        }
        if pointer == 0 {
            break;
        }
        pointer -= 1;
    }
    None
}

/* This function is equivalent to strcasestr(), except in that it scans
 * the string in reverse, starting at pointer. */
pub fn revstrcasestr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    let needle_len = needle.len();
    let tail_len = haystack.len() - pointer.min(haystack.len());
    let mut pointer = if tail_len < needle_len {
        pointer - (needle_len - tail_len)
    } else {
        pointer
    };
    while pointer as isize >= 0 && pointer + needle_len <= haystack.len() {
        if strncasecmp(&haystack[pointer..], needle, needle_len) == 0 {
            return Some(pointer);
        }
        if pointer == 0 {
            break;
        }
        pointer -= 1;
    }
    None
}

/* This function is equivalent to strcasestr() for multibyte strings,
 * except in that it scans the string in reverse, starting at pointer. */
pub fn mbrevstrcasestr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    unsafe {
        if using_utf8 {
            let needle_len = mbstrlen(needle);
            let tail_len = mbstrlen(&haystack[pointer..]);
            let mut pointer = if tail_len < needle_len {
                pointer - (needle_len - tail_len)
            } else {
                pointer
            };
            if (pointer as isize) < 0 {
                return None;
            }
            loop {
                if mbstrncasecmp(&haystack[pointer..], needle, needle_len) == 0 {
                    return Some(pointer);
                }
                if pointer == 0 {
                    return None;
                }
                pointer = step_left(haystack, pointer);
            }
        }
    }
    revstrcasestr(haystack, needle, pointer)
}

/* This function is equivalent to strchr() for multibyte strings. */
pub fn mbstrchr(string: &[u8], chr: &[u8]) -> Option<usize> {
    unsafe {
        if using_utf8 {
            let mut bad_c = false;
            let wc = mbtowide(chr);
            let wc = if wc.is_none() {
                bad_c = true;
                chr[0] as u32
            } else {
                wc.unwrap()
            };
            let mut s = string;
            let mut idx = 0;
            while !s.is_empty() && s[0] != 0 {
                let symlen = mbtowide(s);
                let ws = if symlen.is_none() { s[0] as u32 } else { symlen.unwrap() };
                let bad_s = symlen.is_none();
                if ws == wc && bad_s == bad_c {
                    return Some(idx);
                }
                let l = char_length(s);
                idx += l;
                s = &s[l..];
            }
            return None;
        }
    }
    let ch = chr.first().copied().unwrap_or(0);
    string.iter().position(|&b| b == ch)
}

/* Locate, in the given string, the first occurrence of any of
 * the characters in accept, searching forward. */
pub fn mbstrpbrk(string: &[u8], accept: &[u8]) -> Option<usize> {
    let mut s = string;
    let mut idx = 0;
    while !s.is_empty() && s[0] != 0 {
        if mbstrchr(accept, s).is_some() {
            return Some(idx);
        }
        let l = char_length(s);
        idx += l;
        s = &s[l..];
    }
    None
}

/* Locate, in the string that starts at head, the first occurrence of any of
 * the characters in accept, starting from pointer and searching backwards. */
pub fn mbrevstrpbrk(head: &[u8], accept: &[u8], pointer: usize) -> Option<usize> {
    let mut pointer = if head.get(pointer).copied().unwrap_or(0) == 0 {
        if pointer == 0 {
            return None;
        }
        step_left(head, pointer)
    } else {
        pointer
    };
    loop {
        if mbstrchr(accept, &head[pointer..]).is_some() {
            return Some(pointer);
        }
        if pointer == 0 {
            return None;
        }
        pointer = step_left(head, pointer);
    }
}

/* Return TRUE if the given string contains at least one blank character. */
pub fn has_blank_char(string: &[u8]) -> bool {
    let mut s = string;
    while !s.is_empty() && s[0] != 0 && !is_blank_char(s) {
        let l = char_length(s);
        s = &s[l..];
    }
    !s.is_empty() && s[0] != 0
}

/* Return TRUE when the given string is empty or consists of only blanks. */
pub fn white_string(string: &[u8]) -> bool {
    let mut s = string;
    while !s.is_empty() && s[0] != 0 && (is_blank_char(s) || s[0] == b'\r') {
        let l = char_length(s);
        s = &s[l..];
    }
    s.is_empty() || s[0] == 0
}

/* Remove leading whitespace from the given string. */
pub fn strip_leading_blanks_from(string: &mut [u8]) {
    let mut i = 0;
    while i < string.len() && (string[i] == b' ' || string[i] == b'\t') {
        for j in 0..string.len() - 1 {
            string[j] = string[j + 1];
        }
        if !string.is_empty() {
            string[string.len() - 1] = 0;
        }
        i += 1;
    }
}
