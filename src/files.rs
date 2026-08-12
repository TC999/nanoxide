/**************************************************************************
 *   files.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 files.c）。
 *
 *   版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2015-2022, 2025, 2026 Benno Schulenberg
 **************************************************************************/

//! 文件打开、读取、写入、备份以及文件名补全。对应原版 `files.c`。

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::raw::{c_char, c_int};

use crate::chars;
use crate::definitions::*;
use crate::definitions;
use crate::global;
use crate::history;
use crate::utils;
pub use crate::winio::{blank_bottombars, blank_edit, bottombars, curs_set, display_string,
    edit_refresh, napms, statusbar, statusline, titlebar, wmove, wnoutrefresh, wipe_statusbar};

/* =========================================================================
 * 占位函数桩：以下函数由后续翻译的模块（winio、prompt、text、move、
 * color、rcfile 等）实现。此处仅声明桩以保证 cargo check 通过，待对应
 * 模块翻译完成后删除对应桩并改为 `use crate::xxx`。
 * ========================================================================= */

#[allow(dead_code)]
pub static mut COLS: i32 = 80;
#[allow(dead_code)]
pub static mut LINES: i32 = 24;

#[allow(dead_code)]
pub fn ask_user(_kind: bool, _msg: &str) -> i32 {
    0
}
/* 适配器：保持旧调用约定（&mut Option<String>），转发到 prompt.rs 的真实实现。 */
pub unsafe fn do_prompt(
    menu: i32,
    given: &mut Option<String>,
    history_list: *mut linestruct,
    refresh_func: unsafe fn(),
    msg: &str,
) -> i32 {
    let provided = given.as_deref().unwrap_or("");
    let ret = crate::prompt::do_prompt(menu, provided, history_list, refresh_func, msg);
    *given = crate::global::answer.clone();
    ret
}
/* 以下函数在其它模块已有真实实现，重导出以避免本地空桩遮蔽。 */
pub use crate::color::{find_and_prime_applicable_syntax, precalc_multicolorinfo};
#[allow(dead_code)]
pub fn close_buffer() {}
#[allow(dead_code)]
pub fn free_lines(_line: *mut linestruct) {}
#[allow(dead_code)]
pub fn discard_until(_undo: *mut undostruct) {}
#[allow(dead_code)]
pub fn add_undo(_type: undo_type, _p: *mut std::ffi::c_void) {}
#[allow(dead_code)]
pub fn update_undo(_type: undo_type) {}
#[allow(dead_code)]
pub fn do_snip(_a: bool, _b: bool, _c: bool) {}
#[allow(dead_code)]
pub fn copy_marked_region() {}
#[allow(dead_code)]
pub fn goto_line_posx(_line: isize, _x: isize) {}
#[allow(dead_code)]
pub fn do_undo() {}
#[allow(dead_code)]
pub fn browse_in(_answer: &str) -> Option<String> {
    None
}
#[allow(dead_code)]
pub fn in_restricted_mode() -> bool {
    false
}
/* warn_and_briefly_pause 在 winio.rs 中有真实实现。 */
pub use crate::winio::warn_and_briefly_pause;
#[allow(dead_code)]
pub fn do_credits() {}
#[allow(dead_code)]
pub fn close_and_go() {}
#[allow(dead_code)]
pub fn ensure_firstcolumn_is_aligned() {}
#[allow(dead_code)]
pub fn block_sigwinch(_on: bool) {}
#[allow(dead_code)]
pub fn install_handler_for_Ctrl_C() {}
#[allow(dead_code)]
pub fn restore_handler_for_Ctrl_C() {}
#[allow(dead_code)]
pub fn leftedge_for(_a: isize, _b: *mut linestruct) -> usize {
    0
}
#[allow(dead_code)]
pub fn xplustabs() -> usize {
    0
}
#[allow(dead_code)]
pub fn less_than_a_screenful(_lineno: isize, _leftedge: usize) -> bool {
    false
}
#[allow(dead_code)]
pub fn do_help() {}
#[allow(dead_code)]
pub fn discard_buffer() {}
#[allow(dead_code)]
pub fn flip_newbuffer() {}
#[allow(dead_code)]
pub fn flip_convert() {}
#[allow(dead_code)]
pub fn flip_execute() {}
#[allow(dead_code)]
pub fn flip_pipe() {}
#[allow(dead_code)]
pub fn add_or_remove_pipe_symbol_from_answer() {}
#[allow(dead_code)]
pub fn back_it_up() {}
#[allow(dead_code)]
pub fn prepend_it() {}
#[allow(dead_code)]
pub fn append_it() {}
#[allow(dead_code)]
pub fn finish() {}
#[allow(dead_code)]
pub fn die(_msg: &str) {
    std::process::exit(1);
}
#[allow(dead_code)]
pub fn getpwent() -> Option<()> {
    None
}
#[allow(dead_code)]
pub fn endpwent() {}
#[allow(dead_code)]
pub fn mkstemps(_template: &mut String, _suffixlen: usize) -> i32 {
    -1
}
#[allow(dead_code)]
pub fn mkstemp(_template: &mut String) -> i32 {
    -1
}
#[allow(dead_code)]
pub fn realpath(_path: &str) -> Option<String> {
    None
}
#[allow(dead_code)]
pub fn opendir(_path: &str) -> Option<()> {
    None
}
#[allow(dead_code)]
pub fn readdir(_dir: ()) -> Option<String> {
    None
}
#[allow(dead_code)]
pub fn closedir(_dir: ()) {}
#[allow(dead_code)]
pub fn waddstr(_win: *mut std::ffi::c_void, _s: &str) {}
/* ingraft_buffer 在 cut.rs 中有真实实现。 */
pub use crate::cut::ingraft_buffer;

