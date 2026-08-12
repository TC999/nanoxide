/* winio.rs -- 缈昏瘧鑷?GNU nano 鐨?winio.c
 * 杈撳叆/鎸夐敭/瀹?杞箟搴忓垪瑙ｆ瀽鏍稿績锛屽鍔犲熀浜?crossterm 鐨勭粓绔?I/O 涓庢樉绀鸿緟鍔┿€?*/

#![allow(static_mut_refs)]
#![allow(dangerous_implicit_autorefs)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use std::ffi::c_void;
use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, Show, SetCursorStyle, DisableBlinking, EnableBlinking},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, SetAttribute, SetForegroundColor, SetBackgroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, DisableLineWrap,
        EnableLineWrap,
    },
};

use crate::gettext;

use crate::definitions::{
    message_type, openfilestruct, funcstruct, keystruct,
    linestruct, CONTROL_DELETE, CONTROL_DOWN, CONTROL_END, CONTROL_HOME, CONTROL_LEFT,
    CONTROL_RIGHT, CONTROL_SHIFT_DELETE, CONTROL_UP, ALT_DELETE, ALT_DOWN, ALT_END, ALT_HOME,
    ALT_INSERT, ALT_LEFT, ALT_PAGEUP, ALT_PAGEDOWN, ALT_RIGHT, ALT_UP, DEL_CODE, END_OF_PASTE,
    ESC_CODE, FOREIGN_SEQUENCE, KEY_BACKSPACE, KEY_BTAB, KEY_CANCEL, KEY_CENTER, KEY_DC, KEY_DOWN,
    KEY_END, KEY_ENTER, KEY_F0, KEY_HOME, KEY_IC, KEY_LEFT, KEY_MOUSE, KEY_NPAGE, KEY_PPAGE,
    KEY_RIGHT, KEY_SUSPEND, KEY_UP, MAXCHARLEN, MMAIN, MMOST, MORE_PLANTS, MSPELL, MYESNO,
    NUMBER_OF_ELEMENTS, PLANTED_A_COMMAND, SHIFT_ALT_DOWN, SHIFT_ALT_LEFT, SHIFT_ALT_RIGHT,
    SHIFT_ALT_UP, SHIFT_CONTROL_DOWN, SHIFT_CONTROL_END, SHIFT_CONTROL_HOME, SHIFT_CONTROL_LEFT,
    SHIFT_CONTROL_RIGHT, SHIFT_CONTROL_UP, SHIFT_DELETE, SHIFT_DOWN, SHIFT_END, SHIFT_HOME,
    SHIFT_PAGEUP, SHIFT_PAGEDOWN, SHIFT_TAB, SHIFT_UP, START_OF_PASTE, THE_WINDOW_RESIZED,
};
use crate::global::{
    allfuncs, altdelete, altend, altinsert, altleft, altpageup, altpagedown, altright, altup,
    altdown, althome, commandname, controlend, controlhome, controldelete, controlleft, controlright,
    controldown, controlup, currmenu, didfind, footwin, inhelp, interface_color_pair,
    lastmessage, light_from_col, light_to_col, margin, matchbrackets, meta_key, midwin,
    mute_modifiers, on_a_vt, perturbed, planted_shortcut, recook, refresh_needed,
    search_history, shift_held, shiftdown, shiftup, sidebar, spotlighted, the_window_resized,
    topwin, we_are_running, shifted_metas,
};
use crate::definitions::openfile;
use crate::utils::{nmalloc, nrealloc, xplustabs, actual_x, breadth};
use crate::definitions::copy_of;
use crate::chars::{using_utf8 as using_utf8_static, as_an_at};
use crate::rcfile::strtosc;

use crate::definitions::{
    ISSET, SET, UNSET, TOGGLE, WHITESPACE_DISPLAY, NO_HELP, ZERO, MINIBAR, QUICK_BLANK,
    CONSTANT_SHOW, STATEFLAGS, REBIND_DELETE, RAW_SEQUENCES, PRESERVE, SHOW_CURSOR, SOFTWRAP,
    JUMPY_SCROLLING, ERROR_MESSAGE, SELECTED_TEXT, STATUS_BAR, KEY_COMBO, FUNCTION_TAG,
    LINE_NUMBER, TITLE_BAR, PROMPT_BAR, EMPTY_LINE,
};
use crate::color::{A_NORMAL, A_REVERSE, A_BOLD, COLOR_PAIR};

/* ncurses 閿欒鐮併€?*/
pub const ERR: i32 = -1;

/* 屏幕缓冲模型（替代 ncurses 的 WINDOW）。我们把整屏看作一组行，
 * 每个绘制函数往缓冲写入文本，wrefresh/doupdate 时一次性刷到终端。
 * 每行的属性（颜色/反显）单独记录，以支持标题栏/状态栏高亮。 */
#[derive(Clone)]
struct Cell {
    ch: char,
    attr: i32,
}

static mut SCREEN: Vec<Vec<Cell>> = Vec::new();
static mut SCREEN_ROWS: usize = 0;
static mut SCREEN_COLS: usize = 0;
/* 当前绘制光标（逻辑坐标，对应 ncurses 的 wmove 位置）。 */
static mut CUR_ROW: i32 = 0;
static mut CUR_COL: i32 = 0;
/* 各窗口占用的行范围（topwin 顶部行数）。 */
static mut TOPROWS: usize = 0;

/* 三个窗口的哨兵指针（非空，用于区分 WINDOW* 参数）。 */
static mut WIN_TOP: u8 = 0;
static mut WIN_MID: u8 = 0;
static mut WIN_FOOT: u8 = 0;
/* 当前激活属性的全局值（wattron/wattroff 设置）。 */
static mut CUR_ATTR: i32 = 0;
/* 待刷新的光标位置（place_the_cursor 设置）。 */
static mut CURSOR_ROW: i32 = 0;
static mut CURSOR_COL: i32 = 0;
static mut CURSOR_VALID: bool = false;

/* 确保屏幕缓冲尺寸与当前终端一致；不足时扩展。 */
unsafe fn ensure_screen() {
    let rows = crate::files::LINES as usize;
    let cols = crate::files::COLS as usize;
    if rows == 0 { return; }
    if SCREEN_ROWS != rows || SCREEN_COLS != cols {
        SCREEN = vec![vec![]; rows];
        for r in SCREEN.iter_mut() {
            *r = vec![Cell { ch: ' ', attr: 0 }; cols];
        }
        SCREEN_ROWS = rows;
        SCREEN_COLS = cols;
    }
}

/* 把逻辑窗口坐标转换为整屏绝对行号。 */
unsafe fn abs_row(win: *mut c_void, row: i32) -> i32 {
    if win == crate::global::topwin {
        row
    } else if win == crate::global::footwin {
        crate::files::LINES - (if crate::files::LINES < 3 { 1 } else {
            let minimum = if ISSET(ZERO) { 3 } else if ISSET(MINIBAR) { 4 } else { 5 };
            if ISSET(NO_HELP) || crate::files::LINES < minimum { 1 } else { 3 }
        }) + row
    } else {
        /* midwin（编辑窗口）。 */
        TOPROWS as i32 + row
    }
}

/* 鐢?read_keys_from 濉厖鐨勬寜閿紦鍐插尯銆?*/
static mut KEY_BUFFER: Option<Vec<i32>> = None;
static mut NEXTCODES: usize = 0;
static mut CAPACITY: usize = 32;
static mut WAITING_CODES: usize = 0;
static mut PLANTS_POINTER: Option<String> = None;
static mut DIGIT_COUNT: i32 = 0;
static mut REVEAL_CURSOR: bool = false;
static mut LINGER_AFTER_ESCAPE: bool = false;
static mut COUNTDOWN: i32 = 0;

/* 杞崲琛屼笌缁樺埗鎵€鐢ㄧ殑闈欐€佺姸鎬併€?*/
static mut FROM_X: usize = 0;
static mut TILL_X: usize = 0;
static mut HAS_MORE: bool = false;
static mut IS_SHORTER: bool = true;

/* 瀹忓綍鍒剁姸鎬併€?*/
static mut SEQUEL_COLUMN: usize = 0;
static mut RECORDING: bool = false;
static mut MACRO_BUFFER: Option<Vec<i32>> = None;
static mut MACRO_LENGTH: usize = 0;
static mut MILESTONE: usize = 0;

/* 杞箟搴忓垪瑙ｆ瀽鐢ㄧ殑涓存椂鐘舵€併€?*/
const PROCEED: i32 = -44;
const INVALID_DIGIT: i32 = -77;

/* 寮€濮嬫垨鍋滄褰曞埗鎸夐敭銆?*/
pub fn record_macro() {
    static mut PREVIOUS_MACRO: Option<Vec<i32>> = None;
    static mut PREVIOUS_LENGTH: usize = 0;

    unsafe {
        RECORDING = !RECORDING;

        if RECORDING {
            PREVIOUS_MACRO = MACRO_BUFFER.take();
            PREVIOUS_LENGTH = MACRO_LENGTH;
            MACRO_BUFFER = None;
            MACRO_LENGTH = 0;
            statusline_s(message_type::REMARK, gettext!("Recording a macro..."));
        } else if MILESTONE == 0 {
            MACRO_BUFFER = PREVIOUS_MACRO.take();
            MACRO_LENGTH = PREVIOUS_LENGTH;
            statusline_s(message_type::REMARK, gettext!("Cancelled"));
        } else {
            PREVIOUS_MACRO = None;
            MACRO_LENGTH = MILESTONE;
            statusline_s(message_type::REMARK, gettext!("Stopped recording"));
        }

        if ISSET(STATEFLAGS) {
            titlebar(None);
        }
    }
}

