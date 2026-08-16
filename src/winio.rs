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
fn translate_keycode(key: KeyEvent) -> i32 {
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

/// 刷新屏幕。
pub fn refresh_screen() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Clear(ClearType::All));

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
                // 清除行尾
                let _ = write!(stdout, "{:width$}", "", width = cols.saturating_sub(data.len()));
                let next = c.borrow().next.clone();
                current = next;
                row += 1;
            }
            // 清空剩余编辑行
            while row < edit_rows as u16 {
                let _ = execute!(stdout, cursor::MoveTo(0, 1 + row));
                let _ = write!(stdout, "{:width$}", "", width = cols);
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

/// 放置光标。
pub fn place_the_cursor() {
    with_global(|g| {
        let openfile = g.openfile.clone();
        if let Some(of) = openfile {
            let of_ref = of.borrow();
            let cursor_row = (of_ref.cursor_row + 1) as u16;
            let cursor_col = (of_ref.current_x + of_ref.firstcolumn) as u16;
            let mut stdout = io::stdout();
            let _ = execute!(stdout, cursor::MoveTo(cursor_col as u16, cursor_row));
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