/**************************************************************************
 * chars.rs  --  GNU nano 字符处理函数（对应 chars.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 字符处理函数。对应原版 nano 的 `chars.c`。
//! 转换说明：使用 `GLOBAL` 代替 `static mut` 全局变量。

use crate::definitions::*;

/// 检查是否使用 UTF-8。
pub fn using_utf8() -> bool {
    with_global(|g| g.using_utf8)
}

/// 设置 UTF-8 状态。
pub fn set_using_utf8(val: bool) {
    with_global_mut(|g| g.using_utf8 = val);
}

/// 获取单词字符集。
pub fn word_chars() -> Option<String> {
    with_global(|g| g.word_chars.clone())
}

/// 获取 as_an_at 标志。
pub fn as_an_at() -> bool {
    with_global(|g| g.as_an_at)
}

/// 获取制表符宽度。
pub fn tabsize() -> usize {
    with_global(|g| g.tabsize)
}

/// 将字符转换为宽字符（wchar_t 模拟）。
pub fn mbtowide(c: &[u8]) -> Option<u32> {
    if c.is_empty() {
        return None;
    }
    if using_utf8() {
        let len = utf8_char_len(c[0]);
        if len > c.len() {
            return None;
        }
        let s = std::str::from_utf8(&c[..len]).ok()?;
        let ch = s.chars().next()?;
        Some(ch as u32)
    } else {
        Some(c[0] as u32)
    }
}

/// 计算 UTF-8 字符长度。
fn utf8_char_len(first: u8) -> usize {
    if first & 0x80 == 0 { 1 }
    else if first & 0xE0 == 0xC0 { 2 }
    else if first & 0xF0 == 0xE0 { 3 }
    else if first & 0xF8 == 0xF0 { 4 }
    else { 1 }
}

/// 当前字节位置的字符宽度（列数）。
pub fn mb_cur_max(data: &[u8], pos: usize) -> usize {
    if data.is_empty() || pos >= data.len() {
        return 0;
    }
    if using_utf8() {
        let len = utf8_char_len(data[pos]);
        let end = (pos + len).min(data.len());
        let s = std::str::from_utf8(&data[pos..end]).ok();
        match s {
            Some(ch) => ch.chars().next().map(|c| c.len_utf8()).unwrap_or(1),
            None => 1,
        }
    } else {
        1
    }
}

/// 向左移动一个字符。
pub fn move_mbleft(data: &[u8], pos: &mut usize) {
    if *pos == 0 {
        return;
    }
    if using_utf8() {
        *pos -= 1;
        while *pos > 0 && (data[*pos] & 0xC0) == 0x80 {
            *pos -= 1;
        }
    } else {
        *pos -= 1;
    }
}

/// 向右移动一个字符。
pub fn move_mbright(data: &[u8], pos: &mut usize) {
    if *pos >= data.len() {
        return;
    }
    if using_utf8() {
        let len = utf8_char_len(data[*pos]);
        *pos = (*pos + len).min(data.len());
    } else {
        *pos += 1;
    }
}

/// 判断给定字符是否为字母或数字。
pub fn is_alnum_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    let wc = mbtowide(c);
    match wc {
        Some(w) => char::from_u32(w).map_or(false, |ch| ch.is_alphanumeric()),
        None => c[0].is_ascii_alphanumeric(),
    }
}

/// 判断给定字符是否为空白字符。
pub fn is_blank_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    if c[0] as i8 >= 0 {
        c[0] == b' ' || c[0] == b'\t'
    } else if using_utf8() {
        if let Some(wc) = mbtowide(c) {
            char::from_u32(wc).map_or(false, |ch| ch.is_whitespace())
        } else {
            false
        }
    } else {
        false
    }
}

/// 判断给定字符是否为标点或控制字符。
pub fn is_punct_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    if c[0] as i8 >= 0 {
        c[0].is_ascii_punctuation()
    } else {
        !is_alnum_char(c)
    }
}

/// 判断给定字符是否控制字符（非单词字符）。
pub fn is_control_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return true;
    }
    if c[0] as i8 >= 0 {
        c[0] < 32 || c[0] == 0x7F
    } else {
        false
    }
}

/// 计算给定列位置对应的字节位置。
pub fn actual_x_from_col(data: &[u8], col: usize) -> usize {
    let mut pos = 0;
    let mut cur_col = 0;
    while pos < data.len() && cur_col < col {
        if data[pos] == b'\t' {
            let tab = tabsize();
            cur_col = (cur_col / tab + 1) * tab;
        } else {
            cur_col += 1;
        }
        pos += 1;
    }
    pos.min(data.len())
}

/// 获取字符的显示宽度（列数，制表符按 tab 宽度计算）。
pub fn char_width(data: &[u8], pos: usize) -> usize {
    if data.is_empty() || pos >= data.len() {
        return 0;
    }
    if data[pos] == b'\t' {
        return tabsize();
    }
    if using_utf8() && (data[pos] as i8) < 0 {
        // 多字节字符宽度
        if let Some(wc) = mbtowide(&data[pos..]) {
            char::from_u32(wc).map_or(1, |c| unicode_width(c))
        } else {
            1
        }
    } else {
        1
    }
}

/// 简单的 Unicode 字符宽度计算（仅支持 CJK 等宽字符）。
fn unicode_width(c: char) -> usize {
    // 简单实现：只判断 CJK 统一表意文字
    let code = c as u32;
    if (0x1100..=0x115F).contains(&code)
        || (0x2E80..=0x303E).contains(&code)
        || (0x3040..=0x33FF).contains(&code)
        || (0x3400..=0x4DBF).contains(&code)
        || (0x4E00..=0x9FFF).contains(&code)
        || (0xA000..=0xA4FF).contains(&code)
        || (0xAC00..=0xD7AF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0xFE30..=0xFE6F).contains(&code)
        || (0xFF01..=0xFF60).contains(&code)
        || (0x20000..=0x2FFFF).contains(&code)
        || (0x30000..=0x3FFFF).contains(&code)
    {
        2
    } else {
        1
    }
}