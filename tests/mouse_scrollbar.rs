//! -q/--indicator 滚动条点击跳行（对应 C 版 `get_mouseinput` 的
//! sidebar 分支）。
//!
//! 原版在点击编辑区最右列时按
//! `filebot->lineno * mouse_y / editwinrows + 1` 换算目标行，其中
//! `mouse_y` 是编辑窗口内行号（非零时加一），随后调用
//! `goto_line_and_column(..., TRUE)` 并置 `refresh_needed`。

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
        g.sidebar = true;
    });
}

/// 填入 `count` 行，光标停在最后一行。
fn fill_lines(count: usize) {
    for i in 0..count {
        if i > 0 {
            nanoxide::text::do_enter();
        }
        let text = format!("line{i}");
        nanoxide::text::inject(text.as_bytes(), text.len());
    }
}

fn current_lineno() -> isize {
    with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().current.as_ref().map(|c| c.borrow().lineno))
            .unwrap_or(-1)
    })
}

fn filebot_lineno() -> isize {
    with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().filebot.as_ref().map(|c| c.borrow().lineno))
            .unwrap_or(-1)
    })
}

/// C 版换算公式的边界：第 0 行保持 0，其余行加一；400 行文件、
/// 20 行窗口时第 11 行映射到 221，末行会超出文件尾（由调用方钳制）。
#[test]
fn scrollbar_target_line_formula() {
    assert_eq!(nanoxide::winio::scrollbar_line_for_click(0, 20, 400), 1);
    assert_eq!(nanoxide::winio::scrollbar_line_for_click(1, 20, 400), 41);
    assert_eq!(nanoxide::winio::scrollbar_line_for_click(10, 20, 400), 221);
    assert_eq!(nanoxide::winio::scrollbar_line_for_click(19, 20, 400), 401);
    /* editwinrows 为 0 时不能除零（实际窗口至少 1 行）。 */
    assert_eq!(nanoxide::winio::scrollbar_line_for_click(1, 0, 10), 21);
}

/// 点击滚动条顶部、中部、底部会跳到大致对应的行；超出文件末行时
/// 停在最后一行（nano 的 filebot 是末尾魔法行）。
#[test]
fn scrollbar_click_jumps_to_matching_line() {
    setup();
    fill_lines(100);
    /* 100 行内容之后还有一个末尾魔法行，filebot 指向它。 */
    let filebot = filebot_lineno();
    assert_eq!(current_lineno(), 100);

    /* 屏幕第 2 行是编辑区第一行（标题栏占第 1 行）。 */
    assert!(nanoxide::winio::handle_scrollbar_click(79, 2));
    assert_eq!(current_lineno(), 1);

    /* 编辑区第 11 行：101 * 11 / 20 + 1 = 56。 */
    assert!(nanoxide::winio::handle_scrollbar_click(79, 12));
    assert_eq!(current_lineno(), 56);

    /* 编辑区最后一行：101 * 20 / 20 + 1 = 102，钳到魔法行 101。 */
    assert!(nanoxide::winio::handle_scrollbar_click(79, 21));
    assert_eq!(current_lineno(), filebot);
}

/// 非最右列、非主菜单、未开启 -q、以及编辑区之外都不跳转。
#[test]
fn scrollbar_click_ignores_other_positions() {
    setup();
    fill_lines(100);
    let before = current_lineno();

    assert!(!nanoxide::winio::handle_scrollbar_click(78, 12));
    assert_eq!(current_lineno(), before);

    with_global_mut(|g| g.sidebar = false);
    assert!(!nanoxide::winio::handle_scrollbar_click(79, 12));
    assert_eq!(current_lineno(), before);

    with_global_mut(|g| {
        g.sidebar = true;
        g.currmenu = MWHEREIS;
    });
    assert!(!nanoxide::winio::handle_scrollbar_click(79, 12));
    assert_eq!(current_lineno(), before);

    /* 标题栏（第 1 行）与底部栏都不属于编辑区。 */
    with_global_mut(|g| g.currmenu = MMAIN);
    assert!(!nanoxide::winio::handle_scrollbar_click(79, 1));
    assert!(!nanoxide::winio::handle_scrollbar_click(79, 22));
    assert_eq!(current_lineno(), before);
}
