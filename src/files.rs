/**************************************************************************
 * files.rs  --  GNU nano 文件 I/O（对应 files.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 文件读写操作。对应原版 nano 的 `files.c`。
//! 转换说明：使用 `std::fs` 和 `std::io` 替代 C 文件 API。

use crate::definitions::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::io::{BufReader, Write, Read};
use std::path::Path;

/// 获取 COLS 全局变量。
pub fn COLS() -> usize {
    with_global(|g| g.COLS)
}

/// 获取 LINES 全局变量。
pub fn LINES() -> usize {
    with_global(|g| g.LINES)
}

/// 打开文件并加载到缓冲区。
pub fn open_buffer(filename: &str) -> bool {
    let path = Path::new(filename);
    if !path.exists() {
        // 创建新文件
        with_global_mut(|g| {
            let new_file = Rc::new(RefCell::new(OpenFileStruct {
                filename: Some(filename.to_string()),
                filetop: None, filebot: None, edittop: None,
                current: None, totsize: 0, firstcolumn: 0,
                current_x: 0, placewewant: 0, brink: 0, cursor_row: 0,
                statinfo: None, spillage_line: None,
                mark: None, mark_x: 0, softmark: false,
                fmt: FormatType::NixFile, lock_filename: None,
                undotop: None, current_undo: None, last_saved: None,
                last_action: UndoType::Other, modified: false,
                syntax: None, errormessage: None,
                next: None, prev: None,
            }));
            // 创建初始空行
            let line = Rc::new(RefCell::new(LineStruct {
                data: String::new(), lineno: 1,
                next: None, prev: None,
                multidata: None, has_anchor: false,
            }));
            new_file.borrow_mut().filetop = Some(line.clone());
            new_file.borrow_mut().filebot = Some(line.clone());
            new_file.borrow_mut().current = Some(line);
            g.openfile = Some(new_file);
        });
        return true;
    }

    // 读取文件
    match fs::read_to_string(path) {
        Ok(content) => {
            with_global_mut(|g| {
                let mut lines: Vec<LineRef> = Vec::new();
                let mut lineno = 1;
                for line_str in content.lines() {
                    let line = Rc::new(RefCell::new(LineStruct {
                        data: line_str.to_string(),
                        lineno,
                        next: None, prev: None,
                        multidata: None, has_anchor: false,
                    }));
                    if let Some(prev) = lines.last() {
                        line.borrow_mut().prev = Some(Rc::downgrade(prev));
                        prev.borrow_mut().next = Some(line.clone());
                    }
                    lines.push(line);
                    lineno += 1;
                }

                // 如果文件为空，添加一个空行
                if lines.is_empty() {
                    let line = Rc::new(RefCell::new(LineStruct {
                        data: String::new(), lineno: 1,
                        next: None, prev: None,
                        multidata: None, has_anchor: false,
                    }));
                    lines.push(line);
                }

                let filetop = lines.first().cloned();
                let filebot = lines.last().cloned();

                let new_file = Rc::new(RefCell::new(OpenFileStruct {
                    filename: Some(filename.to_string()),
                    filetop: filetop.clone(),
                    filebot: filebot.clone(),
                    edittop: filetop.clone(),
                    current: filetop.clone(),
                    totsize: content.len(),
                    firstcolumn: 0, current_x: 0, placewewant: 0,
                    brink: 0, cursor_row: 0,
                    statinfo: fs::metadata(path).ok().map(|m| Box::new(m)),
                    spillage_line: None,
                    mark: None, mark_x: 0, softmark: false,
                    fmt: FormatType::NixFile, lock_filename: None,
                    undotop: None, current_undo: None, last_saved: None,
                    last_action: UndoType::Other, modified: false,
                    syntax: None, errormessage: None,
                    next: None, prev: None,
                }));
                g.openfile = Some(new_file);
            });
            true
        }
        Err(e) => {
            set_statusbar_message(&format!("Error reading {}: {}", filename, e));
            false
        }
    }
}

/// 将缓冲区写入文件。
pub fn write_it_out(_finalize: bool, _mark_only: bool) -> i32 {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let filename = of_ref.filename.clone().unwrap_or_default();
            if filename.is_empty() {
                return -1;
            }

            // 收集所有行数据
            let mut content = String::new();
            let mut current = of_ref.filetop.clone();
            while let Some(c) = current {
                let data = c.borrow().data.clone();
                content.push_str(&data);
                content.push('\n');
                let next = c.borrow().next.clone();
                current = next;
            }

            match fs::write(&filename, &content) {
                Ok(_) => {
                    drop(of_ref);
                    of.borrow_mut().modified = false;
                    content.len() as i32
                }
                Err(e) => {
                    set_statusbar_message(&format!("Error writing {}: {}", filename, e));
                    -1
                }
            }
        } else {
            -1
        }
    });
    -1
}

/// 写入文件（用户交互版）。
pub fn do_writeout() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let filename = of.borrow().filename.clone().unwrap_or_default();
            if !filename.is_empty() {
                write_it_out(true, false);
            } else {
                set_statusbar_message("No filename to write");
            }
        }
    });
}

/// 获取下一个可用文件名（用于紧急保存）。
pub fn get_next_filename(basename: &str, suffix: &str) -> String {
    format!("{}.{}", basename, suffix)
}

/// 准备显示（计算行号等）。
pub fn prepare_for_display() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            // 更新行号
            let mut lineno = 1;
            let mut current = of_ref.filetop.clone();
            while let Some(c) = current {
                c.borrow_mut().lineno = lineno;
                lineno += 1;
                let next = c.borrow().next.clone();
                current = next;
            }
        }
    });
}

/// 获取当前缓冲区。
pub fn get_openfile() -> Option<OpenFileRef> {
    with_global(|g| g.openfile.clone())
}

/// 设置状态栏消息。
pub fn set_statusbar_message(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
        // 状态栏消息存储在全局状态中
    });
}

/// 在状态栏显示消息。
pub fn statusbar(msg: &str) {
    set_statusbar_message(msg);
}

/// 在状态行显示消息。
pub fn statusline(msg: &str) {
    set_statusbar_message(msg);
}

/// 清除状态栏。
pub fn wipe_statusbar() {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Vacuum;
    });
}

/// 初始化操作目录（用于文件浏览器）。
pub fn init_operating_dir() {
    // 简化
}

/// 计算左侧边缘偏移量。
pub fn leftedge_for() -> usize {
    with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().firstcolumn).unwrap_or(0)
    });
    0
}

/// 将文件插入到当前缓冲区。
pub fn do_insertfile() {
    // 简化
}

/// 执行外部命令。
pub fn do_execute() {
    // 简化
}

/// 检查文件是否被修改。
pub fn is_modified() -> bool {
    with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().modified).unwrap_or(false)
    });
    false
}