
use crate::chars::*;

use crate::definitions::*;

use crate::files::*;

use crate::global::*;

use crate::rcfile::{to_first_file, to_last_file, do_enter_safe, do_tab_safe, do_delete_safe, do_backspace_safe, do_verbatim_input_safe};
use crate::history::reset_history_pointer_for;
use crate::text::{do_cancel, do_help, full_refresh};
use crate::winio::{wipe_statusbar, titlebar, edit_refresh, get_kbinput, window_init};
use crate::files::free_lines;

use crate::text::*;

use crate::utils::*;

/// 状态栏提示字符串（对应 C 的静态变prompt）
static mut PROMPTTEXT: Option<String> = None;

/// 状态栏输入答案中的光标位置（对C 的静态变typing_x）
static mut TYPING_X: usize = HIGHEST_POSITIVE;

/// 状态栏提示字符串的访问器（对应 C 的全局 prompt 变量） */

pub unsafe fn prompt_string() -> Option<String> {
    unsafe { PROMPTTEXT.clone() }
}

/// 设置状态栏提示字符串（对应 C prompt 赋值）
pub unsafe fn set_prompt_string(s: Option<String>) {
    unsafe { PROMPTTEXT = s; }
}

/// 读取状态栏输入答案中的光标位置（对C typing_x）
pub unsafe fn get_typing_x() -> usize {
    unsafe { TYPING_X }
}

/// 设置状态栏输入答案中的光标位置（对C typing_x 赋值）
pub unsafe fn set_typing_x(x: usize) {
    unsafe { TYPING_X = x; }
}



pub unsafe fn do_statusbar_home() {
    set_typing_x(0);
}



pub unsafe fn do_statusbar_end() {
    let len = unsafe { answer.as_ref().map_or(0, |a| a.len()) };
    set_typing_x(len);
}



pub unsafe fn do_statusbar_prev_word() {
    unsafe {
        let mut seen_a_word = false;
        let mut step_forward = false;

        while get_typing_x() != 0 {
            set_typing_x(get_typing_x().saturating_sub(1));
            let pos = get_typing_x();
            let byte = answer.as_ref().and_then(|a| a.as_bytes().get(pos).copied());
            if let Some(b) = byte {
                let ch = [b];
                if is_word_char(&ch, false) {
                    seen_a_word = true;
                } else if is_zerowidth(&ch) {
                    /* 跳过零宽字符 */
                } else if seen_a_word {
                    step_forward = true;
                    break;
                }
            }
        }

        if step_forward {
            let answer_ref = answer.as_ref().cloned().unwrap_or_default();
            set_typing_x(step_right(answer_ref.as_bytes(), get_typing_x()));
        }
    }
}

pub unsafe fn do_statusbar_next_word() {
    unsafe {
        let answer_ref = answer.as_ref().cloned().unwrap_or_default();
        let bytes = answer_ref.as_bytes();
        let mut idx = get_typing_x();
        let mut seen_space = idx < bytes.len() && !is_word_char(&bytes[idx..idx + 1], false);
        let mut seen_word = !seen_space;

        while idx < bytes.len() {
            idx = step_right(answer_ref.as_bytes(), idx);
            let ch = [bytes[idx]];
            if ISSET(AFTER_ENDS) {
                if is_word_char(&ch, false) {
                    seen_word = true;
                } else if is_zerowidth(&ch) {
                    /* 跳过零宽字符 */
                } else if seen_word {
                    break;
                }
            } else {
                if is_zerowidth(&ch) {
                    /* 跳过零宽字符 */
                } else if !is_word_char(&ch, false) {
                    seen_space = true;
                } else if seen_space {
                    break;
                }
            }
        }

        set_typing_x(idx);
    }
}

pub unsafe fn do_statusbar_left() {
    unsafe {
        if get_typing_x() > 0 {
            set_typing_x(get_typing_x() - 1);
        }
    }
}



pub unsafe fn do_statusbar_right() {
    unsafe {
        let len = answer.as_ref().map_or(0, |a| a.len());
        if get_typing_x() < len {
            set_typing_x(get_typing_x() + 1);
        }
    }
}



