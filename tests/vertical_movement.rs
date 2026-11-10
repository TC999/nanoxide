//! 上下移动应保持目标列（对应 C 版 `do_up`/`do_down` 的 placewewant 语义）。
//!
//! 复现问题：多行文本中用左右箭头移动光标后，placewewant 未更新，
//! 于是按上/下箭头时目标列仍是旧值（行首），光标落到新行行首。

use nanoxide::definitions::*;
use nanoxide::files::make_new_buffer;
use nanoxide::global::global_init;

fn setup() {
    global_init();
    make_new_buffer();
    with_global_mut(|g| {
        g.editwincols = 80;
        g.tabsize = 8;
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

/// 返回 (行号, current_x, placewewant)。
fn state() -> (isize, usize, usize) {
    let of_rc = with_global(|g| g.openfile.as_ref().unwrap().clone());
    let of = of_rc.borrow();
    let lineno = of.current.as_ref().unwrap().borrow().lineno;
    (lineno, of.current_x, of.placewewant)
}

/// 造两行相同文本，光标回到第 1 行行首。
fn make_two_lines(text: &str) {
    nanoxide::text::inject(text.as_bytes(), text.len());
    nanoxide::text::do_enter();
    nanoxide::text::inject(text.as_bytes(), text.len());
    nanoxide::movement::to_first_line();
}

/// ASCII：第 1 行右移 3 列后下移，应停在第 2 行第 3 列。
#[test]
fn down_keeps_column_after_moving_right() {
    setup();
    make_two_lines("abcdef");
    for _ in 0..3 {
        nanoxide::movement::do_right();
    }
    assert_eq!(state(), (1, 3, 3));
    nanoxide::movement::do_down();
    assert_eq!(state(), (2, 3, 3), "下箭头应保持目标列 3");
}

/// 中文：第 1 行右移 4 个汉字后下移，应停在第 2 行第 4 个汉字之后。
#[test]
fn down_keeps_column_after_moving_right_cjk() {
    setup();
    make_two_lines("文本文本文本");
    for _ in 0..4 {
        nanoxide::movement::do_right();
    }
    assert_eq!(state(), (1, 12, 8));
    nanoxide::movement::do_down();
    assert_eq!(state(), (2, 12, 8), "下箭头应保持目标列 8（当前列）");
}

/// 从左向右移动后再上移，同样应保持目标列。
#[test]
fn up_keeps_column_after_moving_right() {
    setup();
    make_two_lines("abcdef");
    nanoxide::movement::do_down();
    for _ in 0..4 {
        nanoxide::movement::do_right();
    }
    assert_eq!(state(), (2, 4, 4));
    nanoxide::movement::do_up();
    assert_eq!(state(), (1, 4, 4), "上箭头应保持目标列 4");
}

/// 左移后目标列应随光标回退，上移不能停在旧列。
#[test]
fn up_keeps_column_after_moving_left() {
    setup();
    make_two_lines("abcdef");
    nanoxide::movement::do_down();
    for _ in 0..5 {
        nanoxide::movement::do_right();
    }
    for _ in 0..3 {
        nanoxide::movement::do_left();
    }
    assert_eq!(state(), (2, 2, 2));
    nanoxide::movement::do_up();
    assert_eq!(state(), (1, 2, 2), "上箭头应保持左移后的目标列 2");
}

/// 目标列超出短行时钳到行末，但 placewewant 保留目标列；
/// 再移回长行时恢复目标列（对应 C 版 do_down/do_up 末尾的恢复逻辑）。
#[test]
fn target_column_survives_short_line() {
    setup();
    nanoxide::text::inject(b"abcdefghij", 10);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"ab", 2);
    nanoxide::movement::to_first_line();
    for _ in 0..8 {
        nanoxide::movement::do_right();
    }
    assert_eq!(state(), (1, 8, 8));
    nanoxide::movement::do_down();
    assert_eq!(state(), (2, 2, 8), "短行只钳到行末，目标列仍为 8");
    nanoxide::movement::do_up();
    assert_eq!(state(), (1, 8, 8), "回到长行应恢复目标列 8");
}

/// 真实按键序列：右箭头三次后下箭头，应停在第 2 行第 3 列。
#[test]
fn key_sequence_right_then_down() {
    setup();
    make_two_lines("abcdef");
    nanoxide::winio::handle_input_key(KEY_RIGHT);
    nanoxide::winio::handle_input_key(KEY_RIGHT);
    nanoxide::winio::handle_input_key(KEY_RIGHT);
    nanoxide::winio::handle_input_key(KEY_DOWN);
    assert_eq!(state(), (2, 3, 3), "按键序列应保持目标列 3");
}
