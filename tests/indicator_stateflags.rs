//! `-q/--indicator` 与 `-%/--stateflags` 的显示状态。
//!
//! 对应 nano.c 的 `sidebar = ...` 与 winio.c 的 `show_states_at`。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
}

#[test]
fn stateflags_letters_follow_show_states_at() {
    setup();
    assert_eq!(
        nanoxide::winio::states_string(),
        "     ",
        "默认五个位置都是空格"
    );

    SET(AUTOINDENT);
    SET(BREAK_LONG_LINES);
    SET(SOFTWRAP);
    with_global_mut(|g| {
        /* 设置标记：把 mark 指向当前行。 */
        let of = g.openfile.as_ref().unwrap().clone();
        let current = of.borrow().current.clone();
        of.borrow_mut().mark = current;
        g.recording = true;
    });

    assert_eq!(
        nanoxide::winio::states_string(),
        "IMLRS",
        "依次为 自动缩进/标记/硬换行/录制/软换行"
    );
}

#[test]
fn indicator_shows_only_when_enabled_and_screen_is_big_enough() {
    setup();
    nanoxide::winio::update_sidebar();
    assert!(!with_global(|g| g.sidebar), "未设置 -q 时不显示");

    SET(INDICATOR);
    nanoxide::winio::update_sidebar();
    assert!(with_global(|g| g.sidebar));
    assert_eq!(with_global(|g| g.editwincols), 79, "应让出最右一列");

    with_global_mut(|g| g.LINES = 5);
    nanoxide::winio::update_sidebar();
    assert!(!with_global(|g| g.sidebar), "LINES 必须大于 5");
    assert_eq!(with_global(|g| g.editwincols), 80);

    with_global_mut(|g| {
        g.LINES = 24;
        g.COLS = 9;
    });
    nanoxide::winio::update_sidebar();
    assert!(!with_global(|g| g.sidebar), "COLS 必须大于 9");
}

#[test]
fn refresh_with_sidebar_does_not_panic() {
    setup();
    SET(INDICATOR);
    nanoxide::winio::update_sidebar();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::winio::refresh_screen();
}

#[test]
fn stateflags_titlebar_does_not_panic() {
    setup();
    SET(STATEFLAGS);
    SET(AUTOINDENT);
    nanoxide::text::inject(b"hello", 5);
    nanoxide::winio::refresh_screen();
}
