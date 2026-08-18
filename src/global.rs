/**************************************************************************
 * global.rs  --  GNU nano 全局变量与快捷键初始化（对应 global.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 全局变量、快捷键与函数列表的初始化。
//! 转换说明：使用 `GLOBAL` 安全全局状态替代 `static mut`。

use crate::definitions::*;
use std::rc::Rc;
use std::cell::RefCell;

/// 初始化全局状态。
pub fn global_init() {
    with_global_mut(|g| {
        g.using_utf8 = true;
        set_using_utf8_independent(true);
        g.tabsize = 8;
        set_tabsize_independent(8);
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.interface_color_pair = vec![0; NUMBER_OF_ELEMENTS];
        g.allfuncs = None;
        g.shortcuts = None;
        g.syntaxes = None;
    });
}

/// 报告光标位置。
pub fn report_cursor_position() {
    use crate::winio;
    with_global(|g| {
        let of = g.openfile.as_ref();
        if let Some(of) = of {
            let of_ref = of.borrow();
            let current = of_ref.current.as_ref();
            let cur_x = of_ref.current_x;
            if let Some(c) = current {
                let c_ref = c.borrow();
                let lineno = c_ref.lineno;
                /* 列号：从 1 开始，且按字符数（不是字节数）计，与 C 版 keyreport 一致。 */
                let data = c_ref.data.as_bytes();
                let mut char_pos = 0usize;
                let mut byte_idx = 0usize;
                while byte_idx < cur_x && byte_idx < data.len() {
                    let clen = crate::chars::char_length(&data[byte_idx..]);
                    byte_idx += clen;
                    char_pos += 1;
                }
                let col = char_pos + 1;
                let msg = format!("Line {}, column {}", lineno, col);
                winio::statusbar(&msg);
            }
        }
    });
}

/// 设置当前菜单。
pub fn set_currmenu(menu: i32) {
    with_global_mut(|g| g.currmenu = menu);
}

/// 获取当前菜单。
pub fn get_currmenu() -> i32 {
    with_global(|g| g.currmenu)
}

/// 设置窗口尺寸。
pub fn set_windowsize(cols: usize, lines: usize) {
    with_global_mut(|g| {
        g.COLS = cols;
        g.LINES = lines;
        g.editwinrows = (lines as i32).saturating_sub(4).max(1);
        // 对应 C 版 nano.c/help.c 中 `editwincols = COLS - margin - sidebar;`。
        // 在 Rust 版中 sidebar 被建模为 bool（C 版为宽度 int），
        // 这里用一个最小的非零值 1 表示启用侧边栏，并保证结果不会下溢。
        let sidebar_width = if g.sidebar { 1 } else { 0 };
        g.editwincols = (cols as i32 - g.margin - sidebar_width).max(1) as usize;
    });
}

/// 获取编辑窗口行数。
pub fn get_editwinrows() -> i32 {
    with_global(|g| g.editwinrows)
}

/// 获取 COLS。
pub fn get_cols() -> usize {
    with_global(|g| g.COLS)
}

/// 获取 LINES。
pub fn get_lines() -> usize {
    with_global(|g| g.LINES)
}

/// 添加函数到函数列表（尾插法，与 C 版一致）。
pub fn add_to_funcs(func: FunctionId, menus: i32, tag: String, phrase: &'static str, blank_after: bool) {
    with_global_mut(|g| {
        let new_func = Rc::new(RefCell::new(FuncStruct {
            func,
            tag,
            phrase,
            blank_after,
            menus,
            next: None,
        }));
        // 尾插：遍历到链表末尾，或设为第一个节点
        if g.allfuncs.is_none() {
            g.allfuncs = Some(new_func);
        } else {
            let mut current = g.allfuncs.clone();
            while let Some(ref node) = current {
                let next = node.borrow().next.clone();
                if next.is_none() {
                    node.borrow_mut().next = Some(new_func);
                    break;
                }
                current = next;
            }
        }
    });
}

