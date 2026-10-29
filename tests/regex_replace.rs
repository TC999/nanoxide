//! 正则搜索/替换（对应 search.c 的 regexp_init 与 replace_line）。
//!
//! 验收点：
//!   1. `regexp_init_real` 按 ERE 编译（`a.*b` 能匹配 `axxb`）；
//!   2. 匹配长度取实际匹配（`[0-9]+` 在 `foo123bar` 上是 3），
//!      而不是正则模式的字符串长度。

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
        g.statusbar_msg.clear();
        g.COLS = 80;
        g.LINES = 24;
    });
    nanoxide::files::make_new_buffer();
}

/// 正则按 ERE 编译：`a.*b` 能匹配 `axxb`。
#[test]
fn regexp_search_uses_extended_regex() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("a.*b"));
    assert_eq!(
        nanoxide::utils::strstrwrapper(b"axxb", b"a.*b", 0),
        Some(0),
        "a.*b 应匹配 axxb"
    );
    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}

/// 原来的字面量路径会失败：确认 `a.*b` 不按字面量匹配。
#[test]
fn regexp_is_not_treated_as_literal() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("a.*b"));
    assert_eq!(
        nanoxide::utils::strstrwrapper(b"a.*b", b"a.*b", 0),
        Some(0),
        "字面量 a.*b 也匹配"
    );
    assert_eq!(
        nanoxide::utils::strstrwrapper(b"axxb", b"a.*b", 0),
        Some(0),
        "但 axxb 不应被字面量匹配失败"
    );
    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}

/// 匹配长度取实际匹配长度（对应 C 的 `regmatches[0].rm_eo - rm_so`）。
#[test]
fn match_length_is_actual_match_len() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("[0-9]+"));

    let line = b"foo123bar";
    assert_eq!(
        nanoxide::search::regexp_match_len(line, 3, 5),
        3,
        "实际匹配长度为 3"
    );
    assert_eq!(
        nanoxide::search::regexp_match_len(line, 0, 5),
        5,
        "未在 from 处命中时退化为 needle 长度"
    );

    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}

/// 非正则模式下匹配长度就是 needle 长度。
#[test]
fn match_length_is_needle_len_without_regexp() {
    setup();
    assert_eq!(
        nanoxide::search::regexp_match_len(b"foo123bar", 3, 5),
        5
    );
}
