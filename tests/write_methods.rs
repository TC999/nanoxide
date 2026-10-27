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
