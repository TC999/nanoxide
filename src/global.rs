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

/// 报告光标位置（对应 winio.c 的 `report_cursor_position`）。
/// 格式：line X/Y (Z%), col X/Y (Z%), char X/Y (Z%)，居中显示。
pub fn report_cursor_position() {
    use crate::utils;
    use crate::winio;

    let report: Option<String> = with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            let of_ref = of.borrow();
            let current = of_ref.current.as_ref()?;
            let filebot = of_ref.filebot.as_ref()?;
            let filetop = of_ref.filetop.as_ref()?;
            let filebot_lineno = filebot.borrow().lineno.max(1);
            let totsize = of_ref.totsize;

            let c_ref = current.borrow();
            let lineno = c_ref.lineno;
            let data = c_ref.data.as_bytes();
            let cur_x = of_ref.current_x;

            /* fullwidth = breadth(current->data) + 1；column = xplustabs() + 1。
             * Rust 版 breadth = mbstrlen，xplustabs 在 current_x 处计算列号，
             * 与原版 keyreport 一致。 */
            let fullwidth = crate::chars::mbstrlen(data) + 1;
            let column = utils::wideness(data, cur_x.min(data.len())) + 1;

            /* sum = number_of_characters_in(filetop, current)：累计每行
             * 字符数加一个换行，最后减 1（不计末尾换行）。 */
            let sum = utils::number_of_characters_in(filetop, current);

            drop(c_ref);
            drop(of_ref);

            let linepct = if filebot_lineno > 0 {
                100 * lineno / filebot_lineno
            } else {
                0
            };
            let colpct = if fullwidth > 0 {
                100 * column / fullwidth
            } else {
                0
            };
            let charpct = if totsize > 0 {
                100 * sum / totsize
            } else {
                0
            };

            /* 数字宽度：与 C 版 digits() 一致。 */
            fn digits(n: usize) -> usize {
                if n == 0 {
                    1
                } else {
                    let mut w = 0;
                    let mut m = n;
                    while m > 0 {
                        m /= 10;
                        w += 1;
                    }
                    w
                }
            }

            let line_w = digits(filebot_lineno.try_into().unwrap());
            let char_w = digits(totsize);

            /* ftl 模板只支持 {argname} 占位符（无宽度语法），
             * 因此把带对齐宽度的数字在 Rust 侧预格式化后再传入，
             * 输出与原版格式串逐字符一致。 */
            Some(crate::t!(
                "winio-cursor_position",
                lineno = format!("{:>line_w$}", lineno),
                filebot_lineno = filebot_lineno,
                linepct = format!("{:>2}", linepct),
                column = format!("{:>2}", column),
                fullwidth = format!("{:>2}", fullwidth),
                colpct = format!("{:>3}", colpct),
                sum = format!("{:>char_w$}", sum),
                totsize = totsize,
                charpct = format!("{:>2}", charpct),
            ))
        })
    });

    if let Some(msg) = report {
        winio::statusline_centered(MessageType::Info, &format!("[ {} ]", msg));
    }
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
/// 返回指定菜单中某函数的第一条快捷键键码（对应 C 的 `first_sc_for`）。
fn first_shortcut_keycode(menu: i32, func: FunctionId) -> Option<i32> {
    let mut shortcut = with_global(|g| g.shortcuts.clone());
    while let Some(node) = shortcut {
        let (menus, f, keycode, next) = {
            let r = node.borrow();
            (r.menus, r.func, r.keycode, r.next.clone())
        };
        if (menus & menu) != 0 && f == func {
            return Some(keycode);
        }
        shortcut = next;
    }
    None
}

/// legacy 键位下提示挂起方式（对应 nano.c 的 `suggest_ctrlT_ctrlZ`）：
/// 仅当主菜单的 Execute 仍是 ^T、执行菜单里 Suspend 是 ^Z 时才提示。
pub fn suggest_ctrlT_ctrlZ() {
    let execute_key = first_shortcut_keycode(MMAIN, FunctionId::DoExecute);
    let suspend_key = first_shortcut_keycode(MEXECUTE, FunctionId::DoSuspend);
    if execute_key == Some(0x14) && suspend_key == Some(0x1A) {
        crate::winio::statusline(MessageType::Ahem, &crate::t!("global-suspend_hint"));
    }
}

