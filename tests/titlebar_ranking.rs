//! 标题栏左上角的多缓冲区计数（对应 winio.c 的 `titlebar` 中
//! `upperleft = ranking` / `BRANDING` 的选择）。
//!
//! C 版用 `more_than_one`（"曾经/现在有多个缓冲区"）而不是当前缓冲区数量，
//! 因此关闭回到一个缓冲区后仍显示 `[1/1]`。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.openfile = None;
        g.more_than_one = false;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
    nanoxide::files::make_new_buffer();
}

#[test]
fn single_buffer_shows_version() {
    setup();
    assert!(!with_global(|g| g.more_than_one));

    let text = nanoxide::winio::titlebar_left_text();
    assert!(text.contains("nax"), "{text:?}");
    assert!(!text.contains('['), "{text:?}");
}

#[test]
fn multiple_buffers_show_ranking_instead_of_version() {
    setup();
    nanoxide::files::make_new_buffer();
    assert!(with_global(|g| g.more_than_one));

    assert_eq!(nanoxide::winio::titlebar_left_text(), " [2/2] ");
}

#[test]
fn ranking_is_kept_after_closing_back_to_one_buffer() {
    setup();
    nanoxide::files::make_new_buffer();
    nanoxide::files::close_buffer();

    assert!(with_global(|g| g.openfile.is_some()));
    assert!(
        with_global(|g| g.more_than_one),
        "C 版在关闭缓冲区时不重置 more_than_one"
    );
    assert_eq!(nanoxide::winio::titlebar_left_text(), " [1/1] ");
}

/// 超长中文文件名按字符边界截断，不能 panic
/// （回归：标题栏的字节下标落在多字节字符内部）。
#[test]
fn long_utf8_filename_is_truncated_on_char_boundary() {
    setup();
    let long_name = "这是一个很长很长的中文文件名用于测试标题栏截断行为.txt";
    with_global_mut(|g| {
        g.COLS = 20;
        g.openfile.as_ref().unwrap().borrow_mut().filename = Some(long_name.to_string());
    });
    nanoxide::winio::titlebar(None);
}
