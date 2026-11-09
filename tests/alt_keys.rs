//! Alt 组合键（软件内写作 `M-`）在 Linux 与 Windows 上都应生效。
//!
//! 对应 nano 的 `winio.c::parse_kbinput`（把 ESC 前缀翻译成 meta_key +
//! 裸键码）与 `global.c::keycode_from_string` / `add_to_sclist`（把键名
//! 翻译成同一个键码）。原版靠「同一个函数产出两侧」保证一致；本项目
//! 输入层由 crossterm 事件替代，一旦绑定表用 C 的 `字符 | 0x20` 老约定、
//! 而输入层用 `0x200 + 字符`，两侧永不相交，表现就是"按 Alt 没有任何
//! 反应"。本测试用真实按下所产生的键码去查绑定，是端到端的"可触达"证明。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
        g.we_are_running = true;
    });
}

fn with_mods(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)
}

/// 当前行的内容（避免在闭包里对同一个 RefCell 反复借用）。
fn current_data() -> String {
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let cur = of.current.as_ref().unwrap().borrow();
        cur.data.clone()
    })
}

/// 按下 Alt+c 后主菜单应命中的功能。
fn main_menu_func(c: char) -> Option<FunctionId> {
    let keycode = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char(c),
        KeyModifiers::ALT,
    ));
    nanoxide::global::find_shortcut(keycode, MMAIN).map(|s| s.borrow().func)
}

/// Alt + 功能键在主菜单命中的功能。
fn main_menu_func_key(code: KeyCode) -> Option<FunctionId> {
    let keycode = nanoxide::winio::translate_keycode(with_mods(code, KeyModifiers::ALT));
    nanoxide::global::find_shortcut(keycode, MMAIN).map(|s| s.borrow().func)
}

/// 字母类 Alt 组合：按 C 的内置表逐个验证（此前全部无法触达）。
#[test]
fn alt_letter_bindings_are_reachable() {
    setup();
    let expected: [(char, FunctionId); 22] = [
        ('a', FunctionId::DoMark),
        ('b', FunctionId::DoFindPrevious),
        ('c', FunctionId::DoToggle),
        ('d', FunctionId::CountWords),
        ('e', FunctionId::DoRedo),
        ('f', FunctionId::DoFindNext),
        ('g', FunctionId::DoGoToLine),
        ('h', FunctionId::DoToggle),
        ('i', FunctionId::DoToggle),
        ('j', FunctionId::DoFullJustify),
        ('k', FunctionId::DoToggle),
        ('l', FunctionId::DoToggle),
        ('m', FunctionId::DoToggle),
        ('n', FunctionId::DoToggle),
        ('o', FunctionId::DoToggle),
        ('p', FunctionId::DoToggle),
        ('q', FunctionId::DoFindPrevious),
        ('r', FunctionId::DoReplace),
        ('s', FunctionId::DoToggle),
        ('t', FunctionId::DoCutToEof),
        ('u', FunctionId::DoUndo),
        ('w', FunctionId::DoFindNext),
        // y/z/x 与下列断言分开写，保持每项都显式。
    ];
    // 上面的数组长度按实际条目给，避免 'y'/'z'/'x' 漏检。
    let extra = [
        ('y', FunctionId::DoToggle),
        ('z', FunctionId::DoToggle),
        ('x', FunctionId::DoToggle),
    ];
    for (c, func) in expected.into_iter().chain(extra) {
        assert_eq!(
            main_menu_func(c),
            Some(func),
            "Alt+{c} 应命中 {func:?}"
        );
    }
}

/// 大写与 Shift 组合折叠为同一个 Alt 组合（对应 C 的 `|= 0x20`）。
#[test]
fn alt_is_case_insensitive() {
    setup();
    for c in ['a', 'd', 'u', 'r', 'x'] {
        let lower = main_menu_func(c);
        let upper = main_menu_func(c.to_ascii_uppercase());
        assert_eq!(lower, upper, "Alt+{c} 与 Alt+{} 应是同一组合", c.to_ascii_uppercase());
        assert!(lower.is_some(), "Alt+{c} 应有绑定");
    }

    let shift_alt = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('A'),
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    let plain = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('a'),
        KeyModifiers::ALT,
    ));
    assert_eq!(shift_alt, plain, "Shift+Alt+字母应与 Alt+小写字母同码（C 的 meta_key 分支）");
}

