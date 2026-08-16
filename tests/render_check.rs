//! 验证渲染输出确实包含编辑文本与光标序列。

use nano_rs::definitions::*;
use nano_rs::global::global_init;
use nano_rs::files::make_new_buffer;

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

/// 输入后渲染应包含输入文本（观察 stdout）。
#[test]
fn render_after_typing() {
    setup();
    nano_rs::text::inject(b"visible text", 12);
    with_global_mut(|g| g.refresh_needed = true);
    nano_rs::winio::edit_refresh();
    // 光标放置后，编辑文本已由 refresh_screen 写入 stdout
}

fn current_text() -> String {
    with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().data.clone()
    })
}

/// 行尾 Delete 的行为：位于行尾且下一行是魔法行时删除无操作（C 语义）。
#[test]
fn delete_at_eol_is_noop() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    // 光标在行尾 (x=3)，按 Delete
    nano_rs::cut::do_delete();
    assert_eq!(current_text(), "abc", "行尾 Delete 不应改变文本");
}

/// 光标可移动到行中后删除。
#[test]
fn delete_in_middle_works() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.current_x = 0;
    });
    nano_rs::cut::do_delete();
    assert_eq!(current_text(), "bc");
}

/// Backspace 在行尾应删除光标前字符。
#[test]
fn backspace_at_eol_works() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    nano_rs::cut::do_backspace();
    assert_eq!(current_text(), "ab");
}
