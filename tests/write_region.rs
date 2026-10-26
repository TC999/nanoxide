// tests/write_region.rs - 写选定区域（write_region_to_file）验证
//
// 对应原版 C 代码（nano/src/files.c write_region_to_file / write_file）：
//   1. 区域 = 从标记位置到光标位置（含头不含尾），多行时行间以 \n 连接；
//   2. 区域末字符后补 \n（除非 NO_NEWLINES）；
//   3. 光标（或标记）停在区域末行行首时，该行不属于区域（bot_x==0 排除末行）；
//   4. annotate=FALSE：区域写入不改变缓冲区文件名、不清除修改标记。

use nanoxide::definitions::{with_global, with_global_mut};
use std::fs;
use std::path::PathBuf;

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
        g.tabsize = 8;
        g.currmenu = nanoxide::definitions::MMAIN;
        g.openfile = None;
        g.statusbar_msg.clear();
        g.lastmessage = nanoxide::definitions::MessageType::Vacuum;
        g.we_are_running = true;
    });
}

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nanoxide_region_{}_{}",
        std::process::id(),
        label
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 打开一个三行文件并把标记设到第 1 行、光标设到给定行/列。
/// `label` 供不同用例使用互不冲突的临时目录。
fn open_and_mark(label: &str, line2_x: usize) -> PathBuf {
    let dir = temp_dir(label);
    let path = dir.join("source.txt");
    fs::write(&path, "aa\nbb\ncc\n").unwrap();
    assert!(matches!(
        nanoxide::files::open_buffer(path.to_str().unwrap()),
        nanoxide::files::OpenBufferResult::FileLoaded
    ));
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        let line1 = of.filetop.clone().unwrap();
        let line2 = line1.borrow().next.clone().unwrap();
        of.mark = Some(line1);
        of.mark_x = 1;
        of.current = Some(line2);
        of.current_x = line2_x;
    });
    path
}

/// 跨行区域：第 1 行自第 2 字符起，第 2 行至第 1 字符（不含）。
/// 文本 "aa\nbb\ncc\n" → 区域 "a\nb" → 文件 "a\nb\n"。
#[test]
fn writes_multiline_region() {
    setup();
    let dir = temp_dir("out");
    let out = dir.join("sel.txt");
    open_and_mark("src_a", 1);

    /* annotate=FALSE：缓冲区文件名与修改标记应保持不变。 */
    nanoxide::files::set_modified();
    let n = nanoxide::files::write_region_to_file(out.to_str().unwrap());
    assert!(n > 0, "区域写入应成功，返回 {n}");
    assert_eq!(fs::read_to_string(&out).unwrap(), "a\nb\n");
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        assert!(of.filename.as_deref().unwrap_or("").ends_with("source.txt"));
        assert!(of.modified, "区域写入不应清除修改标记");
    });
    let _ = fs::remove_dir_all(&dir);
}

/// 光标停在区域末行行首（bot_x == 0）：该行不属于区域。
/// "aa\nbb\ncc\n"，标记在第 1 行第 2 字符、光标第 2 行行首 → 区域 "a" → "a\n"。
#[test]
fn excludes_line_at_row_start() {
    setup();
    let dir = temp_dir("out2");
    let out = dir.join("sel2.txt");
    open_and_mark("src_b", 0);

    let n = nanoxide::files::write_region_to_file(out.to_str().unwrap());
    assert!(n > 0, "区域写入应成功，返回 {n}");
    assert_eq!(fs::read_to_string(&out).unwrap(), "a\n");
    let _ = fs::remove_dir_all(&dir);
}

/// 无标记时返回 -1 且不写文件。
#[test]
fn no_mark_writes_nothing() {
    setup();
    let dir = temp_dir("out3");
    let out = dir.join("sel3.txt");
    let path = dir.join("plain.txt");
    fs::write(&path, "hello\n").unwrap();
    assert!(matches!(
        nanoxide::files::open_buffer(path.to_str().unwrap()),
        nanoxide::files::OpenBufferResult::FileLoaded
    ));

    let n = nanoxide::files::write_region_to_file(out.to_str().unwrap());
    assert_eq!(n, -1, "无标记时不应写文件");
    assert!(!out.exists(), "无标记时不应创建文件");
    let _ = fs::remove_dir_all(&dir);
}