pub fn shortcut_init() {
    // 主菜单快捷键（与 C 版 global.c 对应）
    /* 两套键位共用：搜索、剪切/复制/粘贴、替换、插入文件、换行/制表、
     * 居中与全屏刷新（对应 global.c:1240-1242、1283-1289、1463-1467）。 */
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^B", 2, FunctionId::DoSearchBackward, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, r"^F", 6, FunctionId::DoSearchForward, 0);
    add_to_sclist(MMAIN, r"^K", 11, FunctionId::DoCut, 0);
    add_to_sclist(MMAIN, r"^U", 21, FunctionId::DoPaste, 0);
    add_to_sclist(MMAIN, r"^J", 10, FunctionId::DoJustify, 0);
    add_to_sclist(MMAIN, r"^\", 28, FunctionId::DoReplace, 0);
    add_to_sclist(MMAIN, r"^/", 31, FunctionId::DoGoToLine, 0);
    add_to_sclist(MMAIN, r"^M", 13, FunctionId::DoEnter, 0);
    add_to_sclist(MMAIN, r"^I", 9, FunctionId::DoTab, 0);
    add_to_sclist(MMAIN, "M-R", 0x272, FunctionId::DoReplace, 0);
    add_to_sclist(MMAIN, "Ins", KEY_IC, FunctionId::DoInsertFile, 0);
    // 居中/全屏刷新（对应 global.c:1463-1467：MMAIN 是 do_center，其余是 full_refresh）
    add_to_sclist(MMAIN, r"^L", 12, FunctionId::DoCenter, 0);
    add_to_sclist(
        (MMOST & !MMAIN) | MBROWSER | MHELP | MYESNO,
        r"^L",
        12,
        FunctionId::DoFullRefresh,
        0,
    );

    if ISSET(MODERN_BINDINGS) {
        /* modern 键位（对应 global.c:1243-1262 与 1301 行的 ^E）。 */
        add_to_sclist((MMOST | MBROWSER) & !MFINDINHELP, r"^H", 8, FunctionId::DoHelp, 0);
        add_to_sclist(MHELP, r"^H", 8, FunctionId::DoExit, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^Q", 17, FunctionId::DoExit, 0);
        add_to_sclist(MMAIN, r"^S", 19, FunctionId::DoSaveFile, 0);
        add_to_sclist(MMAIN, r"^W", 23, FunctionId::DoWriteOut, 0);
        add_to_sclist(MMAIN, r"^O", 15, FunctionId::DoInsertFile, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^D", 4, FunctionId::DoFindPrevious, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^G", 7, FunctionId::DoFindNext, 0);
        add_to_sclist(MMAIN, r"^R", 18, FunctionId::DoReplace, 0);
        add_to_sclist(MMAIN, r"^T", 20, FunctionId::DoGoToLine, 0);
        add_to_sclist(MMAIN, r"^P", 16, FunctionId::DoReportLocation, 0);
        add_to_sclist(MMAIN, r"^Z", 26, FunctionId::DoUndo, 0);
        add_to_sclist(MMAIN, r"^Y", 25, FunctionId::DoRedo, 0);
        add_to_sclist(MMAIN, r"^A", 1, FunctionId::DoMark, 0);
        add_to_sclist(MMAIN, r"^X", 24, FunctionId::DoCut, 0);
        add_to_sclist(MMAIN, r"^C", 3, FunctionId::DoCopy, 0);
        add_to_sclist(MMAIN, r"^V", 22, FunctionId::DoPaste, 0);
        add_to_sclist(MMAIN, r"^E", 5, FunctionId::DoExecute, 0);
    } else {
        /* legacy 键位（对应 global.c:1264-1282 与 1301 行的 ^T）。 */
        add_to_sclist((MMOST | MBROWSER) & !MFINDINHELP, r"^G", 7, FunctionId::DoHelp, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^X", 24, FunctionId::DoExit, 0);
        /* 保存/反向搜索在未设置 --preserve 时可用（对应 global.c:1266-1271）。 */
        if !ISSET(PRESERVE) {
            add_to_sclist(MMAIN, r"^S", 19, FunctionId::DoSaveFile, 0);
            add_to_sclist(MMAIN | MBROWSER | MHELP, r"^Q", 17, FunctionId::DoSearchBackward, 0);
        }
        add_to_sclist(MMAIN, r"^O", 15, FunctionId::DoWriteOut, 0);
        add_to_sclist(MMAIN, r"^R", 18, FunctionId::DoInsertFile, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^W", 23, FunctionId::DoSearchForward, 0);
        add_to_sclist(MMAIN, r"^A", 1, FunctionId::DoHome, 0);
        add_to_sclist(MMAIN, r"^E", 5, FunctionId::DoEnd, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^P", 16, FunctionId::DoUp, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP, r"^N", 14, FunctionId::DoDown, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP | MLINTER, r"^Y", 25, FunctionId::DoPageUp, 0);
        add_to_sclist(MMAIN | MBROWSER | MHELP | MLINTER, r"^V", 22, FunctionId::DoPageDown, 0);
        add_to_sclist(MMAIN, r"^C", 3, FunctionId::DoReportLocation, 0);
        add_to_sclist(MMAIN, r"^H", 8, FunctionId::DoBackspace, 0);
        add_to_sclist(MMAIN, r"^D", 4, FunctionId::DoDelete, 0);
        add_to_sclist(MMAIN, r"^T", 20, FunctionId::DoExecute, 0);
        /* legacy 的 ^Z 用来提示挂起方式（对应 global.c:1451）。 */
        add_to_sclist(MMAIN, r"^Z", 26, FunctionId::SuggestSuspend, 0);
    }
    // Alt 组合
    add_to_sclist(MMAIN, "M-U", 0, FunctionId::DoUndo, 0);
    add_to_sclist(MMAIN, "M-E", 0, FunctionId::DoRedo, 0);
    add_to_sclist(MMAIN, "M-A", 0, FunctionId::DoMark, 0);
    add_to_sclist(MMAIN, "M-6", 0, FunctionId::DoCopy, 0);
    add_to_sclist(MMAIN, "M-]", 0x25D, FunctionId::DoFindBracket, 0);
    // 切换鼠标支持（对应 C 的 M-M → mouse toggle；切换后同步鼠标捕获）
    add_to_sclist(MMAIN, "M-M", 0x26D, FunctionId::DoToggle, USE_MOUSE as i32);
    add_to_sclist(MMAIN, "^]", 29, FunctionId::DoWordCompletion, 0);
    // 多缓冲区切换（对应 C 版 rcfile 中常见的 prevfile/nextfile 绑定）
    add_to_sclist(MMAIN, "M-,", 0x22C, FunctionId::DoPrevFile, 0);
    add_to_sclist(MMAIN, "M-.", 0x22E, FunctionId::DoNextFile, 0);
    // 帮助页 Previous/Next（对应 C 的 M-B/M-F → do_findprevious/do_findnext）
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-B", 0x262, FunctionId::DoFindPrevious, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-F", 0x266, FunctionId::DoFindNext, 0);
    /* 备用查找键（对应 global.c：M-W → findnext，M-Q → findprevious）。 */
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-W", 0x277, FunctionId::DoFindNext, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "M-Q", 0x271, FunctionId::DoFindPrevious, 0);
    /* 缩进/反缩进：注意 C 版是 M-} 缩进、M-{ 反缩进（global.c:1345-1347）。 */
    add_to_sclist(MMAIN, "M-}", 0x27D, FunctionId::DoIndent, 0);
    add_to_sclist(MMAIN, "M-{", 0x27B, FunctionId::DoUnindent, 0);
    add_to_sclist(MMAIN, "Sh-Tab", SHIFT_TAB, FunctionId::DoUnindent, 0);
    /* 全文对齐与左右滚动（对应 global.c:1457-1462）。 */
    add_to_sclist(MMAIN, "M-J", 0x26A, FunctionId::DoFullJustify, 0);
    add_to_sclist(MMAIN, "M-<", ALT_LESS, FunctionId::DoScrollLeft, 0);
    add_to_sclist(MMAIN, "M->", ALT_GREATER, FunctionId::DoScrollRight, 0);
    /* 字数统计（对应 global.c:1452 的 M-D）。 */
    add_to_sclist(MMAIN, "M-D", 0x264, FunctionId::CountWords, 0);
    /* 锚点与视口行移动（对应 global.c:1356-1362）。 */
    add_to_sclist(MMAIN, "M-Ins", ALT_INSERT, FunctionId::PutOrLiftAnchor, 0);
    add_to_sclist(MMAIN, "M-Home", ALT_HOME, FunctionId::ToTopRow, 0);
    add_to_sclist(MMAIN, "M-End", ALT_END, FunctionId::ToBottomRow, 0);
    add_to_sclist(MMAIN, "M-PgUp", ALT_PAGEUP, FunctionId::ToPrevAnchor, 0);
    add_to_sclist(MMAIN, "M-PgDn", ALT_PAGEDOWN, FunctionId::ToNextAnchor, 0);
    add_to_sclist(MMAIN, "M-\"", 0x222, FunctionId::PutOrLiftAnchor, 0);
    add_to_sclist(MMAIN, "M-'", 0x227, FunctionId::ToNextAnchor, 0);
    /* 功能键（对应 global.c:1574-1595）。MHELP 的 F1 先注册，
     * 保证帮助查看器里 F1 是“退出”而不是“帮助”。 */
    add_to_sclist(MHELP, "F1", KEY_F0 + 1, FunctionId::DoExit, 0);
    add_to_sclist(MMOST & !MFINDINHELP, "F1", KEY_F0 + 1, FunctionId::DoHelp, 0);
    add_to_sclist(MMAIN | MBROWSER | MHELP, "F2", KEY_F0 + 2, FunctionId::DoExit, 0);
    add_to_sclist(MMAIN, "F3", KEY_F0 + 3, FunctionId::DoWriteOut, 0);
    add_to_sclist(MMAIN, "F4", KEY_F0 + 4, FunctionId::DoJustify, 0);
    add_to_sclist(MMAIN, "F5", KEY_F0 + 5, FunctionId::DoInsertFile, 0);
    add_to_sclist(
        MMAIN | MBROWSER | MHELP,
        "F6",
        KEY_F0 + 6,
        FunctionId::DoSearchForward,
        0,
    );
    add_to_sclist(
        MMAIN | MBROWSER | MHELP | MLINTER,
        "F7",
        KEY_F0 + 7,
        FunctionId::DoPageUp,
        0,
    );
    add_to_sclist(
        MMAIN | MBROWSER | MHELP | MLINTER,
        "F8",
        KEY_F0 + 8,
        FunctionId::DoPageDown,
        0,
    );
    add_to_sclist(MMOST, "F9", KEY_F0 + 9, FunctionId::DoCut, 0);
    add_to_sclist(MMOST, "F10", KEY_F0 + 10, FunctionId::DoPaste, 0);
    add_to_sclist(MMAIN, "F11", KEY_F0 + 11, FunctionId::DoReportLocation, 0);
    add_to_sclist(MMAIN, "F12", KEY_F0 + 12, FunctionId::DoSpell, 0);
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
    /* Go To Line / Search 提示菜单条目（对应 C 版 add_to_funcs 中
     * to_para_begin/to_para_end/to_first_line/to_last_line/flip_goto；
     * 尾插保证显示顺序：Help → Cancel → ParaBegin → ParaEnd → FirstLine
     * → LastLine → ToSearch，与 C 版底部快捷键栏布局一致）。 */
    add_to_funcs(FunctionId::DoParaBegin, MMAIN | MGOTOLINE, crate::t!("key-start_of_paragraph"), "parabegin_gist", true);
    add_to_funcs(FunctionId::DoParaEnd, MMAIN | MGOTOLINE, crate::t!("key-end_of_paragraph"), "paraend_gist", false);
    add_to_funcs(FunctionId::DoFirstLine, MMAIN | MHELP | MGOTOLINE, crate::t!("key-first_line"), "firstline_gist", true);
    add_to_funcs(FunctionId::DoLastLine, MMAIN | MHELP | MGOTOLINE, crate::t!("key-last_line"), "lastline_gist", false);
    add_to_funcs(FunctionId::FlipGoto, MGOTOLINE, crate::t!("key-to_search"), "tosearch_gist", false);
    /* 补充其余函数注册（对应 global.c 的 add_to_funcs，供 rcfile bind 的
     * 菜单限制与 check_vitals_mapped 使用）。 */
    add_to_funcs(FunctionId::DoExit, MBROWSER, crate::t!("key-exit"), "exit_gist", false);
    add_to_funcs(FunctionId::DoComment, MMAIN, crate::t!("key-comment"), "comment_gist", false);
    add_to_funcs(FunctionId::DoIndent, MMAIN, crate::t!("key-indent"), "indent_gist", false);
    add_to_funcs(FunctionId::DoUnindent, MMAIN, crate::t!("key-unindent"), "unindent_gist", false);
    add_to_funcs(FunctionId::DoScrollLeft, MMAIN, crate::t!("key-scroll_left"), "scrollleft_gist", false);
    add_to_funcs(FunctionId::DoScrollRight, MMAIN, crate::t!("key-scroll_right"), "scrollright_gist", false);
    add_to_funcs(FunctionId::DoPrevWord, MMAIN, crate::t!("key-prev_word"), "prevword_gist", false);
    add_to_funcs(FunctionId::DoNextWord, MMAIN, crate::t!("key-next_word"), "nextword_gist", false);
    add_to_funcs(FunctionId::ChopPrevWord, MMAIN, crate::t!("key-chop_left"), "chopwordleft_gist", false);
    add_to_funcs(FunctionId::ChopNextWord, MMAIN, crate::t!("key-chop_right"), "chopwordright_gist", false);
    add_to_funcs(FunctionId::DoFullJustify, MMAIN, crate::t!("key-full_justify"), "fulljustify_gist", false);
    add_to_funcs(FunctionId::CountWords, MMAIN, crate::t!("key-word_count"), "wordcount_gist", false);
    add_to_funcs(FunctionId::DoVerbatimInput, MMAIN, crate::t!("key-verbatim_input"), "verbatim_gist", false);
    add_to_funcs(FunctionId::DoRecordMacro, MMAIN, crate::t!("key-record_macro"), "recordmacro_gist", false);
    add_to_funcs(FunctionId::DoRunMacro, MMAIN, crate::t!("key-run_macro"), "runmacro_gist", false);
    add_to_funcs(FunctionId::DoZap, MMAIN, crate::t!("key-zap"), "zap_gist", false);
    add_to_funcs(FunctionId::PutOrLiftAnchor, MMAIN, crate::t!("key-anchor"), "anchor_gist", false);
    add_to_funcs(FunctionId::ToPrevAnchor, MMAIN, crate::t!("key-up_to_anchor"), "prevanchor_gist", false);
    add_to_funcs(FunctionId::ToNextAnchor, MMAIN, crate::t!("key-down_to_anchor"), "nextanchor_gist", false);
    add_to_funcs(FunctionId::DoSpell, MMAIN, crate::t!("key-spell"), "spell_gist", false);
    add_to_funcs(FunctionId::DoLinter, MMAIN, crate::t!("key-linter"), "linter_gist", false);
    add_to_funcs(FunctionId::DoFormatter, MMAIN, crate::t!("key-formatter"), "formatter_gist", false);
    add_to_funcs(FunctionId::DoSuspend, MMAIN, crate::t!("key-suspend"), "suspend_gist", false);
    add_to_funcs(FunctionId::DoCenter, MMAIN, crate::t!("key-center"), "center_gist", false);
    add_to_funcs(FunctionId::DoCycle, MMAIN, crate::t!("key-cycle"), "cycle_gist", false);
    add_to_funcs(FunctionId::DoSaveFile, MMAIN, crate::t!("key-savefile"), "savefile_gist", false);
    add_to_funcs(FunctionId::DoWordCompletion, MMAIN, crate::t!("key-complete"), "complete_gist", false);
    add_to_funcs(FunctionId::DoAnchor, MMAIN, crate::t!("key-anchor"), "anchor_gist", false);
    add_to_funcs(FunctionId::DoPrevFile, MMAIN, crate::t!("key-prevfile"), "prevbuf_gist", false);
    add_to_funcs(FunctionId::DoNextFile, MMAIN, crate::t!("key-nextfile"), "nextbuf_gist", false);
    add_to_funcs(FunctionId::CaseSensVoid, MWHEREIS | MREPLACE, crate::t!("key-case_sens"), "casesens_gist", false);
    add_to_funcs(FunctionId::RegexpVoid, MWHEREIS | MREPLACE, crate::t!("key-regexp"), "regexp_gist", false);
    add_to_funcs(FunctionId::BackwardsVoid, MWHEREIS | MREPLACE, crate::t!("key-backwards"), "backwards_gist", false);
    add_to_funcs(FunctionId::FlipReplace, MWHEREIS | MREPLACE, crate::t!("key-replace"), "flipreplace_gist", false);
    add_to_funcs(FunctionId::GetOlderItem, MWHEREIS | MREPLACE | MREPLACEWITH | MWHEREISFILE | MEXECUTE, crate::t!("key-older"), "older_gist", false);
    add_to_funcs(FunctionId::GetNewerItem, MWHEREIS | MREPLACE | MREPLACEWITH | MWHEREISFILE | MEXECUTE, crate::t!("key-newer"), "newer_gist", false);
    add_to_funcs(FunctionId::DosFormat, MWRITEFILE, crate::t!("key-dos_format"), "dosformat_gist", false);
    add_to_funcs(FunctionId::BackItUp, MWRITEFILE, crate::t!("key-backup_file"), "backup_gist", false);
    add_to_funcs(FunctionId::AppendIt, MWRITEFILE, crate::t!("key-append"), "append_gist", false);
    add_to_funcs(FunctionId::PrependIt, MWRITEFILE, crate::t!("key-prepend"), "prepend_gist", false);
    add_to_funcs(FunctionId::FlipConvert, MINSERTFILE, crate::t!("key-no_conversion"), "flipconvert_gist", false);
    add_to_funcs(FunctionId::FlipExecute, MINSERTFILE, crate::t!("key-execute_command"), "flipexecute_gist", false);
    add_to_funcs(FunctionId::FlipNewBuffer, MINSERTFILE | MEXECUTE, crate::t!("key-new_buffer"), "flipnewbuffer_gist", false);
    add_to_funcs(FunctionId::FlipPipe, MEXECUTE, crate::t!("key-pipe_text"), "flippipe_gist", false);
    add_to_funcs(FunctionId::ToFiles, MBROWSER, crate::t!("key-to_files"), "tofiles_gist", false);
    add_to_funcs(FunctionId::GotoDir, MBROWSER, crate::t!("key-go_to_dir"), "gotodir_gist", false);
    add_to_funcs(FunctionId::ToFirstFile, MBROWSER, crate::t!("key-first_file"), "firstfile_gist", false);
    add_to_funcs(FunctionId::ToLastFile, MBROWSER, crate::t!("key-last_file"), "lastfile_gist", false);
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
/// Alt+`<` / Alt+`>`：crossterm 把 Alt+字符统一编码为 0x200 + 字符。
const ALT_LESS: i32 = 0x200 + b'<' as i32;
const ALT_GREATER: i32 = 0x200 + b'>' as i32;

/// 注册提示/浏览器/其他菜单的快捷键（对应 global.c 的其余部分）。
pub fn shortcut_init_rest() {
    // Go To Line / Search 菜单专属快捷键（对应 C 版 global.c 的
    // MGOTOLINE 相关绑定；须在共享提示绑定之前，保证优先匹配）。
    add_to_sclist(MWHEREIS | MGOTOLINE, "^T", 20, FunctionId::FlipGoto, 0);
    add_to_sclist(MGOTOLINE, "^W", 23, FunctionId::DoParaBegin, 0);
    add_to_sclist(MGOTOLINE, "^O", 15, FunctionId::DoParaEnd, 0);
    add_to_sclist(MGOTOLINE | MWHEREIS, "^Y", 25, FunctionId::DoFirstLine, 0);
    add_to_sclist(MGOTOLINE | MWHEREIS, "^V", 22, FunctionId::DoLastLine, 0);

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
    add_to_sclist(mmi, "M-Del", ALT_DELETE, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "M-V", 0x256, FunctionId::DoVerbatimInput, 0);
    add_to_sclist(mmi, "^K", 11, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "^U", 21, FunctionId::DoPaste, 0);
    add_to_sclist(mmi, "^H", 8, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "^D", 4, FunctionId::DoDelete, 0);
    add_to_sclist(mmi, "Bsp", KEY_BACKSPACE, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "Sh-Del", SHIFT_DELETE, FunctionId::DoBackspace, 0);
    add_to_sclist(mmi, "Del", KEY_DC, FunctionId::DoDelete, 0);
    add_to_sclist(mmi, "^I", 9, FunctionId::DoTab, 0);

    /* 写文件提示（对应 global.c:1542-1550）：格式/备份/追加/前置/放弃。 */
    add_to_sclist(MWRITEFILE, "^Q", 17, FunctionId::DiscardBuffer, 0);
    add_to_sclist(MWRITEFILE, "M-D", 0x264, FunctionId::DosFormat, 0);
    add_to_sclist(MWRITEFILE, "M-B", 0x262, FunctionId::BackItUp, 0);
    add_to_sclist(MWRITEFILE, "M-A", 0x261, FunctionId::AppendIt, 0);
    add_to_sclist(MWRITEFILE, "M-P", 0x270, FunctionId::PrependIt, 0);
    add_to_sclist(MWRITEFILE, "^T", 20, FunctionId::ToFiles, 0);

    /* 插入/执行提示的翻转键（对应 global.c:1551-1559）。 */
    add_to_sclist(MINSERTFILE | MEXECUTE, "^X", 24, FunctionId::FlipExecute, 0);
    add_to_sclist(MINSERTFILE, "M-N", 0x26E, FunctionId::FlipConvert, 0);
    add_to_sclist(MINSERTFILE | MEXECUTE, "M-F", 0x266, FunctionId::FlipNewBuffer, 0);
    add_to_sclist(MEXECUTE, "M-\\", 0x25C, FunctionId::FlipPipe, 0);

    /* Linter：执行菜单里的 ^Y 调用，linter 模式里 ^X 退出
     * （对应 global.c:1312、1579）。 */
    add_to_sclist(MEXECUTE, "^Y", 25, FunctionId::DoLinter, 0);
    /* 执行菜单里 ^Z 挂起（对应 global.c:1450）。 */
    add_to_sclist(MEXECUTE, "^Z", 26, FunctionId::DoSuspend, 0);
    add_to_sclist(MLINTER, "^X", 24, FunctionId::DoCancel, 0);

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
    add_to_sclist(MBROWSER, "M-<", ALT_LESS, FunctionId::ToFirstFile, 0);
    add_to_sclist(MBROWSER, "M->", ALT_GREATER, FunctionId::ToLastFile, 0);
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

/// 命令行解析结果（对应 C 版 main() 的局部变量 + 剩余的 argv）。
pub struct ParsedCommandLine {
    /// 文件参数（含 +LINE[,COLUMN] 与 +/pattern 关联信息）。
    pub files: Vec<FileArg>,
    /// -b 置 1、-w 置 0，默认 -2（对应 C 版 main 的 hardwrap 三态）。
    pub hardwrap: i32,
    /// -I：不读任何 rc 文件（对应 C 版 main 的 ignore_rcfiles）。
    pub ignore_rcfiles: bool,
    /// -z：列出可用语法后退出。
    pub list_syntaxes: bool,
    /// 遇到 -z 时是否应读取 rc 文件。C 版在 -z 分支里按当时的 -I 状态
    /// 立即调用 do_rcfiles()，故 `-I -z` 不读、`-z -I` 要读。
    pub list_syntaxes_reads_rc: bool,
}

/// 长选项表（对应 nano.c 的 long_options[]）。
struct LongOption {
    name: &'static str,
    has_arg: bool,
    ch: i32,
}

#[rustfmt::skip]
const LONG_OPTIONS: &[LongOption] = &[
    LongOption { name: "boldtext", has_arg: false, ch: b'D' as i32 },
    LongOption { name: "multibuffer", has_arg: false, ch: b'F' as i32 },
    LongOption { name: "newbuffer", has_arg: false, ch: b'F' as i32 },
    LongOption { name: "ignorercfiles", has_arg: false, ch: b'I' as i32 },
    LongOption { name: "rawsequences", has_arg: false, ch: b'K' as i32 },
    LongOption { name: "trimblanks", has_arg: false, ch: b'M' as i32 },
    LongOption { name: "quotestr", has_arg: true, ch: b'Q' as i32 },
    LongOption { name: "restricted", has_arg: false, ch: b'R' as i32 },
    LongOption { name: "quickblank", has_arg: false, ch: b'U' as i32 },
    LongOption { name: "version", has_arg: false, ch: b'V' as i32 },
    LongOption { name: "syntax", has_arg: true, ch: b'Y' as i32 },
    LongOption { name: "breaklonglines", has_arg: false, ch: b'b' as i32 },
    LongOption { name: "constantshow", has_arg: false, ch: b'c' as i32 },
    LongOption { name: "rebinddelete", has_arg: false, ch: b'd' as i32 },
    LongOption { name: "rcfile", has_arg: true, ch: b'f' as i32 },
    LongOption { name: "showcursor", has_arg: false, ch: b'g' as i32 },
    LongOption { name: "help", has_arg: false, ch: b'h' as i32 },
    LongOption { name: "linenumbers", has_arg: false, ch: b'l' as i32 },
    LongOption { name: "mouse", has_arg: false, ch: b'm' as i32 },
    LongOption { name: "operatingdir", has_arg: true, ch: b'o' as i32 },
    LongOption { name: "preserve", has_arg: false, ch: b'p' as i32 },
    LongOption { name: "fill", has_arg: true, ch: b'r' as i32 },
    LongOption { name: "speller", has_arg: true, ch: b's' as i32 },
    LongOption { name: "saveonexit", has_arg: false, ch: b't' as i32 },
    LongOption { name: "view", has_arg: false, ch: b'v' as i32 },
    LongOption { name: "nowrap", has_arg: false, ch: b'w' as i32 },
    LongOption { name: "nohelp", has_arg: false, ch: b'x' as i32 },
    LongOption { name: "listsyntaxes", has_arg: false, ch: b'z' as i32 },
    LongOption { name: "modernbindings", has_arg: false, ch: b'/' as i32 },
    LongOption { name: "smarthome", has_arg: false, ch: b'A' as i32 },
    LongOption { name: "backup", has_arg: false, ch: b'B' as i32 },
    LongOption { name: "backupdir", has_arg: true, ch: b'C' as i32 },
    LongOption { name: "tabstospaces", has_arg: false, ch: b'E' as i32 },
    LongOption { name: "locking", has_arg: false, ch: b'G' as i32 },
    LongOption { name: "historylog", has_arg: false, ch: b'H' as i32 },
    LongOption { name: "guidestripe", has_arg: true, ch: b'J' as i32 },
    LongOption { name: "nonewlines", has_arg: false, ch: b'L' as i32 },
    LongOption { name: "noconvert", has_arg: false, ch: b'N' as i32 },
    LongOption { name: "bookstyle", has_arg: false, ch: b'O' as i32 },
    LongOption { name: "positionlog", has_arg: false, ch: b'P' as i32 },
    LongOption { name: "softwrap", has_arg: false, ch: b'S' as i32 },
    LongOption { name: "tabsize", has_arg: true, ch: b'T' as i32 },
    LongOption { name: "wordbounds", has_arg: false, ch: b'W' as i32 },
    LongOption { name: "wordchars", has_arg: true, ch: b'X' as i32 },
    LongOption { name: "zap", has_arg: false, ch: b'Z' as i32 },
    LongOption { name: "atblanks", has_arg: false, ch: b'a' as i32 },
    LongOption { name: "emptyline", has_arg: false, ch: b'e' as i32 },
    LongOption { name: "autoindent", has_arg: false, ch: b'i' as i32 },
    LongOption { name: "jumpyscrolling", has_arg: false, ch: b'j' as i32 },
    LongOption { name: "cutfromcursor", has_arg: false, ch: b'k' as i32 },
    LongOption { name: "noread", has_arg: false, ch: b'n' as i32 },
    LongOption { name: "indicator", has_arg: false, ch: b'q' as i32 },
    LongOption { name: "unix", has_arg: false, ch: b'u' as i32 },
    LongOption { name: "afterends", has_arg: false, ch: b'y' as i32 },
    LongOption { name: "whitespacedisplay", has_arg: false, ch: 0xCC },
    LongOption { name: "colonparsing", has_arg: false, ch: b'@' as i32 },
    LongOption { name: "stateflags", has_arg: false, ch: b'%' as i32 },
    LongOption { name: "minibar", has_arg: false, ch: b'_' as i32 },
    LongOption { name: "zero", has_arg: false, ch: b'0' as i32 },
    LongOption { name: "solosidescroll", has_arg: false, ch: b'1' as i32 },
];

/// 短选项是否带参数（对应 getopt 字符串里带 ':' 的字母）。
fn short_takes_argument(c: char) -> bool {
    matches!(c, 'C' | 'J' | 'Q' | 'T' | 'X' | 'Y' | 'f' | 'o' | 'r' | 's')
}

/// 是否为 getopt 字符串中存在的短选项。
/// 字母 A-Z/a-z 全在表中（部分带参数），另加 `@ % _ 0 1 /`。
/// 注意：9.2 的 `-!` 只在编译启用 libmagic 时存在；本项目没有 libmagic，
/// 因此与未启用 libmagic 的 nano 一致，不把 `!` 当作有效短选项。
fn is_short_option(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '@' | '%' | '_' | '0' | '1' | '/')
}

/// 输出 getopt 风格的错误，再按 C 版 default 分支提示 `-h` 并退出 1。
fn command_line_error(program: &str, message: &str) -> ! {
    eprintln!("{}: {}", program, message);
    println!("{}", crate::t!("cmdline-type_help", arg = program));
    std::process::exit(1);
}

fn invalid_guide_column(value: &str) -> ! {
    eprintln!("{}", crate::t!("rcfile-guide_invalid", arg = value));
    std::process::exit(1);
}

fn invalid_tab_size(value: &str) -> ! {
    eprintln!("{}", crate::t!("rcfile-tabsize_invalid", arg = value));
    std::process::exit(1);
}

fn invalid_fill_size(value: &str) -> ! {
    eprintln!("{}", crate::t!("rcfile-fill_invalid", arg = value));
    std::process::exit(1);
}

/// 解析一个长选项（允许 GNU getopt_long 的唯一前缀缩写）。
fn resolve_long_option(program: &str, name: &str) -> &'static LongOption {
    if let Some(exact) = LONG_OPTIONS.iter().find(|opt| opt.name == name) {
        return exact;
    }

    let matches: Vec<&LongOption> = LONG_OPTIONS
        .iter()
        .filter(|opt| opt.name.starts_with(name))
        .collect();

    match matches.len() {
        0 => command_line_error(program, &format!("unrecognized option '--{}'", name)),
        1 => matches[0],
        _ => command_line_error(program, &format!("option '--{}' is ambiguous", name)),
    }
}

/// 应用一个命令行选项（对应 C 版 getopt 循环里的 switch 分支）。
fn apply_option(ch: i32, value: Option<String>, state: &mut ParsedCommandLine) {
    let value = value.unwrap_or_default();
    match ch as u8 {
        b'A' => SET(SMART_HOME),
        b'B' => SET(MAKE_BACKUP),
        b'C' => {
            with_global_mut(|g| {
                g.backup_dir = Some(value.clone());
                g.cmdline_backup_dir = Some(value.clone());
            });
        }
        b'D' => SET(BOLD_TEXT),
        b'E' => SET(TABS_TO_SPACES),
        b'F' => SET(NEW_BUFFER),
        b'G' => SET(LOCKING),
        b'H' => SET(HISTORYLOG),
        b'I' => state.ignore_rcfiles = true,
        b'J' => {
            let mut column: isize = 0;
            if !crate::utils::parse_num(&value, &mut column) || column <= 0 {
                invalid_guide_column(&value);
            }
            with_global_mut(|g| {
                g.stripe_column = column as usize;
                g.cmdline_stripe_column = Some(column as usize);
            });
        }
        b'K' => SET(RAW_SEQUENCES),
        b'L' => SET(NO_NEWLINES),
        b'M' => SET(TRIM_BLANKS),
        b'N' => SET(NO_CONVERT),
        b'O' => SET(BOOKSTYLE),
        b'P' => SET(POSITIONLOG),
        b'Q' => {
            with_global_mut(|g| {
                g.quotestr = Some(value.clone());
                g.cmdline_quotestr = Some(value.clone());
            });
        }
        b'R' => SET(RESTRICTED),
        b'S' => SET(SOFTWRAP),
        b'T' => {
            let mut size: isize = 0;
            if !crate::utils::parse_num(&value, &mut size) || size <= 0 {
                invalid_tab_size(&value);
            }
            with_global_mut(|g| {
                g.tabsize = size as usize;
                g.cmdline_tabsize = Some(size as usize);
            });
            set_tabsize_independent(size as usize);
        }
        b'U' => SET(QUICK_BLANK),
        b'V' => {
            print_version();
            std::process::exit(0);
        }
        b'W' => SET(WORD_BOUNDS),
        b'X' => {
            with_global_mut(|g| {
                g.word_chars = Some(value.clone());
                g.cmdline_word_chars = Some(value.clone());
            });
        }
        b'Y' => with_global_mut(|g| g.syntaxstr = Some(value.clone())),
        b'Z' => SET(LET_THEM_ZAP),
        b'a' => SET(AT_BLANKS),
        b'b' => state.hardwrap = 1,
        b'c' => SET(CONSTANT_SHOW),
        b'd' => SET(REBIND_DELETE),
        b'e' => SET(EMPTY_LINE),
        b'f' => with_global_mut(|g| g.custom_nanorc = Some(value.clone())),
        b'g' => SET(SHOW_CURSOR),
        b'h' => {
            print_usage();
            std::process::exit(0);
        }
        b'i' => SET(AUTOINDENT),
        b'j' => SET(JUMPY_SCROLLING),
        b'k' => SET(CUT_FROM_CURSOR),
        b'l' => SET(LINE_NUMBERS),
        b'm' => SET(USE_MOUSE),
        b'n' => SET(NOREAD_MODE),
        b'o' => {
            with_global_mut(|g| {
                g.operating_dir = Some(value.clone());
                g.cmdline_operating_dir = Some(value.clone());
            });
        }
        b'p' => SET(PRESERVE),
        b'q' => SET(INDICATOR),
        b'r' => {
            let mut fill: isize = 0;
            if !crate::utils::parse_num(&value, &mut fill) {
                invalid_fill_size(&value);
            }
            with_global_mut(|g| {
                g.fill = fill;
                g.cmdline_fill = Some(fill);
            });
        }
        b's' => {
            with_global_mut(|g| {
                g.speller = Some(value.clone());
                g.cmdline_speller = Some(value.clone());
            });
        }
        b't' => SET(SAVE_ON_EXIT),
        b'u' => SET(MAKE_IT_UNIX),
        b'v' => SET(VIEW_MODE),
        b'w' => state.hardwrap = 0,
        b'x' => SET(NO_HELP),
        b'y' => SET(AFTER_ENDS),
        b'z' => {
            state.list_syntaxes = true;
            state.list_syntaxes_reads_rc = !state.ignore_rcfiles;
        }
        0xCC => SET(WHITESPACE_DISPLAY),
        b'@' => SET(COLON_PARSING),
        b'%' => SET(STATEFLAGS),
        b'_' => SET(MINIBAR),
        b'0' => SET(ZERO),
        b'1' => SET(SOLO_SIDESCROLL),
        b'/' => SET(MODERN_BINDINGS),
        _ => {}
    }
}

/// 把一个操作数（文件名或 `+定位`）加入结果（对应 C 版 main 的文件循环）。
fn add_operand(
    arg: &str,
    files: &mut Vec<FileArg>,
    givenline: &mut isize,
    givencol: &mut isize,
    searchstring: &mut Option<String>,
) {
    if let Some(rest) = arg.strip_prefix('+') {
        let bs = rest.as_bytes();
        /* 可选的搜索修饰符（可为空）：c/C/r/R（对应 C 版字母循环）。 */
        let mut n = 0;
        while n < bs.len() && bs[n].is_ascii_alphabetic() {
            match bs[n] {
                b'c' => SET(CASE_SENSITIVE),
                b'C' => UNSET(CASE_SENSITIVE),
                b'r' => SET(USE_REGEXP),
                b'R' => UNSET(USE_REGEXP),
                _ => crate::winio::statusline(
                    MessageType::Alert,
                    &crate::t!("search-invalid_modifier", ch = (bs[n] as char).to_string()),
                ),
            }
            n += 1;
        }
        if n < bs.len() && (bs[n] == b'/' || bs[n] == b'?') {
            /* 搜索串参数：+/pattern（正向），+?pattern（反向）。 */
            if n + 1 < bs.len() {
                let pattern = rest[n + 1..].to_string();
                if bs[n] == b'?' {
                    SET(BACKWARDS_SEARCH);
                } else {
                    UNSET(BACKWARDS_SEARCH);
                }
                *searchstring = Some(pattern);
            } else {
                crate::winio::statusline(
                    MessageType::Alert,
                    &crate::t!("search-empty_search_string"),
                );
            }
        } else if rest.is_empty() {
            /* "+" 单独出现：到文件末尾。 */
            *givenline = -1;
        } else {
            /* 无搜索串：按行/列解析（含修饰字母而无 /? 时会失败提示，
             * 与 C 版 parse_line_column 行为一致）。 */
            let mut line = *givenline;
            let mut col = *givencol;
            if crate::utils::parse_line_column(rest, &mut line, &mut col) {
                *givenline = line;
                *givencol = col;
            } else {
                crate::winio::statusline(
                    MessageType::Alert,
                    &crate::t!("search-invalid_line_or_column"),
                );
            }
        }
        return;
    }

    /* 文件名。"-" 表示从标准输入读取（对应 C 版 argv == "-" 时
     * 调用 scoop_stdin 的分支，由 main 处理）。 */
    let mut name = arg.to_string();
    let mut line = *givenline;
    let mut col = *givencol;

    /* colon-parsing：文件名含冒号、无定位且该文件不存在时，尝试
     * 剥掉尾部的 ":行[:列]"（对应 C 版 COLON_PARSING 分支）。 */
    if name != "-"
        && ISSET(COLON_PARSING)
        && line == 0
        && col == 0
        && name.contains(':')
        && !std::path::Path::new(&name).exists()
    {
        if let Some((realname, l, c)) = colon_split(&name) {
            name = realname;
            line = l;
            col = c;
        }
    }

    files.push(FileArg {
        name,
        line,
        col,
        search: searchstring.take(),
    });
    *givenline = 0;
    *givencol = 0;
}

/// 解析整条命令行：先按 getopt_long 的语义处理所有选项（GNU getopt 会重排
/// argv，因此选项可出现在文件名之后），再按原相对顺序处理文件参数。
pub fn parse_command_line(args: &[String]) -> ParsedCommandLine {
    let program = args.first().cloned().unwrap_or_else(|| "nax".to_string());
    let mut state = ParsedCommandLine {
        files: Vec::new(),
        hardwrap: -2,
        ignore_rcfiles: false,
        list_syntaxes: false,
        list_syntaxes_reads_rc: false,
    };
    let mut operands: Vec<String> = Vec::new();
    let mut no_more_options = false;
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];

        if no_more_options || arg == "-" || !arg.starts_with('-') {
            operands.push(arg.clone());
        } else if arg == "--" {
            no_more_options = true;
        } else if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (long, None),
            };
            let option = resolve_long_option(&program, name);

            if option.has_arg {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(value) => value.clone(),
                            None => command_line_error(
                                &program,
                                &format!("option '--{}' requires an argument", option.name),
                            ),
                        }
                    }
                };
                apply_option(option.ch, Some(value), &mut state);
            } else {
                if inline.is_some() {
                    command_line_error(
                        &program,
                        &format!("option '--{}' doesn't allow an argument", option.name),
                    );
                }
                apply_option(option.ch, None, &mut state);
            }
        } else {
            /* 短选项簇：-ES、-T4、-T 4 均受支持。 */
            let chars: Vec<char> = arg[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                let option = chars[j];
                if !is_short_option(option) {
                    command_line_error(
                        &program,
                        &format!("invalid option -- '{}'", option),
                    );
                }
                if short_takes_argument(option) {
                    let rest: String = chars[j + 1..].iter().collect();
                    let value = if !rest.is_empty() {
                        rest
                    } else {
                        i += 1;
                        match args.get(i) {
                            Some(value) => value.clone(),
                            None => command_line_error(
                                &program,
                                &format!("option requires an argument -- '{}'", option),
                            ),
                        }
                    };
                    apply_option(option as i32, Some(value), &mut state);
                    break;
                }
                apply_option(option as i32, None, &mut state);
                j += 1;
            }
        }

        i += 1;
    }

    let mut givenline = 0isize;
    let mut givencol = 0isize;
    let mut searchstring: Option<String> = None;
    for operand in &operands {
        add_operand(
            operand,
            &mut state.files,
            &mut givenline,
            &mut givencol,
            &mut searchstring,
        );
    }

    state
}

