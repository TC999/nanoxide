// tests/open_buffer.rs - open_buffer 启动行为（对应 nano.c main + files.c open_buffer/open_file）
//
// 验收点：
//   1. 文件名不存在 → NewFile，在欢迎消息的位置显示 "[ New File ]"，缓冲区带该文件名；
//   2. 参数是目录 → Directory，显示 "[ '<目录>' is a directory ]"，且不创建带目录名的缓冲区
//      （与原版一致：open_buffer 返回 FALSE 后由 main 打开空白缓冲区）；
//   3. 文件存在 → FileLoaded，正常加载；
//   4. 空文件名 → 不带文件名的空白缓冲区。

use nanoxide::definitions::{with_global, with_global_mut};

/// 初始化全局状态与 i18n（locales 指向仓库内的 locales/ 目录）。
/// 固定 LANG=en-US，使断言不受开发机系统语言影响。
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
        // 保证测试之间状态隔离（即使同一线程串行执行）。
        g.openfile = None;
        g.statusbar_msg.clear();
        g.statusbar_centered = false;
        g.lastmessage = nanoxide::definitions::MessageType::Vacuum;
    });
}

/// 读取全局状态栏当前消息。
fn statusbar_msg() -> String {
    with_global(|g| g.statusbar_msg.clone())
}

/// 读取当前缓冲区文件名（None 表示空白缓冲区）。
fn openfile_filename() -> Option<String> {
    with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().filename.clone())
    })
}

/// 文件不存在：返回 NewFile；按 main.rs 的处理在欢迎消息位置显示 "[ New File ]"。
#[test]
fn open_nonexistent_shows_new_file() {
    setup();
    let name = "_rustnano_nonexistent_xyz.tmp";
    let result = nanoxide::files::open_buffer(name);
    assert!(matches!(result, nanoxide::files::OpenBufferResult::NewFile));

    // main.rs 的 NewFile 分支：在欢迎消息（welcome-message）的位置居中显示。
    nanoxide::winio::statusbar_centered(&format!("[ {} ]", nanoxide::t!("files-new_file")));
    assert_eq!(statusbar_msg(), "[ New File ]");

    // 与原版 open_file 一致：新文件缓冲区也带文件名。
    assert_eq!(openfile_filename().as_deref(), Some(name));
}

/// 参数是目录：返回 Directory，状态栏显示 "[ '<目录>' is a directory ]"，
/// 且不创建带目录名的缓冲区（由 main 后续打开空白缓冲区）。
#[test]
fn open_directory_shows_is_a_directory() {
    setup();
    let result = nanoxide::files::open_buffer("tests");
    assert!(matches!(result, nanoxide::files::OpenBufferResult::Directory));

    assert_eq!(statusbar_msg(), "[ 'tests' is a directory ]");
    // 不创建带目录名的缓冲区。
    assert!(openfile_filename().is_none());

    // 模拟 main.rs 的 Directory 分支：打开空白缓冲区供编辑。
    let blank = nanoxide::files::open_buffer("");
    assert!(matches!(blank, nanoxide::files::OpenBufferResult::FileLoaded));
    assert!(openfile_filename().is_none());
}

/// 文件存在：正常加载，状态栏显示 "Read N lines" 而不是 New File / directory。
#[test]
fn open_existing_file_loads() {
    setup();
    let result = nanoxide::files::open_buffer("Cargo.toml");
    assert!(matches!(result, nanoxide::files::OpenBufferResult::FileLoaded));
    assert_eq!(openfile_filename().as_deref(), Some("Cargo.toml"));

    /* read_file 末尾会报告读入的行数（对应 C 的 "Read %zu line(s)"）；
     * 工作区文件按 core.autocrlf 可能是 CRLF，故按实际字节判断 DOS 变体。 */
    let bytes = std::fs::read("Cargo.toml").unwrap();
    let line_count = String::from_utf8_lossy(&bytes).lines().count();
    let dos = bytes.windows(2).any(|pair| pair == b"\r\n");
    let count = line_count.to_string();
    let expected = match (dos, line_count == 1) {
        (true, true) => nanoxide::t!("files-read_dos_one", count = count),
        (true, false) => nanoxide::t!("files-read_dos_many", count = count),
        (false, true) => nanoxide::t!("files-read_one", count = count),
        (false, false) => nanoxide::t!("files-read_many", count = count),
    };
    assert_eq!(statusbar_msg(), expected);
}

