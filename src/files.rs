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
                fmt: FormatType::NixFile, lock_filename: None,
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
                fmt: FormatType::NixFile, lock_filename: lock_filename.clone(),
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

    // 文件存在：读取内容。
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
                    fmt: FormatType::NixFile, lock_filename: lock_filename.clone(),
                    undotop: None, current_undo: None, last_saved: None,
                    last_action: UndoType::Other, modified: false,
                    syntax: None, errormessage: None,
                    next: None, prev: None,
                }));
                install_buffer(g, new_file, fresh);
            });
            crate::color::find_and_prime_applicable_syntax();
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

    match std::fs::copy(realname, &backupname) {
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

    match fs::write(answer, &content) {
        Ok(_) => {
            /* 保存成功：更新文件名、清除修改标记（对应 write_file 的收尾）。 */
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
                            if let DoLockfileResult::Locked(lockname) = do_lockfile(answer, false) {
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
            let linecount = lines.len();
            let msg = if linecount == 1 {
                crate::t!("files-wrote_one_line", count = linecount.to_string())
            } else {
                crate::t!("files-wrote_lines", count = linecount.to_string())
            };
            winio::statusline(MessageType::Remark, &msg);
            content.len() as i32
        }
        Err(e) => {
            winio::statusline(MessageType::Ahem, &crate::t!("files-error_writing", filename = answer, err = e.to_string()));
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
        &crate::t!("files-write_to_file"),
    );

    if response < 0 {
        winio::statusbar(&crate::t!("files-cancelled"));
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
    /* 若之前未运行的命令输入仍在，恢复它（对应 C 的 foretext）。 */
    let given = with_global(|g| g.foretext.clone()).unwrap_or_default();
    with_global_mut(|g| g.ran_a_tool = false);

    /* 历史列表：执行命令用 execute_history，插入文件用空。 */
    let mut history = with_global(|g| g.execute_history.clone()).unwrap_or_else(|| make_new_node(None));

    let msg = if execute {
        if ISSET(NEW_BUFFER) {
            crate::t!("files-execute_new_buffer")
        } else {
            crate::t!("files-execute_command")
        }
    } else if ISSET(NEW_BUFFER) {
        crate::t!("files-insert_new_buffer")
    } else {
        crate::t!("files-insert_file")
    };

    let response = crate::prompt::do_prompt(
        if execute { MEXECUTE } else { MINSERTFILE },
        &given,
        Some(&mut history),
        Some(winio::edit_refresh),
        &msg,
    );
    with_global_mut(|g| g.execute_history = Some(history));

    /* 取消，或空白回答且非新缓冲区模式时退出。 */
    if response == -1 || (response == -2 && !ISSET(NEW_BUFFER)) {
        winio::statusbar(&crate::t!("files-cancelled"));
        return;
    }

    let answer = crate::prompt::get_answer();

    /* 用户取消或执行了工具时退出。 */
    if with_global(|g| g.ran_a_tool) {
        return;
    }

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
            match fs::read_to_string(&answer) {
                Ok(text) => {
                    insert_text_into_buffer(&text);
                    with_global_mut(|g| g.refresh_needed = true);
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

    with_global_mut(|g| {
        let old = g.openfile.clone();
        match old {
            None => g.openfile = Some(new_of),
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
            }
        }
    });
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

/// 紧急保存所有缓冲区（对应 `emergency_save_all`）。
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
        let filename = openfile.borrow().filename.clone().unwrap_or_default();
        let plainname = if filename.is_empty() {
            format!("nano.{}", std::process::id())
        } else {
            filename.clone()
        };
        let targetname = get_next_filename(&plainname, ".save");
        if !targetname.is_empty() {
            write_it_out(true, false);
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
