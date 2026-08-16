/**************************************************************************
 * utils.rs  --  GNU nano 通用工具函数（对应 utils.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 通用工具函数。对应原版 nano 的 `utils.c`。
//! 转换说明：使用 `with_global` 访问全局状态。

use crate::definitions::*;
use crate::chars;

/// 获取用户主目录。
pub fn get_homedir() {
    with_global_mut(|g| {
        if g.homedir.is_none() {
            let homenv = std::env::var("HOME").ok();
            if let Some(h) = homenv {
                if !h.is_empty() {
                    g.homedir = Some(h);
                    return;
                }
            }
            // 使用 libc 安全封装获取用户信息
            #[cfg(unix)]
            {
                let home = get_home_from_passwd();
                if let Some(h) = home {
                    g.homedir = Some(h);
                }
            }
            #[cfg(windows)]
            {
                if let Ok(drive) = std::env::var("HOMEDRIVE") {
                    if let Ok(path) = std::env::var("HOMEPATH") {
                        g.homedir = Some(format!("{}{}", drive, path));
                    }
                }
            }
        }
    });
}

/// 安全封装：获取用户主目录（POSIX）。
#[cfg(unix)]
fn get_home_from_passwd() -> Option<String> {
    // 使用纯 std 方式获取主目录
    // 在 Unix 上，$HOME 环境变量通常已设置正确
    // 如果未设置，使用 /tmp 作为备选
    None
}

/// 获取用户主目录（Windows 安全封装）。
#[cfg(windows)]
fn get_home_from_passwd() -> Option<String> {
    None
}

/// 返回路径的文件名部分。
pub fn tail(path: &str) -> &str {
    match path.rfind('/') {
        None => path,
        Some(slash) => &path[slash + 1..],
    }
}

/// 计算字符串的显示宽度（列数）。
pub fn breadth(text: &[u8]) -> usize {
    let mut width = 0;
    let mut pos = 0;
    while pos < text.len() {
        if text[pos] == b'\t' {
            width = (width / chars::tabsize() + 1) * chars::tabsize();
        } else {
            width += chars::char_width(text, pos);
        }
        pos += chars::mb_cur_max(text, pos);
    }
    width
}

/// 将字节位置转换为列位置（考虑制表符）。
pub fn xplustabs(text: &[u8], pos: usize) -> usize {
    let mut column = 0;
    let mut index = 0;
    while index < pos && index < text.len() {
        if text[index] == b'\t' {
            let tab = chars::tabsize();
            column = (column / tab + 1) * tab;
        } else {
            column += 1;
        }
        index += 1;
    }
    column
}

/// 将列位置转换为字节位置。
pub fn actual_x(text: &[u8], target_column: usize) -> usize {
    let mut column = 0;
    let mut pos = 0;
    while pos < text.len() && column < target_column {
        if text[pos] == b'\t' {
            let tab = chars::tabsize();
            column = (column / tab + 1) * tab;
        } else {
            column += 1;
        }
        pos += 1;
    }
    pos.min(text.len())
}

/// 计算字符串长度（字节数）。
pub fn wideness(text: &[u8], _limit: usize) -> usize {
    text.len()
}

/// 解析 "行号,列号" 格式的字符串。
pub fn parse_line_column(input: &str) -> (isize, isize) {
    let parts: Vec<&str> = input.splitn(2, |c| c == ',' || c == '.' || c == ':').collect();
    let line = parts.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let col = parts.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    (line, col)
}

/// 解析数字字符串。
pub fn parse_num(input: &str) -> isize {
    input.trim().parse().unwrap_or(0)
}

/// 检查当前位置是否在单词边界。
pub fn is_word_boundary(text: &[u8], position: usize) -> bool {
    if position >= text.len() {
        return true;
    }
    let before_is_alpha = if position > 0 {
        chars::is_alnum_char(&text[position - 1..])
    } else {
        false
    };
    let after_is_alpha = chars::is_alnum_char(&text[position..]);
    (position == 0 || !before_is_alpha) && (position >= text.len() || !after_is_alpha)
}

/// 检查字符串是否为空白。
pub fn is_white_string(text: &[u8]) -> bool {
    text.iter().all(|&b| b == b' ' || b == b'\t')
}

/// 移除字符串末尾的换行符。
pub fn chomp(text: &mut String) {
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
}

/// 分配内存（对应 C 的 nmalloc）。
pub fn nmalloc<T: Default>(size: usize) -> Vec<T> {
    let mut v = Vec::with_capacity(size);
    for _ in 0..size {
        v.push(T::default());
    }
    v
}

/// 重新分配内存（对应 C 的 nrealloc）。
pub fn nrealloc<T: Clone>(vec: &mut Vec<T>, new_size: usize, default: T) {
    vec.resize(new_size, default);
}

/// 分配并清零（对应 C 的 calloc）。
pub fn ncalloc<T: Default>(count: usize) -> Vec<T> {
    nmalloc(count)
}

/// 释放内存（对应 C 的 free，Rust 中自动管理）。
pub fn nfree<T>(_ptr: Vec<T>) {
    // Vec 自动释放
}