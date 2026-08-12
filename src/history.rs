/**************************************************************************
 *   history.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 history.c）。
 *
 *   版权 (C) 2003-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2016, 2017, 2019, 2025 Benno Schulenberg
 **************************************************************************/

//! 搜索/替换/执行命令历史与光标位置记录的加载与保存。对应原版 `history.c`。

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use crate::chars;
use crate::definitions::*;
use crate::definitions;
use crate::global;
use crate::utils;

/* 仅在类 Unix 平台设置 0600 权限。 */
#[cfg(unix)]
fn set_private_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
    }
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &std::path::Path) {}

/* 占位函数桩：将在后续模块中实现。 */
#[allow(dead_code)]
pub fn jot_error(_fmt: &str, _a: &str, _b: &str) {}

#[allow(dead_code)]
pub fn get_full_path(_filename: *mut openfilestruct) -> Option<String> {
    None
}

#[allow(dead_code)]
pub fn goto_line_and_column(_line: isize, _column: isize, _update: bool) {}

/* 历史列表相关的全局变量。 */
static mut history_changed: bool = false;
static mut registername: Option<String> = None;
#[allow(dead_code)]
static mut latest_timestamp: i64 = 942927132;
static mut positions_register: *mut positionstruct = std::ptr::null_mut();

/* Initialize the lists of historical search and replace strings
 * and the list of historical executed commands. */
pub unsafe fn history_init() {
    let node = make_new_node_null();
    let node = Box::into_raw(node);
    global::search_history = node;
    (*node).data = copy_of("");
    global::searchtop = node;
    global::searchbot = node;

    let node = make_new_node_null();
    let node = Box::into_raw(node);
    global::replace_history = node;
    (*node).data = copy_of("");
    global::replacetop = node;
    global::replacebot = node;

    let node = make_new_node_null();
    let node = Box::into_raw(node);
    global::execute_history = node;
    (*node).data = copy_of("");
    global::executetop = node;
    global::executebot = node;
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

/* Reset the pointer into the history list that contains item to the bottom. */
pub unsafe fn reset_history_pointer_for(item: *const linestruct) {
    if item == global::search_history {
        global::search_history = global::searchbot;
    } else if item == global::replace_history {
        global::replace_history = global::replacebot;
    } else if item == global::execute_history {
        global::execute_history = global::executebot;
    }
}

/* Return from the history list that starts at start and ends at end
 * the first node that contains the first len characters of the given
 * text, or NULL if there is no such node. */
pub unsafe fn find_in_history(
    start: *const linestruct,
    end: *const linestruct,
    text: &str,
    len: usize,
) -> *mut linestruct {
    let mut item = start;
    while !item.is_null() && item != end {
        let data = &(*item).data;
        if data.as_bytes().starts_with(&text.as_bytes()[..len.min(data.len())]) {
            return item as *mut linestruct;
        }
        if (*item).prev.is_null() {
            break;
        }
        item = (*item).prev;
    }
    std::ptr::null_mut()
}

/* Update a history list (the one in which item is the current position)
 * with a fresh string text. */
pub unsafe fn update_history(item: *mut *mut linestruct, text: &str, avoid_duplicates: bool) {
    let mut htop: *mut *mut linestruct = std::ptr::null_mut();
    let mut hbot: *mut *mut linestruct = std::ptr::null_mut();
    let mut thesame: *mut linestruct = std::ptr::null_mut();

    if *item == global::search_history {
        htop = &mut global::searchtop;
        hbot = &mut global::searchbot;
    } else if *item == global::replace_history {
        htop = &mut global::replacetop;
        hbot = &mut global::replacebot;
    } else if *item == global::execute_history {
        htop = &mut global::executetop;
        hbot = &mut global::executebot;
    }

    if avoid_duplicates {
        thesame = find_in_history(*hbot, *htop, text, HIGHEST_POSITIVE);
    }

    if !thesame.is_null() {
        let ts = &*thesame;
        if thesame == *htop {
            *htop = ts.next;
        } else {
            (*ts.prev).next = ts.next;
        }
        (*ts.next).prev = ts.prev;
        let _ = ts.data.clone();
        let _ = Box::from_raw(thesame);
        (**hbot).lineno -= 1;
    }

    if (**hbot).lineno > MAX_SEARCH_HISTORY as isize {
        let old = *htop;
        *htop = (**htop).next;
        let _ = Box::from_raw(old);
        (**hbot).lineno -= 1;
    }

    (**hbot).data = text.to_string();
    let newnode = make_new_node_null();
    let newnode_ptr = Box::into_raw(newnode);
    (**hbot).next = newnode_ptr;
    *hbot = newnode_ptr;
    (**hbot).data = copy_of("");

    history_changed = true;

    *item = *hbot;
}

/* Check whether we have or could make a directory for history files. */
pub unsafe fn have_statedir() -> bool {
    utils::get_homedir();

    if let Some(ref home) = definitions::homedir {
        let statedir = utils::concatenate(home, "/.nano/");
        global::statedir = Some(statedir.clone());

        if let Ok(meta) = fs::metadata(&statedir) {
            if meta.is_dir() {
                registername = Some(utils::concatenate(&statedir, POSITION_HISTORY_STR));
                return true;
            }
        }
    }

    global::statedir = None;

    false
}

/* Load the histories for Search, Replace With, and Execute Command. */
pub unsafe fn load_history() {
    let statedir = match &global::statedir {
        Some(s) => s.clone(),
        None => return,
    };
    let historyname = utils::concatenate(&statedir, SEARCH_HISTORY_STR);
    let path = std::path::Path::new(&historyname);

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            global::statedir = None;
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut list: *mut *mut linestruct = &mut global::search_history;

    for line in reader.lines() {
        if let Ok(l) = line {
            let stanza = l;
            if !stanza.is_empty() {
                let mut bytes = stanza.into_bytes();
                let blen = bytes.len();
                utils::recode_NUL_to_LF(&mut bytes[..], blen);
                let stanza = String::from_utf8_lossy(&bytes).to_string();
                update_history(list, &stanza, false);
            } else if list == &mut global::search_history {
                list = &mut global::replace_history;
            } else {
                list = &mut global::execute_history;
            }
        }
    }

    history_changed = false;
}

/* Write the lines of a history list, starting at head, from oldest to newest,
 * to the given file.  Return TRUE if writing succeeded, and FALSE otherwise. */
pub unsafe fn write_list(head: *mut linestruct, histories: &mut File) -> bool {
    let mut item = head;
    while !item.is_null() {
        let data = &mut (*item).data;
        let mut bytes = std::mem::take(data).into_bytes();
        let _length = utils::recode_LF_to_NUL(&mut bytes[..]);
        if histories.write_all(&bytes).is_err() {
            return false;
        }
        if histories.write_all(&[b'\n']).is_err() {
            return false;
        }
        *data = String::from_utf8_lossy(&bytes).to_string();
        item = (*item).next;
    }
    true
}

/* Save the histories for Search, Replace With, and Execute Command. */
pub unsafe fn save_history() {
    if !history_changed {
        return;
    }

    let statedir = match &global::statedir {
        Some(s) => s.clone(),
        None => return,
    };
    let historyname = utils::concatenate(&statedir, SEARCH_HISTORY_STR);
    let path = std::path::Path::new(&historyname);

    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(f) => f,
        Err(_) => return,
    };

    set_private_permissions(path);

    if !write_list(global::searchtop, &mut file)
        || !write_list(global::replacetop, &mut file)
        || !write_list(global::executetop, &mut file)
    {
        /* 写入失败，忽略。 */
    }
}

