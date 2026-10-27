//! `-P/--positionlog`：记录并恢复每个文件的光标位置。
//!
//! 对应 history.c 的 update_positions_register / load_positions_register /
//! restore_cursor_position_if_any。

use nanoxide::definitions::*;

fn setup(dir: &std::path::Path) -> String {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    let reg = dir.join("positions");
    let regstr = reg.to_str().unwrap().to_string();
    with_global_mut(|g| {
        g.statedir = Some(dir.to_str().unwrap().to_string() + "/");
        g.registername = Some(regstr);
        g.latest_timestamp = 0;
        g.positions_register = None;
        g.COLS = 80;
        g.LINES = 24;
    });
    nanoxide::history::history_init();
    reg.to_str().unwrap().to_string()
}

fn current_lineno() -> isize {
    with_global(|g| {
        g.openfile
            .as_ref()
            .unwrap()
            .borrow()
            .current
            .as_ref()
            .unwrap()
            .borrow()
            .lineno
    })
}

#[test]
fn position_is_saved_and_restored() {
    let dir = std::env::temp_dir().join(format!("nanoxide_pos_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sample.txt");
    std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
    let name = file.to_str().unwrap().to_string();
    let regname = setup(&dir);

    SET(POSITIONLOG);
    nanoxide::files::open_buffer(&name);
    nanoxide::search::goto_line_and_column(2, 2, true);
    nanoxide::history::update_positions_register();
    assert!(std::path::Path::new(&regname).exists(), "应写出位置寄存器");

    /* 模拟下次启动：清空内存中的位置表，再从文件重新加载。 */
    with_global_mut(|g| {
        g.positions_register = None;
        g.latest_timestamp = 0;
    });
    nanoxide::history::load_positions_register();
    assert!(
        with_global(|g| g.positions_register.is_some()),
        "位置寄存器应能重新加载"
    );

    /* 重新打开文件并把光标放回开头，再恢复记录的位置。 */
    nanoxide::files::open_buffer(&name);
    assert_eq!(current_lineno(), 1);
    nanoxide::history::restore_cursor_position_if_any();

    assert_eq!(current_lineno(), 2, "应恢复到第 2 行");
    let (x, want) = with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        (of.current_x, of.placewewant)
    });
    assert_eq!(x, 1, "列号 2（1-based）应对应 current_x = 1");
    assert_eq!(want, 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir_all(&dir);
}