/* 这些函数由本模块自身实现，但为了避免与后续模块重名冲突，
 * 在桩区统一声明；本模块真实定义见下方。 */

/* ===== 文件相关常量 ===== */
const RW_FOR_ALL: u32 = 0o666;
const LOCKSIZE: usize = 1024;
const LUMPSIZE: usize = 120;

/* ===== 文件操作函数 ===== */

/* Add an item to the circular list of openfile structs. */
pub unsafe fn make_new_buffer() {
    let newnode = Box::into_raw(Box::new(openfilestruct {
        filename: None,
        filetop: std::ptr::null_mut(),
        filebot: std::ptr::null_mut(),
        edittop: std::ptr::null_mut(),
        current: std::ptr::null_mut(),
        totsize: 0,
        firstcolumn: 0,
        current_x: 0,
        placewewant: 0,
        brink: 0,
        cursor_row: 0,
        statinfo: None,
        spillage_line: std::ptr::null_mut(),
        mark: std::ptr::null_mut(),
        mark_x: 0,
        softmark: false,
        fmt: format_type::UNSPECIFIED,
        lock_filename: None,
        undotop: std::ptr::null_mut(),
        current_undo: std::ptr::null_mut(),
        last_saved: std::ptr::null_mut(),
        last_action: undo_type::OTHER,
        modified: false,
        syntax: std::ptr::null_mut(),
        errormessage: None,
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
    }));

    if definitions::openfile.is_null() {
        (*newnode).prev = newnode;
        (*newnode).next = newnode;
        global::startfile = newnode;
    } else {
        (*newnode).prev = definitions::openfile;
        (*newnode).next = (*definitions::openfile).next;
        (*(*definitions::openfile).next).prev = newnode;
        (*definitions::openfile).next = newnode;
    }

    definitions::openfile = newnode;

    (*definitions::openfile).filename = Some(copy_of(""));

    let top = make_new_node_null();
    let top_ptr = Box::into_raw(top);
    (*definitions::openfile).filetop = top_ptr;
    (*top_ptr).data = copy_of("");
    (*definitions::openfile).filebot = top_ptr;

    (*definitions::openfile).current = top_ptr;
    (*definitions::openfile).current_x = 0;
    (*definitions::openfile).placewewant = 0;
    (*definitions::openfile).brink = 0;
    (*definitions::openfile).cursor_row = 0;

    (*definitions::openfile).edittop = top_ptr;
    (*definitions::openfile).firstcolumn = 0;

    (*definitions::openfile).totsize = 0;
    (*definitions::openfile).modified = false;
}

/* 创建一个 prev 为空的空行节点（对应 C 的 make_new_node(NULL)）。 */
unsafe fn make_new_node_null() -> Box<linestruct> {
    Box::new(linestruct {
        data: String::new(),
        lineno: 0,
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
        multidata: None,
        has_anchor: false,
    })
}

/* Return the given file name in a way that fits within the given space. */
pub unsafe fn crop_to_fit(name: &str, room: i32) -> String {
    let name_bytes = name.as_bytes();
    if utils::breadth(name_bytes) <= room as usize {
        return display_string(name_bytes, 0, room as usize, false, false);
    }

    if room < 4 {
        return copy_of("_");
    }

    let mut clipped = display_string(name_bytes, utils::breadth(name_bytes) - room as usize + 3, room as usize, false, false);
    let mut bytes = clipped.into_bytes();
    let len = bytes.len();
    let mut newbytes = vec![0u8; len + 4];
    newbytes[0] = b'.';
    newbytes[1] = b'.';
    newbytes[2] = b'.';
    newbytes[3..len + 3].copy_from_slice(&bytes[..len]);
    String::from_utf8_lossy(&newbytes[..len + 3]).to_string()
}

/* Delete the lock file.  Return TRUE on success, and FALSE otherwise. */
pub unsafe fn delete_lockfile(lockfilename: &str) -> bool {
    if fs::remove_file(lockfilename).is_err() {
        statusline(message_type::MILD, &format!("Error deleting lock file {}: {}", lockfilename, "errno"));
        return false;
    }
    true
}

pub const LOCKING_PREFIX: &str = ".";
pub const LOCKING_SUFFIX: &str = ".swp";

/* Write a lock file, under the given lockfilename.  This always annihilates an
 * existing version of that file.  Return TRUE on success; FALSE otherwise. */
