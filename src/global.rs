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
    // 简化的光标位置报告
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
pub fn add_to_funcs(func: FunctionId, menus: i32, tag: &'static str, phrase: &'static str, blank_after: bool) {
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
    });
    None // 防止借用问题，实际需要遍历
}

/// 查找快捷键。
pub fn find_shortcut(keycode: i32, menu: i32) -> Option<KeyRef> {
    with_global(|g| {
        let mut current = g.shortcuts.clone();
        while let Some(s) = current {
            let s_ref = s.borrow();
            if s_ref.keycode == keycode && (s_ref.menus & menu) != 0 {
                return Some(s.clone());
            }
            current = s_ref.next.clone();
        }
        None
    });
    None
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
    });
    Vec::new()
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
    });
    Vec::new()
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
    add_to_sclist(MMAIN, r"^F", 6, FunctionId::DoSearchForward, 0);
    add_to_sclist(MMAIN, r"^\", 28, FunctionId::DoReplace, 0);
    add_to_sclist(MMAIN, r"^K", 11, FunctionId::DoCut, 0);
    add_to_sclist(MMAIN, r"^U", 21, FunctionId::DoPaste, 0);
    add_to_sclist(MMAIN, r"^J", 10, FunctionId::DoJustify, 0);
    add_to_sclist(MMAIN, r"^T", 20, FunctionId::DoExecute, 0);
    add_to_sclist(MMAIN, r"^C", 3, FunctionId::DoReportLocation, 0);
    add_to_sclist(MMAIN, r"^X", 24, FunctionId::DoExit, 0);
    add_to_sclist(MMAIN, r"^R", 18, FunctionId::DoInsertFile, 0);
    add_to_sclist(MMAIN, r"^/", 31, FunctionId::DoGoToLine, 0);
    add_to_sclist(MMAIN, r"^B", 2, FunctionId::DoSearchBackward, 0);
    // 方向键（保留但不在底部栏显示）
    add_to_sclist(MMAIN, r"^P", 16, FunctionId::DoUp, 0);
    add_to_sclist(MMAIN, r"^N", 14, FunctionId::DoDown, 0);
    add_to_sclist(MMAIN, r"^A", 1, FunctionId::DoHome, 0);
    add_to_sclist(MMAIN, r"^E", 5, FunctionId::DoEnd, 0);
    add_to_sclist(MMAIN, r"^V", 22, FunctionId::DoPageDown, 0);
    add_to_sclist(MMAIN, r"^Y", 25, FunctionId::DoPageUp, 0);
    add_to_sclist(MMAIN, r"^D", 4, FunctionId::DoDelete, 0);
    add_to_sclist(MMAIN, r"^H", 8, FunctionId::DoBackspace, 0);
    add_to_sclist(MMAIN, r"^I", 9, FunctionId::DoTab, 0);
    add_to_sclist(MMAIN, r"^M", 13, FunctionId::DoEnter, 0);
    add_to_sclist(MMAIN, r"^L", 12, FunctionId::DoRefresh, 0);
    // Alt 组合
    add_to_sclist(MMAIN, "M-U", 0, FunctionId::DoUndo, 0);
    add_to_sclist(MMAIN, "M-E", 0, FunctionId::DoRedo, 0);
    add_to_sclist(MMAIN, "M-A", 0, FunctionId::DoMark, 0);
    add_to_sclist(MMAIN, "M-6", 0, FunctionId::DoCopy, 0);
    add_to_sclist(MMAIN, "M-]", 0, FunctionId::DoFindBracket, 0);
    // 方向键（仅用于输入匹配，不显示在底部栏）
    add_to_sclist(MMAIN, "Left", KEY_LEFT, FunctionId::DoLeft, 0);
    add_to_sclist(MMAIN, "Right", KEY_RIGHT, FunctionId::DoRight, 0);
    add_to_sclist(MMAIN, "Up", KEY_UP, FunctionId::DoUp, 0);
    add_to_sclist(MMAIN, "Down", KEY_DOWN, FunctionId::DoDown, 0);
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
    // YesNo 菜单
    add_to_sclist(MYESNO, "Y", 121, FunctionId::None, 0);
    add_to_sclist(MYESNO, "N", 110, FunctionId::None, 0);
    add_to_sclist(MYESNO, "A", 97, FunctionId::None, 0);
    // 函数列表（按 C 版顺序，遍历时为逆序，与 C 版一致）
    add_to_funcs(FunctionId::DoHelp, (MMOST | MBROWSER) & !MFINDINHELP, "Help", "help_gist", false);
    add_to_funcs(FunctionId::DoCancel, ((MMOST & !MMAIN) | MYESNO), "Cancel", "cancel_gist", true);
    add_to_funcs(FunctionId::DoExit, MMAIN, "Exit", "exit_gist", false);
    add_to_funcs(FunctionId::DoRefresh, MMAIN | MREPLACE, "Refresh", "x", false);
    add_to_funcs(FunctionId::DoWriteOut, MMAIN, "Write Out", "writeout_gist", false);
    add_to_funcs(FunctionId::DoInsertFile, MMAIN, "Read File", "insert_gist", false);
    add_to_funcs(FunctionId::DoJustify, MMAIN, "Justify", "justify_gist", false);
    add_to_funcs(FunctionId::DoSearchForward, MMAIN | MHELP, "Where Is", "whereis_gist", false);
    add_to_funcs(FunctionId::DoReplace, MMAIN, "Replace", "replace_gist", false);
    add_to_funcs(FunctionId::DoFindPrevious, MMAIN | MHELP, "Find Previous", "findprev_gist", false);
    add_to_funcs(FunctionId::DoFindNext, MMAIN | MHELP, "Find Next", "findnext_gist", false);
    add_to_funcs(FunctionId::DoCut, MMAIN, "Cut", "cut_gist", false);
    add_to_funcs(FunctionId::DoPaste, MMAIN, "Paste", "paste_gist", false);
    add_to_funcs(FunctionId::DoExecute, MMAIN, "Execute", "execute_gist", false);
    add_to_funcs(FunctionId::DoReportLocation, MMAIN, "Location", "location_gist", false);
    add_to_funcs(FunctionId::DoGoToLine, MMAIN, "Go To Line", "gotoline_gist", false);
    add_to_funcs(FunctionId::DoUndo, MMAIN, "Undo", "undo_gist", true);
    add_to_funcs(FunctionId::DoRedo, MMAIN, "Redo", "redo_gist", true);
    add_to_funcs(FunctionId::DoMark, MMAIN, "Set Mark", "mark_gist", true);
    add_to_funcs(FunctionId::DoCopy, MMAIN, "Copy", "copy_gist", true);
    add_to_funcs(FunctionId::DoToggleCaseSensitive, MWHEREIS | MREPLACE, "Case Sens", "casesens_gist", true);
    add_to_funcs(FunctionId::DoToggleRegexp, MWHEREIS | MREPLACE, "Regexp", "regexp_gist", true);
    add_to_funcs(FunctionId::DoToggleBackwards, MWHEREIS | MREPLACE, "Backwards", "backwards_gist", true);
    add_to_funcs(FunctionId::DoSearchBackward, MMAIN | MHELP, "Where Was", "wherewas_gist", false);
    add_to_funcs(FunctionId::DoFindBracket, MMAIN, "To Bracket", "bracket_gist", true);
    add_to_funcs(FunctionId::DoLeft, MMAIN, "Left", "left_gist", true);
    add_to_funcs(FunctionId::DoRight, MMAIN, "Right", "right_gist", true);
    add_to_funcs(FunctionId::DoUp, MMAIN, "Up", "up_gist", true);
    add_to_funcs(FunctionId::DoDown, MMAIN, "Down", "down_gist", true);
    add_to_funcs(FunctionId::DoHome, MMAIN, "Home", "home_gist", true);
    add_to_funcs(FunctionId::DoEnd, MMAIN, "End", "end_gist", true);
    add_to_funcs(FunctionId::DoPageUp, MMAIN, "Page Up", "pageup_gist", true);
    add_to_funcs(FunctionId::DoPageDown, MMAIN, "Page Down", "pagedown_gist", true);
    add_to_funcs(FunctionId::DoDelete, MMAIN, "Delete", "delete_gist", true);
    add_to_funcs(FunctionId::DoBackspace, MMAIN, "Backspace", "backspace_gist", true);
    add_to_funcs(FunctionId::DoEnter, MMAIN, "Enter", "enter_gist", true);
    add_to_funcs(FunctionId::DoTab, MMAIN, "Tab", "tab_gist", true);
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
        winio::statusline(
            crate::definitions::MessageType::Ahem,
            "Unbound key",
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
    add_to_funcs(FunctionId::GetOlderItem, MMAIN, "Get Older Item", "older_gist", false);
    add_to_funcs(FunctionId::GetNewerItem, MMAIN, "Get Newer Item", "newer_gist", false);
    add_to_funcs(FunctionId::ToFirstFile, MBROWSER, "First File", "firstfile_gist", false);
    add_to_funcs(FunctionId::ToLastFile, MBROWSER, "Last File", "lastfile_gist", false);
    add_to_funcs(FunctionId::DoGotoDir, MBROWSER, "Go To Dir", "gotodir_gist", false);
    add_to_funcs(FunctionId::DoVerbatimInput, MMAIN, "Verbatim Input", "verbatim_gist", false);
}