pub unsafe fn do_statusbar_backspace() {
    unsafe {
        if get_typing_x() > 0 {
            let new_x = get_typing_x() - 1;
            set_typing_x(new_x);
            if let Some(a) = answer.as_mut() {
                a.remove(new_x);
            }
        }
    }
}



pub unsafe fn do_statusbar_delete() {
    unsafe {
        let len = answer.as_ref().map_or(0, |a| a.len());
        if get_typing_x() < len {
            if let Some(a) = answer.as_mut() {
                a.remove(get_typing_x());
            }
        }
    }
}



pub unsafe fn lop_the_answer() {
    unsafe {
        let x = get_typing_x();
        if let Some(a) = answer.as_mut() {
            a.truncate(x);
        } else if x == 0 {
            answer = Some(String::new());
        }
    }
}



pub unsafe fn copy_the_answer() {
    unsafe {
        if let Some(a) = answer.as_ref() {
            if !a.is_empty() {
                free_lines(cutbuffer);
                let dummy = linestruct {
                    data: String::new(),
                    lineno: 0,
                    next: std::ptr::null_mut(),
                    prev: std::ptr::null_mut(),
                    multidata: None,
                    has_anchor: false,
                };
                cutbuffer = Box::into_raw(make_new_node(&dummy));
                (*cutbuffer).data = copy_of(a);
                set_typing_x(0);
            }
        }
    }
}



pub unsafe fn paste_into_answer() {
    unsafe {
        if !cutbuffer.is_null() {
            let pastelen = (*cutbuffer).data.len();
            let cur = answer.as_ref().map_or(0, |a| a.len());
            let mut new_answer = String::with_capacity(cur + pastelen);
            if let Some(a) = answer.as_ref() {
                new_answer.push_str(a);
            }
            new_answer.push_str(&(*cutbuffer).data);
            answer = Some(new_answer);
            set_typing_x(cur + pastelen);
        }
    }
}



pub unsafe fn process_prompt_click() -> i32 {
    let mut click_row: i32 = 0;
    let mut click_col: i32 = 0;
    let retval = get_mouseinput(&mut click_row, &mut click_col);

    unsafe {
        if retval == 0 && wmouse_trafo(footwin, &mut click_row, &mut click_col, false) {
            let prompt = prompt_string().unwrap_or_default();
            let start_col = breadth(prompt.as_bytes()) + 2;
            if (click_col as usize) >= start_col {
                let base = get_statusbar_page_start(
                    start_col,
                    start_col + wideness(answer.as_ref().unwrap_or(&String::new()).as_bytes(), get_typing_x()),
                );
                set_typing_x(actual_x(answer.as_ref().unwrap_or(&String::new()).as_bytes(), base + (click_col as usize) - start_col));
            } else {
                set_typing_x(0);
            }
        }
    }
    retval
}



pub unsafe fn inject_into_answer(burst: &mut [u8], count: usize) {
    unsafe {
        for index in 0..count {
            if index < burst.len() && burst[index] == 0 {
                burst[index] = b'\n';
            }
        }
        let cur = answer.as_ref().map_or(0, |a| a.len());
        let mut new_answer = String::with_capacity(cur + count);
        if let Some(a) = answer.as_ref() {
            new_answer.push_str(a);
        }
        for &b in burst.iter().take(count) {
            new_answer.push(b as char);
        }
        answer = Some(new_answer);
        set_typing_x(cur + count);
    }
}



pub unsafe fn do_statusbar_verbatim_input() {
    let mut count: usize = 1;
    let bytes = get_verbatim_kbinput(footwin, &mut count);
    if count > 0 && count < 999 {
        let slice = unsafe { std::slice::from_raw_parts(bytes, count) };
        inject_into_answer(&mut slice.to_vec(), count);
    } else if count == 0 {
        beep();
    }
    if !bytes.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(bytes as *mut std::ffi::c_char);
        }
    }
}



