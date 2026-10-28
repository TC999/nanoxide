/**************************************************************************
 * files.rs  --  GNU nano 文件 I/O（对应 files.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 文件读写操作。对应原版 nano 的 `files.c`。
//! 转换说明：使用 `std::fs` 和 `std::io` 替代 C 文件 API。

use crate::definitions::*;
use crate::utils;
use crate::winio;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::Path;

/// 获取当前打开的缓冲区引用（对应 C 的全局 `openfile`）。
fn openfile_ref() -> OpenFileRef {
    with_global(|g| g.openfile.clone()).expect("no open file")
}

/// open_buffer 打开文件后的结果（对应 C 的 open_buffer/open_file 返回值）。
/// 与原版 nano.c 一致：
///   - Ok(FileLoaded)  = 成功加载已有文件
///   - Ok(NewFile)     = 指定了文件名但文件不存在，视为新文件（状态栏应显示 "[ New File ]"）
///   - Ok(Directory)   = 指定的是目录（状态栏应显示 "[ '<name>' is a directory ]"，且不创建缓冲区）
///   - Ok(ErrorRead)   = 文件存在但读取失败（状态栏已显示错误）
///   - Ok(Skipped)     = 锁文件已存在且用户选择不打开（对应 C 版 do_lockfile 返回 SKIPTHISFILE）
///   - Err            = 内部错误
pub enum OpenBufferResult {
    FileLoaded,
    NewFile,
    Directory,
    ErrorRead,
    Skipped,
    /// 设备文件或无效文件名：显示提示后不加载（对应 C 的 open_buffer 返回 FALSE）。
    Unsupported,
}

/// 打开文件并加载到缓冲区。
///
/// 对应原版 C 的 open_buffer + open_file：
/// * 如果 filename 为空，则创建一个空的新缓冲区（不带文件名）。
/// * 如果 filename 非空，路径指向一个目录，则显示 ALERT 并返回 Directory（不创建缓冲区）。
/// * 如果 filename 非空但文件不存在，则创建一个新缓冲区并返回 NewFile。
/// * 否则读取文件内容。
pub fn open_buffer(filename: &str) -> OpenBufferResult {
    open_buffer_impl(filename, false)
}

/// 打开文件到新缓冲区（对应 C 版 `open_buffer(filename, TRUE)`）：
/// 创建新缓冲区并链入缓冲区列表，然后把文件内容读入其中。
/// 用于命令行多文件启动。
pub fn open_another_buffer(filename: &str) -> OpenBufferResult {
    make_new_buffer();
    let result = open_buffer_impl(filename, true);
    /* 目录等失败情况不保留空缓冲区（对应 C：open_buffer 失败时 continue）。 */
    if matches!(
        result,
        OpenBufferResult::Directory | OpenBufferResult::ErrorRead | OpenBufferResult::Skipped
    ) {
        close_buffer();
    }
    result
}

/// 从标准输入读取数据到新缓冲区（对应 `scoop_stdin`，命令行文件名为
/// "-" 时由 main 调用）。整份 stdin 作为缓冲区内容；内容非空且非只读
/// 模式时置修改标记（对应 C 版 !VIEW_MODE && totsize > 0 的 set_modified）。
/// 读取失败返回 FALSE（状态栏已提示）。
pub fn scoop_stdin() -> bool {
    let mut bytes: Vec<u8> = Vec::new();
    if std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut bytes).is_err() {
        winio::statusline(
            MessageType::Alert,
            &crate::t!("files-failed_to_open_stdin"),
        );
        return false;
    }

    /* 与 read_file 一致：按 -N/-u 处理 stdin 的行尾并检测 DOS 格式。 */
    let (stdin_lines, dos_format, _num_lines) = split_file_lines(&bytes);
    let format = if ISSET(MAKE_IT_UNIX) {
        FormatType::NixFile
    } else if dos_format {
        FormatType::DosFile
    } else {
        FormatType::NixFile
    };
    let total_size: usize = stdin_lines.iter().map(|line| line.len()).sum();

    /* 创建一个新缓冲区并把 stdin 内容读入其中（与 open_buffer_impl 的
     * 读取分支同构，但缓冲区无文件名）。 */
    make_new_buffer();
    with_global_mut(|g| {
        let mut lines: Vec<LineRef> = Vec::new();
        let mut lineno = 1;
        for line_str in &stdin_lines {
            let line = Rc::new(RefCell::new(LineStruct {
                data: line_str.clone(),
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
            filename: None,
            filetop: filetop.clone(),
            filebot: filebot.clone(),
            edittop: filetop.clone(),
            current: filetop.clone(),
            totsize: total_size,
            firstcolumn: 0, current_x: 0, placewewant: 0,
            brink: 0, cursor_row: 0,
            statinfo: None, spillage_line: None,
            mark: None, mark_x: 0, softmark: false,
            fmt: format, lock_filename: None,
            undotop: None, current_undo: None, last_saved: None,
            last_action: UndoType::Other, modified: false,
            syntax: None, errormessage: None,
            next: None, prev: None,
        }));
        install_buffer(g, new_file, true);
    });
    crate::color::find_and_prime_applicable_syntax();

    if !ISSET(VIEW_MODE) && total_size > 0 {
        set_modified();
    }
    true
}

/// 把 new_file 安装为当前缓冲区。fresh 为 TRUE 时替换 make_new_buffer
/// 刚创建的空缓冲区（保留链表位置并更新邻居指针），否则直接替换当前。
fn install_buffer(g: &mut GlobalState, new_file: OpenFileRef, fresh: bool) {
    if fresh {
        if let Some(cur) = g.openfile.clone() {
            let next = { let r = cur.borrow(); r.next.clone() };
            let prev = { let r = cur.borrow(); r.prev.clone() };
            /* 更新旧节点的邻居，使其指向 new_file。 */
            if let Some(p) = &prev {
                p.borrow_mut().next = Some(new_file.clone());
            }
            if let Some(n) = &next {
                n.borrow_mut().prev = Some(new_file.clone());
            }
            new_file.borrow_mut().next = next;
            new_file.borrow_mut().prev = prev;
        }
    }
    g.openfile = Some(new_file);
}

