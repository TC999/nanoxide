//! 隔离定位 do_right 死循环：分别测试 do_right 与 edit_refresh。

use nano_rs::definitions::*;
use nano_rs::global::global_init;
use nano_rs::files::make_new_buffer;

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
    let handled = nano_rs::winio::handle_input_key(nano_rs::definitions::KEY_RIGHT);
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

/// write_it_out 应把缓冲区写到指定文件、清除修改标记（对应 files.c write_file 核心）。
#[test]
fn write_it_out_saves_file() {
    setup();
    nano_rs::text::inject(b"hello world", 11);
    let path = std::env::temp_dir().join(format!("nano_rs_test_{}.txt", std::process::id()));
    let ps = path.to_str().unwrap().to_string();
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().filename = Some(ps);
    });
    let n = nano_rs::files::write_it_out(false, true);
    assert!(n > 0, "保存应返回写入字节数");
    let content = std::fs::read_to_string(&path).expect("文件应已写入");
    assert_eq!(content, "hello world\n");
    let modified = with_global(|g| g.openfile.as_ref().unwrap().borrow().modified);
    assert!(!modified, "保存后应清除修改标记");
    let _ = std::fs::remove_file(&path);
}

/// "Write to File" 提示栏应显示前缀与可编辑的回答（不崩溃）。
#[test]
fn write_file_promptbar_displays() {
    nano_rs::global::global_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.prompt = Some("Write to File".to_string());
        g.answer = Some("test_wo.txt".to_string());
    });
    nano_rs::prompt::draw_the_promptbar();
    assert_eq!(nano_rs::prompt::get_answer(), "test_wo.txt");
    assert_eq!(with_global(|g| g.prompt.clone()), Some("Write to File".to_string()));
}

/// 吸收链：普通字符应加入 answer（修复 waiting_keycodes 阻塞读取后）。
#[test]
fn absorb_character_adds_to_answer() {
    nano_rs::global::global_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.currmenu = MWRITEFILE;
        g.prompt = Some("Write to File".to_string());
        g.answer = Some(String::new());
    });
    // 输入 'a'（无快捷键）
    nano_rs::prompt::absorb_character(97, None);
    assert_eq!(nano_rs::prompt::get_answer(), "a", "普通字符应被吸收进 answer");
    // 再输入 'b'
    nano_rs::prompt::absorb_character(98, None);
    assert_eq!(nano_rs::prompt::get_answer(), "ab");
}

/// Enter 不应被 waiting_keycodes 阻塞（吸收函数不读取按键）。
#[test]
fn absorb_enter_does_not_block() {
    nano_rs::global::global_init();
    with_global_mut(|g| {
        g.currmenu = MWRITEFILE;
        g.answer = Some("test.txt".to_string());
    });
    // DoEnter 作为 function 传入，不应阻塞（waiting_keycodes 非阻塞）
    nano_rs::prompt::absorb_character(13, Some(FunctionId::DoEnter));
    assert_eq!(nano_rs::prompt::get_answer(), "test.txt");
}

/// 提示中 Backspace 应删除光标前字符、Delete 删除光标处字符。
#[test]
fn statusbar_backspace_and_delete_remove_chars() {
    nano_rs::global::global_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.currmenu = MWRITEFILE;
        g.answer = Some("abc.txt".to_string());
        g.typing_x = 7;
    });
    nano_rs::prompt::handle_editing(FunctionId::DoBackspace);
    assert_eq!(nano_rs::prompt::get_answer(), "abc.tx", "退格应删除光标前字符");

    with_global_mut(|g| {
        g.answer = Some("abc.txt".to_string());
        g.typing_x = 1;
    });
    nano_rs::prompt::handle_editing(FunctionId::DoDelete);
    assert_eq!(nano_rs::prompt::get_answer(), "ac.txt", "Delete 应删除光标处字符");
}

/// bottombars(menu) 应把当前菜单切到该菜单（对应 C 的 bottombars 内部 currmenu = menu）。
#[test]
fn bottombars_switches_currmenu() {
    nano_rs::global::global_init();
    nano_rs::global::shortcut_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.currmenu = MMAIN;
    });
    nano_rs::winio::bottombars(MWRITEFILE);
    assert_eq!(with_global(|g| g.currmenu), MWRITEFILE, "bottombars(MWRITEFILE) 应切换当前菜单");
    nano_rs::winio::bottombars(MMAIN);
    assert_eq!(with_global(|g| g.currmenu), MMAIN, "bottombars(MMAIN) 应切回主菜单");
}

