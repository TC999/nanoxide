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
use std::path::Path;

/// open_buffer 打开文件后的结果（对应 C 的 open_buffer/open_file 返回值）。
/// 与原版 nano.c 一致：
///   - Ok(FileLoaded)  = 成功加载已有文件
///   - Ok(NewFile)     = 指定了文件名但文件不存在，视为新文件（状态栏应显示 "[ New File ]"）
///   - Ok(Directory)   = 指定的是目录（状态栏应显示 "[ '<name>' is a directory ]"，且不创建缓冲区）
///   - Ok(ErrorRead)   = 文件存在但读取失败（状态栏已显示错误）
///   - Err            = 内部错误
pub enum OpenBufferResult {
    FileLoaded,
    NewFile,
    Directory,
    ErrorRead,
}

/// 打开文件并加载到缓冲区。
///
/// 对应原版 C 的 open_buffer + open_file：
/// * 如果 filename 为空，则创建一个空的新缓冲区（不带文件名）。
/// * 如果 filename 非空，路径指向一个目录，则显示 ALERT 并返回 Directory（不创建缓冲区）。
/// * 如果 filename 非空但文件不存在，则创建一个新缓冲区并返回 NewFile。
/// * 否则读取文件内容。
pub fn open_buffer(filename: &str) -> OpenBufferResult {
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
            g.openfile = Some(new_file);
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
            g.openfile = Some(new_file);
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
                    fmt: FormatType::NixFile, lock_filename: None,
                    undotop: None, current_undo: None, last_saved: None,
                    last_action: UndoType::Other, modified: false,
                    syntax: None, errormessage: None,
                    next: None, prev: None,
                }));
                g.openfile = Some(new_file);
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

/// 关闭当前缓冲区并回到前一个（对应 `close_buffer`）。
pub fn close_buffer() {
    with_global_mut(|g| {
        let of = g.openfile.clone();
        if let Some(cur) = of {
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