/// 符号类 Alt 组合（含需要 shift 才能打出的字符）。
#[test]
fn alt_symbol_bindings_are_reachable() {
    setup();
    let expected: [(char, FunctionId); 16] = [
        (']', FunctionId::DoFindBracket),
        ('{', FunctionId::DoUnindent),
        ('}', FunctionId::DoIndent),
        ('-', FunctionId::DoScrollUp),
        ('+', FunctionId::DoScrollDown),
        ('=', FunctionId::DoScrollDown),
        ('<', FunctionId::DoScrollLeft),
        ('>', FunctionId::DoScrollRight),
        (',', FunctionId::DoPrevFile),
        ('.', FunctionId::DoNextFile),
        ('"', FunctionId::PutOrLiftAnchor),
        ('\'', FunctionId::ToNextAnchor),
        (':', FunctionId::DoRecordMacro),
        (';', FunctionId::DoRunMacro),
        ('%', FunctionId::DoCycle),
        ('6', FunctionId::DoCopy),
    ];
    for (c, func) in expected {
        assert_eq!(main_menu_func(c), Some(func), "Alt+{c} 应命中 {func:?}");
    }

    /* C 里数字键的段落/块跳转（global.c:1415-1424）。 */
    for (c, func) in [
        ('9', FunctionId::DoParaBegin),
        ('0', FunctionId::DoParaEnd),
        ('7', FunctionId::DoPrevBlock),
        ('8', FunctionId::DoNextBlock),
        ('3', FunctionId::DoComment),
    ] {
        assert_eq!(main_menu_func(c), Some(func), "Alt+{c} 应命中 {func:?}");
    }
}

/// Alt+Space 与 Alt+Backspace 这类具名组合。
#[test]
fn alt_named_combos() {
    setup();
    /* M-Space → to_prev_word（global.c:1398）。 */
    assert_eq!(
        main_menu_func(' '),
        Some(FunctionId::DoPrevWord),
        "Alt+Space 应命中 DoPrevWord"
    );
    /* M-Bsp 在 C 里键码就是 CONTROL_SHIFT_DELETE（global.c:1352），
     * 绑定到 chop_previous_word；输入层也必须产出同一键码。 */
    let bsp = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Backspace,
        KeyModifiers::ALT,
    ));
    assert_eq!(bsp, CONTROL_SHIFT_DELETE, "Alt+Backspace 应产出 CONTROL_SHIFT_DELETE");
    assert_eq!(
        nanoxide::global::find_shortcut(bsp, MMAIN).map(|s| s.borrow().func),
        Some(FunctionId::ChopPrevWord),
        "CONTROL_SHIFT_DELETE 应绑到 chop_previous_word"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::Delete),
        Some(FunctionId::DoCut),
        "Alt+Delete 应命中 DoCut"
    );
}

/// Alt + 方向键/Home/End/PageUp/PageDown：键码约定与绑定都要接上。
#[test]
fn alt_function_key_bindings() {
    setup();
    /* 本项目沿用 C 的 ALT_* 专用键码表达 Alt+方向键/Home/End（0x42x），
     * 而不是 0x200 + 字符；C 的 ALT_UP/ALT_DOWN/ALT_PAGEUP/ALT_PAGEDOWN/
     * ALT_INSERT 在 global.c 中确有绑定。 */
    /* Alt+Left/Right：C 在 MMAIN 绑到 switch_to_prev/next_buffer
     * （global.c:1389-1390）；本项目改绑到上一词/下一词，属既有偏离，
     * 已登记在 DIFFERENCES.md（本次只验证 Alt 能被识别，不动这个选择）。 */
    assert_eq!(
        main_menu_func_key(KeyCode::Left),
        Some(FunctionId::DoPrevWord),
        "Alt+Left 应命中上一词"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::Right),
        Some(FunctionId::DoNextWord),
        "Alt+Right 应命中下一词"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::Home),
        Some(FunctionId::ToTopRow),
        "Alt+Home 应到顶行"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::End),
        Some(FunctionId::ToBottomRow),
        "Alt+End 应到底行"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::PageUp),
        Some(FunctionId::ToPrevAnchor),
        "Alt+PageUp 应到上一个锚点"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::PageDown),
        Some(FunctionId::ToNextAnchor),
        "Alt+PageDown 应到下一个锚点"
    );
    assert_eq!(
        main_menu_func_key(KeyCode::Insert),
        Some(FunctionId::PutOrLiftAnchor),
        "Alt+Insert 应设置/取消锚点"
    );
}

