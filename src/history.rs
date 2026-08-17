/**************************************************************************
 * history.rs  --  GNU nano 历史记录管理（对应 history.c）
 * 版权 (C) 2003-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 搜索/替换/执行历史与文件位置记录，完整移植自 `history.c`。
//!
//! 转换说明：
//! - C 的 `linestruct *` 历史链表 → `Rc<RefCell<LineStruct>>` 双向链表；
//! - `search_history`/`searchtop`/`searchbot` 等指针变量放入 [`GlobalState`]；
//! - `getline` 逐行读取 → `BufReader::read_until`；`fopen/fwrite` → `std::fs`；
//! - `stat`/`mkdir`/`chmod` → `std::fs`（Unix 权限用 `PermissionsExt`）；
//! - `revstrstr` 等字符函数复用 [`crate::chars`]；
//! - 所有跨模块访问遵循"先取数据、闭包外计算"模式，避免 `RefCell` 嵌套借用。

use crate::definitions::*;
use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::rc::Rc;

const SEARCH_HISTORY: &str = "search_history";
const POSITION_HISTORY: &str = "filepos_history";

/// 历史链表的种类（对应 C 中 `list` 指针指向哪个链表变量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhichHistory {
    Search,
    Replace,
    Execute,
}

/// 与 C 的 `strncmp` 等价的字节比较（遇到 NUL 或 n 用尽时停止）。
fn strncmp_bytes(a: &[u8], b: &[u8], n: usize) -> i32 {
    for k in 0..n {
        let ca = a.get(k).copied().unwrap_or(0);
        let cb = b.get(k).copied().unwrap_or(0);
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
    }
    0
}

/// 确定给定项属于哪条历史链表。
fn which_history(item: &LineRef) -> Option<WhichHistory> {
    with_global(|g| {
        if g.search_history.as_ref().map(|h| Rc::ptr_eq(h, item)).unwrap_or(false) {
            Some(WhichHistory::Search)
        } else if g.replace_history.as_ref().map(|h| Rc::ptr_eq(h, item)).unwrap_or(false) {
            Some(WhichHistory::Replace)
        } else if g.execute_history.as_ref().map(|h| Rc::ptr_eq(h, item)).unwrap_or(false) {
            Some(WhichHistory::Execute)
        } else {
            None
        }
    })
}

/// 获取指定链表的 (top, bot)。
fn history_bounds(which: WhichHistory) -> (Option<LineRef>, Option<LineRef>) {
    with_global(|g| match which {
        WhichHistory::Search => (g.searchtop.clone(), g.searchbot.clone()),
        WhichHistory::Replace => (g.replacetop.clone(), g.replacebot.clone()),
        WhichHistory::Execute => (g.executetop.clone(), g.executebot.clone()),
    })
}

/// 设置指定链表的 (top, bot)。
fn set_history_bounds(which: WhichHistory, top: Option<LineRef>, bot: Option<LineRef>) {
    with_global_mut(|g| match which {
        WhichHistory::Search => {
            g.searchtop = top;
            g.searchbot = bot;
        }
        WhichHistory::Replace => {
            g.replacetop = top;
            g.replacebot = bot;
        }
        WhichHistory::Execute => {
            g.executetop = top;
            g.executebot = bot;
        }
    });
}

/// 获取指定链表的当前指针。
fn history_pointer(which: WhichHistory) -> Option<LineRef> {
    with_global(|g| match which {
        WhichHistory::Search => g.search_history.clone(),
        WhichHistory::Replace => g.replace_history.clone(),
        WhichHistory::Execute => g.execute_history.clone(),
    })
}

/// 设置指定链表的当前指针。
fn set_history_pointer(which: WhichHistory, item: Option<LineRef>) {
    with_global_mut(|g| match which {
        WhichHistory::Search => g.search_history = item,
        WhichHistory::Replace => g.replace_history = item,
        WhichHistory::Execute => g.execute_history = item,
    });
}

// ======================== 历史列表初始化与更新 ========================

/// 初始化搜索、替换和执行命令的历史字符串列表
/// （对应 `history_init`）。
pub fn history_init() {
    with_global_mut(|g| {
        let sh = make_new_node(None);
        g.search_history = Some(sh.clone());
        g.searchtop = Some(sh.clone());
        g.searchbot = Some(sh);

        let rh = make_new_node(None);
        g.replace_history = Some(rh.clone());
        g.replacetop = Some(rh.clone());
        g.replacebot = Some(rh);

        let eh = make_new_node(None);
        g.execute_history = Some(eh.clone());
        g.executetop = Some(eh.clone());
        g.executebot = Some(eh);
    });
}

/// 将包含 item 的历史列表指针重置到底部
/// （对应 `reset_history_pointer_for`）。
pub fn reset_history_pointer_for(item: &LineRef) {
    let which = match which_history(item) {
        Some(w) => w,
        None => return,
    };
    let bot = history_bounds(which).1;
    set_history_pointer(which, bot);
}

/// 在从 start 开始、到 end 结束的历史列表中，返回第一个含有给定文本
/// 前 len 个字符的节点；若无则返回 None（对应 `find_in_history`）。
pub fn find_in_history(
    start: &LineRef,
    end: &LineRef,
    text: &[u8],
    len: usize,
) -> Option<LineRef> {
    let mut item = Some(start.clone());
    loop {
        let it = item.clone()?;

        /* 到达 end->prev 即停止。 */
        let end_prev = { let r = end.borrow(); r.prev.clone() };
        let at_end_prev = end_prev
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|ep| Rc::ptr_eq(&it, &ep))
            .unwrap_or(false);
        if at_end_prev {
            return None;
        }

        let data = it.borrow().data.clone();
        if strncmp_bytes(data.as_bytes(), text, len) == 0 {
            return Some(it.clone());
        }

        let prev = { let r = it.borrow(); r.prev.clone() };
        item = prev.and_then(|w| w.upgrade());
    }
}

