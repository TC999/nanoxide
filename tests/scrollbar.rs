//! 软换行下滚动条的覆盖行数（对应 winio.c 的 `draw_scrollbar` 中
//! `coveredlines` 循环）。
//!
//! C 版在软换行时从 edittop 起累计每行的额外块数，直到"行号 + 额外块"
//! 超出窗口或到达文件末尾；非软换行时 coveredlines 就是 editwinrows。

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
        g.tabsize = 8;
    });
    nanoxide::files::make_new_buffer();
    SET(SOFTWRAP);
}

#[test]
fn long_line_counts_extra_chunks() {
    setup();
    let long = "x".repeat(200);
    nanoxide::text::inject(long.as_bytes(), 200);

    /* 200 列 / 80 = 3 块，edittop 的额外块数为 2；循环再走到魔法行，
     * coveredlines = 2（与 C 的 lineno + extras 判断一致）。 */
    assert_eq!(nanoxide::winio::scrollbar_covered_lines(20), 2);
}

#[test]
fn short_lines_stop_at_window_height() {
    setup();
    for _ in 0..5 {
        nanoxide::text::inject(b"a", 1);
        nanoxide::text::do_enter();
    }

    /* 窗口足够大时覆盖到文件末尾（含末尾的行）。 */
    let total = with_global(|g| {
        g.openfile
            .as_ref()
            .and_then(|of| of.borrow().filebot.as_ref().map(|b| b.borrow().lineno))
            .unwrap_or(1)
    });
    assert_eq!(nanoxide::winio::scrollbar_covered_lines(20), total);

    /* 窗口只够 3 行时，到累计达到窗口高度的那一行为止。 */
    assert_eq!(nanoxide::winio::scrollbar_covered_lines(3), 3);
}
