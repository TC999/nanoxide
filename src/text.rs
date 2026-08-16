/**************************************************************************
 * text.rs  --  GNU nano 文本编辑操作（对应 text.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 文本编辑操作：插入、删除、换行、缩进、撤销等。对应原版 nano 的 `text.c`。

use crate::definitions::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::chars;

/// 初始化终端。
pub fn terminal_init() {
    // 使用 crossterm 初始化终端
    let _ = crossterm::terminal::enable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
}

/// 恢复终端状态。
pub fn endwin() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::LeaveAlternateScreen);
    let _ = crossterm::terminal::disable_raw_mode();
}

/// 在回答末尾放置光标。
pub fn put_cursor_at_end_of_answer() {
    // 简化
}

/// 插入字符。
pub fn do_enter() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let pos = of_ref.current_x;
                let right_part = data.data[pos..].to_string();
                data.data.truncate(pos);
                drop(data);

                // 创建新行
                let new_line = Rc::new(RefCell::new(LineStruct {
                    data: right_part,
                    lineno: cur.borrow().lineno + 1,
                    next: cur.borrow().next.clone(),
                    prev: Some(Rc::downgrade(&cur)),
                    multidata: None,
                    has_anchor: false,
                }));

                // 更新链表
                let next = cur.borrow().next.clone();
                if let Some(n) = next {
                    n.borrow_mut().prev = Some(Rc::downgrade(&new_line));
                }
                cur.borrow_mut().next = Some(new_line.clone());

                // 更新行号
                let mut renumber = new_line.clone();
                let mut lineno = renumber.borrow().lineno;
                loop {
                    renumber.borrow_mut().lineno = lineno;
                    let next = renumber.borrow().next.clone();
                    match next {
                        Some(n) => {
                            renumber = n;
                            lineno += 1;
                        }
                        None => break,
                    }
                }

                // 更新 filebot
                if Rc::ptr_eq(&cur, of_ref.filebot.as_ref().unwrap()) {
                    of_ref.filebot = Some(new_line.clone());
                }

                of_ref.current = Some(new_line);
                of_ref.current_x = 0;
                of_ref.placewewant = 0;
                of_ref.modified = true;
            }
        }
    });
}

/// 插入制表符。
pub fn do_tab() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let pos = of_ref.current_x;
                if pos <= data.data.len() {
                    if ISSET(TABS_TO_SPACES) {
                        let tab = chars::tabsize();
                        let spaces = tab - (of_ref.placewewant % tab);
                        for _ in 0..spaces {
                            data.data.insert(pos, ' ');
                        }
                        of_ref.current_x = pos + spaces;
                    } else {
                        data.data.insert(pos, '\t');
                        of_ref.current_x = pos + 1;
                    }
                    of_ref.modified = true;
                }
            }
        }
    });
}

/// 注入文本（用于宏回放）。
pub fn inject(_text: &str) {
    // 简化
}

/// 取消操作。
pub fn do_cancel() {
    // 取消当前操作
}

/// 退出编辑器。
pub fn do_exit() {
    with_global_mut(|g| {
        g.we_are_running = false;
    });
}

/// 刷新屏幕。
pub fn do_refresh() {
    with_global_mut(|g| {
        g.refresh_needed = true;
    });
}

/// 撤销操作。
pub fn do_undo() {
    // 简化
}

/// 重做操作。
pub fn do_redo() {
    // 简化
}

/// 缩进当前行。
pub fn do_indent() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                data.data.insert_str(0, "    ");
                of_ref.current_x += 4;
                of_ref.modified = true;
            }
        }
    });
}

/// 取消缩进。
pub fn do_unindent() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let indent = data.data.len().min(4);
                data.data.drain(..indent);
                of_ref.current_x = of_ref.current_x.saturating_sub(indent);
                of_ref.modified = true;
            }
        }
    });
}

/// 注释/取消注释行。
pub fn do_comment() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                data.data.insert_str(0, "# ");
                of_ref.current_x += 2;
                of_ref.modified = true;
            }
        }
    });
}

/// 取消注释。
pub fn do_uncomment() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                if data.data.starts_with("# ") {
                    data.data.drain(..2);
                    of_ref.current_x = of_ref.current_x.saturating_sub(2);
                } else if data.data.starts_with('#') {
                    data.data.drain(..1);
                    of_ref.current_x = of_ref.current_x.saturating_sub(1);
                }
                of_ref.modified = true;
            }
        }
    });
}

/// 挂起编辑器。
pub fn do_suspend() {
    // 在 Windows 上不支持挂起
}

/// 拼写检查。
pub fn do_spell() {
    // 简化
}

/// 格式化。
pub fn do_formatter() {
    // 简化
}

/// 清除所有剪贴板。
pub fn zap_all_cutbuffer() {
    // 简化
}

/// 标记当前位置。
pub fn do_mark() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            // 切换标记
            if of_ref.mark.is_some() {
                of_ref.mark = None;
            } else {
                of_ref.mark = of_ref.current.clone();
                of_ref.mark_x = of_ref.current_x;
            }
        }
    });
}

/// 在当前位置设置锚点。
pub fn do_anchor() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            if let Some(cur) = &of_ref.current {
                let mut data = cur.borrow_mut();
                data.has_anchor = !data.has_anchor;
            }
        }
    });
}

/// 复数形式辅助函数。
pub fn P_<'a>(singular: &'a str, _plural: &'a str, number: usize) -> &'a str {
    if number == 1 { singular } else { singular }
}

/// 插入换行（自动换行时调用）。
pub fn do_wrap(_line: Option<LineRef>) {
    // 简化
}

/// 向缓冲区插入字符。
pub fn insert_char(ch: char) {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let pos = of_ref.current_x;
                if pos <= data.data.len() {
                    data.data.insert(pos, ch);
                    of_ref.current_x = pos + 1;
                    of_ref.modified = true;
                }
            }
        }
    });
}