/// 打开文件的内部实现。fresh 为 FALSE 时替换当前缓冲区（原行为），
/// TRUE 时填充 make_new_buffer 刚创建的新缓冲区。
fn open_buffer_impl(filename: &str, fresh: bool) -> OpenBufferResult {
    let path = Path::new(filename);

    // 空路径：创建一个不带文件名的空缓冲区（与 C 的 open_buffer("") 一致）。
    if filename.is_empty() {
        with_global_mut(|g| {
            let new_file = Rc::new(RefCell::new(OpenFileStruct {
                filename: None,
                filetop: None, filebot: None, edittop: None,
                current: None, totsize: 0, firstcolumn: 0,
                current_x: 0, placewewant: 0, brink: 0, cursor_row: 0,
                statinfo: None, spillage_line: None,
                mark: None, mark_x: 0, softmark: false,
                fmt: FormatType::Unspecified, lock_filename: None,
                undotop: None, current_undo: None, last_saved: None,
                last_action: UndoType::Other, modified: false,
                syntax: None, errormessage: None,
                next: None, prev: None,
            }));
            let line = Rc::new(RefCell::new(LineStruct {
                data: String::new(), lineno: 1,
                next: None, prev: None,
                multidata: None, has_anchor: false,
            }));
            new_file.borrow_mut().filetop = Some(line.clone());
            new_file.borrow_mut().filebot = Some(line.clone());
            new_file.borrow_mut().edittop = Some(line.clone());
            new_file.borrow_mut().current = Some(line);
            install_buffer(g, new_file, fresh);
        });
        crate::color::find_and_prime_applicable_syntax();
        return OpenBufferResult::FileLoaded;
    }

    /* 操作目录限制（对应 files.c open_buffer 中的 outside_of_confinement）。
     * 路径不在 --operatingdir 内时拒绝读取。 */
    if with_global(|g| g.operating_dir.is_some()) && outside_of_confinement(filename, false) {
        let dir = with_global(|g| g.operating_dir.clone()).unwrap_or_default();
        winio::statusline(
            MessageType::Alert,
            &crate::t!("files-cant_read_outside", dir = dir),
        );
        return OpenBufferResult::ErrorRead;
    }

    // 路径存在且是目录：显示 ALERT 提示。对应 files.c 的
    //   statusline(ALERT, _("'%s' is a directory"), realname);
    // C 版 statusline 对短消息会居中显示并自动加上 "[ ]" 方括号，
    // 因此这里用 statusline_centered 并以 "[ {} ]" 包裹，显示为
    //   [ '目录' is a directory ]
    // 与原版一致：目录不创建缓冲区（open_buffer 返回 FALSE），由调用方
    // （nano.c main）最终打开空白缓冲区。
    if path.exists() && path.is_dir() {
        winio::statusline_centered(
            crate::definitions::MessageType::Alert,
            &format!("[ {} ]", crate::t!("files-is_a_directory", filename = filename)),
        );
        return OpenBufferResult::Directory;
    }

    /* 不打开字符/块设备文件（对应 C 的 `S_ISCHR` / `S_ISBLK` 检查）。 */
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if path.exists() {
            if let Ok(meta) = fs::metadata(path) {
                let file_type = meta.file_type();
                if file_type.is_char_device() || file_type.is_block_device() {
                    winio::statusline_centered(
                        crate::definitions::MessageType::Alert,
                        &format!(
                            "[ {} ]",
                            crate::t!("files-is_device", filename = filename)
                        ),
                    );
                    return OpenBufferResult::Unsupported;
                }
            }
        }
    }

    /* 以分隔符结尾却又不存在的文件名无效（对应 C 的
     * `filename[strlen - 1] == '/'` 分支）。 */
    if !path.exists() && (filename.ends_with('/') || filename.ends_with('\\')) {
        winio::statusline_centered(
            crate::definitions::MessageType::Alert,
            &format!(
                "[ {} ]",
                crate::t!("files-invalid_filename", filename = filename)
            ),
        );
        return OpenBufferResult::Unsupported;
    }

    /* 打开文件前先处理锁文件（对应 C 版 open_buffer 中 do_lockfile 的调用：
     * 在 open_file 之前、LOCKING 且非 VIEW_MODE 且文件名非空时创建锁；
     * 对不存在的文件同样创建（原版如此）。用户选择不打开时返回 Skipped，
     * 不创建缓冲区。 */
    let lock_filename = if ISSET(LOCKING) && !ISSET(VIEW_MODE) {
        match do_lockfile(filename, true) {
            DoLockfileResult::Locked(name) => Some(name),
            DoLockfileResult::NoLock => None,
            DoLockfileResult::SkipFile => return OpenBufferResult::Skipped,
        }
    } else {
        None
    };

    /* -n/--noread：不读取文件内容，只建立带文件名的空缓冲区。
     * 对应 C 版 open_buffer 中 `if (*filename && !ISSET(NOREAD_MODE))` 的门槛：
     * 跳过后 descriptor 仍为 0，缓冲区保持 make_new_buffer 的空行。 */
    if ISSET(NOREAD_MODE) {
        with_global_mut(|g| {
            let new_file = Rc::new(RefCell::new(OpenFileStruct {
                filename: Some(filename.to_string()),
                filetop: None, filebot: None, edittop: None,
                current: None, totsize: 0, firstcolumn: 0,
                current_x: 0, placewewant: 0, brink: 0, cursor_row: 0,
                statinfo: None, spillage_line: None,
                mark: None, mark_x: 0, softmark: false,
                fmt: FormatType::Unspecified, lock_filename: lock_filename.clone(),
                undotop: None, current_undo: None, last_saved: None,
                last_action: UndoType::Other, modified: false,
                syntax: None, errormessage: None,
                next: None, prev: None,
            }));
            let line = Rc::new(RefCell::new(LineStruct {
                data: String::new(), lineno: 1,
                next: None, prev: None,
                multidata: None, has_anchor: false,
            }));
            new_file.borrow_mut().filetop = Some(line.clone());
            new_file.borrow_mut().filebot = Some(line.clone());
            new_file.borrow_mut().edittop = Some(line.clone());
            new_file.borrow_mut().current = Some(line);
            install_buffer(g, new_file, fresh);
        });
        crate::color::find_and_prime_applicable_syntax();
        return OpenBufferResult::FileLoaded;
    }

    // 文件不存在：视为新文件。
    if !path.exists() {
        with_global_mut(|g| {
            let new_file = Rc::new(RefCell::new(OpenFileStruct {
                filename: Some(filename.to_string()),
                filetop: None, filebot: None, edittop: None,
                current: None, totsize: 0, firstcolumn: 0,
                current_x: 0, placewewant: 0, brink: 0, cursor_row: 0,
                statinfo: None, spillage_line: None,
                mark: None, mark_x: 0, softmark: false,
                fmt: FormatType::Unspecified, lock_filename: lock_filename.clone(),
                undotop: None, current_undo: None, last_saved: None,
                last_action: UndoType::Other, modified: false,
                syntax: None, errormessage: None,
                next: None, prev: None,
            }));
            let line = Rc::new(RefCell::new(LineStruct {
                data: String::new(), lineno: 1,
                next: None, prev: None,
                multidata: None, has_anchor: false,
            }));
            new_file.borrow_mut().filetop = Some(line.clone());
            new_file.borrow_mut().filebot = Some(line.clone());
            new_file.borrow_mut().edittop = Some(line.clone());
            new_file.borrow_mut().current = Some(line);
            install_buffer(g, new_file, fresh);
        });
        crate::color::find_and_prime_applicable_syntax();
        return OpenBufferResult::NewFile;
    }

    // 文件存在：读取内容（保留/检测 DOS 行尾，受 -N 与 -u 影响）。
    match read_file_lines(path) {
        Ok((lines_data, detected_fmt, num_lines)) => {
            /* -u/--unix 强制按 Unix 格式保存；否则采用检测到的格式
             * （对应 C 版 read_file 末尾的 MAKE_IT_UNIX / UNSPECIFIED 判断）。 */
            let format = if ISSET(MAKE_IT_UNIX) {
                FormatType::NixFile
            } else {
                detected_fmt
            };
            let total_size: usize = lines_data.iter().map(|line| line.len()).sum();
            with_global_mut(|g| {
                let mut lines: Vec<LineRef> = Vec::new();
                let mut lineno = 1;
                for line_str in &lines_data {
                    let line = Rc::new(RefCell::new(LineStruct {
                        data: line_str.clone(),
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

                let filetop = lines.first().cloned();
                let filebot = lines.last().cloned();

                let new_file = Rc::new(RefCell::new(OpenFileStruct {
                    filename: Some(filename.to_string()),
                    filetop: filetop.clone(),
                    filebot: filebot.clone(),
                    edittop: filetop.clone(),
                    current: filetop.clone(),
                    totsize: total_size,
                    firstcolumn: 0, current_x: 0, placewewant: 0,
                    brink: 0, cursor_row: 0,
                    statinfo: fs::metadata(path).ok().map(|m| Box::new(m)),
                    spillage_line: None,
                    mark: None, mark_x: 0, softmark: false,
                    fmt: format, lock_filename: lock_filename.clone(),
                    undotop: None, current_undo: None, last_saved: None,
                    last_action: UndoType::Other, modified: false,
                    syntax: None, errormessage: None,
                    next: None, prev: None,
                }));
                install_buffer(g, new_file, fresh);
            });
            crate::color::find_and_prime_applicable_syntax();
            /* 对应 C 版 read_file 末尾的 report_size = TRUE：MINIBAR 下
             * 下一次刷新时显示读入后的行数。 */
            with_global_mut(|g| g.report_size = true);
            /* 文件不可写时优先提示（对应 C 版 read_file 的 access(W_OK)
             * 检查；查看模式不做检查）。 */
            if !ISSET(VIEW_MODE) && !is_writable_file(filename) {
                winio::statusline(
                    MessageType::Alert,
                    &crate::t!("files-unwritable", filename = filename),
                );
            } else {
                /* "Read N lines" 提示（打开文件路径：undoable = FALSE）。 */
                show_read_message(num_lines, format, false);
            }
            OpenBufferResult::FileLoaded
        }
        Err(e) => {
            set_statusbar_message(&crate::t!("files-error_reading", filename = filename, err = e.to_string()));
            OpenBufferResult::ErrorRead
        }
    }
}

/// 删除锁文件。成功返回 TRUE，失败返回 FALSE（对应 `delete_lockfile`）。
pub fn delete_lockfile(lockfilename: &str) -> bool {
    match std::fs::remove_file(lockfilename) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => {
            winio::statusline(
                MessageType::Mild,
                &crate::t!("files-error_deleting_lockfile", filename = lockfilename, err = e.to_string()),
            );
            false
        }
    }
}

/// 把文件名截短到 room 列宽内（对应 C 版 `crop_to_fit` 的简化版）：
/// 宽度足够时原样返回；超宽时保留尾部并在前面加 "..."；room 太小返回 "_"。
/// 宽度按 ASCII=1、其他=2 近似（文件名多为 ASCII，够用）。
fn crop_to_fit_filename(name: &str, room: usize) -> String {
    if crate::utils::breadth(name.as_bytes()) <= room {
        return name.to_string();
    }
    if room < 4 {
        return "_".to_string();
    }
    let keep = room - 3;
    let mut clipped: Vec<char> = Vec::new();
    let mut width = 0usize;
    for ch in name.chars().rev() {
        let cw = if ch.is_ascii() { 1 } else { 2 };
        if width + cw > keep {
            break;
        }
        clipped.push(ch);
        width += cw;
    }
    let mut out = String::from("...");
    out.extend(clipped.iter().rev());
    out
}

/// 为给定文件名构造锁文件名：`目录/.文件名.swp`（对应 C 版的拼接）。
/// 同时识别 '/' 与 Windows 的 '\\' 分隔符。
pub fn lock_filename_for(filename: &str) -> String {
    let (dir, base) = match filename.rfind(['/', '\\']) {
        Some(slash) => (&filename[..slash], &filename[slash + 1..]),
        None => ("", filename),
    };
    let dir = if dir.is_empty() { "." } else { dir };
    format!("{}/.{}.swp", dir, base)
}

/// 写入锁文件（1024 字节，格式与 vim/nano 兼容）。总是先删除已有锁文件
/// （对应 `write_lockfile`）。成功返回 TRUE。
pub fn write_lockfile(lockfilename: &str, filename: &str, modified: bool) -> bool {
    let mypid = std::process::id();
    let myname = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "localhost".to_string());

    /* 先删除已有锁文件。 */
    if !delete_lockfile(lockfilename) {
        return false;
    }

    /* 创建锁文件——不接受已存在的文件。 */
    let mut data = vec![0u8; 1024];
    data[0] = 0x62;
    data[1] = 0x30;
    let prog = format!("nano {}", env!("CARGO_PKG_VERSION"));
    let prog_bytes = prog.as_bytes();
    let copy_len = prog_bytes.len().min(10);
    data[2..2 + copy_len].copy_from_slice(&prog_bytes[..copy_len]);
    data[24] = (mypid & 0xFF) as u8;
    data[25] = ((mypid >> 8) & 0xFF) as u8;
    data[26] = ((mypid >> 16) & 0xFF) as u8;
    data[27] = ((mypid >> 24) & 0xFF) as u8;
    let name_bytes = myname.as_bytes();
    let copy_len = name_bytes.len().min(16);
    data[28..28 + copy_len].copy_from_slice(&name_bytes[..copy_len]);
    let host_bytes = hostname.as_bytes();
    let copy_len = host_bytes.len().min(32);
    data[68..68 + copy_len].copy_from_slice(&host_bytes[..copy_len]);
    let file_bytes = filename.as_bytes();
    let copy_len = file_bytes.len().min(768);
    data[108..108 + copy_len].copy_from_slice(&file_bytes[..copy_len]);
    if modified {
        data[1007] = 0x55;
    }

    /* 用 create_new 确保不覆盖已有文件。 */
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lockfilename)
    {
        Ok(mut f) => {
            use std::io::Write;
            if f.write_all(&data).is_err() {
                winio::statusline(
                    MessageType::Mild,
                    &crate::t!("files-error_writing_lockfile", filename = lockfilename, err = "write"),
                );
                return false;
            }
            true
        }
        Err(e) => {
            winio::statusline(
                MessageType::Mild,
                &crate::t!("files-error_writing_lockfile", filename = lockfilename, err = e.to_string()),
            );
            false
        }
    }
}

/// do_lockfile 的结果（对应 C 版 do_lockfile 的返回值）。
#[derive(Debug, Clone)]
pub enum DoLockfileResult {
    /// 锁文件已成功写入，携带锁文件名。
    Locked(String),
    /// 未创建锁文件（失败，或坏锁被忽略），但可继续打开文件。
    NoLock,
    /// 锁文件已存在且用户选择不打开该文件（对应 SKIPTHISFILE）。
    SkipFile,
}

/// 检查锁文件是否已存在并视情况提示或询问用户，然后写入锁文件
/// （对应 C 版 `do_lockfile`）。
///
/// * `ask_the_user` 为 TRUE（打开文件时）：锁文件存在则读取并解析其中
///   的程序名/用户名/PID，询问用户是否仍要打开；回答 No 或取消则返回
///   [`DoLockfileResult::SkipFile`]；回答 Yes 则覆盖旧锁。
/// * 为 FALSE（保存改名时）：锁文件存在只提示 "Someone else is also
///   editing this file" 并停留 1200ms，然后照常覆盖。
/// * 锁文件存在但内容无效（不足 68 字节或魔数不对）：提示 "Bad lock
///   file is ignored" 并返回 [`DoLockfileResult::NoLock`]（不覆盖旧文件）。
pub fn do_lockfile(filename: &str, ask_the_user: bool) -> DoLockfileResult {
    let lockfilename = lock_filename_for(filename);

    if Path::new(&lockfilename).exists() {
        if !ask_the_user {
            winio::blank_bottombars();
            winio::statusline(
                MessageType::Alert,
                &crate::t!("files-someone_else_editing"),
            );
            winio::napms(1200);
        } else {
            /* 读取并校验锁文件（对应 C 版 do_lockfile 的读取/解析分支）。 */
            match fs::read(&lockfilename) {
                Ok(lockbuf) if lockbuf.len() >= 68 && lockbuf[0] == 0x62 && lockbuf[1] == 0x30 => {
                    /* 解析程序名（偏移 2，10 字节）、PID（偏移 24，小端 4 字节）、
                     * 用户名（偏移 28，16 字节）。字段以 NUL 填充，去掉尾随 NUL。 */
                    let lockprog = String::from_utf8_lossy(&lockbuf[2..12])
                        .trim_end_matches('\0')
                        .to_string();
                    let lockpid = (lockbuf[24] as u32)
                        | ((lockbuf[25] as u32) << 8)
                        | ((lockbuf[26] as u32) << 16)
                        | ((lockbuf[27] as u32) << 24);
                    let lockuser = String::from_utf8_lossy(&lockbuf[28..44])
                        .trim_end_matches('\0')
                        .to_string();
                    let pidstring = lockpid.to_string();

                    /* 对应 C 版 crop_to_fit：文件名太长时截短，保证
                     * "open anyway?" 等尾部提示完整显示在一行内。
                     * room = COLS - "File " 前缀宽度 - 其余部分宽度。 */
                    let cols = with_global(|g| g.COLS);
                    let tail = format!(
                        " is being edited by {lockuser} (with {lockprog}, PID {pidstring}); open anyway?"
                    );
                    let room = cols
                        .saturating_sub(crate::utils::breadth(tail.as_bytes()))
                        .saturating_sub("File ".len());
                    let postedname = crop_to_fit_filename(filename, room);

                    let question = crate::t!(
                        "files-being_edited",
                        filename = postedname,
                        user = lockuser,
                        prog = lockprog,
                        pid = pidstring
                    );
                    let choice = crate::prompt::ask_user(false, &question);

                    /* 启动时（尚未运行）取消：退出编辑器（对应 C 版 finish()）。 */
                    if choice == CANCEL && !with_global(|g| g.we_are_running) {
                        winio::terminal_restore();
                        std::process::exit(0);
                    }

                    if choice != YES {
                        winio::wipe_statusbar();
                        return DoLockfileResult::SkipFile;
                    }
                }
                Ok(_) => {
                    /* 坏锁：忽略，且不覆盖旧锁文件（对应 C 版 return NULL）。 */
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!("files-bad_lock_file", filename = lockfilename),
                    );
                    return DoLockfileResult::NoLock;
                }
                Err(e) => {
                    /* 锁文件存在但无法读取：报错后放弃（对应 C 版 return NULL）。 */
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!(
                            "files-error_opening_lockfile",
                            filename = lockfilename,
                            err = e.to_string()
                        ),
                    );
                    return DoLockfileResult::NoLock;
                }
            }
        }
    }

    if write_lockfile(&lockfilename, filename, false) {
        DoLockfileResult::Locked(lockfilename)
    } else {
        DoLockfileResult::NoLock
    }
}

