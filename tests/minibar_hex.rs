//! minibar 的十六进制码（对应 winio.c 的 minibar 中 hexadecimal 与
//! successor 部分，只在 `CONSTANT_SHOW` / `-c` 下显示）。
//!
//! 覆盖：ASCII、行尾（有/无下一行）、UTF-8 码点、零宽后继（最多两个）、
//! 非 UTF-8 模式下的单字节显示。

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
        g.using_utf8 = true;
    });
}

fn set_cursor(x: usize) {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().current_x = x;
        }
    });
}

fn hex() -> (String, bool) {
    nanoxide::winio::hexadecimal_under_cursor()
}

#[test]
fn ascii_and_line_end() {
    setup();
    nanoxide::text::inject(b"A", 1);

    set_cursor(0);
    assert_eq!(hex(), ("U+0041".to_string(), false));

    /* 内容行行尾：后面是文件末尾的魔法空行，显示换行码 U+000A。 */
    set_cursor(1);
    assert_eq!(hex(), ("U+000A".to_string(), false));

    /* 魔法空行本身（最后一行）行首：显示占位符。 */
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let bot = of.borrow().filebot.clone().unwrap();
        of.borrow_mut().current = Some(bot);
        of.borrow_mut().current_x = 0;
    });
    assert_eq!(hex(), ("  ----".to_string(), false));
}

#[test]
fn utf8_codepoint_and_zero_width_successors() {
    setup();
    nanoxide::text::inject("你".as_bytes(), 3);
    set_cursor(0);
    assert_eq!(hex(), ("U+4F60".to_string(), false));

    /* 组合符号（U+0301、U+0302）作为零宽后继显示；第三个不显示。 */
    setup();
    nanoxide::text::inject("a\u{0301}\u{0302}".as_bytes(), 5);
    set_cursor(0);
    assert_eq!(hex(), ("U+0061|0301|0302".to_string(), true));

    setup();
    nanoxide::text::inject("a\u{0301}\u{0302}\u{0303}".as_bytes(), 7);
    set_cursor(0);
    assert_eq!(hex(), ("U+0061|0301|0302".to_string(), true));
}

#[test]
fn non_utf8_shows_single_bytes() {
    setup();
    with_global_mut(|g| g.using_utf8 = false);
    nanoxide::chars::set_using_utf8(false);
    nanoxide::text::inject("é".as_bytes(), 2);

    set_cursor(0);
    assert_eq!(hex(), ("  0xC3".to_string(), false));
    set_cursor(1);
    assert_eq!(hex(), ("  0xA9".to_string(), false));

    nanoxide::chars::set_using_utf8(true);
}

/// `-c` + `-_` 同时打开时 minibar 会绘制十六进制码并让位状态字符，不应 panic。
#[test]
fn minibar_with_constant_show_does_not_panic() {
    setup();
    SET(MINIBAR);
    SET(CONSTANT_SHOW);
    SET(STATEFLAGS);
    nanoxide::text::inject("a\u{0301}".as_bytes(), 3);
    nanoxide::winio::minibar();
}