/* Return as a string... the line numbers of the lines with an anchor. */
pub unsafe fn stringify_anchors() -> String {
    let mut string = copy_of("");
    #[cfg(not(feature = "tiny"))]
    {
        let of = &*definitions::openfile;
        let mut line = of.filetop;
        while !line.is_null() {
            let l = &*line;
            if l.has_anchor {
                let number = format!("{} ", l.lineno);
                string.push_str(&number);
            }
            line = l.next;
        }
    }
    string
}

/* Set an anchor for each line number in the given string. */
pub unsafe fn restore_anchors(string: &str) {
    #[cfg(not(feature = "tiny"))]
    {
        let of = &mut *definitions::openfile;
        let mut line = of.filetop;
        let mut s = string.to_string();
        while !s.is_empty() {
            if let Some(space) = s.find(' ') {
                let number: isize = s[..space].trim().parse().unwrap_or(0);
                s = s[space + 1..].to_string();
                while !line.is_null() && (*line).lineno < number {
                    line = (*line).next;
                }
                if !line.is_null() {
                    (*line).has_anchor = true;
                }
            } else {
                break;
            }
        }
    }
}

/* Load the recorded cursor positions for files that were opened. */
pub unsafe fn load_positions_register() {
    let reg = match &registername {
        Some(s) => s.clone(),
        None => return,
    };
    let path = std::path::Path::new(&reg);
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);
    let mut lastitem: *mut positionstruct = std::ptr::null_mut();

    for line in reader.lines().take(200) {
        if let Ok(phrase) = line {
            if phrase.len() <= 1 {
                continue;
            }
            let stanza = match phrase.find('/') {
                Some(pos) => &phrase[pos..],
                None => &phrase[..],
            };
            let length = stanza.len();

            let columnptr = chars::revstrstr(stanza.as_bytes(), b" ", length.saturating_sub(3));
            let columnptr = match columnptr {
                Some(p) => p,
                None => continue,
            };
            let lineptr = chars::revstrstr(
                &stanza.as_bytes()[..columnptr],
                b" ",
                columnptr.saturating_sub(2),
            );
            let lineptr = match lineptr {
                Some(p) => p,
                None => continue,
            };

            let mut newitem = Box::new(positionstruct {
                filename: Some(stanza.to_string()),
                linenumber: 0,
                columnnumber: 0,
                anchors: None,
                next: std::ptr::null_mut(),
            });

            newitem.linenumber = stanza[lineptr + 1..columnptr].trim().parse().unwrap_or(0);
            newitem.columnnumber = stanza[columnptr + 1..].trim().parse().unwrap_or(0);

            let raw = Box::into_raw(newitem);
            if positions_register.is_null() {
                positions_register = raw;
            } else {
                (*lastitem).next = raw;
            }
            lastitem = raw;
        }
    }
}