/// 添加快捷键（尾插法，与 C 版一致）。
pub fn add_to_sclist(menus: i32, keystr: &str, keycode: i32, func: FunctionId, toggle: i32) {
    with_global_mut(|g| {
        let new_key = Rc::new(RefCell::new(KeyStruct {
            keystr: keystr.to_string(),
            keycode,
            menus,
            func,
            toggle,
            ordinal: 0,
            expansion: None,
            next: None,
        }));
        // 尾插
        if g.shortcuts.is_none() {
            g.shortcuts = Some(new_key);
        } else {
            let mut current = g.shortcuts.clone();
            while let Some(ref node) = current {
                let next = node.borrow().next.clone();
                if next.is_none() {
                    node.borrow_mut().next = Some(new_key);
                    break;
                }
                current = next;
            }
        }
    });
}

/// 通过函数 ID 查找快捷键。
pub fn find_shortcut_by_func(func: FunctionId) -> Option<KeyRef> {
    with_global(|g| {
        let mut current = g.shortcuts.clone();
        while let Some(s) = current {
            if s.borrow().func == func {
                return Some(s.clone());
            }
            current = s.borrow().next.clone();
        }
        None
    })
}

/// 查找快捷键。
pub fn find_shortcut(keycode: i32, menu: i32) -> Option<KeyRef> {
    with_global(|g| {
        let mut current = g.shortcuts.clone();
        while let Some(s) = current {
            let s_ref = s.borrow();
            if s_ref.keycode == keycode && (s_ref.menus & menu) != 0 {
                drop(s_ref);
                return Some(s.clone());
            }
            current = s_ref.next.clone();
        }
        None
    })
}

/// 获取所有快捷键的迭代器。
pub fn iter_shortcuts() -> Vec<KeyRef> {
    with_global(|g| {
        let mut result = Vec::new();
        let mut current = g.shortcuts.clone();
        while let Some(s) = current {
            result.push(s.clone());
            current = s.borrow().next.clone();
        }
        result
    })
}

/// 获取所有函数的迭代器。
pub fn iter_funcs() -> Vec<FuncRef> {
    with_global(|g| {
        let mut result = Vec::new();
        let mut current = g.allfuncs.clone();
        while let Some(f) = current {
            result.push(f.clone());
            current = f.borrow().next.clone();
        }
        result
    })
}

/// 设置是否在 VT 终端上。
pub fn set_on_a_vt(val: bool) {
    with_global_mut(|g| g.on_a_vt = val);
}

/// 获取是否在 VT 终端上。
pub fn is_on_a_vt() -> bool {
    with_global(|g| g.on_a_vt)
}

