/**************************************************************************
 * winio.rs  --  GNU nano 终端 I/O 与显示（对应 winio.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 * 转换说明：使用 crossterm 替代 ncurses 进行终端操作。
 **************************************************************************/

//! 终端 I/O、按键解析、屏幕刷新。对应原版 nano 的 `winio.c`。

use crate::definitions::*;
use crate::global;
use crate::color;
use crate::utils;
use crate::chars;
use std::io::{self, Write};
use crossterm::{
    cursor::{self, Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{self, Attribute, Color, SetAttribute, SetForegroundColor, SetBackgroundColor, Print},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, DisableLineWrap, EnableLineWrap},
};

/// 错误码。
pub const ERR: i32 = -1;

/// 初始化屏幕（对应 initscr）。
pub fn initscr() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen, DisableLineWrap, Hide);
    let _ = terminal::enable_raw_mode();
    update_screen_size();
}

/// 更新屏幕尺寸。
pub fn update_screen_size() {
    if let Ok((cols, rows)) = terminal::size() {
        with_global_mut(|g| {
            g.COLS = cols as usize;
            g.LINES = rows as usize;
            g.editwinrows = (rows as i32).saturating_sub(4).max(1);
        });
    }
}

/// 检查是否支持颜色（crossterm 总是支持）。
pub fn has_colors() -> bool {
    true
}

/// 初始化颜色（crossterm 无需特殊初始化）。
pub fn start_color() {}

/// 设置光标样式。
pub fn curs_set(visible: i32) {
    let mut stdout = io::stdout();
    if visible == 0 {
        let _ = execute!(stdout, Hide);
    } else {
        let _ = execute!(stdout, Show);
    }
}

/// 获取按键输入（对应 wgetch）。
pub fn wgetch() -> i32 {
    match event::read() {
        Ok(Event::Key(key)) => {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                translate_keycode(key)
            } else {
                ERR
            }
        }
        Ok(Event::Resize(cols, rows)) => {
            with_global_mut(|g| {
                g.COLS = cols as usize;
                g.LINES = rows as usize;
                g.editwinrows = (rows as i32).saturating_sub(4).max(1);
            });
            THE_WINDOW_RESIZED
        }
        Ok(Event::Paste(data)) => {
            // 括号粘贴
            for ch in data.chars() {
                // 逐个处理粘贴的字符
            }
            START_OF_PASTE
        }
        Ok(Event::FocusGained) => FOCUS_IN,
        Ok(Event::FocusLost) => FOCUS_OUT,
        _ => ERR,
    }
}

/// 获取按键代码（对应 get_keycode）。
pub fn get_keycode() -> i32 {
    wgetch()
}

/// 将 crossterm KeyEvent 转换为 nano 键码。
pub fn translate_keycode(key: KeyEvent) -> i32 {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers == KeyModifiers::CONTROL {
                // Ctrl + 字母
                if c.is_ascii_alphabetic() {
                    (c as u8 & 0x1F) as i32
                } else {
                    c as i32
                }
            } else if key.modifiers == KeyModifiers::ALT {
                // Alt + 字母
                match c {
                    'h' => KEY_HOME,
                    'H' => KEY_HOME,
                    _ => c as i32 | 0x200,
                }
            } else {
                c as i32
            }
        }
        KeyCode::Enter => KEY_ENTER,
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::Tab => '\t' as i32,
        KeyCode::BackTab => KEY_BTAB,
        KeyCode::Esc => ESC_CODE as i32,
        KeyCode::Delete => KEY_DC,
        KeyCode::Home => {
            if key.modifiers == KeyModifiers::CONTROL {
                CONTROL_HOME
            } else if key.modifiers == KeyModifiers::SHIFT {
                SHIFT_HOME
            } else {
                KEY_HOME
            }
        }
        KeyCode::End => {
            if key.modifiers == KeyModifiers::CONTROL {
                CONTROL_END
            } else if key.modifiers == KeyModifiers::SHIFT {
                SHIFT_END
            } else {
                KEY_END
            }
        }
        KeyCode::PageUp => {
            if key.modifiers == KeyModifiers::SHIFT {
                SHIFT_PAGEUP
            } else {
                KEY_PPAGE
            }
        }
        KeyCode::PageDown => {
            if key.modifiers == KeyModifiers::SHIFT {
                SHIFT_PAGEDOWN
            } else {
                KEY_NPAGE
            }
        }
        KeyCode::Left => {
            match key.modifiers {
                KeyModifiers::CONTROL => CONTROL_LEFT,
                KeyModifiers::ALT => ALT_LEFT,
                KeyModifiers::SHIFT => KEY_LEFT,
                _ => KEY_LEFT,
            }
        }
        KeyCode::Right => {
            match key.modifiers {
                KeyModifiers::CONTROL => CONTROL_RIGHT,
                KeyModifiers::ALT => ALT_RIGHT,
                KeyModifiers::SHIFT => KEY_RIGHT,
                _ => KEY_RIGHT,
            }
        }
        KeyCode::Up => {
            match key.modifiers {
                KeyModifiers::CONTROL => CONTROL_UP,
                KeyModifiers::ALT => ALT_UP,
                KeyModifiers::SHIFT => SHIFT_UP,
                _ => KEY_UP,
            }
        }
        KeyCode::Down => {
            match key.modifiers {
                KeyModifiers::CONTROL => CONTROL_DOWN,
                KeyModifiers::ALT => ALT_DOWN,
                KeyModifiers::SHIFT => SHIFT_DOWN,
                _ => KEY_DOWN,
            }
        }
        KeyCode::F(n) => KEY_F0 + n as i32,
        KeyCode::Insert => KEY_IC,
        KeyCode::Null => 0,
        _ => ERR,
    }
}

