//! 插入/执行提示的翻转键（M-N / M-F / ^X / M-\）。
//!
//! 对应 global.c:1551-1559 与 files.c 的 insert_a_file_or：
//! - ^X 在执行/插入之间切换；
//! - M-F 切换新缓冲区；
//! - M-N 切换“不转换”；
//! - M-\ 在命令前加/去管道符号。

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
        g.editwincols = 80;
    });
}

#[test]
fn insert_and_execute_flip_keys_are_bound() {
    setup();
    let func = |key: i32, menu: i32| {
        nanoxide::global::find_shortcut(key, menu).map(|sc| sc.borrow().func)
    };

    assert_eq!(func(24, MINSERTFILE), Some(FunctionId::FlipExecute), "^X 插入→执行");
    assert_eq!(func(24, MEXECUTE), Some(FunctionId::FlipExecute), "^X 执行→插入");
    assert_eq!(func(0x266, MINSERTFILE), Some(FunctionId::FlipNewBuffer), "M-F 新缓冲");
    assert_eq!(func(0x266, MEXECUTE), Some(FunctionId::FlipNewBuffer), "M-F 新缓冲");
    assert_eq!(func(0x26E, MINSERTFILE), Some(FunctionId::FlipConvert), "M-N 不转换");
    assert_eq!(func(0x25C, MEXECUTE), Some(FunctionId::FlipPipe), "M-\\ 管道");
}

/// M-\ 在回答开头加/去管道符号，并同步 typing_x。
#[test]
fn flip_pipe_adds_and_removes_leading_bar() {
    nanoxide::global::global_init();
    with_global_mut(|g| {
        g.answer = Some("ls -l".to_string());
        g.typing_x = 5;
    });

    nanoxide::prompt::add_or_remove_pipe_symbol_from_answer();
    assert_eq!(nanoxide::prompt::get_answer(), "|ls -l");
    assert_eq!(with_global(|g| g.typing_x), 6);

    nanoxide::prompt::add_or_remove_pipe_symbol_from_answer();
    assert_eq!(nanoxide::prompt::get_answer(), "ls -l");
    assert_eq!(with_global(|g| g.typing_x), 5);
}