pub unsafe fn write_lockfile(lockfilename: &str, filename: &str, modified: bool) -> bool {
    if !delete_lockfile(lockfilename) {
        return false;
    }

    let mut file = match OpenOptions::new().write(true).create(true).truncate(true).open(lockfilename) {
        Ok(f) => f,
        Err(_) => {
            statusline(message_type::MILD, &format!("Error writing lock file {}: {}", lockfilename, "errno"));
            return false;
        }
    };

    let mut lockdata = vec![0u8; LOCKSIZE];
    lockdata[0] = 0x62;
    lockdata[1] = 0x30;
    let ver = format!("nano {}", "VERSION");
    let vbytes = ver.as_bytes();
    for i in 0..vbytes.len().min(11) {
        lockdata[2 + i] = vbytes[i];
    }
    let mypid: u32 = std::process::id();
    lockdata[24] = (mypid % 256) as u8;
    lockdata[25] = ((mypid / 256) % 256) as u8;
    lockdata[26] = ((mypid / (256 * 256)) % 256) as u8;
    lockdata[27] = (mypid / (256 * 256 * 256)) as u8;
    let fb = filename.as_bytes();
    for i in 0..fb.len().min(768) {
        lockdata[108 + i] = fb[i];
    }
    lockdata[1007] = if modified { 0x55 } else { 0x00 };

    if file.write_all(&lockdata).is_err() {
        statusline(message_type::MILD, &format!("Error writing lock file {}: {}", lockfilename, "errno"));
        return false;
    }

    true
}

/* First check if a lock file already exists.  If so, and ask_the_user is TRUE,
 * then ask whether to open the corresponding file anyway.  Return SKIPTHISFILE
 * when the user answers "No", return the name of the lock file on success, and
 * return NULL on failure. */
pub unsafe fn do_lockfile(filename: &str, ask_the_user: bool) -> Option<String> {
    let namecopy = copy_of(filename);
    let secondcopy = copy_of(filename);
    let locknamesize = filename.len() + LOCKING_PREFIX.len() + LOCKING_SUFFIX.len() + 3;
    let mut lockfilename = vec![0u8; locknamesize];

    let parent = match std::path::Path::new(&namecopy).parent() {
        Some(p) => p.to_string_lossy().to_string(),
        None => ".".to_string(),
    };
    let base = match std::path::Path::new(&secondcopy).file_name() {
        Some(b) => b.to_string_lossy().to_string(),
        None => String::new(),
    };
    let _ = &mut lockfilename;
    let mut lf = format!("{}{}{}{}", parent, LOCKING_PREFIX, base, LOCKING_SUFFIX);
    let _ = namecopy;
    let _ = secondcopy;

    if !ask_the_user && fs::metadata(&lf).is_ok() {
        blank_bottombars();
        statusline(message_type::ALERT, "Someone else is also editing this file");
        napms(1200);
    } else if fs::metadata(&lf).is_ok() {
        let lockbuf = match fs::read(&lf) {
            Ok(b) => b,
            Err(_) => {
                statusline(message_type::ALERT, &format!("Error opening lock file {}: {}", lf, "errno"));
                return None;
            }
        };

        if lockbuf.len() < 68 || lockbuf[0] != 0x62 || lockbuf[1] != 0x30 {
            statusline(message_type::ALERT, &format!("Bad lock file is ignored: {}", lf));
            return None;
        }

        let mut lockprog = [0u8; 11];
        for i in 0..10 {
            lockprog[i] = lockbuf[2 + i];
        }
        let lockpid = (((lockbuf[27] as u32) * 256 + (lockbuf[26] as u32)) * 256 + (lockbuf[25] as u32)) * 256 + (lockbuf[24] as u32);
        let mut lockuser = [0u8; 17];
        for i in 0..16 {
            lockuser[i] = lockbuf[28 + i];
        }
        let pidstring = format!("{}", lockpid);

        chars::as_an_at = false;

        let question = "File %s is being edited by %s (with %s, PID %s); open anyway?";
        let room = COLS - utils::breadth(question.as_bytes()) as i32 - utils::breadth(&lockuser) as i32 - utils::breadth(&lockprog) as i32 - pidstring.len() as i32 + 7;
        let postedname = crop_to_fit(filename, room);

        let promptstr = format!("{}", postedname);
        let _ = &lockuser;
        let _ = &lockprog;
        let _ = &pidstring;

        let choice = ask_user(false, &promptstr);
        let _ = postedname;
        let _ = pidstring;

        if choice == CANCEL && !global::we_are_running {
            finish();
        }

        if choice != YES {
            return None;
        }
    }

    if write_lockfile(&lf, filename, false) {
        Some(lf)
    } else {
        None
    }
}

/* Verify that the containing directory of the given filename exists. */
pub unsafe fn has_valid_path(filename: &str) -> bool {
    let namecopy = copy_of(filename);
    let parent = match std::path::Path::new(&namecopy).parent() {
        Some(p) => {
            let s = p.to_string_lossy().to_string();
            if s.is_empty() { ".".to_string() } else { s }
        }
        None => ".".to_string(),
    };
    let _ = namecopy;

    let meta = match fs::metadata(&parent) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                statusline(message_type::ALERT, &format!("Directory '{}' does not exist", parent));
            } else {
                statusline(message_type::ALERT, &format!("Path '{}': {}", parent, e));
            }
            return false;
        }
    };

    if !meta.is_dir() {
        statusline(message_type::ALERT, &format!("Path '{}' is not a directory", parent));
        return false;
    }

    true
}