/// 等待按键代码。
pub fn waiting_keycodes() -> i32 {
    wgetch()
}

/// 启用键盘中断。
pub fn enable_kb_interrupt() {
    // crossterm 自动处理
}

/// 刷新编辑窗口。
pub fn edit_refresh() {
    // 简化：重绘整个屏幕
    refresh_screen();
}

/// 刷新屏幕（逐行覆盖重绘，避免全屏 Clear 造成的闪烁）。
pub fn refresh_screen() {
    let mut stdout = io::stdout();

    with_global(|g| {
        let cols = g.COLS;
        let lines = g.LINES;
        let edit_rows = lines.saturating_sub(4); // 标题栏1 + 状态栏1 + 快捷键2

        // 绘制标题栏（屏幕顶部第0行）
        let _ = execute!(stdout, cursor::MoveTo(0, 0));
        draw_titlebar_line(&mut stdout, cols);

        // 绘制编辑区域（从第1行开始）
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let mut current = of_ref.edittop.clone();
            let mut row = 0u16;
            while let Some(c) = current {
                if row >= edit_rows as u16 {
                    break;
                }
                let data = c.borrow().data.clone();
                let _ = execute!(stdout, cursor::MoveTo(0, 1 + row));
                let _ = write!(stdout, "{}", data);
                /* 清除该行剩余部分（比补空格更高效、闪烁更小）。 */
                let _ = execute!(stdout, Clear(ClearType::UntilNewLine));
                let next = c.borrow().next.clone();
                current = next;
                row += 1;
            }
            // 清空剩余编辑行
            while row < edit_rows as u16 {
                let _ = execute!(stdout, cursor::MoveTo(0, 1 + row));
                let _ = execute!(stdout, Clear(ClearType::UntilNewLine));
                row += 1;
            }
        }

        // 绘制状态栏（倒数第3行）
        let status_row = (lines.saturating_sub(3)) as u16;
        let _ = execute!(stdout, cursor::MoveTo(0, status_row));
        draw_statusbar_line(&mut stdout, cols);

        // 绘制底部快捷键（倒数第2行和倒数第1行）
        draw_bottombars_lines(&mut stdout, cols, lines);
    });

    let _ = stdout.flush();

    /* 刷新后恢复光标：显示并移到编辑位置。 */
    let _ = execute!(stdout, Show);
    place_the_cursor();
    let _ = stdout.flush();
}

/// 绘制标题栏行（格式参照 C 版 titlebar）。
fn draw_titlebar_line(stdout: &mut io::Stdout, cols: usize) {
    with_global(|g| {
        let filename = g.openfile.as_ref()
            .and_then(|of| of.borrow().filename.clone())
            .unwrap_or_else(|| "New Buffer".to_string());
        let modified = g.openfile.as_ref()
            .map(|of| of.borrow().modified)
            .unwrap_or(false);
        let state = if modified { " Modified" } else { "" };
        let left_text = format!(" GNU nano {} ", VERSION);
        let right_text = state;

        let left_len = left_text.len();
        let right_len = right_text.len();
        let path_max = cols.saturating_sub(left_len + right_len + 2);

        // 文件名居中或左对齐
        let path_display = if filename.len() > path_max {
            format!("...{}", &filename[filename.len().saturating_sub(path_max.saturating_sub(3))..])
        } else {
            filename.clone()
        };

        let _ = write!(stdout, "{}", left_text);
        // 填充空格使文件名居中
        let mid_pad = cols.saturating_sub(left_len + path_display.len() + right_len) / 2;
        if mid_pad > 0 {
            let _ = write!(stdout, "{:width$}", "", width = mid_pad);
        }
        let _ = write!(stdout, "{}", path_display);
        // 右对齐状态
        if right_len > 0 {
            let right_pad = cols.saturating_sub(left_len + mid_pad + path_display.len() + right_len);
            if right_pad > 0 {
                let _ = write!(stdout, "{:width$}", "", width = right_pad);
            }
            let _ = write!(stdout, "{}", right_text);
        }
    });
}

/// 绘制状态栏行。
fn draw_statusbar_line(stdout: &mut io::Stdout, cols: usize) {
    with_global(|g| {
        let msg = match g.lastmessage {
            MessageType::Vacuum => String::new(),
            _ => "(Info) ".to_string(),
        };
        if msg.len() > cols {
            let _ = write!(stdout, "{}", &msg[..cols]);
        } else {
            let _ = write!(stdout, "{}{:width$}", msg, "", width = cols - msg.len());
        }
    });
}

