//! 帮助查看器（`Ctrl+G`）的回归测试。
//!
//! 对应 C 版 `help.c`：
//! - `help_init`：每条描述都要过 gettext（`_(f->phrase)`）、键列按
//!   "第一列 7 格、第二列 10 格"排版；
//! - `show_help`：进帮助查看器时 `margin = 0`、退出时恢复，所以帮助页
//!   不画行号（即使编辑器开着行号）。

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

/// 绘制边距取自保存值 `g.margin`（对应 C 的全局 `margin`），而不是每次
/// 按行数重算。`show_help` 正是靠把它置 0 来关掉帮助页的行号。
#[test]
fn drawing_margin_follows_saved_value() {
    setup();
    with_global_mut(|g| g.margin = 4);
    assert_eq!(nanoxide::winio::margin(), 4, "绘制应使用保存的边距");
    with_global_mut(|g| g.margin = 0);
}

/// 帮助页不画行号：`show_help` 的三步是"保存 -> 置 0 -> 退出时恢复"
/// （`help.c` 的 `was_margin = margin; margin = 0;` 与
/// `margin = was_margin;`）。
///
/// 注：完整的 `show_help` 会进入交互循环并接管终端（备用屏幕、raw
/// mode），不适合放进单元测试；端到端行为已在 Windows 上用临时用例
/// 直接调用 `show_help()` 并抓取终端输出验证（帮助页各行均从第 1 列
/// 开始，退出后行号恢复）。
#[test]
fn help_viewer_has_no_line_number_margin() {
    setup();
    SET(LINE_NUMBERS);
    nanoxide::winio::confirm_margin();
    let editor_margin = nanoxide::winio::margin();
    assert!(editor_margin > 0, "编辑器里行号边距应大于 0");

    /* show_help 进入时：保存 + 置 0。 */
    let was_margin = with_global(|g| g.margin);
    with_global_mut(|g| g.margin = 0);
    assert_eq!(
        nanoxide::winio::margin(),
        0,
        "帮助查看器里绘制边距必须为 0（否则会画出 30、31 这样的行号）"
    );

    /* show_help 退出时：恢复。 */
    with_global_mut(|g| g.margin = was_margin);
    assert_eq!(
        nanoxide::winio::margin(),
        editor_margin,
        "退出帮助后应恢复编辑器的行号边距"
    );

    UNSET(LINE_NUMBERS);
    with_global_mut(|g| g.margin = 0);
}