/* 鎶婄粰瀹氫唬鐮佸姞鍏ュ畯缂撳啿鍖恒€?*/
pub fn add_to_macrobuffer(code: i32) {
    unsafe {
        MACRO_LENGTH += 1;
        match MACRO_BUFFER.as_mut() {
            Some(buf) => buf.push(code),
            None => MACRO_BUFFER = Some(vec![code]),
        }
    }
}

/* 鎶婂瓨鍌ㄧ殑鎸夐敭搴忓垪澶嶅埗鍥炴櫘閫氭寜閿紦鍐插尯锛屼互渚垮啀娆?鎵ц"銆?*/
pub fn run_macro() {
    unsafe {
        if RECORDING {
            statusline_s(message_type::AHEM, gettext!("Cannot run macro while recording"));
            MACRO_LENGTH = MILESTONE;
            return;
        }

        if MACRO_LENGTH == 0 {
            statusline_s(message_type::AHEM, gettext!("Macro is empty"));
            return;
        }

        for index in (0..MACRO_LENGTH).rev() {
            let code = MACRO_BUFFER.as_ref().map(|b| b[index]).unwrap_or(0);
            put_back(code);
        }

        mute_modifiers = true;
    }
}

/* 涓烘寜閿紦鍐插尯鍒嗛厤璇锋眰鐨勭┖闂淬€?*/
pub fn reserve_space_for(newsize: usize) {
    unsafe {
        if newsize < CAPACITY {
            crate::definitions::die(gettext!("Too much input at once\n"));
        }
        let mut buf = KEY_BUFFER.take().unwrap_or_default();
        buf.reserve(newsize.saturating_sub(buf.len()));
        KEY_BUFFER = Some(buf);
        NEXTCODES = 0;
        CAPACITY = newsize;
    }
}

/* 浠庣粰瀹氱獥鍙ｈ鍙栬嚦灏戜竴涓寜閿苟瀛樺叆鎸夐敭缂撳啿鍖恒€?*/
pub fn read_keys_from(frame: *mut c_void) {
    unsafe {
        doupdate();

        if REVEAL_CURSOR && (!spotlighted || ISSET(SHOW_CURSOR) || currmenu == MSPELL) &&
                (LINES_get() > 1 || lastmessage as i32 <= message_type::HUSH as i32) {
            curs_set(1);
        }

        if currmenu == MMAIN && (((ISSET(MINIBAR) || ISSET(ZERO) || LINES_get() == 1) &&
                    (lastmessage as i32) > message_type::HUSH as i32 &&
                    (lastmessage as i32) < message_type::ALERT as i32 &&
                    (lastmessage as i32) != message_type::INFO as i32) || spotlighted) {
            halfdelay(if ISSET(QUICK_BLANK) { 8 } else { 15 });
            disable_kb_interrupt();
        }

        let mut input: i32 = ERR;
        let mut errcount: usize = 0;

        while input == ERR {
            if !the_window_resized {
                input = wgetch(frame);
            }
            if the_window_resized {
                regenerate_screen();
                input = THE_WINDOW_RESIZED;
            }

            if input == ERR && { errcount += 1; errcount == 12345678 } {
                crate::definitions::die(gettext!("Too many errors from stdin\n"));
            }
        }

        curs_set(0);

        if KEY_BUFFER.is_none() {
            reserve_space_for(CAPACITY);
        }

        {
            let buf = KEY_BUFFER.as_mut().unwrap();
            if buf.is_empty() {
                buf.push(input);
            } else {
                buf[0] = input;
            }
        }
        NEXTCODES = 0;
        WAITING_CODES = 1;

        if currmenu == MMAIN {
            refresh_needed = refresh_needed || spotlighted;
            spotlighted = false;
        }

        if input == THE_WINDOW_RESIZED {
            return;
        }

        MILESTONE = MACRO_LENGTH;

        nodelay(frame, true);

        if input == ESC_CODE as i32 && (LINGER_AFTER_ESCAPE || ISSET(RAW_SEQUENCES)) {
            napms(20);
        }

        loop {
            if RECORDING {
                add_to_macrobuffer(input);
            }
            input = wgetch(frame);
            if input == ERR {
                break;
            }
            if WAITING_CODES == CAPACITY {
                reserve_space_for(2 * CAPACITY);
            }
            let buf = KEY_BUFFER.as_mut().unwrap();
            buf.push(input);
            WAITING_CODES += 1;
        }

        nodelay(frame, false);
    }
}

/* 杩斿洖鎸夐敭缂撳啿鍖轰腑绛夊緟鐨勬寜閿唬鐮佹暟閲忋€?*/
pub fn waiting_keycodes() -> i32 {
    unsafe { WAITING_CODES as i32 }
}

/* 鎶婄粰瀹氭寜閿唬鐮佹斁鍒版寜閿紦鍐插尯澶撮儴銆?*/
pub fn put_back(keycode: i32) {
    unsafe {
        let buf = KEY_BUFFER.get_or_insert_with(Vec::new);
        if NEXTCODES == 0 {
            if WAITING_CODES == CAPACITY {
                reserve_space_for(2 * CAPACITY);
            }
            buf.insert(0, keycode);
        } else {
            NEXTCODES -= 1;
            buf[NEXTCODES] = keycode;
        }
        WAITING_CODES += 1;
    }
}

/* 璁剧疆缁欏畾鐨勫睍寮€瀛楃涓诧紝浜ょ敱閿洏渚嬬▼閫愭"鍚炲叆"銆?*/
pub fn implant(string: &str) {
    unsafe {
        PLANTS_POINTER = Some(string.to_string());
        put_back(MORE_PLANTS);
        mute_modifiers = true;
    }
}

/* 缁х画澶勭悊灞曞紑瀛楃涓层€傝繑鍥為敊璇爜銆佹櫘閫氬瓧绗﹀瓧鑺傛垨鍛戒护蹇嵎閿崰浣嶇銆?*/
pub fn get_code_from_plantation() -> i32 {
    unsafe {
        let mut plants = match PLANTS_POINTER.take() {
            Some(s) => s,
            None => return ERR,
        };

        if plants.starts_with('{') {
            let closing = match plants.find('}') {
                Some(p) => p,
                None => return crate::definitions::MISSING_BRACE,
            };

            let inner = &plants[1..closing];
            if inner == "{" || inner == "}" {
                if closing + 1 >= plants.len() || plants.as_bytes()[closing + 1] != b'}' {
                    return crate::definitions::MISSING_BRACE;
                }
                let ch = plants.as_bytes()[closing - 1] as i32;
                plants = plants[closing + 2..].to_string();
                PLANTS_POINTER = if plants.is_empty() { None } else { Some(plants) };
                if PLANTS_POINTER.is_some() {
                    put_back(MORE_PLANTS);
                }
                return ch;
            }

            commandname = Some(inner.to_string());
            planted_shortcut = strtosc(inner);

            if planted_shortcut.is_null() {
                return crate::definitions::NO_SUCH_FUNCTION;
            }

            plants = plants[closing + 1..].to_string();
            PLANTS_POINTER = if plants.is_empty() { None } else { Some(plants) };
            if PLANTS_POINTER.is_some() {
                put_back(MORE_PLANTS);
            }
            return PLANTED_A_COMMAND;
        } else {
            let opening = plants.find('{');
            let firstbyte = plants.as_bytes()[0] as i32;
            let length = match opening {
                Some(p) => p,
                None => plants.len(),
            };

            if opening.is_some() {
                put_back(MORE_PLANTS);
            }

            for index in (1..length).rev() {
                put_back(plants.as_bytes()[index] as i32);
            }

            plants = plants[length..].to_string();
            PLANTS_POINTER = if plants.is_empty() { None } else { Some(plants) };

            if firstbyte != 0 {
                firstbyte
            } else {
                ERR
            }
        }
    }
}

/* 浠庢寜閿紦鍐插尯杩斿洖涓€涓唬鐮併€傝嫢缂撳啿鍖轰负绌轰絾鏈夌獥鍙ｏ紝鍏堜粠閿洏璇诲彇鏇村銆?*/
pub fn get_input(frame: *mut c_void) -> i32 {
    unsafe {
        if WAITING_CODES > 0 {
            spotlighted = false;
        } else if !frame.is_null() {
            read_keys_from(frame);
        }

        if WAITING_CODES > 0 {
            WAITING_CODES -= 1;
            let buf = KEY_BUFFER.as_ref().unwrap();
            let code = buf[NEXTCODES];
            if code == MORE_PLANTS {
                NEXTCODES += 1;
                return get_code_from_plantation();
            } else {
                NEXTCODES += 1;
                return code;
            }
        } else {
            return ERR;
        }
    }
}

/* 杩斿洖涓庣粰瀹氬瓧姣嶅搴旂殑鏂瑰悜閿唬鐮併€?*/
pub fn arrow_from_ABCD(letter: i32) -> i32 {
    if letter < 'C' as i32 {
        if letter == 'A' as i32 { KEY_UP } else { KEY_DOWN }
    } else {
        if letter == 'D' as i32 { KEY_LEFT } else { KEY_RIGHT }
    }
}

