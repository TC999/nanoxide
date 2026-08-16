//! 定位 do_down 多行场景的死循环。

use nano_rs::definitions::*;
use nano_rs::global::global_init;
use nano_rs::nano::make_new_buffer;

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

fn current_text() -> String {
    with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().data.clone()
    })
}

fn current_lineno() -> isize {
    with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().lineno
    })
}

fn make_three_lines() {
    nano_rs::text::inject(b"LINE ONE", 8);
    nano_rs::text::do_enter();
    nano_rs::text::inject(b"LINE TWO", 8);
    nano_rs::text::do_enter();
    nano_rs::text::inject(b"LINE THREE", 10);
}

/// 多行文件 do_down：应移到第 2 行。
#[test]
fn do_down_moves_to_line_two() {
    setup();
    make_three_lines();
    // 回到第 1 行开头
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    assert_eq!(current_lineno(), 2, "do_down 应移到第 2 行");
    assert_eq!(current_text(), "LINE TWO");
}

/// 下箭头后输入：第 2 行应更新。
#[test]
fn type_after_down_updates_line_two() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    nano_rs::text::inject(b"x", 1);
    assert_eq!(current_text(), "xLINE TWO", "下箭头后输入应更新第 2 行");
}

/// do_up 也应工作。
#[test]
fn do_up_works() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    nano_rs::movement::do_up();
    assert_eq!(current_lineno(), 1);
}

/// 连续下箭头到最后一行。
#[test]
fn down_to_last_line() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    nano_rs::movement::do_down();
    assert_eq!(current_text(), "LINE THREE");
    // 再下箭头到魔法行（空行）
    nano_rs::movement::do_down();
    assert_eq!(current_text(), "");
}

/// do_down + edit_refresh 组合（多行渲染路径）。
#[test]
fn down_plus_refresh() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    nano_rs::winio::edit_refresh();
    nano_rs::text::inject(b"x", 1);
    nano_rs::winio::edit_refresh();
    assert_eq!(current_text(), "xLINE TWO");
}

/// 多行渲染 + place_the_cursor。
#[test]
fn multi_render_with_cursor() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::movement::do_down();
    nano_rs::winio::edit_refresh();
    let row = with_global(|g| g.openfile.as_ref().unwrap().borrow().cursor_row);
    assert_eq!(row, 1, "下箭头后 cursor_row 应为 1（第 2 行）");
}

/// 模拟完整按键序列：'a' → 下箭头 → 'x'。
#[test]
fn full_key_sequence() {
    setup();
    make_three_lines();
    nano_rs::movement::to_first_line();
    nano_rs::nano::handle_input_key('a' as i32);
    nano_rs::nano::handle_input_key(nano_rs::definitions::KEY_DOWN);
    nano_rs::nano::handle_input_key('x' as i32);
    // 第 1 行应有 a（保持目标列 1，x 插到第 2 行第 1 列）
    let line1 = with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().filetop.as_ref().unwrap().borrow().data.clone()
    });
    assert_eq!(line1, "aLINE ONE");
    // 当前应在第 2 行且已有 x
    assert_eq!(current_text(), "LxINE TWO");
}
