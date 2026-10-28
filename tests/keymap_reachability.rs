//! 主菜单缺失绑定的回归测试：绑定表、键码转换与实际分发。
//!
//! 对照 nano 9.2 的 global.c：
//!   M-J → do_full_justify，M-< / M-> → do_scroll_left/right，
//!   M-{ → do_unindent，M-} → do_indent，Sh-Tab → do_unindent，
//!   M-W / M-Q → do_findnext / do_findprevious。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.tabsize = 8;
    });
}

fn shortcut_func(key: i32) -> Option<FunctionId> {
    nanoxide::global::find_shortcut(key, MMAIN).map(|sc| sc.borrow().func)
}

#[test]
fn missing_main_bindings_exist() {
    setup();

    assert_eq!(shortcut_func(0x26A), Some(FunctionId::DoFullJustify), "M-J");
    assert_eq!(shortcut_func(0x23C), Some(FunctionId::DoScrollLeft), "M-<");
    assert_eq!(shortcut_func(0x23E), Some(FunctionId::DoScrollRight), "M->");
    assert_eq!(shortcut_func(0x27B), Some(FunctionId::DoUnindent), "M-{{");
    assert_eq!(shortcut_func(0x27D), Some(FunctionId::DoIndent), "M-}}");
    assert_eq!(
        shortcut_func(SHIFT_TAB),
        Some(FunctionId::DoUnindent),
        "Sh-Tab"
    );
    assert_eq!(shortcut_func(0x277), Some(FunctionId::DoFindNext), "M-W");
    assert_eq!(
        shortcut_func(0x271),
        Some(FunctionId::DoFindPrevious),
        "M-Q"
    );
}

#[test]
fn alt_shift_punctuation_and_backtab_translate() {
    let tc = nanoxide::winio::translate_keycode;

    assert_eq!(
        tc(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT)),
        0x26A,
        "Alt+j 应译为 M-J 键码"
    );
    assert_eq!(
        tc(KeyEvent::new(
            KeyCode::Char('{'),
            KeyModifiers::ALT | KeyModifiers::SHIFT
        )),
        0x27B,
        "Alt+Shift+[（即 M-{{）应可触达"
    );
    assert_eq!(
        tc(KeyEvent::new(
            KeyCode::Char('<'),
            KeyModifiers::ALT | KeyModifiers::SHIFT
        )),
        0x23C,
        "Alt+Shift+,（即 M-<）应可触达"
    );
    assert_eq!(
        tc(KeyEvent::new(
            KeyCode::Char('J'),
            KeyModifiers::ALT | KeyModifiers::SHIFT
        )),
        0x26A,
        "Alt+Shift+J 应按 M-J 处理"
    );
    assert_eq!(
        tc(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        SHIFT_TAB,
        "BackTab 应映射为 C 版的 SHIFT_TAB"
    );
}

/// 验证不是只有绑定表：按键确实会走到函数实现。
#[test]
fn keys_reach_functions_through_dispatch() {
    setup();
    nanoxide::text::inject(b"\tabc", 4);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().current_x = 0;
    });

    /* M-{：反缩进。 */
    assert!(nanoxide::winio::handle_input_key(0x27B));
    assert_eq!(current_line(), "abc", "M-{{ 应调用 do_unindent");

    /* Sh-Tab：同样反缩进。 */
    set_current_line("\tabc");
    assert!(nanoxide::winio::handle_input_key(SHIFT_TAB));
    assert_eq!(current_line(), "abc", "Sh-Tab 应调用 do_unindent");

    /* M-}：缩进。 */
    assert!(nanoxide::winio::handle_input_key(0x27D));
    assert_eq!(current_line(), "\tabc", "M-}} 应调用 do_indent");

    /* M-< / M->：左右滚动（brink 变化即证明确实执行了滚动函数）。 */
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().brink = 8;
    });
    assert!(nanoxide::winio::handle_input_key(0x23C));
    let brink = with_global(|g| g.openfile.as_ref().unwrap().borrow().brink);
    assert_eq!(brink, 0, "M-< 应向左滚动一个 tab 宽");
    assert!(nanoxide::winio::handle_input_key(0x23E));
    let brink = with_global(|g| g.openfile.as_ref().unwrap().borrow().brink);
    assert_eq!(brink, 8, "M-> 应向右滚动一个 tab 宽");
}

