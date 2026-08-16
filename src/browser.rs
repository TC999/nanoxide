/**************************************************************************
 * browser.rs  --  GNU nano 文件浏览器（对应 browser.c）
 * 版权 (C) 2003-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 文件浏览器。对应原版 nano 的 `browser.c`。

use crate::definitions::*;
use crate::global;
use crate::utils;
use crate::files;
use std::fs;
use std::path::Path;

/// 打开文件浏览器。
pub fn do_browser() {
    // 简化：只显示当前目录
    let path = ".";
    browse_directory(path);
}

/// 浏览目录。
pub fn browse_directory(path: &str) -> Vec<String> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(path) {
        for entry in read_dir.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                entries.push(name);
            }
        }
    }
    entries.sort();
    entries
}

/// 进入目录。
pub fn do_browser_enter() {
    // 简化
}

/// 浏览器中上移。
pub fn do_browser_up() {
    // 简化
}

/// 浏览器中下移。
pub fn do_browser_down() {
    // 简化
}

/// 跳转到目录。
pub fn do_goto_dir() {
    // 简化
}

/// 在文件浏览器中搜索文件。
pub fn do_where_is_file() {
    // 简化
}

/// 获取浏览器的当前选择。
pub fn get_browser_selection() -> Option<String> {
    None
}

/// 设置浏览器路径。
pub fn set_browser_path(path: &str) {
    with_global_mut(|g| {
        g.present_path = Some(path.to_string());
    });
}

/// 获取浏览器路径。
pub fn get_browser_path() -> Option<String> {
    with_global(|g| g.present_path.clone())
}

/// 在新缓冲区中打开文件。
pub fn open_file_in_buffer(filename: &str) -> bool {
    files::open_buffer(filename)
}