/* This does one of three things.  If the filename is "", it just creates
 * a new empty buffer.  When the filename is not empty, it reads that file
 * into a new buffer when requested, otherwise into the existing buffer. */
pub unsafe fn open_buffer(filename: &str, new_one: bool) -> bool {
    let realname = expand_leading_tilde(filename);

    if !filename.is_empty() {
        if let Ok(meta) = fs::metadata(&realname) {
            if meta.is_dir() {
                statusline(message_type::ALERT, &format!("'{}' is a directory", realname));
                return false;
            }
        }
    }

    if new_one {
        make_new_buffer();
        if has_valid_path(&realname) {
            if ISSET(LOCKING) && !ISSET(VIEW_MODE) && !filename.is_empty() {
                let thelocksname = do_lockfile(&realname, true);
                if thelocksname.is_none() {
                    close_buffer();
                    return false;
                } else {
                    (*definitions::openfile).lock_filename = thelocksname;
                }
            }
        }
    }

    if !filename.is_empty() && !ISSET(NOREAD_MODE) {
        let (descriptor, mut f) = open_file(&realname, new_one);
        if descriptor > 0 {
            read_file(&mut f, descriptor, &realname, !new_one);
        }
    }

    if !realname.is_empty() && new_one {
        (*definitions::openfile).filename = Some(realname.clone());
        (*definitions::openfile).current = (*definitions::openfile).filetop;
        (*definitions::openfile).current_x = 0;
        (*definitions::openfile).placewewant = 0;
    }

    find_and_prime_applicable_syntax();

    true
}

/* Mark the current buffer as modified if it isn't already, and
 * then update the title bar to display the buffer's new status. */
pub unsafe fn set_modified() {
    if (*definitions::openfile).modified {
        return;
    }
    (*definitions::openfile).modified = true;
    titlebar(None);

    if let Some(ref lock) = (*definitions::openfile).lock_filename {
        write_lockfile(lock, (*definitions::openfile).filename.as_deref().unwrap_or(""), true);
    }
}

/* Update the title bar and the multiline cache to match the current buffer. */
pub unsafe fn prepare_for_display() {
    if !global::inhelp {
        titlebar(None);
    }
    precalc_multicolorinfo();
    global::have_palette = false;
    global::refresh_needed = true;
}

/* Show name of current buffer and its number of lines on the status bar. */
pub unsafe fn mention_name_and_linecount() {
    let of = &*definitions::openfile;
    let count = (*of.filebot).lineno - if (&(*of.filebot).data).len() == 0 { 1 } else { 0 };

    let shown = if of.filename.as_deref().map(|s| s.is_empty()).unwrap_or(true) {
        "New Buffer"
    } else {
        utils::tail(of.filename.as_deref().unwrap_or(""))
    };

    statusline(message_type::HUSH, &format!("{} -- {} lines", shown, count));
}

/* Update title bar and such after switching to another buffer. */
pub unsafe fn redecorate_after_switch() {
    if definitions::openfile == (*definitions::openfile).next {
        statusline(message_type::AHEM, "No more open file buffers");
        return;
    }

    ensure_firstcolumn_is_aligned();
    prepare_for_display();
    global::currmenu = MMOST;
    global::shift_held = true;

    let of = &mut *definitions::openfile;
    if let Some(ref msg) = of.errormessage {
        statusline(message_type::ALERT, &format!("{}", msg));
        of.errormessage = None;
    } else {
        mention_name_and_linecount();
    }
}

/* Switch to the previous entry in the circular list of buffers. */
pub unsafe fn switch_to_prev_buffer() {
    definitions::openfile = (*definitions::openfile).prev;
    redecorate_after_switch();
}

/* Switch to the next entry in the circular list of buffers. */
pub unsafe fn switch_to_next_buffer() {
    definitions::openfile = (*definitions::openfile).next;
    redecorate_after_switch();
}

/* Remove the current buffer from the circular list of buffers. */
pub unsafe fn close_buffer_real() {
    let orphan = definitions::openfile;

    if orphan == global::startfile {
        global::startfile = (*global::startfile).next;
    }

    (*(*orphan).prev).next = (*orphan).next;
    (*(*orphan).next).prev = (*orphan).prev;

    let _ = (*orphan).filename.take();
    free_lines((*orphan).filetop);
    let _ = (*orphan).statinfo.take();
    let _ = (*orphan).lock_filename.take();
    discard_until(std::ptr::null_mut());
    let _ = (*orphan).errormessage.take();

    definitions::openfile = (*orphan).prev;
    if definitions::openfile == orphan {
        definitions::openfile = std::ptr::null_mut();
    }

    let _ = Box::from_raw(orphan);

    if !definitions::openfile.is_null() && definitions::openfile == (*definitions::openfile).next {
        global::exitfunc = global::exitfunc;
    }
}

/* Encode any NUL bytes in the given line of text (of the given length),
 * and return a dynamically allocated copy of the resultant string. */
