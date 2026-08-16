/**************************************************************************
 * history.rs  --  GNU nano 搜索/替换历史记录（对应 history.c）
 * 版权 (C) 2003-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 历史记录管理。对应原版 nano 的 `history.c`。

use crate::definitions::*;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// 获取历史文件路径。
fn history_file() -> PathBuf {
    with_global(|g| {
        g.homedir.clone().map(|h| {
            let mut p = PathBuf::from(&h);
            p.push(".nano");
            p.push("search_history");
            p
        })
    }).unwrap_or_else(|| PathBuf::from(".nano_search_history"))
}

/// 初始化历史记录系统。
pub fn history_init() {
    // 创建目录
    with_global(|g| {
        if let Some(ref home) = g.homedir {
            let mut dir = PathBuf::from(home);
            dir.push(".nano");
            let _ = fs::create_dir_all(&dir);
        }
    });
}

/// 从文件加载历史记录。
pub fn load_history() {
    let path = history_file();
    if let Ok(file) = fs::File::open(&path) {
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        with_global_mut(|g| {
            g.search_history = lines;
        });
    }
}

/// 保存历史记录到文件。
pub fn save_history() {
    let path = history_file();
    with_global(|g| {
        if let Ok(mut file) = fs::File::create(&path) {
            for entry in &g.search_history {
                let _ = writeln!(file, "{}", entry);
            }
        }
    });
}

/// 添加搜索历史条目。
pub fn add_search_history_entry(entry: &str) {
    with_global_mut(|g| {
        // 去重
        if let Some(pos) = g.search_history.iter().position(|e| e == entry) {
            g.search_history.remove(pos);
        }
        g.search_history.push(entry.to_string());
        // 限制数量
        while g.search_history.len() > MAX_SEARCH_HISTORY {
            g.search_history.remove(0);
        }
    });
}

/// 获取搜索历史。
pub fn get_search_history() -> Vec<String> {
    with_global(|g| g.search_history.clone())
}

/// 添加替换历史条目。
pub fn add_replace_history_entry(entry: &str) {
    with_global_mut(|g| {
        if let Some(pos) = g.replace_history.iter().position(|e| e == entry) {
            g.replace_history.remove(pos);
        }
        g.replace_history.push(entry.to_string());
        while g.replace_history.len() > MAX_SEARCH_HISTORY {
            g.replace_history.remove(0);
        }
    });
}

/// 获取替换历史。
pub fn get_replace_history() -> Vec<String> {
    with_global(|g| g.replace_history.clone())
}

/// 加载位置记录。
pub fn load_positions_register() {
    // 位置记录功能简化
}

/// 保存位置记录。
pub fn save_positions_register() {
    // 简化
}

/// 跳转到指定行和列。
pub fn goto_line_and_column(line: isize, column: isize) {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            // 跳转到指定行
            let mut current = of_ref.filetop.clone();
            let mut lineno = 1;
            while let Some(ref c) = current {
                if lineno >= line {
                    break;
                }
                let next = c.borrow().next.clone();
                current = next;
                lineno += 1;
            }
            if let Some(c) = current {
                of_ref.current = Some(c);
                if column > 0 {
                    of_ref.current_x = column as usize;
                } else {
                    of_ref.current_x = 0;
                }
            }
        }
    });
}

/// 获取文件名历史记录。
pub fn get_filename_history() -> Vec<String> {
    Vec::new()
}