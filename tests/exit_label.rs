//! 多缓冲区时底部栏 ^X 的 "Exit"/"Close" 标签切换
//! （对应 files.c 的 `install_buffer` / `close_buffer` 对 `exitfunc->tag`
//! 的修改）。
//!
//! C 版 exitfunc 同时覆盖主菜单与文件浏览器，因此两个菜单的 ^X 都要切换；
//! 关闭到只剩一个缓冲区时改回 "Exit"。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.openfile = None;
        g.more_than_one = false;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
    nanoxide::files::make_new_buffer();
}

/// 取指定菜单里 ^X（DoExit）函数项的标签。
fn exit_tag(menu: i32) -> String {
    let mut result = String::new();
    let mut func = with_global(|g| g.allfuncs.clone());
    while let Some(node) = func {
        let (tag, is_target, next) = {
            let r = node.borrow();
            (
                r.tag.clone(),
                r.func == FunctionId::DoExit && (r.menus & menu) != 0,
                r.next.clone(),
            )
        };
        if is_target {
            result = tag;
            break;
        }
        func = next;
    }
    result
}

#[test]
fn exit_label_switches_to_close_with_multiple_buffers() {
    setup();
    let exit = nanoxide::t!("key-exit");
    let close = nanoxide::t!("key-close");

    assert_eq!(exit_tag(MMAIN), exit);

    nanoxide::files::make_new_buffer();
    assert_eq!(exit_tag(MMAIN), close);

    nanoxide::files::close_buffer();
    assert_eq!(exit_tag(MMAIN), exit);
}

#[test]
fn browser_exit_label_also_switches() {
    setup();
    let exit = nanoxide::t!("key-exit");
    let close = nanoxide::t!("key-close");

    assert_eq!(exit_tag(MBROWSER), exit);
    nanoxide::files::make_new_buffer();
    assert_eq!(exit_tag(MBROWSER), close);
    nanoxide::files::close_buffer();
    assert_eq!(exit_tag(MBROWSER), exit);
}
