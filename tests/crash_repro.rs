//! 复现用户报告的崩溃：刷新屏幕与保存文件。

use nanoxide::definitions::*;
use nanoxide::global::global_init;
use nanoxide::files::make_new_buffer;

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

/// 刷新屏幕不应崩溃。
#[test]
fn refresh_screen_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    // 直接调用刷新路径
    nanoxide::winio::refresh_screen();
    nanoxide::winio::edit_refresh();
}

/// 保存文件（write_it_out）不应崩溃。
#[test]
fn write_it_out_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().filename = Some("test_save.txt".to_string());
    });
    let result = nanoxide::files::write_it_out(true, false);
    assert!(result > 0 || result == -1, "write_it_out returned {result}");
    let content = std::fs::read_to_string("test_save.txt").unwrap_or_default();
    assert_eq!(content, "hello\n");
    let _ = std::fs::remove_file("test_save.txt");
}

/// 状态栏与标题栏绘制不应崩溃。
#[test]
fn draw_bars_works() {
    setup();
    nanoxide::winio::titlebar(None);
    nanoxide::winio::statusbar("test message");
    nanoxide::winio::bottombars(nanoxide::definitions::MMAIN);
}
