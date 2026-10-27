//! `-u/--unix` 与 `-N/--noconvert`：DOS/Unix 行尾的读取与保存。
//!
//! 对应 files.c 的 read_file / write_file：
//! - 默认把 CRLF 转成 LF 存储，fmt 记为 DOS，保存时写回 CRLF。
//! - -N 保留行尾 '\r'（fmt 仍为 NIX），保存时原样写回 CRLF。
//! - -u 强制 fmt = NIX，保存时转成 LF。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
    });
}

fn make_file(tag: &str, bytes: &[u8]) -> String {
    let path = std::env::temp_dir().join(format!(
        "nanoxide_fmt_{}_{}.txt",
        tag,
        std::process::id()
    ));
    std::fs::write(&path, bytes).unwrap();
    path.to_str().unwrap().to_string()
}

fn fmt() -> FormatType {
    with_global(|g| g.openfile.as_ref().unwrap().borrow().fmt)
}

fn first_line() -> String {
    with_global(|g| {
        g.openfile
            .as_ref()
            .unwrap()
            .borrow()
            .filetop
            .as_ref()
            .unwrap()
            .borrow()
            .data
            .clone()
    })
}

fn save_and_read(name: &str) -> Vec<u8> {
    assert!(nanoxide::files::write_it_out(true, true) > 0);
    std::fs::read(name).unwrap()
}

#[test]
fn dos_file_is_loaded_as_unix_and_saved_back_as_dos() {
    setup();
    let name = make_file("dos", b"one\r\ntwo\r\n");
    nanoxide::files::open_buffer(&name);

    assert_eq!(fmt(), FormatType::DosFile, "CRLF 文件应识别为 DOS 格式");
    assert_eq!(first_line(), "one", "内存中应去掉行尾 \\r");

    let bytes = save_and_read(&name);
    assert_eq!(bytes, b"one\r\ntwo\r\n", "保存时应写回 CRLF");
    let _ = std::fs::remove_file(&name);
}

#[test]
fn noconvert_preserves_cr_on_load_and_save() {
    setup();
    SET(NO_CONVERT);
    let name = make_file("noconv", b"one\r\ntwo\r\n");
    nanoxide::files::open_buffer(&name);

    assert_eq!(fmt(), FormatType::NixFile, "-N 时不做格式检测");
    assert_eq!(first_line(), "one\r", "-N 时应保留行尾 \\r");

    let bytes = save_and_read(&name);
    assert_eq!(bytes, b"one\r\ntwo\r\n", "-N 保存后应仍是原始 CRLF");
    let _ = std::fs::remove_file(&name);
}

#[test]
fn make_it_unix_saves_lf() {
    setup();
    SET(MAKE_IT_UNIX);
    let name = make_file("unix", b"one\r\ntwo\r\n");
    nanoxide::files::open_buffer(&name);

    assert_eq!(fmt(), FormatType::NixFile, "-u 应强制 Unix 格式");
    assert_eq!(first_line(), "one");

    let bytes = save_and_read(&name);
    assert_eq!(bytes, b"one\ntwo\n", "-u 保存时应转成 LF");
    let _ = std::fs::remove_file(&name);
}

#[test]
fn plain_unix_file_is_unchanged() {
    setup();
    let name = make_file("plain", b"alpha\nbeta\n");
    nanoxide::files::open_buffer(&name);

    assert_eq!(fmt(), FormatType::NixFile);
    assert_eq!(first_line(), "alpha");
    assert_eq!(save_and_read(&name), b"alpha\nbeta\n");
    let _ = std::fs::remove_file(&name);
}