/// 创建已有文件的备份（对应 `make_backup_of` 的简化版：在文件名后加 ~）。
/// 成功返回 TRUE。
fn make_backup_of(realname: &str) -> bool {
    winio::statusbar(&crate::t!("files-making_backup"));

    let backupname = format!("{}~", realname);

    /* 先删除已有备份文件。 */
    if std::fs::remove_file(&backupname).is_err()
        && !std::path::Path::new(&backupname).exists()
    {
        /* 备份文件不存在则无需删除。 */
    }

    match fs::File::open(realname) {
        Err(e) => {
            winio::statusline(
                MessageType::Alert,
                &crate::t!("files-cannot_write_backup", filename = backupname, err = e.to_string()),
            );
            return false;
        }
        Ok(original) => {
            let mut original = original;
            let mut backup_file = match fs::File::create(&backupname) {
                Err(e) => {
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!("files-cannot_write_backup", filename = backupname, err = e.to_string()),
                    );
                    return false;
                }
                Ok(f) => f,
            };
            /* 流式复制原文件内容到备份文件（对应 C 版 copy_file）。 */
            match copy_file(&mut original, &mut backup_file, true) {
                Ok(_) => true,
                Err(e) => {
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!("files-cannot_write_backup", filename = backupname, err = e.to_string()),
                    );
                    false
                }
            }
        }
    }
}

/// 按 C 版 `read_file` 的规则把文件字节切成行，并检测 DOS 行尾。
///
/// - 每个 `\n` 结束一行；`!NO_CONVERT` 时剥掉行尾的 `\r`，并在第一行
///   遇到 CRLF 时报告 DOS 格式（与 C 的 `num_lines == 0` 判断一致）。
/// - 文件末尾没有换行时保留最后一段；空文件返回一个空行。
/// - `NO_CONVERT` 时保留 `\r`，这样保存回去仍是原始 CRLF。
///
/// 第三个返回值是 C 的 `num_lines`：每个换行计一行，末尾无换行的内容
/// 再计一行；空文件为 0（与用于显示的"一个空行"区分开）。
fn split_file_lines(bytes: &[u8]) -> (Vec<String>, bool, usize) {
    let keep_cr = ISSET(NO_CONVERT);
    let mut lines: Vec<String> = Vec::new();
    let mut start = 0;
    let mut dos_format = false;
    let mut num_lines = 0usize;

    for index in 0..bytes.len() {
        if bytes[index] == b'\n' {
            let mut end = index;
            if end > start && bytes[end - 1] == b'\r' && !keep_cr {
                end -= 1;
                /* 只有第一处行尾才决定格式（对应 C 版的 num_lines == 0）。 */
                if lines.is_empty() {
                    dos_format = true;
                }
            }
            lines.push(String::from_utf8_lossy(&bytes[start..end]).into_owned());
            num_lines += 1;
            start = index + 1;
        }
    }

    if start < bytes.len() {
        lines.push(String::from_utf8_lossy(&bytes[start..]).into_owned());
        num_lines += 1;
    } else if lines.is_empty() {
        lines.push(String::new());
    }

    (lines, dos_format, num_lines)
}

/// 读取文件的全部行并返回检测到的格式（供 open_buffer / insert_a_file 共用）。
fn read_file_lines(path: &Path) -> std::io::Result<(Vec<String>, FormatType, usize)> {
    let bytes = fs::read(path)?;
    let (lines, dos_format, num_lines) = split_file_lines(&bytes);
    let format = if dos_format {
        FormatType::DosFile
    } else {
        FormatType::NixFile
    };
    Ok((lines, format, num_lines))
}

/// 检查文件是否可写（对应 C 版 `read_file` 里的 `access(filename, W_OK)`）。
/// Unix 直接用 `access`；Windows 没有该接口，以追加方式打开探测（不写入内容）。
#[cfg(unix)]
fn is_writable_file(path: &str) -> bool {
    use std::ffi::CString;
    let Ok(c_path) = CString::new(path) else {
        return false;
    };
    /* 安全性：access 只读取路径字符串，不修改任何进程或文件状态。 */
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

/// Windows：以追加方式打开做探测；打开成功即认为可写。
#[cfg(not(unix))]
fn is_writable_file(path: &str) -> bool {
    std::fs::OpenOptions::new().append(true).open(path).is_ok()
}

/// 是否"磁盘已满"错误（对应 C 版 write_file 里的 `errno == ENOSPC`）。
#[cfg(unix)]
pub fn is_disk_full(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(libc::ENOSPC)
}

/// Windows：Win32 错误码 112 = ERROR_DISK_FULL。
#[cfg(windows)]
pub fn is_disk_full(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(112)
}

#[cfg(not(any(unix, windows)))]
pub fn is_disk_full(_err: &std::io::Error) -> bool {
    false
}

/// 目标是否是命名管道（对应 C 的 `S_ISFIFO(fileinfo.st_mode)` 判断）。
#[cfg(unix)]
pub fn is_fifo_file(path: &str) -> bool {
    use std::os::unix::fs::FileTypeExt;
    fs::metadata(path)
        .map(|meta| meta.file_type().is_fifo())
        .unwrap_or(false)
}

/// Windows 没有命名管道文件类型。
#[cfg(not(unix))]
pub fn is_fifo_file(_path: &str) -> bool {
    false
}

/// 显示 "Read N lines" 消息（对应 `read_file` 末尾的状态栏提示）。
///
/// 打开文件时在 ZERO/MINIBAR 下静默（行数交给 minibar 显示）；插入文件时
/// 即使处于这两种模式也显示（对应 C 的 `!(we_are_running && undoable)`）。
fn show_read_message(num_lines: usize, format: FormatType, undoable: bool) {
    if (ISSET(ZERO) || ISSET(MINIBAR)) && !(with_global(|g| g.we_are_running) && undoable) {
        return;
    }

    let count = num_lines.to_string();
    let msg = match (format == FormatType::DosFile, num_lines == 1) {
        (true, true) => crate::t!("files-read_dos_one", count = count),
        (true, false) => crate::t!("files-read_dos_many", count = count),
        (false, true) => crate::t!("files-read_one", count = count),
        (false, false) => crate::t!("files-read_many", count = count),
    };
    winio::statusline(MessageType::Remark, &msg);
}

/// 收集给定缓冲区的全部行数据。
fn lines_of(openfile: &OpenFileRef) -> Vec<String> {
    let of_ref = openfile.borrow();
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

/// 将缓冲区写入给定文件（对应 files.c 的 `write_file` 核心）。
/// 成功返回写入字节数；失败返回 -1。
fn save_to(answer: &str) -> i32 {
    save_to_with_method(answer, WritingType::Overwrite)
}

/// 按 C 的 WritingType 写入缓冲区：Overwrite / Append / Prepend
/// （对应 write_file 的 method 参数）。公开供保存流程与测试使用。
pub fn save_to_with_method(answer: &str, method: WritingType) -> i32 {
    let lines = with_global(|g| {
        g.openfile
            .as_ref()
            .map(lines_of)
            .unwrap_or_default()
    });
    finish_save(answer, &lines, true, method)
}

/// 将给定行集写出到文件（对应 files.c `write_file` 的行写出部分）：
/// 剔除文件末尾的魔法空行 → 备份 → 写入 → （annotate 时）更新文件名/锁/
/// 修改标记与标题栏 → 状态消息。成功返回写入字节数；失败返回 -1（状态栏已提示）。
/// `annotate` 对应 C 版 `write_file` 的 annotate 参数：整缓冲区常规保存为 TRUE
/// （收尾管理 buffer 元数据），区域写入 / 紧急保存为 FALSE（不触碰 buffer）。
/// `method` 对应 Overwrite / Append / Prepend：后两者在写入前读取原内容，
/// 分别拼成「原内容+缓冲区」或「缓冲区+原内容」（C 版用 O_APPEND / 临时文件）。
fn finish_save(
    answer: &str,
    lines: &[String],
    annotate: bool,
    method: WritingType,
) -> i32 {
    if lines.is_empty() {
        return -1;
    }

    /* 魔法行：末尾的空行（非唯一行时）不写入。 */
    let mut lines = lines.to_vec();
    if lines.len() > 1 && lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    /* 行尾按缓冲区格式写出：DOS 为 CRLF，其余为 LF
     * （对应 write_file 中的 `openfile->fmt == DOS_FILE` 分支）。
     * -N 读取时保留的 '\r' 已在行数据里，format 为 NIX，写回仍是 CRLF。 */
    let dos_format = with_global(|g| {
        g.openfile
            .as_ref()
            .map(|of| of.borrow().fmt == FormatType::DosFile)
            .unwrap_or(false)
    });
    let newline = if dos_format { "\r\n" } else { "\n" };

    let mut content = String::new();
    let last = lines.len().saturating_sub(1);
    for (i, l) in lines.iter().enumerate() {
        content.push_str(l);
        /* 每行以换行结尾；NO_NEWLINES 时末行不加。 */
        if i < last || !ISSET(NO_NEWLINES) {
            content.push_str(newline);
        }
    }

    /* 操作目录限制（对应 write_file 中 `normal && operating_dir && ...`）：
     * SPECIAL（紧急保存）不受限制。 */
    if method != WritingType::Special
        && !answer.is_empty()
        && outside_of_confinement(answer, false)
    {
        let dir = with_global(|g| g.operating_dir.clone()).unwrap_or_default();
        winio::statusline(
            MessageType::Ahem,
            &crate::t!("files-cant_write_outside", dir = dir),
        );
        return -1;
    }

    /* 对已存在且为普通文件的文件做备份（对应 write_file 中 MAKE_BACKUP 分支）。 */
    if ISSET(MAKE_BACKUP) && !answer.is_empty() {
        if let Ok(meta) = fs::metadata(answer) {
            if meta.is_file() {
                if !make_backup_of(answer) {
                    return -1;
                }
            }
        }
    }

    /* 按 method 组合最终内容：Append = 原内容 + 缓冲区，
     * Prepend = 缓冲区 + 原内容（C 版分别用 O_APPEND 与临时文件实现）。
     * Overwrite / Special 直接覆盖。 */
    let mut output = content.into_bytes();
    if method == WritingType::Append {
        let mut merged = fs::read(answer).unwrap_or_default();
        merged.extend_from_slice(&output);
        output = merged;
    } else if method == WritingType::Prepend {
        output.extend_from_slice(&fs::read(answer).unwrap_or_default());
    }

    /* 正常写入前提示（对应 C 版 write_file 的 `statusbar("Writing...")`）。 */
    if method != WritingType::Special {
        winio::statusbar(&crate::t!("files-writing"));
    }
    /* 目标是命名管道时单独提示（对应 C 的 `Writing to FIFO...`）。 */
    if method != WritingType::Special && is_fifo_file(answer) {
        winio::statusbar(&crate::t!("files-writing_to_fifo"));
    }

    /* 写入并确保数据落盘（对应 C 版 write_file 的 fflush + fsync）。
     * FIFO 没有可落盘的存储，跳过 fsync（C 的 `!S_ISFIFO(...)` 条件）；
     * 紧急保存（SPECIAL）新建文件时权限收紧为 0600（C 的
     * `S_IRUSR|S_IWUSR`）。 */
    let write_result = (|| -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            if method == WritingType::Special {
                options.mode(0o600);
            }
        }
        let mut file = options.open(answer)?;
        file.write_all(&output)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            let is_fifo = file
                .metadata()
                .map(|meta| meta.file_type().is_fifo())
                .unwrap_or(false);
            if !is_fifo {
                file.sync_all()?;
            }
        }
        #[cfg(not(unix))]
        {
            file.sync_all()?;
        }
        Ok(())
    })();

    match write_result {
        Ok(_) => {
            /* 保存成功。对应 C 版 write_file 的收尾：仅当 annotate（常规整
             * 缓冲区保存）时更新文件名/锁/修改标记并刷新标题栏；区域写入与
             * 紧急保存不触碰 buffer（annotate == FALSE）。 */
            if annotate {
                with_global_mut(|g| {
                    if let Some(of) = &g.openfile {
                        let mut of_ref = of.borrow_mut();
                        let was_filename = of_ref.filename.clone();
                        of_ref.modified = false;
                        if !answer.is_empty() {
                            of_ref.filename = Some(answer.to_string());
                        }
                        /* 文件名变化时，按需更新锁文件（对应 C 版 write_file 中
                         * 删除旧锁后用 do_lockfile(realname, FALSE) 写新锁）。 */
                        if was_filename.as_deref() != Some(answer) && !answer.is_empty() {
                            if let Some(old_lock) = of_ref.lock_filename.take() {
                                delete_lockfile(&old_lock);
                            }
                            if ISSET(LOCKING) {
                                if let DoLockfileResult::Locked(lockname) =
                                    do_lockfile(answer, false)
                                {
                                    of_ref.lock_filename = Some(lockname);
                                }
                            }
                        } else if ISSET(LOCKING) && !ISSET(VIEW_MODE) && !answer.is_empty() {
                            /* 同名保存：更新锁文件的 modified 位。 */
                            if let Some(lock) = &of_ref.lock_filename {
                                write_lockfile(lock, answer, false);
                            }
                        }
                    }
                });
                /* 更新标题栏（对应 C 版 write_file 中 titlebar(NULL)）。 */
                winio::titlebar(None);
            }
            /* 刚保存后：MINIBAR 模式下改为在 minibar 里显示行数，不再往
             * 状态栏写消息（对应 C 版 write_file 末尾的 report_size 分支）；
             * 紧急保存（SPECIAL）不显示 "Wrote" 消息。 */
            let normal = method != WritingType::Special;
            if ISSET(MINIBAR) && !ISSET(ZERO) && with_global(|g| g.LINES) > 1 && annotate {
                with_global_mut(|g| g.report_size = true);
            } else if normal {
                let linecount = lines.len();
                let msg = if linecount == 1 {
                    &format!("[ {} ]", crate::t!("files-wrote_one_line", count = linecount.to_string()))
                } else {
                    &format!("[ {} ]", crate::t!("files-wrote_lines", count = linecount.to_string()))
                };
                winio::statusline_centered(MessageType::Remark, &msg);
            }
            output.len() as i32
        }
        Err(e) => {
            winio::statusline(MessageType::Ahem, &crate::t!("files-error_writing", filename = answer, err = e.to_string()));
            /* 磁盘满：C 版额外给出"文件可能已被截断"的警告与恢复建议，
             * 并在离开前刷新 statinfo。 */
            if method != WritingType::Special && is_disk_full(&e) {
                winio::napms(3200);
                winio::statusline(MessageType::Alert, &crate::t!("files-disk_full"));
                winio::napms(3200);
                winio::statusline(MessageType::Alert, &crate::t!("files-disk_full_remedy"));
                if let Ok(meta) = fs::metadata(answer) {
                    with_global_mut(|g| {
                        if let Some(of) = &g.openfile {
                            of.borrow_mut().statinfo = Some(Box::new(meta));
                        }
                    });
                }
            }
            -1
        }
    }
}

