/**************************************************************************
 *   global.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 global.c）。
 *
 *   版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2014-2026 Benno Schulenberg
 **************************************************************************/

//! 全局变量、快捷键与函数列表的初始化。对应原版 nano 的 `global.c`。

use crate::definitions::*;

/* ===== 全局变量（对应 global.c 中的全局变量） ===== */

#[allow(dead_code)]
pub static mut the_window_resized: bool = false;
#[allow(dead_code)]
pub static mut resized_for_browser: bool = false;

pub static mut on_a_vt: bool = false;
pub static mut shifted_metas: bool = false;
pub static mut meta_key: bool = false;
pub static mut shift_held: bool = false;
pub static mut mute_modifiers: bool = false;
pub static mut we_are_running: bool = false;
pub static mut more_than_one: bool = false;
pub static mut report_size: bool = true;
pub static mut ran_a_tool: bool = false;
pub static mut foretext: Option<String> = None;
pub static mut final_status: i32 = 0;
pub static mut inhelp: bool = false;
pub static mut title: Option<String> = None;
pub static mut refresh_needed: bool = false;
pub static mut focusing: bool = true;
pub static mut control_C_was_pressed: bool = false;
pub static mut lastmessage: message_type = message_type::VACUUM;
pub static mut pletion_line: *mut linestruct = std::ptr::null_mut();
pub static mut answer: Option<String> = None;
pub static mut last_search: Option<String> = None;
pub static mut didfind: i32 = 0;
pub static mut present_path: Option<String> = None;

#[allow(dead_code)]
pub static mut controlleft: i32 = 0;
pub static mut controlright: i32 = 0;
pub static mut controlup: i32 = 0;
pub static mut controldown: i32 = 0;
pub static mut controlhome: i32 = 0;
pub static mut controlend: i32 = 0;
#[allow(dead_code)]
pub static mut controldelete: i32 = 0;
#[allow(dead_code)]
pub static mut controlshiftdelete: i32 = 0;
#[allow(dead_code)]
pub static mut shiftup: i32 = 0;
#[allow(dead_code)]
pub static mut shiftdown: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontrolleft: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontrolright: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontrolup: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontroldown: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontrolhome: i32 = 0;
#[allow(dead_code)]
pub static mut shiftcontrolend: i32 = 0;
#[allow(dead_code)]
pub static mut altleft: i32 = 0;
#[allow(dead_code)]
pub static mut altright: i32 = 0;
#[allow(dead_code)]
pub static mut altup: i32 = 0;
#[allow(dead_code)]
pub static mut altdown: i32 = 0;
#[allow(dead_code)]
pub static mut althome: i32 = 0;
#[allow(dead_code)]
pub static mut altend: i32 = 0;
#[allow(dead_code)]
pub static mut altpageup: i32 = 0;
#[allow(dead_code)]
pub static mut altpagedown: i32 = 0;
#[allow(dead_code)]
pub static mut altinsert: i32 = 0;
#[allow(dead_code)]
pub static mut altdelete: i32 = 0;
#[allow(dead_code)]
pub static mut shiftaltleft: i32 = 0;
#[allow(dead_code)]
pub static mut shiftaltright: i32 = 0;
#[allow(dead_code)]
pub static mut shiftaltup: i32 = 0;
#[allow(dead_code)]
pub static mut shiftaltdown: i32 = 0;
pub static mut mousefocusin: i32 = 0;
pub static mut mousefocusout: i32 = 0;

#[allow(dead_code)]
pub static mut fill: isize = -(COLUMNS_FROM_EOL as isize);
#[allow(dead_code)]
pub static mut wrap_at: usize = 0;

#[allow(dead_code)]
pub static mut topwin: *mut std::ffi::c_void = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut midwin: *mut std::ffi::c_void = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut footwin: *mut std::ffi::c_void = std::ptr::null_mut();
pub static mut editwinrows: i32 = 0;
pub static mut margin: i32 = 0;
pub static mut sidebar: i32 = 0;
#[allow(dead_code)]
pub static mut bardata: *mut i32 = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut stripe_column: isize = 0;
#[allow(dead_code)]
pub static mut cycling_aim: i32 = 0;

pub static mut cutbuffer: *mut linestruct = std::ptr::null_mut();
pub static mut cutbottom: *mut linestruct = std::ptr::null_mut();
pub static mut keep_cutbuffer: bool = false;

#[allow(dead_code)]
pub static mut startfile: *mut openfilestruct = std::ptr::null_mut();

#[allow(dead_code)]
pub static mut matchbrackets: Option<String> = None;
#[allow(dead_code)]
pub static mut whitespace: Option<String> = None;
#[allow(dead_code)]
pub static mut whitelen: [i32; 2] = [0; 2];

#[allow(dead_code)]
pub static mut punct: Option<String> = None;
#[allow(dead_code)]
pub static mut brackets: Option<String> = None;
#[allow(dead_code)]
pub static mut quotereg: Option<Box<regex::Regex>> = None;
#[allow(dead_code)]
pub static mut quotestr: Option<String> = None;

pub static mut tabsize: isize = -1;

#[allow(dead_code)]
pub static mut backup_dir: Option<String> = None;
#[allow(dead_code)]
pub static mut operating_dir: Option<String> = None;

#[allow(dead_code)]
pub static mut alt_speller: Option<String> = None;

#[allow(dead_code)]
pub static mut syntaxes: *mut syntaxtype = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut syntaxstr: Option<String> = None;
#[allow(dead_code)]
pub static mut have_palette: bool = false;
#[allow(dead_code)]
pub static mut rescind_colors: bool = false;
#[allow(dead_code)]
pub static mut perturbed: bool = false;
#[allow(dead_code)]
pub static mut recook: bool = false;

pub static mut currmenu: i32 = MMOST;
pub static mut sclist: *mut keystruct = std::ptr::null_mut();
pub static mut allfuncs: *mut funcstruct = std::ptr::null_mut();
pub static mut tailfunc: *mut funcstruct = std::ptr::null_mut();
pub static mut exitfunc: *mut funcstruct = std::ptr::null_mut();

pub static mut search_history: *mut linestruct = std::ptr::null_mut();
pub static mut replace_history: *mut linestruct = std::ptr::null_mut();
pub static mut execute_history: *mut linestruct = std::ptr::null_mut();

#[allow(dead_code)]
pub static mut searchtop: *mut linestruct = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut searchbot: *mut linestruct = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut replacetop: *mut linestruct = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut replacebot: *mut linestruct = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut executetop: *mut linestruct = std::ptr::null_mut();
#[allow(dead_code)]
pub static mut executebot: *mut linestruct = std::ptr::null_mut();

#[allow(dead_code)]
pub static mut hilite_attribute: i32 = 0;
#[allow(dead_code)]
pub static mut color_combo: [*mut colortype; NUMBER_OF_ELEMENTS] = [std::ptr::null_mut(); NUMBER_OF_ELEMENTS];
#[allow(dead_code)]
pub static mut interface_color_pair: [i32; NUMBER_OF_ELEMENTS] = [0; NUMBER_OF_ELEMENTS];

#[allow(dead_code)]
pub static mut statedir: Option<String> = None;
#[allow(dead_code)]
pub static mut startup_problem: Option<String> = None;
#[allow(dead_code)]
pub static mut custom_nanorc: Option<String> = None;
#[allow(dead_code)]
pub static mut commandname: Option<String> = None;
#[allow(dead_code)]
pub static mut planted_shortcut: *mut keystruct = std::ptr::null_mut();

pub static mut spotlighted: bool = false;
pub static mut light_from_col: usize = 0;
pub static mut light_to_col: usize = 0;

