//! 编辑区分栏补全列表的布局（对应 files.c `input_tab` 末尾的
//! `lastrow` / `longest_name` / `ncols` / `nrows` / `row` 计算）。
//!
//! C 版列间空两格、最后一列后空一格：`ncols = (COLS + 1) / (longest + 2)`；
//! 行数不足时从窗口底部往上排。

use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.openfile = None;
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.editwincols = 80;
        g.margin = 0;
    });
    nanoxide::files::make_new_buffer();
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn short_names_use_multiple_columns() {
    setup();
    let matches = names(&["alpha", "beta", "gamma", "delta", "epsilon"]);

    let (lastrow, longest, ncols, nrows, startrow) =
        nanoxide::winio::completion_list_layout(80, 24, 20, false, &matches);

    assert_eq!(lastrow, 19, "editwinrows - 1");
    assert_eq!(longest, 7, "最长候选 epsilon 的宽度");
    assert_eq!(ncols, 9, "(80 + 1) / (7 + 2)");
    assert_eq!(nrows, 1, "5 个候选排成 1 行");
    assert_eq!(startrow, 18, "从底部往上排");
}

#[test]
fn long_name_is_clipped_and_forces_one_column() {
    setup();
    let long = "x".repeat(100);
    let matches = vec![long, "short".to_string()];

    let (_, longest, ncols, nrows, startrow) =
        nanoxide::winio::completion_list_layout(80, 24, 20, false, &matches);

    assert_eq!(longest, 79, "最长名钳到 COLS - 1");
    assert_eq!(ncols, 1, "(80 + 1) / (79 + 2)");
    assert_eq!(nrows, 2);
    assert_eq!(startrow, 17);
}

#[test]
fn zero_mode_reserves_the_last_row() {
    setup();
    let matches = names(&["a", "b"]);

    let (lastrow, _, ncols, _, startrow) =
        nanoxide::winio::completion_list_layout(80, 24, 20, true, &matches);

    assert_eq!(lastrow, 18, "ZERO 且 LINES > 1 时让出一行");
    assert_eq!(ncols, 27, "(80 + 1) / (1 + 2)");
    assert_eq!(startrow, 17);
}

#[test]
fn draw_completion_list_does_not_panic() {
    setup();
    nanoxide::winio::draw_completion_list(&names(&["one", "two", "three"]));
}