fn current_line() -> String {
    with_global(|g| {
        g.openfile
            .as_ref()
            .unwrap()
            .borrow()
            .current
            .as_ref()
            .unwrap()
            .borrow()
            .data
            .clone()
    })
}

fn set_current_line(text: &str) {
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.current.as_ref().unwrap().borrow_mut().data = text.to_string();
        of.current_x = 0;
        of.placewewant = 0;
    });
}

/// F1–F12 的绑定应与 global.c:1574-1595 一致。
#[test]
fn function_keys_follow_nano_92() {
    setup();
    let f = |n: i32| KEY_F0 + n;

    assert_eq!(shortcut_func(f(1)), Some(FunctionId::DoHelp), "F1 = help");
    assert_eq!(shortcut_func(f(2)), Some(FunctionId::DoExit), "F2 = exit");
    assert_eq!(
        shortcut_func(f(3)),
        Some(FunctionId::DoWriteOut),
        "F3 = write out"
    );
    assert_eq!(
        shortcut_func(f(4)),
        Some(FunctionId::DoJustify),
        "F4 = justify（不是搜索）"
    );
    assert_eq!(
        shortcut_func(f(5)),
        Some(FunctionId::DoInsertFile),
        "F5 = read file"
    );
    assert_eq!(
        shortcut_func(f(6)),
        Some(FunctionId::DoSearchForward),
        "F6 = where is"
    );
    assert_eq!(
        shortcut_func(f(7)),
        Some(FunctionId::DoPageUp),
        "F7 = page up"
    );
    assert_eq!(
        shortcut_func(f(8)),
        Some(FunctionId::DoPageDown),
        "F8 = page down"
    );
    assert_eq!(shortcut_func(f(9)), Some(FunctionId::DoCut), "F9 = cut");
    assert_eq!(shortcut_func(f(10)), Some(FunctionId::DoPaste), "F10 = paste");
    assert_eq!(
        shortcut_func(f(11)),
        Some(FunctionId::DoReportLocation),
        "F11 = report location"
    );
    assert_eq!(shortcut_func(f(12)), Some(FunctionId::DoSpell), "F12 = spell");

    /* 帮助查看器里 F1 是“退出”（global.c:1574），而不是再次打开帮助。 */
    let help_f1 = nanoxide::global::find_shortcut(f(1), MHELP)
        .map(|sc| sc.borrow().func);
    assert_eq!(help_f1, Some(FunctionId::DoExit), "帮助页 F1 = 退出");
}

/// F9/F10 不但在表里，还能真正执行剪切与粘贴。
#[test]
fn f9_and_f10_reach_cut_and_paste() {
    setup();
    set_current_line("abc");

    assert!(nanoxide::winio::handle_input_key(KEY_F0 + 9));
    assert_eq!(current_line(), "", "F9 应剪切当前行");

    assert!(nanoxide::winio::handle_input_key(KEY_F0 + 10));
    assert_eq!(current_line(), "abc", "F10 应把刚剪切的文本粘回来");
}

/// 锚点与视口行移动的键位（对应 global.c:1356-1362）。
#[test]
fn anchor_and_viewport_keys_follow_global_c() {
    setup();

    assert_eq!(
        shortcut_func(ALT_INSERT),
        Some(FunctionId::PutOrLiftAnchor),
        "M-Ins 放置/取消锚点"
    );
    assert_eq!(
        shortcut_func(ALT_HOME),
        Some(FunctionId::ToTopRow),
        "M-Home 移到视口顶行"
    );
    assert_eq!(
        shortcut_func(ALT_END),
        Some(FunctionId::ToBottomRow),
        "M-End 移到视口底行"
    );
    assert_eq!(
        shortcut_func(ALT_PAGEUP),
        Some(FunctionId::ToPrevAnchor),
        "M-PgUp 跳到上一个锚点"
    );
    assert_eq!(
        shortcut_func(ALT_PAGEDOWN),
        Some(FunctionId::ToNextAnchor),
        "M-PgDn 跳到下一个锚点"
    );
    assert_eq!(
        shortcut_func(0x222),
        Some(FunctionId::PutOrLiftAnchor),
        "M-\" 同样放置/取消锚点"
    );
    assert_eq!(
        shortcut_func(0x227),
        Some(FunctionId::ToNextAnchor),
        "M-' 跳到下一个锚点"
    );
}

