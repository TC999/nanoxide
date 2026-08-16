/**************************************************************************
 * movement.rs  --  GNU nano 移动操作（对应 move.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 光标移动函数。对应原版 nano 的 `move.c`。
//! 转换说明：使用安全全局状态 `GLOBAL` 替代 `static mut`。

use crate::definitions::*;
use std::rc::Rc;
use crate::chars::{mb_cur_max};
use crate::utils::{actual_x, wideness};

/// 获取编辑窗口行数。
fn editwinrows() -> i32 {
    with_global(|g| g.editwinrows)
}

/// 将光标向左移动一个字符。
pub fn do_left() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            if of_ref.current_x > 0 {
                let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
                let char_len = mb_cur_max(data.as_bytes(), of_ref.current_x);
                of_ref.current_x = if char_len > 0 { of_ref.current_x - 1 } else { of_ref.current_x.saturating_sub(1) };
                of_ref.placewewant = wideness(data.as_bytes(), of_ref.current_x);
            }
        }
    });
}

/// 将光标向右移动一个字符。
pub fn do_right() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
            let data_len = data.len();
            if of_ref.current_x < data_len {
                let char_len = 1.max(1); // 简化：移动一个字节
                of_ref.current_x = (of_ref.current_x + 1).min(data_len);
                of_ref.placewewant = wideness(data.as_bytes(), of_ref.current_x);
            }
        }
    });
}

/// 将光标上移一行。
pub fn do_up() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let prev = cur.borrow().prev.clone();
                if let Some(p) = prev.and_then(|w| w.upgrade()) {
                    of_ref.current = Some(p.clone());
                    let data = p.borrow().data.clone();
                    let target = of_ref.placewewant;
                    of_ref.current_x = actual_x(data.as_bytes(), target);
                    of_ref.cursor_row -= 1;
                }
            }
        }
    });
}

/// 将光标下移一行。
pub fn do_down() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let next = cur.borrow().next.clone();
                if let Some(n) = next {
                    // 确保不超过 filebot
                    let is_filebot = of_ref.filebot.as_ref().map(|fb| Rc::ptr_eq(&n, fb)).unwrap_or(false);
                    if !is_filebot || n.borrow().data.is_empty() {
                        of_ref.current = Some(n.clone());
                        let data = n.borrow().data.clone();
                        let target = of_ref.placewewant;
                        of_ref.current_x = actual_x(data.as_bytes(), target);
                        of_ref.cursor_row += 1;
                    }
                }
            }
        }
    });
}

/// 移动光标到行首。
pub fn do_home() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
            of_ref.current_x = 0;
            of_ref.placewewant = 0;
        }
    });
}

/// 移动光标到行尾。
pub fn do_end() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
            of_ref.current_x = data.len();
            of_ref.placewewant = wideness(data.as_bytes(), of_ref.current_x);
        }
    });
}

/// 上翻一页。
pub fn do_page_up() {
    let rows = editwinrows();
    for _ in 0..rows.max(1) {
        do_up();
    }
}

/// 下翻一页。
pub fn do_page_down() {
    let rows = editwinrows();
    for _ in 0..rows.max(1) {
        do_down();
    }
}

/// 移动到第一行。
pub fn do_first_line() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            if let Some(top) = of_ref.filetop.clone() {
                of_ref.current = Some(top);
                let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
                of_ref.current_x = 0;
                of_ref.placewewant = 0;
                of_ref.cursor_row = 0;
            }
        }
    });
}

/// 移动到最后一行。
pub fn do_last_line() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            if let Some(bot) = of_ref.filebot.clone() {
                let data = bot.borrow().data.clone();
                of_ref.current = Some(bot);
                of_ref.current_x = 0;
                of_ref.placewewant = 0;
            }
        }
    });
}

/// 移动到上一个单词。
pub fn do_prev_word() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
            let bytes = data.as_bytes();
            let mut pos = of_ref.current_x;
            // 跳过当前单词前的空白
            while pos > 0 && (bytes[pos - 1] == b' ' || bytes[pos - 1] == b'\t') {
                pos -= 1;
            }
            // 跳过单词字符
            while pos > 0 && bytes[pos - 1] != b' ' && bytes[pos - 1] != b'\t' {
                pos -= 1;
            }
            of_ref.current_x = pos;
            of_ref.placewewant = wideness(bytes, pos);
        }
    });
}

/// 移动到下一个单词。
pub fn do_next_word() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let data = of_ref.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
            let bytes = data.as_bytes();
            let mut pos = of_ref.current_x;
            // 跳过空白
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            // 跳过单词字符
            while pos < bytes.len() && bytes[pos] != b' ' && bytes[pos] != b'\t' {
                pos += 1;
            }
            of_ref.current_x = pos;
            of_ref.placewewant = wideness(bytes, pos);
        }
    });
}

/// 滚动上移一行。
pub fn do_scroll_up() {
    do_up();
}

/// 滚动下移一行。
pub fn do_scroll_down() {
    do_down();
}

/// 移动到段落开头。
pub fn to_para_begin() {
    do_home();
}

/// 移动到段落结尾。
pub fn to_para_end() {
    do_end();
}

/// 移动到上一个块。
pub fn to_prev_block() {
    do_page_up();
}

/// 移动到下一个块。
pub fn to_next_block() {
    do_page_down();
}