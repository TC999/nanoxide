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

// ======================== 跳转到指定行列（对应 search.c 的 goto_line_and_column） ========================

/// 转到指定的行和列（注意两者都是从 1 开始计数的）
/// （对应 `goto_line_and_column`）。
pub fn goto_line_and_column(mut line: isize, mut column: isize, hugfloor: bool) {
    /* 负行号表示：从文件末尾倒数。 */
    let mut tail_data: Option<(LineRef, isize, isize)> = None;

    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let mut of = of.borrow_mut();

        let filebot_lineno = of.filebot.as_ref().map(|b| b.borrow().lineno).unwrap_or(1);
        let current_lineno = of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(1);

        if line < 0 {
            line = filebot_lineno + line + 1;
        } else if line == 0 {
            line = current_lineno;
        }
        if line < 1 {
            line = 1;
        }

        /* 若目标行在视口之外，需要重算颜色。 */
        if let (Some(et), Some(cur)) = (&of.edittop, &of.current) {
            let et_lineno = et.borrow().lineno;
            let cur_lineno = cur.borrow().lineno;
            if line > et_lineno + g.editwinrows as isize
                || (g.flags.isset(SOFTWRAP) && line > cur_lineno)
            {
                g.recook |= g.perturbed;
            }
        }

        /* 迭代到请求的行。 */
        let mut current = of.filetop.clone().unwrap();
        let mut remaining = line;
        loop {
            let is_filebot = of.filebot.as_ref().map(|b| std::rc::Rc::ptr_eq(&current, b)).unwrap_or(false);
            if remaining <= 1 || is_filebot {
                break;
            }
            let next = { let r = current.borrow(); r.next.clone() }.unwrap();
            current = next;
            remaining -= 1;
        }
        of.current = Some(current.clone());

        /* 负列号表示：从行末倒数。 */
        let data = current.borrow().data.clone();
        let line_breadth = utils::breadth(data.as_bytes()) as isize;
        if column < 0 {
            column = line_breadth + column + 2;
        } else if column == 0 {
            column = of.placewewant as isize + 1;
        }
        if column < 1 {
            column = 1;
        }

        /* 设置与请求列对应的 x 位置。 */
        of.current_x = utils::actual_x(data.as_bytes(), column as usize - 1);
        of.placewewant = column as usize - 1;

        if g.flags.isset(SOFTWRAP) && of.placewewant / g.editwincols
            > line_breadth as usize / g.editwincols
        {
            of.placewewant = line_breadth as usize;
        }

        if hugfloor {
            tail_data = Some((of.current.clone().unwrap(), filebot_lineno, current_lineno));
        }
    });

    if !hugfloor {
        return;
    }

    /* 注意：以下计算在闭包外执行，因为 leftedge_for/go_forward_chunks
     * 会再次访问全局状态。 */
    let (current, filebot_lineno, current_lineno) = match tail_data {
        Some(t) => t,
        None => return,
    };

    let rows_from_tail = if ISSET(SOFTWRAP) {
        let mut currentline = current;
        let mut leftedge = crate::winio::leftedge_for(utils::xplustabs(), &currentline);
        let rows = with_global(|g| g.editwinrows) / 2;
        rows - crate::winio::go_forward_chunks(rows, &mut currentline, &mut leftedge)
    } else {
        (filebot_lineno - current_lineno) as i32
    };

    let half = with_global(|g| g.editwinrows) / 2;
    let jumpy = ISSET(JUMPY_SCROLLING);

    /* 若目标行接近文件尾部，把最后一行或块放在屏幕底行；
     * 否则，将目标行居中。 */
    if rows_from_tail < half && !jumpy {
        with_global_mut(|g| {
            let of = g.openfile.as_ref().expect("no open file").clone();
            let mut of = of.borrow_mut();
            of.cursor_row = (g.editwinrows - 1 - rows_from_tail) as isize;
        });
        crate::winio::adjust_viewport(UpdateType::Stationary);
    } else {
        crate::winio::adjust_viewport(UpdateType::Centering);
    }
}