/// 绘制底部快捷键（两行，参照 C 版 bottombars 实现）。
fn draw_bottombars_lines(stdout: &mut io::Stdout, cols: usize, lines: usize) {
    with_global(|g| {
        let menu = g.currmenu;

        // 收集所有匹配当前菜单的函数条目
        let mut entries: Vec<(String, String)> = Vec::new();
        let mut current_func = g.allfuncs.clone();
        while let Some(f) = current_func {
            let f_ref = f.borrow();
            if (f_ref.menus & menu) != 0 {
                // 在 shortcuts 中查找匹配当前菜单的对应快捷键（与 C 版 first_sc_for 一致）
                let mut current_sc = g.shortcuts.clone();
                while let Some(s) = current_sc {
                    let s_ref = s.borrow();
                    if (s_ref.menus & menu) != 0 && s_ref.func == f_ref.func && !s_ref.keystr.is_empty() {
                        entries.push((s_ref.keystr.clone(), f_ref.tag.to_string()));
                        break;
                    }
                    current_sc = s_ref.next.clone();
                }
            }
            current_func = f_ref.next.clone();
        }

        // 限制条目数量（与 C 版 shown_entries_for 一致：((COLS + 40) / 20) * 2）
        let maximum = ((cols + 40) / 20) * 2;
        entries.truncate(maximum);

        let number = entries.len();
        if number == 0 {
            return;
        }

        // 计算每个条目的宽度（与 C 版一致）
        let itemwidth = cols / ((number + 1) / 2).max(1);
        if itemwidth == 0 {
            return;
        }

        let bottom_row1 = (lines.saturating_sub(2)) as u16;
        let bottom_row2 = (lines.saturating_sub(1)) as u16;

        for (index, (keystr, tag)) in entries.iter().enumerate() {
            // 计算行和列位置（与 C 版一致：wmove(footwin, 1 + index % 2, (index / 2) * itemwidth)）
            let row = if index % 2 == 0 { bottom_row1 } else { bottom_row2 };
            let col = ((index / 2) * itemwidth) as u16;

            // 处理最后一个条目（当数量为奇数时可能双倍宽度）
            let mut thiswidth = itemwidth;
            if (number % 2 == 1) && (index + 2 == number) {
                thiswidth += itemwidth;
            }
            if index + 2 >= number {
                thiswidth += cols % itemwidth;
            }

            // 写入快捷键和功能标签（格式：^G Help，截断到 thiswidth）
            let display = format!("{} {}", keystr, tag);
            let truncated: String = display.chars().take(thiswidth.min(cols.saturating_sub(col as usize))).collect();
            let _ = execute!(stdout, cursor::MoveTo(col, row));
            let _ = write!(stdout, "{:width$}", truncated, width = thiswidth.min(cols.saturating_sub(col as usize)));
        }
    });
}

/// 在状态栏显示消息。
pub fn statusbar(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
    });
    let mut stdout = io::stdout();
    let lines = with_global(|g| g.LINES);
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    let _ = write!(stdout, "{}", msg);
    let _ = stdout.flush();
}

/// 在状态行显示消息。
pub fn statusline(typ: MessageType, msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = typ;
    });
    let mut stdout = io::stdout();
    let lines = with_global(|g| g.LINES);
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    let _ = write!(stdout, "{}", msg);
    let _ = stdout.flush();
}

/// 在指定位置显示文本。
pub fn mvwaddstr(_win: bool, _row: i32, _col: i32, _text: &str) {
    // 简化
}

/// 清除状态栏。
pub fn wipe_statusbar() {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Vacuum;
    });
    let mut stdout = io::stdout();
    let lines = with_global(|g| g.LINES);
    let cols = with_global(|g| g.COLS);
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    let _ = write!(stdout, "{:width$}", "", width = cols);
    let _ = stdout.flush();
}

/// 显示底部栏快捷键。
pub fn bottombars() {
    let mut stdout = io::stdout();
    let cols = with_global(|g| g.COLS);
    let lines = with_global(|g| g.LINES);
    draw_bottombars_lines(&mut stdout, cols, lines);
    let _ = stdout.flush();
}

/// 清除底部栏。
pub fn blank_bottombars() {
    let mut stdout = io::stdout();
    let lines = with_global(|g| g.LINES);
    let cols = with_global(|g| g.COLS);
    let row1 = (lines.saturating_sub(2)) as u16;
    let row2 = (lines.saturating_sub(1)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, row1));
    let _ = write!(stdout, "{:width$}", "", width = cols);
    let _ = execute!(stdout, cursor::MoveTo(0, row2));
    let _ = write!(stdout, "{:width$}", "", width = cols);
    let _ = stdout.flush();
}

/// 清除编辑区域。
pub fn blank_edit() {
    let mut stdout = io::stdout();
    let lines = with_global(|g| g.LINES);
    let cols = with_global(|g| g.COLS);
    let edit_rows = lines.saturating_sub(4);
    for row in 0..edit_rows as u16 {
        let _ = execute!(stdout, cursor::MoveTo(0, 1 + row));
        let _ = write!(stdout, "{:width$}", "", width = cols);
    }
    let _ = stdout.flush();
}

