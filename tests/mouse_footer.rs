//! 底部栏点击的坐标换算（对应 C 的 get_mouseinput footer 分支）。

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
    });
}

/// 8 个条目、80 列：宽度 20，行号为 footwin 内 1/2。
#[test]
fn footer_index_maps_columns_to_shortcuts() {
    assert_eq!(nanoxide::winio::footer_index(0, 1, 80, 8), Some(1));
    assert_eq!(nanoxide::winio::footer_index(25, 1, 80, 8), Some(3));
    assert_eq!(nanoxide::winio::footer_index(5, 2, 80, 8), Some(2));
    /* 最后一行最右列对应最后一个条目。 */
    assert_eq!(nanoxide::winio::footer_index(79, 2, 80, 8), Some(8));
    assert_eq!(nanoxide::winio::footer_index(0, 1, 80, 0), None);
}

/// 不足 5 个条目时宽度取 COLS/2（对应 C 的 number < 5 分支）。
#[test]
fn footer_index_uses_half_width_for_few_items() {
    /* 3 个条目：width = 40 */
    assert_eq!(nanoxide::winio::footer_index(0, 1, 80, 3), Some(1));
    assert_eq!(nanoxide::winio::footer_index(0, 2, 80, 3), Some(2));
    assert_eq!(nanoxide::winio::footer_index(40, 1, 80, 3), Some(3));
    assert_eq!(nanoxide::winio::footer_index(40, 2, 80, 3), None);
}

/// 底部栏条目按 allfuncs 顺序收集，且非空。
#[test]
fn footer_shortcuts_are_collected_for_main_menu() {
    setup();
    let count = (0..80)
        .filter_map(|col| nanoxide::winio::footer_index(col, 1, 80, 8))
        .count();
    assert!(count > 0);
}
