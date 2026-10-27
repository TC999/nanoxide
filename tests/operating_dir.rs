//! `--operatingdir` 的路径限制（对应 files.c 的 init_operating_dir /
//! outside_of_confinement）。

use nanoxide::definitions::*;

fn setup(operating_dir: &str) -> String {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    let canonical = std::fs::canonicalize(operating_dir).unwrap();
    let dir = canonical.to_string_lossy().into_owned();
    with_global_mut(|g| {
        g.operating_dir = Some(dir.clone());
        g.COLS = 80;
        g.LINES = 24;
    });
    dir
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanoxide_op_{}_{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("inside")).unwrap();
    std::fs::write(root.join("inside/a.txt"), "data\n").unwrap();
    root
}

#[test]
fn inside_and_outside_paths_are_classified() {
    let root = temp_root("classify");
    let _dir = setup(root.to_str().unwrap());

    let inside = root.join("inside/a.txt");
    let sibling = root.with_file_name(format!(
        "{}_other",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("b.txt"), "x\n").unwrap();

    assert!(
        !nanoxide::files::outside_of_confinement(inside.to_str().unwrap(), false),
        "操作目录内的文件"
    );
    assert!(
        !nanoxide::files::outside_of_confinement(root.to_str().unwrap(), false),
        "操作目录本身"
    );
    assert!(
        nanoxide::files::outside_of_confinement(sibling.join("b.txt").to_str().unwrap(), false),
        "仅前缀相同的兄弟目录应算外部"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&sibling);
}

#[test]
fn tabbing_allows_incomplete_parent_paths() {
    let root = temp_root("tabbing");
    let _dir = setup(root.to_str().unwrap());
    let parent = root.parent().unwrap().to_str().unwrap().to_string();

    assert!(
        nanoxide::files::outside_of_confinement(&parent, false),
        "非补全时父目录在操作目录之外"
    );
    assert!(
        !nanoxide::files::outside_of_confinement(&parent, true),
        "补全时父目录可能长成操作目录，应算在里面"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn open_buffer_rejects_outside_path() {
    let root = temp_root("open");
    let _dir = setup(root.to_str().unwrap());
    let outside = std::env::temp_dir().join(format!(
        "nanoxide_op_outside_{}.txt",
        std::process::id()
    ));
    std::fs::write(&outside, "secret\n").unwrap();

    let result = nanoxide::files::open_buffer(outside.to_str().unwrap());
    assert!(
        matches!(result, nanoxide::files::OpenBufferResult::ErrorRead),
        "操作目录外的文件不应打开"
    );
    let msg = with_global(|g| g.statusbar_msg.clone());
    assert!(
        msg.contains("nanoxide_op_"),
        "应提示操作目录限制，实际：{msg:?}"
    );

    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn save_rejects_outside_path() {
    let root = temp_root("save");
    let _dir = setup(root.to_str().unwrap());
    let outside = std::env::temp_dir().join(format!(
        "nanoxide_op_save_{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&outside);

    nanoxide::text::inject(b"hello", 5);
    let written = nanoxide::files::save_to_with_method(
        outside.to_str().unwrap(),
        WritingType::Overwrite,
    );
    assert_eq!(written, -1, "操作目录外保存应失败");
    assert!(!outside.exists(), "不应创建外部文件");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn binary_rejects_invalid_operating_dir() {
    let exe = env!("CARGO_BIN_EXE_nax");
    let bad = std::env::temp_dir().join(format!(
        "nanoxide_op_missing_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&bad);

    let out = std::process::Command::new(exe)
        .args(["-I", "-o"])
        .arg(&bad)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1), "无效操作目录应退出 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("nanoxide_op_missing"),
        "stderr 应包含无效目录名，实际：{stderr}"
    );
}
