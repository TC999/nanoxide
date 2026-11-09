//! M-6 复制当前行与标记区域（对应 cut.c 的 `copy_text` /
//! `copy_marked_region`）。测试通过真实 Alt+6 键码进入主菜单分发，
//! 不只直接调用复制函数。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.we_are_running = true;
    });
}

fn press_alt_6() {
    let key = KeyEvent::new_with_kind(
        KeyCode::Char('6'),
        KeyModifiers::ALT,
        KeyEventKind::Press,
    );
    let keycode = nanoxide::winio::translate_keycode(key);
    assert!(nanoxide::winio::handle_input_key(keycode), "Alt+6 应被处理");
}

fn buffer_lines() -> Vec<String> {
    let head = with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().filetop.clone().unwrap()
    });
    lines_from(head)
}

fn cutbuffer_lines() -> Vec<String> {
    let head = with_global(|g| g.cutbuffer.clone()).expect("复制后 cutbuffer 不应为空");
    lines_from(head)
}

fn lines_from(head: LineRef) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = Some(head);
    while let Some(current) = line {
        let next = {
            let current = current.borrow();
            lines.push(current.data.clone());
            current.next.clone()
        };
        line = next;
    }
    lines
}

#[test]
fn alt_6_copies_the_current_line() {
    setup();
    nanoxide::text::inject(b"hello", 5);

    press_alt_6();

    assert_eq!(cutbuffer_lines(), ["hello", ""], "整行复制应包含换行");
    assert_eq!(
        buffer_lines(),
        ["hello", ""],
        "复制不得修改原缓冲区（末尾空行是 nano 的魔法行）"
    );
}

#[test]
fn alt_6_copies_a_marked_region_within_one_line() {
    setup();
    nanoxide::text::inject(b"hello world", 11);
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let line = of.borrow().current.clone().unwrap();
        let mut of = of.borrow_mut();
        of.mark = Some(line);
        of.mark_x = 6;
        of.current_x = 11;
    });

    press_alt_6();

    assert_eq!(cutbuffer_lines(), ["world"]);
    assert_eq!(
        buffer_lines(),
        ["hello world", ""],
        "复制后原文和魔法行必须完整保留"
    );
    assert!(
        with_global(|g| g.openfile.as_ref().unwrap().borrow().mark.is_none()),
        "复制选区后应取消标记"
    );
}

#[test]
fn alt_6_copied_marked_text_can_be_pasted_back() {
    setup();
    nanoxide::text::inject(b"hello world", 11);
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let line = of.borrow().current.clone().unwrap();
        let mut of = of.borrow_mut();
        of.mark = Some(line);
        of.mark_x = 6;
        of.current_x = 11;
    });

    press_alt_6();

    /* 复制不移动光标（仍在本行 x=11）；回到行首把复制的文本贴回去。 */
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap();
        let mut of = of.borrow_mut();
        of.current_x = 0;
        of.placewewant = 0;
    });
    nanoxide::cut::paste_text();

    assert_eq!(
        buffer_lines(),
        ["worldhello world", ""],
        "M-6 复制的标记区域应能原样粘贴回来（不得夹带 NUL，也不得丢失内容）"
    );
}

#[test]
fn alt_6_copies_all_lines_of_a_marked_region() {
    setup();
    nanoxide::text::inject(b"one", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"two", 3);
    nanoxide::text::do_enter();
    nanoxide::text::inject(b"three", 5);
    with_global_mut(|g| {
        let of = g.openfile.clone().unwrap();
        let first = of.borrow().filetop.clone().unwrap();
        let third = first.borrow().next.as_ref().unwrap().borrow().next.clone().unwrap();
        let mut of = of.borrow_mut();
        of.mark = Some(first);
        of.mark_x = 1;
        of.current = Some(third);
        of.current_x = 2;
    });

    press_alt_6();

    assert_eq!(
        cutbuffer_lines(),
        ["ne", "two", "th"],
        "跨行选区的首行、中间行和末行都要复制"
    );
    assert_eq!(
        buffer_lines(),
        ["one", "two", "three", ""],
        "复制跨行选区不得删改任何原文（末尾空行是魔法行）"
    );
}