/// 将当前标记区域写入给定文件（对应 files.c 的 `write_region_to_file`）。
/// 提示与写入方法与整文件保存共用 `finish_save`（覆盖写）。区域为空
/// （无标记或零字符）时不写文件并返回 -1。
pub fn write_region_to_file(answer: &str) -> i32 {
    write_region_to_file_with_method(answer, WritingType::Overwrite)
}

/// 按指定写入方式（覆盖/追加/前置）写当前标记区域。
pub fn write_region_to_file_with_method(answer: &str, method: WritingType) -> i32 {
    let region = with_global(|g| {
        let of = g.openfile.as_ref()?.borrow();
        let mark = of.mark.clone()?;
        let current = of.current.clone()?;
        /* 标记在光标前（或同处）时，区域 = 标记..光标；否则相反（对应
         * C 版 mark_is_before_cursor 的 get_region 分支）。 */
        let mark_before = {
            let m_line = mark.borrow().lineno;
            let c_line = current.borrow().lineno;
            m_line < c_line || (Rc::ptr_eq(&mark, &current) && of.mark_x <= of.current_x)
        };
        let (top, top_x, bot, bot_x) = if mark_before {
            (mark, of.mark_x, current, of.current_x)
        } else {
            (current, of.current_x, mark, of.mark_x)
        };
        Some((top, top_x, bot, bot_x))
    });

    let (top, top_x, bot, bot_x) = match region {
        Some(r) => r,
        None => return -1,
    };

    /* 收集区域的行集：起始行自 top_x 起，终止行截至 bot_x（不含）。
     * bot_x == 0 且跨行时区域不含 bot 行——对应 C 版把 bot 行当魔法
     * 空行剔除、只保留其上一整行作为末行的语义。 */
    let mut lines: Vec<String> = Vec::new();
    let mut line = top.clone();
    loop {
        let is_top = Rc::ptr_eq(&line, &top);
        let is_bot = Rc::ptr_eq(&line, &bot);
        let data = line.borrow().data.clone();

        if is_bot {
            let end = bot_x.min(data.len());
            let start = if is_top { top_x.min(end) } else { 0 };
            if end > start {
                lines.push(data[start..end].to_string());
            }
            break;
        }
        let start = if is_top { top_x.min(data.len()) } else { 0 };
        lines.push(data[start..].to_string());
        let next = line.borrow().next.clone();
        match next {
            Some(n) => line = n,
            None => break,
        }
    }

    finish_save(answer, &lines, false, method)
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

// 保存时是否已经看过彩蛋（对应 files.c `write_it_out` 里的
// `static bool did_credits`）。
thread_local! {
    static DID_CREDITS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn did_credits() -> bool {
    DID_CREDITS.with(|c| c.get())
}

fn set_did_credits() {
    DID_CREDITS.with(|c| c.set(true));
}

/// 写入文件（对应 files.c 的 `write_it_out`，`withprompt` 恒为 TRUE）：
/// 在状态栏显示 "Write to File: <文件名>" 提示，冒号右侧可编辑要保存的
/// 文件名；运行时不带文件名参数则提示处为空，直接输入新文件名即可。
/// 当标记开启、非受限模式且不是在退出时（`exiting` 为 FALSE）时，改为
/// 写入选中的区域（"Write Selection to File"，对应 C 版 write_it_out 的
/// mark 分支）。
///
/// 提示期间支持 M-D（DOS 格式）、M-B（备份）、M-A/M-P（追加/前置）、
/// ^Q（放弃写入）与 ^T（浏览文件），对应 C 版 write_it_out 的循环。
/// 返回写入字节数；-1 失败；2 表示用户放弃。
///
/// `exiting` 为 TRUE 表示本次写入来自 `do_exit`（对应 C 版
/// `write_it_out(TRUE, TRUE)`），此时忽略标记、整块保存，并可能触发
/// 文件名输入 "zzy" 的彩蛋。
pub fn do_writeout(exiting: bool) -> i32 {
    let (filename, mark_on) = with_global(|g| {
        let filename = g
            .openfile
            .as_ref()
            .and_then(|of| of.borrow().filename.clone())
            .unwrap_or_default();
        let mark_on = g
            .openfile
            .as_ref()
            .map(|of| of.borrow().mark.is_some())
            .unwrap_or(false);
        (filename, mark_on)
    });

    /* 受限模式下不提供区域写入（会允许把选区内内容写往命令行未指定的文件，
     * 对应 C 版 !ISSET(RESTRICTED) 条件）；退出时也整块保存，不写选区。 */
    let selection = mark_on && !exiting && !in_restricted_mode();
    let mut given = if selection {
        String::new()
    } else {
        filename.clone()
    };
    let mut method = WritingType::Overwrite;

    loop {
        /* 提示文本按 method 与格式/备份状态组合（对应 C 的 msg + formatstr +
         * backupstr）。 */
        let mut msg = if selection {
            match method {
                WritingType::Append => crate::t!("files-append_selection"),
                WritingType::Prepend => crate::t!("files-prepend_selection"),
                _ => crate::t!("files-write_selection_to_file"),
            }
        } else {
            match method {
                WritingType::Append => crate::t!("files-append_to_file"),
                WritingType::Prepend => crate::t!("files-prepend_to_file"),
                _ => crate::t!("files-write_to_file"),
            }
        };
        if with_global(|g| {
            g.openfile
                .as_ref()
                .map(|of| of.borrow().fmt == FormatType::DosFile)
                .unwrap_or(false)
        }) {
            msg.push_str(&crate::t!("files-dos_format_marker"));
        }
        if ISSET(MAKE_BACKUP) {
            msg.push_str(&crate::t!("files-backup_marker"));
        }

        let response = crate::prompt::do_prompt(
            MWRITEFILE,
            &given,
            None,
            Some(winio::edit_refresh),
            &msg,
        );

        if response < 0 {
            winio::statusbar(&crate::t!("files-cancelled"));
            return 0;
        }

        /* 非 Enter/取消的退出按键：C 版用 func_from_key(response) 取函数。 */
        let function = if response > 0 {
            crate::global::find_shortcut(response, MWRITEFILE)
                .map(|shortcut| shortcut.borrow().func)
        } else {
            None
        };
        given = crate::prompt::get_answer();

        match function {
            Some(FunctionId::DosFormat) if !in_restricted_mode() => {
                with_global_mut(|g| {
                    if let Some(of) = &g.openfile {
                        let mut of_ref = of.borrow_mut();
                        of_ref.fmt = if of_ref.fmt == FormatType::DosFile {
                            FormatType::NixFile
                        } else {
                            FormatType::DosFile
                        };
                    }
                });
                continue;
            }
            Some(FunctionId::BackItUp) if !in_restricted_mode() => {
                TOGGLE(MAKE_BACKUP);
                continue;
            }
            Some(FunctionId::AppendIt) if !in_restricted_mode() => {
                method = if method == WritingType::Append {
                    WritingType::Overwrite
                } else {
                    WritingType::Append
                };
                if given == filename {
                    given.clear();
                    crate::prompt::set_answer("");
                }
                continue;
            }
            Some(FunctionId::PrependIt) if !in_restricted_mode() => {
                method = if method == WritingType::Prepend {
                    WritingType::Overwrite
                } else {
                    WritingType::Prepend
                };
                if given == filename {
                    given.clear();
                    crate::prompt::set_answer("");
                }
                continue;
            }
            Some(FunctionId::ToFiles) if !in_restricted_mode() => {
                if let Some(chosen) = crate::browser::browse_in(&given) {
                    given = chosen.clone();
                    crate::prompt::set_answer(&chosen);
                }
                continue;
            }
            Some(FunctionId::DiscardBuffer) => {
                with_global_mut(|g| g.final_status = 2);
                return 2;
            }
            Some(FunctionId::DoHelp) => continue,
            _ => {}
        }

        /* 如果用户是在退出时（^X → "Y"）给无名缓冲区输入了 "zzy"，第一次
         * 会滚动播放片尾字幕（对应 C 版 write_it_out 的 ENABLE_EXTRA 彩蛋）。
         * 屏幕太小则拒绝并提示 "Too tiny"。 */
        if exiting
            && !ISSET(SAVE_ON_EXIT)
            && filename.is_empty()
            && given == "zzy"
            && !did_credits()
        {
            if with_global(|g| g.LINES) > 5 && with_global(|g| g.COLS) > 31 {
                winio::do_credits();
                set_did_credits();
            } else {
                /* TRANSLATORS: Concisely say the screen is too small. */
                winio::statusline(MessageType::Ahem, &crate::t!("files-too_tiny"));
            }
            return 0;
        }

        if given.is_empty() {
            return 0;
        }

        return if selection {
            write_region_to_file_with_method(&given, method)
        } else {
            save_to_with_method(&given, method)
        };
    }
}

/// 获取下一个可用文件名（用于紧急保存）。
/// 生成一个尚不存在的文件名：先试 `name + suffix`（suffix 通常形如
/// ".save"），被占用则依次试 `name + suffix + ".1"`、".2"…（对应
/// `get_next_filename`）。极限十万个仍冲突时返回空串（表示无可用名）。
pub fn get_next_filename(name: &str, suffix: &str) -> String {
    let mut i: u64 = 0;
    loop {
        let candidate = if i == 0 {
            format!("{}{}", name, suffix)
        } else {
            format!("{}{}.{}", name, suffix, i)
        };
        if !Path::new(&candidate).exists() {
            return candidate;
        }
        i += 1;
        if i == 100_000 {
            return String::new();
        }
    }
}

/// 准备显示（计算行号等）。
pub fn prepare_for_display() {
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow_mut();
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
        // 状态栏消息存储在全局状态中，供重绘时保留显示。
        g.statusbar_msg = msg.to_string();
        g.statusbar_centered = false;
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

/// 初始化操作目录（对应 files.c 的 `init_operating_dir`）：
/// 解析为绝对路径并 chdir 过去；无效时 die。
pub fn init_operating_dir() {
    let dir = match with_global(|g| g.operating_dir.clone()) {
        Some(dir) => dir,
        None => return,
    };

    let target = match get_full_path(&dir) {
        Some(target) => target,
        None => crate::definitions::die(&crate::t!(
            "files-invalid_operating_dir",
            dir = dir
        )),
    };

    if std::env::set_current_dir(&target).is_err() {
        crate::definitions::die(&crate::t!("files-invalid_operating_dir", dir = dir));
    }

    with_global_mut(|g| g.operating_dir = Some(target));
}

/// 归一化路径用于操作目录前缀比较：统一分隔符、去掉 Windows 的
/// `\\?\` 前缀、Windows 下忽略大小写，并在目录末尾补 `/`。
fn normalize_for_confinement(path: &str, is_dir: bool) -> String {
    let mut normalized = path.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//?/") {
        normalized = rest.to_string();
    }
    #[cfg(windows)]
    {
        normalized = normalized.to_lowercase();
    }
    if is_dir && !normalized.ends_with('/') {
        normalized.push('/');
    }
    normalized
}

/// 判断给定路径是否在操作目录之外（对应 `outside_of_confinement`）。
/// `tabbing` 为 TRUE 时，尚不完整但可能长成操作目录的路径也算“在里面”，
/// 以便文件名补全工作。
pub fn outside_of_confinement(somepath: &str, tabbing: bool) -> bool {
    let operating_dir = match with_global(|g| g.operating_dir.clone()) {
        Some(dir) => dir,
        None => return false,
    };

    let fullpath = match get_full_path(somepath) {
        Some(path) => path,
        /* 取不到绝对路径意味着路径不存在/不可读：非补全时不算越界，
         * 补全时算越界（与 C 版一致）。 */
        None => return tabbing,
    };

    let confined = normalize_for_confinement(&operating_dir, true);
    let full_is_dir = Path::new(&fullpath).is_dir();
    let full = normalize_for_confinement(&fullpath, full_is_dir);

    let is_inside = full.starts_with(&confined);
    let begins_to_be = tabbing && confined.starts_with(&full);

    !is_inside && !begins_to_be
}

/// 计算左侧边缘偏移量。
pub fn leftedge_for() -> usize {
    with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().firstcolumn).unwrap_or(0)
    });
    0
}

