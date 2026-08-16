//! 端到端验证：模拟用户按键序列（字符、方向键、删除、回车）。

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

fn current_text() -> String {
    with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().data.clone()
    })
}

fn current_x() -> usize {
    with_global(|g| g.openfile.as_ref().unwrap().borrow().current_x)
}

/// 输入字符序列，光标应随之移动。
#[test]
fn typing_sequence_handles_keys() {
    setup();
    for ch in ['h', 'e', 'l', 'l', 'o'] {
        nano_rs::winio::handle_input_key(ch as i32);
    }
    assert_eq!(current_text(), "hello");
    assert_eq!(current_x(), 5);
}

/// 右箭头只移动光标，不触发任何提示。
#[test]
fn right_arrow_moves_cursor_only() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    // 光标回行首
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().current_x = 0;
    });
    // 按右箭头
    let handled = nano_rs::winio::handle_input_key(KEY_RIGHT);
    assert!(handled, "右箭头应被处理");
    assert_eq!(current_x(), 1, "右箭头应右移光标");
    assert_eq!(current_text(), "abc", "右箭头不应改变文本");
}

/// 下箭头在单行文件中移到魔法行（空行），不应崩溃。
#[test]
fn down_arrow_does_not_crash() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    let handled = nano_rs::winio::handle_input_key(KEY_DOWN);
    assert!(handled);
    // C 语义：最后一行按 Down 移到末尾魔法行（空行）
    assert_eq!(current_text(), "");
}

/// Delete 删除光标处字符。
#[test]
fn delete_removes_char_at_cursor() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().current_x = 1;
    });
    nano_rs::winio::handle_input_key(KEY_DC);
    assert_eq!(current_text(), "ac");
}

/// Backspace 删除光标前字符。
#[test]
fn backspace_removes_char_before_cursor() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    nano_rs::winio::handle_input_key(KEY_BACKSPACE);
    assert_eq!(current_text(), "ab");
}

/// 回车换行。
#[test]
fn enter_creates_new_line() {
    setup();
    nano_rs::text::inject(b"abc", 3);
    nano_rs::winio::handle_input_key(KEY_ENTER);
    assert_eq!(current_text(), "");
}

/// 方向键码处理：左/上/右/下都不触发搜索/替换提示。
#[test]
fn arrows_do_not_trigger_search() {
    setup();
    nano_rs::text::inject(b"line one", 8);
    for k in [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN] {
        nano_rs::winio::handle_input_key(k);
    }
    // 不应进入搜索菜单
    let menu = with_global(|g| g.currmenu);
    assert_eq!(menu, MMAIN, "方向键不应改变菜单");
}