/// 解析命令行参数（兼容旧接口）：返回第一个文件名。
pub fn parse_args(args: &[String]) -> Option<String> {
    parse_command_line(args).files.first().map(|file| file.name.clone())
}

/// 解析命令行中的文件参数，返回 (文件名, 行号, 列号) 列表。
/// 支持 `+LINE[,COLUMN]` 定位参数（作用于其后的文件）与多个文件名
/// （对应 C 版 main() 的多文件处理）。
/// 一个命令行文件参数：文件名（"-" 表示从标准输入读取）、定位行/列、
/// 以及命令行搜索串（"+c/pattern" 形式，对应 C 版 searchstring）。
pub struct FileArg {
    pub name: String,
    pub line: isize,
    pub col: isize,
    pub search: Option<String>,
}

/// 尝试按 `文件名:行[:列]` 解释命令行参数（对应 C 版 COLON_PARSING 的
/// 截断逻辑）：仅当给定文件不存在且串中含冒号时，从尾部剥掉 ":数字"（至多
/// 两段，对应 C 的 maybe_two 再试一次）。成功返回 (真实文件名, 行, 列)。
fn colon_split(filename: &str) -> Option<(String, isize, isize)> {
    let s = filename.as_bytes();
    /* coda 与 C 版指针语义一致：从串尾向左扫描到数字簇前的 ':'。 */
    let mut coda = s.len();
    loop {
        /* while (--coda > filename + 1 && isdigit(*coda)) */
        loop {
            if coda <= 2 {
                coda = 1;
                break;
            }
            coda -= 1;
            if !s[coda].is_ascii_digit() {
                break;
            }
        }
        if coda <= 1 || coda + 1 >= s.len() || s[coda] != b':' {
            return None;
        }
        if !s[coda + 1].is_ascii_digit() {
            return None;
        }

        let base = &filename[..coda];
        let digits = &filename[coda + 1..];

        /* 剥掉 ":数字" 后文件存在：解析行/列。 */
        if std::path::Path::new(base).exists() {
            let mut line = 0isize;
            let mut col = 0isize;
            if crate::utils::parse_line_column(digits, &mut line, &mut col) {
                return Some((base.to_string(), line, col));
            } else {
                crate::definitions::die(&crate::t!("search-invalid_number"));
            }
        }

        /* 该簇无对应文件：当数字区之后不再含冒号时再向左试更早的冒号簇
         * （对应 C 版 !strchr(coda + 1, ':') 时 goto maybe_two）。 */
        if filename[coda + 1..].contains(':') {
            return None;
        }
    }
}