/// 放置光标（对应 `place_the_cursor`）。
/// 行位置按 `current.lineno - edittop.lineno` 计算（与 C 一致），
/// 并更新 `cursor_row`。
pub fn place_the_cursor() {
    let editwinrows = with_global(|g| g.editwinrows);
    with_global_mut(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let mut of_ref = of.borrow_mut();
            let (cur, edittop) = match (&of_ref.current, &of_ref.edittop) {
                (Some(c), Some(e)) => (c.clone(), e.clone()),
                _ => return,
            };
            let row = {
                let cur_lineno = cur.borrow().lineno;
                let edit_lineno = edittop.borrow().lineno;
                cur_lineno - edit_lineno
            };
            of_ref.cursor_row = row;
            if row < editwinrows as isize {
                /* 光标列用显示列宽计算（而非字节偏移）。 */
                let data = cur.borrow().data.clone();
                let column = crate::utils::wideness(data.as_bytes(), of_ref.current_x);
                let mut stdout = io::stdout();
                let _ = execute!(stdout, cursor::MoveTo(column as u16, (row + 1) as u16));
            }
        }
    });
}

/// 重绘标题栏。
pub fn titlebar(_file: Option<&str>) {
    let mut stdout = io::stdout();
    let cols = with_global(|g| g.COLS);
    let _ = execute!(stdout, cursor::MoveTo(0, 0));
    draw_titlebar_line(&mut stdout, cols);
    let _ = stdout.flush();
}

/// 窗口初始化。
pub fn window_init() {
    initscr();
}

/// 进入终端（备用屏幕）。
pub fn enter_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen);
}

/// 离开终端。
pub fn leave_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

/// 睡眠指定毫秒。
pub fn napms(ms: u32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

/// 获取键盘输入。
pub fn get_kbinput() -> i32 {
    wgetch()
}

/// 检查是否有未处理的按键事件。
pub fn kbhit() -> bool {
    event::poll(std::time::Duration::from_millis(0)).unwrap_or(false)
}

/// 设置颜色并打印文本。
pub fn print_colored(fg: Color, bg: Color, text: &str) {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, SetForegroundColor(fg), SetBackgroundColor(bg));
    let _ = write!(stdout, "{}", text);
    let _ = execute!(stdout, SetForegroundColor(Color::Reset), SetBackgroundColor(Color::Reset));
}

/// 恢复终端状态（编辑器退出时调用）。
pub fn terminal_restore() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen, EnableLineWrap, Show);
    let _ = terminal::disable_raw_mode();
}

/// 终端响铃（对应 curses 的 `beep`）。
pub fn beep() {
    use std::io::Write;
    let _ = std::io::stdout().write_all(b"\x07");
    let _ = std::io::stdout().flush();
}

/// 重绘编辑窗口：必要时调整视口并标记需要刷新
/// （对应 `edit_redraw`；渲染细节由 `edit_refresh` 完成）。
pub fn edit_redraw(old_current: &LineRef, manner: UpdateType) {
    if manner == UpdateType::Flowing {
        /* 若光标移动，确保当前行可见。 */
        let moved = with_global(|g| {
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                of.current.as_ref().map(|c| !std::rc::Rc::ptr_eq(c, old_current)).unwrap_or(false)
            }).unwrap_or(false)
        });
        if moved && current_is_offscreen() {
            adjust_viewport(UpdateType::Stationary);
        }
    } else if current_is_offscreen() {
        adjust_viewport(manner);
    }

    with_global_mut(|g| g.refresh_needed = true);
}

// ======================== 软换行（对应 winio.c 软换行函数组） ========================

use std::cell::Cell;

/// `get_softwrap_breakpoint` 的静态状态（对应 C 的 static text/column）。
thread_local! {
    static SW_LINE_START: Cell<usize> = const { Cell::new(0) };
    static SW_INDEX: Cell<usize> = const { Cell::new(0) };
    static SW_COLUMN: Cell<usize> = const { Cell::new(0) };
}

fn editwincols_value() -> usize {
    with_global(|g| g.editwincols)
}

/// SHIM 宏：在 ZERO 模式下、替换/确认菜单中把底栏算作一行。
fn shim_value() -> i32 {
    with_global(|g| {
        if ISSET(ZERO) && (g.currmenu == MREPLACEWITH || g.currmenu == MYESNO) {
            1
        } else {
            0
        }
    })
}