/// M-Ins 真正切换当前行的锚点（而不是旧实现里的“标记”）。
#[test]
fn meta_insert_toggles_anchor() {
    setup();
    set_current_line("abc");

    let anchor_state = || {
        with_global(|g| {
            g.openfile
                .as_ref()
                .unwrap()
                .borrow()
                .current
                .as_ref()
                .unwrap()
                .borrow()
                .has_anchor
        })
    };

    assert!(nanoxide::winio::handle_input_key(ALT_INSERT));
    assert!(anchor_state(), "第一次 M-Ins 放置锚点");

    assert!(nanoxide::winio::handle_input_key(ALT_INSERT));
    assert!(!anchor_state(), "第二次 M-Ins 取消锚点");
}

/// 主菜单控制键与 `global.c:1264-1288` 的 legacy 绑定一致。
#[test]
fn control_keys_follow_legacy_bindings() {
    setup();

    assert_eq!(
        shortcut_func(19),
        Some(FunctionId::DoSaveFile),
        "^S 保存（无 --preserve）"
    );
    assert_eq!(
        shortcut_func(17),
        Some(FunctionId::DoSearchBackward),
        "^Q 反向搜索"
    );
    assert_eq!(
        shortcut_func(23),
        Some(FunctionId::DoSearchForward),
        "^W 正向搜索"
    );
    assert_eq!(shortcut_func(12), Some(FunctionId::DoCenter), "^L 居中");
    assert_eq!(
        shortcut_func(KEY_IC),
        Some(FunctionId::DoInsertFile),
        "Ins 插入文件"
    );
    assert_eq!(shortcut_func(0x272), Some(FunctionId::DoReplace), "M-R 替换");
    assert_eq!(
        shortcut_func(20),
        Some(FunctionId::DoExecute),
        "^T 执行命令"
    );
}

/// `--preserve` 时 ^S / ^Q 不绑定（对应 C 的 `!ISSET(PRESERVE)`），^W 仍可用。
#[test]
fn preserve_disables_save_and_backward_search_keys() {
    nanoxide::global::global_init();
    SET(PRESERVE);
    nanoxide::global::shortcut_init();
    with_global_mut(|g| g.openfile = None);
    nanoxide::files::make_new_buffer();

    let f = |key: i32| nanoxide::global::find_shortcut(key, MMAIN).map(|sc| sc.borrow().func);
    assert_eq!(f(19), None, "PRESERVE 下 ^S 不绑定");
    assert_eq!(f(17), None, "PRESERVE 下 ^Q 不绑定");
    assert_eq!(f(23), Some(FunctionId::DoSearchForward), "^W 不受影响");

    UNSET(PRESERVE);
}

/// `--modernbindings` 的主菜单键位（对应 global.c:1243-1262、1301）。
#[test]
fn modern_bindings_follow_global_c() {
    nanoxide::global::global_init();
    SET(MODERN_BINDINGS);
    nanoxide::global::shortcut_init();
    with_global_mut(|g| g.openfile = None);
    nanoxide::files::make_new_buffer();

    let f = |key: i32| nanoxide::global::find_shortcut(key, MMAIN).map(|sc| sc.borrow().func);

    assert_eq!(f(8), Some(FunctionId::DoHelp), "^H 帮助");
    assert_eq!(f(17), Some(FunctionId::DoExit), "^Q 退出");
    assert_eq!(f(19), Some(FunctionId::DoSaveFile), "^S 保存");
    assert_eq!(f(23), Some(FunctionId::DoWriteOut), "^W 写文件");
    assert_eq!(f(15), Some(FunctionId::DoInsertFile), "^O 读文件");
    assert_eq!(f(4), Some(FunctionId::DoFindPrevious), "^D 上一个匹配");
    assert_eq!(f(7), Some(FunctionId::DoFindNext), "^G 下一个匹配");
    assert_eq!(f(18), Some(FunctionId::DoReplace), "^R 替换");
    assert_eq!(f(20), Some(FunctionId::DoGoToLine), "^T 跳行");
    assert_eq!(f(16), Some(FunctionId::DoReportLocation), "^P 报告位置");
    assert_eq!(f(26), Some(FunctionId::DoUndo), "^Z 撤销");
    assert_eq!(f(25), Some(FunctionId::DoRedo), "^Y 重做");
    assert_eq!(f(1), Some(FunctionId::DoMark), "^A 标记");
    assert_eq!(f(24), Some(FunctionId::DoCut), "^X 剪切");
    assert_eq!(f(3), Some(FunctionId::DoCopy), "^C 复制");
    assert_eq!(f(22), Some(FunctionId::DoPaste), "^V 粘贴");
    assert_eq!(f(5), Some(FunctionId::DoExecute), "^E 执行命令");

    UNSET(MODERN_BINDINGS);
}