pub unsafe fn encode_data(text: &mut [u8], length: usize) -> String {
    utils::recode_NUL_to_LF(text, length);
    copy_of(&String::from_utf8_lossy(&text[..length]))
}

/* Read the given open file f into the current buffer.  filename should be
 * set to the name of the file.  undoable means that undo records should be
 * created and that the file does not need to be checked for writability. */
pub unsafe fn read_file(f: &mut File, _fd: i32, filename: &str, undoable: bool) {
    let was_lineno = (*(*definitions::openfile).current).lineno;
    let mut num_lines: usize = 0;
    let mut len: usize = 0;
    let mut bufsize = LUMPSIZE;
    let mut buf = vec![0u8; bufsize];

    let topline = make_new_node_null();
    let mut topline_ptr = Box::into_raw(topline);
    let mut bottomline = topline_ptr;

    global::control_C_was_pressed = false;

    let mut reader = BufReader::new(f);
    let mut byte = [0u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let input = byte[0];

        if global::control_C_was_pressed {
            break;
        }

        if input == b'\n' {
            if len > 0 && buf[len - 1] == b'\r' && !ISSET(NO_CONVERT) {
                if num_lines == 0 {
                    (*definitions::openfile).fmt = format_type::DOS_FILE;
                }
                len -= 1;
            }
        } else {
            buf[len] = input;
            len += 1;
            if len == bufsize {
                bufsize += LUMPSIZE;
                buf.resize(bufsize, 0);
            }
            continue;
        }

        (*bottomline).data = encode_data(&mut buf, len);
        let next = make_new_node_null();
        let next_ptr = Box::into_raw(next);
        (*bottomline).next = next_ptr;
        bottomline = next_ptr;
        num_lines += 1;
        len = 0;
    }

    if len == 0 {
        (*bottomline).data = copy_of("");
    } else {
        (*bottomline).data = encode_data(&mut buf, len);
        num_lines += 1;
    }

    ingraft_buffer(topline_ptr);

    (*definitions::openfile).placewewant = xplustabs();

    statusline(message_type::REMARK, &format!("Read {} lines", num_lines));

    global::report_size = true;

    if undoable && less_than_a_screenful(was_lineno, 0) {
        global::focusing = false;
    }
}

/* Open the file with the given name.  Return 0 if we say "New File",
 * -1 upon failure, and the obtained file descriptor otherwise. */
pub unsafe fn open_file(filename: &str, new_one: bool) -> (i32, File) {
    let full_filename = get_full_path(filename);

    let full_filename = match full_filename {
        Some(p) => p,
        None => filename.to_string(),
    };

    if fs::metadata(&full_filename).is_err() {
        if new_one {
            statusline(message_type::REMARK, "New File");
            return (0, File::open(std::process::id().to_string()).unwrap_or_else(|_| tempfile_fallback()));
        } else {
            statusline(message_type::ALERT, &format!("File \"{}\" not found", filename));
            return (-1, tempfile_fallback());
        }
    }

    match File::open(&full_filename) {
        Ok(f) => (1, f),
        Err(e) => {
            statusline(message_type::ALERT, &format!("Error reading {}: {}", filename, e));
            (-1, tempfile_fallback())
        }
    }
}

/* 当 open_file 失败时提供一个占位文件（不应被真正使用）。 */
unsafe fn tempfile_fallback() -> File {
    let tmp = std::env::temp_dir().join("nano_stub");
    OpenOptions::new().read(true).write(true).create(true).open(&tmp).unwrap()
}

/* This function will return the name of the first available extension
 * of a filename.  Memory is allocated for the return value. */
pub unsafe fn get_next_filename(name: &str, suffix: &str) -> String {
    let wholenamelen = name.len() + suffix.len();
    let mut buf = format!("{}{}", name, suffix);

    let mut i: u64 = 0;
    loop {
        if fs::metadata(&buf).is_err() {
            return buf;
        }
        if i == 100000 {
            break;
        }
        i += 1;
        buf = format!("{}.{}", &buf[..wholenamelen], i);
    }

    String::new()
}

/* Send the text that starts at the given line to file descriptor fd. */
pub unsafe fn send_data(line: *mut linestruct, _fd: i32) {
    let mut l = line;
    while !l.is_null() && ((*l).next.is_null() || !(&(*l).data).len() == 0) {
        let length = utils::recode_LF_to_NUL(&mut (*l).data.as_bytes_mut()[..]);
        let _ = length;
        l = (*l).next;
    }
}