/// 空文件名：创建不带文件名的空白缓冲区。
#[test]
fn open_empty_filename_creates_blank_buffer() {
    setup();
    let result = nanoxide::files::open_buffer("");
    assert!(matches!(result, nanoxide::files::OpenBufferResult::FileLoaded));
    assert!(openfile_filename().is_none());
}

/// 以分隔符结尾且不存在的文件名应被拒绝（对应 C 的 invalid filename）。
#[test]
fn open_invalid_filename_is_rejected() {
    setup();
    let name = "no_such_dir_xyz/";
    let result = nanoxide::files::open_buffer(name);
    assert!(matches!(
        result,
        nanoxide::files::OpenBufferResult::Unsupported
    ));

    let expected = format!(
        "[ {} ]",
        nanoxide::t!("files-invalid_filename", filename = name)
    );
    assert_eq!(statusbar_msg(), expected);
}

/// 字符设备文件不加载（对应 C 的 `S_ISCHR` 检查）。
#[cfg(unix)]
#[test]
fn open_device_file_is_rejected() {
    setup();
    let result = nanoxide::files::open_buffer("/dev/null");
    assert!(matches!(
        result,
        nanoxide::files::OpenBufferResult::Unsupported
    ));

    let expected = format!(
        "[ {} ]",
        nanoxide::t!("files-is_device", filename = "/dev/null")
    );
    assert_eq!(statusbar_msg(), expected);
}

/// root 只读提示的判定（对应 C 的 `new_one && !(mode & 0222) && euid==0`）。
#[test]
fn read_only_warning_condition_follows_c() {
    assert!(nanoxide::files::should_warn_read_only(true, 0o444, 1));
    assert!(!nanoxide::files::should_warn_read_only(false, 0o444, 1));
    assert!(!nanoxide::files::should_warn_read_only(true, 0o644, 1));
    assert!(!nanoxide::files::should_warn_read_only(true, 0o444, 0));
}

/// 父目录检查（对应 C 的 `has_valid_path`）。
#[test]
fn has_valid_path_checks_parent_directory() {
    setup();
    with_global_mut(|g| g.statusbar_msg.clear());

    assert!(!nanoxide::files::has_valid_path("no_such_dir_xyz/file.txt"));
    let expected = nanoxide::t!(
        "files-directory_does_not_exist",
        dir = "no_such_dir_xyz"
    );
    assert_eq!(statusbar_msg(), expected);

    /* 父路径是普通文件时报告“不是目录”。 */
    let file = std::env::temp_dir().join(format!("nanoxide_path_{}", std::process::id()));
    std::fs::write(&file, b"x").unwrap();
    let child = format!("{}/child.txt", file.to_str().unwrap());
    assert!(!nanoxide::files::has_valid_path(&child));
    let expected = nanoxide::t!(
        "files-path_not_a_directory",
        path = file.to_str().unwrap()
    );
    assert_eq!(statusbar_msg(), expected);

    /* 当前目录有效。 */
    assert!(nanoxide::files::has_valid_path("plain.txt"));

    let _ = std::fs::remove_file(&file);
}

/// 不可进入的目录（Unix）应被拒绝（对应 C 的 `access(X_OK)` 检查）。
#[cfg(unix)]
#[test]
fn has_valid_path_rejects_inaccessible_directory() {
    use std::os::unix::fs::PermissionsExt;

    /* root 忽略权限位，跳过。 */
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    setup();
    let dir = std::env::temp_dir().join(format!("nanoxide_noexec_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut perms = std::fs::metadata(&dir).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&dir, perms).unwrap();

    let child = format!("{}/x", dir.to_str().unwrap());
    assert!(!nanoxide::files::has_valid_path(&child));
    let expected = nanoxide::t!(
        "files-path_not_accessible",
        path = dir.to_str().unwrap()
    );
    assert_eq!(statusbar_msg(), expected);

    let mut perms = std::fs::metadata(&dir).unwrap().permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(&dir, perms).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
