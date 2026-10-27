//! `-t/--saveonexit`：有文件名时退出直接保存，不再询问。
//!
//! 对应 nano.c 的 do_exit：
//!   else if (ISSET(SAVE_ON_EXIT) && openfile->filename[0]) choice = YES;

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.we_are_running = true;
    });
}

fn current_name() -> String {
    with_global(|g| {
        g.openfile
            .as_ref()
            .unwrap()
            .borrow()
            .filename
            .clone()
            .unwrap_or_default()
    })
}

#[test]
fn save_onexit_writes_named_buffer_without_prompt() {
    setup();
    let path = std::env::temp_dir().join(format!(
        "nanoxide_saveonexit_{}.txt",
        std::process::id()
    ));
    let name = path.to_str().unwrap().to_string();
    with_global_mut(|g| {
        g.openfile.as_ref().unwrap().borrow_mut().filename = Some(name.clone());
    });
    nanoxide::text::inject(b"hello", 5);
    assert!(nanoxide::files::is_modified(), "注入后应处于已修改状态");

    SET(SAVE_ON_EXIT);
    nanoxide::text::do_exit();

    assert!(
        !with_global(|g| g.we_are_running),
        "-t 且文件有名字时应直接保存并退出"
    );
    let content = std::fs::read_to_string(&path).expect("文件应已写入");
    assert_eq!(content, "hello\n");
    assert!(!nanoxide::files::is_modified(), "保存后应清除修改标记");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn view_mode_exits_without_saving() {
    setup();
    let path = std::env::temp_dir().join(format!(
        "nanoxide_viewmode_{}.txt",
        std::process::id()
    ));
    let name = path.to_str().unwrap().to_string();
    with_global_mut(|g| {
        g.openfile.as_ref().unwrap().borrow_mut().filename = Some(name.clone());
    });
    nanoxide::text::inject(b"hello", 5);
    assert_eq!(current_name(), name);

    /* 只读模式对应 C 版 `!openfile->modified || ISSET(VIEW_MODE)` 的 NO 分支。 */
    SET(VIEW_MODE);
    nanoxide::text::do_exit();

    assert!(!with_global(|g| g.we_are_running));
    assert!(!path.exists(), "只读模式不应写文件");
}