/// 输入层与 rc 的 `bind M-x` 必须使用同一约定，否则用户 rc 里的 Alt
/// 绑定永远匹配不到真实按键。
#[test]
fn rc_bind_keycode_matches_runtime() {
    setup();
    for c in ['a', 'd', 'u', 't', 'x', 'z', '6', '[', '-', '\\', ' ', '#', '$', '^', '9'] {
        let name = format!("M-{c}");
        let parsed = nanoxide::global::keycode_from_string(&name);
        let runtime = nanoxide::winio::translate_keycode(with_mods(
            KeyCode::Char(c),
            KeyModifiers::ALT,
        ));
        assert!(parsed >= 0, "{name} 应能解析");
        assert_eq!(
            parsed, runtime,
            "{name}：rc 解析 0x{parsed:03X} 与运行时 0x{runtime:03X} 不一致"
        );
    }

    /* 具名形式。 */
    for (name, code) in [
        ("M-Space", ' '),
        ("M-space", ' '),
        ("M-6", '6'),
        ("M-^", '^'),
        ("M-$", '$'),
    ] {
        let parsed = nanoxide::global::keycode_from_string(name);
        let runtime = nanoxide::winio::translate_keycode(with_mods(
            KeyCode::Char(code),
            KeyModifiers::ALT,
        ));
        assert_eq!(parsed, runtime, "{name} 的解析值应与运行时一致");
    }

    /* 具名功能键形式沿用 C 的 ALT_* 常量。 */
    assert_eq!(nanoxide::global::keycode_from_string("M-Left"), ALT_LEFT);
    assert_eq!(nanoxide::global::keycode_from_string("M-Right"), ALT_RIGHT);
    assert_eq!(nanoxide::global::keycode_from_string("M-Up"), ALT_UP);
    assert_eq!(nanoxide::global::keycode_from_string("M-Down"), ALT_DOWN);
    assert_eq!(nanoxide::global::keycode_from_string("M-Ins"), ALT_INSERT);
    assert_eq!(nanoxide::global::keycode_from_string("M-Del"), ALT_DELETE);
    assert_eq!(
        nanoxide::global::keycode_from_string("M-Left"),
        nanoxide::winio::translate_keycode(with_mods(KeyCode::Left, KeyModifiers::ALT)),
        "M-Left 的解析值应与 Alt+Left 的实际产出一致"
    );
}

/// 绑过 `Sh-M-<字母>` 后，Alt+Shift+字母 与 Alt+字母 不再同码
/// （对应 C 的 `if (... && !shifted_metas) keycode |= 0x20`）。
#[test]
fn shifted_metas_separates_the_two_combos() {
    setup();
    let plain = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('D'),
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    /* 未绑定 Sh-M- 时折叠为小写，与 Alt+d 同码。 */
    assert_eq!(
        plain,
        nanoxide::global::alt_keycode_for('d'),
        "未启用 Sh-M 时 Alt+Shift+D 应等于 Alt+d"
    );

    /* 解析一次 Sh-M-D 会置位 shifted_metas（对应 C 的 keycode_from_string）。 */
    let sh = nanoxide::global::keycode_from_string("Sh-M-D");
    assert_eq!(sh, nanoxide::global::alt_keycode_raw('D'), "Sh-M-D 应保持大写");
    assert!(with_global(|g| g.shifted_metas), "解析 Sh-M- 应置位 shifted_metas");

    let after = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('D'),
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    assert_eq!(after, sh, "启用后 Alt+Shift+D 应产出 Sh-M-D 的键码");
    assert_ne!(after, plain, "启用后两个组合必须区分开");
}