/* 鎶婁互 "Esc O" 寮€澶寸殑搴忓垪缈昏瘧鎴愬搴旂殑鎸夐敭浠ｇ爜銆?*/
pub fn convert_SS3_sequence(seq: &[i32], length: usize, consumed: &mut i32) -> i32 {
    if seq.is_empty() {
        return FOREIGN_SEQUENCE;
    }
    *consumed = 1;
    let c = seq[0] as u8 as char;
    match c {
        '1' => {
            if length > 3 && seq[1] == ';' as i32 {
                *consumed = 4;
                if seq[2] == '2' as i32 &&
                        ('A' as i32 <= seq[3] && seq[3] <= 'D' as i32) {
                    unsafe { shift_held = true; }
                    return arrow_from_ABCD(seq[3]);
                } else if seq[2] == '5' as i32 {
                    return match seq[3] as u8 as char {
                        'A' => CONTROL_UP,
                        'B' => CONTROL_DOWN,
                        'C' => CONTROL_RIGHT,
                        'D' => CONTROL_LEFT,
                        _ => FOREIGN_SEQUENCE,
                    };
                }
            }
            FOREIGN_SEQUENCE
        }
        '2' | '3' | '4' | '5' | '6' | '7' | '8' => {
            if length > 1 {
                *consumed = 2;
                if seq[0] == '4' as i32 || seq[0] > '5' as i32 {
                    return FOREIGN_SEQUENCE;
                }
                match seq[1] as u8 as char {
                    'A' => CONTROL_UP,
                    'B' => CONTROL_DOWN,
                    'C' => CONTROL_RIGHT,
                    'D' => CONTROL_LEFT,
                    _ => seq[1] - 0x40,
                }
            } else {
                FOREIGN_SEQUENCE
            }
        }
        'A' | 'B' | 'C' | 'D' => arrow_from_ABCD(seq[0]),
        'F' => KEY_END,
        'H' => KEY_HOME,
        'M' => KEY_ENTER,
        'P' | 'Q' | 'R' | 'S' | 'T' | 'U' | 'V' | 'W' | 'X' | 'Y' =>
            KEY_F0 + (seq[0] - 'O' as i32),
        'a' => CONTROL_UP,
        'b' => CONTROL_DOWN,
        'c' => CONTROL_RIGHT,
        'd' => CONTROL_LEFT,
        'j' => '*' as i32,
        'k' => '+' as i32,
        'l' => ',' as i32,
        'm' => '-' as i32,
        'n' => KEY_DC,
        'o' => '/' as i32,
        'p' => KEY_IC,
        'q' => KEY_END,
        'r' => KEY_DOWN,
        's' => KEY_NPAGE,
        't' => KEY_LEFT,
        'v' => KEY_RIGHT,
        'w' => KEY_HOME,
        'x' => KEY_UP,
        'y' => KEY_PPAGE,
        _ => FOREIGN_SEQUENCE,
    }
}

/* 鎶婁互 "Esc [" 寮€澶寸殑搴忓垪缈昏瘧鎴愬搴旂殑鎸夐敭浠ｇ爜銆?*/
pub fn convert_CSI_sequence(seq: &[i32], length: usize, consumed: &mut i32) -> i32 {
    if seq.is_empty() {
        return FOREIGN_SEQUENCE;
    }
    if seq[0] < '9' as i32 && length > 1 {
        *consumed = 2;
    }

    let c = seq[0] as u8 as char;
    let d3 = if seq.len() > 3 { seq[3] as u8 as char } else { '\0' };
    let d2 = if seq.len() > 2 { seq[2] as u8 as char } else { '\0' };
    match c {
        '1' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_HOME;
            } else if length > 2 && seq[2] == '~' as i32 {
                *consumed = 3;
                return match seq[1] {
                    53 => KEY_F0 + (seq[1] - '0' as i32),
                    55 => KEY_F0 + (seq[1] - '1' as i32),
                    56 => KEY_F0 + (seq[1] - '1' as i32),
                    57 => KEY_F0 + (seq[1] - '1' as i32),
                    _ => FOREIGN_SEQUENCE,
                };
            } else if length > 3 && seq[1] == ';' as i32 {
                *consumed = 4;
                if seq[2] == '2' as i32 {
                    return match d3 {
                        'A' | 'B' | 'C' | 'D' => {
                            unsafe { shift_held = true; }
                            arrow_from_ABCD(seq[3])
                        }
                        'F' => SHIFT_END,
                        'H' => SHIFT_HOME,
                        _ => FOREIGN_SEQUENCE,
                    };
                } else if seq[2] == '3' as i32 {
                    return match d3 {
                        'A' => ALT_UP, 'B' => ALT_DOWN, 'C' => ALT_RIGHT, 'D' => ALT_LEFT,
                        'F' => ALT_END, 'H' => ALT_HOME, _ => FOREIGN_SEQUENCE,
                    };
                } else if seq[2] == '4' as i32 {
                    return match d3 {
                        'A' => SHIFT_PAGEUP, 'B' => SHIFT_PAGEDOWN,
                        'C' => SHIFT_END, 'D' => SHIFT_HOME, _ => FOREIGN_SEQUENCE,
                    };
                } else if seq[2] == '5' as i32 {
                    return match d3 {
                        'A' => CONTROL_UP, 'B' => CONTROL_DOWN, 'C' => CONTROL_RIGHT,
                        'D' => CONTROL_LEFT, 'E' => KEY_CENTER, 'F' => CONTROL_END,
                        'H' => CONTROL_HOME, _ => FOREIGN_SEQUENCE,
                    };
                } else if seq[2] == '6' as i32 {
                    return match d3 {
                        'A' => SHIFT_CONTROL_UP, 'B' => SHIFT_CONTROL_DOWN,
                        'C' => SHIFT_CONTROL_RIGHT, 'D' => SHIFT_CONTROL_LEFT,
                        'F' => SHIFT_CONTROL_END, 'H' => SHIFT_CONTROL_HOME,
                        _ => FOREIGN_SEQUENCE,
                    };
                }
            } else if length > 4 && seq[2] == ';' as i32 && seq[4] == '~' as i32 {
                *consumed = 5;
            }
            FOREIGN_SEQUENCE
        }
        '2' => {
            if length > 2 && seq[2] == '~' as i32 {
                *consumed = 3;
                return match seq[1] {
                    48 => KEY_F0 + 9,
                    49 => KEY_F0 + 10,
                    51 => KEY_F0 + 11,
                    52 => KEY_F0 + 12,
                    53 => KEY_F0 + 13,
                    54 => KEY_F0 + 14,
                    56 => KEY_F0 + 15,
                    57 => KEY_F0 + 16,
                    _ => FOREIGN_SEQUENCE,
                };
            } else if length > 1 && seq[1] == '~' as i32 {
                return KEY_IC;
            } else if length > 3 && seq[1] == ';' as i32 && seq[3] == '~' as i32 {
                *consumed = 4;
                if seq[2] == '3' as i32 {
                    return ALT_INSERT;
                }
            } else if length > 4 && seq[2] == ';' as i32 && seq[4] == '~' as i32 {
                *consumed = 5;
            } else if length > 3 && seq[1] == '0' as i32 && seq[3] == '~' as i32 {
                *consumed = 4;
                return if seq[2] == '0' as i32 { START_OF_PASTE } else { END_OF_PASTE };
            } else {
                *consumed = length as i32;
                return FOREIGN_SEQUENCE;
            }
            FOREIGN_SEQUENCE
        }
        '3' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_DC;
            }
            if length > 3 && seq[1] == ';' as i32 && seq[3] == '~' as i32 {
                *consumed = 4;
                return match d2 {
                    '2' => SHIFT_DELETE,
                    '3' => ALT_DELETE,
                    '5' => CONTROL_DELETE,
                    '6' => CONTROL_SHIFT_DELETE,
                    _ => FOREIGN_SEQUENCE,
                };
            }
            if length > 1 && seq[1] == '$' as i32 {
                return SHIFT_DELETE;
            }
            if length > 1 && seq[1] == '^' as i32 {
                return CONTROL_DELETE;
            }
            if length > 1 && seq[1] == '@' as i32 {
                return CONTROL_SHIFT_DELETE;
            }
            if length > 2 && seq[2] == '~' as i32 {
                *consumed = 3;
            }
            FOREIGN_SEQUENCE
        }
        '4' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_END;
            }
            FOREIGN_SEQUENCE
        }
        '5' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_PPAGE;
            } else if length > 3 && seq[1] == ';' as i32 && seq[3] == '~' as i32 {
                *consumed = 4;
                if seq[2] == '2' as i32 {
                    return SHIFT_ALT_UP;
                } else if seq[2] == '3' as i32 {
                    return ALT_PAGEUP;
                }
            }
            FOREIGN_SEQUENCE
        }
        '6' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_NPAGE;
            } else if length > 3 && seq[1] == ';' as i32 && seq[3] == '~' as i32 {
                *consumed = 4;
                if seq[2] == '2' as i32 {
                    return SHIFT_ALT_DOWN;
                } else if seq[2] == '3' as i32 {
                    return ALT_PAGEDOWN;
                }
            }
            FOREIGN_SEQUENCE
        }
        '7' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_HOME;
            } else if length > 1 && seq[1] == '$' as i32 {
                return SHIFT_HOME;
            } else if length > 1 && seq[1] == '^' as i32 {
                return CONTROL_HOME;
            } else if length > 1 && seq[1] == '@' as i32 {
                return SHIFT_CONTROL_HOME;
            }
            FOREIGN_SEQUENCE
        }
        '8' => {
            if length > 1 && seq[1] == '~' as i32 {
                return KEY_END;
            } else if length > 1 && seq[1] == '$' as i32 {
                return SHIFT_END;
            } else if length > 1 && seq[1] == '^' as i32 {
                return CONTROL_END;
            } else if length > 1 && seq[1] == '@' as i32 {
                return SHIFT_CONTROL_END;
            }
            FOREIGN_SEQUENCE
        }
        '9' => KEY_DC,
        '@' => KEY_IC,
        'A' | 'B' | 'C' | 'D' => arrow_from_ABCD(seq[0]),
        'F' => KEY_END,
        'G' => KEY_NPAGE,
        'H' => KEY_HOME,
        'I' => KEY_PPAGE,
        'L' => KEY_IC,
        'M' | 'N' | 'O' | 'P' | 'Q' | 'R' | 'S' | 'T' => KEY_F0 + (seq[0] - 'L' as i32),
        'U' => KEY_NPAGE,
        'V' => KEY_PPAGE,
        'W' => KEY_F0 + 11,
        'X' => KEY_F0 + 12,
        'Y' => KEY_END,
        'Z' => SHIFT_TAB,
        'a' | 'b' | 'c' | 'd' => {
            unsafe { shift_held = true; }
            arrow_from_ABCD(seq[0] - 0x20)
        }
        '[' => {
            if length > 1 {
                *consumed = 2;
                if ('@' as i32) < seq[1] && seq[1] < 'F' as i32 {
                    return KEY_F0 + (seq[1] - '@' as i32);
                }
            }
            FOREIGN_SEQUENCE
        }
        _ => FOREIGN_SEQUENCE,
    }
}