/// 获取 leftedge 之后可断开给定 linedata 的列号并返回。
/// （结果至多比 leftedge 大 editwincols。）
/// 当 kickoff 为 TRUE 时从 linedata 开头开始；否则从上一次调用处继续。
/// 当搜索断点时到达行尾，将 end_of_line 置为 TRUE。
/// （对应 `get_softwrap_breakpoint`。）
pub fn get_softwrap_breakpoint(
    linedata: &[u8],
    leftedge: usize,
    kickoff: &mut bool,
    end_of_line: &mut bool,
) -> usize {
    let rightside = leftedge + editwincols_value();
    /* 可在其处断开文本的列（无更佳选择时）。 */
    let mut breaking_col = rightside;
    /* 最近见到的空白字符的列位置。 */
    let mut last_blank_col = 0;
    /* 最近见到的空白字符的位置。 */
    let mut farthest_blank: Option<usize> = None;

    /* 换行时初始化静态变量。 */
    if *kickoff {
        SW_LINE_START.set(linedata.as_ptr() as usize);
        SW_INDEX.set(0);
        SW_COLUMN.set(0);
        *kickoff = false;
    }

    let mut index = SW_INDEX.get();
    let mut column = SW_COLUMN.get();

    /* 先找到文本中当前块开始的位置。 */
    while chars::byte_at(linedata, index) != 0 && column < leftedge {
        index += chars::advance_over(&linedata[index..], &mut column);
    }

    /* 再找到文本中本块应结束的位置。 */
    while chars::byte_at(linedata, index) != 0 && column <= rightside {
        /* 在空白处断行时，在目标列 *之前* 断开。 */
        if ISSET(AT_BLANKS) && chars::is_blank_char(&linedata[index..]) && column < rightside {
            farthest_blank = Some(index);
            last_blank_col = column;
        }

        breaking_col = if linedata[index] == b'\t' { rightside } else { column };
        index += chars::advance_over(&linedata[index..], &mut column);
    }

    /* 保存静态状态，供下一次调用继续。 */
    SW_INDEX.set(index);
    SW_COLUMN.set(column);

    /* 若未越过限制，则已找到断点；若甚至未*到达*限制则已到行尾。 */
    if column <= rightside {
        *end_of_line = column < rightside;
        return column;
    }

    /* 若在空白处软换行且找到至少一个空白，则在该空白之后断开——
     * 只要它不越过屏幕边缘。 */
    if let Some(fb) = farthest_blank {
        let mut aftertheblank = last_blank_col;
        let onestep = chars::advance_over(&linedata[fb..], &mut aftertheblank);

        if aftertheblank <= rightside {
            SW_INDEX.set(fb + onestep);
            SW_COLUMN.set(aftertheblank);
            return aftertheblank;
        }

        /* 若是越过边缘的制表符，则在屏幕边缘断开。 */
        if linedata[fb] == b'\t' {
            breaking_col = rightside;
        }
    }

    /* 否则，在最后一个不越界的字符处断开。 */
    if editwincols_value() > 1 {
        breaking_col
    } else {
        column.saturating_sub(1)
    }
}

/// 返回给定行中、给定列所在的软换行块的行号（相对首行，零基）。
/// 若 leftedge 非 None，在其中返回该块的最左列。
/// （对应 `get_chunk_and_edge`。）
pub fn get_chunk_and_edge(column: usize, line: &LineRef, leftedge: Option<&mut usize>) -> usize {
    let mut current_chunk = 0;
    let mut end_of_line = false;
    let mut kickoff = true;
    let mut start_col = 0;

    loop {
        let data = line.borrow().data.clone();
        let end_col = get_softwrap_breakpoint(data.as_bytes(), start_col, &mut kickoff, &mut end_of_line);

        /* 当列在范围内或到达行尾时，结束。 */
        if end_of_line || (start_col <= column && column < end_col) {
            if let Some(le) = leftedge {
                *le = start_col;
            }
            return current_chunk;
        }

        start_col = end_col;
        current_chunk += 1;
    }
}

/// 返回给定行软换行时需要的额外行数（对应 `extra_chunks_in`）。
pub fn extra_chunks_in(line: &LineRef) -> usize {
    get_chunk_and_edge(usize::MAX >> 1, line, None)
}

/// 返回给定行中、column 所在的软换行块的行号（相对首行，零基）
/// （对应 `chunk_for`）。
pub fn chunk_for(column: usize, line: &LineRef) -> usize {
    get_chunk_and_edge(column, line, None)
}

/// 返回给定行中、给定列所在的软换行块的最左列（对应 `leftedge_for`）。
pub fn leftedge_for(column: usize, line: &LineRef) -> usize {
    let mut leftedge = 0;
    get_chunk_and_edge(column, line, Some(&mut leftedge));
    leftedge
}

/// 软换行模式下，若给定列位于软换行块的断点处或其之后，则将其移回
/// 断点前的最后一列。给定列相对于 current 中的给定 leftedge；
/// 返回的列相对于文本开头（对应 `actual_last_column`）。
pub fn actual_last_column(leftedge: usize, mut column: usize) -> usize {
    if ISSET(SOFTWRAP) {
        let mut kickoff = true;
        let mut last_chunk = false;
        let data = with_global(|g| {
            g.openfile.as_ref().map(|of| {
                of.borrow().current.as_ref().map(|c| c.borrow().data.clone()).unwrap_or_default()
            }).unwrap_or_default()
        });
        let end_col = get_softwrap_breakpoint(data.as_bytes(), leftedge, &mut kickoff, &mut last_chunk) - leftedge;

        /* 若不在最后一块，则已越过行末一列。后退一列可能落在多列字符
         * 中间，但 actual_x() 稍后会修正。 */
        let end_col = if last_chunk { end_col } else { end_col.saturating_sub(1) };

        if column > end_col {
            column = end_col;
        }
    }

    leftedge + column
}

