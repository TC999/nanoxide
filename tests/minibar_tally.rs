//! minibar 的行数与多缓冲区计数（对应 winio.c 的 `minibar`）。
//!
//! 覆盖：
//! - `buffer_number` 从第一个缓冲区开始计数（C 版沿 prev 数到 startfile）；
//! - `report_size` 在 minibar 绘制后被消费，屏幕过窄时保留；
//! - MINIBAR 下保存成功后改为请求行数显示，而不是在状态栏写消息。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
}

fn temp_target(tag: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "nanoxide_mb_{}_{}.txt",
        tag,
        std::process::id()
    ));
    path.to_str().unwrap().to_string()
}

#[test]
fn buffer_number_counts_from_the_first_buffer() {
    setup();
    let first = with_global(|g| g.openfile.clone()).unwrap();
    assert_eq!(nanoxide::winio::buffer_number(&first), 1);

    nanoxide::files::make_new_buffer();
    let second = with_global(|g| g.openfile.clone()).unwrap();
    assert_eq!(nanoxide::winio::buffer_number(&second), 2);
    assert_eq!(
        nanoxide::winio::buffer_number(&first),
        1,
        "旧缓冲区仍是第 1 个"
    );
}

#[test]
fn minibar_consumes_report_size_only_when_wide_enough() {
    setup();
    SET(MINIBAR);

    with_global_mut(|g| g.report_size = true);
    nanoxide::winio::minibar();
    assert!(
        !with_global(|g| g.report_size),
        "80 列时应绘制行数并清除 report_size"
    );

    /* C 版只在 COLS > 35 时显示行数，否则保留请求等下次。 */
    with_global_mut(|g| {
        g.COLS = 30;
        g.report_size = true;
    });
    nanoxide::winio::minibar();
    assert!(
        with_global(|g| g.report_size),
        "屏幕过窄时不应消费 report_size"
    );
}

#[test]
fn saving_under_minibar_requests_line_count() {
    setup();
    SET(MINIBAR);
    nanoxide::text::inject(b"one", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"two", 3);
    with_global_mut(|g| g.report_size = false);

    let target = temp_target("save");
    assert!(
        nanoxide::files::save_to_with_method(&target, WritingType::Overwrite) > 0
    );
    assert!(
        with_global(|g| g.report_size),
        "MINIBAR 下保存后应请求显示行数"
    );
    let _ = std::fs::remove_file(&target);
}

#[test]
fn saving_without_minibar_does_not_request_line_count() {
    setup();
    UNSET(MINIBAR);
    nanoxide::text::inject(b"one", 3);
    with_global_mut(|g| g.report_size = false);

    let target = temp_target("plain");
    assert!(
        nanoxide::files::save_to_with_method(&target, WritingType::Overwrite) > 0
    );
    assert!(!with_global(|g| g.report_size));
    let _ = std::fs::remove_file(&target);
}