/// 以一条新字符串 text 更新一条历史列表（item 是其中的当前位置）：
/// 添加 text，或将其移到末尾（对应 `update_history`）。
pub fn update_history(item: &mut LineRef, text: &str, avoid_duplicates: bool) {
    let which = match which_history(item) {
        Some(w) => w,
        None => return,
    };
    update_history_which(&mut Some(which), text, avoid_duplicates);
    /* 将当前位置设为列表底部。 */
    let bot = history_bounds(which).1;
    *item = bot.unwrap();
}

/// 按链表种类更新历史（供 load_history 使用，C 中通过 `list` 指针选择）。
fn update_history_which(which: &mut Option<WhichHistory>, text: &str, avoid_duplicates: bool) {
    let w = match which {
        Some(w) => *w,
        None => return,
    };
    let (mut htop, mut hbot) = history_bounds(w);

    /* 当要求去重时，检查字符串是否已在历史中。 */
    let thesame = if avoid_duplicates {
        match (&hbot, &htop) {
            (Some(hb), Some(ht)) => find_in_history(hb, ht, text.as_bytes(), HIGHEST_POSITIVE),
            _ => None,
        }
    } else {
        None
    };

    /* 若找到相同字符串，删除该项。 */
    if let Some(same) = &thesame {
        let same_next = { let r = same.borrow(); r.next.clone() };
        let same_prev = { let r = same.borrow(); r.prev.clone() };

        if htop.as_ref().map(|t| Rc::ptr_eq(t, same)).unwrap_or(false) {
            htop = same_next.clone();
        } else if let Some(sp) = same_prev.as_ref().and_then(|w| w.upgrade()) {
            sp.borrow_mut().next = same_next.clone();
        }
        if let Some(sn) = &same_next {
            sn.borrow_mut().prev = same_prev;
        }

        if let Some(hb) = &hbot {
            hb.borrow_mut().lineno -= 1;
        }
    }

    /* 若历史已满，删除最旧的项（列表头部），为末尾的新项腾出空间。 */
    if let Some(hb) = &hbot {
        if hb.borrow().lineno > MAX_SEARCH_HISTORY as isize {
            let old_head = htop.clone();
            if let Some(oh) = &old_head {
                let new_head = { let r = oh.borrow(); r.next.clone() };
                if let Some(nh) = &new_head {
                    nh.borrow_mut().prev = None;
                }
                htop = new_head;
            }
            hb.borrow_mut().lineno -= 1;
        }
    }

    /* 将新字符串存入最后一项，然后创建新项。 */
    if let Some(hb) = &hbot {
        hb.borrow_mut().data = text.to_string();
        let given = hb.borrow();
        let newnode = make_new_node(Some(&*given));
        drop(given);
        newnode.borrow_mut().prev = Some(Rc::downgrade(hb));
        hb.borrow_mut().next = Some(newnode.clone());
        hbot = Some(newnode);
    }

    /* 指示历史在退出时需要保存。 */
    with_global_mut(|g| g.history_changed = true);

    set_history_bounds(w, htop, hbot);
}

