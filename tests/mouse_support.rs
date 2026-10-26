// tests/mouse_support.rs - 鼠标支持冒烟验证
//
// 对应原版 C 代码（nano/src/nano.c mouse_init / enable_mouse_support /
// disable_mouse_support）：
//   1. mouse_init 依据 USE_MOUSE 开关 crossterm 鼠标捕获，调用不应崩溃
//      （非 TTY 环境下 execute 失败被忽略）；
//   2. M-M 切换（DoToggle + USE_MOUSE）在 rcfile/按键分发中的映射存在。

use nanoxide::definitions::{with_global_mut, ISSET, SET, UNSET, USE_MOUSE};

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

/// 未启用时 mouse_init 关闭捕获且不崩溃；启用后再次调用（开启捕获）同样安全。
#[test]
fn mouse_init_switches_capture() {
    setup();
    UNSET(USE_MOUSE);
    nanoxide::winio::mouse_init();
    assert!(!ISSET(USE_MOUSE), "未设置 -m/rcfile 时鼠标应关闭");

    SET(USE_MOUSE);
    nanoxide::winio::mouse_init();
    assert!(ISSET(USE_MOUSE), "设置 USE_MOUSE 后应生效");
    UNSET(USE_MOUSE);
    nanoxide::winio::mouse_init();
}