/// 解析命令行中的“文件参数”（含 +LINE[,COLUMN] 定位与 +c/pattern
/// 搜索参数，对应 C 版 main 的文件读取循环）。
pub fn parse_file_args(args: &[String]) -> Vec<FileArg> {
    parse_command_line(args).files
}
/// 打印版本信息。
fn print_version() {
    println!("nanoxide version {}", env!("CARGO_PKG_VERSION"));
    println!("(Rust translation of GNU nano)");
    println!("Compiled options: --enable-utf8");
}

/// 打印使用说明（对应 C 版 nano.c 的 usage()）。
fn print_usage() {
    let print_opt = |short: &str, long: &str, description: &str| {
        println!(" {:<14} {:<24}{}", short, long, description);
    };

    println!("Usage: nax [OPTIONS] [[+LINE[,COLUMN]] FILE]...");
    println!();
    println!("To place the cursor on a specific line of a file, put the line number with");
    println!("a '+' before the filename.  The column number can be added after a comma.");
    println!("When a filename is '-', nax reads data from standard input.");
    println!();
    print_opt("Option", "Long option", "Meaning");
    print_opt("-A", "--smarthome", "Enable smart home key");
    if !ISSET(RESTRICTED) {
        print_opt("-B", "--backup", "Save backups of existing files");
        print_opt("-C <dir>", "--backupdir=<dir>", "Directory for saving unique backup files");
    }
    print_opt("-D", "--boldtext", "Use bold instead of reverse video text");
    print_opt("-E", "--tabstospaces", "Convert typed tabs to spaces");
    if !ISSET(RESTRICTED) {
        print_opt("-F", "--newbuffer", "Read a file into a new buffer by default");
    }
    print_opt("-G", "--locking", "Use (vim-style) lock files");
    if !ISSET(RESTRICTED) {
        print_opt("-H", "--historylog", "Save & reload old search/replace strings");
    }
    print_opt("-I", "--ignorercfiles", "Don't look at nanorc files");
    print_opt("-J <number>", "--guidestripe=<number>", "Show a guiding bar at this column");
    print_opt("-K", "--rawsequences", "Fix numeric keypad key confusion problem");
    print_opt("-L", "--nonewlines", "Don't add an automatic newline");
    print_opt("-M", "--trimblanks", "Trim tail spaces when hard-wrapping");
    print_opt("-N", "--noconvert", "Don't convert files from DOS format");
    print_opt("-O", "--bookstyle", "Leading whitespace means new paragraph");
    if !ISSET(RESTRICTED) {
        print_opt("-P", "--positionlog", "Save & restore position of the cursor");
    }
    print_opt("-Q <regex>", "--quotestr=<regex>", "Regular expression to match quoting");
    if !ISSET(RESTRICTED) {
        print_opt("-R", "--restricted", "Restrict access to the filesystem");
    }
    print_opt("-S", "--softwrap", "Display overlong lines on multiple rows");
    print_opt("-T <number>", "--tabsize=<number>", "Make a tab this number of columns wide");
    print_opt("-U", "--quickblank", "Wipe status bar upon next keystroke");
    print_opt("-V", "--version", "Print version information and exit");
    print_opt("-W", "--wordbounds", "Detect word boundaries more accurately");
    print_opt("-X <string>", "--wordchars=<string>", "Which other characters are word parts");
    print_opt("-Y <name>", "--syntax=<name>", "Syntax definition to use for coloring");
    print_opt("-Z", "--zap", "Let Bsp and Del erase a marked region");
    print_opt("-a", "--atblanks", "When soft-wrapping, do it at whitespace");
    print_opt("-b", "--breaklonglines", "Automatically hard-wrap overlong lines");
    print_opt("-c", "--constantshow", "Constantly show cursor position");
    print_opt("-d", "--rebinddelete", "Fix Backspace/Delete confusion problem");
    print_opt("-e", "--emptyline", "Keep the line below the title bar empty");
    print_opt("-f <file>", "--rcfile=<file>", "Use only this file for configuring nax");
    print_opt("-g", "--showcursor", "Show cursor in file browser & help text");
    print_opt("-h", "--help", "Show this help text and exit");
    print_opt("-i", "--autoindent", "Automatically indent new lines");
    print_opt("-j", "--jumpyscrolling", "Scroll per half-screen, not per line");
    print_opt("-k", "--cutfromcursor", "Cut from cursor to end of line");
    print_opt("-l", "--linenumbers", "Show line numbers in front of the text");
    print_opt("-m", "--mouse", "Enable the use of the mouse");
    print_opt("-n", "--noread", "Do not read the file (only write it)");
    print_opt("-o <dir>", "--operatingdir=<dir>", "Set operating directory");
    print_opt("-p", "--preserve", "Preserve XON (^Q) and XOFF (^S) keys");
    print_opt("-q", "--indicator", "Show a position+portion indicator");
    print_opt("-r <number>", "--fill=<number>", "Set width for hard-wrap and justify");
    if !ISSET(RESTRICTED) {
        print_opt("-s <program>", "--speller=<program>", "Use this alternative spell checker");
    }
    print_opt("-t", "--saveonexit", "Save changes on exit, don't prompt");
    print_opt("-u", "--unix", "Save a file by default in Unix format");
    print_opt("-v", "--view", "View mode (read-only)");
    print_opt("-w", "--nowrap", "Don't hard-wrap long lines [default]");
    print_opt("-x", "--nohelp", "Don't show the two help lines");
    print_opt("-y", "--afterends", "Make Ctrl+Right stop at word ends");
    print_opt("-z", "--listsyntaxes", "List the names of available syntaxes");
    print_opt("-@", "--colonparsing", "Accept 'filename:linenumber' notation");
    print_opt("-%", "--stateflags", "Show some states on the title bar");
    print_opt("-_", "--minibar", "Show a feedback bar at the bottom");
    print_opt("-0", "--zero", "Hide all bars, use whole terminal");
    print_opt("-1", "--solosidescroll", "Scroll only the current line sideways");
    print_opt("-/", "--modernbindings", "Use better-known key bindings");
}