/// 在三条历史列表之一中向后遍历，从 *here 处开始，查找给定字符串的
/// 制表补全（只看其前 len 个字符）。找到时让 *here 指向该项并返回其
/// 字符串；否则原样返回给定字符串（对应 `get_history_completion`）。
pub fn get_history_completion(here: &mut LineRef, string: &mut String, len: usize) -> String {
    let which = match which_history(here) {
        Some(w) => w,
        None => return string.clone(),
    };
    let (htop, hbot) = history_bounds(which);
    let (Some(ht), Some(hb)) = (htop, hbot) else {
        return string.clone();
    };

    /* 先从当前位置向前搜索 len 个字符的匹配；跳过完全相同的项。 */
    let here_prev = { let r = here.borrow(); r.prev.clone() };
    let mut item = here_prev
        .as_ref()
        .and_then(|w| w.upgrade())
        .and_then(|p| find_in_history(&p, &ht, string.as_bytes(), len));

    while let Some(it) = &item {
        let data = it.borrow().data.clone();
        if data == *string {
            let prev = { let r = it.borrow(); r.prev.clone() };
            item = prev
                .as_ref()
                .and_then(|w| w.upgrade())
                .and_then(|p| find_in_history(&p, &ht, string.as_bytes(), len));
        } else {
            break;
        }
    }

    if let Some(it) = &item {
        *here = it.clone();
        let data = it.borrow().data.clone();
        *string = data.clone();
        return data;
    }

    /* 现在从列表底部搜索到原始位置。 */
    let mut item = find_in_history(&hb, here, string.as_bytes(), len);

    while let Some(it) = &item {
        let data = it.borrow().data.clone();
        if data == *string {
            let prev = { let r = it.borrow(); r.prev.clone() };
            item = prev
                .as_ref()
                .and_then(|w| w.upgrade())
                .and_then(|p| find_in_history(&p, here, string.as_bytes(), len));
        } else {
            break;
        }
    }

    if let Some(it) = &item {
        *here = it.clone();
        let data = it.borrow().data.clone();
        *string = data.clone();
        return data;
    }

    /* 未找到有用匹配时，原样返回给定字符串。 */
    string.clone()
}

// ======================== 状态目录（对应 have_statedir） ========================

/// 检查是否拥有或能够创建存放历史文件的目录
/// （对应 `have_statedir`）。
pub fn have_statedir() -> bool {
    crate::utils::get_homedir();
    let homedir = with_global(|g| g.homedir.clone());

    if let Some(home) = &homedir {
        let statedir = format!("{}/.nano/", home);
        if let Ok(meta) = std::fs::metadata(&statedir) {
            if meta.is_dir() {
                with_global_mut(|g| {
                    g.statedir = Some(statedir.clone());
                    g.registername = Some(format!("{}{}", statedir, POSITION_HISTORY));
                });
                return true;
            }
        }
    }

    let xdgdatadir = std::env::var("XDG_DATA_HOME").ok();

    if homedir.is_none() && xdgdatadir.is_none() {
        return false;
    }

    let statedir = match &xdgdatadir {
        Some(x) => format!("{}/nano/", x),
        None => format!("{}/.local/share/nano/", homedir.as_ref().unwrap()),
    };

    if std::fs::metadata(&statedir).is_err() {
        if xdgdatadir.is_none() {
            let home = homedir.as_ref().unwrap();
            let _ = std::fs::create_dir(format!("{}/.local", home));
            let _ = std::fs::create_dir(format!("{}/.local/share", home));
        }
        if std::fs::create_dir(&statedir).is_err() {
            crate::rcfile::jot_error(&format!(
                "Unable to create directory {}: {}\n\
                 It is required for saving/loading \
                 search history or cursor positions.\n",
                statedir,
                std::io::Error::last_os_error()
            ));
            return false;
        }
    } else if let Ok(meta) = std::fs::metadata(&statedir) {
        if !meta.is_dir() {
            crate::rcfile::jot_error(&format!(
                "Path {} is not a directory and needs to be.\n\
                 Nano will be unable to load or save \
                 search history or cursor positions.\n",
                statedir
            ));
            return false;
        }
    }

    with_global_mut(|g| {
        g.statedir = Some(statedir.clone());
        g.registername = Some(format!("{}{}", statedir, POSITION_HISTORY));
    });
    true
}

