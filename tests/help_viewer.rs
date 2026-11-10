//! 帮助查看器（`Ctrl+G`）的快捷键列表回归。
//!
//! 对应 C 版 `help.c` 的 `help_init`：
//! - 每条描述都要过 gettext（`sprintf(ptr, "%s\n", _(f->phrase))`）；
//! - 键列按"第一列 7 格、第二列 10 格"排版，描述从第 17 列开始；
//! - `add_to_funcs` 对同一菜单不会重复登记同一函数。

use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    restore_flags(GlobalFlags::new());
    nanoxide::global::shortcut_init();
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.currmenu = MMAIN;
        g.margin = 0;
    });
}

fn help_text() -> String {
    setup();
    nanoxide::help::help_init();
    with_global(|g| g.help_text.clone()).unwrap_or_default()
}

/// 帮助文本里的快捷键描述应是译文，而不是 `*_gist` / `gist-*` 键名。
#[test]
fn help_shortcut_descriptions_are_translated() {
    let text = help_text();

    assert!(text.contains("Main nano help text"), "应含主帮助标题");
    /* 用户报告的症状：帮助页出现 `^G              (F1)help_gist`。 */
    assert!(!text.contains("_gist"), "帮助文本泄漏了内部键名 _gist：\n{text}");
    assert!(!text.contains("gist-"), "帮助文本泄漏了内部键名 gist-：\n{text}");
    assert!(!text.contains("[["), "帮助文本含未找到的翻译键占位：\n{text}");

    /* 抽查两条与原版（C 的 help_gist / exit_gist）一致的行。 */
    assert!(
        text.contains("Display this help text"),
        "缺少 Help 的描述：\n{text}"
    );
    assert!(
        text.contains("Close the current buffer / Exit from nano"),
        "缺少 Exit 的描述：\n{text}"
    );
}

/// 键列宽度按 C 版「第一列 7 格、第二列 10 格」排版：
/// 无论有无替代键，描述都从第 17 列开始。
#[test]
fn help_key_columns_are_seventeen_cells_wide() {
    let text = help_text();

    let help_line = text
        .lines()
        .find(|line| line.contains("Display this help text"))
        .expect("应有 Help 条目");
    assert!(
        help_line.starts_with("^G     (F1)      "),
        "两键条目应排成 7+10 列：{help_line:?}"
    );

    let verbatim = text
        .lines()
        .find(|line| line.contains("Insert the next keystroke verbatim"))
        .expect("应有 Verbatim 条目");
    assert!(
        verbatim.starts_with("M-V              "),
        "单键条目描述也应从第 17 列开始：{verbatim:?}"
    );
}

/// 中文环境下描述应取 `zh_CN.po` 的译文。
///
/// 该用例会改写进程级 `LANG`，与同文件其它用例（固定 en-US）并行会互相
/// 干扰，因此单独放在 `tests/help_i18n.rs`（独立测试二进制 = 独立进程）。
/// 这里只断言"同一次协商下描述与本文件其余用例一致"，作为占位说明。
#[test]
fn help_descriptions_are_looked_up_through_i18n() {
    let text = help_text();
    /* 描述是查表结果，而不是把 phrase 键名直接拼进去。 */
    assert!(
        nanoxide::i18n::tx_or_msgid("gist-help") == "Display this help text",
        "gist-help 应能在 en-US 中查到"
    );
    assert!(text.contains("Display this help text"));
}
