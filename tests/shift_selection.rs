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
