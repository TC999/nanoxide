//! `execute_by_id` 兜底：rc bind 的占位函数不再静默吞键。
//!
//! C 版里 dos_format / append_it / flip_* / *_void 等函数本身是空函数，
//! 行为在写文件提示循环中；Rust 尚未移植提示循环，但至少应给出提示。

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
        g.editwinrows = 20;
    });
}

#[test]
fn unimplemented_bound_function_reports_instead_of_silence() {
    setup();
    with_global_mut(|g| {
        g.bound_keys.push(BoundKey {
            keystr: "^Y".to_string(),
            keycode: 25,
            menus: MMAIN,
            func: FunctionId::CaseSensVoid,
            toggle: 0,
            ordinal: 0,
            expansion: None,
        });
    });

    assert!(nanoxide::winio::handle_input_key(25));
    let msg = with_global(|g| g.statusbar_msg.clone());
    assert!(
        msg.contains("CaseSensVoid"),
        "未实现的绑定应显示函数名，实际消息：{msg:?}"
    );
    assert!(
        !msg.is_empty(),
        "不应静默消费按键（状态栏应出现提示）"
    );
}

#[test]
fn implemented_bound_function_still_runs() {
    setup();
    nanoxide::text::inject(b"abc", 3);
    with_global_mut(|g| {
        g.bound_keys.push(BoundKey {
            keystr: "^Y".to_string(),
            keycode: 25,
            menus: MMAIN,
            func: FunctionId::DoHome,
            toggle: 0,
            ordinal: 0,
            expansion: None,
        });
    });

    assert!(nanoxide::winio::handle_input_key(25));
    let x = with_global(|g| g.openfile.as_ref().unwrap().borrow().current_x);
    assert_eq!(x, 0, "已实现的绑定应执行 DoHome");
    assert!(
        !with_global(|g| g.statusbar_msg.contains("not implemented")),
        "已实现的函数不应出现未实现提示"
    );
}
