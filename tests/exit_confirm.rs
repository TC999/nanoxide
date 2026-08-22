//! 集成测试：退出确认与 "Modified" 标记。
//!
//! 说明：`do_exit` 在缓冲区已修改时会进入 `ask_user` 交互循环（阻塞读取终端
//! 按键），无法在无终端的环境中直接测试；这里覆盖其"未修改直接退出"路径，
//! 以及"输入文本 → 标记已修改 → 保存后清除"的标记生命周期。

use nanoxide::definitions::*;
use nanoxide::global::global_init;
use nanoxide::files::make_new_buffer;

fn setup() {
    global_init();
    make_new_buffer();
    with_global_mut(|g| {
        g.editwincols = 80;
        g.tabsize = 8;
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

/// 未修改时 do_exit 应直接退出（设置 we_are_running = false）。
#[test]
fn do_exit_unmodified_quits_immediately() {
    setup();
    with_global_mut(|g| g.we_are_running = true);
    assert!(!nanoxide::files::is_modified(), "新缓冲区未修改");
    nanoxide::text::do_exit();
    let running = with_global(|g| g.we_are_running);
    assert!(!running, "未修改时 do_exit 应直接退出");
}

/// 输入任意文本后缓冲区应标记为已修改（标题栏据此显示 "Modified"）。
#[test]
fn typing_marks_buffer_modified() {
    setup();
    assert!(!nanoxide::files::is_modified());
    nanoxide::text::inject(b"abc", 3);
    assert!(nanoxide::files::is_modified(), "输入文本后应标记为已修改");
}

/// 删除操作同样应标记为已修改。
#[test]
fn deletion_marks_buffer_modified() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    nanoxide::cut::do_backspace();
    assert!(nanoxide::files::is_modified(), "删除字符后应标记为已修改");
}

/// 没有打开任何缓冲区时 is_modified 应返回 false（do_exit 不会误判）。
#[test]
fn is_modified_false_without_openfile() {
    global_init();
    assert!(!nanoxide::files::is_modified());
}

/// 保存成功后应清除修改标记（do_exit 据此判定保存成功并退出）。
#[test]
fn saving_clears_modified_flag() {
    setup();
    nanoxide::text::inject(b"hello world", 11);
    let path = std::env::temp_dir().join(format!("nanoxide_exit_test_{}.txt", std::process::id()));
    let ps = path.to_str().unwrap().to_string();
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().filename = Some(ps);
    });
    let n = nanoxide::files::write_it_out(false, true);
    assert!(n > 0, "保存应返回写入字节数");
    assert!(!nanoxide::files::is_modified(), "保存后应清除修改标记");
    let _ = std::fs::remove_file(&path);
}