/* 瑙ｉ噴甯︽湁缁欏畾璧峰瀛楄妭銆佸叾浣欏簭鍒椾粛鍦ㄦ寜閿紦鍐插尯涓殑杞箟搴忓垪銆?*/
pub fn parse_escape_sequence(starter: i32) -> i32 {
    unsafe {
        let buf = KEY_BUFFER.as_ref().unwrap();
        let seq: Vec<i32> = buf[NEXTCODES..NEXTCODES + WAITING_CODES].to_vec();
        let mut consumed: i32 = 1;
        let keycode = if starter == 'O' as i32 {
            convert_SS3_sequence(&seq, WAITING_CODES, &mut consumed)
        } else if starter == '[' as i32 {
            convert_CSI_sequence(&seq, WAITING_CODES, &mut consumed)
        } else {
            FOREIGN_SEQUENCE
        };
        WAITING_CODES -= consumed as usize;
        NEXTCODES += consumed as usize;
        keycode
    }
}

/* 渚濇璋冪敤锛屾妸缁欏畾鏁板瓧鍑戞垚涓€涓笁浣嶅崄杩涘埗瀛楄妭鐮侊紙000-255锛夈€?*/
pub fn assemble_byte_code(keycode: i32) -> i32 {
    static mut BYTE: i32 = 0;
    unsafe {
        DIGIT_COUNT += 1;
        if DIGIT_COUNT == 1 {
            BYTE = (keycode - '0' as i32) * 100;
            return PROCEED;
        }
        if DIGIT_COUNT == 2 {
            if BYTE < 200 || keycode <= '5' as i32 {
                BYTE += (keycode - '0' as i32) * 10;
                return PROCEED;
            } else {
                return keycode;
            }
        }
        if BYTE < 250 || keycode <= '5' as i32 {
            BYTE + keycode - '0' as i32
        } else {
            keycode
        }
    }
}

/* 鎶婃櫘閫?ASCII 瀛楃缈昏瘧鎴愬搴旂殑鎺у埗鐮併€?*/
pub fn convert_to_control(kbinput: i32) -> i32 {
    if '@' as i32 <= kbinput && kbinput <= '_' as i32 {
        kbinput - '@' as i32
    } else if '`' as i32 <= kbinput && kbinput <= '~' as i32 {
        kbinput - '`' as i32
    } else if '3' as i32 <= kbinput && kbinput <= '7' as i32 {
        kbinput - 24
    } else if kbinput == '?' as i32 || kbinput == '8' as i32 {
        DEL_CODE as i32
    } else if kbinput == ' ' as i32 || kbinput == '2' as i32 {
        0
    } else if kbinput == '/' as i32 {
        31
    } else {
        kbinput
    }
}

