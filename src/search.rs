/**************************************************************************
 * search.rs  --  GNU nano 搜索/替换功能（对应 search.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 搜索和替换操作。对应原版 nano 的 `search.c`。
//! 转换说明：使用 `MatchPattern` 替代 POSIX regex。

use crate::definitions::*;
use crate::chars;
use crate::utils;
use crate::history;
use crate::global;

/// 初始化正则表达式（模式匹配）。
pub fn regexp_init(pattern: &str) -> bool {
    with_global_mut(|g| {
        let case_sensitive = g.flags.isset(CASE_SENSITIVE);
        let pat = if pattern.contains('*') || pattern.contains('?') {
            MatchPattern::from_glob(pattern)
        } else {
            MatchPattern::from_literal(pattern)
        };
        g.search_regexp = Some(pat);
        g.regexp_nsub = 0;
    });
    true
}

/// 释放正则表达式。
pub fn regexp_cleanup() {
    with_global_mut(|g| {
        g.search_regexp = None;
        g.regexp_nsub = 0;
    });
}

/// 查找下一个匹配。
pub fn find_next_match(needle: &str, start_line: Option<LineRef>, start_x: usize, backwards: bool) -> Option<(LineRef, usize)> {
    with_global(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let current = start_line.or_else(|| of_ref.current.clone())?;
            let mut line = current.clone();
            let mut pos = start_x;

            if backwards {
                // 向后搜索（从当前位置向前）
                loop {
                    let data = line.borrow().data.clone();
                    if !needle.is_empty() {
                        if let Some(found) = if g.flags.isset(CASE_SENSITIVE) {
                            data[..pos].rfind(needle)
                        } else {
                            let lower = data[..pos].to_lowercase();
                            let needle_lower = needle.to_lowercase();
                            lower.rfind(&needle_lower)
                        } {
                            return Some((line.clone(), found));
                        }
                    }
                    // 移动到上一行
                    let prev = line.borrow().prev.clone().and_then(|w| w.upgrade());
                    match prev {
                        Some(p) => {
                            line = p;
                            pos = line.borrow().data.len();
                        }
                        None => break,
                    }
                }
            } else {
                // 向前搜索
                loop {
                    let data = line.borrow().data.clone();
                    if !needle.is_empty() {
                        if let Some(found) = if g.flags.isset(CASE_SENSITIVE) {
                            data[pos..].find(needle)
                        } else {
                            let lower = data[pos..].to_lowercase();
                            let needle_lower = needle.to_lowercase();
                            lower.find(&needle_lower)
                        } {
                            return Some((line.clone(), pos + found));
                        }
                    }
                    // 移动到下一行
                    let next = line.borrow().next.clone();
                    match next {
                        Some(n) => {
                            line = n;
                            pos = 0;
                        }
                        None => break,
                    }
                }
            }
        }
        None
    });
    None
}

/// 执行搜索（向前）。
pub fn do_search_forward() {
    with_global_mut(|g| {
        let needle = g.last_search.clone().unwrap_or_default();
        if needle.is_empty() {
            return;
        }
        if let Some((found_line, found_x)) = find_next_match(&needle, None, 0, false) {
            if let Some(of) = &g.openfile {
                let mut of_ref = of.borrow_mut();
                of_ref.current = Some(found_line);
                of_ref.current_x = found_x;
                g.didfind = 1;
            }
        } else {
            g.didfind = 0;
            set_statusbar_message("Not found");
        }
    });
}

/// 执行搜索（向后）。
pub fn do_search_backward() {
    with_global_mut(|g| {
        let needle = g.last_search.clone().unwrap_or_default();
        if needle.is_empty() {
            return;
        }
        let current = g.openfile.as_ref().and_then(|of| of.borrow().current.clone());
        let current_x = g.openfile.as_ref().map(|of| of.borrow().current_x).unwrap_or(0);
        if let Some((found_line, found_x)) = find_next_match(&needle, current, current_x, true) {
            if let Some(of) = &g.openfile {
                let mut of_ref = of.borrow_mut();
                of_ref.current = Some(found_line);
                of_ref.current_x = found_x;
                g.didfind = 1;
            }
        } else {
            g.didfind = 0;
            set_statusbar_message("Not found");
        }
    });
}

/// 查找下一个。
pub fn do_find_next() {
    do_search_forward();
}

/// 查找上一个。
pub fn do_find_previous() {
    do_search_backward();
}

/// 执行替换。
pub fn do_replace() {
    // 简化：提示用户输入搜索和替换字符串
    set_statusbar_message("Replace (not fully implemented)");
}

/// 替换所有匹配。
pub fn replace_all(needle: &str, replacement: &str) -> usize {
    with_global_mut(|g| {
        let mut count = 0;
        if let Some(of) = &g.openfile {
            let mut current = of.borrow().filetop.clone();
            while let Some(c) = current {
                let mut data = c.borrow_mut();
                if !needle.is_empty() {
                    let mut pos = 0;
                    while let Some(found) = if g.flags.isset(CASE_SENSITIVE) {
                        data.data[pos..].find(needle)
                    } else {
                        let lower = data.data[pos..].to_lowercase();
                        let needle_lower = needle.to_lowercase();
                        lower.find(&needle_lower)
                    } {
                        let start = pos + found;
                        let end = start + needle.len();
                        data.data.replace_range(start..end, replacement);
                        count += 1;
                        pos = start + replacement.len();
                    }
                }
                let next = c.borrow().next.clone();
                current = next;
            }
            of.borrow_mut().modified = count > 0;
        }
        count
    })
}

/// 设置状态栏消息。
fn set_statusbar_message(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
    });
}

/// 查找下一个匹配位置（用于高亮）。
pub fn find_next_match_highlight(needle: &str, from_line: Option<LineRef>) -> Option<(LineRef, usize, usize)> {
    with_global(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let start = from_line.or_else(|| of_ref.current.clone())?;
            let mut line = start;
            loop {
                let data = line.borrow().data.clone();
                if !needle.is_empty() {
                    let found = if g.flags.isset(CASE_SENSITIVE) {
                        data.find(needle)
                    } else {
                        data.to_lowercase().find(&needle.to_lowercase())
                    };
                    if let Some(pos) = found {
                        return Some((line.clone(), pos, pos + needle.len()));
                    }
                }
                let next = line.borrow().next.clone();
                match next {
                    Some(n) => line = n,
                    None => break,
                }
            }
        }
        None
    });
    None
}