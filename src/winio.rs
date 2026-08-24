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
use crate::movement;
use crate::cut;
use crate::text;
use crate::search;
use crate::help;
use crate::files;
use std::io::{self, Write};
use std::rc::Rc;
use crossterm::{
    cursor::{self, Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::{Attribute, Color, SetAttribute, SetForegroundColor, SetBackgroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, DisableLineWrap, EnableLineWrap},
};

/// 错误码。
pub const ERR: i32 = -1;

// ======================== 植入机制（对应 winio.c 的 implant / get_code_from_plantation） ========================

/// 植入队列中的一项：普通字符键码或函数命令。
enum Planted {
    Key(i32),
    Command(FunctionId, i32),
}

thread_local! {
    static PLANTED_QUEUE: std::cell::RefCell<std::collections::VecDeque<Planted>> =
        const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
}

/// UTF-8 首字节的长度。
fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

/// 植入一个展开字符串（对应 C 版 `implant` + `get_code_from_plantation`）。
/// 支持 `{function}` 占位符与 `{{}` / `{}}` 转义；普通文本按字符压入输入队列。
pub fn implant(expansion: &str) {
    PLANTED_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        let bytes = expansion.as_bytes();
        let n = bytes.len();
        let mut i = 0;
        while i < n {
            if bytes[i] == b'{' {
                /* {{} 与 {}} 转义为字面花括号。 */
                if i + 2 < n && bytes[i + 2] == b'}' && (bytes[i + 1] == b'{' || bytes[i + 1] == b'}') {
                    queue.push_back(Planted::Key(bytes[i + 1] as i32));
                    i += 3;
                    continue;
                }
                /* {command}：函数名占位符。 */
                if let Some(closing) = expansion[i + 1..].find('}') {
                    let name = &expansion[i + 1..i + 1 + closing];
                    if let Some((func, toggle)) = global::strtosc(name) {
                        queue.push_back(Planted::Command(func, toggle));
                    }
                    /* 未知函数：忽略（对应 C 版 NO_SUCH_FUNCTION）。 */
                    i += 1 + closing + 1;
                    continue;
                }
                /* 未闭合的 {：按字面处理。 */
                queue.push_back(Planted::Key(b'{' as i32));
                i += 1;
                continue;
            }
            /* 普通字符：按 code point 压入，避免逐字节拆分多字节字符。 */
            let ch_len = utf8_len(bytes[i]).min(n - i);
            if let Some(c) = expansion[i..i + ch_len].chars().next() {
                queue.push_back(Planted::Key(c as i32));
            }
            i += ch_len;
        }
    });
}

// ======================== 宏录制与回放（对应 winio.c record_macro / run_macro） ========================

/// 切换宏录制状态（对应 `record_macro`）。
pub fn record_macro() {
    let recording = with_global(|g| g.recording);
    if !recording {
        with_global_mut(|g| {
            g.recording = true;
            g.macro_buffer.clear();
        });
        statusline(MessageType::Remark, &crate::t!("macro-recording"));
    } else {
        with_global_mut(|g| {
            g.recording = false;
            /* 剪掉触发停止的按键。 */
            g.macro_buffer.pop();
        });
        statusline(MessageType::Remark, &crate::t!("macro-stopped"));
    }
}

/// 把宏按键序列排入输入队列（对应 `run_macro`）。
pub fn run_macro() {
    if with_global(|g| g.recording) {
        statusline(MessageType::Ahem, &crate::t!("macro-while_recording"));
        return;
    }
    let buffer = with_global(|g| g.macro_buffer.clone());
    if buffer.is_empty() {
        statusline(MessageType::Ahem, &crate::t!("macro-empty"));
        return;
    }
    /* 正序压入植入队列（C 版逆序 put_back + 栈式读取等价于正序执行）。 */
    PLANTED_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        for code in buffer {
            queue.push_back(Planted::Key(code));
        }
    });
}

/// 初始化屏幕（对应 initscr）。
pub fn initscr() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen, DisableLineWrap, Hide);
    let _ = terminal::enable_raw_mode();
    update_screen_size();
}

/// 根据 `fill` 和 `COLS` 更新 `wrap_at`。
/// 对应原版 C 中 `wrap_at` 的语义：
/// - `fill > 0` → `wrap_at = fill`
/// - `fill == 0` → `wrap_at = COLS`（终端宽度处换行）
/// - `fill < 0` → `wrap_at = 0`（禁用换行）
pub fn update_wrap_at() {
    with_global_mut(|g| {
        g.wrap_at = if g.fill > 0 {
            g.fill as usize
        } else if g.fill == 0 {
            g.COLS
        } else {
            0
        };
    });
}

/// 更新屏幕尺寸。
pub fn update_screen_size() {
    if let Ok((cols, rows)) = terminal::size() {
        with_global_mut(|g| {
            g.COLS = cols as usize;
            g.LINES = rows as usize;
            g.editwinrows = (rows as i32).saturating_sub(4).max(1);
            let sidebar_width = if g.sidebar { 1 } else { 0 };
            g.editwincols = (cols as i32 - g.margin - sidebar_width).max(1) as usize;
        });
    } else {
        // 终端尺寸探测失败（如非 TTY / 重定向）时给出合理默认，
        // 确保 editwincols 永不为 0，避免后续减法溢出。
        with_global_mut(|g| {
            if g.editwincols < 2 {
                g.editwincols = std::cmp::max(g.COLS, 2);
            }
        });
    }
    update_wrap_at();
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
    // 优先消费植入队列（对应 C 版 get_input 先处理 put_back 的键）。
    if let Some(planted) = PLANTED_QUEUE.with(|q| q.borrow_mut().pop_front()) {
        match planted {
            Planted::Key(k) => return k,
            Planted::Command(func, toggle) => {
                /* 函数命令直接执行（对应 C 版主循环对 PLANTED_A_COMMAND 的处理）。 */
                if func == FunctionId::DoToggle {
                    TOGGLE(toggle as usize);
                    edit_refresh();
                } else {
                    let _ = execute_by_id(func);
                    edit_refresh();
                }
                return FOREIGN_SEQUENCE;
            }
        }
    }
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
            // 括号粘贴：把粘贴文本作为普通输入插入缓冲区（支持多行）。
            // 返回 FOREIGN_SEQUENCE 让主循环跳过（文本已在此插入）。
            let text = data.replace("\r\n", "\n").replace('\r', "");
            if !ISSET(VIEW_MODE) {
                for (i, part) in text.split('\n').enumerate() {
                    if i > 0 {
                        crate::text::do_enter();
                    }
                    if !part.is_empty() {
                        crate::text::inject(part.as_bytes(), part.len());
                    }
                }
                edit_refresh();
            }
            FOREIGN_SEQUENCE
        }
        // 焦点事件视为杂散码丢弃（对应 C 版 winio.c get_keycode() 中
        // "if (keycode == mousefocusin || keycode == mousefocusout) return ERR;"）。
        // 若不丢弃，0x491/0x499 会被 handle_input_key 当作普通 Unicode 字符
        // （U+0491 'ґ'/U+0499 'ҙ'）插入文本，表现为切换窗口时出现随机字符。
        Ok(Event::FocusGained) | Ok(Event::FocusLost) => ERR,
        _ => ERR,
    }
}

/// 获取按键代码（对应 get_keycode）。
pub fn get_keycode() -> i32 {
    wgetch()
}

/// Ctrl + 字符 → nano 键码（对应原版 ncurses wgetch 的控制字符语义）。
///
/// 两套后端对 Ctrl 组合键的表示不同，必须按平台区分，互不干扰：
///
/// - **Windows**：crossterm 直接报告“原始字符 + CONTROL”（如 Ctrl+\ →
///   `Char('\\')`、Ctrl+/ → `Char('/')`），按 nano 原语义 `c & 0x1F` 编码。
/// - **Unix/Linux**：终端先把手按的键转成控制字节（Ctrl+\ → 0x1C、
///   Ctrl+/ → 0x1F），crossterm 再把字节重新编码为
///   `Char('a'..'z')`（0x01-0x1A）、`Char('4'..'7')`（0x1C-0x1F）、
///   `Char(' ')`（0x00）+ CONTROL；这里须反向解码回原始字节。
///   若终端启用了 kitty 键盘协议（CSI-u），则直接收到精确字符
///   （如 `Char('/')`），同样按原语义编码。
#[cfg(unix)]
fn ctrl_char_code(c: char) -> i32 {
    match c {
        // crossterm unix：0x01-0x1A（Ctrl+A..Ctrl+Z）→ 'a'..='z' + CONTROL。
        'a'..='z' => (c as u8 - b'a' + 1) as i32,
        // crossterm unix：0x1C-0x1F（Ctrl+\ ] ^ _ /）→ '4'..='7' + CONTROL。
        '4'..='7' => (c as u8 - b'4' + 0x1C) as i32,
        // crossterm unix：0x00（Ctrl+@ / Ctrl+2 / Ctrl+Space）→ ' ' + CONTROL。
        ' ' => 0,
        // kitty 协议下的精确 Ctrl+/：终端发送 0x1F（与 Ctrl+_ 相同，对应
        // Go To Line 功能），而 '/' & 0x1F 会错误地得到 15（Ctrl+O）。
        '/' => 0x1F,
        // 其余 ASCII（如 kitty 协议下的 Ctrl+\、Ctrl+[ 等）：按原语义编码。
        c if c.is_ascii() => (c as u8 & 0x1F) as i32,
        _ => c as i32,
    }
}

#[cfg(not(unix))]
fn ctrl_char_code(c: char) -> i32 {
    // Windows：crossterm 报告原始字符 + CONTROL，按 nano 原语义 c & 0x1F。
    // 例如 Ctrl+A → 1, Ctrl+\ → 28, Ctrl+[ → 27 (等价 ESC)。
    // 特例：Ctrl+/ 在终端上发送 0x1F（与 Ctrl+_ 相同，对应 Go To Line
    // 功能），而 '/' & 0x1F 会错误地得到 15（Ctrl+O）。
    if c == '/' {
        0x1F
    } else if c.is_ascii() {
        (c as u8 & 0x1F) as i32
    } else {
        c as i32
    }
}

