//! 光标停在 UTF-8 BOM 上时的提示（对应 nano.c 主循环的 BOM 检测，
//! `winio::cursor_is_on_bom`）。

use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    nanoxide::global::shortcut_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.openfile = None;
        g.using_utf8 = true;
        g.COLS = 80;
        g.LINES = 24;
    });
    nanoxide::files::make_new_buffer();
}

fn set_current_x(x: usize) {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().current_x = x;
        }
    });
}

/// 行首且行以 EF BB BF 开头、using_utf8 时判定为停在 BOM 上。
#[test]
fn cursor_on_bom_detects_utf8_bom() {
    setup();
    nanoxide::text::inject("\u{feff}abc".as_bytes(), 6);

    set_current_x(0);
    assert!(nanoxide::winio::cursor_is_on_bom());

    set_current_x(1);
    assert!(!nanoxide::winio::cursor_is_on_bom(), "移开列后不应提示");

    set_current_x(0);
    with_global_mut(|g| g.using_utf8 = false);
    assert!(!nanoxide::winio::cursor_is_on_bom(), "非 UTF-8 模式不应提示");
}

/// 普通文本行首不是 BOM。
#[test]
fn cursor_on_plain_line_is_not_bom() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    set_current_x(0);
    assert!(!nanoxide::winio::cursor_is_on_bom());
}

/// 状态栏延迟被设为 1（对应 set_blankdelay_to_one）。
#[test]
fn bom_notice_sets_blank_delay_to_one() {
    setup();
    nanoxide::winio::set_blankdelay_to_one();
    assert_eq!(with_global(|g| g.statusbar_countdown), 1);
}
