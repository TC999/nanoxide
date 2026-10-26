// tests/file_tools.rs - 补全的文件工具函数验证
//
// 对应原版 C 代码（nano/src/files.c is_dir / diralphasort / copy_file）：
//   1. is_dir：展开 ~ 后判定目录（此处验证普通目录/文件判定）；
//   2. diralphasort：目录优先 + 忽略大小写 + 字节序兜底（qsort 排序语义）；
//   3. copy_file：将源流全部字节复制到目标流并返回字节数。

use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nanoxide_tools_{}_{}",
        std::process::id(),
        label
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// diralphasort：目录排在文件前；同类别按忽略大小写的字符序；
/// 忽略大小写相同时再按字节序比较。
#[test]
fn is_dir_and_diralphasort() {
    let dir = temp_dir("sort");
    fs::create_dir_all(dir.join("alpha_dir")).unwrap();
    fs::write(dir.join("beta.txt"), "x").unwrap();
    fs::write(dir.join("Alpha.txt"), "y").unwrap();
    fs::write(dir.join("alpha.txt"), "z").unwrap();

    let p = |name: &str| dir.join(name).to_str().unwrap().to_string();

    let mut list = vec![p("beta.txt"), p("alpha_dir"), p("Alpha.txt"), p("alpha.txt")];
    list.sort_by(|a, b| nanoxide::files::diralphasort(a, b));

    // 目录必须排在第一位。
    assert!(list[0].ends_with("alpha_dir"), "目录应排最前: {list:?}");
    assert!(nanoxide::files::is_dir(&list[0]), "alpha_dir 应判定为目录");
    assert!(!nanoxide::files::is_dir(&list[1]), "文件不应判定为目录");

    // 其余按忽略大小写排序：alpha.txt 与 Alpha.txt 等价（字节序兜底
    // 使大写在前：'A' < 'a'），之后是 beta.txt。
    assert!(
        list[1].ends_with("Alpha.txt") && list[2].ends_with("alpha.txt"),
        "忽略大小写相等时按字节序(大写在前): {list:?}"
    );
    assert!(list[3].ends_with("beta.txt"), "beta 在 alpha 之后: {list:?}");
}

/// copy_file：完整复制内容（含非法 UTF-8 字节）并返回字节数。
#[test]
fn copy_file_streams_bytes() {
    let dir = temp_dir("copy");
    let src = dir.join("src.bin");
    let dst = dir.join("dst.bin");
    fs::write(&src, b"\x00\x01\xFF binary \xFE \xE6\x95\xB0\xE6\x8D\xAE tail").unwrap();

    let inn = fs::File::open(&src).unwrap();
    let out = fs::File::create(&dst).unwrap();
    let copied = nanoxide::files::copy_file(inn, out, true).unwrap();

    assert_eq!(copied, fs::metadata(&src).unwrap().len(), "应复制全部字节");
    assert_eq!(fs::read(&dst).unwrap(), fs::read(&src).unwrap(), "目标内容应逐字节一致");

    let _ = fs::remove_dir_all(&dir);
}

/// is_dir 对不存在的路径返回 false。
#[test]
fn is_dir_missing_path() {
    let dir = temp_dir("missing");
    assert!(!nanoxide::files::is_dir(dir.join("nope").to_str().unwrap()));
    let _ = fs::remove_dir_all(&dir);
}
