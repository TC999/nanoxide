//! 翻译键完整性：`src/` 里所有 `t!("key")` 字面量键必须同时存在于
//! `locales/en-US.ftl` 与 `locales/zh-CN.ftl`。
//!
//! 这能防止底部栏标签之类的 `t!` 调用写出拼错或漏加的键（漏加时会显示
//! `[[key]]` 占位）。`i18n.rs` 的文档注释里有示例 `t!("key")`，跳过该文件。

use std::collections::BTreeSet;
use std::path::Path;

fn collect_source_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    for entry in std::fs::read_dir(&src).expect("读取 src 目录失败") {
        let path = entry.expect("读取 src 条目失败").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("i18n.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("读取源文件失败");

        /* 前面不是标识符字符才算 t!（排除 format! 等），\s* 覆盖跨行写法。 */
        let pattern = regex_lite_find_keys(&text);
        keys.extend(pattern);
    }

    keys
}

/// 手写的键提取（不引入 regex 依赖）：找到 `t!(` 后跳过空白，读引号内文本。
fn regex_lite_find_keys(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut keys = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let Some(pos) = text[index..].find("t!(") else {
            break;
        };
        let start = index + pos;
        index = start + 3;

        /* 前一个字符是字母/数字/下划线时不是 t! 宏调用（如 format!）。 */
        if start > 0 {
            let prev = bytes[start - 1] as char;
            if prev.is_alphanumeric() || prev == '_' {
                continue;
            }
        }

        let rest = &text[index..];
        let trimmed = rest.trim_start();
        if !trimmed.starts_with('"') {
            continue;
        }
        if let Some(end) = trimmed[1..].find('"') {
            keys.push(trimmed[1..1 + end].to_string());
        }
    }

    keys
}

fn locale_keys(file: &str) -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales").join(file);
    let text = std::fs::read_to_string(path).expect("读取语言文件失败");

    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            line.split_once('=').map(|(key, _)| key.trim().to_string())
        })
        .collect()
}

#[test]
fn every_translation_key_exists_in_both_locales() {
    let keys = collect_source_keys();
    assert!(keys.len() > 200, "提取到的键太少：{}", keys.len());

    for file in ["en-US.ftl", "zh-CN.ftl"] {
        let available = locale_keys(file);
        let missing: Vec<&String> = keys.iter().filter(|key| !available.contains(*key)).collect();
        assert!(missing.is_empty(), "{file} 缺少翻译键：{missing:?}");
    }
}