/// 将 crossterm KeyEvent 转换为 nano 键码。
pub fn translate_keycode(key: KeyEvent) -> i32 {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers == KeyModifiers::CONTROL {
                // Ctrl + 字符：按平台各自的编码规则转换为 nano 键码。
                ctrl_char_code(c)
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
        KeyCode::Enter => 13, /* 主键盘 Enter 对应 '\r'（与 C 的 wgetch 一致） */
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
/// 返回是否有等待中的按键（非阻塞；对应 C 的 `waiting_codes` 计数）。
pub fn waiting_keycodes() -> i32 {
    if event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        1
    } else {
        0
    }
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

/// 当前行号边距（对应 C 的 confirm_margin：digits(filebot->lineno) + 1）。
/// 未开启行号或边距超过 COLS-4 时返回 0。
pub fn current_margin() -> usize {
    if !ISSET(LINE_NUMBERS) {
        return 0;
    }
    with_global(|g| {
        let cols = g.COLS;
        let lineno = g
            .openfile
            .as_ref()
            .and_then(|of| of.borrow().filebot.clone())
            .map(|b| b.borrow().lineno)
            .unwrap_or(1)
            .max(1);
        let needed = lineno.to_string().len() + 1;
        if needed > cols.saturating_sub(4) {
            0
        } else {
            needed
        }
    })
}

/// 确认行号边距：当缓冲区最高行号所需的边距变化时更新全局
/// margin / editwincols，并安排全刷新（对应 nano.c 的 `confirm_margin`）。
pub fn confirm_margin() {
    let needed = current_margin() as i32;
    with_global_mut(|g| {
        if needed != g.margin {
            let keep_focus = g.margin > 0 && g.focusing;
            g.margin = needed;
            g.editwincols = g
                .COLS
                .saturating_sub(needed as usize)
                .saturating_sub(if g.sidebar { 1 } else { 0 });
            g.focusing = keep_focus;
            /* 边距变化——安排全刷新。 */
            g.refresh_needed = true;
        }
    });
}

/// 显示只读模式警告（对应 nano.c 的 `print_view_warning`）。
pub fn print_view_warning() {
    statusline(MessageType::Ahem, &crate::t!("winio-view_warning"));
}

/// 绘制极简状态栏（对应 winio.c 的 `minibar`）：在底部信息行显示
/// 文件名、修改标记、光标行/列和行号百分比。
/// 仅当 MINIBAR 模式、非 ZERO、且行数足够时由主循环调用。
pub fn minibar() {
    if !ISSET(MINIBAR) || ISSET(ZERO) {
        return;
    }
    let (cols, lines) = with_global(|g| (g.COLS, g.LINES));
    if lines <= 1 {
        return;
    }

    let mut stdout = io::stdout();
    let status_row = lines.saturating_sub(3) as u16;

    /* 用 MINI_INFOBAR 颜色对绘制整条底色条。 */
    let pair = color::interface_color_pair(MINI_INFOBAR);
    apply_attributes(&mut stdout, pair);
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    let _ = write!(stdout, "{:width$}", "", width = cols);

    let (filename, modified, lineno, filebot_lineno, column, has_anchor) = with_global(|g| {
        let of = g.openfile.as_ref().map(|o| o.borrow());
        let filename = of
            .as_ref()
            .and_then(|r| r.filename.clone())
            .unwrap_or_default();
        let filename = if filename.is_empty() {
            crate::t!("winio-nameless")
        } else {
            filename
        };
        (
            filename,
            of.as_ref().map(|r| r.modified).unwrap_or(false),
            of.as_ref().and_then(|r| r.current.as_ref()).map(|c| c.borrow().lineno).unwrap_or(1),
            of.as_ref().and_then(|r| r.filebot.as_ref()).map(|b| b.borrow().lineno).unwrap_or(1),
            of.as_ref().map(|r| r.current_x + 1).unwrap_or(1),
            of.as_ref().and_then(|r| r.current.as_ref()).map(|c| c.borrow().has_anchor).unwrap_or(false),
        )
    });

    /* 文件名（过长时省略号截断）加修改星号。 */
    let mut left = String::new();
    if cols > 4 {
        let name_max = cols.saturating_sub(10);
        if utils::breadth(filename.as_bytes()) > name_max {
            let start = utils::actual_x(filename.as_bytes(), name_max.saturating_sub(8));
            left.push_str("...");
            left.push_str(&filename[start..]);
        } else {
            left.push_str(&filename);
        }
        left.push_str(if modified { " *" } else { "  " });
    }

    /* 右侧：行号百分比。 */
    let pct = if filebot_lineno > 0 {
        100 * lineno / filebot_lineno
    } else {
        0
    };
    let right = format!("{:3}%", pct);
    let right_len = utils::breadth(right.as_bytes());

    /* 光标位置（行,列）尽量显示。 */
    let location = format!("{},{}", lineno, column);
    let loc_len = utils::breadth(location.as_bytes());
    let anchor_mark = if has_anchor { "†" } else { "" };

    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    let mut col = 0usize;
    let _ = write!(stdout, "{}", left);
    col += utils::breadth(left.as_bytes());

    /* 行/列位置：放在中间偏右。 */
    if col + loc_len + right_len + 6 < cols {
        let _ = execute!(stdout, cursor::MoveTo((cols - right_len - loc_len - 6).min(cols.saturating_sub(1)) as u16, status_row));
        let _ = write!(stdout, "{}", location);
    }

    /* 锚点标记。 */
    if col + 6 < cols && !anchor_mark.is_empty() {
        let _ = execute!(stdout, cursor::MoveTo(cols.saturating_sub(right_len + 5) as u16, status_row));
        let _ = write!(stdout, "{}", anchor_mark);
    }

    /* 百分比（最右）。 */
    let _ = execute!(stdout, cursor::MoveTo(cols.saturating_sub(right_len + 1) as u16, status_row));
    let _ = write!(stdout, "{}", right);

    reset_attributes(&mut stdout);
    let _ = stdout.flush();
}

/// 刷新屏幕（逐行覆盖重绘，避免全屏 Clear 造成的闪烁）。
pub fn refresh_screen() {
    let mut stdout = io::stdout();
    let _margin = current_margin();

    let (cols, lines, edit_rows) =
        with_global(|g| (g.COLS, g.LINES, g.LINES.saturating_sub(4)));

    // 绘制标题栏（屏幕顶部第0行）
    let _ = execute!(stdout, cursor::MoveTo(0, 0));
    draw_titlebar_line(&mut stdout, cols);

    // 绘制编辑区域（从第1行开始）：逐行调用 update_line（含语法高亮）。
    let (edittop, current, current_x) = with_global(|g| match &g.openfile {
        Some(of) => {
            let o = of.borrow();
            (o.edittop.clone(), o.current.clone(), o.current_x)
        }
        None => (None, None, 0),
    });

    if let Some(edittop) = edittop {
        let mut cur: Option<LineRef> = Some(edittop);
        let mut row = 0i32;
        while let Some(c) = cur {
            if row >= edit_rows as i32 {
                break;
            }
            let x = if current.as_ref().map(|cc| Rc::ptr_eq(cc, &c)).unwrap_or(false) {
                current_x
            } else {
                0
            };
            /* update_line 返回该行消耗的屏幕行数（软换行时为块数）。 */
            row += update_line(&c, x);
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        // 清空剩余编辑行
        while row < edit_rows as i32 {
            let _ = execute!(stdout, cursor::MoveTo(0, 1 + row as u16));
            let _ = execute!(stdout, Clear(ClearType::UntilNewLine));
            row += 1;
        }
    } else {
        for row in 0..edit_rows as u16 {
            let _ = execute!(stdout, cursor::MoveTo(0, 1 + row));
            let _ = execute!(stdout, Clear(ClearType::UntilNewLine));
        }
    }

    // 绘制状态栏（倒数第3行）
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    draw_statusbar_line(&mut stdout, cols);

    // 绘制底部快捷键（倒数第2行和倒数第1行）
    draw_bottombars_lines(&mut stdout, cols, lines);

    let _ = stdout.flush();

    /* 刷新后恢复光标：显示并移到编辑位置。 */
    let _ = execute!(stdout, Show);
    place_the_cursor();
    let _ = stdout.flush();
}

/// 绘制标题栏行（格式参照 C 版 titlebar）。
fn draw_titlebar_line(stdout: &mut io::Stdout, cols: usize) {
    with_global(|g| {
        // C 版 titlebar: openfile->filename[0] == '\0' 时显示 "New Buffer"。
        // Rust 版 filename 为 Some("")（空串）表示"无文件名"，须同样回退。
        let filename = g.openfile.as_ref()
            .and_then(|of| of.borrow().filename.clone())
            .unwrap_or_default();
        let filename = if filename.is_empty() {
            crate::t!("winio-new_buffer")
        } else {
            filename
        };
        let modified = g.openfile.as_ref()
            .map(|of| of.borrow().modified)
            .unwrap_or(false);
        let state = if modified { &crate::t!("winio-modified") } else { "" };
        let left_text = format!(" nax {} ", env!("CARGO_PKG_VERSION"));
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

/// 把消息按显示宽度截断到 cols 列（ASCII 占 1 列，其他如中文占 2 列），
/// 返回截断后的文本及其显示宽度。
fn clip_to_width(msg: &str, cols: usize) -> (String, usize) {
    let mut clipped = String::new();
    let mut width = 0usize;
    for ch in msg.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if width + w > cols {
            break;
        }
        clipped.push(ch);
        width += w;
    }
    (clipped, width)
}

/// 绘制状态栏行（先清空整行再写入，避免短消息覆盖长消息时残留）。
fn draw_statusbar_line(stdout: &mut io::Stdout, cols: usize) {
    with_global(|g| {
        let msg = &g.statusbar_msg;
        let centered = g.statusbar_centered;

        /* 先清空整行。 */
        let _ = write!(stdout, "{:width$}", "", width = cols);
        if msg.is_empty() {
            return;
        }

        let (clipped, width) = clip_to_width(msg, cols);
        if centered {
            let pad = cols.saturating_sub(width) / 2;
            let _ = write!(stdout, "{:width$}{}", "", clipped, width = pad);
        } else {
            let _ = write!(stdout, "{}", clipped);
        }
    });
}

/// 在底部快捷键栏绘制单个"键 + 说明"条目（对应 C 版 `post_one_key`）。
/// 键串按显示宽度截断到 width；剩余空间不足 2 列时省略说明文字。
fn post_one_key(
    stdout: &mut io::Stdout,
    row: u16,
    col: u16,
    keystroke: &str,
    tag: &str,
    width: usize,
) {
    let _ = execute!(stdout, cursor::MoveTo(col, row));

    /* 键串本身截断到 width。 */
    let mut key_part: String = String::new();
    let mut key_width = 0usize;
    for ch in keystroke.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if key_width + w > width {
            break;
        }
        key_part.push(ch);
        key_width += w;
    }
    let _ = write!(stdout, "{}", key_part);

    /* 剩余空间太小则省略说明。 */
    if width < key_width + 2 {
        return;
    }
    let _ = write!(stdout, " ");

    /* 说明文字截断到剩余宽度。 */
    let tag_max = width - key_width - 1;
    let mut tag_part: String = String::new();
    let mut tag_width = 0usize;
    for ch in tag.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if tag_width + w > tag_max {
            break;
        }
        tag_part.push(ch);
        tag_width += w;
    }
    let _ = write!(stdout, "{}", tag_part);
}

/// 绘制底部快捷键（两行，参照 C 版 bottombars 实现）。
fn draw_bottombars_lines(stdout: &mut io::Stdout, cols: usize, lines: usize) {
    with_global(|g| {
        let menu = g.currmenu;

        /* MYESNO 菜单（Yes/No 询问）：与 C 版 ask_user 一致，手动绘制
         * "Y Yes"、"N No" 与 "^C Cancel"。All 场景（替换确认）暂不在
         * 快捷键栏显示 "A All"，但 A 键的应答逻辑仍可用。 */
        if menu == MYESNO {
            let mut width = 16;
            if cols < 32 {
                width = cols / 2;
            }
            let bottom_row1 = (lines.saturating_sub(2)) as u16;
            let bottom_row2 = (lines.saturating_sub(1)) as u16;

            /* 先清空两行，避免旧快捷键残影。 */
            let _ = execute!(stdout, cursor::MoveTo(0, bottom_row1));
            let _ = write!(stdout, "{:width$}", "", width = cols);
            let _ = execute!(stdout, cursor::MoveTo(0, bottom_row2));
            let _ = write!(stdout, "{:width$}", "", width = cols);

            post_one_key(stdout, bottom_row1, 0, " Y", &crate::t!("key-yes"), width);
            post_one_key(stdout, bottom_row2, 0, " N", &crate::t!("key-no"), width);
            post_one_key(stdout, bottom_row2, width as u16, "^C", &crate::t!("key-cancel"), width);
            return;
        }

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
                        entries.push((s_ref.keystr.clone(), f_ref.tag.clone()));
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

/// 在状态栏写入消息（status_row 行）；centered 时水平居中。
/// 先清空整行并按显示宽度截断，避免短消息覆盖长消息时旧文本残留、
/// 或中文内容按字符数 pad 后超出终端宽度折行。
fn write_statusbar_impl(msg: &str, centered: bool) {
    let mut stdout = io::stdout();
    let (lines, cols) = with_global(|g| (g.LINES, g.COLS));
    let status_row = (lines.saturating_sub(3)) as u16;
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));
    /* 先清空整行。 */
    let _ = write!(stdout, "{:width$}", "", width = cols);
    let _ = execute!(stdout, cursor::MoveTo(0, status_row));

    if msg.is_empty() {
        let _ = stdout.flush();
        return;
    }

    let (clipped, width) = clip_to_width(msg, cols);
    if centered {
        let pad = cols.saturating_sub(width) / 2;
        let _ = write!(stdout, "{:width$}", "", width = pad);
    }
    let _ = write!(stdout, "{}", clipped);
    let _ = stdout.flush();
}

/// 在状态栏显示消息（左对齐）。
pub fn statusbar(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
        g.statusbar_msg = msg.to_string();
        g.statusbar_centered = false;
    });
    write_statusbar_impl(msg, false);
}

/// 在状态栏居中显示消息。
pub fn statusbar_centered(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
        g.statusbar_msg = msg.to_string();
        g.statusbar_centered = true;
    });
    write_statusbar_impl(msg, true);
}