// ======================== 历史文件的加载与保存 ========================

/// 加载 Search、Replace With 和 Execute Command 的历史
/// （对应 `load_history`）。
pub fn load_history() {
    let historyname = match with_global(|g| {
        g.statedir.as_ref().map(|s| format!("{}{}", s, SEARCH_HISTORY))
    }) {
        Some(h) => h,
        None => return,
    };

    let file = match std::fs::File::open(&historyname) {
        /* 若读取已有文件失败（文件不存在除外），退出时不保存历史。 */
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            crate::rcfile::jot_error(&crate::t!("history-error_reading", name = historyname, err = e.to_string()));
            UNSET(HISTORYLOG);
            return;
        }
        Ok(f) => f,
    };

    /* 从最旧到最新加载三条历史列表（先搜索，再替换，后执行）。
     * 两条列表之间有一个空行。 */
    let mut list = Some(WhichHistory::Search);
    let mut stanza = Vec::new();
    let mut reader = std::io::BufReader::new(&file);

    loop {
        stanza.clear();
        let read = match reader.read_until(b'\n', &mut stanza) {
            Ok(n) => n,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }

        /* 去掉末尾的换行（getline 会包含它）。 */
        stanza.truncate(read - 1);
        let read = read - 1;

        if read > 0 {
            crate::utils::recode_NUL_to_LF(&mut stanza, read);
            let text = String::from_utf8_lossy(&stanza[..read]).into_owned();
            update_history_which(&mut list, &text, IGNORE_DUPLICATES);
        } else if list == Some(WhichHistory::Search) {
            list = Some(WhichHistory::Replace);
        } else {
            list = Some(WhichHistory::Execute);
        }
    }

    /* 读入列表已将它们标记为已更改；撤销此副作用。 */
    with_global_mut(|g| g.history_changed = false);
}

/// 将一条历史列表的各行（从 head 开始，从最旧到最新）写入给定文件。
/// 写入成功返回 TRUE，否则返回 FALSE（对应 `write_list`）。
pub fn write_list(head: &LineRef, histories: &mut std::fs::File) -> bool {
    let mut item = Some(head.clone());

    while let Some(it) = item {
        /* 将 0x0A 字节解码为内嵌 NUL。 */
        let mut data = it.borrow().data.clone().into_bytes();
        let length = crate::utils::recode_LF_to_NUL(&mut data);
        /* 与 C 一致：recoding 就地修改了行的数据。 */
        it.borrow_mut().data = String::from_utf8_lossy(&data).into_owned();

        if histories.write_all(&data[..length]).is_err() {
            return false;
        }
        if histories.write_all(b"\n").is_err() {
            return false;
        }

        let next = { let r = it.borrow(); r.next.clone() };
        item = next;
    }

    true
}