pub unsafe fn absorb_character(input: i32, function: Option<
unsafe fn()>) {

static mut PUDDLE: *mut u8 = std::ptr::null_mut();

static mut CAPACITY: usize = 8;

static mut DEPTH: usize = 0;

    unsafe {
        if function.is_none() {
            if (input < 0x20 && input != b'\t' as i32) || meta_key || input > 0xFF {
                beep();
            } else if !ISSET(RESTRICTED) || currmenu != MWRITEFILE
                || openfile.as_ref().map_or(true, |f| f.filename.as_ref().map_or(true, |s| s.is_empty()))
            {
                if DEPTH + 1 == CAPACITY {
                    CAPACITY *= 2;
                    PUDDLE = alloc_buffer(CAPACITY);
                } else if PUDDLE.is_null() {
                    PUDDLE = alloc_buffer(CAPACITY);
                }
                if !PUDDLE.is_null() {
                    *PUDDLE.add(DEPTH) = input as u8;
                    DEPTH += 1;
                }
            }
        }

        if DEPTH > 0 && (function.is_some() || waiting_keycodes() == 0) {
            let slice = std::slice::from_raw_parts(PUDDLE, DEPTH + 1);
            let mut vec = slice.to_vec();
            vec[DEPTH] = 0;
            inject_into_answer(&mut vec, DEPTH);
            DEPTH = 0;
        }
    }
}



pub unsafe fn handle_editing(function: Option<
fn()>) -> bool {
    if function == Some(do_left as
fn()) {
        do_statusbar_left();
    } else if function == Some(do_right as
fn()) {
        do_statusbar_right();
    } else if function == Some(to_prev_word as
fn()) {
        do_statusbar_prev_word();
    } else if function == Some(to_next_word as
fn()) {
        do_statusbar_next_word();
    } else if function == Some(do_home as
fn()) {
        do_statusbar_home();
    } else if function == Some(do_end as
fn()) {
        do_statusbar_end();
    } else if ISSET(RESTRICTED) && currmenu == MWRITEFILE
        && openfile.as_ref().map_or(false, |f| {
            f.filename.as_ref().map_or(false, |s| !s.is_empty())
        })
        && (function == Some(do_verbatim_input_safe as
fn())
            || function == Some(do_delete_safe as
fn())
            || function == Some(do_backspace_safe as
fn())
            || function == Some(cut_text as
fn())
            || function == Some(paste_text as
fn()))
    {
        
    } else if function == Some(do_verbatim_input_safe as
fn()) {
        do_statusbar_verbatim_input();
    } else if function == Some(do_delete_safe as
fn()) {
        do_statusbar_delete();
    } else if function == Some(do_backspace_safe as
fn()) {
        do_statusbar_backspace();
    } else if function == Some(cut_text as
fn()) {
        lop_the_answer();
    } else if function == Some(copy_text as
fn()) {
        copy_the_answer();
    } else if function == Some(paste_text as
fn()) {
        paste_into_answer();
    } else {
        return false;
    }

    true
}



pub unsafe fn get_statusbar_page_start(base: usize, column: usize) -> usize {
    if column == base || column < (COLS as usize).saturating_sub(1) {
        0
    } else if (COLS as usize) > base + 2 {
        column - base - 1 - (column - base - 1) % ((COLS as usize) - base - 2)
    } else {
        column - 2
    }
}



pub unsafe fn put_cursor_at_end_of_answer() {
    set_typing_x(HIGHEST_POSITIVE);
}



pub unsafe fn draw_the_promptbar() {
    let prompt = prompt_string().unwrap_or_default();
    let base = breadth(prompt.as_bytes()) + 2;
    let column = base + wideness(answer.as_ref().unwrap_or(&String::new()).as_bytes(), get_typing_x());
    let the_page = get_statusbar_page_start(base, column);
    let end_page = get_statusbar_page_start(base, base + breadth(answer.as_ref().unwrap_or(&String::new()).as_bytes()) - 1);

    wattron(footwin, interface_color_pair[PROMPT_BAR]);
    mvwprintw(footwin, 0, 0, "%*s", COLS, " ");
    mvwaddstr(footwin, 0, 0, &prompt);
    waddch(footwin, if the_page == 0 { ' ' } else { '<' });

    let expanded = display_string(answer.as_ref().unwrap_or(&String::new()).as_bytes(), the_page, (COLS as usize) - base, false, true);
    waddstr(footwin, &expanded);
    free_string(expanded);

    if the_page < end_page && base + breadth(answer.as_ref().unwrap_or(&String::new()).as_bytes()) - the_page > (COLS as usize) {
        mvwaddch(footwin, 0, (COLS - 1) as i32, '>');
    }

    wattroff(footwin, interface_color_pair[PROMPT_BAR]);

    wmove(footwin, 0, (column - the_page) as i32);
    wnoutrefresh(footwin);
}