/// 尝试从给定行和给定列（leftedge）向上移动 nrows 个软换行块。
/// 移动后，leftedge 将设为当前块的起始列。
/// 返回未能向上移动的块数，完全成功时为零（对应 `go_back_chunks`）。
pub fn go_back_chunks(nrows: i32, line: &mut LineRef, leftedge: &mut usize) -> i32 {
    let mut i = nrows;

    if ISSET(SOFTWRAP) {
        /* 回退请求的块数。 */
        while i > 0 {
            let chunk = chunk_for(*leftedge, line);
            *leftedge = 0;

            if chunk as i32 >= i {
                return go_forward_chunks(chunk as i32 - i, line, leftedge);
            }

            let at_filetop = with_global(|g| {
                g.openfile.as_ref().map(|of| {
                    let of = of.borrow();
                    of.filetop.as_ref().map(|t| std::rc::Rc::ptr_eq(t, line)).unwrap_or(false)
                }).unwrap_or(false)
            });
            if at_filetop {
                break;
            }

            i -= chunk as i32;
            let prev = { let r = line.borrow(); r.prev.clone() };
            *line = prev.and_then(|w| w.upgrade()).unwrap();
            *leftedge = usize::MAX >> 1;
        }

        if *leftedge == usize::MAX >> 1 {
            *leftedge = leftedge_for(*leftedge, line);
        }
    } else {
        while i > 0 {
            let has_prev = { let r = line.borrow(); r.prev.is_some() };
            if !has_prev {
                break;
            }
            let prev = { let r = line.borrow(); r.prev.clone() };
            *line = prev.and_then(|w| w.upgrade()).unwrap();
            i -= 1;
        }
    }

    i
}

/// 尝试从给定行和给定列（leftedge）向下移动 nrows 个软换行块。
/// 移动后，leftedge 将设为当前块的起始列。
/// 返回未能向下移动的块数，完全成功时为零（对应 `go_forward_chunks`）。
pub fn go_forward_chunks(nrows: i32, line: &mut LineRef, leftedge: &mut usize) -> i32 {
    let mut i = nrows;

    if ISSET(SOFTWRAP) {
        let mut current_leftedge = *leftedge;
        let mut kickoff = true;

        /* 前进请求的块数。 */
        while i > 0 {
            let mut end_of_line = false;
            let data = { let r = line.borrow(); r.data.clone() };
            current_leftedge = get_softwrap_breakpoint(data.as_bytes(), current_leftedge, &mut kickoff, &mut end_of_line);

            if !end_of_line {
                i -= 1;
                continue;
            }

            let at_filebot = with_global(|g| {
                g.openfile.as_ref().map(|of| {
                    let of = of.borrow();
                    of.filebot.as_ref().map(|b| std::rc::Rc::ptr_eq(b, line)).unwrap_or(false)
                }).unwrap_or(false)
            });
            if at_filebot {
                break;
            }

            let next = { let r = line.borrow(); r.next.clone() };
            *line = next.unwrap();
            current_leftedge = 0;
            kickoff = true;
            i -= 1;
        }

        /* 仅当确实能够移动时才更改 leftedge。 */
        if i < nrows {
            *leftedge = current_leftedge;
        }
    } else {
        while i > 0 {
            let has_next = { let r = line.borrow(); r.next.is_some() };
            if !has_next {
                break;
            }
            let next = { let r = line.borrow(); r.next.clone() };
            *line = next.unwrap();
            i -= 1;
        }
    }

    i
}

/// 返回 TRUE 如果 current[current_x] 在视口之前（对应 `current_is_above_screen`）。
pub fn current_is_above_screen() -> bool {
    with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let current = of.current.clone().unwrap();
        let edittop = of.edittop.clone().unwrap();
        let cur_lineno = current.borrow().lineno;
        let edit_lineno = edittop.borrow().lineno;

        if ISSET(SOFTWRAP) {
            cur_lineno < edit_lineno
                || (cur_lineno == edit_lineno && utils::xplustabs() < of.firstcolumn)
        } else {
            cur_lineno < edit_lineno
        }
    })
}

/// 返回 TRUE 如果 current[current_x] 在视口之外（对应 `current_is_below_screen`）。
pub fn current_is_below_screen() -> bool {
    with_global(|g| {
        let shim = shim_value();
        if ISSET(SOFTWRAP) {
            let mut line = g.openfile.as_ref().expect("no open file").borrow().edittop.clone().unwrap();
            let mut leftedge = g.openfile.as_ref().unwrap().borrow().firstcolumn;
            let rows = g.editwinrows - 1 - shim;
            go_forward_chunks(rows, &mut line, &mut leftedge);
            let of = g.openfile.as_ref().unwrap().borrow();
            let current = of.current.clone().unwrap();
            line.borrow().lineno < current.borrow().lineno
                || (line.borrow().lineno == current.borrow().lineno
                    && leftedge < leftedge_for(utils::xplustabs(), &current))
        } else {
            let of = g.openfile.as_ref().unwrap().borrow();
            let current = of.current.clone().unwrap();
            let edittop = of.edittop.clone().unwrap();
            let cur_lineno = current.borrow().lineno;
            let edit_lineno = edittop.borrow().lineno;
            cur_lineno >= edit_lineno + (g.editwinrows - shim) as isize
        }
    })
}

/// 返回 TRUE 如果 current[current_x] 在视口之外（对应 `current_is_offscreen`）。
pub fn current_is_offscreen() -> bool {
    current_is_above_screen() || current_is_below_screen()
}