/* 浠庤緭鍏ユ祦涓彇鍑轰竴涓寜閿€傜炕璇戣浆涔夊簭鍒椾笌鏁板瓧閿洏鐮併€?*/
pub fn parse_kbinput(frame: *mut c_void) -> i32 {
    static mut FIRST_ESCAPE_WAS_ALONE: bool = false;
    static mut LAST_ESCAPE_WAS_ALONE: bool = false;
    static mut ESCAPES: i32 = 0;

    unsafe {
        meta_key = false;
        shift_held = false;

        let mut keycode = get_input(frame);

        if keycode == ESC_CODE as i32 {
            FIRST_ESCAPE_WAS_ALONE = LAST_ESCAPE_WAS_ALONE;
            LAST_ESCAPE_WAS_ALONE = (WAITING_CODES == 0);
            if DIGIT_COUNT > 0 {
                DIGIT_COUNT = 0;
                ESCAPES = 1;
            } else if { ESCAPES += 1; ESCAPES > 2 } {
                ESCAPES = if LAST_ESCAPE_WAS_ALONE { 0 } else { 1 };
            }
            return ERR;
        } else if keycode == ERR {
            return ERR;
        }

        let utf8 = using_utf8_static;
        let sm = unsafe { shifted_metas };

        let keycode = if ESCAPES == 0 {
            if keycode < 0xFF && keycode != '\t' as i32 && keycode != DEL_CODE as i32 {
                keycode
            } else {
                ESCAPES = 0;
                let buf = KEY_BUFFER.as_ref().unwrap();
                if keycode < 0x20 || 0x7E < keycode {
                    if keycode == '\t' as i32 {
                        SHIFT_TAB
                    } else if keycode == KEY_BACKSPACE || keycode == 0x08 ||
                            keycode == DEL_CODE as i32 {
                        CONTROL_SHIFT_DELETE
                    } else if 0xC0 <= keycode && keycode <= 0xFF && utf8 {
                        while WAITING_CODES > 0 && {
                            let b = buf[NEXTCODES]; b >= 0x80 && b <= 0xBF
                        } {
                            get_input(std::ptr::null_mut());
                        }
                        FOREIGN_SEQUENCE
                    } else if keycode < 0x20 && !LAST_ESCAPE_WAS_ALONE {
                        meta_key = true;
                        keycode
                    } else {
                        keycode
                    }
                } else if WAITING_CODES == 0 || buf[NEXTCODES] == ESC_CODE as i32 ||
                        (keycode != 'O' as i32 && keycode != '[' as i32) {
                    if 'A' as i32 <= keycode && keycode <= 'Z' as i32 && !sm {
                        keycode | 0x20
                    } else {
                        meta_key = true;
                        keycode
                    }
                } else {
                    parse_escape_sequence(keycode)
                }
            }
        } else if ESCAPES == 1 {
            ESCAPES = 0;
            let buf = KEY_BUFFER.as_ref().unwrap();
            if keycode < 0x20 || 0x7E < keycode {
                if keycode == '\t' as i32 {
                    SHIFT_TAB
                } else if keycode == KEY_BACKSPACE || keycode == 0x08 ||
                        keycode == DEL_CODE as i32 {
                    CONTROL_SHIFT_DELETE
                } else if 0xC0 <= keycode && keycode <= 0xFF && utf8 {
                    while WAITING_CODES > 0 && {
                        let b = buf[NEXTCODES]; b >= 0x80 && b <= 0xBF
                    } {
                        get_input(std::ptr::null_mut());
                    }
                    FOREIGN_SEQUENCE
                } else if keycode < 0x20 && !LAST_ESCAPE_WAS_ALONE {
                    meta_key = true;
                    keycode
                } else {
                    keycode
                }
            } else if WAITING_CODES == 0 || buf[NEXTCODES] == ESC_CODE as i32 ||
                    (keycode != 'O' as i32 && keycode != '[' as i32) {
                if 'A' as i32 <= keycode && keycode <= 'Z' as i32 && !sm {
                    keycode = keycode | 0x20;
                } else {
                    meta_key = true;
                }
                keycode
            } else {
                parse_escape_sequence(keycode)
            }
        } else {
            ESCAPES = 0;
            let buf = KEY_BUFFER.as_ref().unwrap();
            if keycode == '[' as i32 && WAITING_CODES > 0 &&
                    (('A' as i32 <= buf[NEXTCODES] && buf[NEXTCODES] <= 'D' as i32) ||
                     ('a' as i32 <= buf[NEXTCODES] && buf[NEXTCODES] <= 'd' as i32)) {
                match get_input(std::ptr::null_mut()) as u8 as char {
                    'A' => return KEY_HOME,
                    'B' => return KEY_END,
                    'C' => return CONTROL_RIGHT,
                    'D' => return CONTROL_LEFT,
                    'a' => { shift_held = true; return KEY_PPAGE; }
                    'b' => { shift_held = true; return KEY_NPAGE; }
                    'c' => { shift_held = true; return KEY_HOME; }
                    'd' => { shift_held = true; return KEY_END; }
                    _ => { keycode }
                }
            } else if WAITING_CODES > 0 && buf[NEXTCODES] != ESC_CODE as i32 &&
                    (keycode == '[' as i32 || keycode == 'O' as i32) {
                let kc = parse_escape_sequence(keycode);
                meta_key = true;
                kc
            } else if '0' as i32 <= keycode && (keycode <= '2' as i32 ||
                            (keycode <= '9' as i32 && DIGIT_COUNT > 0)) {
                let byte = assemble_byte_code(keycode);
                if byte == PROCEED {
                    ESCAPES = 2;
                    return ERR;
                } else if byte == '\t' as i32 || byte == DEL_CODE as i32 {
                    byte
                } else {
                    return byte;
                }
            } else if DIGIT_COUNT == 0 {
                if FIRST_ESCAPE_WAS_ALONE && !LAST_ESCAPE_WAS_ALONE {
                    if 'A' as i32 <= keycode && keycode <= 'Z' as i32 && !sm {
                        keycode = keycode | 0x20;
                    }
                    meta_key = true;
                } else {
                    keycode = convert_to_control(keycode);
                }
                keycode
            } else {
                keycode
            }
        };

        if keycode == controlleft {
            return CONTROL_LEFT;
        } else if keycode == controlright {
            return CONTROL_RIGHT;
        } else if keycode == controlup {
            return CONTROL_UP;
        } else if keycode == controldown {
            return CONTROL_DOWN;
        } else if keycode == controlhome {
            return CONTROL_HOME;
        } else if keycode == controlend {
            return CONTROL_END;
        } else if keycode == controldelete {
            return CONTROL_DELETE;
        } else if keycode == CONTROL_SHIFT_DELETE {
            return CONTROL_SHIFT_DELETE;
        } else if keycode == shiftup {
            unsafe { shift_held = true; }
            return KEY_UP;
        } else if keycode == shiftdown {
            unsafe { shift_held = true; }
            return KEY_DOWN;
        } else if keycode == SHIFT_CONTROL_LEFT {
            unsafe { shift_held = true; }
            return CONTROL_LEFT;
        } else if keycode == SHIFT_CONTROL_RIGHT {
            unsafe { shift_held = true; }
            return CONTROL_RIGHT;
        } else if keycode == SHIFT_CONTROL_UP {
            unsafe { shift_held = true; }
            return CONTROL_UP;
        } else if keycode == SHIFT_CONTROL_DOWN {
            unsafe { shift_held = true; }
            return CONTROL_DOWN;
        } else if keycode == SHIFT_CONTROL_HOME {
            unsafe { shift_held = true; }
            return CONTROL_HOME;
        } else if keycode == SHIFT_CONTROL_END {
            unsafe { shift_held = true; }
            return CONTROL_END;
        } else if keycode == altleft {
            return ALT_LEFT;
        } else if keycode == altright {
            return ALT_RIGHT;
        } else if keycode == altup {
            return ALT_UP;
        } else if keycode == altdown {
            return ALT_DOWN;
        } else if keycode == althome {
            return ALT_HOME;
        } else if keycode == altend {
            return ALT_END;
        } else if keycode == altpageup {
            return ALT_PAGEUP;
        } else if keycode == altpagedown {
            return ALT_PAGEDOWN;
        } else if keycode == altinsert {
            return ALT_INSERT;
        } else if keycode == altdelete {
            return ALT_DELETE;
        } else if keycode == SHIFT_ALT_LEFT {
            unsafe { shift_held = true; }
            return KEY_HOME;
        } else if keycode == SHIFT_ALT_RIGHT {
            unsafe { shift_held = true; }
            return KEY_END;
        } else if keycode == SHIFT_ALT_UP {
            unsafe { shift_held = true; }
            return KEY_PPAGE;
        } else if keycode == SHIFT_ALT_DOWN {
            unsafe { shift_held = true; }
            return KEY_NPAGE;
        } else if (KEY_F0 + 24) < keycode && keycode < (KEY_F0 + 64) {
            return FOREIGN_SEQUENCE;
        }

        match keycode {
            KEY_SLEFT => { unsafe { shift_held = true; } KEY_LEFT }
            KEY_SRIGHT => { unsafe { shift_held = true; } KEY_RIGHT }
            crate::definitions::KEY_SR | crate::definitions::KEY_SUP =>
                { unsafe { shift_held = true; } KEY_UP }
            crate::definitions::KEY_SF | crate::definitions::KEY_SDOWN =>
                { unsafe { shift_held = true; } KEY_DOWN }
            SHIFT_HOME | crate::definitions::KEY_A1 => { unsafe { shift_held = true; } KEY_HOME }
            SHIFT_END | crate::definitions::KEY_C1 => { unsafe { shift_held = true; } KEY_END }
            crate::definitions::KEY_EOL => CONTROL_END,
            SHIFT_PAGEUP | crate::definitions::KEY_A3 => KEY_PPAGE,
            SHIFT_PAGEDOWN | crate::definitions::KEY_C3 => KEY_NPAGE,
            127 =>
                if ISSET(REBIND_DELETE) { KEY_DC } else { KEY_BACKSPACE },
            KEY_BACKSPACE =>
                if ISSET(REBIND_DELETE) { KEY_DC } else { KEY_BACKSPACE },
            KEY_DC => if ISSET(REBIND_DELETE) { KEY_BACKSPACE } else { KEY_DC },
            crate::definitions::KEY_SDC => SHIFT_DELETE,
            crate::definitions::KEY_SCANCEL => KEY_CANCEL,
            crate::definitions::KEY_SSUSPEND | KEY_SUSPEND => 0x1A,
            KEY_BTAB => SHIFT_TAB,
            crate::definitions::KEY_SBEG | crate::definitions::KEY_BEG | crate::definitions::KEY_B2
                | crate::definitions::KEY_RESIZE | crate::definitions::KEY_FRESH => ERR,
            _ => keycode,
        }
    }
}

/* 璇诲彇涓€涓寜閿紝蹇界暐浠讳綍鏃犳晥鐨勬寜閿€?*/
pub fn get_kbinput(frame: *mut c_void, showcursor: bool) -> i32 {
    let mut kbinput = ERR;

    unsafe { REVEAL_CURSOR = showcursor; }

    while kbinput == ERR {
        kbinput = parse_kbinput(frame);
    }

    if frame == unsafe { midwin } {
        blank_it_when_expired();
    }

    kbinput
}

/* 璇诲彇涓€涓帶鍒跺瓧绗︼紙鎴?iTerm/Eterm/rxvt 鐨勫弻 Esc锛夛紝
 * 鎴栨妸鍏綅鏁板瓧搴忓垪杞崲鎴?Unicode 鐮佺偣銆俢ount 杩斿洖 1 鎴?2銆?*/
pub fn parse_verbatim_kbinput(frame: *mut c_void, count: *mut usize) -> *mut i32 {
    let mut keycode: i32;
    let yield_ptr: *mut i32;

    unsafe { REVEAL_CURSOR = true; }

    keycode = get_input(frame);

    unsafe {
        if keycode == THE_WINDOW_RESIZED {
            *count = 999;
            return std::ptr::null_mut();
        }

        let mut yield_vec = nmalloc(6 * 4);
        yield_ptr = yield_vec.as_mut_ptr() as *mut i32;
        std::mem::forget(yield_vec);

        let utf8 = using_utf8_static;
        if utf8 && isxdigit(keycode) {
            let mut unicode = assemble_unicode(keycode);
            let mut multibyte: [i8; 6] = [0; 6];

            while unicode == PROCEED as i64 {
                keycode = get_input(frame);
                unicode = assemble_unicode(keycode);
            }

            if keycode == THE_WINDOW_RESIZED {
                *count = 999;
                let _ = Vec::from_raw_parts(yield_ptr as *mut u8, 0, 6 * 4);
                return std::ptr::null_mut();
            }

            if unicode == INVALID_DIGIT as i64 {
                if keycode == ESC_CODE as i32 && WAITING_CODES > 0 {
                    get_input(std::ptr::null_mut());
                    while WAITING_CODES > 0 && {
                        let b = KEY_BUFFER.as_ref().unwrap()[NEXTCODES];
                        b > 0x1F && b < 0x40
                    } {
                        get_input(std::ptr::null_mut());
                    }
                    if WAITING_CODES > 0 && {
                        let b = KEY_BUFFER.as_ref().unwrap()[NEXTCODES];
                        b > 0x3F && b < 0x7F
                    } {
                        get_input(std::ptr::null_mut());
                    }
                } else if 0xC0 <= keycode && keycode <= 0xFF {
                    while WAITING_CODES > 0 && {
                        let b = KEY_BUFFER.as_ref().unwrap()[NEXTCODES];
                        b > 0x7F && b < 0xC0
                    } {
                        get_input(std::ptr::null_mut());
                    }
                }
            }

            let mcount = wctomb(&mut multibyte, unicode as i32);
            if mcount > MAXCHARLEN as i32 {
                *count = 0;
            } else {
                *count = mcount as usize;
            }
            for i in 0..*count {
                *yield_ptr.add(i) = multibyte[i] as i32;
            }
            return yield_ptr;
        }

        *yield_ptr = keycode;

        if keycode == ESC_CODE as i32 && WAITING_CODES > 0 {
            *yield_ptr.add(1) = get_input(std::ptr::null_mut());
            *count = 2;
        }
    }

    yield_ptr
}