/* ===== 空操作函数（对应原版 global.c 中的 void 占位函数） ===== */

pub fn case_sens_void() {}
pub fn regexp_void() {}
pub fn backwards_void() {}
#[allow(dead_code)]
pub fn get_older_item() {}
#[allow(dead_code)]
pub fn get_newer_item() {}
pub fn flip_replace() {}
pub fn flip_goto() {}
#[allow(dead_code)]
pub fn to_files() {}
#[allow(dead_code)]
pub fn goto_dir() {}
#[allow(dead_code)]
pub fn do_nothing() {}
#[allow(dead_code)]
pub fn do_toggle() {}
#[allow(dead_code)]
pub fn dos_format() {}
#[allow(dead_code)]
pub fn append_it() {}
#[allow(dead_code)]
pub fn prepend_it() {}
#[allow(dead_code)]
pub fn back_it_up() {}
#[allow(dead_code)]
pub fn flip_execute() {}
#[allow(dead_code)]
pub fn flip_pipe() {}
#[allow(dead_code)]
pub fn flip_convert() {}
#[allow(dead_code)]
pub fn flip_newbuffer() {}
pub fn discard_buffer() {}
pub fn do_cancel() {}
pub fn suck_up_input_and_paste_it() {}

/* ===== 命令函数空桩（后续模块实现后改为真实引用） ===== */

pub fn do_help() {}
pub fn do_exit() {}
pub fn full_refresh() {}
pub fn do_writeout() {}
pub fn do_insertfile() {}
pub fn do_justify() {}
pub fn do_search_forward() {}
pub fn do_replace() {}
pub fn do_search_backward() {}
pub fn do_findprevious() {}
pub fn do_findnext() {}
pub fn cut_text() {}
pub fn paste_text() {}
pub fn do_execute() {}
pub fn report_cursor_position() {}
pub fn do_gotolinecolumn() {}
pub fn do_undo() {}
pub fn do_redo() {}
pub fn do_mark() {}
pub fn copy_text() {}
pub fn to_prev_word() {}
pub fn to_next_word() {}
pub fn do_find_bracket() {}
pub fn to_first_line() {}
pub fn to_last_line() {}
pub fn do_left() {}
pub fn do_right() {}
pub fn do_home() {}
pub fn do_end() {}
pub fn do_scroll_left() {}
pub fn do_scroll_right() {}
pub fn do_up() {}
pub fn do_down() {}
pub fn do_scroll_up() {}
pub fn do_scroll_down() {}
pub fn to_prev_block() {}
pub fn to_next_block() {}
pub fn to_para_begin() {}
pub fn to_para_end() {}
pub fn to_top_row() {}
pub fn to_bottom_row() {}
pub fn do_page_up() {}
pub fn do_page_down() {}
pub fn count_lines_words_and_characters() {}
pub fn do_verbatim_input() {}
pub fn do_indent() {}
pub fn do_unindent() {}
pub fn cut_till_eof() {}
pub fn do_full_justify() {}
pub fn do_comment() {}
pub fn complete_a_word() {}
pub fn record_macro() {}
pub fn run_macro() {}
pub fn zap_text() {}
pub fn put_or_lift_anchor() {}
pub fn to_prev_anchor() {}
pub fn to_next_anchor() {}
pub fn do_spell() {}
pub fn do_linter() {}
pub fn do_formatter() {}
#[allow(dead_code)]
pub fn suggest_ctrlT_ctrlZ() {}
pub fn do_center() {}
pub fn do_cycle() {}
pub fn do_savefile() {}
#[allow(dead_code)]
pub fn to_files_stub() {}
#[allow(dead_code)]
pub fn show_curses_version() {}

/* ===== 链表辅助函数 ===== */

/* Add a function to the linked list of functions. */
pub unsafe fn add_to_funcs(
    function: unsafe fn(),
    menus: i32,
    tag: &'static str,
    phrase: &'static str,
    _blank_after: bool,
) {
    let f = Box::new(funcstruct {
        func: Some(function),
        tag,
        phrase,
        blank_after: _blank_after,
        menus,
        next: std::ptr::null_mut(),
    });

    if allfuncs.is_null() {
        allfuncs = Box::into_raw(f);
        tailfunc = allfuncs;
    } else {
        (*tailfunc).next = Box::into_raw(f);
        tailfunc = (*tailfunc).next;
    }
}

/* Parse the given keystring and return the corresponding keycode,
 * or return -1 when the string is invalid. */
pub fn keycode_from_string(keystring: &str) -> i32 {
    let bytes = keystring.as_bytes();
    if bytes.is_empty() {
        return -1;
    }
    if bytes[0] == b'^' {
        if bytes.len() == 2 {
            if bytes[1] == b'/' || bytes[1] == b'-' {
                return 31;
            }
            let c = bytes[1] as i8;
            if c <= b'_' as i8 {
                return (c as i32) - 64;
            }
            if bytes[1] == b'`' {
                return 0;
            }
            return -1;
        } else if keystring.eq_ignore_ascii_case("^Space") {
            return 0;
        } else {
            return -1;
        }
    } else if bytes[0] == b'M' {
        if bytes.len() == 3 && bytes[1] == b'-' {
            if b'A' as u8 <= bytes[2] && bytes[2] <= b'Z' as u8 {
                return (bytes[2] | 0x20) as i32;
            } else {
                return bytes[2] as i32;
            }
        }
        if keystring.eq_ignore_ascii_case("M-Space") {
            return ' ' as i32;
        } else if keystring.eq_ignore_ascii_case("M-Left") {
            return ALT_LEFT;
        } else if keystring.eq_ignore_ascii_case("M-Right") {
            return ALT_RIGHT;
        } else if keystring.eq_ignore_ascii_case("M-Up") {
            return ALT_UP;
        } else if keystring.eq_ignore_ascii_case("M-Down") {
            return ALT_DOWN;
        } else if keystring.eq_ignore_ascii_case("M-Ins") {
            return ALT_INSERT;
        } else if keystring.eq_ignore_ascii_case("M-Del") {
            return ALT_DELETE;
        } else {
            return -1;
        }
    } else if bytes[0] == b'F' {
        let fn_num = keystring[1..].parse::<i32>().unwrap_or(-1);
        if fn_num < 1 || fn_num > 24 {
            return -1;
        }
        return KEY_F0 + fn_num;
    } else if keystring.eq_ignore_ascii_case("Ins") {
        return KEY_IC;
    } else if keystring.eq_ignore_ascii_case("Del") {
        return KEY_DC;
    } else {
        return -1;
    }
}

/* 维护快捷键链表尾指针。 */
static mut TAIL_SC: *mut keystruct = std::ptr::null_mut();

/* Add a key combo to the linked list of shortcuts. */
pub unsafe fn add_to_sclist(menus: i32, scstring: &'static str, keycode: i32, function: unsafe fn(), toggle: i32) {
    let sc = Box::new(keystruct {
        keystr: scstring,
        keycode: if keycode != 0 { keycode } else { keycode_from_string(scstring) },
        menus,
        func: Some(function),
        toggle,
        ordinal: 0,
        expansion: None,
        next: std::ptr::null_mut(),
    });

    let raw = Box::into_raw(sc);
    if sclist.is_null() {
        sclist = raw;
    } else {
        (*TAIL_SC).next = raw;
    }
    TAIL_SC = raw;
}

/* Return the first shortcut in the list of shortcuts that
 * matches the given function in the given menu. */
