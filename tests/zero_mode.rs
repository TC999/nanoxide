//! ZERO/MINIBAR/NO_HELP 的窗口布局（对应 nano.c `window_init` 的
//! `compute_editwinrows` / `toprows_for` / `bottomrows_for`）。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
    });
}

/// 普通模式：标题 1 行 + 编辑区 19 行 + 状态 1 行 + 底部 3 行。
#[test]
fn normal_layout_numbers() {
    setup();
    assert_eq!(nanoxide::winio::toprows_for(24), 1);
    assert_eq!(nanoxide::winio::bottomrows_for(24), 3);
    assert_eq!(nanoxide::winio::compute_editwinrows(24), 24 - 1 - 3);
}

/// ZERO：无标题栏，编辑区延伸到状态行（+1）。
#[test]
fn zero_mode_has_no_title_and_extends_edit_area() {
    setup();
    SET(ZERO);
    assert_eq!(nanoxide::winio::toprows_for(24), 0, "ZERO 无标题栏");
    assert_eq!(nanoxide::winio::bottomrows_for(24), 3);
    assert_eq!(
        nanoxide::winio::compute_editwinrows(24),
        24 - 3 + 1,
        "ZERO 编辑区延伸到状态行"
    );
    UNSET(ZERO);
}

/// NO_HELP：底部栏只剩 1 行（状态行）。
#[test]
fn no_help_shrinks_bottom_rows() {
    setup();
    SET(NO_HELP);
    assert_eq!(nanoxide::winio::bottomrows_for(24), 1);
    assert_eq!(nanoxide::winio::compute_editwinrows(24), 24 - 1 - 1);
    UNSET(NO_HELP);
}

/// MINIBAR：无标题栏。
#[test]
fn minibar_hides_title() {
    setup();
    SET(MINIBAR);
    assert_eq!(nanoxide::winio::toprows_for(24), 0);
    UNSET(MINIBAR);
}

/// 极矮终端（<3 行）不画标题栏。
#[test]
fn very_short_terminal_has_no_title() {
    setup();
    assert_eq!(nanoxide::winio::toprows_for(2), 0);
    assert_eq!(nanoxide::winio::compute_editwinrows(2), 1);
}