pub unsafe fn add_or_remove_pipe_symbol_from_answer() {
    unsafe {
        if answer.as_ref().map_or(false, |a| a.starts_with('|')) {
            if let Some(a) = answer.as_mut() {
                a.remove(0);
            }
            if get_typing_x() > 0 {
                set_typing_x(get_typing_x() - 1);
            }
        } else {
            let cur = answer.as_ref().map_or(0, |a| a.len());
            let mut new_answer = String::with_capacity(cur + 1);
            new_answer.push('|');
            if let Some(a) = answer.as_ref() {
                new_answer.push_str(a);
            }
            answer = Some(new_answer);
            set_typing_x(cur + 1);
        }
    }
}



pub unsafe fn acquire_an_answer(actual: &mut i32, listed: &mut bool, mut history_list: *mut linestruct,
                         refresh_func:
unsafe
fn()) -> Option<
unsafe fn()> {
    let mut stored_string: Option<String> = None;
    let mut previous_was_tab = false;
    let mut fragment_length: usize = 0;
    let mut bracketed_paste = false;
    let mut shortcut: *mut keystruct = std::ptr::null_mut();
    let mut function: Option<
unsafe fn()> = None;
    let mut input: i32;

    if get_typing_x() > answer.as_ref().map_or(0, |a| a.len()) {
        set_typing_x(answer.as_ref().map_or(0, |a| a.len()));
    }

    'outer: loop {
        draw_the_promptbar();

        input = get_kbinput(footwin, VISIBLE);

        if input == THE_WINDOW_RESIZED {
            *actual = THE_WINDOW_RESIZED;
            stored_string = None;
            return None;
        }
        if input == START_OF_PASTE || input == END_OF_PASTE {
            bracketed_paste = input == START_OF_PASTE;
        }

        shortcut = get_shortcut(input);
        function = if shortcut.is_null() { None } else { unsafe { (*shortcut).func } };

        if input == b'\t' as i32 && bracketed_paste {
            function = None;
        }

        absorb_character(input, function);

        if bracketed_paste {
            if function.is_some() && function != Some(do_nothing) {
                beep();
            }
            continue 'outer;
        }

        if function == Some(do_cancel) || function == Some(do_enter_safe) {
            break 'outer;
        }

        if function == Some(do_tab_safe) {
            if !history_list.is_null() {
                if !previous_was_tab {
                    fragment_length = answer.as_ref().map_or(0, |a| a.len());
                }
                if fragment_length > 0 {
                    answer = get_history_completion(history_list, answer.as_ref().unwrap_or(&String::new()).as_str(), fragment_length);
                    set_typing_x(answer.as_ref().map_or(0, |a| a.len()));
                }
            } else if (currmenu & (MINSERTFILE | MWRITEFILE | MGOTODIR)) != 0 && !ISSET(RESTRICTED) {
                answer = Some(input_tab(answer.as_ref().unwrap_or(&String::new()).as_str(), &mut get_typing_x(), refresh_func, listed));
            }
        } else if function == Some(get_older_item) && !history_list.is_null() {
            if stored_string.is_none() {
                reset_history_pointer_for(history_list);
            }
            if !(*history_list).next.is_null() {
                history_list = (*history_list).prev;
                answer = Some(String::from_utf8_lossy(&mallocstrcpy(None::<Vec<u8>>, (*history_list).data.as_str().as_bytes())).into_owned());
                set_typing_x(answer.as_ref().map_or(0, |a| a.len()));
            }
        } else if function == Some(get_newer_item) && !history_list.is_null() {
            if !(*history_list).next.is_null() {
                history_list = (*history_list).next;
                answer = Some(String::from_utf8_lossy(&mallocstrcpy(None::<Vec<u8>>, (*history_list).data.as_str().as_bytes())).into_owned());
                set_typing_x(answer.as_ref().map_or(0, |a| a.len()));
            }
            if (*history_list).next.is_null() {
                if let Some(s) = stored_string.take() {
                    answer = Some(String::from_utf8_lossy(&mallocstrcpy(None::<Vec<u8>>, s.as_bytes())).into_owned());
                    set_typing_x(answer.as_ref().map_or(0, |a| a.len()));
                }
            }
        } else if function == Some(do_help) || function == Some(full_refresh) {
            if let Some(f) = function {
                f();
            }
        } else if function == Some(do_toggle) {
            if !shortcut.is_null() && unsafe { (*shortcut).toggle } == NO_HELP as i32 {
                TOGGLE(NO_HELP);
                window_init();
                focusing = false;
                refresh_func();
                bottombars(currmenu);
            }
        } else if function == Some(do_nothing) {
            
        } else if let Some(f) = function {
            if !ISSET(VIEW_MODE) || !changes_something(f) {
                if currmenu == MEXECUTE && f == do_enter_safe {
                    foretext = Some(String::from_utf8_lossy(&mallocstrcpy(foretext.take().map(|s| s.into_bytes()), answer.as_ref().unwrap_or(&String::new()).as_bytes())).into_owned());
                }
                f();
                break 'outer;
            } else {
                beep();
            }
        }

        previous_was_tab = function == Some(do_tab_safe);
    }

    if currmenu == MEXECUTE && function == Some(do_enter_safe) {
        foretext = Some(String::new());
    }
    if let Some(s) = stored_string {
        reset_history_pointer_for(history_list);
        free_string(s);
    }

    *actual = input;
    function
}