pub unsafe fn first_sc_for(menu: i32, function: unsafe fn()) -> *mut keystruct {
    let mut sc = sclist;
    while !sc.is_null() {
        let ks = &*sc;
        if (ks.menus & menu) != 0 && ks.func == Some(function) && !ks.keystr.is_empty() {
            return sc;
        }
        sc = ks.next;
    }
    std::ptr::null_mut()
}

/* Return the number of entries that can be shown in the given menu. */
pub unsafe fn shown_entries_for(menu: i32) -> usize {
    let mut item = allfuncs;
    let maximum: usize = ((get_cols() + 40) / 20) * 2;
    let mut count: usize = 0;

    while !item.is_null() && count < maximum {
        if (*item).menus & menu != 0 {
            count += 1;
        }
        item = (*item).next;
    }

    if menu == MWRITEFILE && item.is_null() && first_sc_for(menu, discard_buffer).is_null() {
        count -= 1;
    }

    count
}

/* Return the first shortcut in the current menu that matches the given input. */
pub unsafe fn get_shortcut(keycode: i32) -> *mut keystruct {
    if !meta_key && 0x20 <= keycode && keycode <= 0xFF {
        return std::ptr::null_mut();
    }

    if meta_key && keycode < 0x20 {
        return std::ptr::null_mut();
    }

    let mut sc = sclist;
    while !sc.is_null() {
        let ks = &*sc;
        if (ks.menus & currmenu) != 0 && keycode == ks.keycode {
            return sc;
        }
        sc = ks.next;
    }

    std::ptr::null_mut()
}

/* Return a pointer to the function that is bound to the given key. */
pub unsafe fn func_from_key(keycode: i32) -> Option<unsafe fn()> {
    let sc = get_shortcut(keycode);
    if sc.is_null() {
        None
    } else {
        (*sc).func
    }
}

/* 返回 COLS（终端列数）。占位实现：返回 80。 */
pub fn get_cols() -> usize {
    80
}

