// tests/stdin_args.rs - 命令行 "-"（从标准输入读取）与 colon-parsing、
// 命令行搜索串解析验证
//
// 对应原版 C 代码（nano/src/nano.c main）：
//   - 文件名为 "-" 时不打开文件，而是把标准输入读入一个新缓冲区（scoop_stdin）；
//   - COLON_PARSING：文件不存在且含冒号时，把尾部的 ":行[:列]" 解释为定位；
//   - "+c/pattern" 搜索参数作用于其后的文件。

use nanoxide::definitions::{with_global_mut, SET, COLON_PARSING};
use std::fs;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

fn parse(args: &[&str]) -> Vec<nanoxide::global::FileArg> {
    let v: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    nanoxide::global::parse_file_args(&v)
}

fn temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nanoxide_colon_{}_{}",
        std::process::id(),
        label
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// "-" 单独出现：作为 stdin 标记保留在文件列表中（不再被跳过）。
#[test]
fn dash_means_stdin() {
    setup();
    let r = parse(&["nax", "-"]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "-");
    assert_eq!((r[0].line, r[0].col), (0, 0));
}

/// "-" 与普通文件混用：各自保留且不串行号定位。
#[test]
fn dash_mixed_with_files() {
    setup();
    let r = parse(&["nax", "a.txt", "-", "b.txt"]);
    let names: Vec<&str> = r.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, vec!["a.txt", "-", "b.txt"]);
}

/// +行号只作用于其后紧跟的文件（stdin 也适用）。
#[test]
fn dash_keeps_attached_position() {
    setup();
    let r = parse(&["nax", "+3", "-"]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "-");
    assert_eq!(r[0].line, 3);
}

/// colon-parsing：不存在的 "文件:行" 被拆成真实文件与行号。
#[test]
fn colon_line_is_split_off() {
    setup();
    SET(COLON_PARSING);
    let dir = temp_dir("line");
    let path = dir.join("doc.txt");
    fs::write(&path, "a\nb\nc\n").unwrap();

    let full = format!("{}:2", path.to_str().unwrap());
    let r = parse(&["nax", &full]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, path.to_str().unwrap(), "冒号后的行号应被剥掉");
    assert_eq!((r[0].line, r[0].col), (2, 0));
    let _ = fs::remove_dir_all(&dir);
}

/// colon-parsing：存在的文件（即使名字含冒号）不做拆分。
#[test]
fn existing_colon_file_untouched() {
    setup();
    SET(COLON_PARSING);
    let dir = temp_dir("exist");
    let path = dir.join("weird:1");
    fs::write(&path, "x\n").unwrap();

    let full = path.to_str().unwrap();
    let r = parse(&["nax", full]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, full, "文件存在时不拆分");
    assert_eq!((r[0].line, r[0].col), (0, 0));
    let _ = fs::remove_dir_all(&dir);
}

/// colon-parsing 关闭时（默认）不拆分。
#[test]
fn colon_split_requires_flag() {
    setup();
    let dir = temp_dir("flag");
    let path = dir.join("doc.txt");
    fs::write(&path, "x\n").unwrap();

    let full = format!("{}:3", path.to_str().unwrap());
    let r = parse(&["nax", &full]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, full, "未启用 COLON_PARSING 时原样保留");
    assert_eq!((r[0].line, r[0].col), (0, 0));
    let _ = fs::remove_dir_all(&dir);
}

/// 命令行搜索串 +c/pattern 作用于其后文件（case 修饰符 c 置位）。
#[test]
fn search_pattern_attaches_to_next_file() {
    setup();
    let r = parse(&["nax", "+c/needle", "a.txt"]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "a.txt");
    assert_eq!(r[0].search.as_deref(), Some("needle"));
    assert!(nanoxide::definitions::ISSET(nanoxide::definitions::CASE_SENSITIVE));
}

/// '?' 表示反向搜索（设置 BACKWARDS_SEARCH）。
#[test]
fn question_mark_means_backwards() {
    setup();
    let r = parse(&["nax", "+?tail", "a.txt"]);
    assert_eq!(r[0].search.as_deref(), Some("tail"));
    assert!(nanoxide::definitions::ISSET(nanoxide::definitions::BACKWARDS_SEARCH));
}
