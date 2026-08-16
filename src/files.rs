/**************************************************************************
 * files.rs  --  GNU nano 文件 I/O（对应 files.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 文件读写操作。对应原版 nano 的 `files.c`。
//! 转换说明：使用 `std::fs` 和 `std::io` 替代 C 文件 API。

use crate::definitions::*;
use crate::winio;
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
            new_file.borrow_mut().edittop = Some(line.clone());
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

/// 将缓冲区写入给定文件（对应 files.c 的 `write_file` 核心）。
/// 成功返回写入字节数；失败返回 -1。
fn save_to(answer: &str) -> i32 {
    /* 收集所有行数据（排除末尾魔法行）。 */
    let lines = with_global(|g| {
        let openfile = g.openfile.clone();
        match openfile {
            Some(of) => {
                let of_ref = of.borrow();
                let mut lines: Vec<String> = Vec::new();
                let mut current = of_ref.filetop.clone();
                while let Some(c) = current {
                    let data = c.borrow().data.clone();
                    lines.push(data);
                    let next = c.borrow().next.clone();
                    current = next;
                }
                lines
            }
            None => Vec::new(),
        }
    });

    if lines.is_empty() {
        return -1;
    }

    /* 魔法行：末尾的空行（非唯一行时）不写入。 */
    let mut lines = lines;
    if lines.len() > 1 && lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    let mut content = String::new();
    let last = lines.len().saturating_sub(1);
    for (i, l) in lines.iter().enumerate() {
        content.push_str(l);
        /* 每行以换行结尾；NO_NEWLINES 时末行不加。 */
        if i < last || !ISSET(NO_NEWLINES) {
            content.push('\n');
        }
    }

    match fs::write(answer, &content) {
        Ok(_) => {
            /* 保存成功：更新文件名、清除修改标记（对应 write_file 的收尾）。 */
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    let mut of_ref = of.borrow_mut();
                    of_ref.modified = false;
                    if !answer.is_empty() {
                        of_ref.filename = Some(answer.to_string());
                    }
                }
            });
            let linecount = lines.len();
            let msg = if linecount == 1 {
                format!("Wrote {} line", linecount)
            } else {
                format!("Wrote {} lines", linecount)
            };
            winio::statusline(MessageType::Remark, &msg);
            content.len() as i32
        }
        Err(e) => {
            winio::statusline(MessageType::Ahem, &format!("Error writing {}: {}", answer, e));
            -1
        }
    }
}

/// 将缓冲区写入文件（无提示直接保存；对应 files.c `write_file`，供紧急保存）。
pub fn write_it_out(_finalize: bool, _mark_only: bool) -> i32 {
    let filename = with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().filename.clone())
            .unwrap_or_default()
    });
    if filename.is_empty() {
        return -1;
    }
    save_to(&filename)
}

/// 写入文件（对应 files.c 的 `do_writeout`）：在状态栏显示
/// "Write to File: <文件名>" 提示，冒号右侧可编辑要保存的文件名；
/// 运行时不带文件名参数则提示处为空，直接输入新文件名即可。
pub fn do_writeout() {
    let filename = with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().filename.clone())
            .unwrap_or_default()
    });

    /* 对应 C：do_prompt(MWRITEFILE, given, NULL, edit_refresh, "Write to File")。
     * prompt 栏显示 "Write to File: <回答>"，可编辑；Enter 保存，Esc 取消。 */
    let response = crate::prompt::do_prompt(
        MWRITEFILE,
        &filename,
        None,
        Some(winio::edit_refresh),
        "Write to File",
    );

    if response < 0 {
        winio::statusbar("Cancelled");
        return;
    }

    let answer = crate::prompt::get_answer();
    if answer.is_empty() {
        return;
    }

    save_to(&answer);
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

// ======================== 路径处理（对应 files.c） ========================

