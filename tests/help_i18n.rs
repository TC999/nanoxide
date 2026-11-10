//! 帮助页描述的语言协商（单独成文件：改写进程级 `LANG` 会与并行用例冲突）。
//!
//! 对应 C 版 `help_init` 里对 `f->phrase` 的 `_(...)` 调用：描述取自
//! `po/zh_CN.po` 的 msgstr，而不是 ftl 键名。

use nanoxide::definitions::*;

#[test]
fn help_descriptions_use_chinese_translation() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "zh-CN");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    assert_eq!(nanoxide::i18n::current_lang(), "zh-CN", "应协商到 zh-CN");

    nanoxide::global::shortcut_init();
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.currmenu = MMAIN;
    });
    nanoxide::help::help_init();
    let text = with_global(|g| g.help_text.clone()).unwrap_or_default();

    /* 译文逐条取自 nano/po/zh_CN.po（help_gist / exit_gist）。 */
    assert!(text.contains("显示帮助"), "gist-help 应取 po 译文：\n{text}");
    assert!(
        text.contains("关闭当前缓冲区 / 离开 nano"),
        "gist-exit 应取 po 译文：\n{text}"
    );
    assert!(!text.contains("gist-"), "不应泄漏键名：\n{text}");
    assert!(!text.contains("_gist"), "不应泄漏键名：\n{text}");
}