/* 璇诲彇涓€涓帶鍒剁爜銆佷竴涓瓧绗﹀瓧鑺傛垨杞箟搴忓垪鐨勫墠瀵?Esc锛岃繑鍥炲瓧鑺傛暟鍒?count銆?*/
pub fn get_verbatim_kbinput(frame: *mut c_void, count: *mut usize) -> *mut u8 {
    let mut bytes = nmalloc(MAXCHARLEN + 2);
    let input: *mut i32;

    unsafe {
        if ISSET(PRESERVE) {
            disable_flow_control();
        }
        if !ISSET(RAW_SEQUENCES) {
            keypad(frame, false);
        }

        println!("\x1B[?2004l");
        std::io::stdout().flush().ok();

        LINGER_AFTER_ESCAPE = true;

        input = parse_verbatim_kbinput(frame, count);

        if !input.is_null() && *count > 0 {
            let p = *input;
            if p >= 0x80 && *count == 1 {
                put_back(p);
                *count = 999;
            } else if (p == '\n' as i32 && as_an_at) || (p == 0 && !as_an_at) {
                *count = 0;
            }
        }

        LINGER_AFTER_ESCAPE = false;

        if ISSET(PRESERVE) {
            enable_flow_control();
        }

        if !ISSET(RAW_SEQUENCES) {
            keypad(midwin, true);
            keypad(footwin, true);
        }

        if *count < 999 {
            for i in 0..*count {
                *bytes.as_mut_ptr().add(i) = *input.add(i) as u8;
            }
            *bytes.as_mut_ptr().add(*count) = 0;
        }

        if !input.is_null() {
            let _ = Vec::from_raw_parts(input as *mut u8, *count, 6 * 4);
        }
    }

    bytes.as_mut_ptr()
}

/* 鎶婂叚浣嶅崄鍏繘鍒舵暟瀛楀簭鍒楃粍瑁呮垚 Unicode 鐮佺偣銆?*/
pub fn assemble_unicode(symbol: i32) -> i64 {
    static mut UNICODE: i64 = 0;
    static mut DIGITS: i32 = 0;
    let mut outcome: i64 = PROCEED as i64;

    unsafe {
        if '0' as i32 <= symbol && symbol <= '9' as i32 {
            UNICODE = (UNICODE << 4) + (symbol - '0' as i32) as i64;
        } else if ('a' as i32 <= (symbol | 0x20)) && ((symbol | 0x20) <= 'f' as i32) {
            UNICODE = (UNICODE << 4) + ((symbol | 0x20) - 'a' as i32) as i64 + 10;
        } else if symbol == '\r' as i32 || symbol == ' ' as i32 {
            outcome = UNICODE;
        } else {
            outcome = INVALID_DIGIT as i64;
        }

        if { DIGITS += 1; DIGITS == 6 } && outcome == PROCEED as i64 {
            outcome = if UNICODE < 0x110000 { UNICODE } else { INVALID_DIGIT as i64 };
        }

        if outcome == PROCEED as i64 && currmenu == MMAIN {
            let partial = format!("{:0width$X}", UNICODE, width = DIGITS as usize);
            statusline(message_type::INFO, gettext!("Unicode Input: %s"));
        }

        if outcome != PROCEED as i64 {
            UNICODE = 0;
            DIGITS = 0;
        }
    }

    outcome
}

/* 鎶婃枃鏈寜鏄剧ず瀹藉害鎴柇/杞崲锛岃繑鍥炲彲鏄剧ず瀛楃涓层€?*/
pub fn display_string(text: &[u8], start: usize, len: usize, _when_wrapping: bool,
        _allow_midword: bool) -> String {
    let slice = if start < text.len() { &text[start..] } else { &text[0..0] };
    let mut out = String::new();
    let mut cols = 0;
    for &b in slice {
        if cols >= len {
            break;
        }
        out.push(b as char);
        cols += 1;
    }
    out
}

/* 鍦ㄧ姸鎬佹爮涓婂畨闈欏湴鏄剧ず涓€鏉℃櫘閫氭秷鎭€?*/
pub fn statusbar(msg: &str) {
    statusline(message_type::HUSH, msg);
}

/* 鍦ㄧ姸鎬佹爮涓婃樉绀烘秷鎭紱importance 浣庝簬涓婁竴鏉″垯蹇界暐銆?*/
pub fn statusline(importance: message_type, msg: &str) {
    unsafe {
        if importance as i32 >= message_type::AHEM as i32 {
            WAITING_CODES = 0;
        }

        if (importance as i32) < lastmessage as i32 && lastmessage as i32 > message_type::NOTICE as i32 {
            return;
        }

        lastmessage = importance;

        if importance as i32 > message_type::NOTICE as i32 {
            if importance == message_type::ALERT {
                beep();
            }
        }

        COUNTDOWN = if ISSET(QUICK_BLANK) { 1 } else { 20 };

        /* 把消息绘制到状态栏（footwin 第 0 行）。 */
        ensure_screen();
        let r = abs_row(crate::global::footwin, 0);
        if r >= 0 {
            for c in 0..SCREEN_COLS as i32 {
                put_cell(r, c, ' ');
            }
            let colorpair = if importance as i32 > message_type::NOTICE as i32 {
                interface_color_pair[ERROR_MESSAGE]
            } else if importance == message_type::NOTICE {
                interface_color_pair[SELECTED_TEXT]
            } else {
                interface_color_pair[STATUS_BAR]
            };
            CUR_ROW = r;
            CUR_COL = 0;
            CUR_ATTR = colorpair;
            let shown = if msg.chars().count() > crate::files::COLS as usize {
                let mut t: String = msg.chars().take((crate::files::COLS as usize).saturating_sub(1)).collect();
                t.push('…');
                t
            } else {
                msg.to_string()
            };
            put_str(&shown, 0);
        }
    }
}

/* 瑕嗙洊 statusline 鐨勫彲鍙樺弬鏁扮増鏈紙浠呬竴涓瓧绗︿覆鍙傛暟锛夈€?*/
pub fn statusline_s(importance: message_type, msg: &str) {
    statusline(importance, msg);
}

/* 鍦ㄧ姸鎬佹爮涓婅鍛婄敤鎴峰苟鏆傚仠鐗囧埢銆?*/
pub fn warn_and_briefly_pause(msg: &str) {
    blank_bottombars();
    statusline(message_type::ALERT, msg);
    unsafe { lastmessage = message_type::VACUUM; }
    napms(1500);
}

/* 鍦ㄥ睆骞曞簳閮ㄦ樉绀轰竴瀵?鎸夐敭 + 璇存槑"銆?*/
pub fn post_one_key(keystroke: &str, tag: &str, width: i32) {
    unsafe {
        ensure_screen();
        wattron(footwin, interface_color_pair[KEY_COMBO]);
        waddnstr(footwin, keystroke, actual_x(keystroke.as_bytes(), width as usize));
        wattroff(footwin, interface_color_pair[KEY_COMBO]);

        let mut width = width - breadth(keystroke.as_bytes()) as i32;
        if width < 2 {
            return;
        }

        waddch(footwin, ' ');
        wattron(footwin, interface_color_pair[FUNCTION_TAG]);
        waddnstr(footwin, tag, actual_x(tag.as_bytes(), (width - 1) as usize));
        wattroff(footwin, interface_color_pair[FUNCTION_TAG]);
    }
}

/* 鍦ㄧ獥鍙ｅ簳閮ㄤ袱琛屾樉绀哄搴旇彍鍗曠殑蹇嵎閿垪琛ㄣ€?*/
pub fn bottombars(menu: i32) {
    unsafe {
        currmenu = menu;

        if ISSET(NO_HELP) ||
            LINES_get() < (if ISSET(ZERO) { 3 } else if ISSET(MINIBAR) { 4 } else { 5 }) {
            return;
        }

        /* 纭缭灞忓箷缂撳啿宸茬粡鍒濆銆?*/
        ensure_screen();

        /* 纭畾瑕佹樉绀虹殑蹇嵎閿閿潯鐩鐩暟銆?*/
        let number = shown_entries_for(menu);

        /* 璁绠楃┖闂?*/
        let itemwidth = crate::files::COLS as usize / ((number + 1) / 2);

        if itemwidth == 0 {
            return;
        }

        blank_bottombars();

        let mut f = allfuncs;
        let mut index: usize = 0;
        while !f.is_null() && index < number {
            let mut thiswidth = itemwidth;

            if (*f).menus & menu == 0 {
                f = (*f).next;
                continue;
            }

            let s = first_sc_for(menu, (*f).func.unwrap());

            if s.is_null() {
                f = (*f).next;
                continue;
            }

            wmove(footwin, 1 + (index % 2) as i32, (index / 2) as i32 * itemwidth as i32);

            if (number % 2) == 1 && index + 2 == number {
                thiswidth += itemwidth;
            }

            if index + 2 >= number {
                thiswidth += crate::files::COLS as usize % itemwidth;
            }

            post_one_key(&(*s).keystr, gettext!((*f).tag), thiswidth as i32);

            index += 1;
            f = (*f).next;
        }

        wrefresh(footwin);
    }
}

/* 娓呯┖鐘舵€佹爮銆?*/
pub fn blank_statusbar() {
    unsafe {
        ensure_screen();
        let r = abs_row(crate::global::footwin, 0);
        if r >= 0 {
            for c in 0..SCREEN_COLS as i32 {
                put_cell(r, c, ' ');
            }
        }
    }
}

/* 娓呯呯┖搴曢儴涓よ蹇蹇嵎閿爮銆?*/
pub fn blank_bottombars() {
    unsafe {
        ensure_screen();
        let base = abs_row(crate::global::footwin, 1);
        for row in 1..3i32 {
            let r = base + (row - 1);
            if r >= 0 {
                for c in 0..SCREEN_COLS as i32 {
                    put_cell(r, c, ' ');
                }
            }
        }
    }
}

