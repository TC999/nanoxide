// tests/open_buffer.rs - open_buffer 启动行为（对应 nano.c main + files.c open_buffer/open_file）
//
// 验收点：
//   1. 文件名不存在 → NewFile，在欢迎消息的位置显示 "[ New File ]"，缓冲区带该文件名；
//   2. 参数是目录 → Directory，显示 "[ '<目录>' is a directory ]"，且不创建带目录名的缓冲区
//      （与原版一致：open_buffer 返回 FALSE 后由 main 打开空白缓冲区）；
//   3. 文件存在 → FileLoaded，正常加载；
//   4. 空文件名 → 不带文件名的空白缓冲区。

use nano_rs::definitions::{with_global, with_global_mut};

/// 初始化全局状态与 i18n（locales 指向仓库内的 locales/ 目录）。
fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    nano_rs::global::global_init();
    nano_rs::i18n::init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        // 保证测试之间状态隔离（即使同一线程串行执行）。
        g.openfile = None;
        g.statusbar_msg.clear();
        g.statusbar_centered = false;
        g.lastmessage = nano_rs::definitions::MessageType::Vacuum;
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
    let result = nano_rs::files::open_buffer(name);
    assert!(matches!(result, nano_rs::files::OpenBufferResult::NewFile));

    // main.rs 的 NewFile 分支：在欢迎消息（welcome-message）的位置居中显示。
    nano_rs::winio::statusbar_centered(&format!("[ {} ]", nano_rs::t!("files-new_file")));
    assert_eq!(statusbar_msg(), "[ New File ]");

    // 与原版 open_file 一致：新文件缓冲区也带文件名。
    assert_eq!(openfile_filename().as_deref(), Some(name));
}

/// 参数是目录：返回 Directory，状态栏显示 "[ '<目录>' is a directory ]"，
/// 且不创建带目录名的缓冲区（由 main 后续打开空白缓冲区）。
#[test]
fn open_directory_shows_is_a_directory() {
    setup();
    let result = nano_rs::files::open_buffer("tests");
    assert!(matches!(result, nano_rs::files::OpenBufferResult::Directory));

    assert_eq!(statusbar_msg(), "[ 'tests' is a directory ]");
    // 不创建带目录名的缓冲区。
    assert!(openfile_filename().is_none());

    // 模拟 main.rs 的 Directory 分支：打开空白缓冲区供编辑。
    let blank = nano_rs::files::open_buffer("");
    assert!(matches!(blank, nano_rs::files::OpenBufferResult::FileLoaded));
    assert!(openfile_filename().is_none());
}

/// 文件存在：正常加载，状态栏不出现 New File / directory 消息。
#[test]
fn open_existing_file_loads() {
    setup();
    let result = nano_rs::files::open_buffer("Cargo.toml");
    assert!(matches!(result, nano_rs::files::OpenBufferResult::FileLoaded));
    assert_eq!(openfile_filename().as_deref(), Some("Cargo.toml"));
    assert!(statusbar_msg().is_empty());
}

/// 空文件名：创建不带文件名的空白缓冲区。
#[test]
fn open_empty_filename_creates_blank_buffer() {
    setup();
    let result = nano_rs::files::open_buffer("");
    assert!(matches!(result, nano_rs::files::OpenBufferResult::FileLoaded));
    assert!(openfile_filename().is_none());
}
