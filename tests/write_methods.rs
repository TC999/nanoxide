//! 写文件提示的追加/前置写入与功能键。
//!
//! 对应 files.c 的 write_file(method) 与 write_it_out 循环：
//! M-A append、M-P prepend、M-D DOS 格式、M-B 备份、^Q 放弃。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
    });
}

fn make_target(tag: &str, content: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "nanoxide_wm_{}_{}.txt",
        tag,
        std::process::id()
    ));
    std::fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn overwrite_replaces_existing_content() {
    setup();
    nanoxide::text::inject(b"new", 3);
    let target = make_target("over", "old\n");
    assert!(nanoxide::files::save_to_with_method(&target, WritingType::Overwrite) > 0);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");
    let _ = std::fs::remove_file(&target);
}

#[test]
fn append_adds_after_existing_content() {
    setup();
    nanoxide::text::inject(b"new", 3);
    let target = make_target("append", "old\n");
    assert!(nanoxide::files::save_to_with_method(&target, WritingType::Append) > 0);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old\nnew\n");
    let _ = std::fs::remove_file(&target);
}

#[test]
fn prepend_adds_before_existing_content() {
    setup();
    nanoxide::text::inject(b"new", 3);
    let target = make_target("prepend", "old\n");
    assert!(nanoxide::files::save_to_with_method(&target, WritingType::Prepend) > 0);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\nold\n");
    let _ = std::fs::remove_file(&target);
}

#[test]
fn write_prompt_keys_are_bound() {
    setup();
    let func = |key: i32| {
        nanoxide::global::find_shortcut(key, MWRITEFILE).map(|sc| sc.borrow().func)
    };

    assert_eq!(func(17), Some(FunctionId::DiscardBuffer), "^Q 放弃");
    assert_eq!(func(0x264), Some(FunctionId::DosFormat), "M-D DOS 格式");
    assert_eq!(func(0x262), Some(FunctionId::BackItUp), "M-B 备份");
    assert_eq!(func(0x261), Some(FunctionId::AppendIt), "M-A 追加");
    assert_eq!(func(0x270), Some(FunctionId::PrependIt), "M-P 前置");
    assert_eq!(func(20), Some(FunctionId::ToFiles), "^T 浏览");
}

/// 磁盘满错误的识别（对应 C 版 `errno == ENOSPC` 的警告分支）。
#[cfg(unix)]
#[test]
fn disk_full_error_is_recognized() {
    let full = std::io::Error::from_raw_os_error(libc::ENOSPC);
    assert!(nanoxide::files::is_disk_full(&full));

    let denied = std::io::Error::from_raw_os_error(libc::EACCES);
    assert!(!nanoxide::files::is_disk_full(&denied));
}

/// Windows 的 ERROR_DISK_FULL 是 112。
#[cfg(windows)]
#[test]
fn disk_full_error_is_recognized() {
    let full = std::io::Error::from_raw_os_error(112);
    assert!(nanoxide::files::is_disk_full(&full));

    let denied = std::io::Error::from_raw_os_error(5); // ERROR_ACCESS_DENIED
    assert!(!nanoxide::files::is_disk_full(&denied));
}