// ======================== rcfile 辅助（对应 global.c / rcfile.c） ========================

/// 将键名字符串转换为键码（对应 `keycode_from_string`）。
/// 失败返回 -1。
pub fn keycode_from_string(keystring: &str) -> i32 {
    let bytes = keystring.as_bytes();
    if bytes.first() == Some(&b'^') {
        if bytes.len() == 2 {
            let ch = bytes[1];
            if ch == b'/' || ch == b'-' {
                return 31;
            }
            if ch <= b'_' {
                return ch as i32 - 64;
            }
            if ch == b'`' {
                return 0;
            }
            return -1;
        } else if keystring.eq_ignore_ascii_case("^Space") {
            return 0;
        } else {
            return -1;
        }
    } else if bytes.first() == Some(&b'M') {
        if bytes.len() == 3 && bytes[1] == b'-' {
            let ch = bytes[2];
            if (b'A'..=b'Z').contains(&ch) {
                return (ch | 0x20) as i32;
            }
            return ch as i32;
        }
        if keystring.eq_ignore_ascii_case("M-Space") {
            return b' ' as i32;
        } else if keystring.eq_ignore_ascii_case("M-Left") {
            return ALT_LEFT;
        } else if keystring.eq_ignore_ascii_case("M-Right") {
            return ALT_RIGHT;
        } else if keystring.eq_ignore_ascii_case("M-Up") {
            return ALT_UP;
        } else if keystring.eq_ignore_ascii_case("M-Down") {
            return ALT_DOWN;
        } else if keystring.eq_ignore_ascii_case("M-Ins") {
            return ALT_INSERT;
        } else if keystring.eq_ignore_ascii_case("M-Del") {
            return ALT_DELETE;
        } else {
            return -1;
        }
    } else if keystring.len() >= 5
        && keystring[..5].eq_ignore_ascii_case("Sh-M-")
        && keystring.len() == 6
    {
        let ch = keystring.as_bytes()[5];
        if ch.is_ascii_alphabetic() {
            with_global_mut(|g| g.shifted_metas = true);
            return (ch & 0x5F) as i32;
        }
        return -1;
    } else if bytes.first() == Some(&b'F') {
        let n: i32 = keystring[1..].parse().unwrap_or(0);
        if (1..=24).contains(&n) {
            return KEY_F0 + n;
        }
        return -1;
    } else if keystring.eq_ignore_ascii_case("Ins") {
        return KEY_IC;
    } else if keystring.eq_ignore_ascii_case("Del") {
        return KEY_DC;
    } else {
        return -1;
    }
}