/// 多字节的 M-名按 C 属于无效键名（rc 报 "invalid key name"），
/// 不能悄悄生成一条匹配不到的死绑定。
#[test]
fn multibyte_m_name_is_invalid() {
    setup();
    assert_eq!(nanoxide::global::keycode_from_string("M-é"), -1, "M-é 应为无效键名");
    assert_eq!(nanoxide::global::keycode_from_string("M-ab"), -1, "M-ab 应为无效键名");
    assert_eq!(nanoxide::global::keycode_from_string("M-"), -1, "M- 应为无效键名");
}

/// 内置绑定表里不得再有键码为 0 的条目：C 的 `add_to_sclist` 会在 keycode
/// 为 0 时用键名补齐，若漏掉该回退，这些条目就永远不会被匹配。
/// 也不得有 -1（键名解析失败），那同样是不可触达的死条目。
#[test]
fn no_binding_has_zero_keycode() {
    setup();
    let zeros: Vec<String> = nanoxide::global::iter_shortcuts()
        .iter()
        .filter(|s| s.borrow().keycode == 0)
        .map(|s| s.borrow().keystr.clone())
        .collect();
    assert!(zeros.is_empty(), "以下绑定的键码仍为 0（不可触达）：{zeros:?}");

    let bad: Vec<String> = nanoxide::global::iter_shortcuts()
        .iter()
        .filter(|s| s.borrow().keycode < 0)
        .map(|s| format!("{}（{}）", s.borrow().keystr, s.borrow().keycode))
        .collect();
    assert!(bad.is_empty(), "以下绑定的键码为负（键名解析失败）：{bad:?}");
}

/// Alt 标志不得残留在后续按键上：C 的 `parse_kbinput` 每次解析都先清
/// `meta_key`，否则一次 Alt 会让之后所有普通键都被判成 Alt 组合。
#[test]
fn meta_flag_is_reset_between_keystrokes() {
    setup();
    nanoxide::winio::translate_keycode(with_mods(KeyCode::Char('a'), KeyModifiers::ALT));
    assert!(with_global(|g| g.meta_key), "Alt 组合应置位 meta_key");

    nanoxide::winio::translate_keycode(with_mods(KeyCode::Char('b'), KeyModifiers::NONE));
    assert!(!with_global(|g| g.meta_key), "后续普通按键应清掉 meta_key");
}

/// 未绑定的 Alt 组合应给出提示而不是静默吞掉（对应 C 的 `unbound_key`：
/// `Unbound key: M-X`）。这是用户判断"Alt 有没有被识别"最直接的依据。
#[test]
fn unbound_alt_combo_is_reported() {
    setup();
    /* '[' 与 '&' 在 C 的主菜单里没有 Alt 绑定。 */
    let code = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('&'),
        KeyModifiers::ALT,
    ));
    assert!(
        nanoxide::global::find_shortcut(code, MMAIN).is_none(),
        "Alt+& 在本项目里应确实未绑定"
    );
    let handled = nanoxide::winio::handle_input_key(code);
    assert!(handled, "未绑定的 Alt 组合应被消费并给出反馈");
    let msg = with_global(|g| g.statusbar_msg.clone());
    assert!(
        msg.contains("M-&"),
        "提示应回显组合名 M-&，实际为：{msg}"
    );
    with_global_mut(|g| g.statusbar_msg.clear());
}

/// 未绑定的普通 Alt 字母（C 里也没绑定）同样给出 `Unbound key: M-X`。
#[test]
fn unbound_alt_letter_message() {
    setup();
    /* C 的主菜单没有 Alt+[ 绑定。 */
    let code = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('['),
        KeyModifiers::ALT,
    ));
    assert!(nanoxide::global::find_shortcut(code, MMAIN).is_none());
    nanoxide::global::unbound_key(code);
    let msg = with_global(|g| g.statusbar_msg.clone());
    assert!(msg.contains("M-["), "应显示 M-[ 的未绑定提示，实际：{msg}");
    with_global_mut(|g| g.statusbar_msg.clear());
}

/// 真实的 Alt 键不应被当普通字符插入缓冲区（此前 Alt 组合被吞掉但
/// 非 ASCII 码点判断也一并松动，这里守住插入路径）。
#[test]
fn alt_combo_does_not_insert_text() {
    setup();
    let before = current_data();
    let code = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('y'),
        KeyModifiers::ALT,
    ));
    nanoxide::winio::handle_input_key(code);
    let after = current_data();
    assert_eq!(before, after, "未绑定的 Alt 组合不应插入任何字符");
}