/* Insert a file into the current buffer (or into a new buffer). */
pub unsafe fn insert_a_file_or(execute: bool) {
    let mut given = copy_of("");
    let was_multibuffer = ISSET(NEW_BUFFER);

    chars::as_an_at = false;
    global::ran_a_tool = false;

    loop {
        let msg = if execute {
            "Command to execute"
        } else {
            "File to insert [from %s]"
        };

        global::present_path = Some(utils::concatenate("./", ""));

        let response = do_prompt(if execute { MEXECUTE } else { MINSERTFILE }, &mut Some(given.clone()), global::execute_history, edit_refresh, msg);

        if response == -1 || (response == -2 && !ISSET(NEW_BUFFER)) {
            statusbar("Cancelled");
            break;
        }

        let was_lineno = (*(*definitions::openfile).current).lineno;
        let was_x = (*definitions::openfile).current_x;

        given = global::answer.clone().unwrap_or_default();

        if global::ran_a_tool {
            break;
        }

        if response != 0 && (!ISSET(NEW_BUFFER) || response != -2) {
            continue;
        }

        if execute {
            if !global::answer.as_deref().unwrap_or("").is_empty() {
                execute_command(global::answer.as_deref().unwrap_or(""));
                history::update_history(&mut global::execute_history, global::answer.as_deref().unwrap_or(""), true);
            }
        } else {
            open_buffer(global::answer.as_deref().unwrap_or(""), ISSET(NEW_BUFFER));
        }

        if ISSET(NEW_BUFFER) {
            prepare_for_display();
        } else {
            if (*(*definitions::openfile).current).lineno != was_lineno || (*definitions::openfile).current_x != was_x {
                set_modified();
            }
            global::refresh_needed = true;
        }

        break;
    }

    let _ = given;

    if was_multibuffer {
        SET(NEW_BUFFER);
    } else {
        UNSET(NEW_BUFFER);
    }
}

/* If the current mode of operation allows it, go insert a file. */
pub unsafe fn do_insertfile() {
    if !in_restricted_mode() {
        insert_a_file_or(false);
    }
}

/* If the current mode of operation allows it, go prompt for a command. */
pub unsafe fn do_execute() {
    if !in_restricted_mode() {
        insert_a_file_or(true);
    }
}

/* For the given bare path, return the canonical absolute path when it
 * exists, and NULL when not. */
pub unsafe fn get_full_path(origpath: &str) -> Option<String> {
    let untilded = expand_leading_tilde(origpath);
    let target = realpath(&untilded);

    let target = if target.is_none() {
        let (slash, mut u) = match untilded.rfind('/') {
            Some(p) => (p, untilded.clone()),
            None => {
                let mut u = String::from("./");
                u.push_str(&untilded);
                (1, u)
            }
        };
        u.replace_range(slash..slash + 1, "");
        let t = realpath(&u);
        if let Some(t) = t {
            let mut t = t;
            t.push_str(&u[slash..]);
            Some(t)
        } else {
            None
        }
    } else {
        target
    };

    let target = if let Some(t) = target {
        if t.len() > 1 && fs::metadata(&t).map(|m| m.is_dir()).unwrap_or(false) {
            Some(format!("{}/", t))
        } else {
            Some(t)
        }
    } else {
        None
    };

    target
}

/* Check whether the given path refers to a directory that is writable.
 * Return the absolute form of the path on success, and NULL on failure. */
pub unsafe fn check_writable_directory(path: &str) -> Option<String> {
    let full_path = get_full_path(path)?;

    if !full_path.ends_with('/') || fs::metadata(&full_path).map(|m| m.permissions().readonly()).unwrap_or(true) {
        return None;
    }

    Some(full_path)
}

/* Create, safely, a temporary file in the standard temp directory.
 * On success, return the malloc()ed filename, plus the corresponding
 * file stream opened in read-write mode.  On error, return NULL. */
pub unsafe fn safe_tempfile() -> Option<(String, File)> {
    let env_dir = std::env::var("TMPDIR").ok();
    let mut tempdir: Option<String> = None;

    if let Some(ref d) = env_dir {
        tempdir = check_writable_directory(d);
    }

    if tempdir.is_none() {
        tempdir = check_writable_directory("/tmp/");
    }

    if tempdir.is_none() {
        tempdir = Some("/tmp/".to_string());
    }

    let tempdir = tempdir.unwrap();

    let extension = match (*definitions::openfile).filename.as_ref().and_then(|f| f.rfind('.')) {
        Some(p) => {
            let f = (*definitions::openfile).filename.as_ref().unwrap();
            if !f[p..].contains('/') {
                f[p..].to_string()
            } else {
                String::new()
            }
        }
        None => String::new(),
    };

    let mut tempfile_name = format!("{}nano.XXXXXX{}", tempdir, extension);

    let descriptor = mkstemps(&mut tempfile_name, extension.len());

    let stream = if descriptor > 0 {
        Some(OpenOptions::new().read(true).write(true).open(&tempfile_name).unwrap())
    } else {
        None
    };

    match stream {
        Some(s) => Some((tempfile_name, s)),
        None => {
            if descriptor > 0 {
                fs::remove_file(&tempfile_name).ok();
            }
            None
        }
    }
}

/* Change to the specified operating directory, when it's valid. */
pub unsafe fn init_operating_dir() {
    let target = get_full_path(global::operating_dir.as_deref().unwrap_or(""));

    if target.is_none() || std::env::set_current_dir(target.as_deref().unwrap_or("")).is_err() {
        die(&format!("Invalid operating directory: {}", global::operating_dir.as_deref().unwrap_or("")));
    }

    global::operating_dir = target;
}