/// 移动 edittop 使 current 显示在屏幕上。manner 说明方式：
/// STATIONARY 表示光标应保持在同一个屏幕行上，
/// CENTERING 表示 current 应位于屏幕中央，
/// FLOWING 表示只需滚动到足以让 current 进入视野。
/// （对应 `adjust_viewport`。）
pub fn adjust_viewport(manner: UpdateType) {
    let mut goal = 0;

    if manner == UpdateType::Stationary {
        goal = with_global(|g| g.openfile.as_ref().unwrap().borrow().cursor_row as i32);
    } else if manner == UpdateType::Centering {
        goal = with_global(|g| g.editwinrows) / 2;
    } else if !current_is_above_screen() {
        goal = with_global(|g| g.editwinrows) - 1 - shim_value();
    }

    let (current, pww, softwrap) = with_global(|g| {
        let r = g.openfile.as_ref().expect("no open file").borrow();
        let cur = r.current.clone().unwrap();
        let p = utils::wideness(cur.borrow().data.as_bytes(), r.current_x);
        (cur, p, g.flags.isset(SOFTWRAP))
    });

    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let mut of = of.borrow_mut();
        of.edittop = of.current.clone();
    });

    if softwrap {
        let fc = leftedge_for(pww, &current);
        with_global_mut(|g| {
            let of = g.openfile.as_ref().expect("no open file").clone();
            of.borrow_mut().firstcolumn = fc;
        });
    }

    /* 从 current[current_x] 开始将 edittop 回退 goal 行。 */
    let (edittop0, firstcolumn0) = with_global(|g| {
        let r = g.openfile.as_ref().expect("no open file").borrow();
        (r.edittop.clone().unwrap(), r.firstcolumn)
    });
    let mut edittop = edittop0;
    let mut firstcolumn = firstcolumn0;
    go_back_chunks(goal, &mut edittop, &mut firstcolumn);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let mut of = of.borrow_mut();
        of.edittop = Some(edittop);
        of.firstcolumn = firstcolumn;
    });
}
// ======================== 视口滚动与行更新（对应 winio.c） ========================

/// 检查标记是否开启，或 old_column 与 new_column 是否在不同"页"上
/// （软换行模式下仅前者适用），这意味着相关行需要重绘
/// （对应 `line_needs_update`）。
pub fn line_needs_update(old_column: usize, new_column: usize) -> bool {
    if crate::utils::get_page_start(old_column) == crate::utils::get_page_start(new_column) {
        return with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false));
    }
    let united = with_global(|g| g.united_sidescroll);
    if united {
        with_global_mut(|g| g.refresh_needed = true);
    }
    !with_global(|g| g.refresh_needed)
}

/// 把编辑窗口顶行向上（BACKWARD）或向下（FORWARD）移动一行或一块，
/// 并重绘新出现的行。crossterm 架构下以全量刷新等价实现
/// （对应 `edit_scroll`）。
pub fn edit_scroll(direction: ScrollDirection) {
    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let mut of = of.borrow_mut();
        let mut edittop = of.edittop.clone().unwrap();
        let mut firstcolumn = of.firstcolumn;

        if direction == ScrollDirection::Backward {
            go_back_chunks(1, &mut edittop, &mut firstcolumn);
        } else {
            go_forward_chunks(1, &mut edittop, &mut firstcolumn);
        }
        of.edittop = Some(edittop);
        of.firstcolumn = firstcolumn;
    });
    with_global_mut(|g| g.refresh_needed = true);
}

/// 无条件重绘整个屏幕（对应 `full_refresh`）。
pub fn full_refresh() {
    with_global_mut(|g| g.refresh_needed = true);
    edit_refresh();
}

/// 绘制屏幕的三个元素：标题栏、编辑窗口内容、底栏
/// （对应 `draw_all_subwindows`）。
pub fn draw_all_subwindows() {
    edit_refresh();
    bottombars();
}

/// 滚动方向枚举（对应 winio.c 的 `update_type` 中 FORWARD/BACKWARD 用法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Forward,
    Backward,
}

/// 重绘给定行（对应 winio.c 的 `update_line`）。
/// 返回该行占用的行数（软换行时为块数，否则为 1）。
/// 在 crossterm 架构下，逐行渲染由 `edit_refresh` 的全量重绘完成，
/// 这里标记需要刷新并返回正确的行数。
pub fn update_line(line: &LineRef, index: usize) -> i32 {
    let _ = index;
    with_global_mut(|g| g.refresh_needed = true);
    if ISSET(SOFTWRAP) {
        (extra_chunks_in(line) + 1) as i32
    } else {
        1
    }
}

// ======================== 字符串显示与逐字输入（对应 winio.c） ========================

