//! Shift+方向键的软标记选择（对应 nano.c 的 `process_a_keystroke`
//! 与 winio.c 的 `parse_kbinput` / `shift_held` 处理）。
//!
//! 原版把 Shift+方向键解析成普通 KEY_* 并置 `shift_held`，主循环在
//! 执行移动前建立软标记（`softmark`），松开 Shift 后再移动则丢弃标记。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
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

fn translate(code: KeyCode, modifiers: KeyModifiers) -> i32 {
    nanoxide::winio::translate_keycode(KeyEvent::new(code, modifiers))
}

fn current_x() -> usize {
    with_global(|g| g.openfile.as_ref().unwrap().borrow().current_x)
}

fn mark_state() -> (bool, bool) {
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.mark.is_some(), of.softmark)
    })
}

fn line_count() -> usize {
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let mut count = 0;
        let mut line = of.filetop.clone();
        while let Some(l) = line {
            count += 1;
            line = l.borrow().next.clone();
        }
        count
    })
}

/// Shift+方向/Home/End/PgUp/PgDn 都解析成普通 KEY_* 并置 `shift_held`，
/// 不再产生 SHIFT_* 码（此前 Up/Down 会被当成普通字符插入）。
#[test]
fn shift_movement_keys_return_plain_codes() {
    setup();
    let cases: [(KeyCode, i32); 8] = [
        (KeyCode::Left, KEY_LEFT),
        (KeyCode::Right, KEY_RIGHT),
        (KeyCode::Up, KEY_UP),
        (KeyCode::Down, KEY_DOWN),
        (KeyCode::Home, KEY_HOME),
        (KeyCode::End, KEY_END),
        (KeyCode::PageUp, KEY_PPAGE),
        (KeyCode::PageDown, KEY_NPAGE),
    ];

    for (code, expected) in cases {
        with_global_mut(|g| g.shift_held = false);
        assert_eq!(
            translate(code, KeyModifiers::SHIFT),
            expected,
            "{code:?} + Shift 应返回普通键码"
        );
        assert!(
            with_global(|g| g.shift_held),
            "{code:?} + Shift 应置位 shift_held"
        );
    }
}

/// Ctrl+Shift+方向回到 CONTROL_* 并保持 shift_held；
/// Shift+Alt+方向按 C 版映射为 Home/End/PgUp/PgDn。
#[test]
fn modified_shift_arrows_follow_c_mapping() {
    setup();
    with_global_mut(|g| g.shift_held = false);
    assert_eq!(
        translate(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        CONTROL_RIGHT
    );
    assert!(with_global(|g| g.shift_held));

    with_global_mut(|g| g.shift_held = false);
    assert_eq!(
        translate(KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT),
        KEY_PPAGE
    );
    assert!(with_global(|g| g.shift_held));

    with_global_mut(|g| g.shift_held = false);
    assert_eq!(
        translate(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT),
        KEY_HOME
    );
    assert!(with_global(|g| g.shift_held));
}

/// Shift+Right 建立软标记并移动光标；继续按住 Shift 会扩展选择；
/// 松开 Shift 后的普通移动丢弃软标记。
#[test]
fn shift_right_selects_then_plain_right_clears_mark() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    with_global_mut(|g| {
        g.openfile.as_ref().unwrap().borrow_mut().current_x = 0;
    });

    let code = translate(KeyCode::Right, KeyModifiers::SHIFT);
    assert!(nanoxide::winio::handle_input_key(code));
    assert_eq!(current_x(), 1);
    let (marked, soft) = mark_state();
    assert!(marked && soft, "Shift+Right 应建立软标记");

    let code = translate(KeyCode::Right, KeyModifiers::SHIFT);
    nanoxide::winio::handle_input_key(code);
    assert_eq!(current_x(), 2, "继续 Shift+Right 应扩展选择");
    assert!(mark_state().0, "按住 Shift 时标记应保留");

    let code = translate(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(code, KEY_RIGHT);
    nanoxide::winio::handle_input_key(code);
    assert_eq!(current_x(), 3);
    assert!(!mark_state().0, "松开 Shift 后移动应丢弃软标记");
}

/// Shift+Down 设置软标记并下移一行，不插入任何乱码字符。
#[test]
fn shift_down_selects_without_inserting_text() {
    setup();
    nanoxide::text::inject(b"one", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"two", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap();
        let mut r = of.borrow_mut();
        r.current = r.filetop.clone();
        r.current_x = 0;
    });

    let lines_before = line_count();
    let code = translate(KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(code, KEY_DOWN);
    assert!(nanoxide::winio::handle_input_key(code));

    assert_eq!(line_count(), lines_before, "Shift+Down 不应插入字符或行");
    let (marked, soft) = mark_state();
    assert!(marked && soft, "Shift+Down 应建立软标记");
    assert_eq!(
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
        }),
        "two"
    );
}

/// 选中区域与可视块求交（对应 C 版 draw_row 的 mark 分支）：
/// 单行中间、行首、跨行整行、范围外以及水平滚动。
#[test]
fn selection_paint_range_covers_marked_part() {
    let data = b"hello world";

    /* 单行中间 5 个字符。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, data, 0, 1, 1, 6, 1, 11),
        Some((6, 5))
    );

    /* 从行首到行中。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, data, 0, 1, 1, 0, 1, 5),
        Some((0, 5))
    );

    /* 跨行时中间行整行选中。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, data, 0, 2, 1, 6, 3, 2),
        Some((0, data.len()))
    );

    /* 当前行不在标记区域。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, data, 0, 2, 1, 0, 1, 5),
        None
    );

    /* 终点越过行尾时画到 converted 末尾。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, data, 0, 1, 1, 6, 1, 100),
        Some((6, 5))
    );

    /* 非软换行水平滚动：from_col=6 时本块是 "world"。 */
    assert_eq!(
        nanoxide::winio::selection_paint_range(data, b"world", 6, 1, 1, 0, 1, 11),
        Some((0, 5))
    );
}

/// SELECTED_TEXT 默认取 hilite_attribute（C 的 A_REVERSE），
/// 确保选中区域有可见属性。
#[test]
fn selected_text_defaults_to_reverse() {
    setup();
    with_global_mut(|g| g.hilite_attribute = nanoxide::color::A_REVERSE);
    nanoxide::color::set_interface_colorpairs();

    let attrs = with_global(|g| g.interface_color_pair[SELECTED_TEXT]);
    assert_ne!(
        attrs & nanoxide::color::A_REVERSE,
        0,
        "SELECTED_TEXT 默认应反显"
    );
}