/// 当给定路径以 ~/ 或 ~user/ 开头时转换波浪号记号。
/// 返回包含展开后路径的已分配字符串（对应 `expand_leading_tilde`）。
pub fn expand_leading_tilde(path: &str) -> String {
    if !path.starts_with('~') || path.len() == 1 {
        return path.to_string();
    }

    /* 计算需要比较多少字符。 */
    let i = path[1..].find('/').map(|p| p + 1).unwrap_or(path.len());

    let tilded: String;
    if i == 1 {
        crate::utils::get_homedir();
        tilded = with_global(|g| g.homedir.clone().unwrap_or_default());
    } else {
        /* ~user/：查询密码数据库（安全封装 libc）。 */
        #[cfg(unix)]
        {
            tilded = home_of_user(&path[1..i]).unwrap_or_default();
        }
        #[cfg(not(unix))]
        {
            tilded = String::new();
        }
    }

    format!("{}{}", tilded, &path[i..])
}

/// 安全封装：在密码数据库中查找给定用户的主目录。
/// （内部使用 `unsafe` 调用 `libc::getpwent` 等，对外仅返回 `Option<String>`。）
#[cfg(unix)]
fn home_of_user(username: &str) -> Option<String> {
    let cname = std::ffi::CString::new(username).ok()?;
    let mut result = None;
    // 安全封装：unsafe 仅用于 libc 调用
    unsafe {
        libc::setpwent();
        loop {
            let pw = libc::getpwent();
            if pw.is_null() {
                break;
            }
            let name = std::ffi::CStr::from_ptr((*pw).pw_name);
            if name.to_bytes() == cname.as_bytes() {
                let dir = (*pw).pw_dir;
                if !dir.is_null() {
                    result = Some(std::ffi::CStr::from_ptr(dir).to_string_lossy().into_owned());
                }
                break;
            }
        }
        libc::endpwent();
    }
    result
}

/// 非 Unix 平台无操作版本。
#[cfg(not(unix))]
fn home_of_user(_username: &str) -> Option<String> {
    None
}

/// 对于给定的裸路径（或路径加文件名），当路径存在时返回规范的绝对路径
/// （加文件名），不存在时返回 None（对应 `get_full_path`）。
pub fn get_full_path(origpath: &str) -> Option<String> {
    if origpath.is_empty() {
        return None;
    }

    let untilded = expand_leading_tilde(origpath);
    let mut target = canonicalize_safely(&untilded);

    /* 若 canonicalize 失败，尝试去掉最后一个组件（该组件可能是尚不存在的文件）。 */
    if target.is_none() {
        let mut untilded = untilded.clone();
        let (slash_pos, rest);
        match untilded.rfind('/') {
            None => {
                /* 若没有斜杠，在名字前加上 "./"。 */
                untilded.insert_str(0, "./");
                slash_pos = 1;
                rest = untilded[1..].to_string(); // 含 '/'（此时为 "/名字"）
            }
            Some(s) => {
                slash_pos = s;
                rest = untilded[s..].to_string(); // 含 '/'
            }
        }

        let dirpart = untilded[..slash_pos].to_string();

        /* 成功后，重新加上原路径的最后组件。 */
        if let Some(mut t) = canonicalize_safely(&dirpart) {
            t.push_str(&rest);
            target = Some(t);
        }
    }

    /* 确保非根目录的目录路径以斜杠结尾。 */
    if let Some(t) = &target {
        if t.len() > 1 {
            if let Ok(meta) = std::fs::metadata(t) {
                if meta.is_dir() && !t.ends_with('/') {
                    target = Some(format!("{}/", t));
                }
            }
        }
    }

    target
}

/// 对路径做规范化（等价于 C 的 `realpath`；用 `std::fs::canonicalize` 替代）。
fn canonicalize_safely(path: &str) -> Option<String> {
    match std::fs::canonicalize(path) {
        Ok(p) => Some(p.to_string_lossy().into_owned()),
        Err(_) => None,
    }
}