/// 将文件插入到当前缓冲区（对应 files.c 的 `do_insertfile`）。
pub fn do_insertfile() {
    if in_restricted_mode() {
        return;
    }
    insert_a_file_or(false);
}

/// 执行外部命令（对应 files.c 的 `do_execute`）。
pub fn do_execute() {
    if in_restricted_mode() {
        return;
    }
    insert_a_file_or(true);
}

/// 将给定文本按行拆成独立的行链表（对应 C 版 `read_file` 中构建行的部分）。
/// 文本末尾的换行符不会生成多余的空行（与文件读取语义一致：魔法行由
/// `ingraft_buffer` 统一维护）。
fn text_into_lines(text: &str) -> LineRef {
    let head = make_new_node(None);
    let mut tail = head.clone();

    let mut lines = text.split('\n');
    if let Some(first) = lines.next() {
        tail.borrow_mut().data = first.to_string();
        for rest in lines {
            let newnode = make_new_node(Some(&*tail.borrow()));
            newnode.borrow_mut().data = rest.to_string();
            files_link_after(&tail, &newnode);
            tail = newnode;
        }
    }

    /* 若文本以换行结尾，去掉由 split 产生的末尾空行（魔法行）。 */
    if text.ends_with('\n') && !Rc::ptr_eq(&head, &tail) {
        let last = tail.clone();
        let prev = { let r = last.borrow(); r.prev.clone() };
        if let Some(p) = prev.and_then(|w| w.upgrade()) {
            let next = { let r = last.borrow(); r.next.clone() };
            if let Some(n) = &next {
                n.borrow_mut().prev = Some(Rc::downgrade(&p));
            }
            p.borrow_mut().next = next;
        }
    }

    head
}

/// 把 newnode 链接到 afterthis 之后（仅调整指针，不更新 filebot）。
fn files_link_after(afterthis: &LineRef, newnode: &LineRef) {
    let after_next = { let r = afterthis.borrow(); r.next.clone() };
    newnode.borrow_mut().next = after_next.clone();
    newnode.borrow_mut().prev = Some(Rc::downgrade(afterthis));
    if let Some(an) = &after_next {
        an.borrow_mut().prev = Some(Rc::downgrade(newnode));
    }
    afterthis.borrow_mut().next = Some(newnode.clone());
}

/// 把行链表转换为文本（每行以 \n 结尾；末行不加）。
fn lines_to_text(top: &LineRef) -> String {
    let mut result = String::new();
    let mut cur = Some(top.clone());
    let mut first = true;
    while let Some(c) = cur {
        let (data, next) = {
            let r = c.borrow();
            (r.data.clone(), r.next.clone())
        };
        if first {
            result.push_str(&data);
            first = false;
        } else {
            result.push('\n');
            result.push_str(&data);
        }
        cur = next;
    }
    result
}

/// 把给定文本插入到当前缓冲区光标处（对应 C 版 `read_file` 的插入语义）。
pub fn insert_text_into_buffer(text: &str) {
    let topline = text_into_lines(text);
    crate::cut::ingraft_buffer(&topline);
}

/// 执行给定命令，可选择把缓冲区的文本喂给命令（管道），并把输出
/// 插入到当前光标处（对应 files.c 的 `execute_command`）。
///
/// 命令以 `|` 开头表示把（标记区域或整个）缓冲区内容作为命令的 stdin，
/// 命令输出替换被过滤的文本；`||` 开头表示输出不捕获而直接送到终端。
pub fn execute_command(command: &str) {
    let mut command = command;
    let mut should_pipe = false;
    let mut capture_output = true;
    if let Some(rest) = command.strip_prefix('|') {
        should_pipe = true;
        if let Some(rest2) = rest.strip_prefix('|') {
            capture_output = false;
            command = rest2;
        } else {
            command = rest;
        }
    }

    let was_lineno = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let r = of.borrow();
            r.current.as_ref().map(|c| c.borrow().lineno)
        }).unwrap_or(1)
    });

    /* 管道模式：把要过滤的文本剪入 cutbuffer 并收集为输入。 */
    let input = if should_pipe {
        with_global_mut(|g| g.ran_a_tool = true);
        add_undo_couple_begin();
        {
            let of = openfile_ref();
            let mut r = of.borrow_mut();
            let marked = r.mark.is_some();
            if !marked {
                if let Some(t) = &r.filetop {
                    r.current = Some(t.clone());
                }
                r.current_x = 0;
            }
        }
        with_global_mut(|g| g.keep_cutbuffer = false);
        crate::cut::do_snip(mark_is_set(), !mark_is_set(), false);
        let text = with_global(|g| g.cutbuffer.clone()).map(|cb| lines_to_text(&cb)).unwrap_or_default();
        text
    } else {
        String::new()
    };

    winio::statusbar(&crate::t!("files-executing"));

    /* 执行命令并捕获输出。 */
    let output = run_shell_command(command, &input, capture_output);

    match output {
        Ok(bytes) => {
            /* 把命令输出插入当前缓冲区。 */
            if capture_output && !bytes.is_empty() {
                insert_text_into_buffer(&bytes);
            }
            if should_pipe {
                add_undo_couple_end();
                /* 管道后回到过滤开始的行（对应 C：was_lineno 定位）。 */
                crate::search::goto_line_posx(was_lineno as isize, 0);
            }
            winio::statusbar(&crate::t!("files-executing"));
        }
        Err(e) => {
            let msg = crate::t!("files-command_failed", err = e);
            winio::statusline(MessageType::Alert, &msg);
            /* 出错时撤销命令所做的改动（对应 C：do_undo + discard_until）。 */
            if should_pipe {
                crate::text::do_undo();
            }
        }
    }

    with_global_mut(|g| {
        g.ran_a_tool = true;
        g.refresh_needed = true;
    });
}

fn mark_is_set() -> bool {
    with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let r = of.borrow();
            r.mark.as_ref().map(|m| {
                r.current.as_ref().map(|c| !Rc::ptr_eq(m, c) || r.mark_x != r.current_x).unwrap_or(true)
            }).unwrap_or(false)
        }).unwrap_or(false)
    })
}

fn add_undo_couple_begin() {
    crate::text::add_undo(UndoType::CoupleBegin, Some("filtering"));
}

fn add_undo_couple_end() {
    crate::text::add_undo(UndoType::CoupleEnd, Some("filtering"));
}

/// 在 shell 中运行命令（跨平台）。`input` 非空时写入 stdin；
/// `capture_output` 为 FALSE 时让输出直接到终端。
fn run_shell_command(command: &str, input: &str, capture_output: bool) -> Result<String, String> {
    use std::process::{Command, Stdio};

    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(command);
        c
    };

    let has_input = !input.is_empty();
    if has_input {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    if capture_output {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    if has_input {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    }

    if capture_output {
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if output.status.success() {
            let mut bytes = output.stdout;
            bytes.extend_from_slice(&output.stderr);
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            let mut bytes = output.stdout;
            bytes.extend_from_slice(&output.stderr);
            Err(String::from_utf8_lossy(&bytes).into_owned())
        }
    } else {
        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(String::new())
        } else {
            Err(format!("command exited with {}", status))
        }
    }
}

