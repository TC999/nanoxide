/**************************************************************************
 * cut.rs  --  GNU nano 剪切/删除操作（对应 cut.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 剪切、删除、复制、粘贴操作。对应原版 nano 的 `cut.c`。

use crate::definitions::*;
use crate::movement;

/// 是否从光标处剪切（而非整行）。
fn cut_from_cursor() -> bool {
    ISSET(CUT_FROM_CURSOR)
}

/// 删除当前光标下的字符。
pub fn do_delete() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                if of_ref.current_x < data.data.len() {
                    let char_len = 1; // 简化：删除一个字节
                    let end = (of_ref.current_x + char_len).min(data.data.len());
                    data.data.drain(of_ref.current_x..end);
                    of_ref.totsize = of_ref.totsize.saturating_sub(1);
                    of_ref.modified = true;
                }
            }
        }
    });
}

/// 退格删除。
pub fn do_backspace() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            if of_ref.current_x > 0 {
                let current = of_ref.current.clone();
                if let Some(cur) = current {
                    let mut data = cur.borrow_mut();
                    let char_len = 1; // 简化
                    let start = of_ref.current_x.saturating_sub(char_len);
                    data.data.drain(start..of_ref.current_x);
                    of_ref.current_x = start;
                    of_ref.totsize = of_ref.totsize.saturating_sub(1);
                    of_ref.modified = true;
                }
            }
        }
    });
}

/// 剪切当前行或选中文本。
pub fn do_cut() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                if cut_from_cursor() {
                    // 从光标剪切到行尾
                    let start = of_ref.current_x;
                    let removed = data.data[start..].to_string();
                    data.data.truncate(start);
                    of_ref.totsize = of_ref.totsize.saturating_sub(removed.len());
                } else {
                    // 剪切整行
                    let removed = data.data.clone();
                    data.data.clear();
                    of_ref.totsize = of_ref.totsize.saturating_sub(removed.len());
                    of_ref.current_x = 0;
                }
                of_ref.modified = true;
            }
        }
    });
}

/// 剪切到文件末尾。
pub fn do_cut_to_eof() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let start = of_ref.current_x;
                data.data.truncate(start);
                of_ref.totsize = 0; // 简化
                of_ref.modified = true;
            }
        }
    });
}

/// 粘贴文本。
pub fn do_paste() {
    // 简化的粘贴操作
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let pos = of_ref.current_x;
                // 插入一个占位文本
                data.data.insert(pos, ' ');
                of_ref.current_x = pos + 1;
                of_ref.totsize += 1;
                of_ref.modified = true;
            }
        }
    });
}

/// 复制文本。
pub fn do_copy() {
    // 复制操作——在 nano 中通常与剪切配合
}

/// 删除整行。
pub fn do_cut_previous() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                data.data.clear();
                of_ref.current_x = 0;
                of_ref.modified = true;
            }
        }
    });
}

/// 从光标处剪切到行尾。
pub fn cut_to_right() {
    do_cut();
}

/// 从行首剪切到光标处。
pub fn cut_to_left() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let current = of_ref.current.clone();
            if let Some(cur) = current {
                let mut data = cur.borrow_mut();
                let end = of_ref.current_x;
                data.data.drain(..end);
                of_ref.current_x = 0;
                of_ref.totsize = of_ref.totsize.saturating_sub(end);
                of_ref.modified = true;
            }
        }
    });
}

/// 将给定行插入到剪切缓冲区。
pub fn copy_from(_line: Option<LineRef>) {
    // 简化实现
}

/// 将整行添加到剪切缓冲区。
pub fn add_to_cutbuffer(_line: Option<LineRef>, _data: &str) {
    // 简化实现
}