/// 在状态行显示消息（左对齐）。
pub fn statusline(typ: MessageType, msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = typ;
        g.statusbar_msg = msg.to_string();
        g.statusbar_centered = false;
    });
    write_statusbar_impl(msg, false);
}

/// 在状态行居中显示消息。
pub fn statusline_centered(typ: MessageType, msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = typ;
        g.statusbar_msg = msg.to_string();
        g.statusbar_centered = true;
    });
    write_statusbar_impl(msg, true);
}

/// 在指定位置显示文本。
pub fn mvwaddstr(_win: bool, _row: i32, _col: i32, _text: &str) {
    // 简化
}

/// 清除状态栏。
pub fn wipe_statusbar() {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Vacuum;
        g.statusbar_msg.clear();
        g.statusbar_centered = false;
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
/// 在底部两行显示指定菜单的快捷键，并把当前菜单设为该菜单
/// （对应 C 的 `bottombars(menu)`，内部先 `currmenu = menu`）。
pub fn bottombars(menu: i32) {
    with_global_mut(|g| g.currmenu = menu);
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
/// 屏幕列按 C 版换算：非软换行减去视口左缘列（`get_page_start`），
/// 软换行减去当前块的最左列并把行号累计为"含块的行数"（详见
/// `cursor_screen_position`）。
pub fn place_the_cursor() {
    let editwinrows = with_global(|g| g.editwinrows);
    let margin = current_margin();
    let (cur, edittop, current_x, firstcolumn) = with_global(|g| {
        if let Some(of) = &g.openfile {
            let of = of.borrow();
            if let (Some(c), Some(e)) = (&of.current, &of.edittop) {
                return (Some(c.clone()), Some(e.clone()), of.current_x, of.firstcolumn);
            }
        }
        (None, None, 0, 0)
    });
    if let (Some(cur), Some(edittop)) = (cur, edittop) {
        let (row, screen_column) = cursor_screen_position(&cur, &edittop, current_x, firstcolumn);
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                /* 钳位到 0：软换行下若光标列落在视口左缘（firstcolumn）
                 * 所在块之前（被滚出屏），row 累计可为负；负值会被
                 * adjust_viewport 的 STATIONARY 分支当作 goal 传入
                 * go_back_chunks，导致向错误方向滚动。正常路径下光标
                 * 总在屏内（row >= 0），钳位仅防御该异常状态。 */
                of.borrow_mut().cursor_row = row.max(0);
            }
        });
        if row >= 0 && row < editwinrows as isize {
            let mut stdout = io::stdout();
            let _ = execute!(
                stdout,
                cursor::MoveTo((screen_column + margin) as u16, (row + 1) as u16)
            );
        }
    }
}

/// 计算光标应处的屏幕行与列（编辑区相对坐标，行 0 对应编辑区首行）。
///
/// 注意：本函数内部会调用 `with_global`（经 ISSET/`get_page_start`/
/// `chunk_for` 等），因此**必须在 `with_global`/`with_global_mut`
/// 闭包之外调用**，否则会触发 RefCell 双重借用 panic。
///
/// 对齐 C 版 `place_the_cursor` 的换算：超长行时光标的绝对显示列
/// （`wideness(current->data, current_x)`）必须折算为屏幕内相对列——
/// 非软换行减去视口左缘列（`get_page_start`），软换行减去当前块的最
/// 左列（leftedge）；软换行下行号还要累计 edittop 到当前行之间每行的
/// "1 + 额外块数"以及当前行内光标之前的块数。若不折算，光标列超过
/// 屏幕宽度后会被直接移出终端（表现为"移动到某个位置就停住"）。
fn cursor_screen_position(
    current: &LineRef,
    edittop: &LineRef,
    current_x: usize,
    firstcolumn: usize,
) -> (isize, usize) {
    let column = crate::utils::wideness(current.borrow().data.as_bytes(), current_x);

    let row: isize;
    let screen_column: usize;
    if ISSET(SOFTWRAP) {
        /* edittop 上方被滚出屏的块数（取负），加上从 edittop 到当前行
         * 之间各行的显示行数（每行 1 + 额外块数），再加上当前行中光标
         * 之前的块数。 */
        let mut row_acc: isize = -(chunk_for(firstcolumn, edittop) as isize);
        let mut line: Option<LineRef> = Some(edittop.clone());
        while let Some(l) = line {
            if Rc::ptr_eq(&l, current) {
                break;
            }
            row_acc += 1 + extra_chunks_in(&l) as isize;
            let next = { let r = l.borrow(); r.next.clone() };
            line = next;
        }
        let mut leftedge = 0usize;
        row_acc += get_chunk_and_edge(column, current, Some(&mut leftedge)) as isize;
        row = row_acc;
        screen_column = column.saturating_sub(leftedge);
    } else {
        row = current.borrow().lineno as isize - edittop.borrow().lineno as isize;
        screen_column = column.saturating_sub(crate::utils::get_page_start(column));
    }

    (row, screen_column)
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

// `get_softwrap_breakpoint` 的静态状态（对应 C 的 static text/column）。
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
    let menu = with_global(|g| g.currmenu);
    bottombars(menu);
}

/// 滚动方向枚举（对应 winio.c 的 `update_type` 中 FORWARD/BACKWARD 用法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Forward,
    Backward,
}

/// 单行扫描的最大匹配数（对应 PAINT_LIMIT）。
const PAINT_LIMIT: usize = 2000;

/// 把属性（颜色对 + 样式位）应用到 stdout。
fn apply_attributes(stdout: &mut io::Stdout, attrs: i32) {
    let pair = color::pairnum_from(attrs);
    if pair > 0 {
        if let Some((fg, bg)) = color::lookup_pair(pair) {
            let _ = execute!(
                stdout,
                SetForegroundColor(color::nano_to_crossterm_color(fg)),
                SetBackgroundColor(color::nano_to_crossterm_color(bg))
            );
        }
    }
    if attrs & color::A_BOLD != 0 {
        let _ = execute!(stdout, SetAttribute(Attribute::Bold));
    }
    if attrs & color::A_UNDERLINE != 0 {
        let _ = execute!(stdout, SetAttribute(Attribute::Underlined));
    }
    if attrs & color::A_REVERSE != 0 {
        let _ = execute!(stdout, SetAttribute(Attribute::Reverse));
    }
}

