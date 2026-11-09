//! 提示栏（搜索/跳转等）里按 Alt 组合的行为。
//!
//! 对应 prompt.c 的 `absorb_character`：
//! `if (input < 0x20 || meta_key) beep();` —— Alt 组合在提示栏里不是
//! 可插入的字符，应响铃丢弃。
//!
//! 这一条同时守住一个附带缺陷：本项目用 `0x200 + 字符` 表达 Alt，
//! 若输入层没有置位 `meta_key`，提示栏的判断 `input < 0x20 || meta_key`
//! 两个条件都不成立，于是 Alt+d 的 0x264 会被当成普通码点注入回答——
//! 实测表现为在搜索框里凭空出现一个 U+0264 字符（`ɤ`）。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
        g.answer = Some(String::new());
        g.typing_x = 0;
    });
}

/// 在提示栏里按 Alt+d：应丢弃（不写进回答），而不是截断成 'd' 插入。
#[test]
fn alt_combo_is_not_inserted_into_answer() {
    setup();
    let code = nanoxide::winio::translate_keycode(KeyEvent::new_with_kind(
        KeyCode::Char('d'),
        KeyModifiers::ALT,
        KeyEventKind::Press,
    ));
    assert_eq!(code, 0x264, "Alt+d 的键码约定");
    assert!(
        with_global(|g| g.meta_key),
        "输入层必须置位 meta_key，否则提示栏无法区分 Alt 组合与普通字符"
    );

    nanoxide::prompt::absorb_character(code, None);
    assert_eq!(
        nanoxide::prompt::get_answer(),
        "",
        "Alt 组合不应被截断成字母插入回答"
    );
}

/// 普通字符仍应正常进入回答（确认上一条不是因“什么都不插”而通过）。
#[test]
fn plain_chars_still_go_into_answer() {
    setup();
    let code = nanoxide::winio::translate_keycode(KeyEvent::new_with_kind(
        KeyCode::Char('d'),
        KeyModifiers::NONE,
        KeyEventKind::Press,
    ));
    assert!(!with_global(|g| g.meta_key), "普通按键不应置位 meta_key");
    nanoxide::prompt::absorb_character(code, None);
    assert_eq!(nanoxide::prompt::get_answer(), "d", "普通字符应进入回答");
}

/// 多字节字符（中文）在提示栏里仍按码点注入，不受 Alt 收窄影响。
#[test]
fn multibyte_still_injected_in_prompt() {
    setup();
    let code = nanoxide::winio::translate_keycode(KeyEvent::new_with_kind(
        KeyCode::Char('中'),
        KeyModifiers::NONE,
        KeyEventKind::Press,
    ));
    nanoxide::prompt::absorb_character(code, None);
    assert_eq!(nanoxide::prompt::get_answer(), "中", "中文应能输入提示栏");
}