/// 插入文件或执行命令（对应 files.c 的 `insert_a_file_or`）。
/// execute 为 TRUE 时运行命令并插入输出，否则插入文件内容。
fn insert_a_file_or(execute: bool) {
    let mut execute = execute;
    /* 若之前未运行的命令输入仍在，恢复它（对应 C 的 foretext）。 */
    let mut given = with_global(|g| g.foretext.clone()).unwrap_or_default();
    let was_multibuffer = ISSET(NEW_BUFFER);
    with_global_mut(|g| g.ran_a_tool = false);

    /* 历史列表：执行命令用 execute_history，插入文件用空。 */
    let mut history = with_global(|g| g.execute_history.clone()).unwrap_or_else(|| make_new_node(None));

    let proceed = loop {
        let menu = if execute { MEXECUTE } else { MINSERTFILE };
        let msg = if execute {
            if ISSET(NEW_BUFFER) {
                crate::t!("files-execute_new_buffer")
            } else {
                crate::t!("files-execute_command")
            }
        } else if ISSET(NEW_BUFFER) {
            if ISSET(NO_CONVERT) {
                crate::t!("files-insert_unconverted_new_buffer")
            } else {
                crate::t!("files-insert_new_buffer")
            }
        } else if ISSET(NO_CONVERT) {
            crate::t!("files-insert_unconverted")
        } else {
            crate::t!("files-insert_file")
        };

        let response = crate::prompt::do_prompt(
            menu,
            &given,
            if execute { Some(&mut history) } else { None },
            Some(winio::edit_refresh),
            &msg,
        );
        with_global_mut(|g| g.execute_history = Some(history.clone()));

        /* 取消，或空白回答且非新缓冲区模式时退出。 */
        if response == -1 || (response == -2 && !ISSET(NEW_BUFFER)) {
            winio::statusbar(&crate::t!("files-cancelled"));
            break false;
        }

        given = crate::prompt::get_answer();

        /* 执行了工具时退出。 */
        if with_global(|g| g.ran_a_tool) {
            break false;
        }

        /* 提示期间的翻转键（对应 insert_a_file_or 的 flip_* 分支）。 */
        let function = if response > 0 {
            crate::global::find_shortcut(response, menu)
                .map(|shortcut| shortcut.borrow().func)
        } else {
            None
        };
        match function {
            Some(FunctionId::FlipNewBuffer) => {
                if ISSET(VIEW_MODE) {
                    winio::beep();
                } else {
                    TOGGLE(NEW_BUFFER);
                }
                continue;
            }
            Some(FunctionId::FlipConvert) => {
                TOGGLE(NO_CONVERT);
                continue;
            }
            Some(FunctionId::FlipExecute) => {
                execute = !execute;
                continue;
            }
            Some(FunctionId::FlipPipe) => {
                crate::prompt::add_or_remove_pipe_symbol_from_answer();
                given = crate::prompt::get_answer();
                continue;
            }
            Some(FunctionId::ToFiles) => {
                if let Some(chosen) = crate::browser::browse_in(&given) {
                    given = chosen.clone();
                    crate::prompt::set_answer(&given);
                }
                continue;
            }
            _ => {}
        }

        /* 其余功能键：重新提示（C 的 `response != 0` 分支）。 */
        if response != 0 && (!ISSET(NEW_BUFFER) || response != -2) {
            continue;
        }

        break true;
    };

    /* NEW_BUFFER 只是本次提示的临时状态，结束后恢复原值（C 的 was_multibuffer）。 */
    if !proceed {
        if was_multibuffer {
            SET(NEW_BUFFER);
        } else {
            UNSET(NEW_BUFFER);
        }
        return;
    }

    let answer = given.clone();

    /* 记住用户最后输入的内容。 */
    if !answer.is_empty() {
        with_global_mut(|g| g.foretext = Some(answer.clone()));
    }

    if execute {
        /* 新缓冲区模式：先打开空白缓冲区。 */
        if ISSET(NEW_BUFFER) {
            make_new_buffer();
        }
        if !answer.is_empty() {
            execute_command(&answer);
            let mut eh = with_global(|g| g.execute_history.clone()).unwrap_or_else(|| make_new_node(None));
            crate::history::update_history(&mut eh, &answer, true);
            with_global_mut(|g| g.execute_history = Some(eh));
        }
    } else {
        /* 插入文件内容。 */
        if ISSET(NEW_BUFFER) {
            let result = open_buffer(&answer);
            /* 用户拒绝覆盖已有锁时（Skipped）：不修改当前缓冲区的状态。 */
            if !matches!(result, OpenBufferResult::Skipped) {
                if let Some(of) = with_global(|g| g.openfile.clone()) {
                    of.borrow_mut().modified = false;
                }
                prepare_for_display();
            }
        } else if outside_of_confinement(&answer, false) {
            /* 与 open_buffer 相同的操作目录检查（C 的 open_buffer 也用于插入）。 */
            let dir = with_global(|g| g.operating_dir.clone()).unwrap_or_default();
            winio::statusline(
                MessageType::Alert,
                &crate::t!("files-cant_read_outside", dir = dir),
            );
        } else {
            let was_lineno = with_global(|g| {
                g.openfile.as_ref().and_then(|of| {
                    let r = of.borrow();
                    r.current.as_ref().map(|c| c.borrow().lineno)
                }).unwrap_or(1)
            });
            let was_x = with_global(|g| {
                g.openfile.as_ref().map(|of| of.borrow().current_x).unwrap_or(0)
            });
            match read_file_lines(Path::new(&answer)) {
                Ok((read_lines, detected_fmt, num_lines)) => {
                    /* 与 C 版 read_file 一致：插入时也按 -N/-u 处理行尾，
                     * 且仅当缓冲区格式仍是 UNSPECIFIED 时采用检测到的格式。 */
                    let text = read_lines.join("\n");
                    insert_text_into_buffer(&text);
                    with_global_mut(|g| {
                        g.refresh_needed = true;
                        if let Some(of) = &g.openfile {
                            let mut of_ref = of.borrow_mut();
                            if ISSET(MAKE_IT_UNIX) {
                                of_ref.fmt = FormatType::NixFile;
                            } else if of_ref.fmt == FormatType::Unspecified {
                                of_ref.fmt = detected_fmt;
                            }
                        }
                    });
                    /* 缓冲区变化时标记为已修改。 */
                    let now_lineno = with_global(|g| {
                        g.openfile.as_ref().and_then(|of| {
                            let r = of.borrow();
                            r.current.as_ref().map(|c| c.borrow().lineno)
                        }).unwrap_or(1)
                    });
                    let now_x = with_global(|g| {
                        g.openfile.as_ref().map(|of| of.borrow().current_x).unwrap_or(0)
                    });
                    if now_lineno != was_lineno || now_x != was_x {
                        set_modified();
                    }
                    /* 对应 C 版 read_file 末尾的 report_size = TRUE。 */
                    with_global_mut(|g| g.report_size = true);
                    /* 插入文件时 undoable = TRUE：即使 ZERO/MINIBAR 也显示
                     * "Read N lines"；格式按 -u 的覆盖规则判断。 */
                    let shown_format = if ISSET(MAKE_IT_UNIX) {
                        FormatType::NixFile
                    } else {
                        detected_fmt
                    };
                    show_read_message(num_lines, shown_format, true);
                }
                Err(e) => {
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!("files-error_reading", filename = answer, err = e.to_string()),
                    );
                }
            }
        }
    }

    if was_multibuffer {
        SET(NEW_BUFFER);
    } else {
        UNSET(NEW_BUFFER);
    }
}