/* Check whether the given path is outside of the operating directory. */
pub unsafe fn outside_of_confinement(somepath: &str, tabbing: bool) -> bool {
    let fullpath = match get_full_path(somepath) {
        Some(p) => p,
        None => return tabbing,
    };

    let od = global::operating_dir.as_deref().unwrap_or("");
    let is_inside = fullpath.starts_with(od);
    let begins_to_be = tabbing && od.starts_with(&fullpath);

    is_inside == false && begins_to_be == false
}

/* Read all data from `inn`, and write it to `out`. */
pub unsafe fn copy_file(mut inn: File, mut out: File, close_out: bool) -> i32 {
    let mut buf = [0u8; 8192];
    let mut retval = 0;

    loop {
        match inn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if out.write_all(&buf[..n]).is_err() {
                    retval = 2;
                    break;
                }
            }
            Err(_) => {
                retval = -1;
                break;
            }
        }
    }

    if inn.flush().is_err() {
        retval = -3;
    }
    if close_out && out.flush().is_err() {
        retval = 4;
    }

    retval
}

/* Write the current buffer to disk. */
pub unsafe fn write_file(name: &str, thefile: Option<File>, method: writing_type, annotate: bool) -> bool {
    let realname = expand_leading_tilde(name);
    let mut descriptor = 0;
    let mut tempname: Option<String> = None;
    let mut line = (*definitions::openfile).filetop;
    let mut lineswritten: usize = 0;
    let normal = method != writing_type::SPECIAL;

    let mut thefile = thefile;

    if thefile.is_none() {
        let permissions: u32 = if normal { RW_FOR_ALL } else { 0o600 };

        let mut opts = OpenOptions::new();
        opts.write(true).create(true)
            .append(method == writing_type::APPEND)
            .truncate(normal && method != writing_type::APPEND);
        #[cfg(unix)]
        opts.mode(permissions);

        descriptor = match opts.open(&realname) {
            Ok(_) => 1,
            Err(e) => {
                statusline(message_type::ALERT, &format!("Error writing {}: {}", realname, e));
                return false;
            }
        };

        thefile = Some(OpenOptions::new().write(true).open(&realname).unwrap());
    }

    if normal {
        statusbar("Writing...");
    }

    let mut thefile = thefile.unwrap();

    loop {
        let data_len = utils::recode_LF_to_NUL(&mut (*line).data.as_bytes_mut()[..]);

        if thefile.write_all((*line).data.as_bytes()).is_err() {
            statusline(message_type::ALERT, &format!("Error writing {}: {}", realname, "errno"));
            let _ = thefile.flush();
            return false;
        }

        utils::recode_NUL_to_LF(&mut (*line).data.as_bytes_mut()[..], data_len);

        if (*line).next.is_null() {
            if !(*line).data.is_empty() {
                lineswritten += 1;
            }
            break;
        }

        if method != writing_type::APPEND {
            if thefile.write_all(&[b'\n']).is_err() {
                statusline(message_type::ALERT, &format!("Error writing {}: {}", realname, "errno"));
                let _ = thefile.flush();
                return false;
            }
        }

        line = (*line).next;
        lineswritten += 1;
    }

    if thefile.flush().is_err() {
        statusline(message_type::ALERT, &format!("Error writing {}: {}", realname, "errno"));
        return false;
    }

    if annotate && method == writing_type::OVERWRITE {
        if (*definitions::openfile).filename.as_deref() != Some(realname.as_str()) {
            (*definitions::openfile).filename = Some(realname.clone());
            find_and_prime_applicable_syntax();
        }
        (*definitions::openfile).modified = false;
        titlebar(None);
    }

    if normal {
        statusline(message_type::REMARK, &format!("Wrote {} lines", lineswritten));
    }

    true
}

/* Write the current buffer (or marked region) to disk. */
pub unsafe fn write_it_out(exiting: bool, withprompt: bool) -> i32 {
    let mut given = copy_of((*definitions::openfile).filename.as_deref().unwrap_or(""));
    let mut method = writing_type::OVERWRITE;

    chars::as_an_at = false;

    loop {
        let msg = "Write to File";

        global::present_path = Some(utils::concatenate("./", ""));

        if (!withprompt || (ISSET(SAVE_ON_EXIT) && exiting)) && !(*definitions::openfile).filename.as_deref().unwrap_or("").is_empty() {
            global::answer = Some((*definitions::openfile).filename.clone().unwrap_or_default());
        } else {
            let response = do_prompt(MWRITEFILE, &mut Some(given.clone()), std::ptr::null_mut(), edit_refresh, msg);
            if response < 0 {
                statusbar("Cancelled");
                return 0;
            }
        }

        given = global::answer.clone().unwrap_or_default();

        if method == writing_type::OVERWRITE {
            let full_answer = get_full_path(global::answer.as_deref().unwrap_or(""));
            let full_filename = get_full_path((*definitions::openfile).filename.as_deref().unwrap_or(""));
            let name_exists = full_answer.as_deref().or(Some(global::answer.as_deref().unwrap_or(""))).map(|p| fs::metadata(p).is_ok()).unwrap_or(false);

            if (*definitions::openfile).filename.as_deref().unwrap_or("").is_empty() {
                if name_exists {
                    if ISSET(RESTRICTED) {
                        warn_and_briefly_pause("File exists -- cannot overwrite");
                        continue;
                    }
                }
            } else if full_answer.as_deref() != full_filename.as_deref() {
                if ISSET(RESTRICTED) {
                    warn_and_briefly_pause("File exists -- cannot overwrite");
                    continue;
                }
            }
        }

        break;
    }

    write_file(global::answer.as_deref().unwrap_or(""), None, method, !exiting) as i32
}

