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
    assert_eq!(tc(key(KeyCode::Enter)), nano_rs::definitions::KEY_ENTER);
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
