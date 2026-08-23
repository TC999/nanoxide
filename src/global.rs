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
pub fn shortcut_init() {
    // 主菜单快捷键（与 C 版 global.c 对应）
    add_to_sclist((MMOST | MBROWSER) & !MFINDINHELP, r"^G", 7, FunctionId::DoHelp, 0);
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
    add_to_sclist(MMAIN, "M-]", 0x25D, FunctionId::DoFindBracket, 0);
    add_to_sclist(MMAIN, "^]", 29, FunctionId::DoWordCompletion, 0);
    // 多缓冲区切换（对应 C 版 rcfile 中常见的 prevfile/nextfile 绑定）
    add_to_sclist(MMAIN, "M-,", 0x22C, FunctionId::DoPrevFile, 0);
    add_to_sclist(MMAIN, "M-.", 0x22E, FunctionId::DoNextFile, 0);
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
    add_to_funcs(FunctionId::DoScrollLeft, MMAIN, "Scroll Left".to_string(), "scrollleft_gist", false);
    add_to_funcs(FunctionId::DoScrollRight, MMAIN, "Scroll Right".to_string(), "scrollright_gist", false);
    add_to_funcs(FunctionId::DoPrevWord, MMAIN, crate::t!("key-prev_word"), "prevword_gist", false);
    add_to_funcs(FunctionId::DoNextWord, MMAIN, crate::t!("key-next_word"), "nextword_gist", false);
    add_to_funcs(FunctionId::ChopPrevWord, MMAIN, "Chop Previous Word".to_string(), "chopwordleft_gist", false);
    add_to_funcs(FunctionId::ChopNextWord, MMAIN, "Chop Next Word".to_string(), "chopwordright_gist", false);
    add_to_funcs(FunctionId::DoFullJustify, MMAIN, crate::t!("key-full_justify"), "fulljustify_gist", false);
    add_to_funcs(FunctionId::CountWords, MMAIN, "Word Count".to_string(), "wordcount_gist", false);
    add_to_funcs(FunctionId::DoVerbatimInput, MMAIN, crate::t!("key-verbatim"), "verbatim_gist", false);
    add_to_funcs(FunctionId::DoRecordMacro, MMAIN, crate::t!("key-record_macro"), "recordmacro_gist", false);
    add_to_funcs(FunctionId::DoRunMacro, MMAIN, crate::t!("key-run_macro"), "runmacro_gist", false);
    add_to_funcs(FunctionId::DoZap, MMAIN, crate::t!("key-zap"), "zap_gist", false);
    add_to_funcs(FunctionId::PutOrLiftAnchor, MMAIN, crate::t!("key-anchor"), "anchor_gist", false);
    add_to_funcs(FunctionId::ToPrevAnchor, MMAIN, "Previous Anchor".to_string(), "prevanchor_gist", false);
    add_to_funcs(FunctionId::ToNextAnchor, MMAIN, "Next Anchor".to_string(), "nextanchor_gist", false);
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
    add_to_funcs(FunctionId::CaseSensVoid, MWHEREIS | MREPLACE, "Case Sens".to_string(), "casesens_gist", false);
    add_to_funcs(FunctionId::RegexpVoid, MWHEREIS | MREPLACE, "Regexp".to_string(), "regexp_gist", false);
    add_to_funcs(FunctionId::BackwardsVoid, MWHEREIS | MREPLACE, "Backwards".to_string(), "backwards_gist", false);
    add_to_funcs(FunctionId::FlipReplace, MWHEREIS | MREPLACE, "Flip Replace".to_string(), "flipreplace_gist", false);
    add_to_funcs(FunctionId::GetOlderItem, MWHEREIS | MREPLACE | MREPLACEWITH | MWHEREISFILE | MEXECUTE, "Older".to_string(), "older_gist", false);
    add_to_funcs(FunctionId::GetNewerItem, MWHEREIS | MREPLACE | MREPLACEWITH | MWHEREISFILE | MEXECUTE, "Newer".to_string(), "newer_gist", false);
    add_to_funcs(FunctionId::DosFormat, MWRITEFILE, "DOS Format".to_string(), "dosformat_gist", false);
    add_to_funcs(FunctionId::BackItUp, MWRITEFILE, "Backup".to_string(), "backup_gist", false);
    add_to_funcs(FunctionId::AppendIt, MWRITEFILE, "Append".to_string(), "append_gist", false);
    add_to_funcs(FunctionId::PrependIt, MWRITEFILE, "Prepend".to_string(), "prepend_gist", false);
    add_to_funcs(FunctionId::FlipConvert, MINSERTFILE, "Flip Convert".to_string(), "flipconvert_gist", false);
    add_to_funcs(FunctionId::FlipExecute, MINSERTFILE, "Flip Execute".to_string(), "flipexecute_gist", false);
    add_to_funcs(FunctionId::FlipNewBuffer, MINSERTFILE | MEXECUTE, "Flip New Buffer".to_string(), "flipnewbuffer_gist", false);
    add_to_funcs(FunctionId::FlipPipe, MEXECUTE, "Flip Pipe".to_string(), "flippipe_gist", false);
    add_to_funcs(FunctionId::ToFiles, MBROWSER, crate::t!("key-to_files"), "tofiles_gist", false);
    add_to_funcs(FunctionId::GotoDir, MBROWSER, crate::t!("key-goto_dir"), "gotodir_gist", false);
    add_to_funcs(FunctionId::ToFirstFile, MBROWSER, crate::t!("key-firstfile"), "firstfile_gist", false);
    add_to_funcs(FunctionId::ToLastFile, MBROWSER, crate::t!("key-lastfile"), "lastfile_gist", false);
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
    add_to_sclist(mmi, "M-D", ALT_DELETE, FunctionId::DoCut, 0);
    add_to_sclist(mmi, "M-V", 0x256, FunctionId::DoVerbatimInput, 0);
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
                    if let Some(v) = args.get(i) {
                        with_global_mut(|g| {
                            g.quotestr = Some(v.clone());
                            g.cmdline_quotestr = Some(v.clone());
                        });
                    }
                }
                "-r" | "--fill" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(f) = args[i].parse::<isize>() {
                            with_global_mut(|g| {
                                g.fill = f;
                                g.cmdline_fill = Some(f);
                            });
                        }
                    }
                }
                "-T" | "--tabsize" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(s) = args[i].parse::<usize>() {
                            with_global_mut(|g| {
                                g.tabsize = s;
                                g.cmdline_tabsize = Some(s);
                            });
                            set_tabsize_independent(s);
                        }
                    }
                }
                "-R" | "--restricted" => SET(RESTRICTED),
                "-o" | "--operatingdir" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        with_global_mut(|g| {
                            g.operating_dir = Some(v.clone());
                            g.cmdline_operating_dir = Some(v.clone());
                        });
                    }
                }
                "-f" | "--rcfile" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        with_global_mut(|g| g.custom_nanorc = Some(v.clone()));
                    }
                }
                "-K" | "--rebinddelete" => SET(REBIND_DELETE),
                "-s" | "--speller" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        with_global_mut(|g| {
                            g.speller = Some(v.clone());
                            g.cmdline_speller = Some(v.clone());
                        });
                    }
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
                    if let Some(v) = args.get(i) {
                        if let Ok(n) = v.parse::<usize>() {
                            with_global_mut(|g| {
                                g.stripe_column = n;
                                g.cmdline_stripe_column = Some(n);
                            });
                        }
                    }
                }
                "-t" | "--saveonexit" => SET(SAVE_ON_EXIT),
                "-0" | "--zero" => SET(ZERO),
                "-M" | "--modernbindings" => SET(MODERN_BINDINGS),
                "-H" | "--historylog" => SET(HISTORYLOG),
                "-B" | "--backup" => SET(MAKE_BACKUP),
                "-C" | "--backupdir" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        with_global_mut(|g| {
                            g.backup_dir = Some(v.clone());
                            g.cmdline_backup_dir = Some(v.clone());
                        });
                    }
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