/// modern 下 ^X 是剪切（不是退出），且会先于 legacy 硬编码链生效。
#[test]
fn modern_x_cuts_instead_of_exiting() {
    nanoxide::global::global_init();
    SET(MODERN_BINDINGS);
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.openfile = None;
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
    });
    nanoxide::files::make_new_buffer();
    set_current_line("abc");

    assert!(nanoxide::winio::handle_input_key(24));
    assert_eq!(current_line(), "", "modern ^X 应剪切当前行");

    UNSET(MODERN_BINDINGS);
}

/// legacy 下 ^Z 提示挂起方式（对应 nano.c 的 `suggest_ctrlT_ctrlZ`）。
#[test]
fn legacy_ctrl_z_suggests_suspend_keys() {
    setup();
    with_global_mut(|g| g.statusbar_msg.clear());

    assert_eq!(
        shortcut_func(26),
        Some(FunctionId::SuggestSuspend),
        "legacy ^Z 是挂起提示"
    );
    assert!(nanoxide::winio::handle_input_key(26));
    let msg = with_global(|g| g.statusbar_msg.clone());
    assert_eq!(msg, nanoxide::t!("global-suspend_hint"));
}

/// modern 下 ^Z 是撤销，不显示挂起提示。
#[test]
fn modern_ctrl_z_undoes_instead_of_suggesting() {
    nanoxide::global::global_init();
    SET(MODERN_BINDINGS);
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.openfile = None;
        g.currmenu = MMAIN;
    });
    nanoxide::files::make_new_buffer();

    assert_eq!(
        shortcut_func(26),
        Some(FunctionId::DoUndo),
        "modern ^Z 是撤销"
    );

    UNSET(MODERN_BINDINGS);
}

/// 执行菜单（MEXECUTE）的快捷键与 `global.c:1304-1305、1315、1449、1458` 一致。
#[test]
fn execute_menu_keys_follow_global_c() {
    setup();
    let f = |key: i32| {
        nanoxide::global::find_shortcut(key, MEXECUTE).map(|sc| sc.borrow().func)
    };

    assert_eq!(f(19), Some(FunctionId::DoSpell), "^S 拼写检查（无 --preserve）");
    assert_eq!(f(20), Some(FunctionId::DoSpell), "^T 拼写检查");
    assert_eq!(f(15), Some(FunctionId::DoFormatter), "^O 格式化");
    assert_eq!(f(22), Some(FunctionId::DoCutToEof), "^V 剪切到文件末尾");
    assert_eq!(f(10), Some(FunctionId::DoFullJustify), "^J 全文对齐");
    assert_eq!(f(25), Some(FunctionId::DoLinter), "^Y linter");
    assert_eq!(f(26), Some(FunctionId::DoSuspend), "^Z 挂起");
    assert_eq!(f(0x25C), Some(FunctionId::FlipPipe), "M-\\ 管道");
}