/// 重置样式与颜色。
fn reset_attributes(stdout: &mut io::Stdout) {
    let _ = execute!(stdout, SetAttribute(Attribute::Reset));
}

/// 在 row 行绘制一行 converted 文本，并应用语法高亮（对应 C 的 `draw_row`）。
/// 对当前行上匹配到的搜索字符串进行聚光高亮（对应 winio.c 的 spotlight）。
/// 仅在 spotlighted 已设置且该行是当前行时生效。
/// consume 为 TRUE 时画完即清除 spotlight
/// 标志（单行、每行一次）；软换行多块绘制时传 FALSE，由调用方统
/// 一清除，避免仅第一块被高亮。
fn spotlight_line(
    stdout: &mut io::Stdout,
    row: u16,
    converted: &[u8],
    line: &LineRef,
    from_col: usize,
    margin: usize,
    consume: bool,
) {
    let (spotlighted, light_from_col, light_to_col) = with_global(|g| (g.spotlighted, g.light_from_col, g.light_to_col));
    if !spotlighted {
        return;
    }

    if !is_current_line(line) {
        return;
    }

    if consume {
        /* 与 C 版一致：绘制后清除 spotlight，避免重复绘制（draw_row 会在
         * paint_syntax 后调用；C 版在 redraw_line 中置 FALSE）。 */
        with_global_mut(|g| g.spotlighted = false);
    }

    /* light_from_col/light_to_col 是绝对列号；转换为相对屏幕行（converted）的列号。 */
    let conv_start_col = light_from_col.saturating_sub(from_col);
    let conv_end_col = light_to_col.saturating_sub(from_col);
    if conv_start_col >= conv_end_col || conv_start_col >= converted.len() {
        return;
    }

    /* 把列号转成 converted 里的字节偏移，得到要绘制的字节范围。 */
    let start_x = crate::utils::actual_x(converted, conv_start_col).min(converted.len());
    let end_x = crate::utils::actual_x(converted, conv_end_col).min(converted.len());
    if start_x >= end_x {
        return;
    }
    let paint_conv = &converted[start_x..end_x];
    if paint_conv.is_empty() {
        return;
    }

    let color = with_global(|g| *g.interface_color_pair.get(SPOTLIGHTED).unwrap_or(&0));
    if color == 0 {
        return;
    }

    let _ = execute!(stdout, cursor::MoveTo((margin + conv_start_col) as u16, row));
    apply_attributes(stdout, color);
    let _ = write!(stdout, "{}", String::from_utf8_lossy(paint_conv));
    reset_attributes(stdout);
}

fn draw_row(
    stdout: &mut io::Stdout,
    row: u16,
    converted: &[u8],
    line: &LineRef,
    from_col: usize,
    till_x: usize,
) {
    let margin = current_margin();

    let (ln, syntax, linenum_color) = with_global(|g| {
        let of = g.openfile.as_ref();
        let ln = line.borrow().lineno;
        let syntax = of.and_then(|o| o.borrow().syntax.clone());
        let linenum_color = g.interface_color_pair.get(LINE_NUMBER).copied().unwrap_or(0);
        (ln, syntax, linenum_color)
    });

    /* 行号。 */
    if margin > 0 {
        let _ = execute!(stdout, cursor::MoveTo(0, row));
        apply_attributes(stdout, linenum_color);
        /* 软换行的后续块不重复显示行号，只留空白（对应 C 版
         * draw_row 中 "%*s" 分支）。 */
        if ISSET(SOFTWRAP) && from_col != 0 {
            let _ = write!(stdout, "{:>width$} ", "", width = margin - 1);
        } else {
            let _ = write!(stdout, "{:>width$} ", ln, width = margin - 1);
        }
        reset_attributes(stdout);
    }

    /* 正文。 */
    let _ = execute!(stdout, cursor::MoveTo(margin as u16, row));
    let _ = write!(stdout, "{}", String::from_utf8_lossy(converted));
    let _ = execute!(stdout, Clear(ClearType::UntilNewLine));

    /* 语法高亮。 */
    if let Some(sntx) = syntax {
        if !ISSET(NO_SYNTAX) {
            paint_syntax_rules(stdout, row, converted, line, from_col, till_x, &sntx, margin);
        }
    }
    /* 聚光高亮由调用方 update_line / update_softwrapped_line 在处理完
     * 各自的行（或全部软换行块）后调用 spotlight_line 绘制。 */
}

/// 应用当前语法的全部颜色规则到一行（对应 C 的 draw_row 中 ENABLE_COLOR 部分）。
/// till_x 是本行/块显示到的结束列（非软换行时为 from_col + 行宽）。
fn paint_syntax_rules(
    stdout: &mut io::Stdout,
    row: u16,
    converted: &[u8],
    line: &LineRef,
    from_col: usize,
    till_x: usize,
    sntx: &SyntaxRef,
    margin: usize,
) {
    let from_x = from_col;
    let data = line.borrow().data.clone();
    let data_bytes = data.as_bytes();

    /* 多行正则需要逐行缓存。 */
    let multiscore = sntx.borrow().multiscore;
    if multiscore > 0 && line.borrow().multidata.is_none() {
        line.borrow_mut().multidata = Some(vec![0; multiscore as usize]);
    }

    /* 收集颜色规则（避免借用冲突）。 */
    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = sntx.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    for ink in &colors {
        let (attrs, id, is_multiline, start_pat, end_pat) = {
            let r = ink.borrow();
            (
                r.attributes,
                r.id,
                r.end.is_some(),
                r.start.clone(),
                r.end.clone(),
            )
        };
        let Some(start_pat) = start_pat else { continue };

        /* 单行规则：循环匹配 start。 */
        if !is_multiline {
            let mut index = 0usize;
            while index < PAINT_LIMIT && index < till_x {
                let (so, eo) = match start_pat.find_from(data_bytes, index, index != 0) {
                    Some(m) => m,
                    None => break,
                };
                index = eo;
                if so >= till_x {
                    break;
                }
                /* 零宽匹配：前进。 */
                if so == eo {
                    if data_bytes.get(index).copied().unwrap_or(0) == 0 {
                        break;
                    }
                    index = chars::step_right(data_bytes, index);
                    continue;
                }
                if eo <= from_x {
                    continue;
                }
                let start_col = if so > from_x {
                    crate::utils::wideness(data_bytes, so) - from_col
                } else {
                    0
                };
                let the_start = crate::utils::actual_x(converted, start_col).min(converted.len());
                let thetext = &converted[the_start..];
                let paintlen = crate::utils::actual_x(
                    thetext,
                    crate::utils::wideness(data_bytes, eo)
                        .saturating_sub(from_col)
                        .saturating_sub(start_col),
                );
                let _ = execute!(stdout, cursor::MoveTo((margin + start_col) as u16, row));
                apply_attributes(stdout, attrs);
                let _ = write!(stdout, "{}", String::from_utf8_lossy(&thetext[..paintlen.min(thetext.len())]));
                reset_attributes(stdout);
            }
            continue;
        }

        /* 多行规则。 */
        let prior_state = {
            let prev = line.borrow().prev.as_ref().and_then(|w| w.upgrade());
            match prev {
                Some(p) => p
                    .borrow()
                    .multidata
                    .clone()
                    .and_then(|m| m.get(id as usize).copied()),
                None => None,
            }
        };

        /* 假定本行初始不适用。 */
        if let Some(md) = line.borrow_mut().multidata.as_mut() {
            md[id as usize] = NOTHING as i16;
        }

        let mut index = 0usize;

        /* 前一行有未闭合的 start。 */
        if prior_state == Some(WHOLELINE as i16) || prior_state == Some(STARTSHERE as i16) {
            let endmatch = end_pat.as_ref().and_then(|e| e.find_from(data_bytes, 0, false));
            if endmatch.is_none() {
                /* 无 end：整行着色。 */
                let _ = execute!(stdout, cursor::MoveTo(margin as u16, row));
                apply_attributes(stdout, attrs);
                let _ = write!(stdout, "{}", String::from_utf8_lossy(converted));
                reset_attributes(stdout);
                if let Some(md) = line.borrow_mut().multidata.as_mut() {
                    md[id as usize] = WHOLELINE as i16;
                }
                continue;
            }
            let (_, end_eo) = endmatch.unwrap();
            if end_eo > from_x {
                let paintlen = crate::utils::actual_x(
                    converted,
                    crate::utils::wideness(data_bytes, end_eo).saturating_sub(from_col),
                )
                .min(converted.len());
                let _ = execute!(stdout, cursor::MoveTo(margin as u16, row));
                apply_attributes(stdout, attrs);
                let _ = write!(stdout, "{}", String::from_utf8_lossy(&converted[..paintlen]));
                reset_attributes(stdout);
            }
            if let Some(md) = line.borrow_mut().multidata.as_mut() {
                md[id as usize] = ENDSHERE as i16;
            }
            index = end_eo;
        }

        /* 在本行寻找 start 匹配。 */
        while index < PAINT_LIMIT {
            let (start_so, start_eo) = match start_pat.find_from(data_bytes, index, index != 0) {
                Some(m) => m,
                None => break,
            };
            if start_so >= till_x {
                break;
            }

            let start_col = if start_so > from_x {
                crate::utils::wideness(data_bytes, start_so) - from_col
            } else {
                0
            };
            let the_start = crate::utils::actual_x(converted, start_col).min(converted.len());
            let thetext = &converted[the_start..];

            /* 同一行有 end 匹配。 */
            let endmatch = end_pat.as_ref().and_then(|e| e.find_from(data_bytes, start_eo, start_eo != 0));
            if let Some((_, end_eo)) = endmatch {
                if end_eo > from_x && end_eo > start_so {
                    let paintlen = crate::utils::actual_x(
                        thetext,
                        crate::utils::wideness(data_bytes, end_eo)
                            .saturating_sub(from_col)
                            .saturating_sub(start_col),
                    );
                    let _ = execute!(stdout, cursor::MoveTo((margin + start_col) as u16, row));
                    apply_attributes(stdout, attrs);
                    let _ = write!(stdout, "{}", String::from_utf8_lossy(&thetext[..paintlen.min(thetext.len())]));
                    reset_attributes(stdout);
                    if let Some(md) = line.borrow_mut().multidata.as_mut() {
                        md[id as usize] = JUSTONTHIS as i16;
                    }
                }
                index = end_eo;
                /* 若 start 与 end 都是零宽，强制前进。 */
                if start_so == start_eo && end_eo == end_eo && data_bytes.get(index).copied().unwrap_or(0) == 0 {
                    break;
                }
                if start_so == start_eo {
                    index = chars::step_right(data_bytes, index);
                }
                continue;
            }

            /* 无 end：剩余部分着色，标记 STARTSHERE。 */
            let _ = execute!(stdout, cursor::MoveTo((margin + start_col) as u16, row));
            apply_attributes(stdout, attrs);
            let _ = write!(stdout, "{}", String::from_utf8_lossy(thetext));
            reset_attributes(stdout);
            if let Some(md) = line.borrow_mut().multidata.as_mut() {
                md[id as usize] = STARTSHERE as i16;
            }
            break;
        }
    }
}