/// 解析命令行中的文件参数，返回 (文件名, 行号, 列号) 列表。
/// 支持 `+LINE[,COLUMN]` 定位参数（作用于其后的文件）与多个文件名
/// （对应 C 版 main() 的多文件处理）。
pub fn parse_file_args(args: &[String]) -> Vec<(String, isize, isize)> {
    use crate::winio;
    let mut result = Vec::new();
    let mut i = 1;
    let mut givenline: isize = 0;
    let mut givencol: isize = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(rest) = arg.strip_prefix('+') {
            /* +LINE[,COLUMN]：作用于其后的文件。 */
            if rest.is_empty() {
                givenline = -1;
            } else {
                let mut line = givenline;
                let mut col = givencol;
                if crate::utils::parse_line_column(rest, &mut line, &mut col) {
                    givenline = line;
                    givencol = col;
                } else {
                    winio::statusline(
                        MessageType::Alert,
                        &crate::t!("search-invalid_line_or_column"),
                    );
                }
            }
        } else if !arg.starts_with('-') {
            /* 文件名（"-" 表示 stdin，Rust 版简化为跳过）。 */
            if arg != "-" {
                result.push((arg.clone(), givenline, givencol));
            }
            givenline = 0;
            givencol = 0;
        }
        i += 1;
    }
    result
}
/// 打印版本信息。
fn print_version() {
    println!("nanoxide version {}", VERSION);
    println!("(Rust translation of GNU nano)");
    println!("Compiled options: --enable-utf8");
}

/// 打印使用说明。
fn print_usage() {
    println!("Usage: nax [OPTIONS] [FILE]");
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