/// 行号边距：未开启 LINE_NUMBERS 时为 0，开启后为 总行数位数+1。
#[test]
fn linenumbers_margin() {
    setup();
    let m0 = nano_rs::winio::current_margin();
    assert_eq!(m0, 0, "默认不显示行号");
    SET(LINE_NUMBERS);
    let m1 = nano_rs::winio::current_margin();
    assert_eq!(m1, 2, "单行文件行号位数1 + 空格 = 2");
    /* 构造 10 行：9 次回车 + 每行一个字符。 */
    for _ in 0..9 {
        nano_rs::text::inject(b"a", 1);
        nano_rs::text::do_enter();
    }
    nano_rs::text::inject(b"a", 1);
    nano_rs::files::prepare_for_display();
    let m2 = nano_rs::winio::current_margin();
    assert_eq!(m2, 3, "10 行文件行号位数2 + 空格 = 3");

    UNSET(LINE_NUMBERS);
    assert_eq!(nano_rs::winio::current_margin(), 0, "关闭后恢复 0");
}

/// 选项可出现在文件名之后（对应 GNU getopt 的重排 argv）。
#[test]
fn args_after_filename_parsed() {
    setup();
    // 文件名在前，选项在后
    let f = nano_rs::global::parse_args(&[
        "nano-rs".to_string(), "file.txt".to_string(), "-l".to_string(),
    ]);
    assert_eq!(f.as_deref(), Some("file.txt"));
    assert!(ISSET(LINE_NUMBERS), "-l 在文件名后应生效");
    UNSET(LINE_NUMBERS);
    // 选项在前，文件名在后（常规形式）
    let f2 = nano_rs::global::parse_args(&[
        "nano-rs".to_string(), "--linenumbers".to_string(), "file.txt".to_string(),
    ]);
    assert_eq!(f2.as_deref(), Some("file.txt"));
    assert!(ISSET(LINE_NUMBERS));
    UNSET(LINE_NUMBERS);
    // "--" 之后不再解析选项
    let f3 = nano_rs::global::parse_args(&[
        "nano-rs".to_string(), "--".to_string(), "file.txt".to_string(), "-l".to_string(),
    ]);
    assert_eq!(f3.as_deref(), Some("file.txt"));
    assert!(!ISSET(LINE_NUMBERS), "-- 之后 -l 应视为文件名而不生效");
}

/// Ctrl+G 帮助：组装帮助文本并可换行入缓冲（不应死循环/panic）。
#[test]
fn help_init_and_wrap_works() {
    setup();
    nano_rs::help::help_init();
    let txt = with_global(|g| g.help_text.clone()).unwrap_or_default();
    assert!(!txt.is_empty(), "帮助文本不应为空");
    assert!(txt.contains("Main nano help text"), "应含主帮助标题");
    // 换行入新缓冲（若死循环会卡住该测试）
    nano_rs::help::wrap_help_text_into_buffer();
    let rows = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let mut n = 0;
        let mut l = of.filetop.clone();
        while let Some(x) = l {
            n += 1;
            l = { let r = x.borrow(); r.next.clone() };
        }
        n
    });
    assert!(rows > 5, "帮助缓冲应有多个换行行，实际 {rows}");
}

/// 帮助退出（close_buffer）后应恢复原编辑缓冲的内容。
#[test]
fn help_close_restores_original_buffer() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    // 组装帮助文本并换行入新缓冲（帮助缓冲的 prev = 原编辑缓冲）
    nano_rs::help::help_init();
    nano_rs::help::wrap_help_text_into_buffer();
    let help_rows = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let mut n = 0;
        let mut l = of.filetop.clone();
        while let Some(x) = l {
            n += 1;
            l = { let r = x.borrow(); r.next.clone() };
        }
        n
    });
    assert!(help_rows > 5, "帮助缓冲应有多行，实际 {help_rows}");
    // 模拟 Ctrl+X 退出帮助：丢弃帮助缓冲
    nano_rs::files::close_buffer();
    // 应恢复原编辑缓冲，内容为 "abc"
    let (txt, rows) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let mut s = String::new();
        let mut n = 0;
        let mut l = of.filetop.clone();
        while let Some(x) = l {
            let b = x.borrow();
            s.push_str(&b.data);
            n += 1;
            l = b.next.clone();
        }
        (s, n)
    });
    assert_eq!(txt, "abc", "退出帮助后应恢复原编辑内容");
    assert_eq!(rows, 2, "原缓冲 = abc 行 + 魔法行");
}
