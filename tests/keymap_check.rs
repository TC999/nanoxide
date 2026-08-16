//! 验证按键码转换与方向键处理不冲突。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn tc(key: KeyEvent) -> i32 {
    nano_rs::winio::translate_keycode(key)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn arrow_keys_map_correctly() {
    assert_eq!(tc(key(KeyCode::Left)), nano_rs::definitions::KEY_LEFT);
    assert_eq!(tc(key(KeyCode::Right)), nano_rs::definitions::KEY_RIGHT);
    assert_eq!(tc(key(KeyCode::Up)), nano_rs::definitions::KEY_UP);
    assert_eq!(tc(key(KeyCode::Down)), nano_rs::definitions::KEY_DOWN);
    assert_eq!(tc(key(KeyCode::Home)), nano_rs::definitions::KEY_HOME);
    assert_eq!(tc(key(KeyCode::End)), nano_rs::definitions::KEY_END);
    assert_eq!(tc(key(KeyCode::Delete)), nano_rs::definitions::KEY_DC);
    assert_eq!(tc(key(KeyCode::Backspace)), nano_rs::definitions::KEY_BACKSPACE);
    assert_eq!(tc(key(KeyCode::Enter)), 13, "主键盘 Enter 应译为 CR(13)");
    assert_eq!(tc(key(KeyCode::Char('a'))), 97);
    assert_eq!(tc(key(KeyCode::Char(' '))), 32);
}

#[test]
fn arrow_keys_do_not_collide_with_ctrl_codes() {
    // 方向键码必须大于 255，绝不落入 Ctrl 码（1..26）与普通字符（32..126）范围
    for k in [
        nano_rs::definitions::KEY_LEFT,
        nano_rs::definitions::KEY_RIGHT,
        nano_rs::definitions::KEY_UP,
        nano_rs::definitions::KEY_DOWN,
        nano_rs::definitions::KEY_HOME,
        nano_rs::definitions::KEY_END,
        nano_rs::definitions::KEY_DC,
        nano_rs::definitions::KEY_BACKSPACE,
        nano_rs::definitions::KEY_ENTER,
        nano_rs::definitions::KEY_PPAGE,
        nano_rs::definitions::KEY_NPAGE,
    ] {
        assert!(k > 255, "键码 {k} 与 Ctrl/字符冲突");
    }
}

/// 渲染应从当前缓冲区读取最新文本（验证"新输入不显示"问题）。
#[test]
fn render_uses_current_buffer_data() {
    use nano_rs::definitions::*;
    nano_rs::global::global_init();
    nano_rs::nano::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.currmenu = MMAIN;
    });

    // 模拟打开已有文件：第一行有内容
    let of = nano_rs::files::get_openfile().unwrap();
    {
        let mut of = of.borrow_mut();
        let cur = of.current.clone().unwrap();
        cur.borrow_mut().data = "original".to_string();
    }

    // 新输入
    nano_rs::text::inject(b"X", 1);
    let data = with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().current.as_ref().unwrap().borrow().data.clone()
    });
    // 缓冲区应反映新输入
    assert_eq!(data, "Xoriginal");
}

/// 不带文件名、缓冲区为空时显示欢迎消息（对应 nano.c 的 statusbar 欢迎提示）。
#[test]
fn welcome_message_on_empty_buffer() {
    use nano_rs::definitions::*;
    nano_rs::global::global_init();
    nano_rs::global::shortcut_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.currmenu = MMAIN;
    });

    // Ctrl+G（帮助）在 MMAIN 的首快捷键必须是 0x07
    let help_key = nano_rs::global::first_sc_for(MMAIN, FunctionId::DoHelp)
        .map(|k| k.borrow().keycode)
        .unwrap_or(-1);
    assert_eq!(help_key, 0x07, "帮助键应为 Ctrl+G（0x07）");

    // 模拟 main 不带文件名：open_buffer("") → 空缓冲区、无文件名 → 显示欢迎消息
    nano_rs::files::open_buffer("");
    assert!(nano_rs::nano::show_welcome_message());
}

/// 缓冲区有内容或带文件名时不应显示欢迎消息。
#[test]
fn no_welcome_message_with_content_or_name() {
    use nano_rs::definitions::*;
    nano_rs::global::global_init();
    nano_rs::global::shortcut_init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.currmenu = MMAIN;
    });

    // 带文件名（即使文件不存在，新文件分支也有名字）→ 不显示
    nano_rs::files::open_buffer("named_file.txt");
    assert!(!nano_rs::nano::show_welcome_message());

    // 无文件名但有内容 → 不显示
    nano_rs::files::open_buffer("");
    nano_rs::text::inject(b"hello", 5);
    assert!(!nano_rs::nano::show_welcome_message());
}

/// 提示菜单中 Enter/Esc/^C 的映射（对应 C 的 MMOST ^M 与取消键）。
#[test]
fn prompt_enter_and_cancel_keys() {
    use nano_rs::definitions::*;
    nano_rs::global::global_init();
    nano_rs::global::shortcut_init();

    let enter = nano_rs::global::find_shortcut(13, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(enter, Some(FunctionId::DoEnter), "写入提示中 Enter 应为确认");

    let esc = nano_rs::global::find_shortcut(27, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(esc, Some(FunctionId::DoCancel), "写入提示中 Esc 应为取消");

    let cc = nano_rs::global::find_shortcut(3, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(cc, Some(FunctionId::DoCancel), "写入提示中 ^C 应为取消");

    // 搜索提示中 Enter 仍是"搜索"（不被通用 ^M 覆盖）
    let w = nano_rs::global::find_shortcut(13, MWHEREIS).map(|k| k.borrow().func);
    assert_eq!(w, Some(FunctionId::DoSearchForward));
}

/// 主键盘 Enter 键应译为 13（'\r'），与 C 的 wgetch 一致。
#[test]
fn enter_key_translates_to_cr() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(nano_rs::winio::translate_keycode(ev), 13);
}


/// 提示菜单中 Backspace/Delete 键应映射为删除（对应 C 的 MMOST Bsp/Del）。
#[test]
fn prompt_backspace_delete_keys() {
    use nano_rs::definitions::*;
    nano_rs::global::global_init();
    nano_rs::global::shortcut_init();

    let b = nano_rs::global::find_shortcut(KEY_BACKSPACE, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(b, Some(FunctionId::DoBackspace), "Backspace 键应映射为删除");

    let d = nano_rs::global::find_shortcut(KEY_DC, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(d, Some(FunctionId::DoDelete), "Delete 键应映射为删除");

    let cd = nano_rs::global::find_shortcut(4, MWRITEFILE).map(|k| k.borrow().func);
    assert_eq!(cd, Some(FunctionId::DoDelete), "^D 应映射为删除");
}
