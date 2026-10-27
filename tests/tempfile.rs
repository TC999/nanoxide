//! `check_writable_directory` 与 `safe_tempfile_name`
//! （对应 files.c 的 check_writable_directory / safe_tempfile）。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
}

#[test]
fn writable_directory_is_accepted() {
    let dir = std::env::temp_dir().join(format!(
        "nanoxide_tmpdir_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let result = nanoxide::files::check_writable_directory(dir.to_str().unwrap())
        .expect("临时目录应可写");
    assert!(
        result.ends_with('/') || result.ends_with('\\'),
        "返回的目录应以分隔符结尾：{result:?}"
    );
    assert!(std::path::Path::new(&result).is_dir());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_path_and_regular_file_are_rejected() {
    let base = std::env::temp_dir().join(format!(
        "nanoxide_tmpbad_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        nanoxide::files::check_writable_directory(base.to_str().unwrap()).is_none(),
        "不存在的路径应被拒绝"
    );

    std::fs::write(&base, b"x").unwrap();
    assert!(
        nanoxide::files::check_writable_directory(base.to_str().unwrap()).is_none(),
        "普通文件不是目录，应被拒绝"
    );
    let _ = std::fs::remove_file(&base);
}

#[test]
fn safe_tempfile_keeps_the_buffer_extension() {
    setup();
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().filename = Some("sample.rs".to_string());
        }
    });

    let path = nanoxide::files::safe_tempfile_name().expect("应能创建临时文件");
    assert!(std::path::Path::new(&path).is_file(), "临时文件应已创建");
    assert!(path.ends_with(".rs"), "应保留缓冲区的扩展名：{path:?}");
    assert!(
        std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("nano."),
        "文件名应以 nano. 开头：{path:?}"
    );
    let _ = std::fs::remove_file(&path);
}