/// 重绘给定行（对应 winio.c 的 `update_line`）。
/// 返回该行占用的行数（软换行时为块数，否则为 1）。
pub fn update_line(line: &LineRef, index: usize) -> i32 {
    if ISSET(SOFTWRAP) {
        return update_softwrapped_line(line);
    }

    let mut stdout = io::stdout();
    let margin = current_margin();
    let data = line.borrow().data.clone();

    let from_col = crate::utils::get_page_start(crate::utils::wideness(data.as_bytes(), index));
    let (cols, span) = with_global(|g| (g.COLS, g.COLS.saturating_sub(margin + 1)));
    let (converted, has_more) = display_string(data.as_bytes(), from_col, span, true, false);

    /* 目标行号 = line.lineno - edittop.lineno（+1 因为编辑区从第 1 行开始）。 */
    let row = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let edittop_lineno = of.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(1);
        line.borrow().lineno - edittop_lineno
    });
    let row = (1 + row) as u16;

    draw_row(&mut stdout, row, converted.as_bytes(), line, from_col, from_col + span);

    /* 聚光高亮（对应 edit_draw 中 line == current 时的 spotlight 调用）。
     * 单行绘制：画完即清除标志。 */
    spotlight_line(&mut stdout, row, converted.as_bytes(), line, from_col, margin, true);

    /* 超长行的截断标记：左边有内容被滚动出时画 '<'，右边还有内容
     * 未显示时画 '>'（对应 edit_draw 中基于 from_col 与 has_more 的
     * 两处 waddch；颜色用 hilite_attribute）。 */
    let hilite = with_global(|g| g.hilite_attribute);
    if from_col > 0 && !converted.is_empty() {
        let _ = execute!(stdout, cursor::MoveTo(margin as u16, row));
        apply_attributes(&mut stdout, hilite);
        let _ = write!(stdout, "<");
        reset_attributes(&mut stdout);
    }
    if has_more {
        let _ = execute!(stdout, cursor::MoveTo(cols.saturating_sub(1) as u16, row));
        apply_attributes(&mut stdout, hilite);
        let _ = write!(stdout, ">");
        reset_attributes(&mut stdout);
    }

    let _ = stdout.flush();
    1
}

/// 软换行模式下重绘给定行的全部块（对应 winio.c 的
/// `update_softwrapped_line`）。除非该行是 edittop（此时从列
/// firstcolumn 开始显示），否则显示整行。返回占用的屏幕行数。
fn update_softwrapped_line(line: &LineRef) -> i32 {
    /* 编辑器显示区域的行数（对应 editwinrows）。 */
    let editwinrows = with_global(|g| g.LINES.saturating_sub(4) as i32);

    let (edittop, firstcolumn) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        (of.edittop.clone().expect("no edittop"), of.firstcolumn)
    });
    let is_edittop = Rc::ptr_eq(&edittop, line);

    let mut screen_row: i32 = 0;
    let mut from_col = 0usize;
    if is_edittop {
        from_col = firstcolumn;
    } else {
        /* edittop 的第一块可能在屏幕上方的滚动区域之外。 */
        screen_row -= chunk_for(firstcolumn, &edittop) as i32;
    }

    /* 找出目标行应显示在哪个屏幕行。 */
    let mut someline: Option<LineRef> = Some(edittop);
    while let Some(s) = someline {
        if Rc::ptr_eq(&s, line) {
            break;
        }
        screen_row += 1 + extra_chunks_in(&s) as i32;
        let next = { let r = s.borrow(); r.next.clone() };
        someline = next;
    }

    /* 第一块在屏幕外：不显示。 */
    if screen_row < 0 || screen_row >= editwinrows {
        return 0;
    }
    let starting_row = screen_row;

    let mut stdout = io::stdout();
    let mut kickoff = true;
    let mut end_of_line = false;
    let data = line.borrow().data.clone();

    /* 逐块转换并绘制。 */
    while !end_of_line && screen_row < editwinrows {
        let to_col = get_softwrap_breakpoint(data.as_bytes(), from_col, &mut kickoff, &mut end_of_line);

        /* 进度守卫：editwincols<=1 的退化情形下断点可能不前进，避免死循环。 */
        if to_col <= from_col {
            break;
        }

        let (converted, _has_more) = display_string(data.as_bytes(), from_col, to_col - from_col, true, false);
        draw_row(&mut stdout, (1 + screen_row) as u16, converted.as_bytes(), line, from_col, to_col);
        /* 聚光高亮逐块绘制；consume 传 FALSE，避免第一块画完就清掉
         * 标志导致后续块不高亮。 */
        spotlight_line(&mut stdout, (1 + screen_row) as u16, converted.as_bytes(), line, from_col, current_margin(), false);

        from_col = to_col;
        screen_row += 1;
    }

    /* 软换行下聚光高亮跨多个块：块循环中不清除标志，当前行画完后
     * 统一清除（单行路径的 clearing 在 spotlight_line(consume=TRUE) 中）。 */
    if is_current_line(line) {
        with_global_mut(|g| g.spotlighted = false);
    }

    screen_row - starting_row
}

/// 判断给定行是否为当前行。
fn is_current_line(line: &LineRef) -> bool {
    with_global(|g| {
        g.openfile.as_ref()
            .and_then(|of| of.borrow().current.clone())
            .map(|c| Rc::ptr_eq(&c, line))
            .unwrap_or(false)
    })
}

// ======================== 字符串显示与逐字输入（对应 winio.c） ========================