/* 娓呯┖鏍囬鏍忋€?*/
pub fn blank_titlebar() {
    unsafe {
        ensure_screen();
        let r = abs_row(crate::global::topwin, 0);
        if r >= 0 {
            for c in 0..SCREEN_COLS as i32 {
                put_cell(r, c, ' ');
            }
        }
    }
}

/* 鏄剧ず鏍囬鏍忋€?*/
pub fn titlebar(_s: Option<&str>) {
    unsafe {
        ensure_screen();
        blank_titlebar();
        let r = abs_row(crate::global::topwin, 0);
        if r < 0 { return; }
        let cols = crate::files::COLS as usize;
        let path = unsafe {
            if crate::definitions::openfile.is_null() {
                String::new()
            } else {
                (*crate::definitions::openfile).filename.as_ref().map(|s| s.clone()).unwrap_or_default()
            }
        };
        let label = if path.is_empty() { "New Buffer" } else { path.as_str() };
        let text = format!("  GNU nano  {}  {}", crate::definitions::VERSION, label);
        CUR_ROW = r;
        CUR_COL = 0;
        CUR_ATTR = interface_color_pair[TITLE_BAR];
        let shown = if text.chars().count() > cols {
            let mut t: String = text.chars().take(cols.saturating_sub(3)).collect();
            t.push_str("...");
            t
        } else {
            text
        };
        put_str(&shown, 0);
    }
}

/* 娓呯┖缂栬緫绐楀彛銆?*/
pub fn blank_edit() {
    unsafe {
        ensure_screen();
        for row in 0..crate::global::editwinrows {
            let r = abs_row(crate::global::midwin, row);
            if r >= 0 {
                for c in 0..SCREEN_COLS as i32 {
                    put_cell(r, c, ' ');
                }
            }
        }
    }
}

/* 鎿﹂櫎鐘舵€佹爮鍐呭銆?*/
pub fn wipe_statusbar() {
    unsafe {
        crate::global::lastmessage = crate::definitions::message_type::VACUUM;
        blank_statusbar();
    }
}

/* 鍒锋柊缂栬緫绐楀彛銆?*/
pub fn edit_refresh() {
    unsafe {
        ensure_screen();
        blank_edit();
        if crate::definitions::openfile.is_null() {
            doupdate();
            crate::global::refresh_needed = false;
            return;
        }
        let of = &mut *crate::definitions::openfile;
        let mut line = of.edittop;
        let mut row: i32 = 0;
        while !line.is_null() && row < crate::global::editwinrows {
            let r = abs_row(crate::global::midwin, row);
            let linestr = &*line;
            CUR_ROW = r;
            CUR_COL = 0;
            CUR_ATTR = 0;
            let data = &linestr.data;
            let converted = display_string(data.as_bytes(), 0, crate::files::COLS as usize, true, false);
            put_str(&converted, 0);
            /* 清除行尾。 */
            for c in (CUR_COL as usize)..(crate::files::COLS as usize) {
                put_cell(r, c as i32, ' ');
            }
            line = linestr.next;
            row += 1;
        }
        place_the_cursor();
        doupdate();
        crate::global::refresh_needed = false;
    }
}

/* 鎶婂厜鏍囨斁鍒扮紪杈戠獥鍙鍙ｇ殑 (cursor_row, current_x)銆?*/
pub fn place_the_cursor() {
    unsafe {
        if crate::definitions::openfile.is_null() { return; }
        let of = &mut *crate::definitions::openfile;
        let row_isize = (*of.current).lineno - (*of.edittop).lineno;
        let col = of.current_x as i32;
        if row_isize >= 0 && row_isize < crate::global::editwinrows as isize {
            CURSOR_ROW = abs_row(crate::global::midwin, row_isize as i32);
            CURSOR_COL = col;
            CURSOR_VALID = true;
            of.cursor_row = row_isize;
        }
    }
}

/* 瀹屾暣鍒锋柊鏁翠釜灞忓箷銆?*/
pub fn full_refresh() {
    doupdate();
}

/* 杩蜂綘鐘舵€佹爮妯″紡鍒锋柊銆?*/
pub fn minibar() {}

/* 閲嶇粯缁欏畾琛岀殑鏄剧ず銆?*/
pub fn update_line(_line: *mut linestruct, _x: usize) {}

/* ncurses 初始化与终端查询占位实现（crossterm 后续接入）。 */
pub fn initscr() -> *mut c_void {
    std::ptr::null_mut()
}
pub fn has_colors() -> bool {
    false
}
pub fn start_color() {}
pub fn get_keycode(_name: &str, _fallback: i32) -> i32 {
    0
}
pub fn enable_kb_interrupt() {}
pub fn set_blankdelay_to_one() {}

/* 是否已进入原始模式（启用备用屏幕）。 */
static mut RAW_MODE: bool = false;

/* 进入备用屏幕并启用原始模式。 */
pub fn enter_terminal() {
    unsafe {
        if RAW_MODE {
            return;
        }
        RAW_MODE = true;
        let _ = terminal::enable_raw_mode();
        let _ = execute!(
            io::stdout(),
            EnterAlternateScreen,
            DisableLineWrap,
            Hide,
        );
        /* 记录初始终端尺寸。 */
        if let Ok((cols, lines)) = terminal::size() {
            crate::files::COLS = cols as i32;
            crate::files::LINES = lines as i32;
        }
    }
}

/* 离开备用屏幕并恢复规范模式。 */
pub fn leave_terminal() {
    unsafe {
        if !RAW_MODE {
            return;
        }
        RAW_MODE = false;
        let _ = execute!(
            io::stdout(),
            Show,
            EnableLineWrap,
            LeaveAlternateScreen,
        );
        let _ = terminal::disable_raw_mode();
    }
}

/* 返回当前终端尺寸并刷新 COLS/LINES。 */
pub fn refresh_size() {
    if let Ok((cols, lines)) = terminal::size() {
        unsafe {
            crate::files::COLS = cols as i32;
            crate::files::LINES = lines as i32;
        }
    }
}

/* 把 crossterm 的键盘事件翻译为一个 nano 键码（字节或 KEY_* 常量）。 */
fn translate_key_event(ev: KeyEvent) -> i32 {
    use crate::definitions::{
        ALT_DELETE, ALT_DOWN, ALT_END, ALT_HOME, ALT_INSERT, ALT_LEFT, ALT_PAGEUP, ALT_PAGEDOWN,
        ALT_RIGHT, ALT_UP, CONTROL_DELETE, CONTROL_DOWN, CONTROL_END, CONTROL_HOME, CONTROL_LEFT,
        CONTROL_RIGHT, CONTROL_UP, DEL_CODE, KEY_BACKSPACE, KEY_DC, KEY_DOWN, KEY_END, KEY_ENTER,
        KEY_F0, KEY_HOME, KEY_IC, KEY_LEFT, KEY_NPAGE, KEY_PPAGE, KEY_RIGHT, KEY_UP, SHIFT_TAB,
        THE_WINDOW_RESIZED,
    };
    use crate::global::{meta_key, shift_held};

    if ev.kind != KeyEventKind::Press {
        return ERR;
    }

    let ctrl = ev.modifiers.contains(KeyModifiers::CONTROL);
    let shift = ev.modifiers.contains(KeyModifiers::SHIFT);
    let alt = ev.modifiers.contains(KeyModifiers::ALT);

    unsafe {
        shift_held = shift;
        meta_key = alt;
    }

    match ev.code {
        KeyCode::Char(c) => {
            if ctrl {
                /* 控制字符：'a'..'z' → 1..26，其它常见键做近似。 */
                let lower = c.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    return (lower as i32 - 'a' as i32) + 1;
                }
                return c as i32;
            }
            if alt {
                /* Alt+字符：交给上层按 ESC 序列处理，这里直接返回字符。 */
                return c as i32;
            }
            c as i32
        }
        KeyCode::Enter => KEY_ENTER,
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::Delete => KEY_DC,
        KeyCode::Tab => '\t' as i32,
        KeyCode::BackTab => SHIFT_TAB,
        KeyCode::Left => {
            if ctrl { CONTROL_LEFT } else { KEY_LEFT }
        }
        KeyCode::Right => {
            if ctrl { CONTROL_RIGHT } else { KEY_RIGHT }
        }
        KeyCode::Up => {
            if ctrl { CONTROL_UP } else { KEY_UP }
        }
        KeyCode::Down => {
            if ctrl { CONTROL_DOWN } else { KEY_DOWN }
        }
        KeyCode::Home => {
            if ctrl { CONTROL_HOME } else { KEY_HOME }
        }
        KeyCode::End => {
            if ctrl { CONTROL_END } else { KEY_END }
        }
        KeyCode::PageUp => KEY_PPAGE,
        KeyCode::PageDown => KEY_NPAGE,
        KeyCode::Insert => KEY_IC,
        KeyCode::Esc => ESC_CODE as i32,
        KeyCode::F(n) => KEY_F0 + n as i32,
        _ => FOREIGN_SEQUENCE,
    }
}

/* 从 crossterm 读取一个按键事件并翻译为 nano 键码。 */
fn read_one_key_event() -> i32 {
    loop {
        match event::read() {
            Ok(Event::Key(ev)) => {
                let code = translate_key_event(ev);
                if code != ERR {
                    return code;
                }
            }
            Ok(Event::Resize(_, _)) => {
                unsafe { crate::global::the_window_resized = true; }
                return THE_WINDOW_RESIZED;
            }
            Ok(_) => continue,
            Err(_) => return ERR,
        }
    }
}