/// FIFO 识别与紧急保存的 0600 权限（对应 C 的 `S_ISFIFO` 与
/// `S_IRUSR|S_IWUSR`）。
#[cfg(unix)]
#[test]
fn fifo_detection_and_special_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("nanoxide_fifo_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let fifo = dir.join("pipe");
    let c_path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    /* 安全性：mkfifo 只创建命名管道，不涉及内存或文件内容。 */
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    assert!(nanoxide::files::is_fifo_file(fifo.to_str().unwrap()));

    let plain = dir.join("plain");
    std::fs::write(&plain, b"x").unwrap();
    assert!(!nanoxide::files::is_fifo_file(plain.to_str().unwrap()));
    assert!(!nanoxide::files::is_fifo_file(
        dir.join("missing").to_str().unwrap()
    ));

    /* SPECIAL（紧急保存）新建文件应为 0600。 */
    setup();
    nanoxide::text::inject(b"one", 3);
    let target = dir.join("emergency");
    let target_str = target.to_str().unwrap().to_string();
    assert!(
        nanoxide::files::save_to_with_method(&target_str, WritingType::Special) > 0
    );
    let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "SPECIAL 新建文件应为 0600，实际 {mode:o}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// MAKE_BACKUP：默认在文件名后加 `~`，内容是保存前的原文件。
#[test]
fn backup_appends_tilde_by_default() {
    setup();
    SET(MAKE_BACKUP);

    let dir = std::env::temp_dir().join(format!("nanoxide_bak_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("note.txt");
    std::fs::write(&target, "old\n").unwrap();
    let target_str = target.to_str().unwrap().to_string();

    nanoxide::text::inject(b"new", 3);
    assert!(nanoxide::files::save_to_with_method(&target_str, WritingType::Overwrite) > 0);

    let backup = format!("{}~", target_str);
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "old\n");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");

    UNSET(MAKE_BACKUP);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--backupdir`：备份写到指定目录，完整路径的分隔符换成 `!`。
#[test]
fn backup_uses_backup_directory() {
    setup();
    let dir = std::env::temp_dir().join(format!("nanoxide_bdir_{}", std::process::id()));
    let backups = dir.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let target = dir.join("note.txt");
    std::fs::write(&target, "old\n").unwrap();
    let target_str = target.to_str().unwrap().to_string();

    let sep = std::path::MAIN_SEPARATOR;
    SET(MAKE_BACKUP);
    with_global_mut(|g| {
        g.backup_dir = Some(format!("{}{}", backups.display(), sep));
    });

    nanoxide::text::inject(b"new", 3);
    assert!(nanoxide::files::save_to_with_method(&target_str, WritingType::Overwrite) > 0);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");

    let entries: Vec<_> = std::fs::read_dir(&backups)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "应在备份目录生成一个备份");
    let name = entries[0].file_name().to_string_lossy().to_string();
    assert!(name.contains('!'), "路径分隔符应换成 !，实际：{name}");
    assert_eq!(std::fs::read_to_string(entries[0].path()).unwrap(), "old\n");

    with_global_mut(|g| g.backup_dir = None);
    UNSET(MAKE_BACKUP);
    let _ = std::fs::remove_dir_all(&dir);
}

/// INSECURE_BACKUP：只读的旧备份在普通模式下导致失败，设置后可覆盖。
#[cfg(unix)]
#[test]
fn insecure_backup_overwrites_readonly_backup() {
    use std::os::unix::fs::PermissionsExt;

    setup();
    let dir = std::env::temp_dir().join(format!("nanoxide_ins_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("note.txt");
    std::fs::write(&target, "old\n").unwrap();
    let target_str = target.to_str().unwrap().to_string();
    let backup = format!("{}~", target_str);
    std::fs::write(&backup, "stale\n").unwrap();
    let mut perms = std::fs::metadata(&backup).unwrap().permissions();
    perms.set_mode(0o400);
    std::fs::set_permissions(&backup, perms).unwrap();

    SET(MAKE_BACKUP);
    nanoxide::text::inject(b"first", 5);
    assert!(
        nanoxide::files::save_to_with_method(&target_str, WritingType::Overwrite) <= 0,
        "只读旧备份在普通模式下应导致失败"
    );

    SET(INSECURE_BACKUP);
    nanoxide::text::inject(b"second", 6);
    assert!(nanoxide::files::save_to_with_method(&target_str, WritingType::Overwrite) > 0);
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "first\n");

    UNSET(INSECURE_BACKUP);
    UNSET(MAKE_BACKUP);
    let _ = std::fs::remove_dir_all(&dir);
}
