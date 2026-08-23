// 已知剩余偏差修复的回归测试：锚点导航、字数统计、宏状态、视口移动等。
use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

/// 构造三行缓冲，光标在首行。
fn make_three_lines() {
    nanoxide::text::inject(b"abc", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"def", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"ghi", 3);
    // 回到首行
    nanoxide::movement::do_first_line();
}

/// 锚点放置与前后导航。
#[test]
fn anchor_navigation() {
    setup();
    make_three_lines();

    // 在第二行放置锚点
    nanoxide::movement::do_down();
    nanoxide::text::do_anchor();
    // 在第三行放置锚点
    nanoxide::movement::do_down();
    nanoxide::text::do_anchor();

    // 回到首行
    nanoxide::movement::do_first_line();
    let first_lineno = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().lineno);

    // 下一个锚点 → 第二行
    nanoxide::text::to_next_anchor();
    let lineno = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().lineno);
    assert_eq!(lineno, first_lineno + 1, "to_next_anchor 应跳到第二个锚点");

    // 上一个锚点 → 回到第二行锚点（当前在第三行锚点，prev 是第二行）
    nanoxide::text::to_next_anchor();
    nanoxide::text::to_prev_anchor();
    let lineno = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().lineno);
    assert_eq!(lineno, first_lineno + 1, "to_prev_anchor 应跳回前一个锚点");
}

/// 字数统计与视口移动函数不崩溃。
#[test]
fn count_and_viewport_functions() {
    setup();
    make_three_lines();

    nanoxide::text::count_lines_words_and_characters();
    nanoxide::movement::to_top_row();
    nanoxide::movement::to_bottom_row();
    nanoxide::movement::do_center();
    nanoxide::movement::do_cycle();

    let current = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone());
    assert!(current.is_some(), "视口函数不应破坏 current");
}

/// 宏录制状态切换与缓冲内容。
#[test]
fn macro_recording_state() {
    setup();
    nanoxide::winio::record_macro();
    assert!(with_global(|g| g.recording), "开始录制后 recording 应为 true");

    // 模拟按键记录（含触发停止的键）
    with_global_mut(|g| g.macro_buffer.push(97));
    with_global_mut(|g| g.macro_buffer.push(98));
    with_global_mut(|g| g.macro_buffer.push(25)); // M-U 触发键
    nanoxide::winio::record_macro();
    assert!(!with_global(|g| g.recording), "停止录制后 recording 应为 false");
    assert_eq!(with_global(|g| g.macro_buffer.clone()), vec![97, 98], "触发键应被剪掉");
}

/// 无语法时 linter 与 implant 不应崩溃。
#[test]
fn linter_and_implant_no_crash() {
    setup();
    nanoxide::text::do_linter();
    nanoxide::winio::implant("plain text");
}
