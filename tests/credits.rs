//! 片尾彩蛋（对应 winio.c 的 `do_credits`，ENABLE_EXTRA）与 "zzy" 触发条件。
//!
//! 动画本身依赖真实终端，集成测试只验证与终端无关的部分：
//! - 文本表与 C 版 `credits[]` 一致（条数、固定条目、NULL 占位的展开）；
//! - 新增的翻译键在 en-US / zh-CN 两个语言文件中都存在。

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    nanoxide::global::global_init();
    nanoxide::i18n::init();
}

/// 对应 C 版 `credits[]` 的 52 行：固定条目逐字一致，NULL 位置已展开。
#[test]
fn credit_sequence_matches_c_table() {
    setup();
    let lines = nanoxide::winio::credits_sequence();

    assert_eq!(lines.len(), 52, "CREDIT_LEN");

    /* C 版第 2 行是编译时的 VERSION；本程序显示自身版本。 */
    assert_eq!(lines[2], env!("CARGO_PKG_VERSION"));
    assert_eq!(lines[3], "");
    assert_eq!(lines[5], "Chris Allegretta");
    assert_eq!(lines[6], "Benno Schulenberg");
    assert_eq!(lines[7], "David Lawrence Ramsey");
    assert_eq!(lines[24], "");
    assert_eq!(lines[26], "Monique, Brielle & Joseph");
    assert_eq!(lines[27], "Plattsburgh State University");
    assert_eq!(lines[32], "Richard Kolb II");
    assert_eq!(lines[34], "Linus Torvalds");
    assert_eq!(lines[37], "Thomas Dickey");
    assert_eq!(lines[40], "Eric S. Raymond");
    assert_eq!(lines[42], "");
    assert_eq!(lines[43], "");
    assert_eq!(lines[45], "");
    assert_eq!(lines[46], "");
    assert_eq!(lines[47], "(C) 2026");
    assert_eq!(lines[48], "Free Software Foundation, Inc.");
    assert_eq!(lines[49], "");
    assert_eq!(lines[50], "");
    assert_eq!(lines[51], "https://nano-editor.org/");

    /* C 版用 NULL 占位、运行时按顺序取译本的 9 行都必须非空。 */
    for index in [0usize, 1, 4, 25, 33, 35, 36, 41, 44] {
        assert!(!lines[index].is_empty(), "第 {index} 行应替换为译本");
    }
}

/// 新增的片尾文案与 "Too tiny" 必须在两种语言里都有翻译。
#[test]
fn credit_and_too_tiny_keys_exist_in_both_locales() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let keys = [
        "files-too_tiny",
        "winio-credits_the_nano_text_editor",
        "winio-credits_version",
        "winio-credits_brought_to_you_by",
        "winio-credits_special_thanks_to",
        "winio-credits_the_free_software_foundation",
        "winio-credits_the_many_translators",
        "winio-credits_for_ncurses",
        "winio-credits_and_anyone_else",
        "winio-credits_thank_you",
    ];

    for lang in ["en-US.ftl", "zh-CN.ftl"] {
        let text = std::fs::read_to_string(dir.join(lang))
            .unwrap_or_else(|err| panic!("读取 {lang} 失败：{err}"));
        for key in keys {
            assert!(
                text.contains(&format!("\n{key} = ")),
                "{lang} 缺少翻译键 {key}"
            );
        }
    }
}