pub unsafe fn do_prompt(menu: i32, provided: &str, history_list: *mut linestruct,
                 refresh_func:
unsafe
fn(), msg: &str) -> i32 {
    let mut was_typing_x = get_typing_x();
    let saved_prompt = prompt_string();

    bottombars(menu);

    if answer.as_ref().map_or(true, |a| a.as_str() != provided) {
        answer = Some(provided.to_string());
    }

    let prompt_buf = format!("{:<width$}", msg, width = (COLS as usize).saturating_sub(5).max(1));
    set_prompt_string(Some(prompt_buf));

    lastmessage = message_type::VACUUM;

    let mut listed = false;
    let mut actual: i32 = 0;
    let function = acquire_an_answer(&mut actual, &mut listed, history_list, refresh_func);

    set_prompt_string(None);

    if actual == THE_WINDOW_RESIZED {
        return do_prompt(menu, provided, history_list, refresh_func, msg);
    }

    set_prompt_string(saved_prompt);

    if function == Some(do_cancel) || function == Some(do_enter_safe)
        || function == Some(to_first_file) || function == Some(to_last_file)
        || function == Some(to_first_line) || function == Some(to_last_line)
    {
        set_typing_x(was_typing_x);
    }

    let retval = if function == Some(do_cancel) {
        -1
    } else if function == Some(do_enter_safe) {
        if answer.as_ref().map_or(true, |a| a.is_empty()) { -2 } else { 0 }
    } else {
        actual
    };

    if lastmessage == message_type::VACUUM {
        wipe_statusbar();
    }

    if listed {
        refresh_func();
    }

    retval
}