/// Alt+字母在提示菜单（MMOST 类）里也要能触达：C 把 M-V / M-Space / M-6
/// 等绑到 MMOST（覆盖各提示菜单）。
#[test]
fn alt_works_in_prompt_menus() {
    setup();
    for menu in [MWHEREIS, MGOTOLINE, MWRITEFILE, MINSERTFILE] {
        with_global_mut(|g| g.currmenu = menu);
        let code = nanoxide::winio::translate_keycode(with_mods(
            KeyCode::Char('v'),
            KeyModifiers::ALT,
        ));
        assert_eq!(
            nanoxide::global::find_shortcut(code, menu).map(|s| s.borrow().func),
            Some(FunctionId::DoVerbatimInput),
            "菜单 {menu} 里 Alt+v 应命中逐字输入"
        );
    }
    with_global_mut(|g| g.currmenu = MMAIN);
}

/// 搜索菜单里的 Alt 选项键（C 的 M-C / M-R / M-B 在 MWHEREIS）。
#[test]
fn alt_search_option_keys() {
    setup();
    with_global_mut(|g| g.currmenu = MWHEREIS);
    let f = |c: char| {
        let code = nanoxide::winio::translate_keycode(with_mods(
            KeyCode::Char(c),
            KeyModifiers::ALT,
        ));
        nanoxide::global::find_shortcut(code, MWHEREIS).map(|s| s.borrow().func)
    };
    assert_eq!(f('c'), Some(FunctionId::DoToggleCaseSensitive), "M-C");
    assert_eq!(f('r'), Some(FunctionId::DoToggleRegexp), "M-R");
    assert_eq!(f('b'), Some(FunctionId::DoToggleBackwards), "M-B");
    with_global_mut(|g| g.currmenu = MMAIN);
}

/// 端到端：Alt+a 真的设置标记、Alt+u 真的撤销（验证分发而不只是查表）。
#[test]
fn alt_combos_actually_execute() {
    setup();
    nanoxide::text::inject(b"hello", 5);

    let mark = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('a'),
        KeyModifiers::ALT,
    ));
    assert!(nanoxide::winio::handle_input_key(mark), "Alt+a 应被处理");
    assert!(
        with_global(|g| g.openfile.as_ref().unwrap().borrow().mark.is_some()),
        "Alt+a 应设置标记"
    );

    /* Alt+u 撤销：回到空缓冲区。 */
    let undo = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('u'),
        KeyModifiers::ALT,
    ));
    assert!(nanoxide::winio::handle_input_key(undo), "Alt+u 应被处理");
    let data = current_data();
    assert!(data.is_empty(), "Alt+u 应撤销刚才的输入，实际内容：{data:?}");
}

/// Alt+字符里非 ASCII 的（如 Alt+é）按 C 属于 FOREIGN_SEQUENCE，
/// 不得污染码点、也不得插入文本。
#[test]
fn alt_with_non_ascii_is_not_injected() {
    setup();
    let before = current_data();
    let code = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('é'),
        KeyModifiers::ALT,
    ));
    assert_eq!(code, nanoxide::definitions::FOREIGN_SEQUENCE, "Alt+非 ASCII 应作为杂散序列");
    nanoxide::winio::handle_input_key(code);
    let after = current_data();
    assert_eq!(before, after, "Alt+é 不应插入任何字符");
}

/// 普通 Alt+字母不影响 --modernbindings 的表查询路径。
#[test]
fn alt_still_works_with_modernbindings() {
    setup();
    SET(MODERN_BINDINGS);
    nanoxide::global::shortcut_init();
    with_global_mut(|g| g.currmenu = MMAIN);
    let code = nanoxide::winio::translate_keycode(with_mods(
        KeyCode::Char('a'),
        KeyModifiers::ALT,
    ));
    assert_eq!(
        nanoxide::global::find_shortcut(code, MMAIN).map(|s| s.borrow().func),
        Some(FunctionId::DoMark),
        "modern 下 M-A 仍是标记"
    );
    UNSET(MODERN_BINDINGS);
}