/// 16 个菜单名与对应符号（对应 rcfile.c 的 menunames/menusymbols）。
const MENU_NAMES: [&str; 16] = [
    "main", "search", "replace", "replacewith",
    "yesno", "gotoline", "writeout", "insert",
    "execute", "help", "spell", "linter",
    "browser", "whereisfile", "gotodir",
    "all",
];

const MENU_SYMBOLS: [i32; 16] = [
    MMAIN, MWHEREIS, MREPLACE, MREPLACEWITH,
    MYESNO, MGOTOLINE, MWRITEFILE, MINSERTFILE,
    MEXECUTE, MHELP, MSPELL, MLINTER,
    MBROWSER, MWHEREISFILE, MGOTODIR,
    MMOST | MBROWSER | MHELP | MYESNO,
];

/// 返回给定菜单名对应的符号；未知返回 0（对应 `name_to_menu`）。
pub fn name_to_menu(name: &str) -> i32 {
    for (index, entry) in MENU_NAMES.iter().enumerate() {
        if *entry == name {
            return MENU_SYMBOLS[index];
        }
    }
    0
}

/// 返回给定菜单符号对应的名称（对应 `menu_to_name`）。
pub fn menu_to_name(menu: i32) -> &'static str {
    for (index, symbol) in MENU_SYMBOLS.iter().enumerate() {
        if *symbol == menu {
            return MENU_NAMES[index];
        }
    }
    "boooo"
}

