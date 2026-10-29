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

/// 正则替换展开 \1..\9（对应 C 的 replace_regexp / replace_line）。
#[test]
fn regexp_replacement_expands_backreferences() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("([a-z]+)-([0-9]+)"));

    nanoxide::text::inject(b"abc-123", 7);
    with_global_mut(|g| {
        g.answer = Some(r"\2-\1".to_string());
        g.openfile.as_ref().unwrap().borrow_mut().current_x = 0;
    });

    let data = b"abc-123";
    assert_eq!(nanoxide::search::regexp_match_len(data, 0, 7), 7);

    let (g0, g1, g2) = with_global(|g| (g.regmatches[0], g.regmatches[1], g.regmatches[2]));
    assert_eq!(g0, (Some(0), Some(7)));
    assert_eq!(g1, (Some(0), Some(3)));
    assert_eq!(g2, (Some(4), Some(7)));

    let mut expanded = String::new();
    let size = nanoxide::search::replace_regexp(Some(&mut expanded));
    assert_eq!(expanded, "123-abc");
    assert_eq!(size, 7);

    assert_eq!(
        nanoxide::search::replace_line("([a-z]+)-([0-9]+)"),
        "123-abc"
    );

    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}

/// 组号超过 re_nsub 时 `\2` 按字面量保留（对应 C 的
/// `digit <= search_regexp.re_nsub` 条件）。
#[test]
fn regexp_replacement_keeps_out_of_range_backrefs_literal() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("(a)"));

    nanoxide::text::inject(b"a", 1);
    with_global_mut(|g| {
        g.answer = Some(r"\2".to_string());
        g.openfile.as_ref().unwrap().borrow_mut().current_x = 0;
    });

    assert_eq!(nanoxide::search::regexp_match_len(b"a", 0, 1), 1);
    let mut expanded = String::new();
    nanoxide::search::replace_regexp(Some(&mut expanded));
    assert_eq!(expanded, r"\2");

    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}

/// 非正则替换不展开反斜杠（对应 C 的 else 分支 `strcpy(..., answer)`）。
#[test]
fn literal_replacement_keeps_backslash_literal() {
    setup();
    UNSET(USE_REGEXP);
    nanoxide::text::inject(b"a", 1);
    with_global_mut(|g| {
        g.answer = Some(r"\1".to_string());
        g.openfile.as_ref().unwrap().borrow_mut().current_x = 0;
    });

    assert_eq!(nanoxide::search::replace_line("a"), r"\1");
}

/// 替换文本中的中文按 UTF-8 原样保留（replace_regexp 按字节构造）。
#[test]
fn regexp_replacement_keeps_utf8_replacement_text() {
    setup();
    SET(USE_REGEXP);
    assert!(nanoxide::search::regexp_init_real("(a)"));

    nanoxide::text::inject(b"a", 1);
    with_global_mut(|g| {
        g.answer = Some("中\\1文".to_string());
        g.openfile.as_ref().unwrap().borrow_mut().current_x = 0;
    });

    assert_eq!(nanoxide::search::regexp_match_len(b"a", 0, 1), 1);
    assert_eq!(nanoxide::search::replace_line("(a)"), "中a文");

    UNSET(USE_REGEXP);
    nanoxide::search::regexp_cleanup();
}