/// 搜索/替换菜单的选项键与 `global.c:866-877` 一致。
#[test]
fn search_menu_options_follow_global_c() {
    setup();
    let f = |key: i32, menu: i32| {
        nanoxide::global::find_shortcut(key, menu).map(|sc| sc.borrow().func)
    };

    for menu in [MWHEREIS, MREPLACE] {
        assert_eq!(
            f(0x263, menu),
            Some(FunctionId::DoToggleCaseSensitive),
            "M-C 区分大小写"
        );
        assert_eq!(
            f(0x272, menu),
            Some(FunctionId::DoToggleRegexp),
            "M-R 正则表达式"
        );
        assert_eq!(
            f(0x262, menu),
            Some(FunctionId::DoToggleBackwards),
            "M-B 反向搜索"
        );
    }

    /* whereis 提示里 ^R 是切换到替换（C 的 flip_replace），不是正则。 */
    assert_eq!(
        f(18, MWHEREIS),
        Some(FunctionId::FlipReplace),
        "^R 切换到替换"
    );
}

/// 提示历史翻页与首/末文件键（对应 `global.c:884-895、1529-1533`）。
#[test]
fn prompt_history_and_file_bounds_follow_global_c() {
    setup();
    let f = |key: i32, menu: i32| {
        nanoxide::global::find_shortcut(key, menu).map(|sc| sc.borrow().func)
    };

    for menu in [
        MWHEREIS,
        MREPLACE,
        MREPLACEWITH,
        MWHEREISFILE,
        MFINDINHELP,
        MEXECUTE,
    ] {
        assert_eq!(f(16, menu), Some(FunctionId::GetOlderItem), "^P 上一条历史");
        assert_eq!(f(14, menu), Some(FunctionId::GetNewerItem), "^N 下一条历史");
    }

    assert_eq!(
        f(25, MWHEREISFILE),
        Some(FunctionId::ToFirstFile),
        "查找文件提示 ^Y 首文件"
    );
    assert_eq!(
        f(22, MWHEREISFILE),
        Some(FunctionId::ToLastFile),
        "查找文件提示 ^V 末文件"
    );
    assert_eq!(
        f(0x25C, MBROWSER),
        Some(FunctionId::ToFirstFile),
        "浏览器 M-\\ 首文件"
    );
    assert_eq!(
        f(0x22F, MBROWSER),
        Some(FunctionId::ToLastFile),
        "浏览器 M-/ 末文件"
    );
    assert_eq!(
        f(25, MFINDINHELP),
        Some(FunctionId::DoFirstLine),
        "查找帮助 ^Y 首行"
    );
}

/// 浏览器菜单与 insert/write 提示的键位（对应 `global.c:1537-1556`）。
#[test]
fn browser_and_prompt_bindings_follow_global_c() {
    setup();
    let f = |key: i32, menu: i32| {
        nanoxide::global::find_shortcut(key, menu).map(|sc| sc.borrow().func)
    };

    assert_eq!(f(20, MBROWSER), Some(FunctionId::DoExit), "^T 退出浏览器");
    assert_eq!(f(KEY_HOME, MBROWSER), Some(FunctionId::ToFirstFile), "Home 首文件");
    assert_eq!(f(KEY_END, MBROWSER), Some(FunctionId::ToLastFile), "End 末文件");
    assert_eq!(
        f(CONTROL_HOME, MBROWSER),
        Some(FunctionId::ToFirstFile),
        "^Home 首文件"
    );
    assert_eq!(
        f(CONTROL_END, MBROWSER),
        Some(FunctionId::ToLastFile),
        "^End 末文件"
    );
    assert_eq!(f(31, MBROWSER), Some(FunctionId::DoGotoDir), "^/ 切换目录");
    assert_eq!(f(0x267, MBROWSER), Some(FunctionId::DoGotoDir), "M-G 切换目录");
    assert_eq!(f(0x20, MBROWSER), Some(FunctionId::DoPageDown), "Space 下一页");
    assert_eq!(
        f(KEY_BACKSPACE, MBROWSER),
        Some(FunctionId::DoPageUp),
        "Bsp 上一页"
    );

    /* ^T 在写文件与插入文件提示里都是浏览（C 的 to_files）。 */
    assert_eq!(f(20, MWRITEFILE), Some(FunctionId::ToFiles), "写文件 ^T");
    assert_eq!(f(20, MINSERTFILE), Some(FunctionId::ToFiles), "插入文件 ^T");
}
