//! 集成测试：模拟编辑器核心输入路径，验证无 RefCell 双重借用崩溃。

use nanoxide::definitions::*;
use nanoxide::global::global_init;
use nanoxide::files::make_new_buffer;

fn setup() {
    global_init();
    make_new_buffer();
    // 初始化常用状态
    with_global_mut(|g| {
        g.editwincols = 80;
        g.tabsize = 8;
        g.currmenu = MMAIN;
        g.united_sidescroll = false;
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

/// 输入普通字符不应崩溃。
#[test]
fn typing_characters_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    assert_eq!(current_text(), "hello");
    assert_eq!(current_x(), 5);
}

/// 连续输入多个字符（模拟连续输入）。
#[test]
fn typing_sequence_works() {
    setup();
    nanoxide::text::inject(b"h", 1);
    nanoxide::text::inject(b"i", 1);
    nanoxide::text::inject(b"!", 1);
    assert_eq!(current_text(), "hi!");
}

/// 按回车不应崩溃。
#[test]
fn pressing_enter_works() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    nanoxide::text::do_enter();
    assert_eq!(current_text(), "");
}

/// 按退格不应崩溃。
#[test]
fn backspace_works() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    nanoxide::cut::do_backspace();
    assert_eq!(current_text(), "ab");
}

/// 按删除不应崩溃。
#[test]
fn delete_works() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.current_x = 1;
    });
    nanoxide::cut::do_delete();
    assert_eq!(current_text(), "ac");
}

/// 制表符不应崩溃。
#[test]
fn tab_works() {
    setup();
    nanoxide::text::do_tab();
    assert_eq!(current_text(), "\t");
}

/// 剪切不应崩溃。
#[test]
fn cut_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::cut::cut_text();
    assert_eq!(current_text(), "");
}

/// 撤销不应崩溃。
#[test]
fn undo_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::text::do_undo();
    assert_eq!(current_text(), "");
}

/// 粘贴不应崩溃。
#[test]
fn paste_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::cut::cut_text();
    nanoxide::cut::paste_text();
    assert_eq!(current_text(), "hello");
}

/// 移动光标不应崩溃。
#[test]
fn movement_works() {
    setup();
    nanoxide::text::inject(b"hello world", 11);
    nanoxide::movement::do_home();
    nanoxide::movement::do_right();
    nanoxide::movement::do_down();
    nanoxide::movement::do_up();
    nanoxide::movement::do_end();
}

/// 重做不应崩溃。
#[test]
fn redo_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::text::do_undo();
    nanoxide::text::do_redo();
    assert_eq!(current_text(), "hello");
}

/// 缩进/取消缩进不应崩溃。
#[test]
fn indent_unindent_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    nanoxide::text::do_indent();
    assert!(current_text().starts_with('\t'));
    nanoxide::text::do_unindent();
    assert_eq!(current_text(), "hello");
}

/// 注释/取消注释不应崩溃。
#[test]
fn comment_works() {
    setup();
    nanoxide::text::inject(b"hello", 5);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.syntax = Some(std::rc::Rc::new(std::cell::RefCell::new(SyntaxType::new())));
        if let Some(s) = &of.syntax {
            s.borrow_mut().comment = Some("#".to_string());
        }
    });
    nanoxide::text::do_comment();
    assert_eq!(current_text(), "#hello");
}

/// 滚动不应崩溃。
#[test]
fn scrolling_works() {
    setup();
    nanoxide::text::inject(b"line one", 8);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"line two", 8);
    nanoxide::movement::do_scroll_up();
    nanoxide::movement::do_scroll_down();
    nanoxide::movement::do_page_up();
    nanoxide::movement::do_page_down();
}

/// 多行编辑后撤销不应崩溃。
#[test]
fn multi_line_undo_works() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"def", 3);
    nanoxide::text::do_undo();
    nanoxide::text::do_undo();
    nanoxide::text::do_redo();
    nanoxide::text::do_redo();
}

/// 搜索不应崩溃。
#[test]
fn search_works() {
    setup();
    nanoxide::text::inject(b"hello world hello", 17);
    with_global_mut(|g| g.last_search = Some("hello".to_string()));
    nanoxide::search::do_research();
    nanoxide::search::do_findnext();
    nanoxide::search::do_findprevious();
}

/// 单词移动不应崩溃。
#[test]
fn word_movement_works() {
    setup();
    nanoxide::text::inject(b"hello world foo", 15);
    nanoxide::movement::do_prev_word();
    nanoxide::movement::do_next_word(false);
    nanoxide::movement::to_prev_word();
    nanoxide::movement::to_next_word();
}

/// 段落与块移动不应崩溃。
#[test]
fn para_block_movement_works() {
    setup();
    nanoxide::text::inject(b"first paragraph", 15);
    nanoxide::text::do_enter();
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"second", 6);
    nanoxide::movement::to_para_begin();
    nanoxide::movement::to_para_end();
    nanoxide::movement::to_prev_block();
    nanoxide::movement::to_next_block();
}

/// 制表符转空格不应崩溃。
#[test]
fn tabs_to_spaces_works() {
    setup();
    nanoxide::definitions::SET(nanoxide::definitions::TABS_TO_SPACES);
    nanoxide::text::do_tab();
    assert_eq!(current_text(), "        ");
}