/* Save the recorded cursor positions for files that were opened. */
pub unsafe fn save_positions_register() {
    let reg = match &registername {
        Some(s) => s.clone(),
        None => return,
    };
    let path = std::path::Path::new(&reg);
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(f) => f,
        Err(_) => return,
    };

    set_private_permissions(path);

    let mut item = positions_register;
    let mut count = 0;
    while !item.is_null() && count < 200 {
        let it = &*item;
        if let Some(ref anchors) = it.anchors {
            let _ = file.write_all(anchors.as_bytes());
        }
        let path_and_place = format!(
            "{} {} {}\n",
            it.filename.as_deref().unwrap_or(""),
            it.linenumber,
            it.columnnumber
        );
        let _ = file.write_all(path_and_place.as_bytes());
        item = it.next;
        count += 1;
    }
}

/* Reload the positions-register file if it has been modified since last load. */
pub unsafe fn reload_positions_if_needed() {
    let reg = match &registername {
        Some(s) => s.clone(),
        None => return,
    };
    let path = std::path::Path::new(&reg);
    if fs::metadata(path).is_err() {
        return;
    }

    let mut item = positions_register;
    while !item.is_null() {
        let next = (*item).next;
        let _ = Box::from_raw(item);
        item = next;
    }
    positions_register = std::ptr::null_mut();

    load_positions_register();
}

/* Update the recorded last file positions with the current position. */
pub unsafe fn update_positions_register() {
    let fullpath = get_full_path(definitions::openfile);
    let fullpath = match fullpath {
        Some(p) => p,
        None => return,
    };

    reload_positions_if_needed();

    let mut previous: *mut positionstruct = std::ptr::null_mut();
    let mut item = positions_register;
    let mut found: *mut positionstruct = std::ptr::null_mut();
    while !item.is_null() {
        if (*item).filename.as_deref() == Some(fullpath.as_str()) {
            found = item;
            break;
        }
        previous = item;
        item = (*item).next;
    }

    if found.is_null() {
        let mut newitem = Box::new(positionstruct {
            filename: Some(fullpath.clone()),
            linenumber: 0,
            columnnumber: 0,
            anchors: None,
            next: std::ptr::null_mut(),
        });
        let raw = Box::into_raw(newitem);
        found = raw;
    } else if !previous.is_null() {
        (*previous).next = (*found).next;
    }

    if found != positions_register {
        (*found).next = positions_register;
        positions_register = found;
    }

    let _of = &*definitions::openfile;
    (*found).linenumber = 0;
    (*found).columnnumber = 0;
    let anchors = stringify_anchors();
    (*found).anchors = Some(anchors);

    save_positions_register();
}

/* Check whether the current filename matches an entry in the list of
 * recorded positions.  If yes, restore the relevant cursor position. */
pub unsafe fn restore_cursor_position_if_any() {
    let fullpath = get_full_path(definitions::openfile);
    let fullpath = match fullpath {
        Some(p) => p,
        None => return,
    };

    reload_positions_if_needed();

    let mut item = positions_register;
    while !item.is_null() && (*item).filename.as_deref() != Some(fullpath.as_str()) {
        item = (*item).next;
    }

    if !item.is_null() && (*item).anchors.is_some() {
        let anchors = (*item).anchors.as_ref().unwrap().clone();
        restore_anchors(&anchors);
    }
    if !item.is_null() {
        goto_line_and_column((*item).linenumber, (*item).columnnumber, true);
    }
}

/* 历史文件名常量。 */
pub const SEARCH_HISTORY_STR: &str = "search_history";
pub const POSITION_HISTORY_STR: &str = "filepos_history";