/// 将给定文本转换为可在终端显示的字符串：控制字符显示为 ^X，
/// 制表符展开为空格，宽字符保留，零宽字符处理等。
/// column 是起始列，span 是可用宽度（对应 `display_string`）。
/// 返回 (转换后的字符串, has_more)——has_more 表示右侧还有内容未显示，
/// 调用方应画 ">" 截断标记（对应 C 版返回后的全局 has_more）。
pub fn display_string(text: &[u8], column: usize, span: usize, isdata: bool, isprompt: bool) -> (String, bool) {
    let start_x = crate::utils::actual_x(text, column);
    let start_col = crate::utils::wideness(text, start_x);
    let beyond = column + span;

    let mut pos = start_x;
    let mut col = start_col;
    let mut converted: Vec<u8> = Vec::new();

    /* 预取重绘相关的全局配置到局部变量，避免在下面逐字符循环内每遇到
     * 空格/制表符/多字节字符都 with_global() 借用 + clone。原实现每次空格
     * 或制表符都触发一次全局借用与 whitespace 克隆，整屏重绘时每行每字符
     * 都付出此代价。 */
    let (ws_bytes, wl0, wl1, tabsize, on_a_vt) = with_global(|g| {
        let ws = g.whitespace.clone().unwrap_or_default();
        let (a, b) = g.whitelen;
        (ws, a, b, g.tabsize, g.on_a_vt)
    });
    let show_whitespace = ISSET(WHITESPACE_DISPLAY);

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
            if show_whitespace {
                for i in wl0..wl0 + wl1 {
                    if i < ws_bytes.len() {
                        converted.push(ws_bytes[i]);
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
            let show_ws = show_whitespace
                && (converted.len() > 0 || !isdata || !ISSET(SOFTWRAP)
                    || col % tabsize == 0 || col == start_col);
            if show_ws {
                for i in 0..wl0 {
                    if i < ws_bytes.len() {
                        converted.push(ws_bytes[i]);
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
                if charwidth == 0 {
                    /* 在 Linux 控制台上跳过零宽字符。 */
                    if on_a_vt {
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
    let has_more = col > beyond
        || (chars::byte_at(text, pos) != 0 && (isprompt || (isdata && !ISSET(SOFTWRAP))));
    if has_more {
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

    (String::from_utf8_lossy(&converted).into_owned(), has_more)
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

// ======================== 按键分发（对应 nano.c 的主循环处理） ========================

/// 不带文件名且缓冲区为空时，在状态栏显示欢迎消息。
/// 条件与 nano.c 的 main() 一致：无文件名、缓冲区为空、
/// 未禁用帮助、且 Ctrl+G（帮助键）未被重绑定。
pub fn show_welcome_message() -> bool {
    let (filename_empty, totsize_zero) = with_global(|g| match &g.openfile {
        Some(o) => {
            let of = o.borrow();
            (
                of.filename
                    .as_deref()
                    .map(|s| s.is_empty())
                    .unwrap_or(true),
                of.totsize == 0,
            )
        }
        None => (true, true),
    });
    let not_rebound = global::first_sc_for(MMAIN, FunctionId::DoHelp)
        .map(|k| k.borrow().keycode == 0x07)
        .unwrap_or(false);
    let show = filename_empty && totsize_zero && !ISSET(NO_HELP) && not_rebound;
    if show {
        statusbar_centered(&format!("[ {} ]", crate::t!("welcome-message")));
    }
    show
}

/// 处理单个按键：执行快捷键或作为普通字符输入。
/// 返回 TRUE 表示已处理。
pub fn handle_input_key(key: i32) -> bool {
    /* 宏录制：记录每个按键（触发停止的按键由 record_macro 弹出）。 */
    if with_global(|g| g.recording) {
        with_global_mut(|g| g.macro_buffer.push(key));
    }
    let menu = with_global(|g| g.currmenu);
    let handled = execute_function(key, menu);

    if !handled {
        // 处理普通字符输入。键码可以是任意 Unicode 码点（中文等 > 255），
        // 但需排除：控制字符、Alt 组合键（0x200..=0x2FF，未绑定功能时忽略）。
        if key > 0 && key != ESC_CODE as i32 {
            if let Some(c) = char::from_u32(key as u32) {
                if !c.is_control() && !(0x200..=0x2FF).contains(&(key as u32)) {
                    if !ISSET(VIEW_MODE) {
                        text::insert_char(c);
                        edit_refresh();
                        return true;
                    }
                }
            }
        }
    }

    handled
}

/// 按 FunctionId 执行对应函数（对应 C 版 process_a_keystroke 的函数分发）。
/// 返回 true 表示已处理。未实现的函数返回 true（消费按键，无操作）。
fn execute_by_id(func: FunctionId) -> bool {
    match func {
        FunctionId::DoCancel => text::do_cancel(),
        FunctionId::DoExit => text::do_exit(),
        FunctionId::DoHelp => help::do_help(),
        FunctionId::DoLeft => movement::do_left(),
        FunctionId::DoRight => movement::do_right(),
        FunctionId::DoUp => movement::do_up(),
        FunctionId::DoDown => movement::do_down(),
        FunctionId::DoHome => movement::do_home(),
        FunctionId::DoEnd => movement::do_end(),
        FunctionId::DoPageUp => movement::do_page_up(),
        FunctionId::DoPageDown => movement::do_page_down(),
        FunctionId::DoDelete => cut::do_delete(),
        FunctionId::DoBackspace => cut::do_backspace(),
        FunctionId::DoEnter => text::do_enter(),
        FunctionId::DoTab => text::do_tab(),
        FunctionId::DoCut => cut::cut_text(),
        FunctionId::DoCopy => cut::copy_text(),
        FunctionId::DoPaste => cut::paste_text(),
        FunctionId::DoCutToEof => cut::cut_till_eof(),
        FunctionId::DoSearchForward => search::do_search_forward(),
        FunctionId::DoSearchBackward => search::do_search_backward(),
        FunctionId::DoFindNext => search::do_findnext(),
        FunctionId::DoFindPrevious => search::do_findprevious(),
        FunctionId::DoReplace => search::do_replace(),
        FunctionId::DoGoToLine => search::do_gotolinecolumn(),
        FunctionId::DoWriteOut => files::do_writeout(),
        FunctionId::DoInsertFile => files::do_insertfile(),
        FunctionId::DoExecute => files::do_execute(),
        FunctionId::DoSpell => text::do_spell(),
        FunctionId::DoFormatter => text::do_formatter(),
        FunctionId::DoIndent => text::do_indent(),
        FunctionId::DoUnindent => text::do_unindent(),
        FunctionId::DoComment | FunctionId::DoUncomment => text::do_comment(),
        FunctionId::DoUndo => text::do_undo(),
        FunctionId::DoRedo => text::do_redo(),
        FunctionId::DoRefresh => text::do_refresh(),
        FunctionId::DoSuspend => text::do_suspend(),
        FunctionId::DoScrollUp => movement::do_scroll_up(),
        FunctionId::DoScrollDown => movement::do_scroll_down(),
        FunctionId::DoPrevBlock => movement::to_prev_block(),
        FunctionId::DoNextBlock => movement::to_next_block(),
        FunctionId::DoParaBegin => movement::to_para_begin(),
        FunctionId::DoParaEnd => movement::to_para_end(),
        FunctionId::DoFirstLine => movement::do_first_line(),
        FunctionId::DoLastLine => movement::do_last_line(),
        FunctionId::DoNextWord => movement::to_next_word(),
        FunctionId::DoPrevWord => movement::to_prev_word(),
        FunctionId::DoMark => text::do_mark(),
        FunctionId::DoAnchor => text::do_anchor(),
        FunctionId::DoFullRefresh => full_refresh(),
        FunctionId::DoJustify => text::do_justify(),
        FunctionId::DoWordCompletion => text::complete_a_word(),
        FunctionId::DoPrevFile => files::switch_to_prev_buffer(),
        FunctionId::DoNextFile => files::switch_to_next_buffer(),
        FunctionId::DoFindBracket => search::do_find_bracket(),
        FunctionId::DoReportLocation => global::report_cursor_position(),
        FunctionId::DoVerbatimInput => text::do_verbatim_input(),
        FunctionId::FlipGoto => search::flip_goto(),
        FunctionId::ToTopRow => movement::to_top_row(),
        FunctionId::ToBottomRow => movement::to_bottom_row(),
        FunctionId::DoCycle => movement::do_cycle(),
        FunctionId::DoCenter => movement::do_center(),
        FunctionId::ChopPrevWord => cut::chop_previous_word(),
        FunctionId::ChopNextWord => cut::chop_next_word(),
        FunctionId::PutOrLiftAnchor => text::do_anchor(),
        FunctionId::ToPrevAnchor => text::to_prev_anchor(),
        FunctionId::ToNextAnchor => text::to_next_anchor(),
        FunctionId::CountWords => text::count_lines_words_and_characters(),
        FunctionId::DoZap => cut::zap_text(),
        FunctionId::DoSaveFile => files::do_savefile(),
        FunctionId::DoRecordMacro => record_macro(),
        FunctionId::DoRunMacro => run_macro(),
        FunctionId::DoLinter => text::do_linter(),
        FunctionId::ToFirstFile => crate::browser::to_first_file(),
        FunctionId::ToLastFile => crate::browser::to_last_file(),
        FunctionId::ToFiles => {
            let start = with_global(|g| {
                g.present_path
                    .clone()
                    .or_else(|| g.openfile.as_ref().and_then(|of| of.borrow().filename.clone()))
                    .unwrap_or_else(|| ".".to_string())
            });
            if let Some(chosen) = crate::browser::browse(&start) {
                let _ = files::open_buffer(&chosen);
            }
        }
        FunctionId::GotoDir => {
            let start = with_global(|g| g.present_path.clone().unwrap_or_else(|| ".".to_string()));
            let _ = crate::browser::browse_in(&start);
        }
        FunctionId::DoNothing => return false,
        _ => {}
    }
    true
}

/// 执行 rcfile bind 登记的用户绑定（对应 C 版 sclist 分发）。
fn execute_bound(bound: &BoundKey) -> bool {
    match bound.func {
        FunctionId::Implant => {
            /* 把植入字符串排入输入队列（对应 C 版 implant）。 */
            if let Some(expansion) = &bound.expansion {
                implant(expansion);
            }
            true
        }
        FunctionId::DoToggle => {
            TOGGLE(bound.toggle as usize);
            edit_refresh();
            true
        }
        _ => {
            let handled = execute_by_id(bound.func);
            if handled {
                edit_refresh();
            }
            handled
        }
    }
}

/// 根据键码执行对应函数。
fn execute_function(key: i32, _menu: i32) -> bool {
    // 用户 rcfile 绑定优先（对应 C 版 interpret/find_shortcut 的 sclist 分发）。
    let currmenu = with_global(|g| g.currmenu);
    let unbound = with_global(|g| {
        g.unbound_keys
            .iter()
            .any(|(k, m)| *k == key && (*m & currmenu) != 0)
    });
    if unbound {
        return false;
    }
    let bound = with_global(|g| {
        g.bound_keys
            .iter()
            .find(|b| b.keycode == key && (b.menus & currmenu) != 0)
            .cloned()
    });
    if let Some(b) = bound {
        return execute_bound(&b);
    }
    // 使用 if/else 链替代 match，避免表达式模式的问题
    if key == 1 { movement::do_home(); edit_refresh(); return true; }           // Ctrl+A
    if key == 2 {
        let menu = with_global(|g| g.currmenu);
        if menu == MMAIN || menu == MBROWSER || menu == MHELP {
            search::do_search_backward();
            return true;
        }
        movement::do_left();
        edit_refresh();
        return true;
    }                                                              // Ctrl+B: MMAIN/MBROWSER/MHELP 向后搜索；其余菜单向左
    if key == 3 {
        let menu = with_global(|g| g.currmenu);
        if menu == MMAIN {
            global::report_cursor_position();
        } else {
            text::do_cancel();
        }
        return true;
    }                                                              // Ctrl+C: MMAIN 报告光标位置；其余菜单取消
    if key == 4 { cut::do_delete(); edit_refresh(); return true; }              // Ctrl+D
    if key == 5 { movement::do_end(); edit_refresh(); return true; }            // Ctrl+E
    if key == 6 {
        let menu = with_global(|g| g.currmenu);
        if menu == MMAIN || menu == MBROWSER || menu == MHELP {
            search::do_search_forward();
            return true;
        }
        movement::do_right();
        edit_refresh();
        return true;
    }                                                              // Ctrl+F: MMAIN/MBROWSER/MHELP 搜索；其余菜单向右
    if key == 7 { help::do_help(); return true; }                               // Ctrl+G
    if key == 8 { cut::do_backspace(); edit_refresh(); return true; }           // Ctrl+H
    if key == 9 { text::do_tab(); edit_refresh(); return true; }                // Ctrl+I (Tab)
    if key == 10 { text::do_justify(); edit_refresh(); return true; }             // Ctrl+J: 对齐段落
    if key == 11 { cut::cut_text(); edit_refresh(); return true; }              // Ctrl+K
    if key == 12 { text::do_refresh(); edit_refresh(); return true; }           // Ctrl+L
    if key == 13 { text::do_enter(); edit_refresh(); return true; }             // Ctrl+M (Enter)
    if key == 14 { movement::do_down(); edit_refresh(); return true; }          // Ctrl+N
    if key == 15 { files::do_writeout(); edit_refresh(); return true; }         // Ctrl+O
    if key == 16 { movement::do_up(); edit_refresh(); return true; }            // Ctrl+P
    if key == 17 { text::do_refresh(); return true; }                           // Ctrl+Q
    if key == 18 { files::do_insertfile(); edit_refresh(); return true; }       // Ctrl+R
    if key == 19 { text::do_suspend(); return true; }                           // Ctrl+S
    if key == 20 { text::do_spell(); return true; }                             // Ctrl+T
    if key == 21 { cut::paste_text(); edit_refresh(); return true; }            // Ctrl+U
    if key == 22 { movement::do_page_down(); edit_refresh(); return true; }     // Ctrl+V
    if key == 23 { search::do_search_forward(); edit_refresh(); return true; }  // Ctrl+W
    if key == 28 { search::do_replace(); edit_refresh(); return true; }          // Ctrl+\: 替换（对应 C 版 MMAIN "^\\", do_replace）
    if key == 24 {                                                              // Ctrl+X
        if with_global(|g| g.inhelp) { /* 退出帮助 */ }
        text::do_exit();
        return true;
    }
    if key == 25 { movement::do_page_up(); edit_refresh(); return true; }       // Ctrl+Y
    if key == 26 { text::do_undo(); edit_refresh(); return true; }              // Ctrl+Z (Undo)

    // 功能键
    if key == KEY_F0 + 1 { help::do_help(); return true; }                      // F1
    if key == KEY_F0 + 2 { text::do_exit(); return true; }                      // F2
    if key == KEY_F0 + 3 { files::do_writeout(); return true; }                 // F3
    if key == KEY_F0 + 4 { search::do_search_forward(); return true; }          // F4
    if key == KEY_F0 + 5 { text::do_refresh(); return true; }                   // F5
    if key == KEY_F0 + 6 { text::do_spell(); return true; }                     // F6
    if key == KEY_F0 + 7 { return true; }                                       // F7
    if key == KEY_F0 + 8 { return true; }                                       // F8
    if key == KEY_F0 + 9 { cut::cut_text(); edit_refresh(); return true; }      // F9
    if key == KEY_F0 + 10 { cut::paste_text(); edit_refresh(); return true; }   // F10
    if key == KEY_F0 + 11 { return true; }                                      // F11
    if key == KEY_F0 + 12 { return true; }                                      // F12

    // 方向键
    if key == KEY_LEFT { movement::do_left(); edit_refresh(); return true; }
    if key == KEY_RIGHT { movement::do_right(); edit_refresh(); return true; }
    if key == KEY_UP { movement::do_up(); edit_refresh(); return true; }
    if key == KEY_DOWN { movement::do_down(); edit_refresh(); return true; }
    if key == KEY_HOME { movement::do_home(); edit_refresh(); return true; }
    if key == KEY_END { movement::do_end(); edit_refresh(); return true; }
    if key == KEY_PPAGE { movement::do_page_up(); edit_refresh(); return true; }
    if key == KEY_NPAGE { movement::do_page_down(); edit_refresh(); return true; }
    if key == KEY_DC { cut::do_delete(); edit_refresh(); return true; }
    if key == KEY_BACKSPACE { cut::do_backspace(); edit_refresh(); return true; }
    if key == KEY_ENTER { text::do_enter(); edit_refresh(); return true; }
    if key == 9 || key == KEY_BTAB { text::do_tab(); edit_refresh(); return true; }

    // 修饰键
    if key == CONTROL_LEFT { movement::do_prev_word(); edit_refresh(); return true; }
    if key == CONTROL_RIGHT { movement::do_next_word(false); edit_refresh(); return true; }
    if key == CONTROL_HOME { movement::do_first_line(); edit_refresh(); return true; }
    if key == CONTROL_END { movement::do_last_line(); edit_refresh(); return true; }
    if key == CONTROL_DELETE { cut::do_delete(); edit_refresh(); return true; }
    if key == CONTROL_UP { movement::do_scroll_up(); edit_refresh(); return true; }
    if key == CONTROL_DOWN { movement::do_scroll_down(); edit_refresh(); return true; }

    // Alt 组合
    if key == ALT_LEFT { movement::do_prev_word(); edit_refresh(); return true; }
    if key == ALT_RIGHT { movement::do_next_word(false); edit_refresh(); return true; }
    if key == ALT_UP { movement::to_para_begin(); edit_refresh(); return true; }
    if key == ALT_DOWN { movement::to_para_end(); edit_refresh(); return true; }
    if key == ALT_HOME { movement::do_first_line(); edit_refresh(); return true; }
    if key == ALT_END { movement::do_last_line(); edit_refresh(); return true; }
    if key == ALT_PAGEUP { movement::to_prev_block(); edit_refresh(); return true; }
    if key == ALT_PAGEDOWN { movement::to_next_block(); edit_refresh(); return true; }
    if key == ALT_INSERT { text::do_mark(); edit_refresh(); return true; }

    // 其他
    if key == KEY_IC { text::do_mark(); edit_refresh(); return true; }
    if key == KEY_SUSPEND { text::do_suspend(); return true; }
    if key == 29 { text::complete_a_word(); edit_refresh(); return true; }        // Ctrl+]: 单词补全
    if key == 31 { search::do_gotolinecolumn(); edit_refresh(); return true; }   // Ctrl+/ 或 Ctrl+_: 跳转到行
    if key == 0x25D { search::do_find_bracket(); edit_refresh(); return true; }   // M-]: 括号匹配
    if key == 0x22C { files::switch_to_prev_buffer(); edit_refresh(); return true; } // M-,: 前一个缓冲区
    if key == 0x22E { files::switch_to_next_buffer(); edit_refresh(); return true; } // M-.: 下一个缓冲区
    if key == ESC_CODE as i32 { return true; } // 忽略单独的 Esc

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::{set_flag, unset_flag, SOFTWRAP};
    use crate::files::make_new_buffer;

    /// MYESNO 菜单（Yes/No 询问）的快捷键栏应能绘制 Y/N/^C 三项且不崩溃。
    #[test]
    fn yesno_bottombars_draws_three_items() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.currmenu = MYESNO;
        });
        let mut out = io::stdout();
        draw_bottombars_lines(&mut out, 80, 24);
    }

    /// post_one_key 在宽度不足时不应越界（截断逻辑冒烟）。
    #[test]
    fn post_one_key_narrow_width_ok() {
        let mut out = io::stdout();
        post_one_key(&mut out, 23, 0, "Y", "Yes", 3);
        post_one_key(&mut out, 23, 0, "^C", "Cancel", 2);
        post_one_key(&mut out, 23, 0, "^C", "Cancel", 1);
    }

    /// clip_to_width 按显示宽度截断：全角字符计 2 列，不应劈开字符。
    #[test]
    fn clip_to_width_respects_double_width() {
        let msg = "保存已修改的缓冲区？";
        // 10 个全角字符 = 20 列，恰好放得下。
        let (s, w) = clip_to_width(msg, 20);
        assert_eq!(w, 20);
        assert_eq!(s, msg);
        // 19 列只能容纳 9 个全角字符（18 列），第 10 个字符被截掉。
        let (s, w) = clip_to_width(msg, 19);
        assert_eq!(w, 18);
        assert_eq!(s.chars().count(), 9);
        // 混合 ASCII 与中文。
        let (s, w) = clip_to_width("a中文b", 4);
        assert_eq!(w, 3);
        assert_eq!(s, "a中");
    }

    /// 植入队列解析：普通字符、{command} 占位符、{{}/{}} 转义、多字节字符。
    #[test]
    fn implant_parses_placeholders() {
        let drain = || {
            PLANTED_QUEUE.with(|q| {
                let mut q = q.borrow_mut();
                let mut out = Vec::new();
                while let Some(item) = q.pop_front() {
                    match item {
                        Planted::Key(k) => out.push(("key", k)),
                        Planted::Command(f, t) => out.push(("cmd", if f == FunctionId::DoEnter { 1000 + t } else { -1 })),
                    }
                }
                out
            })
        };
        implant("ab{enter}cd");
        let items = drain();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0], ("key", 97)); // a
        assert_eq!(items[1], ("key", 98)); // b
        // {enter} → DoEnter 命令
        assert_eq!(items[2], ("cmd", 1000));
        assert_eq!(items[3], ("key", 99)); // c
        assert_eq!(items[4], ("key", 100)); // d

        // 转义与多字节
        implant("{{}中{}}");
        let items = drain();
        assert_eq!(items, vec![("key", 123), ("key", 0x4E2D), ("key", 125)]);

        // 未知函数忽略
        implant("x{nosuchfunc}y");
        let items = drain();
        assert_eq!(items.len(), 2);
    }

    /// 宏录制与回放队列。
    #[test]
    fn macro_record_and_run() {
        crate::global::global_init();
        // 开始录制
        record_macro();
        assert!(with_global(|g| g.recording));
        with_global_mut(|g| g.macro_buffer.push(97));
        with_global_mut(|g| g.macro_buffer.push(98));
        // 停止录制（record_macro 会弹出触发键）
        with_global_mut(|g| g.macro_buffer.push(25)); // M-U 触发键
        record_macro();
        assert!(!with_global(|g| g.recording));
        assert_eq!(with_global(|g| g.macro_buffer.clone()), vec![97, 98]);

        // 回放：压入植入队列
        run_macro();
        let got = PLANTED_QUEUE.with(|q| {
            let mut q = q.borrow_mut();
            let mut v = Vec::new();
            while let Some(Planted::Key(k)) = q.pop_front() {
                v.push(k);
            }
            v
        });
        assert_eq!(got, vec![97, 98]);
    }

    /// display_string 的 has_more：内容放得下时为 FALSE；
    /// 放不下（非软换行的 data 行或 prompt）时置 TRUE，供画 ">" 标记。
    #[test]
    fn display_string_reports_has_more() {
        crate::global::global_init();
        with_global_mut(|g| g.COLS = 80);

        // 短行：span 足够，无截断。
        let (s, more) = display_string(b"hello", 0, 10, true, false);
        assert_eq!(s, "hello");
        assert!(!more);

        // 长行：非软换行的 data 行溢出 span → has_more。
        let (s, more) = display_string(b"hello world this is a very long line", 0, 10, true, false);
        assert!(more);
        assert!(s.chars().count() <= 10);

        // prompt 模式下同样报告。
        let (_, more) = display_string(b"abcdef", 0, 4, false, true);
        assert!(more);

        // 软换行下逐块显示：内容恰好填满一块（列未越过 beyond）时不报
        // has_more——块尾无需画 ">"。
        set_flag(SOFTWRAP);
        let (s, more) = display_string(b"0123456789abcdef", 0, 10, true, false);
        // 转换后超过 10 列？不会：col==beyond 即停
        assert!(s.chars().count() <= 10);
        assert!(!more, "软换行块尾不应报告 has_more");
        unset_flag(SOFTWRAP);
    }

    /// 非软换行时 update_line 返回 1；软换行时返回 1 + extra_chunks_in，
    /// 即该行占用的屏幕行数（对应 update_softwrapped_line 的返回值）。
    #[test]
    fn update_line_reports_consumed_rows() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 60;
        });

        // 注入 200 字符的单行文本。
        let long = vec![b'A'; 200];
        crate::text::inject(&long, long.len());
        let line = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone().unwrap());

        // 非软换行：恒为 1 行。
        unset_flag(SOFTWRAP);
        assert_eq!(update_line(&line, 0), 1);
        // 光标在中部时同样 1 行（横向滚动，不占更多行）。
        assert_eq!(update_line(&line, 100), 1);

        // 软换行：200 列 / 60 列断点 = 4 块（200/60=3 余 20）。
        set_flag(SOFTWRAP);
        let consumed = update_line(&line, 0);
        assert_eq!(consumed, (extra_chunks_in(&line) + 1) as i32);
        assert_eq!(consumed, 4, "200 列文本在 60 列块宽下应占 4 行");

        // 关闭软换行后恢复。
        unset_flag(SOFTWRAP);
    }

    /// 超长行横向滚动时 update_line 绘制 "<" 与 ">" 截断标记不应崩溃
    /// （from_col>0 与 has_more 两个分支）。
    #[test]
    fn update_line_draws_cutoff_markers() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 60;
        });
        unset_flag(SOFTWRAP);

        let long = vec![b'B'; 200];
        crate::text::inject(&long, 0);
        let line = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone().unwrap());

        // 光标在中后部：from_col > 0（行首画 '<'），右侧还有内容（画 '>'）。
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 150;
        });
        assert_eq!(update_line(&line, 150), 1);
    }

    /// 软换行下 update_softwrapped_line 对 edittop（含 firstcolumn 偏移）
    /// 与普通行都应给出合理的占用行数且不崩溃。
    #[test]
    fn update_softwrapped_line_edittop_offset_ok() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 20;
        });
        set_flag(SOFTWRAP);

        // 一行长文本 + 一行短文本。
        let long = vec![b'C'; 50];
        crate::text::inject(&long, long.len());
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            // 以换行断开：注入一个 MagicLine 后再追加文本
            of.current_x = 50;
        });
        crate::text::do_enter(); // 拆出新行
        crate::text::inject(b"short", 5);

        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            /* edittop 保持第一行；firstcolumn 设到第三块内（模拟上方
             * 块被滚出屏），行首只显示最后一小块。 */
            of.firstcolumn = 45;
        });

        let (edittop, second) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            let filetop = of.filetop.clone().unwrap();
            let second = {
                let r = filetop.borrow();
                r.next.clone().unwrap()
            };
            (of.edittop.clone().unwrap(), second)
        });

        /* edittop 行：50 列 / 20 列断点 = 3 块，但从 firstcolumn=45
         * 起只剩 [45,50) 一块。 */
        let rows1 = update_softwrapped_line(&edittop);
        assert_eq!(rows1, 1, "firstcolumn=45 时 edittop 只显示最后一块");
        assert!(rows1 < (extra_chunks_in(&edittop) + 1) as i32);

        /* 第二行：edittop 有 3 块且首块已整块滚出（chunk_for(45)=2），
         * 第二行应显示在其后；仍只占 1 行。 */
        let rows2 = update_softwrapped_line(&second);
        assert_eq!(rows2, (extra_chunks_in(&second) + 1) as i32);
        assert_eq!(rows2, 1);

        unset_flag(SOFTWRAP);
    }

    /// 光标在超长行的屏幕坐标换算（对应 C 版 place_the_cursor 的
    /// column -= get_page_start / column -= leftedge 折算）：
    /// 绝对显示列必须折算为屏幕内相对列，否则光标会被移出终端。
    #[test]
    fn cursor_screen_position_folds_horizontal_offset() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 60;
        });
        unset_flag(SOFTWRAP);

        // 200 列超长行，光标在字节 150（非软换行，页面滚动第二页）。
        let long = vec![b'A'; 200];
        crate::text::inject(&long, long.len());
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 150;
        });

        let (cur, edittop, cx, fc) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.current.clone().unwrap(), of.edittop.clone().unwrap(), of.current_x, of.firstcolumn)
        });
        let (row, col) = cursor_screen_position(&cur, &edittop, cx, fc);
        /* 屏幕列 = 150 - get_page_start(150)；应落在 [0, editwincols) 内，
         * 而不是原实现的 150（超出屏幕）。 */
        assert_eq!(row, 0, "光标仍在首行");
        let page = crate::utils::get_page_start(150);
        assert_eq!(col, 150 - page);
        assert!(col < 60, "折算后列必须在编辑窗口宽度内，实际 {col}");

        // 光标移到行尾（列 200）：仍在屏幕内。
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 200;
        });
        let (cur, edittop, cx, fc) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.current.clone().unwrap(), of.edittop.clone().unwrap(), of.current_x, of.firstcolumn)
        });
        let (row, col) = cursor_screen_position(&cur, &edittop, cx, fc);
        let page = crate::utils::get_page_start(200);
        assert_eq!(col, 200 - page);
        assert!(col < 60, "行尾列也必须折算到屏幕内，实际 {col}");
        assert_eq!(row, 0);
    }

    /// 软换行下光标坐标：行号累计各块，列折算到当前块内。
    #[test]
    fn cursor_screen_position_softwrap_chunks() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 20;
        });
        set_flag(SOFTWRAP);

        // 50 列长行 + 一行短行。
        let long = vec![b'C'; 50];
        crate::text::inject(&long, long.len());
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 50;
        });
        crate::text::do_enter();
        crate::text::inject(b"short", 5);

        // 光标在长行第三块内（字节 47 → 块 [40,50)）。
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current = of.filetop.clone();
            of.current_x = 47;
        });
        let (cur, edittop, cx, fc) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.current.clone().unwrap(), of.edittop.clone().unwrap(), of.current_x, of.firstcolumn)
        });
        let (row, col) = cursor_screen_position(&cur, &edittop, cx, fc);
        assert_eq!(row, 2, "第三块应显示在第 2 个屏幕行（0 基）");
        assert_eq!(col, 7, "47 - 块左缘 40 = 7");
        assert!(col < 20, "列必须在块宽内");

        // 光标在短行（第二行）：行号累计 1 + 3 块 = 4，列 = 2。
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            let second = {
                let ft = of.filetop.clone().unwrap();
                let nxt = ft.borrow().next.clone();
                nxt
            };
            of.current = second;
            of.current_x = 2;
        });
        let (cur, edittop, cx, fc) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.current.clone().unwrap(), of.edittop.clone().unwrap(), of.current_x, of.firstcolumn)
        });
        let (row, col) = cursor_screen_position(&cur, &edittop, cx, fc);
        /* 长行 50 列 / 20 块宽 = 3 块：短行显示在第 1+extra(2) = 3 行之后，
         * 即 row=3，列 = 2。 */
        assert_eq!(row, 3, "长行占 3 个屏幕行，短行在其后");
        assert_eq!(col, 2);

        unset_flag(SOFTWRAP);
    }

    /// 软换行 + edittop 上方块滚出（firstcolumn > 0）时，光标行号
    /// 从负偏移累计，仍落在正确屏幕行。
    #[test]
    fn cursor_screen_position_edittop_scrolled() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 20;
        });
        set_flag(SOFTWRAP);

        let long = vec![b'D'; 50];
        crate::text::inject(&long, long.len());
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 50;
        });
        crate::text::do_enter();
        crate::text::inject(b"tail", 4);

        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            // 模拟上两整块被滚出屏：edittop 从第三块（[40,50)）开始显示。
            of.firstcolumn = 40;
            let second = {
                let ft = of.filetop.clone().unwrap();
                let nxt = ft.borrow().next.clone();
                nxt
            };
            of.current = second;
            of.current_x = 0;
        });

        let (cur, edittop, cx, fc) = with_global(|g| {
            let of = g.openfile.as_ref().unwrap().borrow();
            (of.current.clone().unwrap(), of.edittop.clone().unwrap(), of.current_x, of.firstcolumn)
        });
        let (row, col) = cursor_screen_position(&cur, &edittop, cx, fc);
        /* -chunk_for(40, edittop) = -2（长行第 3 块 [40,50)）；加 edittop 的
         * 1+2 行；短行为首块。行 = -2 + 3 + 0 = 1，列 = 0。 */
        assert_eq!(row, 1, "短行应显示在 edittop 最后一块之下");
        assert_eq!(col, 0);

        unset_flag(SOFTWRAP);
    }

    /// 软换行下光标列落在视口左缘（firstcolumn 所在块）之前时，
    /// cursor_screen_position 累计行号会为负；place_the_cursor 写回
    /// cursor_row 时应钳位到 0，避免 adjust_viewport(STATIONARY) 把
    /// 负 goal 传给 go_back_chunks 造成反向滚动。
    #[test]
    fn place_the_cursor_clamps_negative_row() {
        crate::global::global_init();
        make_new_buffer();
        with_global_mut(|g| {
            g.COLS = 80;
            g.LINES = 24;
            g.editwincols = 20;
        });
        set_flag(SOFTWRAP);

        let long = vec![b'E'; 50];
        crate::text::inject(&long, long.len());
        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            of.current_x = 50;
        });
        crate::text::do_enter();
        crate::text::inject(b"end", 3);

        with_global_mut(|g| {
            let of = g.openfile.as_ref().unwrap().clone();
            let mut of = of.borrow_mut();
            /* 光标回到首行（长行）开头：视口左缘在第 3 块（[40,50)），
             * 而光标列 0 远在其左，行号累计为负（-2 + 0 + 0 = -2）。 */
            of.current = of.filetop.clone();
            of.firstcolumn = 40;
            of.current_x = 0;
        });

        place_the_cursor();

        let cursor_row = with_global(|g| g.openfile.as_ref().unwrap().borrow().cursor_row);
        assert!(cursor_row >= 0, "cursor_row 必须被钳位为非负，实际 {cursor_row}");
        assert_eq!(cursor_row, 0);

        unset_flag(SOFTWRAP);
    }
}
