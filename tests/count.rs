//! 字数统计的行/词/字符计数与单复数标签
//! （对应 text.c 的 `count_lines_words_and_characters`）。
//!
//! 行数按 C 的公式：文件末尾魔法行内容为空时不计入；词数用
//! `do_next_word` 从起点逐步前进到终点（与 C 一致）。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.openfile = None;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
    nanoxide::files::make_new_buffer();
}

/// 把标记放在第一行的 `top_x`、光标放在第二行的 `bot_x`。
fn set_region_across_lines(top_x: usize, bot_x: usize) {
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let first = of.borrow().filetop.clone().unwrap();
        let second = first.borrow().next.clone().unwrap();
        let mut r = of.borrow_mut();
        r.mark = Some(first);
        r.mark_x = top_x;
        r.current = Some(second);
        r.current_x = bot_x;
    });
}

/// 把标记与光标放在同一行的 `top_x` / `bot_x`。
fn set_region_in_line(top_x: usize, bot_x: usize) {
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let line = of.borrow().filetop.clone().unwrap();
        let mut r = of.borrow_mut();
        r.mark = Some(line.clone());
        r.mark_x = top_x;
        r.current = Some(line);
        r.current_x = bot_x;
    });
}

fn totsize() -> usize {
    with_global(|g| g.openfile.as_ref().unwrap().borrow().totsize)
}

#[test]
fn empty_buffer_has_no_lines_or_words() {
    setup();
    let (lines, words, chars) = nanoxide::text::count_buffer();
    assert_eq!((lines, words), (0, 0), "魔法空行不算一行");
    assert_eq!(chars, totsize(), "字符数取 totsize");
}

#[test]
fn counts_lines_and_words() {
    setup();
    nanoxide::text::inject(b"one two", 7);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"three", 5);

    let (lines, words, chars) = nanoxide::text::count_buffer();
    assert_eq!(lines, 2, "两行内容，魔法行不计");
    assert_eq!(words, 3);
    assert_eq!(chars, totsize());
}

#[test]
fn single_line_is_counted_once() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    assert_eq!(nanoxide::text::count_buffer().0, 1);
}

/// 单复数标签在两种语言里都存在且非空。
#[test]
fn plural_labels_are_translated() {
    for key in [
        "count-line_one",
        "count-line_many",
        "count-word_one",
        "count-word_many",
        "count-char_one",
        "count-char_many",
        "count-in_selection",
        "count-report",
    ] {
        assert!(!nanoxide::i18n::tx(key).is_empty(), "{key} 应为非空译文");
    }
}

/// 显示统计消息不应 panic。
#[test]
fn reporting_does_not_panic() {
    setup();
    nanoxide::text::inject(b"one", 3);
    nanoxide::text::count_lines_words_and_characters();
}

/// 对应 global.c：M-D 是字数统计，M-Del（ALT_DELETE）才是剪切整行。
#[test]
fn word_count_is_bound_to_meta_d() {
    nanoxide::global::global_init();
    nanoxide::global::shortcut_init();

    let m_d = nanoxide::global::find_shortcut(0x264, MMAIN).map(|sc| sc.borrow().func);
    assert_eq!(m_d, Some(FunctionId::CountWords), "M-D 是字数统计");

    let m_del = nanoxide::global::find_shortcut(ALT_DELETE, MMAIN).map(|sc| sc.borrow().func);
    assert_eq!(m_del, Some(FunctionId::DoCut), "M-Del 才是剪切整行");
}

/// 选区跨两行时只统计选区（行/词/字符与 C 的公式一致）。
#[test]
fn marked_region_across_lines_counts_only_selection() {
    setup();
    nanoxide::text::inject(b"one two", 7);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"three four", 10);

    set_region_across_lines(0, 5);
    assert_eq!(
        nanoxide::text::count_buffer(),
        (2, 3, 13),
        "两行、三个词、13 个字符（\"one two\\nthree\"）"
    );
}

/// 同一行内的选区与零长度选区。
#[test]
fn marked_region_in_one_line() {
    setup();
    nanoxide::text::inject(b"one two", 7);

    set_region_in_line(0, 3);
    assert_eq!(nanoxide::text::count_buffer(), (1, 1, 3), "\"one\"");

    set_region_in_line(2, 2);
    assert_eq!(nanoxide::text::count_buffer(), (0, 0, 0));
}

/// 有选区时消息带 "In Selection:" 前缀（C 的 `_("In Selection:  ")`）。
#[test]
fn report_mentions_selection() {
    setup();
    nanoxide::text::inject(b"one two", 7);
    set_region_in_line(0, 3);
    nanoxide::text::count_lines_words_and_characters();

    let msg = with_global(|g| g.statusbar_msg.clone());
    assert!(
        msg.starts_with(&nanoxide::t!("count-in_selection")),
        "{msg}"
    );
}