/// 保存 Search、Replace With 和 Execute Command 的历史
/// （对应 `save_history`）。
pub fn save_history() {
    /* 若历史未更改，不必保存。 */
    if !with_global(|g| g.history_changed) {
        return;
    }

    let historyname = match with_global(|g| {
        g.statedir.as_ref().map(|s| format!("{}{}", s, SEARCH_HISTORY))
    }) {
        Some(h) => h,
        None => return,
    };

    let mut histories = match std::fs::File::create(&historyname) {
        Err(e) => {
            crate::rcfile::jot_error(&crate::t!("history-error_writing", name = historyname, err = e.to_string()));
            return;
        }
        Ok(f) => f,
    };

    /* 不允许其他人读取或写入历史文件。 */
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::set_permissions(&historyname, std::fs::Permissions::from_mode(0o600)).is_err() {
            crate::rcfile::jot_error(&format!(
                "Cannot limit permissions on {}: {}",
                historyname,
                std::io::Error::last_os_error()
            ));
        }
    }

    let (st, rt, et) = with_global(|g| {
        (
            g.searchtop.clone(),
            g.replacetop.clone(),
            g.executetop.clone(),
        )
    });

    let mut ok = true;
    if let Some(h) = &st {
        ok = write_list(h, &mut histories) && ok;
    }
    if let Some(h) = &rt {
        ok = write_list(h, &mut histories) && ok;
    }
    if let Some(h) = &et {
        ok = write_list(h, &mut histories) && ok;
    }

    if !ok {
        crate::rcfile::jot_error(&format!(
            "Error writing {}: {}",
            historyname,
            std::io::Error::last_os_error()
        ));
    }
}

// ======================== 锚点（对应 stringify_anchors / restore_anchors） ========================

/// 以字符串形式返回带锚点的行的行号（对应 `stringify_anchors`）。
pub fn stringify_anchors() -> String {
    let mut string = String::new();

    with_global(|g| {
        if let Some(of) = &g.openfile {
            let of = of.borrow();
            let mut line = of.filetop.clone();
            while let Some(l) = line {
                let (has_anchor, lineno) = {
                    let r = l.borrow();
                    (r.has_anchor, r.lineno)
                };
                if has_anchor {
                    string.push_str(&format!("{} ", lineno));
                }
                let next = { let r = l.borrow(); r.next.clone() };
                line = next;
            }
        }
    });

    string
}

/// 为给定字符串中的每个行号设置锚点（对应 `restore_anchors`）。
pub fn restore_anchors(string: &str) {
    let mut rest = string;

    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let of = of.borrow();
        let mut line = of.filetop.clone();

        while !rest.is_empty() {
            let Some(space) = rest.find(' ') else { return };
            let number: isize = rest[..space].parse().unwrap_or(0);
            rest = &rest[space + 1..];

            /* 推进到行号 >= number 的行；越界则返回。 */
            loop {
                let Some(l) = line.clone() else { return };
                let lineno = l.borrow().lineno;
                if lineno >= number {
                    break;
                }
                let next = { let r = l.borrow(); r.next.clone() };
                line = next;
            }

            if let Some(l) = &line {
                l.borrow_mut().has_anchor = true;
            }
        }
    });
}

// ======================== 位置寄存器（对应 load/save/reload/update/restore） ========================

/// 加载已打开文件记录的光标位置（对应 `load_positions_register`）。
pub fn load_positions_register() {
    let registername = match with_global(|g| g.registername.clone()) {
        Some(r) => r,
        None => return,
    };

    let file = match std::fs::File::open(&registername) {
        /* 若读取已有文件失败（文件不存在除外），退出时不保存寄存器。 */
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            crate::rcfile::jot_error(&crate::t!("history-error_reading", name = registername, err = e.to_string()));
            UNSET(POSITIONLOG);
            return;
        }
        Ok(f) => f,
    };

    let mut reader = std::io::BufReader::new(&file);
    let mut phrase = Vec::new();
    let mut count = 0;
    let mut lastitem: Option<PositionRef> = None;

    /* 读取并解析每一行，存储提取的数据。 */
    loop {
        if count >= 200 {
            break;
        }
        count += 1;

        phrase.clear();
        let length = match reader.read_until(b'\n', &mut phrase) {
            Ok(n) => n,
            Err(_) => break,
        };
        if length <= 1 {
            break;
        }

        /* stanza 是 '/' 的位置（绝对路径起始）。 */
        let Some(sp) = phrase.iter().position(|&b| b == b'/') else {
            continue;
        };
        let mut stanza = phrase[sp..].to_vec();
        let length = length - sp;

        /* 将 NUL 解码为内嵌换行。 */
        crate::utils::recode_NUL_to_LF(&mut stanza, length);

        /* 找到列号和行号之前的空格。 */
        let col_rel = match crate::chars::revstrstr(&stanza, b" ", length.saturating_sub(3)) {
            Some(c) => c,
            None => continue,
        };
        let line_rel = match crate::chars::revstrstr(&stanza, b" ", col_rel.saturating_sub(2)) {
            Some(l) => l,
            None => continue,
        };

        /* 现在分隔行的三个元素。 */
        let filename = String::from_utf8_lossy(&stanza[..line_rel]).into_owned();
        let line_str = String::from_utf8_lossy(&stanza[line_rel + 1..col_rel]).into_owned();
        let col_str = String::from_utf8_lossy(&stanza[col_rel + 1..]).into_owned();
        let linenumber: isize = line_str.trim().parse().unwrap_or(0);
        let columnnumber: isize = col_str.trim().parse().unwrap_or(0);

        /* '/' 之前的部分（若有）是锚点行号串。 */
        let anchors = if sp == 0 {
            None
        } else {
            Some(String::from_utf8_lossy(&phrase[..sp]).into_owned())
        };

        /* 创建新的位置记录并加入列表。 */
        let newitem = Rc::new(RefCell::new(PositionStruct {
            filename: Some(filename),
            linenumber,
            columnnumber,
            anchors,
            next: None,
        }));

        with_global_mut(|g| {
            if g.positions_register.is_none() {
                g.positions_register = Some(newitem.clone());
            } else if let Some(li) = &lastitem {
                li.borrow_mut().next = Some(newitem.clone());
            }
        });
        lastitem = Some(newitem);
    }

    if let Ok(meta) = std::fs::metadata(&registername) {
        if let Ok(modified) = meta.modified() {
            if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                with_global_mut(|g| g.latest_timestamp = d.as_secs() as i64);
            }
        }
    }
}