/// 将给定文本转换为可在终端显示的字符串：控制字符显示为 ^X，
/// 制表符展开为空格，宽字符保留，零宽字符处理等。
/// column 是起始列，span 是可用宽度（对应 `display_string`）。
pub fn display_string(text: &[u8], column: usize, span: usize, isdata: bool, isprompt: bool) -> String {
    let start_x = crate::utils::actual_x(text, column);
    let start_col = crate::utils::wideness(text, start_x);
    let beyond = column + span;

    let mut pos = start_x;
    let mut col = start_col;
    let mut converted: Vec<u8> = Vec::new();

    /* 若第一个字符在左边缘之前开始，或被 "<" 记号覆盖，显示占位符。 */
    if (start_col < column || (start_col > 0 && isdata && !ISSET(SOFTWRAP)))
        && chars::byte_at(text, pos) != 0
        && chars::byte_at(text, pos) != b'\t'
    {
        if chars::is_cntrl_char(&text[pos..]) {
            if start_col < column {
                converted.push(chars::control_mbrep(&text[pos..], isdata));
                col += 1;
                pos += chars::char_length(&text[pos..]);
            }
        } else if chars::is_doublewidth(&text[pos..]) {
            if start_col == column {
                converted.push(b' ');
                col += 1;
            }
            /* 双宽字符的右半显示为 ']'。 */
            converted.push(b']');
            col += 1;
            pos += chars::char_length(&text[pos..]);
        }
    }

    while chars::byte_at(text, pos) != 0 && (col < beyond || chars::is_zerowidth(&text[pos..])) {
        let c = text[pos];

        /* 普通可打印 ASCII 字符占一字节一列。 */
        if (c as i8) > 0x20 && c != DEL_CODE {
            converted.push(c);
            pos += 1;
            col += 1;
            continue;
        }

        /* 空格显示为可见字符或空格。 */
        if c == b' ' {
            if ISSET(WHITESPACE_DISPLAY) {
                let (ws, (wl0, wl1)) = with_global(|g| (g.whitespace.clone(), g.whitelen));
                if let Some(w) = ws {
                    for i in wl0..wl0 + wl1 {
                        if i < w.len() {
                            converted.push(w[i]);
                        }
                    }
                }
            } else {
                converted.push(b' ');
            }
            col += 1;
            pos += 1;
            continue;
        }

        /* 制表符显示为可见字符加空格，或仅空格。 */
        if c == b'\t' {
            let tabsize = with_global(|g| g.tabsize);
            let show_ws = ISSET(WHITESPACE_DISPLAY)
                && (converted.len() > 0 || !isdata || !ISSET(SOFTWRAP)
                    || col % tabsize == 0 || col == start_col);
            if show_ws {
                let (ws, (wl0, _)) = with_global(|g| (g.whitespace.clone(), g.whitelen));
                if let Some(w) = ws {
                    for i in 0..wl0 {
                        if i < w.len() {
                            converted.push(w[i]);
                        }
                    }
                }
            } else {
                converted.push(b' ');
            }
            col += 1;
            /* 用所需数量的空格填满制表符。 */
            while col % tabsize != 0 && col < beyond {
                converted.push(b' ');
                col += 1;
            }
            pos += 1;
            continue;
        }

        /* 控制字符以前导脱字符表示。 */
        if chars::is_cntrl_char(&text[pos..]) {
            converted.push(b'^');
            converted.push(chars::control_mbrep(&text[pos..], isdata));
            pos += chars::char_length(&text[pos..]);
            col += 2;
            continue;
        }

        /* 多字节字符：转换为宽字符确定宽度。 */
        match chars::mbtowide(&text[pos..]) {
            Err(()) => {
                /* 非法字符显示为替换符。 */
                converted.extend_from_slice(b"\xEF\xBF\xBD");
                pos += 1;
                col += 1;
            }
            Ok((wc, charlen)) => {
                let charwidth = chars::wcwidth(wc);
                let on_vt = with_global(|g| g.on_a_vt);
                if charwidth == 0 {
                    /* 在 Linux 控制台上跳过零宽字符。 */
                    if on_vt {
                        pos += charlen;
                        continue;
                    }
                }
                for i in 0..charlen {
                    if pos + i < text.len() {
                        converted.push(text[pos + i]);
                    }
                }
                pos += charlen;
                col += if charwidth < 0 { 1 } else { charwidth as usize };
            }
        }
    }

    /* 若有更多文本无法显示，为 ">" 腾出空间。 */
    if col > beyond || (chars::byte_at(text, pos) != 0 && (isprompt || (isdata && !ISSET(SOFTWRAP)))) {
        /* 后退一个字符（跳过零宽字符）。 */
        loop {
            if converted.is_empty() {
                break;
            }
            let step = chars::step_left(&converted, converted.len());
            converted.truncate(step);
            if converted.is_empty() || !chars::is_zerowidth(&converted[converted.len()..]) {
                break;
            }
        }
        /* 双宽字符的左半显示为 '['。 */
        if !converted.is_empty() {
            let clen = chars::char_length(&converted[converted.len()..]);
            let start = converted.len() - clen;
            if chars::is_doublewidth(&converted[start..]) {
                converted.truncate(start);
                converted.push(b'[');
            }
        }
    }

    String::from_utf8_lossy(&converted).into_owned()
}

/// 读取一个逐字按键（一个或两个转义序列），返回其字节
/// （对应 `get_verbatim_kbinput`；基于 crossterm 按键流）。
pub fn get_verbatim_kbinput(count: &mut usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    *count = 0;

    let code = get_keycode();
    if code < 0 {
        return bytes;
    }
    let c = code as u8;
    bytes.push(c);
    *count = 1;

    /* 若收到的是 UTF-8 起始字节，也读取续字节并组装成一个字符。 */
    if (0xC0..=0xF7).contains(&c) {
        let extras = (c / 16) % 4 + if c <= 0xCF { 1 } else { 0 };
        for _ in 0..extras {
            let next = get_keycode();
            if next < 0 {
                break;
            }
            bytes.push(next as u8);
            *count += 1;
        }
    }

    bytes
}