pub unsafe fn ask_user(withall: bool, question: &str) -> i32 {
    let mut choice: i32 = UNDECIDED;
    let mut width: i32 = 16;
    let yesstr = crate::gettext!("Yy");
    let nostr = crate::gettext!("Nn");
    let allstr = crate::gettext!("Aa");
    let mut shortcut: *mut keystruct = std::ptr::null_mut();
    let mut function: Option<
unsafe fn()> = None;
    let mut kbinput: i32;

    while choice == UNDECIDED {
        if !ISSET(NO_HELP) {
            let mut shortstr = [0u8; (MAXCHARLEN + 2) as usize];
            let cancelshortcut = first_sc_for(MYESNO, do_cancel as
fn());
            if (COLS as i32) < 32 {
                width = COLS / 2;
            }
            blank_bottombars();
            shortstr[0] = b' ';
            shortstr[1] = yesstr.as_bytes()[0];
            wmove(footwin, 1, 0);
            post_one_key(std::str::from_utf8(&shortstr[..2]).unwrap_or(""), crate::gettext!("Yes"), width);
            shortstr[1] = nostr.as_bytes()[0];
            wmove(footwin, 2, 0);
            post_one_key(std::str::from_utf8(&shortstr[..2]).unwrap_or(""), crate::gettext!("No"), width);
            if withall {
                shortstr[1] = allstr.as_bytes()[0];
                wmove(footwin, 1, width);
                post_one_key(std::str::from_utf8(&shortstr[..2]).unwrap_or(""), crate::gettext!("All"), width);
            }
            wmove(footwin, 2, width);
            if !cancelshortcut.is_null() {
                post_one_key(unsafe { &(*cancelshortcut).keystr }, crate::gettext!("Cancel"), width);
            }
        }

        wattron(footwin, interface_color_pair[PROMPT_BAR]);
        mvwprintw(footwin, 0, 0, "%*s", COLS, " ");
        mvwaddnstr(footwin, 0, 0, question, actual_x(question.as_bytes(), (COLS as usize).saturating_sub(1)));
        wattroff(footwin, interface_color_pair[PROMPT_BAR]);
        wnoutrefresh(footwin);

        currmenu = MYESNO;

        kbinput = get_kbinput(footwin, !withall);

        if kbinput == THE_WINDOW_RESIZED {
            continue;
        }

        if kbinput == START_OF_PASTE {
            let _ = get_kbinput(footwin, BLIND);
            while get_kbinput(footwin, BLIND) != END_OF_PASTE {}
        }

        let mut letter = [0u8; (MAXCHARLEN + 1) as usize];
        letter[0] = kbinput as u8;
        let mut index = 1;
        if (0xC0..=0xF7).contains(&kbinput) && using_utf8 {
            let extras = (kbinput / 16) % 4 + if kbinput <= 0xCF { 1 } else { 0 };
            while (extras as usize) <= waiting_keycodes() as usize && extras > 0 {
                letter[index] = get_kbinput(footwin, !withall) as u8;
                index += 1;
            }
        }
        letter[index] = 0;

        if strstr(yesstr.as_bytes(), &letter[..index]) {
            choice = YES;
        } else if strstr(nostr.as_bytes(), &letter[..index]) {
            choice = NO;
        } else if withall && strstr(allstr.as_bytes(), &letter[..index]) {
            choice = ALL;
        } else {
            shortcut = get_shortcut(kbinput);
            function = if shortcut.is_null() { None } else { unsafe { (*shortcut).func } };
            if function == Some(do_cancel) {
                choice = CANCEL;
            } else if function == Some(full_refresh) {
                if let Some(f) = function {
                    f();
                }
            } else if function == Some(do_toggle) && !shortcut.is_null()
                && unsafe { (*shortcut).toggle } == NO_HELP as i32
            {
                TOGGLE(NO_HELP);
                window_init();
                titlebar(None);
                focusing = false;
                edit_refresh();
                focusing = true;
            } else if kbinput == 0x0E
                || (kbinput == 0x11 && !ISSET(MODERN_BINDINGS))
                || (kbinput == 0x18 && ISSET(MODERN_BINDINGS))
            {
                choice = NO;
                if kbinput != 0x0E {
                    final_status = 2;
                }
            } else if kbinput == 0x19 {
                choice = YES;
            } else if withall && kbinput == 0x01 {
                choice = ALL;
            } else {
                beep();
            }
        }
    }

    choice
}

/// 未决状态的占位常量（对C UNDECIDED）pub
const UNDECIDED: i32 = -2;

/* ===== 以下为尚未翻译的 ncurses / history 辅助函数的占位桩，待后续模块落地后移除 ===== */

pub use crate::winio::{
    get_mouseinput, mvwaddstr, mvwprintw, post_one_key, waiting_keycodes, wattroff, wattron,
    wmouse_trafo,
};
pub fn waddch(_win: *mut std::ffi::c_void, _ch: char) {}
pub fn mvwaddnstr(_win: *mut std::ffi::c_void, _y: i32, _x: i32, _s: &str, _n: usize) {}
pub fn mvwaddch(_win: *mut std::ffi::c_void, _y: i32, _x: i32, _ch: char) {}
pub fn wrefresh(_win: *mut std::ffi::c_void) {}
pub fn get_history_completion(_history: *mut linestruct, _answer: &str, _len: usize) -> Option<String> { None }
pub fn changes_something(_f: unsafe fn()) -> bool { true }
pub fn strstr(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}
pub fn alloc_buffer(_size: usize) -> *mut u8 { std::ptr::null_mut() }
pub fn free_string(_s: String) {}