/// 保存已打开文件记录的光标位置（对应 `save_positions_register`）。
pub fn save_positions_register() {
    let registername = match with_global(|g| g.registername.clone()) {
        Some(r) => r,
        None => return,
    };

    let mut registry = match std::fs::File::create(&registername) {
        Err(e) => {
            crate::rcfile::jot_error(&crate::t!("history-error_writing", name = registername, err = e.to_string()));
            return;
        }
        Ok(f) => f,
    };

    /* 不允许其他人读取或写入位置寄存器文件。 */
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::set_permissions(&registername, std::fs::Permissions::from_mode(0o600)).is_err() {
            crate::rcfile::jot_error(&format!(
                "Cannot limit permissions on {}: {}",
                registername,
                std::io::Error::last_os_error()
            ));
        }
    }

    let items: Vec<PositionRef> = with_global(|g| {
        let mut v = Vec::new();
        let mut item = g.positions_register.clone();
        let mut count = 0;
        while let Some(it) = item {
            if count >= 200 {
                break;
            }
            count += 1;
            v.push(it.clone());
            let next = { let r = it.borrow(); r.next.clone() };
            item = next;
        }
        v
    });

    for item in &items {
        let (anchors, filename, linenumber, columnnumber) = {
            let it = item.borrow();
            (it.anchors.clone(), it.filename.clone().unwrap_or_default(), it.linenumber, it.columnnumber)
        };

        /* 先写带锚点的行号串（若有）。 */
        if let Some(a) = &anchors {
            if !a.is_empty() && registry.write_all(a.as_bytes()).is_err() {
                crate::rcfile::jot_error(&format!(
                    "Error writing {}: {}",
                    registername,
                    std::io::Error::last_os_error()
                ));
            }
        }

        /* 假设行号与列号各 20 个十进制位，加两个空格、换行与 NUL。 */
        let mut path_and_place = format!("{} {} {}\n", filename, linenumber, columnnumber).into_bytes();

        /* 将文件名中的换行编码为 NUL。 */
        let length = crate::utils::recode_LF_to_NUL(&mut path_and_place);
        /* 恢复末尾换行。 */
        path_and_place[length - 1] = b'\n';

        if registry.write_all(&path_and_place[..length]).is_err() {
            crate::rcfile::jot_error(&format!(
                "Error writing {}: {}",
                registername,
                std::io::Error::last_os_error()
            ));
        }
    }

    if let Ok(meta) = std::fs::metadata(&registername) {
        if let Ok(modified) = meta.modified() {
            if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                with_global_mut(|g| g.latest_timestamp = d.as_secs() as i64);
            }
        }
    }
}

