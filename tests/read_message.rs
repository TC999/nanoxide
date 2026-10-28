//! 读取文件后的 "Read N lines" 提示（对应 files.c 的 `read_file` 末尾）。
//!
//! 覆盖：单行/多行/空文件、DOS 格式变体、MINIBAR 下打开文件时静默。
//! 插入文件（undoable = TRUE）需要交互式提示，这里未覆盖。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    UNSET(LOCKING);
    with_global_mut(|g| {
        g.openfile = None;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
}

fn make_file(tag: &str, bytes: &[u8]) -> String {
    let path = std::env::temp_dir().join(format!(
        "nanoxide_read_{}_{}",
        tag,
        std::process::id()
    ));
    std::fs::write(&path, bytes).unwrap();
    path.to_str().unwrap().to_string()
}

fn message() -> String {
    with_global(|g| g.statusbar_msg.clone())
}

#[test]
fn read_message_counts_lines() {
    setup();
    let path = make_file("two", b"one\ntwo\n");
    nanoxide::files::open_buffer(&path);
    assert_eq!(
        message(),
        nanoxide::t!("files-read_many", count = "2")
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn read_message_for_single_line_without_newline() {
    setup();
    let path = make_file("one", b"only");
    nanoxide::files::open_buffer(&path);
    assert_eq!(message(), nanoxide::t!("files-read_one", count = "1"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn read_message_for_empty_file_is_zero_lines() {
    setup();
    let path = make_file("empty", b"");
    nanoxide::files::open_buffer(&path);
    assert_eq!(
        message(),
        nanoxide::t!("files-read_many", count = "0"),
        "空文件的 num_lines 为 0（显示用的空行不算）"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn read_message_mentions_dos_conversion() {
    setup();
    let path = make_file("dos", b"one\r\ntwo\r\n");
    nanoxide::files::open_buffer(&path);
    assert_eq!(
        message(),
        nanoxide::t!("files-read_dos_many", count = "2")
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn read_message_is_silent_under_minibar() {
    setup();
    SET(MINIBAR);
    let path = make_file("mini", b"a\nb\n");
    nanoxide::files::open_buffer(&path);
    assert_ne!(
        message(),
        nanoxide::t!("files-read_many", count = "2"),
        "MINIBAR 下打开文件不显示 Read 消息"
    );
    let _ = std::fs::remove_file(&path);
}

/// 只读文件打开时优先提示不可写（对应 C 版 read_file 的 access(W_OK)）。
#[test]
fn unwritable_file_shows_warning_instead_of_read_message() {
    setup();
    let path = make_file("readonly", b"one\n");
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();

    /* root 下 access(W_OK) 仍成功，此时按"可写"处理，跳过断言。 */
    #[cfg(unix)]
    if unsafe { libc::geteuid() } == 0 {
        let _ = std::fs::remove_file(&path);
        return;
    }

    nanoxide::files::open_buffer(&path);
    assert_eq!(
        message(),
        nanoxide::t!("files-unwritable", filename = path.clone())
    );

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).unwrap();
    let _ = std::fs::remove_file(&path);
}