/* 在后续模块实现后，此函数将被真正的 shortcut_init 取代；此处仅声明签名。 */
/* 初始化函数列表与快捷键列表。对应原版 nano 的 shortcut_init（全功能构建）。 */
pub fn shortcut_init() {
    unsafe {
    let help_key = "^H";
    let slash_or_dash = if on_a_vt { "^-" } else { "^/" };

        let cancel_gist = "Cancel the current function";

        let help_gist = "Display this help text";
        let exit_gist = "Close the current buffer / Exit from nano";
        let writeout_gist = "Write the current buffer (or the marked region) to disk";
        let readfile_gist = "Insert another file into current buffer (or into new buffer)";
        let whereis_gist = "Search forward for a string or a regular expression";
        let wherewas_gist = "Search backward for a string or a regular expression";
        let cut_gist = "Cut current line (or marked region) and store it in cutbuffer";
        let copy_gist = "Copy current line (or marked region) and store it in cutbuffer";
        let paste_gist = "Paste the contents of cutbuffer at current cursor position";
        let cursorpos_gist = "Display the position of the cursor";
        let spell_gist = "Invoke the spell checker, if available";
        let replace_gist = "Replace a string or a regular expression";
        let gotoline_gist = "Go to line and column number";
        let bracket_gist = "Go to the matching bracket";
        let mark_gist = "Mark text starting from the cursor position";
        let zap_gist = "Throw away the current line (or marked region)";
        let indent_gist = "Indent the current line (or marked lines)";
        let unindent_gist = "Unindent the current line (or marked lines)";
        let undo_gist = "Undo the last operation";
        let redo_gist = "Redo the last undone operation";
        let back_gist = "Go back one character";
        let forward_gist = "Go forward one character";
        let prevword_gist = "Go back one word";
        let nextword_gist = "Go forward one word";
        let prevline_gist = "Go to previous line";
        let nextline_gist = "Go to next line";
        let home_gist = "Go to beginning of current line";
        let end_gist = "Go to end of current line";
        let prevblock_gist = "Go to previous block of text";
        let nextblock_gist = "Go to next block of text";
        let parabegin_gist = "Go to beginning of paragraph; then of previous paragraph";
        let paraend_gist = "Go just beyond end of paragraph; then of next paragraph";
        let toprow_gist = "Go to first row in the viewport";
        let bottomrow_gist = "Go to last row in the viewport";
        let center_gist = "Center the line where the cursor is";
        let cycle_gist = "Push the cursor line to the center, then top, then bottom";
        let prevpage_gist = "Go one screenful up";
        let nextpage_gist = "Go one screenful down";
        let firstline_gist = "Go to the first line of the file";
        let lastline_gist = "Go to the last line of the file";
        let scrollleft_gist = "Scroll the viewport a tabsize to the left";
        let scrollright_gist = "Scroll the viewport a tabsize to the right";
        let scrollup_gist = "Scroll up one line without moving the cursor textually";
        let scrolldown_gist = "Scroll down one line without moving the cursor textually";
        let prevfile_gist = "Switch to the previous file buffer";
        let nextfile_gist = "Switch to the next file buffer";
        let verbatim_gist = "Insert the next keystroke verbatim";
        let tab_gist = "Insert a tab at the cursor position (or indent marked lines)";
        let enter_gist = "Insert a newline at the cursor position";
        let delete_gist = "Delete the character under the cursor";
        let backspace_gist = "Delete the character to the left of the cursor";
        let chopwordleft_gist = "Delete backward from cursor to word start";
        let chopwordright_gist = "Delete forward from cursor to next word start";
        let cuttilleof_gist = "Cut from the cursor position to the end of the file";
        let justify_gist = "Justify the current paragraph";
        let fulljustify_gist = "Justify the entire file";
        let wordcount_gist = "Count the number of lines, words, and characters";
        let suspend_gist = "Suspend the editor (return to the shell)";
        let refresh_gist = "Refresh (redraw) the current screen";
        let completion_gist = "Try and complete the current word";
        let comment_gist = "Comment/uncomment the current line (or marked lines)";
        let savefile_gist = "Save file without prompting";
        let findprev_gist = "Search next occurrence backward";
        let findnext_gist = "Search next occurrence forward";
        let recordmacro_gist = "Start/stop recording a macro";
        let runmacro_gist = "Run the last recorded macro";
        let anchor_gist = "Place or remove an anchor at the current line";
        let prevanchor_gist = "Jump backward to the nearest anchor";
        let nextanchor_gist = "Jump forward to the nearest anchor";
        let case_gist = "Toggle the case sensitivity of the search";
        let reverse_gist = "Reverse the direction of the search";
        let regexp_gist = "Toggle the use of regular expressions";
        let older_gist = "Recall the previous search/replace string";
        let newer_gist = "Recall the next search/replace string";
        let dos_gist = "Toggle the use of DOS format";
        let append_gist = "Toggle appending";
        let prepend_gist = "Toggle prepending";
        let backup_gist = "Toggle backing up of the original file";
        let execute_gist = "Execute a function or an external command";
        let pipe_gist = "Pipe the current buffer (or marked region) to the command";
        let older_command_gist = "Recall the previous command";
        let newer_command_gist = "Recall the next command";
        let convert_gist = "Do not convert from DOS format";
        let newbuffer_gist = "Toggle the use of a new buffer";
        let discardbuffer_gist = "Close buffer without saving it";
        let tofiles_gist = "Go to file browser";
        let exitbrowser_gist = "Exit from the file browser";
        let firstfile_gist = "Go to the first file in the list";
        let lastfile_gist = "Go to the last file in the list";
        let backfile_gist = "Go to the previous file in the list";
        let forwardfile_gist = "Go to the next file in the list";
        let browserlefthand_gist = "Go to lefthand column";
        let browserrighthand_gist = "Go to righthand column";
        let browsertoprow_gist = "Go to first row in this column";
        let browserbottomrow_gist = "Go to last row in this column";
        let browserwhereis_gist = "Search forward for a string";
        let browserwherewas_gist = "Search backward for a string";
        let browserrefresh_gist = "Refresh the file list";
        let gotodir_gist = "Go to directory";
        let lint_gist = "Invoke the linter, if available";
        let prevlint_gist = "Go to previous linter msg";
        let nextlint_gist = "Go to next linter msg";
        let formatter_gist = "Invoke a program to format/arrange/manipulate the buffer";
        add_to_funcs(crate::help::do_help, (MMOST | MBROWSER) & !MFINDINHELP, "Help", help_gist, false);
        add_to_funcs(crate::text::do_cancel, ((MMOST & !MMAIN) | MYESNO), "Cancel", cancel_gist, true);
        add_to_funcs(crate::global::do_exit, MMAIN, "Exit", exit_gist, false);
        add_to_funcs(crate::global::do_exit, MBROWSER, "Close", exitbrowser_gist, false);
        add_to_funcs(crate::files::do_writeout, MMAIN, "Write Out", writeout_gist, false);
        add_to_funcs(crate::winio::full_refresh, MHELP, "Refresh", "x", true);
        add_to_funcs(crate::global::do_exit, MHELP, "Close", "x", true);
        add_to_funcs(crate::search::do_search_forward, MMAIN|MHELP, "Where Is", whereis_gist, false);
        add_to_funcs(crate::search::do_replace, MMAIN, "Replace", replace_gist, false);
        add_to_funcs(crate::cut::cut_text, MMAIN, "Cut", cut_gist, false);
        add_to_funcs(crate::cut::paste_text, MMAIN, "Paste", paste_gist, true);
            add_to_funcs(crate::files::do_execute, MMAIN, "Execute", execute_gist, false);
            add_to_funcs(crate::text::do_justify, MMAIN, "Justify", justify_gist, true);
        add_to_funcs(crate::global::report_cursor_position, MMAIN, "Location", cursorpos_gist, false);
        add_to_funcs(crate::search::do_gotolinecolumn, MMAIN, "Go To Line", gotoline_gist, true);
        add_to_funcs(crate::text::do_undo, MMAIN, "Undo", undo_gist, false);
        add_to_funcs(crate::text::do_redo, MMAIN, "Redo", redo_gist, true);
        add_to_funcs(crate::text::do_mark, MMAIN, "Set Mark", mark_gist, false);
        add_to_funcs(crate::cut::copy_text, MMAIN, "Copy", copy_gist, true);
        add_to_funcs(crate::global::case_sens_void, MWHEREIS|MREPLACE, "Case sensitive", case_gist, false);
        add_to_funcs(crate::global::regexp_void, MWHEREIS|MREPLACE, "Reg.expression", regexp_gist, false);
        add_to_funcs(crate::global::backwards_void, MWHEREIS|MREPLACE, "Backwards", reverse_gist, true);
        add_to_funcs(crate::global::flip_replace, MWHEREIS, "Replace", replace_gist, true);
        add_to_funcs(crate::global::flip_replace, MREPLACE, "No Replace", whereis_gist, true);
        add_to_funcs(crate::global::get_older_item, MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE, "Older", older_gist, false);
        add_to_funcs(crate::global::get_newer_item, MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE, "Newer", newer_gist, true);
        add_to_funcs(crate::global::get_older_item, MEXECUTE, "Older", older_command_gist, false);
        add_to_funcs(crate::global::get_newer_item, MEXECUTE, "Newer", newer_command_gist, true);
        add_to_funcs(crate::global::goto_dir, MBROWSER, "Go To Dir", gotodir_gist, false);
        add_to_funcs(crate::winio::full_refresh, MBROWSER, "Refresh", browserrefresh_gist, true);
        add_to_funcs(crate::search::do_search_forward, MBROWSER, "Where Is", browserwhereis_gist, false);
        add_to_funcs(crate::search::do_search_backward, MBROWSER, "Where Was", browserwherewas_gist, false);
        add_to_funcs(crate::search::do_findprevious, MBROWSER, "Previous", findprev_gist, false);
        add_to_funcs(crate::search::do_findnext, MBROWSER, "Next", findnext_gist, true);
        add_to_funcs(crate::search::do_find_bracket, MMAIN, "To Bracket", bracket_gist, true);
        add_to_funcs(crate::search::do_search_backward, MMAIN|MHELP, "Where Was", wherewas_gist, false);
        add_to_funcs(crate::search::do_findprevious, MMAIN|MHELP, "Previous", findprev_gist, false);
        add_to_funcs(crate::search::do_findnext, MMAIN|MHELP, "Next", findnext_gist, true);
        add_to_funcs(crate::r#move::do_left, MMAIN, "Back", back_gist, false);
        add_to_funcs(crate::r#move::do_right, MMAIN, "Forward", forward_gist, false);
        add_to_funcs(crate::r#move::do_left, MBROWSER, "Back", backfile_gist, false);
        add_to_funcs(crate::r#move::do_right, MBROWSER, "Forward", forwardfile_gist, false);
        add_to_funcs(crate::r#move::to_prev_word, MMAIN, "Prev Word", prevword_gist, false);
        add_to_funcs(crate::r#move::to_next_word, MMAIN, "Next Word", nextword_gist, false);
        add_to_funcs(crate::r#move::do_home, MMAIN, "Home", home_gist, false);
        add_to_funcs(crate::r#move::do_end, MMAIN, "End", end_gist, false);
        add_to_funcs(crate::global::do_scroll_left, MMAIN, "Scroll Left", scrollleft_gist, false);
        add_to_funcs(crate::global::do_scroll_right, MMAIN, "Scroll Right", scrollright_gist, true);
        add_to_funcs(crate::r#move::do_up, MMAIN|MBROWSER|MHELP, "Prev Line", prevline_gist, false);
        add_to_funcs(crate::r#move::do_down, MMAIN|MBROWSER|MHELP, "Next Line", nextline_gist, false);
        add_to_funcs(crate::r#move::do_scroll_up, MMAIN, "Scroll Up", scrollup_gist, false);
        add_to_funcs(crate::r#move::do_scroll_down, MMAIN, "Scroll Down", scrolldown_gist, true);
        add_to_funcs(crate::r#move::to_prev_block, MMAIN, "Prev Block", prevblock_gist, false);
        add_to_funcs(crate::r#move::to_next_block, MMAIN, "Next Block", nextblock_gist, false);
        add_to_funcs(crate::r#move::to_para_begin, MMAIN|MGOTOLINE, "Start of Paragraph", parabegin_gist, false);
        add_to_funcs(crate::r#move::to_para_end, MMAIN|MGOTOLINE, "End of Paragraph", paraend_gist, true);
        add_to_funcs(crate::r#move::to_top_row, MMAIN, "Top Row", toprow_gist, false);
        add_to_funcs(crate::r#move::to_bottom_row, MMAIN, "Bottom Row", bottomrow_gist, true);
        add_to_funcs(crate::r#move::do_page_up, MMAIN|MHELP, "Prev Page", prevpage_gist, false);
        add_to_funcs(crate::r#move::do_page_down, MMAIN|MHELP, "Next Page", nextpage_gist, false);
        add_to_funcs(crate::r#move::to_first_line, MMAIN|MHELP|MGOTOLINE, "First Line", firstline_gist, false);
        add_to_funcs(crate::r#move::to_last_line, MMAIN|MHELP|MGOTOLINE, "Last Line", lastline_gist, true);
        add_to_funcs(crate::files::switch_to_prev_buffer, MMAIN, "Prev File", prevfile_gist, false);
        add_to_funcs(crate::files::switch_to_next_buffer, MMAIN, "Next File", nextfile_gist, true);
        add_to_funcs(crate::text::do_tab, MMAIN, "Tab", tab_gist, false);
        add_to_funcs(crate::text::do_enter, MMAIN, "Enter", enter_gist, true);
        add_to_funcs(crate::cut::do_backspace, MMAIN, "Backspace", backspace_gist, false);
        add_to_funcs(crate::cut::do_delete, MMAIN, "Delete", delete_gist, true);
        add_to_funcs(crate::cut::chop_previous_word, MMAIN, "Chop Left", chopwordleft_gist, false);
        add_to_funcs(crate::cut::chop_next_word, MMAIN, "Chop Right", chopwordright_gist, false);
        add_to_funcs(crate::cut::cut_till_eof, MMAIN, "Cut Till End", cuttilleof_gist, true);
        add_to_funcs(crate::text::do_full_justify, MMAIN, "Full Justify", fulljustify_gist, false);
        add_to_funcs(crate::text::count_lines_words_and_characters, MMAIN, "Word Count", wordcount_gist, false);
        add_to_funcs(crate::text::do_verbatim_input, MMAIN, "Verbatim", verbatim_gist, true);
        add_to_funcs(crate::text::do_indent, MMAIN, "Indent", indent_gist, false);
        add_to_funcs(crate::text::do_unindent, MMAIN, "Unindent", unindent_gist, true);
        add_to_funcs(crate::text::do_comment, MMAIN, "Comment Lines", comment_gist, false);
        add_to_funcs(crate::text::complete_a_word, MMAIN, "Complete", completion_gist, true);
        add_to_funcs(crate::winio::record_macro, MMAIN, "Record", recordmacro_gist, false);
        add_to_funcs(crate::winio::run_macro, MMAIN, "Run Macro", runmacro_gist, true);
        add_to_funcs(crate::cut::zap_text, MMAIN, "Zap", zap_gist, true);
        add_to_funcs(crate::search::put_or_lift_anchor, MMAIN, "Anchor", anchor_gist, false);
        add_to_funcs(crate::search::to_prev_anchor, MMAIN, "Up to anchor", prevanchor_gist, false);
        add_to_funcs(crate::search::to_next_anchor, MMAIN, "Down to anchor", nextanchor_gist, true);
        add_to_funcs(crate::text::do_spell, MMAIN, "Spell Check", spell_gist, false);
        add_to_funcs(crate::text::do_linter, MMAIN, "Linter", lint_gist, false);
        add_to_funcs(crate::text::do_formatter, MMAIN, "Formatter", formatter_gist, true);
        add_to_funcs(crate::rcfile::do_suspend, MMAIN, "Suspend", suspend_gist, false);
        add_to_funcs(crate::winio::full_refresh, MMAIN, "Refresh", refresh_gist, true);
        add_to_funcs(crate::r#move::do_center, MMAIN, "Center", center_gist, false);
        add_to_funcs(crate::r#move::do_cycle, MMAIN, "Cycle", cycle_gist, true);
        add_to_funcs(crate::files::do_savefile, MMAIN, "Save", savefile_gist, true);
        add_to_funcs(crate::files::flip_pipe, MEXECUTE, "Pipe Text", pipe_gist, true);
        add_to_funcs(crate::text::do_spell, MEXECUTE, "Spell Check", spell_gist, false);
        add_to_funcs(crate::text::do_linter, MEXECUTE, "Linter", lint_gist, true);
        add_to_funcs(crate::text::do_full_justify, MEXECUTE, "Full Justify", fulljustify_gist, false);
        add_to_funcs(crate::text::do_formatter, MEXECUTE, "Formatter", formatter_gist, true);
        add_to_funcs(crate::global::dos_format, MWRITEFILE, "DOS Format", dos_gist, false);
            add_to_funcs(crate::files::back_it_up, MWRITEFILE, "Backup File", backup_gist, false);
            add_to_funcs(crate::files::append_it, MWRITEFILE, "Append", append_gist, false);
            add_to_funcs(crate::files::prepend_it, MWRITEFILE, "Prepend", prepend_gist, true);
        add_to_funcs(crate::files::flip_convert, MINSERTFILE, "No Conversion", convert_gist, true);
        add_to_funcs(crate::cut::cut_till_eof, MEXECUTE, "Cut Till End", cuttilleof_gist, true);
        add_to_funcs(crate::rcfile::do_suspend, MEXECUTE, "Suspend", suspend_gist, true);
        add_to_funcs(crate::files::discard_buffer, MWRITEFILE, "Discard buffer", discardbuffer_gist, true);
        add_to_funcs(crate::r#move::do_page_up, MBROWSER, "Prev Page", prevpage_gist, false);
        add_to_funcs(crate::r#move::do_page_down, MBROWSER, "Next Page", nextpage_gist, false);
        add_to_funcs(crate::browser::to_first_file, MBROWSER|MWHEREISFILE, "First File", firstfile_gist, false);
        add_to_funcs(crate::browser::to_last_file, MBROWSER|MWHEREISFILE, "Last File", lastfile_gist, true);
        add_to_funcs(crate::r#move::to_prev_word, MBROWSER, "Left Column", browserlefthand_gist, false);
        add_to_funcs(crate::r#move::to_next_word, MBROWSER, "Right Column", browserrighthand_gist, false);
        add_to_funcs(crate::r#move::to_prev_block, MBROWSER, "Top Row", browsertoprow_gist, false);
        add_to_funcs(crate::r#move::to_next_block, MBROWSER, "Bottom Row", browserbottomrow_gist, true);
        add_to_funcs(crate::r#move::do_page_up, MLINTER, "Previous Linter message", prevlint_gist, false);
        add_to_funcs(crate::r#move::do_page_down, MLINTER, "Next Linter message", nextlint_gist, false);
        add_to_sclist(MMOST|MBROWSER, "^M", '\r' as i32, crate::text::do_enter, 0);
        add_to_sclist(MMOST|MBROWSER, "Enter", KEY_ENTER, crate::text::do_enter, 0);
        add_to_sclist(MMOST, "^I", '\t' as i32, crate::text::do_tab, 0);
        add_to_sclist(MMOST, "Tab", '\t' as i32, crate::text::do_tab, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "^B", 0, crate::search::do_search_backward, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "^F", 0, crate::search::do_search_forward, 0);
        if ISSET(MODERN_BINDINGS) {
            add_to_sclist((MMOST|MBROWSER) & !MFINDINHELP, help_key, 0, crate::help::do_help, 0);
            add_to_sclist(MHELP, help_key, 0, crate::global::do_exit, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^Q", 0, crate::global::do_exit, 0);
            add_to_sclist(MMAIN, "^S", 0, crate::files::do_savefile, 0);
            add_to_sclist(MMAIN, "^W", 0, crate::files::do_writeout, 0);
            add_to_sclist(MMAIN, "^O", 0, crate::files::do_insertfile, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^D", 0, crate::search::do_findprevious, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^G", 0, crate::search::do_findnext, 0);
            add_to_sclist(MMAIN, "^R", 0, crate::search::do_replace, 0);
            add_to_sclist(MMAIN, "^T", 0, crate::search::do_gotolinecolumn, 0);
            add_to_sclist(MMAIN, "^P", 0, crate::global::report_cursor_position, 0);
            add_to_sclist(MMAIN, "^Z", 0, crate::text::do_undo, 0);
            add_to_sclist(MMAIN, "^Y", 0, crate::text::do_redo, 0);
            add_to_sclist(MMAIN, "^A", 0, crate::text::do_mark, 0);
            add_to_sclist(MMAIN, "^X", 0, crate::cut::cut_text, 0);
            add_to_sclist(MMAIN, "^C", 0, crate::cut::copy_text, 0);
            add_to_sclist(MMAIN, "^V", 0, crate::cut::paste_text, 0);
        } else {
            add_to_sclist((MMOST|MBROWSER) & !MFINDINHELP, "^G", 0, crate::help::do_help, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^X", 0, crate::global::do_exit, 0);
            add_to_sclist(MMAIN, "^O", 0, crate::files::do_writeout, 0);
            add_to_sclist(MMAIN, "^R", 0, crate::files::do_insertfile, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^W", 0, crate::search::do_search_forward, 0);
            add_to_sclist(MMOST, "^A", 0, crate::r#move::do_home, 0);
            add_to_sclist(MMOST, "^E", 0, crate::r#move::do_end, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^P", 0, crate::r#move::do_up, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "^N", 0, crate::r#move::do_down, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "^Y", 0, crate::r#move::do_page_up, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "^V", 0, crate::r#move::do_page_down, 0);
            add_to_sclist(MMAIN, "^C", 0, crate::global::report_cursor_position, 0);
            add_to_sclist(MMOST, "^H", 0x08, crate::cut::do_backspace, 0);
            add_to_sclist(MMOST, "^D", 0, crate::cut::do_delete, 0);
        }
        add_to_sclist(MMOST, "Bsp", KEY_BACKSPACE, crate::cut::do_backspace, 0);
        add_to_sclist(MMOST, "Sh-Del", SHIFT_DELETE, crate::cut::do_backspace, 0);
        add_to_sclist(MMOST, "Del", KEY_DC, crate::cut::do_delete, 0);
        add_to_sclist(MMAIN, "Ins", KEY_IC, crate::files::do_insertfile, 0);
        add_to_sclist(MMAIN, "^\\", 0, crate::search::do_replace, 0);
        add_to_sclist(MMAIN, "M-R", 0, crate::search::do_replace, 0);
        add_to_sclist(MMOST, "^K", 0, crate::cut::cut_text, 0);
        add_to_sclist(MMOST, "M-6", 0, crate::cut::copy_text, 0);
        add_to_sclist(MMOST, "M-^", 0, crate::cut::copy_text, 0);
        add_to_sclist(MMOST, "^U", 0, crate::cut::paste_text, 0);
        add_to_sclist(MMAIN, if ISSET(MODERN_BINDINGS) { "^E" } else { "^T" }, 0, crate::files::do_execute, 0);
        add_to_sclist(MEXECUTE, "^T", 0, crate::text::do_spell, 0);
        add_to_sclist(MMAIN, "^J", '\n' as i32, crate::text::do_justify, 0);
        add_to_sclist(MEXECUTE, "^Y", 0, crate::text::do_linter, 0);
        add_to_sclist(MEXECUTE, "^O", 0, crate::text::do_formatter, 0);
        add_to_sclist(MMAIN, slash_or_dash, 0, crate::search::do_gotolinecolumn, 0);
        add_to_sclist(MMAIN, "M-G", 0, crate::search::do_gotolinecolumn, 0);
        add_to_sclist(MMAIN, "^_", 0, crate::search::do_gotolinecolumn, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "PgUp", KEY_PPAGE, crate::r#move::do_page_up, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "PgDn", KEY_NPAGE, crate::r#move::do_page_down, 0);
        add_to_sclist(MBROWSER|MHELP, "Bsp", KEY_BACKSPACE, crate::r#move::do_page_up, 0);
        add_to_sclist(MBROWSER|MHELP, "Sh-Del", SHIFT_DELETE, crate::r#move::do_page_up, 0);
        add_to_sclist(MBROWSER|MHELP, "Space", 0x20, crate::r#move::do_page_down, 0);
        add_to_sclist(MMAIN|MHELP, "M-\\", 0, crate::r#move::to_first_line, 0);
        add_to_sclist(MMAIN|MHELP, "^Home", CONTROL_HOME, crate::r#move::to_first_line, 0);
        add_to_sclist(MMAIN|MHELP, "M-/", 0, crate::r#move::to_last_line, 0);
        add_to_sclist(MMAIN|MHELP, "^End", CONTROL_END, crate::r#move::to_last_line, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "M-B", 0, crate::search::do_findprevious, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "M-F", 0, crate::search::do_findnext, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "M-W", 0, crate::search::do_findnext, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "M-Q", 0, crate::search::do_findprevious, 0);
        add_to_sclist(MMAIN, "M-]", 0, crate::search::do_find_bracket, 0);
        add_to_sclist(MMAIN, "M-A", 0, crate::text::do_mark, 0);
        add_to_sclist(MMAIN, "^6", 0, crate::text::do_mark, 0);
        add_to_sclist(MMAIN, "^^", 0, crate::text::do_mark, 0);
        add_to_sclist(MMAIN, "M-}", 0, crate::text::do_indent, 0);
        add_to_sclist(MMAIN, "M-{", 0, crate::text::do_unindent, 0);
        add_to_sclist(MMAIN, "Sh-Tab", SHIFT_TAB, crate::text::do_unindent, 0);
        add_to_sclist(MMAIN, "M-:", 0, crate::winio::record_macro, 0);
        add_to_sclist(MMAIN, "M-;", 0, crate::winio::run_macro, 0);
        add_to_sclist(MMAIN, "M-U", 0, crate::text::do_undo, 0);
        add_to_sclist(MMAIN, "M-E", 0, crate::text::do_redo, 0);
        add_to_sclist(MMAIN, "M-Bsp", CONTROL_SHIFT_DELETE, crate::cut::chop_previous_word, 0);
        add_to_sclist(MMAIN, "Sh-^Del", CONTROL_SHIFT_DELETE, crate::cut::chop_previous_word, 0);
        add_to_sclist(MMAIN, "^Del", CONTROL_DELETE, crate::cut::chop_next_word, 0);
        add_to_sclist(MMAIN, "M-Del", ALT_DELETE, crate::cut::zap_text, 0);
        add_to_sclist(MMAIN, "M-Ins", ALT_INSERT, crate::search::put_or_lift_anchor, 0);
        add_to_sclist(MMAIN, "M-Home", ALT_HOME, crate::r#move::to_top_row, 0);
        add_to_sclist(MMAIN, "M-End", ALT_END, crate::r#move::to_bottom_row, 0);
        add_to_sclist(MMAIN, "M-PgUp", ALT_PAGEUP, crate::search::to_prev_anchor, 0);
        add_to_sclist(MMAIN, "M-PgDn", ALT_PAGEDOWN, crate::search::to_next_anchor, 0);
        add_to_sclist(MMAIN, "M-\"", 0, crate::search::put_or_lift_anchor, 0);
        add_to_sclist(MMAIN, "M-'", 0, crate::search::to_next_anchor, 0);
        add_to_sclist(MMAIN, "^]", 0, crate::text::complete_a_word, 0);
        add_to_sclist(MMAIN, "M-3", 0, crate::text::do_comment, 0);
        add_to_sclist(MMOST & !MMAIN, "^B", 0, crate::r#move::do_left, 0);
        add_to_sclist(MMOST & !MMAIN, "^F", 0, crate::r#move::do_right, 0);
            add_to_sclist(MMOST|MBROWSER|MHELP, "◂", KEY_LEFT, crate::r#move::do_left, 0);
            add_to_sclist(MMOST|MBROWSER|MHELP, "▸", KEY_RIGHT, crate::r#move::do_right, 0);
            add_to_sclist(MSOME, "^◂", CONTROL_LEFT, crate::r#move::to_prev_word, 0);
            add_to_sclist(MSOME, "^▸", CONTROL_RIGHT, crate::r#move::to_next_word, 0);
                add_to_sclist(MMAIN, "M-◂", ALT_LEFT, crate::files::switch_to_prev_buffer, 0);
                add_to_sclist(MMAIN, "M-▸", ALT_RIGHT, crate::files::switch_to_next_buffer, 0);
            add_to_sclist(MMOST|MBROWSER|MHELP, "Left", KEY_LEFT, crate::r#move::do_left, 0);
            add_to_sclist(MMOST|MBROWSER|MHELP, "Right", KEY_RIGHT, crate::r#move::do_right, 0);
            add_to_sclist(MSOME, "^Left", CONTROL_LEFT, crate::r#move::to_prev_word, 0);
            add_to_sclist(MSOME, "^Right", CONTROL_RIGHT, crate::r#move::to_next_word, 0);
                add_to_sclist(MMAIN, "M-Left", ALT_LEFT, crate::files::switch_to_prev_buffer, 0);
                add_to_sclist(MMAIN, "M-Right", ALT_RIGHT, crate::files::switch_to_next_buffer, 0);
        add_to_sclist(MMOST, "M-Space", 0, crate::r#move::to_prev_word, 0);
        add_to_sclist(MMOST, "^Space", 0, crate::r#move::to_next_word, 0);
        add_to_sclist(MMOST, "Home", KEY_HOME, crate::r#move::do_home, 0);
        add_to_sclist(MMOST, "End", KEY_END, crate::r#move::do_end, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "▴", KEY_UP, crate::r#move::do_up, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "▾", KEY_DOWN, crate::r#move::do_down, 0);
            add_to_sclist(MMAIN|MBROWSER|MLINTER, "^▴", CONTROL_UP, crate::r#move::to_prev_block, 0);
            add_to_sclist(MMAIN|MBROWSER|MLINTER, "^▾", CONTROL_DOWN, crate::r#move::to_next_block, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "Up", KEY_UP, crate::r#move::do_up, 0);
            add_to_sclist(MMAIN|MBROWSER|MHELP, "Down", KEY_DOWN, crate::r#move::do_down, 0);
            add_to_sclist(MMAIN|MBROWSER|MLINTER, "^Up", CONTROL_UP, crate::r#move::to_prev_block, 0);
            add_to_sclist(MMAIN|MBROWSER|MLINTER, "^Down", CONTROL_DOWN, crate::r#move::to_next_block, 0);
        add_to_sclist(MMAIN, "M-7", 0, crate::r#move::to_prev_block, 0);
        add_to_sclist(MMAIN, "M-8", 0, crate::r#move::to_next_block, 0);
        add_to_sclist(MMAIN, "M-(", 0, crate::r#move::to_para_begin, 0);
        add_to_sclist(MMAIN, "M-9", 0, crate::r#move::to_para_begin, 0);
        add_to_sclist(MMAIN, "M-)", 0, crate::r#move::to_para_end, 0);
        add_to_sclist(MMAIN, "M-0", 0, crate::r#move::to_para_end, 0);
            add_to_sclist(MMAIN|MHELP, "M-▴", ALT_UP, crate::r#move::do_scroll_up, 0);
            add_to_sclist(MMAIN|MHELP, "M-▾", ALT_DOWN, crate::r#move::do_scroll_down, 0);
            add_to_sclist(MMAIN|MHELP, "M-Up", ALT_UP, crate::r#move::do_scroll_up, 0);
            add_to_sclist(MMAIN|MHELP, "M-Down", ALT_DOWN, crate::r#move::do_scroll_down, 0);
        add_to_sclist(MMAIN|MHELP, "M--", 0, crate::r#move::do_scroll_up, 0);
        add_to_sclist(MMAIN|MHELP, "M-_", 0, crate::r#move::do_scroll_up, 0);
        add_to_sclist(MMAIN|MHELP, "M-+", 0, crate::r#move::do_scroll_down, 0);
        add_to_sclist(MMAIN|MHELP, "M-=", 0, crate::r#move::do_scroll_down, 0);
        add_to_sclist(MMAIN, "M-,", 0, crate::files::switch_to_prev_buffer, 0);
        add_to_sclist(MMAIN, "M-.", 0, crate::files::switch_to_next_buffer, 0);
        add_to_sclist(MMOST, "M-V", 0, crate::text::do_verbatim_input, 0);
        add_to_sclist(MMAIN, "M-T", 0, crate::cut::cut_till_eof, 0);
        add_to_sclist(MEXECUTE, "^V", 0, crate::cut::cut_till_eof, 0);
        add_to_sclist(MEXECUTE, "^Z", 0, crate::rcfile::do_suspend, 0);
        add_to_sclist(MMAIN, "^Z", 0, crate::global::suggest_ctrlT_ctrlZ, 0);
        add_to_sclist(MMAIN, "M-D", 0, crate::text::count_lines_words_and_characters, 0);
        add_to_sclist(MMAIN, "M-J", 0, crate::text::do_full_justify, 0);
        add_to_sclist(MEXECUTE, "^J", 0, crate::text::do_full_justify, 0);
        add_to_sclist(MMAIN, "M-<", 0, crate::global::do_scroll_left, 0);
        add_to_sclist(MMAIN, "M->", 0, crate::global::do_scroll_right, 0);
        add_to_sclist(MMAIN, "^L", 0, crate::r#move::do_center, 0);
        add_to_sclist(MMAIN, "M-%", 0, crate::r#move::do_cycle, 0);
        add_to_sclist((MMOST|MBROWSER|MHELP|MYESNO)&!MMAIN, "^L", 0, crate::winio::full_refresh, 0);
        add_to_sclist(MMAIN, "M-Z", 0, crate::global::do_toggle, ZERO as i32);
        add_to_sclist((MMOST|MBROWSER|MYESNO) & !MFINDINHELP, "M-X", 0, crate::global::do_toggle, NO_HELP as i32);
        add_to_sclist(MMAIN, "M-C", 0, crate::global::do_toggle, CONSTANT_SHOW as i32);
        add_to_sclist(MMAIN, "M-S", 0, crate::global::do_toggle, SOFTWRAP as i32);
        add_to_sclist(MMAIN, "M-$", 0, crate::global::do_toggle, SOFTWRAP as i32);
        add_to_sclist(MMAIN, "M-N", 0, crate::global::do_toggle, LINE_NUMBERS as i32);
        add_to_sclist(MMAIN, "M-#", 0, crate::global::do_toggle, LINE_NUMBERS as i32);
        add_to_sclist(MMAIN, "M-P", 0, crate::global::do_toggle, WHITESPACE_DISPLAY as i32);
        add_to_sclist(MMAIN, "M-Y", 0, crate::global::do_toggle, NO_SYNTAX as i32);
        add_to_sclist(MMAIN, "M-H", 0, crate::global::do_toggle, SMART_HOME as i32);
        add_to_sclist(MMAIN, "M-I", 0, crate::global::do_toggle, AUTOINDENT as i32);
        add_to_sclist(MMAIN, "M-K", 0, crate::global::do_toggle, CUT_FROM_CURSOR as i32);
        add_to_sclist(MMAIN, "M-L", 0, crate::global::do_toggle, BREAK_LONG_LINES as i32);
        add_to_sclist(MMAIN, "M-O", 0, crate::global::do_toggle, TABS_TO_SPACES as i32);
        add_to_sclist(MMAIN, "M-M", 0, crate::global::do_toggle, USE_MOUSE as i32);
        add_to_sclist(((MMOST & !MMAIN) | MYESNO), "^C", 0, crate::text::do_cancel, 0);
        add_to_sclist(MWHEREIS|MREPLACE, "M-C", 0, crate::global::case_sens_void, 0);
        add_to_sclist(MWHEREIS|MREPLACE, "M-R", 0, crate::global::regexp_void, 0);
        add_to_sclist(MWHEREIS|MREPLACE, "M-B", 0, crate::global::backwards_void, 0);
        add_to_sclist(MWHEREIS|MREPLACE, "^R", 0, crate::global::flip_replace, 0);
        add_to_sclist(MWHEREIS|MGOTOLINE, "^T", 0, crate::global::flip_goto, 0);
        add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "^P", 0, crate::global::get_older_item, 0);
        add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "^N", 0, crate::global::get_newer_item, 0);
            add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "▴", KEY_UP, crate::global::get_older_item, 0);
            add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "▾", KEY_DOWN, crate::global::get_newer_item, 0);
            add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "Up", KEY_UP, crate::global::get_older_item, 0);
            add_to_sclist(MWHEREIS|MREPLACE|MREPLACEWITH|MWHEREISFILE|MFINDINHELP|MEXECUTE, "Down", KEY_DOWN, crate::global::get_newer_item, 0);
        add_to_sclist(MGOTOLINE, "^W", 0, crate::r#move::to_para_begin, 0);
        add_to_sclist(MGOTOLINE, "^O", 0, crate::r#move::to_para_end, 0);
        add_to_sclist(MGOTOLINE|MWHEREIS|MFINDINHELP, "^Y", 0, crate::r#move::to_first_line, 0);
        add_to_sclist(MGOTOLINE|MWHEREIS|MFINDINHELP, "^V", 0, crate::r#move::to_last_line, 0);
        add_to_sclist(MWHEREISFILE, "^Y", 0, crate::browser::to_first_file, 0);
        add_to_sclist(MWHEREISFILE, "^V", 0, crate::browser::to_last_file, 0);
        add_to_sclist(MBROWSER|MWHEREISFILE, "M-\\", 0, crate::browser::to_first_file, 0);
        add_to_sclist(MBROWSER|MWHEREISFILE, "M-/", 0, crate::browser::to_last_file, 0);
        add_to_sclist(MBROWSER, "Home", KEY_HOME, crate::browser::to_first_file, 0);
        add_to_sclist(MBROWSER, "End", KEY_END, crate::browser::to_last_file, 0);
        add_to_sclist(MBROWSER, "^Home", CONTROL_HOME, crate::browser::to_first_file, 0);
        add_to_sclist(MBROWSER, "^End", CONTROL_END, crate::browser::to_last_file, 0);
        add_to_sclist(MBROWSER, slash_or_dash, 0, crate::global::goto_dir, 0);
        add_to_sclist(MBROWSER, "M-G", 0, crate::global::goto_dir, 0);
        add_to_sclist(MBROWSER, "^_", 0, crate::global::goto_dir, 0);
        add_to_sclist(MWRITEFILE, "M-D", 0, crate::global::dos_format, 0);
            add_to_sclist(MWRITEFILE, "M-B", 0, crate::files::back_it_up, 0);
            add_to_sclist(MWRITEFILE, "M-A", 0, crate::files::append_it, 0);
            add_to_sclist(MWRITEFILE, "M-P", 0, crate::files::prepend_it, 0);
            add_to_sclist(MINSERTFILE|MEXECUTE, "^X", 0, crate::files::flip_execute, 0);
        add_to_sclist(MINSERTFILE, "M-N", 0, crate::files::flip_convert, 0);
            add_to_sclist(MINSERTFILE|MEXECUTE, "M-F", 0, crate::files::flip_newbuffer, 0);
            add_to_sclist(MEXECUTE, "M-\\", 0, crate::files::flip_pipe, 0);
        add_to_sclist(MBROWSER|MHELP, "^C", 0, crate::global::do_exit, 0);
        add_to_sclist(MBROWSER, "^T", 0, crate::global::do_exit, 0);
        add_to_sclist(MHELP, "^G", 0, crate::global::do_exit, 0);
        add_to_sclist(MHELP, "F1", KEY_F0 + 1, crate::global::do_exit, 0);
        add_to_sclist(MHELP, "Home", KEY_HOME, crate::r#move::to_first_line, 0);
        add_to_sclist(MHELP, "End", KEY_END, crate::r#move::to_last_line, 0);
        add_to_sclist(MLINTER, "^X", 0, crate::text::do_cancel, 0);
        add_to_sclist(MMOST & !MFINDINHELP, "F1", KEY_F0 + 1, crate::help::do_help, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "F2", KEY_F0 + 2, crate::global::do_exit, 0);
        add_to_sclist(MMAIN, "F3", KEY_F0 + 3, crate::files::do_writeout, 0);
        add_to_sclist(MMAIN, "F4", KEY_F0 + 4, crate::text::do_justify, 0);
        add_to_sclist(MMAIN, "F5", KEY_F0 + 5, crate::files::do_insertfile, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP, "F6", KEY_F0 + 6, crate::search::do_search_forward, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "F7", KEY_F0 + 7, crate::r#move::do_page_up, 0);
        add_to_sclist(MMAIN|MBROWSER|MHELP|MLINTER, "F8", KEY_F0 + 8, crate::r#move::do_page_down, 0);
        add_to_sclist(MMOST, "F9", KEY_F0 + 9, crate::cut::cut_text, 0);
        add_to_sclist(MMOST, "F10", KEY_F0 + 10, crate::cut::paste_text, 0);
        add_to_sclist(MMAIN, "F11", KEY_F0 + 11, crate::global::report_cursor_position, 0);
        add_to_sclist(MMAIN, "F12", KEY_F0 + 12, crate::text::do_spell, 0);
        add_to_sclist((MMOST & !MMAIN) | MYESNO, "", KEY_CANCEL, crate::text::do_cancel, 0);
        add_to_sclist(MMAIN, "", KEY_CENTER, crate::r#move::do_center, 0);
        add_to_sclist(MMAIN, "", KEY_SIC, crate::files::do_insertfile, 0);
        add_to_sclist(MMAIN, "", START_OF_PASTE, crate::global::suck_up_input_and_paste_it, 0);
        add_to_sclist(MMOST, "", START_OF_PASTE, crate::global::do_nothing, 0);
        add_to_sclist(MMOST, "", END_OF_PASTE, crate::global::do_nothing, 0);
    }
}

/* 返回与给定标志对应的文字描述（对应 C 的 epithet_of_flag）。全功能构建。 */
pub fn epithet_of_flag(flag: usize) -> &'static str {
    match flag {
        /* TRANSLATORS: The next thirteen strings are toggle descriptions;
         * they are best kept shorter than 40 characters, but may be longer. */
        ZERO => N_("Hidden interface"),
        NO_HELP => N_("Help mode"),
        CONSTANT_SHOW => N_("Constant cursor position display"),
        SOFTWRAP => N_("Soft wrapping of overlong lines"),
        LINE_NUMBERS => N_("Line numbering"),
        WHITESPACE_DISPLAY => N_("Whitespace display"),
        NO_SYNTAX => N_("Color syntax highlighting"),
        SMART_HOME => N_("Smart home key"),
        AUTOINDENT => N_("Auto indent"),
        CUT_FROM_CURSOR => N_("Cut to end"),
        BREAK_LONG_LINES => N_("Hard wrapping of overlong lines"),
        TABS_TO_SPACES => N_("Conversion of typed tabs to spaces"),
        USE_MOUSE => N_("Mouse support"),
        _ => "Ehm...",
    }
}