/// 分配并设置快捷键列表（所有菜单）。
pub fn shortcut_init() {
    // 主菜单快捷键（与 C 版 global.c 对应）
    add_to_sclist(MMAIN, r"^G", 7, FunctionId::DoHelp, 0);
    add_to_sclist(MMAIN, r"^O", 15, FunctionId::DoWriteOut, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^F", 6, FunctionId::DoSearchForward, 0);
    add_to_sclist(MMAIN, r"^\", 28, FunctionId::DoReplace, 0);
    add_to_sclist(MMAIN, r"^K", 11, FunctionId::DoCut, 0);
    add_to_sclist(MMAIN, r"^U", 21, FunctionId::DoPaste, 0);
    add_to_sclist(MMAIN, r"^J", 10, FunctionId::DoJustify, 0);
    add_to_sclist(MMAIN, r"^T", 20, FunctionId::DoExecute, 0);
    add_to_sclist(MMAIN, r"^C", 3, FunctionId::DoReportLocation, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^X", 24, FunctionId::DoExit, 0);
    add_to_sclist(MMAIN, r"^R", 18, FunctionId::DoInsertFile, 0);
    add_to_sclist(MMAIN, r"^/", 31, FunctionId::DoGoToLine, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^B", 2, FunctionId::DoSearchBackward, 0);
    // 方向键（保留但不在底部栏显示）
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^P", 16, FunctionId::DoUp, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^N", 14, FunctionId::DoDown, 0);
    add_to_sclist(MMAIN, r"^A", 1, FunctionId::DoHome, 0);
    add_to_sclist(MMAIN, r"^E", 5, FunctionId::DoEnd, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP | MLINTER, r"^V", 22, FunctionId::DoPageDown, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP | MLINTER, r"^Y", 25, FunctionId::DoPageUp, 0);
    add_to_sclist(MMAIN, r"^D", 4, FunctionId::DoDelete, 0);
    add_to_sclist(MMAIN, r"^H", 8, FunctionId::DoBackspace, 0);
    add_to_sclist(MMAIN, r"^I", 9, FunctionId::DoTab, 0);
    add_to_sclist(MMAIN, r"^M", 13, FunctionId::DoEnter, 0);
    // 全屏刷新（对应 C 的 full_refresh：MMOST|MBROWSER|MHELP|MYESNO）
    add_to_sclist(MMAIN | MBROWSER | MHELP | MYESNO, r"^L", 12, FunctionId::DoFullRefresh, 0);
    // Alt 组合
    add_to_sclist(MMAIN, "M-U", 0, FunctionId::DoUndo, 0);
    add_to_sclist(MMAIN, "M-E", 0, FunctionId::DoRedo, 0);
    add_to_sclist(MMAIN, "M-A", 0, FunctionId::DoMark, 0);
    add_to_sclist(MMAIN, "M-6", 0, FunctionId::DoCopy, 0);
    add_to_sclist(MMAIN, "M-]", 0, FunctionId::DoFindBracket, 0);
    // 帮助页 Previous/Next（对应 C 的 M-B/M-F → do_findprevious/do_findnext）
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-B", 0x262, FunctionId::DoFindPrevious, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-F", 0x266, FunctionId::DoFindNext, 0);
    // 方向键（仅用于输入匹配，不显示在底部栏）
    add_to_sclist(MMAIN, "Left", KEY_LEFT, FunctionId::DoLeft, 0);
    add_to_sclist(MMAIN, "Right", KEY_RIGHT, FunctionId::DoRight, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "Up", KEY_UP, FunctionId::DoUp, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "Down", KEY_DOWN, FunctionId::DoDown, 0);
    add_to_sclist(MMAIN, "Home", KEY_HOME, FunctionId::DoHome, 0);
    add_to_sclist(MMAIN, "End", KEY_END, FunctionId::DoEnd, 0);
    add_to_sclist(MMAIN, "PageUp", KEY_PPAGE, FunctionId::DoPageUp, 0);
    add_to_sclist(MMAIN, "PageDown", KEY_NPAGE, FunctionId::DoPageDown, 0);
    // 搜索菜单
    add_to_sclist(MWHEREIS, r"^M", 13, FunctionId::DoSearchForward, 0);
    add_to_sclist(MWHEREIS, r"^C", 3, FunctionId::DoCancel, 0);
    add_to_sclist(MWHEREIS, r"^R", 18, FunctionId::DoToggleRegexp, 0);
    add_to_sclist(MWHEREIS, r"^B", 2, FunctionId::DoToggleBackwards, 0);
    // 替换菜单
    add_to_sclist(MREPLACE, r"^M", 13, FunctionId::DoReplace, 0);
    add_to_sclist(MREPLACE, r"^C", 3, FunctionId::DoCancel, 0);
    add_to_sclist(MREPLACE, r"^R", 18, FunctionId::DoToggleRegexp, 0);
    add_to_sclist(MREPLACE, r"^B", 2, FunctionId::DoToggleBackwards, 0);
    // 通用提示快捷键（对应 C 的 MMOST|MBROWSER ^M、取消 ^C 与 Esc）
    add_to_sclist(MMOST | MBROWSER, r"^M", 13, FunctionId::DoEnter, 0);
    add_to_sclist((MMOST & !MMAIN) | MYESNO, r"^C", 3, FunctionId::DoCancel, 0);
    add_to_sclist((MMOST & !MMAIN) | MYESNO, "Cancel", ESC_CODE as i32, FunctionId::DoCancel, 0);
    // YesNo 菜单
    add_to_sclist(MYESNO, "Y", 121, FunctionId::None, 0);
    add_to_sclist(MYESNO, "N", 110, FunctionId::None, 0);
    add_to_sclist(MYESNO, "A", 97, FunctionId::None, 0);
    // 函数列表（按 C 版顺序，遍历时为逆序，与 C 版一致）
    add_to_funcs(FunctionId::DoHelp, (MMOST | MBROWSER) & !MFINDINHELP, crate::t!("key-help"), "help_gist", false);
    /* 帮助查看器专有条目（对应 C：add_to_funcs(full_refresh, MHELP, ...) 等）。 */
    add_to_funcs(FunctionId::DoFullRefresh, MHELP, crate::t!("key-refresh"), "x", false);
    add_to_funcs(FunctionId::DoExit, MHELP, crate::t!("key-close"), "x", false);
    add_to_funcs(FunctionId::DoCancel, (MMOST & !MMAIN) | MYESNO, crate::t!("key-cancel"), "cancel_gist", true);
    add_to_funcs(FunctionId::DoExit, MMAIN, crate::t!("key-exit"), "exit_gist", false);
    add_to_funcs(FunctionId::DoRefresh, MMAIN | MREPLACE, crate::t!("key-refresh"), "x", false);
    add_to_funcs(FunctionId::DoWriteOut, MMAIN, crate::t!("key-write_out"), "writeout_gist", false);
    add_to_funcs(FunctionId::DoInsertFile, MMAIN, crate::t!("key-read_file"), "insert_gist", false);
    add_to_funcs(FunctionId::DoJustify, MMAIN, crate::t!("key-justify"), "justify_gist", false);
    add_to_funcs(FunctionId::DoSearchForward, MMAIN | MHELP, crate::t!("key-where_is"), "whereis_gist", false);
    add_to_funcs(FunctionId::DoSearchBackward, MMAIN | MHELP, crate::t!("key-where_was"), "wherewas_gist", false);
    add_to_funcs(FunctionId::DoReplace, MMAIN, crate::t!("key-replace"), "replace_gist", false);
    add_to_funcs(FunctionId::DoFindPrevious, MMAIN | MHELP, crate::t!("key-previous"), "findprev_gist", false);
    add_to_funcs(FunctionId::DoFindNext, MMAIN | MHELP, crate::t!("key-next"), "findnext_gist", false);
    add_to_funcs(FunctionId::DoCut, MMAIN, crate::t!("key-cut"), "cut_gist", false);
    add_to_funcs(FunctionId::DoPaste, MMAIN, crate::t!("key-paste"), "paste_gist", false);
    add_to_funcs(FunctionId::DoExecute, MMAIN, crate::t!("key-execute"), "execute_gist", false);
    add_to_funcs(FunctionId::DoReportLocation, MMAIN, crate::t!("key-location"), "location_gist", false);
    add_to_funcs(FunctionId::DoGoToLine, MMAIN, crate::t!("key-go_to_line"), "gotoline_gist", false);
    add_to_funcs(FunctionId::DoUndo, MMAIN, crate::t!("key-undo"), "undo_gist", true);
    add_to_funcs(FunctionId::DoRedo, MMAIN, crate::t!("key-redo"), "redo_gist", true);
    add_to_funcs(FunctionId::DoMark, MMAIN, crate::t!("key-set_mark"), "mark_gist", true);
    add_to_funcs(FunctionId::DoCopy, MMAIN, crate::t!("key-copy"), "copy_gist", true);
    add_to_funcs(FunctionId::DoToggleCaseSensitive, MWHEREIS | MREPLACE, crate::t!("key-case_sens"), "casesens_gist", true);
    add_to_funcs(FunctionId::DoToggleRegexp, MWHEREIS | MREPLACE, crate::t!("key-regexp"), "regexp_gist", true);
    add_to_funcs(FunctionId::DoToggleBackwards, MWHEREIS | MREPLACE, crate::t!("key-backwards"), "backwards_gist", true);
    add_to_funcs(FunctionId::DoFindBracket, MMAIN, crate::t!("key-to_bracket"), "bracket_gist", true);
    add_to_funcs(FunctionId::DoLeft, MMAIN, crate::t!("key-left"), "left_gist", true);
    add_to_funcs(FunctionId::DoRight, MMAIN, crate::t!("key-right"), "right_gist", true);
    add_to_funcs(FunctionId::DoUp, MMAIN | MBROWSER | MHELP, crate::t!("key-prev_line"), "prevline_gist", true);
    add_to_funcs(FunctionId::DoDown, MMAIN | MBROWSER | MHELP, crate::t!("key-next_line"), "nextline_gist", true);
    add_to_funcs(FunctionId::DoHome, MMAIN, crate::t!("key-home"), "home_gist", true);
    add_to_funcs(FunctionId::DoEnd, MMAIN, crate::t!("key-end"), "end_gist", true);
    add_to_funcs(FunctionId::DoPageUp, MMAIN | MHELP, crate::t!("key-prev_page"), "prevpage_gist", true);
    add_to_funcs(FunctionId::DoPageDown, MMAIN | MHELP, crate::t!("key-next_page"), "nextpage_gist", true);
    add_to_funcs(FunctionId::DoDelete, MMAIN, crate::t!("key-delete"), "delete_gist", true);
    add_to_funcs(FunctionId::DoBackspace, MMAIN, crate::t!("key-backspace"), "backspace_gist", true);
    add_to_funcs(FunctionId::DoEnter, MMAIN, crate::t!("key-enter"), "enter_gist", true);
    add_to_funcs(FunctionId::DoTab, MMAIN, crate::t!("key-tab"), "tab_gist", true);
    /* 其余菜单的快捷键（提示/浏览器等，对应 C 的 shortcut_init 后半部分）。 */
    shortcut_init_rest();
}
// ======================== 按键解释（对应 global.c） ========================

/// 将按键码解释为函数（对应 `interpret`）。
pub fn interpret(kbinput: i32) -> Option<FunctionId> {
    let currmenu = with_global(|g| g.currmenu);
    find_shortcut(kbinput, currmenu).map(|s| s.borrow().func)
}

/// 未绑定按键的提示（对应 `unbound_key`）。
pub fn unbound_key(kbinput: i32) {
    if kbinput >= 0 {
        use crate::winio;
        winio::statusline_centered(
            crate::definitions::MessageType::Ahem,
            &format!("[ {} ]", crate::t!("key-unbound")),
        );
        winio::beep();
    }
}

/// 返回 flag 的简短名称（对应 `epithet_of_flag`）。
pub fn epithet_of_flag(_flag: i32) -> &'static str {
    "toggle"
}

/// 在给定菜单中查找第一个绑定到给定函数的快捷键（对应 `first_sc_for`）。
pub fn first_sc_for(menu: i32, func: FunctionId) -> Option<KeyRef> {
    with_global(|g| {
        let mut current = g.shortcuts.clone();
        while let Some(s) = current {
            let s_ref = s.borrow();
            if (s_ref.menus & menu) != 0 && s_ref.func == func && !s_ref.keystr.is_empty() {
                return Some(s.clone());
            }
            current = s_ref.next.clone();
        }
        None
    })
}

const ALT_BACKSPACE: i32 = 0x40A;
const ALT_SHIFT_COMMA: i32 = 0x43C;
const ALT_SHIFT_DOT: i32 = 0x43E;

/// 注册提示/浏览器/其他菜单的快捷键（对应 global.c 的其余部分）。
pub fn shortcut_init_rest() {
    // 提示菜单：方向键与编辑键
    let mmi = MMAIN | MWHEREIS | MREPLACE | MREPLACEWITH | MGOTOLINE | MINSERTFILE
        | MWRITEFILE | MEXECUTE | MSPELL | MLINTER;
    add_to_sclist(mmi, "Left", KEY_LEFT, FunctionId::DoLeft, 0);
    add_to_sclist(mmi, "Right", KEY_RIGHT, FunctionId::DoRight, 0);
    add_to_sclist(mmi, "Up", KEY_UP, FunctionId::GetOlderItem, 0);
    add_to_sclist(mmi, "Down", KEY_DOWN, FunctionId::GetNewerItem, 0);
    add_to_sclist(mmi, "Home", KEY_HOME, FunctionId::DoHome, 0);
    add_to_sclist(mmi, "End", KEY_END, FunctionId::DoEnd, 0);
    add_to_sclist(mmi, "M-Left", ALT_LEFT, FunctionId::DoPrevWord, 0);
    add_to_sclist(mmi, "M-Right", ALT_RIGHT, FunctionId::DoNextWord, 0);
    add_to_sclist(mmi, "M-Backspace", ALT_BACKSPACE, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "M-D", ALT_DELETE, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "^V", 22, FunctionId::DoVerbatimInput, 0);
    add_to_sclist(mmi, "^X", 24, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "^K", 11, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "^U", 21, FunctionId::DoPaste, 0);
    add_to_sclist(mmi, "^H", 8, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "^D", 4, FunctionId::DoDelete, 0);
    add_to_sclist(mmi, "Bsp", KEY_BACKSPACE, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "Sh-Del", SHIFT_DELETE, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "Del", KEY_DC, FunctionId::DoDelete, 0);
    add_to_sclist(mmi, "^I", 9, FunctionId::DoTab, 0);

    // 浏览器菜单
    add_to_sclist(MBROWSER, "^F", 6, FunctionId::DoSearchForward, 0);
    add_to_sclist(MBROWSER, "^B", 2, FunctionId::DoSearchBackward, 0);
    add_to_sclist(MBROWSER, "^C", 3, FunctionId::DoExit, 0);
    add_to_sclist(MBROWSER, "^S", 19, FunctionId::DoEnter, 0);
    add_to_sclist(MBROWSER, "^H", 8, FunctionId::DoGotoDir, 0);
    add_to_sclist(MBROWSER, "Up", KEY_UP, FunctionId::DoUp, 0);
    add_to_sclist(MBROWSER, "Down", KEY_DOWN, FunctionId::DoDown, 0);
    add_to_sclist(MBROWSER, "Left", KEY_LEFT, FunctionId::DoLeft, 0);
    add_to_sclist(MBROWSER, "Right", KEY_RIGHT, FunctionId::DoRight, 0);
    add_to_sclist(MBROWSER, "M-<", ALT_SHIFT_COMMA, FunctionId::ToFirstFile, 0);
    add_to_sclist(MBROWSER, "M->", ALT_SHIFT_DOT, FunctionId::ToLastFile, 0);
    add_to_sclist(MBROWSER, "PageUp", KEY_PPAGE, FunctionId::DoPageUp, 0);
    add_to_sclist(MBROWSER, "PageDown", KEY_NPAGE, FunctionId::DoPageDown, 0);

    // 函数列表补充
    add_to_funcs(FunctionId::GetOlderItem, MMAIN, crate::t!("key-get_older_item"), "older_gist", false);
    add_to_funcs(FunctionId::GetNewerItem, MMAIN, crate::t!("key-get_newer_item"), "newer_gist", false);
    add_to_funcs(FunctionId::ToFirstFile, MBROWSER, crate::t!("key-first_file"), "firstfile_gist", false);
    add_to_funcs(FunctionId::ToLastFile, MBROWSER, crate::t!("key-last_file"), "lastfile_gist", false);
    add_to_funcs(FunctionId::DoGotoDir, MBROWSER, crate::t!("key-go_to_dir"), "gotodir_gist", false);
    add_to_funcs(FunctionId::DoVerbatimInput, MMAIN, crate::t!("key-verbatim_input"), "verbatim_gist", false);
}

// ======================== 命令行参数解析（对应 nano.c 的 main） ========================

/// 解析命令行参数。
/// 解析命令行选项（对应 C 的 getopt_long 循环）。
/// GNU getopt 默认会重排 argv，因此选项可以出现在文件名之后；
/// 这里记录第一个文件名，同时继续解析后续选项。
pub fn parse_args(args: &[String]) -> Option<String> {
    let mut i = 1;
    let mut filename: Option<String> = None;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') {
            match arg.as_str() {
                "-V" | "--version" => {
                    print_version();
                    std::process::exit(0);
                }
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                "-v" | "--view" => SET(VIEW_MODE),
                "-x" | "--nohelp" => SET(NO_HELP),
                "-S" | "--softwrap" => SET(SOFTWRAP),
                "-m" | "--mouse" => SET(USE_MOUSE),
                "-i" | "--autoindent" => SET(AUTOINDENT),
                "-k" | "--cutfromcursor" => SET(CUT_FROM_CURSOR),
                "-l" | "--linenumbers" => SET(LINE_NUMBERS),
                "-b" | "--boldtext" => SET(BOLD_TEXT),
                "-u" | "--unix" => SET(MAKE_IT_UNIX),
                "-w" | "--nowrap" => SET(NO_WRAP),
                "-c" | "--constantshow" => SET(CONSTANT_SHOW),
                "-p" | "--preserve" => SET(PRESERVE),
                "-A" | "--smarthome" => SET(SMART_HOME),
                "-E" | "--tabstospaces" => SET(TABS_TO_SPACES),
                "-Q" | "--quotestr" => {
                    i += 1;
                    if i < args.len() {
                        // 设置引用字符串模式
                    }
                }
                "-r" | "--fill" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(f) = args[i].parse::<isize>() {
                            with_global_mut(|g| g.fill = f);
                        }
                    }
                }
                "-T" | "--tabsize" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(s) = args[i].parse::<usize>() {
                            with_global_mut(|g| g.tabsize = s);
                            set_tabsize_independent(s);
                        }
                    }
                }
                "-R" | "--restricted" => SET(RESTRICTED),
                "-o" | "--operatingdir" => {
                    i += 1;
                    // 设置操作目录
                }
                "-f" | "--rcfile" => {
                    i += 1;
                    // 指定 rc 文件
                }
                "-K" | "--rebinddelete" => SET(REBIND_DELETE),
                "-s" | "--speller" => {
                    i += 1;
                    // 设置拼写检查器
                }
                "-Y" | "--syntax" => {
                    i += 1;
                    // 设置语法
                }
                "-g" | "--positionlog" => SET(POSITIONLOG),
                "-Z" | "--locking" => SET(LOCKING),
                "-U" | "--quickblank" => SET(QUICK_BLANK),
                "-j" | "--jumpyscrolling" => SET(JUMPY_SCROLLING),
                "-e" | "--emptyline" => SET(EMPTY_LINE),
                "-J" | "--guidestripe" => {
                    i += 1;
                    // 设置引导线
                }
                "-t" | "--saveonexit" => SET(SAVE_ON_EXIT),
                "-0" | "--zero" => SET(ZERO),
                "-M" | "--modernbindings" => SET(MODERN_BINDINGS),
                "-H" | "--historylog" => SET(HISTORYLOG),
                "-B" | "--backup" => SET(MAKE_BACKUP),
                "-C" | "--backupdir" => {
                    i += 1;
                    // 设置备份目录
                }
                "-I" | "--insecurebackup" => SET(INSECURE_BACKUP),
                "-N" | "--noconvert" => SET(NO_CONVERT),
                "-L" | "--nonewlines" => SET(NO_NEWLINES),
                "-X" | "--wordbounds" => SET(WORD_BOUNDS),
                "-W" | "--whitespacedisplay" => SET(WHITESPACE_DISPLAY),
                "-O" | "--colonparsing" => SET(COLON_PARSING),
                "-F" | "--multibuffer" => SET(NEW_BUFFER),
                _ => {
                    if arg == "--" {
                        /* "--" 之后的参数都是文件名，不再解析选项。 */
                        i += 1;
                        if filename.is_none() {
                            filename = args.get(i).cloned();
                        }
                        break;
                    }
                }
            }
        } else {
            /* 文件名参数；继续解析后续选项（对应 GNU getopt 的重排）。 */
            if filename.is_none() {
                filename = Some(arg.clone());
            }
        }
        i += 1;
    }
    filename
}