/// 解释 rc 文件中的函数字符串，返回 (函数, toggle 值)。
/// 无法识别时返回 None（对应 rcfile.c 的 `strtosc`）。
pub fn strtosc(input: &str) -> Option<(FunctionId, i32)> {
    let s = input;
    let plain = |func: FunctionId| Some((func, 0));
    match s {
        "cancel" => plain(FunctionId::DoCancel),
        "help" => plain(FunctionId::DoHelp),
        "exit" => plain(FunctionId::DoExit),
        "discardbuffer" => plain(FunctionId::DiscardBuffer),
        "writeout" => plain(FunctionId::DoWriteOut),
        "savefile" => plain(FunctionId::DoSaveFile),
        "insert" => plain(FunctionId::DoInsertFile),
        "whereis" => plain(FunctionId::DoSearchForward),
        "wherewas" => plain(FunctionId::DoSearchBackward),
        "findprevious" => plain(FunctionId::DoFindPrevious),
        "findnext" => plain(FunctionId::DoFindNext),
        "replace" => plain(FunctionId::DoReplace),
        "cut" => plain(FunctionId::DoCut),
        "copy" => plain(FunctionId::DoCopy),
        "paste" => plain(FunctionId::DoPaste),
        "execute" => plain(FunctionId::DoExecute),
        "cutrestoffile" => plain(FunctionId::DoCutToEof),
        "zap" => plain(FunctionId::DoZap),
        "mark" => plain(FunctionId::DoMark),
        "tospell" | "speller" => plain(FunctionId::DoSpell),
        "linter" => plain(FunctionId::DoLinter),
        "formatter" => plain(FunctionId::DoFormatter),
        "location" => plain(FunctionId::DoReportLocation),
        "gotoline" => plain(FunctionId::DoGoToLine),
        "justify" => plain(FunctionId::DoJustify),
        "fulljustify" => plain(FunctionId::DoFullJustify),
        "beginpara" => plain(FunctionId::DoParaBegin),
        "endpara" => plain(FunctionId::DoParaEnd),
        "comment" => plain(FunctionId::DoComment),
        "complete" => plain(FunctionId::DoWordCompletion),
        "indent" => plain(FunctionId::DoIndent),
        "unindent" => plain(FunctionId::DoUnindent),
        "chopwordleft" => plain(FunctionId::ChopPrevWord),
        "chopwordright" => plain(FunctionId::ChopNextWord),
        "findbracket" => plain(FunctionId::DoFindBracket),
        "wordcount" => plain(FunctionId::CountWords),
        "recordmacro" => plain(FunctionId::DoRecordMacro),
        "runmacro" => plain(FunctionId::DoRunMacro),
        "anchor" => plain(FunctionId::PutOrLiftAnchor),
        "prevanchor" => plain(FunctionId::ToPrevAnchor),
        "nextanchor" => plain(FunctionId::ToNextAnchor),
        "undo" => plain(FunctionId::DoUndo),
        "redo" => plain(FunctionId::DoRedo),
        "suspend" => plain(FunctionId::DoSuspend),
        "left" | "back" => plain(FunctionId::DoLeft),
        "right" | "forward" => plain(FunctionId::DoRight),
        "up" | "prevline" => plain(FunctionId::DoUp),
        "down" | "nextline" => plain(FunctionId::DoDown),
        "scrollleft" => plain(FunctionId::DoScrollLeft),
        "scrollright" => plain(FunctionId::DoScrollRight),
        "scrollup" => plain(FunctionId::DoScrollUp),
        "scrolldown" => plain(FunctionId::DoScrollDown),
        "prevword" => plain(FunctionId::DoPrevWord),
        "nextword" => plain(FunctionId::DoNextWord),
        "home" => plain(FunctionId::DoHome),
        "end" => plain(FunctionId::DoEnd),
        "prevblock" => plain(FunctionId::DoPrevBlock),
        "nextblock" => plain(FunctionId::DoNextBlock),
        "toprow" => plain(FunctionId::ToTopRow),
        "bottomrow" => plain(FunctionId::ToBottomRow),
        "center" => plain(FunctionId::DoCenter),
        "cycle" => plain(FunctionId::DoCycle),
        "pageup" | "prevpage" => plain(FunctionId::DoPageUp),
        "pagedown" | "nextpage" => plain(FunctionId::DoPageDown),
        "firstline" => plain(FunctionId::DoFirstLine),
        "lastline" => plain(FunctionId::DoLastLine),
        "prevbuf" => plain(FunctionId::DoPrevFile),
        "nextbuf" => plain(FunctionId::DoNextFile),
        "verbatim" => plain(FunctionId::DoVerbatimInput),
        "tab" => plain(FunctionId::DoTab),
        "enter" => plain(FunctionId::DoEnter),
        "delete" => plain(FunctionId::DoDelete),
        "backspace" => plain(FunctionId::DoBackspace),
        "refresh" => plain(FunctionId::DoFullRefresh),
        "casesens" => plain(FunctionId::CaseSensVoid),
        "regexp" => plain(FunctionId::RegexpVoid),
        "backwards" => plain(FunctionId::BackwardsVoid),
        "flipreplace" => plain(FunctionId::FlipReplace),
        "older" => plain(FunctionId::GetOlderItem),
        "newer" => plain(FunctionId::GetNewerItem),
        "dosformat" => plain(FunctionId::DosFormat),
        "append" => plain(FunctionId::AppendIt),
        "prepend" => plain(FunctionId::PrependIt),
        "backup" => plain(FunctionId::BackItUp),
        "flipexecute" => plain(FunctionId::FlipExecute),
        "flippipe" => plain(FunctionId::FlipPipe),
        "flipconvert" => plain(FunctionId::FlipConvert),
        "flipnewbuffer" => plain(FunctionId::FlipNewBuffer),
        "tofiles" | "browser" => plain(FunctionId::ToFiles),
        "gotodir" => plain(FunctionId::GotoDir),
        "firstfile" => plain(FunctionId::ToFirstFile),
        "lastfile" => plain(FunctionId::ToLastFile),
        /* do_toggle 及其 toggle 值（对应 C 版 strtosc 末尾的 do_toggle 分支）。 */
        "nohelp" => Some((FunctionId::DoToggle, NO_HELP as i32)),
        "zero" => Some((FunctionId::DoToggle, ZERO as i32)),
        "constantshow" => Some((FunctionId::DoToggle, CONSTANT_SHOW as i32)),
        "softwrap" => Some((FunctionId::DoToggle, SOFTWRAP as i32)),
        "linenumbers" => Some((FunctionId::DoToggle, LINE_NUMBERS as i32)),
        "whitespacedisplay" => Some((FunctionId::DoToggle, WHITESPACE_DISPLAY as i32)),
        "nosyntax" => Some((FunctionId::DoToggle, NO_SYNTAX as i32)),
        "smarthome" => Some((FunctionId::DoToggle, SMART_HOME as i32)),
        "autoindent" => Some((FunctionId::DoToggle, AUTOINDENT as i32)),
        "cutfromcursor" => Some((FunctionId::DoToggle, CUT_FROM_CURSOR as i32)),
        "breaklonglines" => Some((FunctionId::DoToggle, BREAK_LONG_LINES as i32)),
        "tabstospaces" => Some((FunctionId::DoToggle, TABS_TO_SPACES as i32)),
        "mouse" => Some((FunctionId::DoToggle, USE_MOUSE as i32)),
        _ => None,
    }
}

/// 函数是否在几乎所有菜单中都有（对应 rcfile.c 的 `is_universal`）。
pub fn is_universal(func: FunctionId) -> bool {
    matches!(
        func,
        FunctionId::DoLeft
            | FunctionId::DoRight
            | FunctionId::DoHome
            | FunctionId::DoEnd
            | FunctionId::DoPrevWord
            | FunctionId::DoNextWord
            | FunctionId::DoDelete
            | FunctionId::DoBackspace
            | FunctionId::DoCut
            | FunctionId::DoPaste
            | FunctionId::DoTab
            | FunctionId::DoEnter
            | FunctionId::DoVerbatimInput
    )
}

/// 打印致命错误消息并退出（对应 `die`）。
pub fn die(msg: &str) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1);
}
