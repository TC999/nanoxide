//! `-U/--quickblank`：状态栏消息在按键后清除。
//!
//! 对应 winio.c 的 countdown / blank_it_when_expired：
//!   countdown = (ISSET(QUICK_BLANK) ? 1 : 20);

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
    });
}

fn message() -> String {
    with_global(|g| g.statusbar_msg.clone())
}

fn countdown() -> i32 {
    with_global(|g| g.statusbar_countdown)
}

#[test]
fn quickblank_clears_after_one_key() {
    setup();
    SET(QUICK_BLANK);
    nanoxide::winio::statusbar("saved");
    assert_eq!(message(), "saved");
    assert_eq!(countdown(), 1, "--quickblank 的倒计时应为 1");

    nanoxide::winio::blank_it_when_expired();
    assert_eq!(message(), "", "一次按键后状态栏应清空");
    assert_eq!(countdown(), 0);

    /* 已清空后继续按键不应有额外影响。 */
    nanoxide::winio::blank_it_when_expired();
    assert_eq!(message(), "");
}

#[test]
fn normal_message_lasts_twenty_keys() {
    setup();
    nanoxide::winio::statusbar("saved");
    assert_eq!(countdown(), 20, "默认倒计时应为 20");

    for _ in 0..19 {
        nanoxide::winio::blank_it_when_expired();
    }
    assert_eq!(message(), "saved", "19 次按键后消息仍在");
    nanoxide::winio::blank_it_when_expired();
    assert_eq!(message(), "", "第 20 次按键后消息应清除");
}

#[test]
fn new_message_resets_the_countdown() {
    setup();
    SET(QUICK_BLANK);
    nanoxide::winio::statusbar("first");
    nanoxide::winio::blank_it_when_expired();
    assert_eq!(message(), "");

    nanoxide::winio::statusbar("second");
    assert_eq!(countdown(), 1);
    assert_eq!(message(), "second");
}