/// 检查文件是否被修改。
pub fn is_modified() -> bool {
    with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().modified).unwrap_or(false)
    })
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

    /* 确保非根目录的目录路径以分隔符结尾（Windows 用 `\`）。 */
    if let Some(t) = &target {
        if t.len() > 1 {
            if let Ok(meta) = std::fs::metadata(t) {
                if meta.is_dir() && !t.ends_with('/') && !t.ends_with('\\') {
                    target = Some(format!("{}{}", t, std::path::MAIN_SEPARATOR));
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

// ======================== 文件工具（对应 files.c / utils.c） ========================

/// 判断给定路径是否是目录（对应 files.c 的 `is_dir`）。
/// 与 C 版一致：先展开开头的 `~`，再检查 stat 的目录位。
pub fn is_dir(path: &str) -> bool {
    let thepath = expand_leading_tilde(path);
    std::fs::metadata(&thepath).map(|m| m.is_dir()).unwrap_or(false)
}

/// 文件浏览器与文件名补全的排序例程（对应 files.c 的 `diralphasort`）：
/// 目录排在文件之前，其余按不区分大小写的字符序排列；
/// 不区分大小写相同时再按字节序比较（与 C 版 strcmp 兜底一致）。
/// 两个参数均为完整路径（filelist 中存放的形式）。
pub fn diralphasort(a: &str, b: &str) -> std::cmp::Ordering {
    let aisdir = is_dir(a);
    let bisdir = is_dir(b);

    if aisdir && !bisdir {
        return std::cmp::Ordering::Less;
    }
    if !aisdir && bisdir {
        return std::cmp::Ordering::Greater;
    }

    let difference = crate::chars::mbstrcasecmp(a.as_bytes(), b.as_bytes());

    /* 忽略大小写仍相同则按字节比较。 */
    if difference == 0 {
        a.as_bytes().cmp(b.as_bytes())
    } else if difference < 0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

/// 补全时使用的基准目录：浏览器当前目录，否则进程工作目录。
/// 保证以路径分隔符结尾（对应 C 的 `present_path`，默认 "./"）。
fn completion_base_dir() -> String {
    let base = with_global(|g| g.present_path.clone())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|_| ".".to_string())
        });
    if base.ends_with('/') || base.ends_with('\\') {
        base
    } else {
        format!("{}/", base)
    }
}

/// 尝试按 `~用户名` 补全（对应 files.c 的 `username_completion`）。
/// 非 Unix 平台没有 passwd 数据库，返回 None。
#[cfg(unix)]
pub fn username_completion(morsel: &str, length: usize) -> Option<Vec<String>> {
    use std::ffi::CStr;

    let end = length.min(morsel.len());
    if end <= 1 {
        return None;
    }
    let fragment = &morsel[1..end];
    let mut matches = Vec::new();

    // 安全性：setpwent/getpwent/endpwent 是 libc 的 passwd 遍历接口；
    // 返回的指针在 endpwent 前有效，这里立即复制字符串。
    unsafe {
        libc::setpwent();
        loop {
            let entry = libc::getpwent();
            if entry.is_null() {
                break;
            }
            let name = CStr::from_ptr((*entry).pw_name)
                .to_string_lossy()
                .into_owned();
            if !name.starts_with(fragment) {
                continue;
            }
            let home_ptr = (*entry).pw_dir;
            if home_ptr.is_null() {
                continue;
            }
            let home = CStr::from_ptr(home_ptr).to_string_lossy().into_owned();
            /* 操作目录之外的用户主目录不参与补全（C 的 tabbing 语义）。 */
            if outside_of_confinement(&home, true) {
                continue;
            }
            matches.push(format!("~{}", name));
        }
        libc::endpwent();
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

#[cfg(not(unix))]
pub fn username_completion(_morsel: &str, _length: usize) -> Option<Vec<String>> {
    None
}

/// 尝试把片段补全为目录中的文件名（对应 files.c 的 `filename_completion`）。
/// 返回的是文件名（不含目录部分）；无法读取目录时返回 None。
pub fn filename_completion(morsel: &str) -> Option<Vec<String>> {
    let separator = morsel.rfind(['/', '\\']);
    let (dirname, filename) = match separator {
        Some(index) => {
            let dirpart = &morsel[..=index];
            let dirname = if Path::new(dirpart).is_absolute() {
                dirpart.to_string()
            } else {
                format!("{}{}", completion_base_dir(), dirpart)
            };
            (dirname, &morsel[index + 1..])
        }
        None => (completion_base_dir(), morsel),
    };

    let dirname = expand_leading_tilde(&dirname);
    let entries = fs::read_dir(&dirname).ok()?;

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." || !name.starts_with(filename) {
            continue;
        }

        let fullname = format!("{}{}", dirname, name);
        /* 操作目录限制（tabbing 模式：不完整路径可长成操作目录）。 */
        if outside_of_confinement(&fullname, true) {
            continue;
        }
        /* 在“转到目录”提示里只补全目录。 */
        if with_global(|g| g.currmenu) == MGOTODIR && !is_dir(&fullname) {
            continue;
        }
        matches.push(name);
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

/// 求一组补全候选的最长公共前缀（按字符边界）。
fn common_prefix(matches: &[String]) -> String {
    let first: Vec<char> = matches[0].chars().collect();
    let mut length = first.len();

    for item in &matches[1..] {
        let other: Vec<char> = item.chars().collect();
        length = length.min(other.len());
        let mut index = 0;
        while index < length && first[index] == other[index] {
            index += 1;
        }
        length = index;
        if length == 0 {
            break;
        }
    }

    first[..length].iter().collect()
}

/// 在状态栏提示里执行文件名补全（对应 files.c 的 `input_tab`）。
/// `morsel` 是当前回答，`place` 是光标位置；命中多个候选时在状态栏列出。
pub fn input_tab(
    morsel: &mut String,
    place: &mut usize,
    refresh_func: Option<fn()>,
    listed: &mut bool,
) {
    /* 光标不在片段末尾时不补全。 */
    if *place != morsel.len() {
        winio::beep();
        return;
    }

    /* 以 ~ 开头且不含斜杠时先尝试用户名补全。 */
    let mut matches = if morsel.starts_with('~') && !morsel.contains(['/', '\\']) {
        username_completion(morsel, *place)
    } else {
        None
    };

    if matches.is_none() {
        matches = filename_completion(morsel);
    }

    let Some(mut matches) = matches else {
        winio::beep();
        return;
    };

    /* 之前列出过候选，但这次只剩一个：恢复编辑窗口。 */
    if *listed && matches.len() < 2 {
        if let Some(refresh) = refresh_func {
            refresh();
        }
        *listed = false;
    }

    let common = common_prefix(&matches);
    let path_length = morsel
        .rfind(['/', '\\'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut shared = format!("{}{}", &morsel[..path_length], common);

    /* 唯一候选且是目录时补上斜杠。 */
    let glued = format!("{}{}", completion_base_dir(), shared);
    if matches.len() == 1
        && (is_dir(&shared) || is_dir(&glued))
        && !shared.ends_with('/')
        && !shared.ends_with('\\')
    {
        shared.push(std::path::MAIN_SEPARATOR);
    }

    if shared.len() != *place {
        *morsel = shared;
        *place = morsel.len();
    } else if matches.len() == 1 {
        winio::beep();
    }

    /* 多个候选：排序后在编辑窗口分栏列出（对应 C 版 input_tab 末尾的
     * qsort + 网格绘制；不在状态栏写消息，以免覆盖提示行）。 */
    if matches.len() > 1 {
        matches.sort_by(|a, b| {
            let folded_a = a.to_lowercase();
            let folded_b = b.to_lowercase();
            folded_a.cmp(&folded_b).then_with(|| a.cmp(b))
        });
        if !*listed {
            winio::beep();
        }
        winio::draw_completion_list(&matches);
        *listed = true;
    }
}

/// 检查给定路径是否为可写目录，成功时返回绝对路径（对应
/// files.c 的 `check_writable_directory`）。C 用 access(W_OK)；这里用
/// 创建探测文件的方式，跨平台且能反映真实 ACL。
pub fn check_writable_directory(path: &str) -> Option<String> {
    let full = get_full_path(path)?;
    if !Path::new(&full).is_dir() {
        return None;
    }

    /* 统一目录结尾分隔符：Windows 的 verbatim 路径（\\?\C:\...）不能
     * 混用 '/'，因此去掉已有分隔符后按平台追加。 */
    let mut base = full.clone();
    let stem = base.trim_end_matches(['/', '\\']);
    if !stem.is_empty() && !stem.ends_with(':') {
        base = stem.to_string();
    }
    let dir = if base.ends_with('/') || base.ends_with('\\') {
        base
    } else {
        format!("{}{}", base, std::path::MAIN_SEPARATOR)
    };

    for attempt in 0..100u32 {
        let probe = format!(
            "{}nano_write_test.{}.{}",
            dir,
            std::process::id(),
            attempt
        );
        match fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
            Ok(_) => {
                let _ = fs::remove_file(&probe);
                return Some(dir);
            }
            Err(_) => continue,
        }
    }

    None
}

/// 创建一个唯一的临时文件并返回其路径（对应 C 版 `safe_tempfile`）。
/// 目录依次尝试 `$TMPDIR`、系统临时目录、当前目录；文件名保留当前
/// 缓冲区的扩展名（与 C 的 `nano.XXXXXX<ext>` 一致，便于 linter 识别）。
pub fn safe_tempfile_name() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        if !tmpdir.is_empty() {
            candidates.push(tmpdir);
        }
    }
    candidates.push(std::env::temp_dir().to_string_lossy().into_owned());
    candidates.push(".".to_string());

    let dir = candidates
        .iter()
        .find_map(|candidate| check_writable_directory(candidate))?;

    let extension = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let filename = of.borrow().filename.clone()?;
            if filename.is_empty() {
                return None;
            }
            let name = std::path::Path::new(&filename)
                .file_name()?
                .to_string_lossy()
                .into_owned();
            let dot = name.rfind('.')?;
            Some(name[dot..].to_string())
        })
    })
    .unwrap_or_default();

    for attempt in 0..100u32 {
        let path = format!(
            "{}nano.{}.{}{}",
            dir,
            std::process::id(),
            attempt,
            extension
        );
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Some(path),
            Err(_) => continue,
        }
    }

    winio::statusline(
        MessageType::Alert,
        &crate::t!("files-no_temp_file", err = "cannot create unique file"),
    );
    None
}

/// 将指定备份目录转换为绝对路径，并校验其可用（对应 `init_backup_dir`）。
/// 若取不到绝对路径（目录不存在或不可访问），或不是目录，则致命退出
/// （对应 C 版 die(_("Invalid backup directory: %s\n"), backup_dir)）。
pub fn init_backup_dir() {
    let target = with_global(|g| g.backup_dir.clone())
        .and_then(|d| get_full_path(&d));

    /* 可用的目录路径会被 get_full_path 补上结尾的分隔符。 */
    let usable = match &target {
        Some(t) => t.ends_with('/') || t.ends_with('\\'),
        None => false,
    };

    if !usable {
        let dir = with_global(|g| g.backup_dir.clone()).unwrap_or_default();
        crate::definitions::die(&crate::t!("files-invalid_backup_dir", dir = dir));
    }

    with_global_mut(|g| g.backup_dir = target);
}

/// 将 `inn` 中剩余的全部数据写入 `out`（对应 files.c 的 `copy_file`）。
/// `inn` 与 `out` 在返回前都会被关闭（所有权释放）；调用方以 `close_out`
/// 决定是否在返回前把 `out` 冲刷（`close_out` 为假时由调用方继续写并负责冲刷，
/// 对应 C 版 `close_out == FALSE` 时的 fflush）。
/// 成功返回复制的字节数，失败返回 Err。
pub fn copy_file<R: std::io::Read, W: std::io::Write>(
    mut inn: R,
    mut out: W,
    close_out: bool,
) -> std::io::Result<u64> {
    let copied = std::io::copy(&mut inn, &mut out)?;
    if close_out {
        out.flush()?;
    }
    Ok(copied)
}

// ======================== 行节点操作（对应 nano.c 的节点函数） ========================

/// 将新节点插入既有 linestruct 链表中（对应 `splice_node`）。
pub fn splice_node(afterthis: &LineRef, newnode: &LineRef) {
    let after_next = { let r = afterthis.borrow(); r.next.clone() };

    newnode.borrow_mut().next = after_next.clone();
    newnode.borrow_mut().prev = Some(Rc::downgrade(afterthis));
    if let Some(an) = &after_next {
        an.borrow_mut().prev = Some(Rc::downgrade(newnode));
    }
    afterthis.borrow_mut().next = Some(newnode.clone());

    /* 当节点插入到缓冲区末尾之后时…… */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let is_filebot = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, afterthis)).unwrap_or(false);
            if is_filebot {
                of.filebot = Some(newnode.clone());
            }
        }
    });
}

/// 释放给定节点中的数据结构（对应 `delete_node`）。
pub fn delete_node(line: &LineRef) {
    /* 若屏幕首行被删除，后退一行。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let is_edittop = of.edittop.as_ref().map(|e| Rc::ptr_eq(e, line)).unwrap_or(false);
            if is_edittop {
                let prev = { let r = line.borrow(); r.prev.clone() };
                of.edittop = prev.and_then(|w| w.upgrade());
            }
            /* 若硬换行的溢出行被删除…… */
            let is_spillage = of.spillage_line.as_ref().map(|s| Rc::ptr_eq(s, line)).unwrap_or(false);
            if is_spillage {
                of.spillage_line = None;
            }
        }
    });
    /* data 与 multidata 由 Rc 自动释放。 */
}

/// 将节点从链表中断开并删除（对应 `unlink_node`）。
pub fn unlink_node(line: &LineRef) {
    let (prev, next) = {
        let r = line.borrow();
        (r.prev.clone(), r.next.clone())
    };

    if let Some(p) = prev.as_ref().and_then(|w| w.upgrade()) {
        p.borrow_mut().next = next.clone();
    }
    if let Some(n) = &next {
        n.borrow_mut().prev = prev.clone();
    }

    /* 删除缓冲区末尾的节点时…… */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let is_filebot = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, line)).unwrap_or(false);
            if is_filebot {
                of.filebot = prev.as_ref().and_then(|w| w.upgrade());
            }
        }
    });

    delete_node(line);
}

/// 释放整条 linestruct 链表（对应 `free_lines`）。
pub fn free_lines(src: Option<LineRef>) {
    let mut src = match src {
        Some(s) => s,
        None => return,
    };

    loop {
        let next = { let r = src.borrow(); r.next.clone() };
        match next {
            Some(n) => {
                let prev = { let r = n.borrow(); r.prev.clone() };
                if let Some(p) = prev.as_ref().and_then(|w| w.upgrade()) {
                    delete_node(&p);
                }
                src = n;
            }
            None => break,
        }
    }

    delete_node(&src);
}

/// 复制一个 linestruct 节点（对应 `copy_node`）。
pub fn copy_node(src: &LineStruct) -> LineRef {
    Rc::new(RefCell::new(LineStruct {
        data: src.data.clone(),
        lineno: src.lineno,
        next: None,
        prev: None,
        multidata: None,
        has_anchor: src.has_anchor,
    }))
}

/// 复制整条 linestruct 链表（对应 `copy_buffer`）。
pub fn copy_buffer(src: &LineRef) -> LineRef {
    let head = copy_node(&src.borrow());
    head.borrow_mut().prev = None;

    let mut item = head.clone();
    let mut srcline = { let r = src.borrow(); r.next.clone() };

    while let Some(s) = srcline {
        let newnode = copy_node(&s.borrow());
        newnode.borrow_mut().prev = Some(Rc::downgrade(&item));
        item.borrow_mut().next = Some(newnode.clone());

        item = newnode;
        srcline = { let r = s.borrow(); r.next.clone() };
    }

    item.borrow_mut().next = None;

    head
}

/// 从给定行开始重新编号缓冲区中的行（对应 `renumber_from`）。
pub fn renumber_from(line: &LineRef) {
    let mut number = {
        let prev = { let r = line.borrow(); r.prev.clone() };
        match prev.and_then(|w| w.upgrade()) {
            Some(p) => p.borrow().lineno,
            None => 0,
        }
    };

    let mut l = line.clone();
    loop {
        number += 1;
        l.borrow_mut().lineno = number;
        let next = { let r = l.borrow(); r.next.clone() };
        match next {
            Some(n) => l = n,
            None => break,
        }
    }
}

// ======================== 缓冲区管理（对应 nano.c） ========================

/// 切换底部栏 ^X 的标签（对应 C 版 install_buffer / close_buffer 对
/// `exitfunc->tag` 的修改）。C 版的 exitfunc 同时覆盖主菜单与文件浏览器，
/// 因此这里改这两个菜单里的 DoExit 项；帮助查看器固定为 "Close"（C 的
/// MHELP 项用的是 close_tag，不参与切换）。
fn set_exit_label(close: bool) {
    let tag = if close {
        crate::t!("key-close")
    } else {
        crate::t!("key-exit")
    };
    with_global_mut(|g| {
        let mut func = g.allfuncs.clone();
        while let Some(f) = func {
            let (menus, next) = {
                let r = f.borrow();
                (r.menus, r.next.clone())
            };
            if menus & (MMAIN | MBROWSER) != 0 {
                let is_exit = { f.borrow().func == FunctionId::DoExit };
                if is_exit {
                    f.borrow_mut().tag = tag.clone();
                }
            }
            func = next;
        }
    });
}