/// 若位置寄存器文件自上次加载以来被修改，则重新加载
/// （对应 `reload_positions_if_needed`）。
pub fn reload_positions_if_needed() {
    let registername = match with_global(|g| g.registername.clone()) {
        Some(r) => r,
        None => return,
    };

    let mtime = match std::fs::metadata(&registername) {
        Ok(meta) => meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
        Err(_) => return,
    };
    let Some(mtime) = mtime else { return };

    if mtime == with_global(|g| g.latest_timestamp) {
        return;
    }

    with_global_mut(|g| {
        g.positions_register = None; // Rc 自动释放旧列表
    });

    load_positions_register();
}

/// 用当前缓冲区中的当前位置更新记录的最近文件位置。
/// 若无现有条目，则在顶部添加新条目（对应 `update_positions_register`）。
pub fn update_positions_register() {
    let filename = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
    });
    let Some(filename) = filename else { return };

    let fullpath = match crate::files::get_full_path(&filename) {
        Some(p) => p,
        None => return,
    };

    reload_positions_if_needed();

    /* 在列表中查找匹配的文件名。 */
    let (previous, found) = with_global(|g| {
        let mut prev: Option<PositionRef> = None;
        let mut item = g.positions_register.clone();
        loop {
            let Some(it) = item.clone() else { break };
            let name = it.borrow().filename.clone().unwrap_or_default();
            if name == fullpath {
                break;
            }
            prev = Some(it.clone());
            let next = { let r = it.borrow(); r.next.clone() };
            item = next;
        }
        (prev, item)
    });

    /* 当前光标位置与锚点（闭包外取数据）。 */
    let (cur_lineno, cur_x, cur_data) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let lineno = of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(1);
        let x = of.current_x;
        let data = of.current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default();
        (lineno, x, data)
    });
    let column = crate::utils::wideness(cur_data.as_bytes(), cur_x) + 1;
    let anchors = stringify_anchors();

    with_global_mut(|g| {
        /* 若无匹配，创建新节点；否则摘除匹配项。 */
        let item = found.clone();
        let item = match item {
            Some(it) => it,
            None => {
                let newitem = Rc::new(RefCell::new(PositionStruct {
                    filename: Some(fullpath.clone()),
                    linenumber: 0,
                    columnnumber: 0,
                    anchors: None,
                    next: None,
                }));
                newitem
            }
        };

        if let Some(p) = &previous {
            let next = item.borrow().next.clone();
            p.borrow_mut().next = next;
        }

        /* 若尚未在开头，将其移到开头。 */
        let at_head = g.positions_register.as_ref().map(|h| Rc::ptr_eq(h, &item)).unwrap_or(false);
        if !at_head {
            let next = g.positions_register.clone();
            item.borrow_mut().next = next;
            g.positions_register = Some(item.clone());
        }

        /* 记录最后的光标位置与锚点。 */
        item.borrow_mut().linenumber = cur_lineno;
        item.borrow_mut().columnnumber = column as isize;
        item.borrow_mut().anchors = Some(anchors.clone());
    });

    save_positions_register();
}

/// 检查当前文件名是否匹配记录位置列表中的条目；
/// 若匹配则恢复相应的光标位置（对应 `restore_cursor_position_if_any`）。
pub fn restore_cursor_position_if_any() {
    let filename = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
    });
    let Some(filename) = filename else { return };

    let fullpath = match crate::files::get_full_path(&filename) {
        Some(p) => p,
        None => return,
    };

    reload_positions_if_needed();

    let item = with_global(|g| {
        let mut item = g.positions_register.clone();
        while let Some(it) = item.clone() {
            let name = it.borrow().filename.clone().unwrap_or_default();
            if name == fullpath {
                break;
            }
            let next = { let r = it.borrow(); r.next.clone() };
            item = next;
        }
        item
    });

    let (linenumber, columnnumber, anchors) = match &item {
        Some(it) => {
            let r = it.borrow();
            (r.linenumber, r.columnnumber, r.anchors.clone())
        }
        None => return,
    };

    if let Some(a) = &anchors {
        restore_anchors(a);
    }
    crate::search::goto_line_and_column(linenumber, columnnumber, true);
}
