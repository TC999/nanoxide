//! 隔离定位 do_right 死循环：分别测试 do_right 与 edit_refresh。

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

/// 只调用 do_right（不刷新）。
#[test]
fn do_right_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().current_x = 0;
    });
    nano_rs::movement::do_right();
    let x = with_global(|g| g.openfile.as_ref().unwrap().borrow().current_x);
    assert_eq!(x, 1);
}

/// 只调用 edit_refresh。
#[test]
fn edit_refresh_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    nano_rs::winio::edit_refresh();
}

/// edit_redraw（do_right 的收尾）单独测试。
#[test]
fn edit_redraw_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    let cur = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone().unwrap());
    nano_rs::winio::edit_redraw(&cur, UpdateType::Flowing);
}

/// current_is_offscreen 相关函数单独测试。
#[test]
fn offscreen_checks_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    let above = nano_rs::winio::current_is_above_screen();
    let below = nano_rs::winio::current_is_below_screen();
    let off = nano_rs::winio::current_is_offscreen();
    println!("above={above} below={below} off={off}");
}

/// adjust_viewport 单独测试。
#[test]
fn adjust_viewport_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    nano_rs::winio::adjust_viewport(UpdateType::Flowing);
}

/// handle_input_key(KEY_RIGHT) 应只移动光标。
#[test]
fn handle_right_key_alone() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().current_x = 0;
    });
    let handled = nano_rs::nano::handle_input_key(nano_rs::definitions::KEY_RIGHT);
    assert!(handled);
    let x = with_global(|g| g.openfile.as_ref().unwrap().borrow().current_x);
    assert_eq!(x, 1);
}

/// 不带文件名启动：open_buffer("") 必须设置 edittop/current，place_the_cursor 不 panic。
#[test]
fn empty_buffer_startup_no_panic() {
    setup();
    nano_rs::files::open_buffer("");
    let (cur, edittop) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.current.clone(), of.edittop.clone())
    });
    assert!(cur.is_some(), "空缓冲区必须设置 current");
    assert!(edittop.is_some(), "空缓冲区必须设置 edittop（回归：曾为 None 导致启动崩溃）");
    nano_rs::winio::place_the_cursor();
}

/// 文件不存在时启动：走新文件分支，place_the_cursor 不 panic。
#[test]
fn nonexistent_file_startup_no_panic() {
    setup();
    nano_rs::files::open_buffer("definitely_missing_file_xyz.txt");
    let edittop = with_global(|g| g.openfile.as_ref().unwrap().borrow().edittop.clone());
    assert!(edittop.is_some());
    nano_rs::winio::place_the_cursor();
    let cur = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone());
    assert!(cur.is_some());
}