/// 创建新缓冲区并把它设为当前（对应 `make_new_buffer`）。
pub fn make_new_buffer() {
    let new_of = Rc::new(RefCell::new(OpenFileStruct::new()));
    let line = make_new_node(None);
    {
        let mut of = new_of.borrow_mut();
        of.filetop = Some(line.clone());
        of.filebot = Some(line.clone());
        of.current = of.filetop.clone();
        of.edittop = of.filetop.clone();
        of.totsize = 1;
    }

    let became_multiple = with_global_mut(|g| {
        let old = g.openfile.clone();
        match old {
            None => {
                g.openfile = Some(new_of);
                false
            }
            Some(o) => {
                let next = { let r = o.borrow(); r.next.clone() };
                let prev = { let r = o.borrow(); r.prev.clone() };
                new_of.borrow_mut().next = next.clone();
                new_of.borrow_mut().prev = prev;
                if let Some(n) = &next {
                    n.borrow_mut().prev = Some(new_of.clone());
                }
                o.borrow_mut().next = Some(new_of.clone());
                new_of.borrow_mut().prev = Some(o.clone());
                g.openfile = Some(new_of);
                /* 已有多个缓冲区：标题栏改为显示 [x/n]（对应 C 版
                 * install_buffer 中的 `more_than_one = !inhelp || more_than_one`）。 */
                g.more_than_one = !g.inhelp || g.more_than_one;
                true
            }
        }
    });

    /* 多缓冲区时底部栏的 ^X 改为 "Close"（对应 C 版 install_buffer 中的
     * `exitfunc->tag = close_tag`）。set_exit_label 内部要借用全局状态，
     * 必须放在上面的可变借用结束之后。 */
    if became_multiple {
        set_exit_label(true);
    }
}

/// 删除当前缓冲区的锁文件并清空其记录
/// （对应 C 版 `close_and_go` 开头的
/// `if (openfile->lock_filename) delete_lockfile(openfile->lock_filename)`）。
pub fn delete_lockfile_of_current_buffer() {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let lock = { of.borrow_mut().lock_filename.take() };
            if let Some(lock) = lock {
                delete_lockfile(&lock);
            }
        }
    });
}

/// 删除所有缓冲区的锁文件（对应 C 版 `die()` 中遍历删除）。
pub fn delete_all_lockfiles() {
    let openfiles: Vec<OpenFileRef> = {
        let mut result = Vec::new();
        let mut current = with_global(|g| g.openfile.clone());
        loop {
            let next = match current {
                Some(ref ofile) => {
                    result.push(ofile.clone());
                    ofile.borrow().next.clone()
                }
                None => break,
            };
            current = next;
        }
        result
    };
    for ofile in openfiles {
        let lock = { ofile.borrow_mut().lock_filename.take() };
        if let Some(lock) = lock {
            delete_lockfile(&lock);
        }
    }
}

/// 关闭当前缓冲区并回到前一个（对应 `close_buffer`）。
pub fn close_buffer() {
    /* -P/--positionlog：关闭前记录当前缓冲区的位置（对应 C 版
     * close_and_go 中的 update_positions_register）。须在摘除缓冲区之前。 */
    if ISSET(POSITIONLOG) {
        crate::history::update_positions_register();
    }
    with_global_mut(|g| {
        let of = g.openfile.clone();
        if let Some(cur) = of {
            /* 删除此缓冲区的锁文件。 */
            let lock_filename = { let r = cur.borrow(); r.lock_filename.clone() };
            if let Some(lock) = lock_filename {
                delete_lockfile(&lock);
            }
            let prev = { let r = cur.borrow(); r.prev.clone() };
            let next = { let r = cur.borrow(); r.next.clone() };

            /* 从双向链表摘除当前缓冲区。 */
            if let Some(p) = &prev {
                p.borrow_mut().next = next.clone();
            }
            if let Some(n) = &next {
                n.borrow_mut().prev = prev.clone();
            }

            /* 回到前一个缓冲区；若无，则回到下一个。 */
            g.openfile = prev.or(next);
        }
    });

    /* 只剩一个缓冲区时把底部栏 ^X 的标签改回 "Exit"
     * （对应 C 版 close_buffer 末尾的 `if (openfile && openfile == openfile->next)`）。 */
    let single_left = with_global(|g| {
        g.openfile
            .as_ref()
            .map(|of| {
                let r = of.borrow();
                r.next.is_none() && r.prev.is_none()
            })
            .unwrap_or(false)
    });
    if single_left {
        set_exit_label(false);
    }
}

/// 在状态栏显示当前缓冲区的名字与行数（对应 `mention_name_and_linecount`）。
pub fn mention_name_and_linecount() {
    let (count, filename, fmt) = with_global(|g| {
        let of = g.openfile.as_ref().map(|o| o.borrow());
        let count = of.as_ref()
            .and_then(|r| r.filebot.as_ref())
            .map(|b| {
                let data = b.borrow().data.clone();
                let is_empty = data.is_empty();
                let lineno = b.borrow().lineno;
                lineno.saturating_sub(if is_empty { 1 } else { 0 })
            })
            .unwrap_or(0);
        let filename = of.as_ref().and_then(|r| r.filename.clone()).unwrap_or_default();
        let fmt = of.as_ref().map(|r| r.fmt).unwrap_or(FormatType::NixFile);
        (count, filename, fmt)
    });

    if ISSET(MINIBAR) {
        with_global_mut(|g| g.report_size = true);
        return;
    } else if ISSET(ZERO) {
        return;
    }

    let name = if filename.is_empty() {
        crate::t!("winio-new_buffer")
    } else {
        utils::tail(&filename).to_string()
    };

    let msg = if count == 1 {
        crate::t!("files-lines_one", name = name, count = count.to_string())
    } else {
        crate::t!("files-lines_many", name = name, count = count.to_string())
    };

    if matches!(fmt, FormatType::DosFile) {
        /* 非 Unix 格式附加格式说明。 */
        let with_fmt = format!("{} ({})", msg, "DOS");
        winio::statusline(MessageType::Hush, &with_fmt);
    } else {
        winio::statusline(MessageType::Hush, &msg);
    }
}

/// 切换缓冲区后更新标题栏等（对应 `redecorate_after_switch`）。
pub fn redecorate_after_switch() {
    /* 只有一个缓冲区时无需更新。 */
    let only_one = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let r = of.borrow();
            r.next.is_none() && r.prev.is_none()
        }).unwrap_or(true)
    });
    if only_one {
        winio::statusline(MessageType::Ahem, &crate::t!("files-no_more_open_buffers"));
        return;
    }

    /* 更新标题栏与多行信息以匹配当前缓冲区。 */
    prepare_for_display();

    /* 确保主循环重绘帮助行。 */
    with_global_mut(|g| g.currmenu = MMOST);

    /* 防止可能的 Shift 选择被取消。 */
    with_global_mut(|g| g.shift_held = true);

    /* 切换到有错误的缓冲区时显示一次错误消息；否则显示文件名。 */
    let errormessage = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let mut r = of.borrow_mut();
            r.errormessage.take()
        })
    });
    match errormessage {
        Some(e) => winio::statusline(MessageType::Alert, &e),
        None => mention_name_and_linecount(),
    }
}

/// 切换到前一个缓冲区（对应 `switch_to_prev_buffer`）。
pub fn switch_to_prev_buffer() {
    with_global_mut(|g| {
        let of = g.openfile.clone();
        if let Some(cur) = of {
            let prev = { let r = cur.borrow(); r.prev.clone() };
            /* 非循环链表：首个缓冲区的 prev 为 None 时切到最后一个。 */
            match prev {
                Some(p) => g.openfile = Some(p),
                None => {
                    let mut last = cur.clone();
                    loop {
                        let next = { let r = last.borrow(); r.next.clone() };
                        match next {
                            Some(n) => last = n,
                            None => break,
                        }
                    }
                    g.openfile = Some(last);
                }
            }
        }
    });
    redecorate_after_switch();
}

/// 切换到下一个缓冲区（对应 `switch_to_next_buffer`）。
pub fn switch_to_next_buffer() {
    with_global_mut(|g| {
        let of = g.openfile.clone();
        if let Some(cur) = of {
            let next = { let r = cur.borrow(); r.next.clone() };
            /* 非循环链表：末尾缓冲区的 next 为 None 时切回第一个。 */
            match next {
                Some(n) => g.openfile = Some(n),
                None => {
                    let mut first = cur.clone();
                    loop {
                        let prev = { let r = first.borrow(); r.prev.clone() };
                        match prev {
                            Some(p) => first = p,
                            None => break,
                        }
                    }
                    g.openfile = Some(first);
                }
            }
        }
    });
    redecorate_after_switch();
}

/// 将当前缓冲区标记为已修改（对应 `set_modified`）。
pub fn set_modified() {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().modified = true;
        }
    });
    winio::titlebar(None);
}

/// 受限模式时显示警告并返回 TRUE，否则返回 FALSE
/// （对应 `in_restricted_mode`）。
pub fn in_restricted_mode() -> bool {
    if ISSET(RESTRICTED) {
        winio::statusline(MessageType::Ahem, &crate::t!("files-restricted_mode"));
        winio::beep();
        true
    } else {
        false
    }
}

/// 将单个缓冲区以 `.save` 后缀紧急写出（对应 C 版 `emergency_save`）：
/// 未命名缓冲区用 "nano.<pid>"，目标名在冲突时由 `get_next_filename`
/// 唯一化；成功/失败时向 stderr 报告（崩溃恢复场景下状态栏不可用）。
fn emergency_save(openfile: &OpenFileRef) {
    let filename = openfile.borrow().filename.clone().unwrap_or_default();
    let plainname = if filename.is_empty() {
        format!("nano.{}", std::process::id())
    } else {
        filename.clone()
    };
    let targetname = get_next_filename(&plainname, ".save");

    if targetname.is_empty() {
        eprintln!("\n{}", crate::t!("files-too_many_savefiles"));
    } else {
        /* annotate=FALSE：紧急保存不改写缓冲区文件名、不清除修改标记
         * （对应 C 版 write_file(targetname, NULL, SPECIAL, NONOTES)）。 */
        let lines = lines_of(openfile);
        if finish_save(&targetname, &lines, false, WritingType::Special) >= 0 {
            eprintln!("\n{}", crate::t!("files-buffer_written_to", name = targetname));
        }
    }
}

/// 紧急保存所有缓冲区（对应 C 版 `die()` 的保存循环）：对每个已修改且
/// 非受限模式的缓冲区写 `<名字>.save`（未命名者写 `nano.<pid>.save`）。
/// 与 C 一致：紧急保存不改写原文件，也不清除缓冲区的修改标记。
pub fn emergency_save_all() {
    let openfiles: Vec<OpenFileRef> = {
        let mut result = Vec::new();
        let mut current = with_global(|g| g.openfile.clone());
        loop {
            let next = match current {
                Some(ref ofile) => {
                    result.push(ofile.clone());
                    ofile.borrow().next.clone()
                }
                None => break,
            };
            current = next;
        }
        result
    };
    for openfile in openfiles {
        let modified = openfile.borrow().modified;
        /* 受限模式不写命令行未指定的文件（对应 C 版 !ISSET(RESTRICTED)）。 */
        if modified && !in_restricted_mode() {
            emergency_save(&openfile);
        }
    }
}

/// 保存当前缓冲区（对应 `do_savefile`：直接写文件，不进入 WriteOut 提示）。
pub fn do_savefile() {
    let result = write_it_out(false, false);
    if result == 2 {
        /* C 版：write_it_out 返回 2 表示关闭并退出；此处关闭当前缓冲区。 */
        close_buffer();
    }
}
