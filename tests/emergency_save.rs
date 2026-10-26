// tests/emergency_save.rs - 崩溃/终止时的紧急保存验证
//
// 对应原版 C 代码（nano/src/nano.c die / emergency_save、
// nano/src/files.c get_next_filename）：
//   1. 已修改的有名缓冲区 → 写入 `<名字>.save`（不改写原文件）；
//   2. 未命名缓冲区 → 写入 `nano.<pid>.save`；
//   3. 目标名冲突时唯一化（`<名字>.save.1`、`.2`…）；
//   4. 紧急保存不改变缓冲区文件名、不清除修改标记。

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
        "nanoxide_emerg_{}_{}",
        std::process::id(),
        label
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 修改当前缓冲区的第一行（模拟编辑后未保存）。
fn type_into_current(extra: &str) {
    let of = with_global(|g| g.openfile.as_ref().unwrap().clone());
    let line = of.borrow().current.clone().unwrap();
    line.borrow_mut().data.push_str(extra);
    nanoxide::files::set_modified();
}

/// 已修改的有名缓冲区 → `<名字>.save`；不改原文件、不改 buffer 名。
#[test]
fn named_buffer_goes_to_dot_save() {
    setup();
    let dir = temp_dir("named");
    let path = dir.join("work.txt");
    fs::write(&path, "original\n").unwrap();
    assert!(matches!(
        nanoxide::files::open_buffer(path.to_str().unwrap()),
        nanoxide::files::OpenBufferResult::FileLoaded
    ));
    type_into_current("+edited");

    nanoxide::files::emergency_save_all();

    /* 原文件保持原样，.save 含修改后的内容。 */
    assert_eq!(fs::read_to_string(&path).unwrap(), "original\n");
    let save = dir.join("work.txt.save");
    assert!(save.exists(), "应生成 work.txt.save");
    assert_eq!(fs::read_to_string(&save).unwrap(), "original+edited\n");

    /* 缓冲区文件名与修改标记不变。 */
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        assert!(of.filename.as_deref().unwrap_or("").ends_with("work.txt"));
        assert!(of.modified, "紧急保存不应清除修改标记");
    });
    let _ = fs::remove_dir_all(&dir);
}

/// 已存在 .save 时唯一化为 `.save.1`。
#[test]
fn colliding_save_name_is_suffixed() {
    setup();
    let dir = temp_dir("uniq");
    let path = dir.join("doc.txt");
    fs::write(&path, "v1\n").unwrap();
    fs::write(dir.join("doc.txt.save"), "stale\n").unwrap();
    assert!(matches!(
        nanoxide::files::open_buffer(path.to_str().unwrap()),
        nanoxide::files::OpenBufferResult::FileLoaded
    ));
    type_into_current("+x");

    nanoxide::files::emergency_save_all();

    assert_eq!(fs::read_to_string(dir.join("doc.txt.save")).unwrap(), "stale\n");
    let save1 = dir.join("doc.txt.save.1");
    assert!(save1.exists(), "应生成 doc.txt.save.1");
    assert_eq!(fs::read_to_string(&save1).unwrap(), "v1+x\n");
    let _ = fs::remove_dir_all(&dir);
}

/// 未命名缓冲区 → `nano.<pid>.save`。
#[test]
fn nameless_buffer_goes_to_nano_pid_save() {
    setup();
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.openfile.as_ref().unwrap().borrow_mut().filename = None;
    });
    type_into_current("unsaved words");

    nanoxide::files::emergency_save_all();

    /* 目标名 `nano.<pid>.save` 是相对路径：落在进程工作目录（与 C 一致）。 */
    let cwd = std::env::current_dir().unwrap();
    let save = cwd.join(format!("nano.{}.save", std::process::id()));
    assert!(save.exists(), "应生成 nano.<pid>.save: {}", save.display());
    assert_eq!(
        fs::read_to_string(&save).unwrap(),
        "unsaved words\n",
        "未命名缓冲区内容应写入 nano.<pid>.save"
    );
    let _ = fs::remove_file(&save);
}