/* 初始化终端窗口。 */
pub fn window_init() {
    unsafe {
        let lines = crate::files::LINES;
        let cols = crate::files::COLS;

        if lines < 3 {
            crate::global::editwinrows = if ISSET(ZERO) { lines } else { 1 };
        } else {
            let minimum = if ISSET(ZERO) { 3 } else if ISSET(MINIBAR) { 4 } else { 5 };
            let toprows = if ISSET(EMPTY_LINE) && lines > minimum { 2 } else { 1 };
            let bottomrows = if ISSET(NO_HELP) || lines < minimum { 1 } else { 3 };
            let toprows = if ISSET(MINIBAR) || ISSET(ZERO) { 0 } else { toprows };
            crate::global::editwinrows = lines - toprows - bottomrows + if ISSET(ZERO) { 1 } else { 0 };
        }

        /* 记录布局：topwin 占 toprows 行，midwin 占 editwinrows 行，footwin 占剩余。 */
        TOPROWS = if lines < 3 { 0 } else {
            let minimum = if ISSET(ZERO) { 3 } else if ISSET(MINIBAR) { 4 } else { 5 };
            if ISSET(MINIBAR) || ISSET(ZERO) { 0 }
            else if ISSET(EMPTY_LINE) && lines > minimum { 2 } else { 1 }
        };

        /* 设置三个窗口哨兵指针（非空，便于绘制函数区分窗口）。 */
        crate::global::topwin = std::ptr::addr_of_mut!(WIN_TOP) as *mut c_void;
        crate::global::midwin = std::ptr::addr_of_mut!(WIN_MID) as *mut c_void;
        crate::global::footwin = std::ptr::addr_of_mut!(WIN_FOOT) as *mut c_void;
    }
}

/* 终端 I/O 后端（基于 crossterm）。 */
pub fn doupdate() {
    unsafe {
        ensure_screen();
        let mut out = String::new();
        /* 移动到左上角并清屏。 */
        out.push_str("\x1B[H\x1B[2J");
        for r in 0..SCREEN_ROWS {
            let mut cur_attr: i32 = -1;
            for c in 0..SCREEN_COLS {
                let cell = &SCREEN[r][c];
                if cell.attr != cur_attr {
                    /* 重置后再设置。 */
                    out.push_str("\x1B[0m");
                    if cell.attr & A_REVERSE != 0 {
                        out.push_str("\x1B[7m");
                    }
                    if cell.attr & A_BOLD != 0 {
                        out.push_str("\x1B[1m");
                    }
                    cur_attr = cell.attr;
                }
                out.push(cell.ch);
            }
            if r + 1 < SCREEN_ROWS {
                out.push_str("\x1B[0m\r\n");
            }
        }
        out.push_str("\x1B[0m");
        if CURSOR_VALID {
            out.push_str(&format!("\x1B[{};{}H", CURSOR_ROW + 1, CURSOR_COL + 1));
        }
        let _ = execute!(io::stdout(), crossterm::style::Print(out));
        let _ = io::stdout().flush();
    }
}

pub fn curs_set(v: i32) {
    let _ = match v {
        0 => queue!(io::stdout(), Hide),
        _ => queue!(io::stdout(), Show),
    };
    let _ = io::stdout().flush();
}

pub fn halfdelay(_t: i32) {}
pub fn raw() {}
pub fn disable_kb_interrupt() {}

pub fn wgetch(_frame: *mut c_void) -> i32 {
    read_one_key_event()
}

pub fn regenerate_screen() {}
pub fn nodelay(_frame: *mut c_void, _bf: bool) {}
pub fn napms(_ms: i32) {}
pub fn keypad(_frame: *mut c_void, _bf: bool) {}
pub fn enable_flow_control() {}
pub fn disable_flow_control() {}
pub fn wredrawln(_w: *mut c_void, _beg: i32, _num: i32) {}
pub fn blank_it_when_expired() {}

/* ncurses 缁樺埗杈呭姪鍗犱綅銆?*/
pub fn wattron(_w: *mut c_void, attr: i32) {
    unsafe { CUR_ATTR |= attr; }
}
pub fn wattroff(_w: *mut c_void, attr: i32) {
    unsafe { CUR_ATTR &= !attr; }
}
pub fn wmove(win: *mut c_void, y: i32, x: i32) {
    unsafe {
        CUR_ROW = abs_row(win, y);
        CUR_COL = x;
    }
}
/* 在屏幕缓冲的 (row,col) 处写入一个字符（带当前属性）。 */
unsafe fn put_cell(row: i32, col: i32, ch: char) {
    if row < 0 || col < 0 { return; }
    let r = row as usize;
    let c = col as usize;
    if r >= SCREEN_ROWS || c >= SCREEN_COLS { return; }
    SCREEN[r][c] = Cell { ch, attr: CUR_ATTR };
}
/* 把字符串写入当前绘制位置（按字符推进，处理制表符）。 */
unsafe fn put_str(s: &str, n: usize) {
    let mut count = 0;
    for ch in s.chars() {
        if n > 0 && count >= n { break; }
        if ch == '\t' {
            let tabstop = 8;
            let mut col = CUR_COL;
            loop {
                put_cell(CUR_ROW, col, ' ');
                col += 1;
                if col % tabstop == 0 { break; }
            }
            CUR_COL = col;
            count += 1;
            continue;
        }
        put_cell(CUR_ROW, CUR_COL, ch);
        CUR_COL += 1;
        count += 1;
    }
}
pub fn waddstr(win: *mut c_void, s: &str) {
    unsafe {
        ensure_screen();
        if win == topwin || win == midwin || win == footwin {
            put_str(s, 0);
        }
    }
}
pub fn waddch(win: *mut c_void, ch: char) {
    let mut buf = [0u8; 4];
    let s = if ch == '\t' { "\t" } else { ch.encode_utf8(&mut buf) };
    waddstr(win, s);
}
pub fn waddnstr(win: *mut c_void, s: &str, n: usize) {
    unsafe {
        ensure_screen();
        if win == topwin || win == midwin || win == footwin {
            put_str(s, n);
        }
    }
}
pub fn mvwprintw(win: *mut c_void, y: i32, x: i32, fmt: &str, col: i32, s: &str) {
    unsafe {
        ensure_screen();
        wmove(win, y, x);
        let width = col.unsigned_abs() as usize;
        if fmt.contains("%*s") || fmt.contains("%*zd") {
            let padded = if (col as usize) < s.chars().count() && col >= 0 {
                s.to_string()
            } else {
                format!("{:width$}", s, width = width)
            };
            put_str(&padded, 0);
        } else {
            put_str(s, 0);
        }
    }
}
pub fn mvwaddstr(win: *mut c_void, y: i32, x: i32, s: &str) {
    unsafe {
        ensure_screen();
        wmove(win, y, x);
        put_str(s, 0);
    }
}
pub fn mvwaddnstr(win: *mut c_void, y: i32, x: i32, s: &str, n: usize) {
    unsafe {
        ensure_screen();
        wmove(win, y, x);
        put_str(s, n);
    }
}
pub fn wclrtoeol(_win: *mut c_void) {
    unsafe {
        let r = CUR_ROW;
        if r < 0 { return; }
        for c in CUR_COL..SCREEN_COLS as i32 {
            put_cell(r, c, ' ');
        }
    }
}
pub fn wnoutrefresh(_w: *mut c_void) {}
pub fn wrefresh(_w: *mut c_void) {}
pub fn beep() {
    let _ = execute!(io::stdout(), crossterm::style::Print("\x07"));
}

/* 榧犳爣鐩稿叧鍗犱綅銆?*/
pub fn wmouse_trafo(_w: *mut c_void, _y: &mut i32, _x: &mut i32, _to_screen: bool) -> bool { true }
pub fn get_mouseinput(_y: &mut i32, _x: &mut i32) -> i32 { 0 }

/* 鎶?Unicode 鐮佺偣杞崲鎴愬瀛楄妭搴忓垪锛堢畝鍖栧疄鐜帮級銆?*/
pub fn wctomb(buf: &mut [i8; 6], code: i32) -> i32 {
    if code < 0x80 {
        buf[0] = code as i8;
        1
    } else {
        let s = format!("{}", std::char::from_u32(code as u32).unwrap_or('?'));
        let bytes = s.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            buf[i] = b as i8;
        }
        bytes.len() as i32
    }
}

pub fn isxdigit(c: i32) -> bool {
    ('0' as i32 <= c && c <= '9' as i32) ||
    ('a' as i32 <= c && c <= 'f' as i32) ||
    ('A' as i32 <= c && c <= 'F' as i32)
}

/* 杩斿洖缁欏畾鑿滃崟瑕佹樉绀虹殑蹇嵎閿潯鐩暟銆?*/
pub fn shown_entries_for(menu: i32) -> usize {
    unsafe { crate::global::shown_entries_for(menu) }
}

/* 杩斿洖缁欏畾鑿滃崟銆佺粰瀹氬嚱鏁板搴旂殑绗竴涓揩鎹烽敭銆?*/
pub fn first_sc_for(menu: i32, function: unsafe fn()) -> *mut keystruct {
    unsafe { crate::global::first_sc_for(menu, function) }
}

/* 返回 LINES（终端行数）。 */
pub fn LINES_get() -> i32 {
    unsafe { crate::files::LINES }
}

/* 返回 COLS（终端列数）。 */
pub fn COLS_get() -> i32 {
    unsafe { crate::files::COLS }
}

/* WINIO_CHUNK3_END */