/// 打印版本信息。
fn print_version() {
    println!("nano-rs version {}", VERSION);
    println!("(Rust translation of GNU nano)");
    println!("Compiled options: --enable-utf8");
}

/// 打印使用说明。
fn print_usage() {
    println!("Usage: nano [OPTIONS] [FILE]");
    println!("");
    println!("GNU nano - a small, friendly text editor");
    println!("");
    println!("Basic options:");
    println!("  -V, --version          Print version information");
    println!("  -h, --help             Print this help message");
    println!("  -v, --view             View mode (read-only)");
    println!("  -x, --nohelp           Hide the help lines");
    println!("  -S, --softwrap         Soft wrap lines");
    println!("  -m, --mouse            Enable mouse");
    println!("  -i, --autoindent       Auto-indent new lines");
    println!("  -k, --cutfromcursor     Cut from cursor to end of line");
    println!("  -l, --linenumbers      Show line numbers");
    println!("  -b, --boldtext         Use bold text");
    println!("  -u, --unix             Save in Unix format");
    println!("  -w, --nowrap           Don't wrap long lines");
    println!("  -c, --constantshow     Constantly show cursor position");
    println!("  -p, --preserve         Preserve XON/XOFF");
    println!("  -A, --smarthome        Smart home key");
    println!("  -E, --tabstospaces     Convert typed tabs to spaces");
    println!("  -T, --tabsize=N        Tab size (default 8)");
    println!("  -r, --fill=N           Target width for wrap (default -2)");
}
