// tests/lockfile_lifecycle.rs - 锁文件（.swp）生命周期验证
//
// 对应原版 C 代码（nano/src/files.c do_lockfile / write_lockfile /
// delete_lockfile、nano/src/nano.c close_and_go / die）的锁文件行为：
//   1. 打开已存在文件（启用 LOCKING）→ 在文件同目录生成 `.<文件名>.swp`；
//   2. do_exit（未修改直接退出）→ 删除当前缓冲区的锁文件并停止主循环
//      （对应 C 版 close_and_go 开头 `delete_lockfile(openfile->lock_filename)`）；
//   3. delete_all_lockfiles → 删除所有缓冲区的锁文件并清空记录
//      （对应 C 版 die() 中遍历删除）。

use nanoxide::definitions::{with_global, with_global_mut, LOCKING, SET, UNSET};
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
        g.openfile = None;
        g.statusbar_msg.clear();
        g.statusbar_centered = false;
        g.lastmessage = nanoxide::definitions::MessageType::Vacuum;
        g.we_are_running = true;
    });
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nanoxide_swp_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 单一测试覆盖全部验收点，避免多个测试并发争用全局 LOCKING 标志。
#[test]
fn lockfile_created_on_open_and_removed_on_exit() {
    setup();
    SET(LOCKING);

    let dir = temp_dir();
    let path = dir.join("test.txt");
    fs::write(&path, "hello\n").unwrap();
    let lockpath = dir.join(".test.txt.swp");

    // 1. 打开已存在文件 → 生成锁文件。
    let result = nanoxide::files::open_buffer(path.to_str().unwrap());
    assert!(matches!(result, nanoxide::files::OpenBufferResult::FileLoaded));
    assert!(lockpath.exists(), "打开已存在文件后应生成 .swp 锁文件");

    // 2. 未修改时 do_exit → 删除当前缓冲区的锁文件并停止主循环。
    nanoxide::text::do_exit();
    assert!(!lockpath.exists(), "退出后 .swp 锁文件应被删除");
    assert!(!with_global(|g| g.we_are_running), "do_exit 应停止主循环");

    // 3. 重新打开（替换当前缓冲区）→ 锁文件重新创建；
    //    delete_all_lockfiles → 删除所有缓冲区的锁文件并清空记录。
    let result = nanoxide::files::open_buffer(path.to_str().unwrap());
    assert!(matches!(result, nanoxide::files::OpenBufferResult::FileLoaded));
    assert!(lockpath.exists(), "再次打开后应重新生成 .swp 锁文件");

    nanoxide::files::delete_all_lockfiles();
    assert!(!lockpath.exists(), "delete_all_lockfiles 应删除所有缓冲区的锁文件");
    with_global(|g| {
        let of = g.openfile.as_ref().expect("当前应有打开的缓冲区");
        assert!(
            of.borrow().lock_filename.is_none(),
            "锁文件删除后缓冲区记录中的 lock_filename 应被清空"
        );
    });

    UNSET(LOCKING);
    fs::remove_dir_all(&dir).ok();
}
