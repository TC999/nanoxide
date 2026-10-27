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

/// minibar 右侧元素的位置遵循 C 版 minibar 的固定偏移：
/// 状态字符在 `COLS - 11 - padding`、锚点在 `COLS - 5 - padding`、
/// 百分比在 `COLS - 4 - padding`；文件名过长时 padding 从 2 降为 0。
#[test]
fn minibar_right_positions_follow_c_offsets() {
    /* 80 列、短文件名：两侧各留 2 列。 */
    let (padding, states, anchor, percent) =
        nanoxide::winio::minibar_right_positions(80, 10);
    assert_eq!(padding, 2);
    assert_eq!(states, Some(67)); // 80 - 11 - 2
    assert_eq!(anchor, Some(73)); // 80 - 5 - 2
    assert_eq!(percent, Some(74)); // 80 - 4 - 2

    /* 文件名过长（namewidth + 19 > COLS）时放弃两侧空格。 */
    let (padding, states, anchor, percent) =
        nanoxide::winio::minibar_right_positions(80, 70);
    assert_eq!(padding, 0);
    assert_eq!(states, None, "70 + 14 + 0 不满足 < 80");
    assert_eq!(anchor, Some(75));
    assert_eq!(percent, Some(76));

    /* 窄屏时同样先放弃两侧空格，各元素再按各自的条件隐藏。 */
    let (padding, states, anchor, percent) =
        nanoxide::winio::minibar_right_positions(20, 5);
    assert_eq!(padding, 0, "5 + 19 > 20，放弃两侧空格");
    assert_eq!(states, Some(9)); // 5 + 14 + 0 < 20
    assert_eq!(anchor, Some(15));
    assert_eq!(percent, Some(16));
}

/// `-%` 打开时 minibar 会绘制状态字符（含 CONSTANT_SHOW 的位置与十六进制
/// 区域留白），不应 panic。
#[test]
fn minibar_shows_stateflags_without_panicking() {
    setup();
    SET(MINIBAR);
    SET(STATEFLAGS);
    SET(CONSTANT_SHOW);
    SET(AUTOINDENT);
    SET(SOFTWRAP);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let current = of.borrow().current.clone();
        of.borrow_mut().mark = current;
    });
    nanoxide::winio::minibar();
}