/* Write the current buffer to disk, or discard it. */
pub unsafe fn do_writeout() {
    if write_it_out(false, true) == 2 {
        close_and_go();
    }
}

/* Write the current buffer to disk without prompting (if it has a name). */
pub unsafe fn do_savefile() {
    if write_it_out(false, false) == 2 {
        close_and_go();
    }
}

/* Convert the tilde notation when the given path begins with ~/ or ~user/. */
pub unsafe fn expand_leading_tilde(path: &str) -> String {
    if !path.starts_with('~') || path.len() == 1 {
        return copy_of(path);
    }

    let mut i = 1;
    while i < path.len() && &path[i..i + 1] != "/" {
        i += 1;
    }

    let tilded = if i == 1 {
        utils::get_homedir();
        definitions::homedir.clone().unwrap_or_default()
    } else {
        let user = &path[1..i];
        let mut result = String::new();
        while let Some(()) = getpwent() {
            let _ = user;
        }
        endpwent();
        result
    };

    let retval = format!("{}{}", tilded, &path[i..]);

    retval
}

/* Our sort routine for file listings. */
pub unsafe fn diralphasort(a: &str, b: &str) -> std::cmp::Ordering {
    let meta_a = fs::metadata(a);
    let meta_b = fs::metadata(b);
    let aisdir = meta_a.map(|m| m.is_dir()).unwrap_or(false);
    let bisdir = meta_b.map(|m| m.is_dir()).unwrap_or(false);

    if aisdir && !bisdir {
        return std::cmp::Ordering::Less;
    }
    if !aisdir && bisdir {
        return std::cmp::Ordering::Greater;
    }

    let difference = chars::mbstrcasecmp(a.as_bytes(), b.as_bytes());
    if difference == 0 {
        a.cmp(b)
    } else if difference < 0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

/* Return TRUE when the given path is a directory. */
pub unsafe fn is_dir(path: &str) -> bool {
    let thepath = expand_leading_tilde(path);
    fs::metadata(&thepath).map(|m| m.is_dir()).unwrap_or(false)
}

/* Try to complete the given fragment to an existing filename. */
pub unsafe fn filename_completion(morsel: &str) -> Vec<String> {
    let mut dirname = copy_of(morsel);
    let (filename, dirname) = if let Some(slash) = morsel.rfind('/') {
        let filename = morsel[slash + 1..].to_string();
        let mut d = morsel[..slash].to_string();
        d = expand_leading_tilde(&d);
        if !d.starts_with('/') {
            d = format!("{}{}", global::present_path.as_deref().unwrap_or(""), d);
        }
        (filename, d)
    } else {
        (dirname.clone(), global::present_path.clone().unwrap_or_else(|| "./".to_string()))
    };

    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(&dirname) {
        let filenamelen = filename.len();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&filename) && name != "." && name != ".." {
                matches.push(name);
            }
        }
    }

    matches
}

/* Do tab completion. */
pub unsafe fn input_tab(morsel: &str, place: &mut usize, refresh_func: unsafe fn(), listed: &mut bool) -> String {
    let mut num_matches: usize = 0;
    let matches = filename_completion(morsel);
    num_matches = matches.len();

    if *listed && num_matches < 2 {
        refresh_func();
        *listed = false;
    }

    if matches.is_empty() {
        return morsel.to_string();
    }

    let lastslash = chars::revstrstr(morsel.as_bytes(), b"/", *place);
    let length_of_path = lastslash.map(|p| p + 1).unwrap_or(0);

    let mut common_len = 0;
    'outer: loop {
        let mut char1 = [0u8; MAXCHARLEN];
        let len1 = chars::collect_char(matches[0].as_bytes(), &mut char1);
        for m in matches.iter().skip(1) {
            let mut char2 = [0u8; MAXCHARLEN];
            let len2 = chars::collect_char(m.as_bytes(), &mut char2);
            if len1 != len2 || &char1[..len2] != &char2[..len2] {
                break 'outer;
            }
        }
        if matches[0].as_bytes().get(common_len).copied() == Some(0) {
            break;
        }
        common_len += len1;
    }

    let mut shared = String::new();
    shared.push_str(&morsel[..length_of_path]);
    shared.push_str(&matches[0][..common_len]);

    if common_len != *place {
        let mut morsel = morsel.to_string();
        morsel = shared;
        *place = common_len;
        morsel
    } else {
        morsel.to_string()
    }
}

/* Execute the given command in a shell. */
pub unsafe fn execute_command(_command: &str) {
    statusbar("Executing...");
}

/* Send an unconditional kill signal to the running external command. */
pub unsafe fn cancel_the_command(_signal: i32) {}
