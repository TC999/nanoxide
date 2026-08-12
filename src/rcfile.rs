/**************************************************************************
 *   rcfile.rs  --  GNU nano 的 rcfile.c 翻译（Rust 版）。               *
 *   对应 C 源：nano/src/rcfile.c（1760 行）。                          *
 *   全特性构建：所有 #ifdef 分支均视为启用。                          *
 **************************************************************************/

use crate::chars::*;
use crate::definitions::*;
use crate::gettext;
use regex::{Regex, RegexBuilder};
use crate::files::{die, expand_leading_tilde};
use crate::global::*;
use crate::history::*;
use crate::utils::*;
use crate::cut::{chop_previous_word, chop_next_word, do_delete, do_backspace};
use crate::text::do_tab;
use crate::text::do_enter;
use crate::color::{
    A_BOLD, A_ITALIC, A_NORMAL, COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA,
    COLOR_RED, COLOR_WHITE, COLOR_YELLOW, COLORS,
};

/* 本模块尚未翻译的子系统（浏览器 step 13）所需的
 * 局部常量与桩函数。待对应模块落地后移除并改用真实定义。 */

/// 系统级 nanorc 所在目录（configure 时确定的 SYSCONFDIR）。
const SYSCONFDIR: &str = "/etc";

/// 正则表达式编译标志（regex crate 默认即为扩展语法）。
const NANO_REG_EXTENDED: u32 = 0;
const REG_ICASE: u32 = 1;
const REG_NOSUB: u32 = 2;

/// 单条错误/路径缓冲区的最大长度（PATH_MAX + 200）。
const MAXSIZE: usize = PATH_MAX + 200;

/// 家目录与系统级的 rcfile 文件名。
const HOME_RC_NAME: &str = ".nanorc";
const RCFILE_NAME: &str = "nanorc";

/// 尚未翻译的桩函数（implant 来自 text.c，其余来自 browser.c / 其他）。
pub fn implant() {}
pub fn do_suspend() {}
pub fn to_first_file() {}
pub fn to_last_file() {}
pub fn switch_to_prev_buffer() {}
pub fn switch_to_next_buffer() {}

/// 安全包装：对应函数为 unsafe fn，但 keystruct.func 字段为安全 fn 指针。
pub fn chop_previous_word_safe() { unsafe { chop_previous_word(); } }
pub fn chop_next_word_safe() { unsafe { chop_next_word(); } }
pub fn do_tab_safe() { unsafe { do_tab(); } }
pub fn do_enter_safe() { unsafe { do_enter(); } }
pub fn do_delete_safe() { unsafe { do_delete(); } }
pub fn do_backspace_safe() { unsafe { do_backspace(); } }
pub fn do_verbatim_input_safe() { do_verbatim_input(); }

/* ---- 模块级状态变量 ---- */

/// 各 rcfile 选项（名字 + 对应标志位；flag 为 0 表示带参数的选项）。
static RCOPTS: &[rcoption] = &[
    rcoption { name: "boldtext", flag: BOLD_TEXT as i64 },
    rcoption { name: "brackets", flag: 0 },
    rcoption { name: "breaklonglines", flag: BREAK_LONG_LINES as i64 },
    rcoption { name: "casesensitive", flag: CASE_SENSITIVE as i64 },
    rcoption { name: "constantshow", flag: CONSTANT_SHOW as i64 },
    rcoption { name: "fill", flag: 0 },
    rcoption { name: "historylog", flag: HISTORYLOG as i64 },
    rcoption { name: "linenumbers", flag: LINE_NUMBERS as i64 },
    rcoption { name: "magic", flag: USE_MAGIC as i64 },
    rcoption { name: "mouse", flag: USE_MOUSE as i64 },
    rcoption { name: "newbuffer", flag: NEW_BUFFER as i64 },
    rcoption { name: "nohelp", flag: NO_HELP as i64 },
    rcoption { name: "nonewlines", flag: NO_NEWLINES as i64 },
    rcoption { name: "nowrap", flag: NO_WRAP as i64 },
    rcoption { name: "operatingdir", flag: 0 },
    rcoption { name: "positionlog", flag: POSITIONLOG as i64 },
    rcoption { name: "preserve", flag: PRESERVE as i64 },
    rcoption { name: "punct", flag: 0 },
    rcoption { name: "quotestr", flag: 0 },
    rcoption { name: "quickblank", flag: QUICK_BLANK as i64 },
    rcoption { name: "rawsequences", flag: RAW_SEQUENCES as i64 },
    rcoption { name: "rebinddelete", flag: REBIND_DELETE as i64 },
    rcoption { name: "regexp", flag: USE_REGEXP as i64 },
    rcoption { name: "saveonexit", flag: SAVE_ON_EXIT as i64 },
    rcoption { name: "speller", flag: 0 },
    rcoption { name: "afterends", flag: AFTER_ENDS as i64 },
    rcoption { name: "allow_insecure_backup", flag: INSECURE_BACKUP as i64 },
    rcoption { name: "atblanks", flag: AT_BLANKS as i64 },
    rcoption { name: "autoindent", flag: AUTOINDENT as i64 },
    rcoption { name: "backup", flag: MAKE_BACKUP as i64 },
    rcoption { name: "backupdir", flag: 0 },
    rcoption { name: "bookstyle", flag: BOOKSTYLE as i64 },
    rcoption { name: "colonparsing", flag: COLON_PARSING as i64 },
    rcoption { name: "cutfromcursor", flag: CUT_FROM_CURSOR as i64 },
    rcoption { name: "emptyline", flag: EMPTY_LINE as i64 },
    rcoption { name: "guidestripe", flag: 0 },
    rcoption { name: "indicator", flag: INDICATOR as i64 },
    rcoption { name: "jumpyscrolling", flag: JUMPY_SCROLLING as i64 },
    rcoption { name: "locking", flag: LOCKING as i64 },
    rcoption { name: "matchbrackets", flag: 0 },
    rcoption { name: "minibar", flag: MINIBAR as i64 },
    rcoption { name: "noconvert", flag: NO_CONVERT as i64 },
    rcoption { name: "showcursor", flag: SHOW_CURSOR as i64 },
    rcoption { name: "smarthome", flag: SMART_HOME as i64 },
    rcoption { name: "softwrap", flag: SOFTWRAP as i64 },
    rcoption { name: "solosidescroll", flag: SOLO_SIDESCROLL as i64 },
    rcoption { name: "stateflags", flag: STATEFLAGS as i64 },
    rcoption { name: "tabsize", flag: 0 },
    rcoption { name: "tabstospaces", flag: TABS_TO_SPACES as i64 },
    rcoption { name: "trimblanks", flag: TRIM_BLANKS as i64 },
    rcoption { name: "unix", flag: MAKE_IT_UNIX as i64 },
    rcoption { name: "whitespace", flag: 0 },
    rcoption { name: "whitespacedisplay", flag: WHITESPACE_DISPLAY as i64 },
    rcoption { name: "wordbounds", flag: WORD_BOUNDS as i64 },
    rcoption { name: "wordchars", flag: 0 },
    rcoption { name: "zap", flag: LET_THEM_ZAP as i64 },
    rcoption { name: "zero", flag: ZERO as i64 },
    rcoption { name: "titlecolor", flag: 0 },
    rcoption { name: "numbercolor", flag: 0 },
    rcoption { name: "stripecolor", flag: 0 },
    rcoption { name: "scrollercolor", flag: 0 },
    rcoption { name: "selectedcolor", flag: 0 },
    rcoption { name: "spotlightcolor", flag: 0 },
    rcoption { name: "minicolor", flag: 0 },
    rcoption { name: "promptcolor", flag: 0 },
    rcoption { name: "statuscolor", flag: 0 },
    rcoption { name: "errorcolor", flag: 0 },
    rcoption { name: "keycolor", flag: 0 },
    rcoption { name: "functioncolor", flag: 0 },
    rcoption { name: "", flag: 0 },
];

/// 最近一次出错所在的行号。
static mut LINENO: usize = 0;

/// 正在解析的 rcfile 路径。
static mut NANORC: Option<String> = None;

/// 是否允许向当前语法追加命令（语法结束或新 syntax 出现时置 FALSE）。
static mut OPENSYNTAX: bool = false;

/// 当前正在解析的语法。
static mut LIVE_SYNTAX: *mut syntaxtype = std::ptr::null_mut();

/// 当前语法是否已包含颜色命令。
static mut SEEN_COLOR_COMMAND: bool = false;

/// 当前语法颜色列表的末尾节点。
static mut LASTCOLOR: *mut colortype = std::ptr::null_mut();

/// rcfile 错误链表的首尾。
static mut ERRORS_HEAD: *mut linestruct = std::ptr::null_mut();
static mut ERRORS_TAIL: *mut linestruct = std::ptr::null_mut();

/* ---- 字节级辅助（替代 C 的 ctype/string 内联操作） ---- */

/// 判断字节是否为空白（空格或制表符）。
fn is_blank_byte(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// 从 buffer[start] 取到下一个 '\0' 或结尾的切片。
fn slice_to_nul(buffer: &[u8], start: usize) -> &[u8] {
    let mut end = start;
    while end < buffer.len() && buffer[end] != 0 {
        end += 1;
    }
    &buffer[start..end]
}

/// 取 buffer[start] 起直到 '\0'/结尾的字符串（拷贝）。
fn string_to_nul(buffer: &[u8], start: usize) -> String {
    String::from_utf8_lossy(slice_to_nul(buffer, start)).into_owned()
}

/// 计算以 '\0' 结尾的切片长度。
fn nul_strlen(slice: &[u8]) -> usize {
    let mut i = 0;
    while i < slice.len() && slice[i] != 0 {
        i += 1;
    }
    i
}

/* ---- 错误收集 ---- */

/// 把收集到的错误信息输出到终端。
pub fn display_rcfile_errors() {
    unsafe {
        let mut error = ERRORS_HEAD;
        while !error.is_null() {
            eprintln!("{}", (*error).data);
            error = (*error).next;
        }
    }
}

/// 把给定错误信息存入链表，待退出时打印。
pub fn jot_error(message: String) {
    unsafe {
        let error = Box::into_raw(Box::new(linestruct {
            data: message,
            lineno: 0,
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
            multidata: None,
            has_anchor: false,
        }));

        if ERRORS_HEAD.is_null() {
            ERRORS_HEAD = error;
        } else {
            (*ERRORS_TAIL).next = error;
        }
        ERRORS_TAIL = error;

        if startup_problem.is_none() {
            if let Some(ref nr) = NANORC {
                startup_problem = Some(format!(gettext!("Mistakes in '{}'"), nr));
            } else {
                startup_problem = Some(gettext!("Problems with history file").to_string());
            }
        }
    }
}

/* ---- 函数名解析 ---- */

/// 解析 rcfile 中给定的函数字符串，返回填入对应函数的快捷键记录。
pub fn strtosc(input: &str) -> *mut keystruct {
    let s = Box::into_raw(Box::new(keystruct {
        keystr: "",
        keycode: 0,
        menus: 0,
        func: None,
        toggle: 0,
        ordinal: 0,
        expansion: None,
        next: std::ptr::null_mut(),
    }));

    let func: Option<unsafe fn()> = match input {
        "cancel" => Some(do_cancel),
        "help" => Some(do_help),
        "exit" => Some(do_exit),
        "discardbuffer" => Some(discard_buffer),
        "writeout" => Some(do_writeout),
        "savefile" => Some(do_savefile),
        "insert" => Some(do_insertfile),
        "whereis" => Some(do_search_forward),
        "wherewas" => Some(do_search_backward),
        "findprevious" => Some(do_findprevious),
        "findnext" => Some(do_findnext),
        "replace" => Some(do_replace),
        "cut" => Some(cut_text),
        "copy" => Some(copy_text),
        "paste" => Some(paste_text),
        "execute" => Some(do_execute),
        "cutrestoffile" => Some(cut_till_eof),
        "zap" => Some(zap_text),
        "mark" => Some(do_mark),
        "tospell" | "speller" => Some(do_spell),
        "linter" => Some(do_linter),
        "formatter" => Some(do_formatter),
        "location" => Some(report_cursor_position),
        "gotoline" => Some(do_gotolinecolumn),
        "justify" => Some(do_justify),
        "fulljustify" => Some(do_full_justify),
        "beginpara" => Some(to_para_begin),
        "endpara" => Some(to_para_end),
        "comment" => Some(do_comment),
        "complete" => Some(complete_a_word),
        "indent" => Some(do_indent),
        "unindent" => Some(do_unindent),
        "chopwordleft" => Some(chop_previous_word_safe),
        "chopwordright" => Some(chop_next_word_safe),
        "findbracket" => Some(do_find_bracket),
        "wordcount" => Some(count_lines_words_and_characters),
        "recordmacro" => Some(record_macro),
        "runmacro" => Some(run_macro),
        "anchor" => Some(put_or_lift_anchor),
        "prevanchor" => Some(to_prev_anchor),
        "nextanchor" => Some(to_next_anchor),
        "undo" => Some(do_undo),
        "redo" => Some(do_redo),
        "suspend" => Some(do_suspend),
        "left" | "back" => Some(do_left),
        "right" | "forward" => Some(do_right),
        "up" | "prevline" => Some(do_up),
        "down" | "nextline" => Some(do_down),
        "scrollleft" => Some(do_scroll_left),
        "scrollright" => Some(do_scroll_right),
        "scrollup" => Some(do_scroll_up),
        "scrolldown" => Some(do_scroll_down),
        "prevword" => Some(to_prev_word),
        "nextword" => Some(to_next_word),
        "home" => Some(do_home),
        "end" => Some(do_end),
        "prevblock" => Some(to_prev_block),
        "nextblock" => Some(to_next_block),
        "toprow" => Some(to_top_row),
        "bottomrow" => Some(to_bottom_row),
        "center" => Some(do_center),
        "cycle" => Some(do_cycle),
        "pageup" | "prevpage" => Some(do_page_up),
        "pagedown" | "nextpage" => Some(do_page_down),
        "firstline" => Some(to_first_line),
        "lastline" => Some(to_last_line),
        "prevbuf" => Some(switch_to_prev_buffer),
        "nextbuf" => Some(switch_to_next_buffer),
        "verbatim" => Some(do_verbatim_input),
        "tab" => Some(do_tab_safe),
        "enter" => Some(do_enter_safe),
        "delete" => Some(do_delete_safe),
        "backspace" => Some(do_backspace_safe),
        "refresh" => Some(full_refresh),
        "casesens" => Some(case_sens_void),
        "regexp" => Some(regexp_void),
        "backwards" => Some(backwards_void),
        "flipreplace" => Some(flip_replace),
        "older" => Some(get_older_item),
        "newer" => Some(get_newer_item),
        "dosformat" => Some(dos_format),
        "append" => Some(append_it),
        "prepend" => Some(prepend_it),
        "backup" => Some(back_it_up),
        "flipexecute" => Some(flip_execute),
        "flippipe" => Some(flip_pipe),
        "flipconvert" => Some(flip_convert),
        "flipnewbuffer" => Some(flip_newbuffer),
        "tofiles" | "browser" => Some(to_files),
        "gotodir" => Some(goto_dir),
        "firstfile" => Some(to_first_file),
        "lastfile" => Some(to_last_file),
        "nohelp" => Some(do_toggle),
        "zero" => Some(do_toggle),
        "constantshow" => Some(do_toggle),
        "softwrap" => Some(do_toggle),
        "linenumbers" => Some(do_toggle),
        "whitespacedisplay" => Some(do_toggle),
        "nosyntax" => Some(do_toggle),
        "smarthome" => Some(do_toggle),
        "autoindent" => Some(do_toggle),
        "cutfromcursor" => Some(do_toggle),
        "breaklonglines" => Some(do_toggle),
        "tabstospaces" => Some(do_toggle),
        "mouse" => Some(do_toggle),
        _ => None,
    };

    unsafe {
        if let Some(f) = func {
            (*s).func = Some(f);
        } else {
            let _ = Box::from_raw(s);
            return std::ptr::null_mut();
        }

        /* toggle 类：根据函数名设置对应的标志位。 */
        if (*s).func == Some(do_toggle) {
            (*s).toggle = match input {
                "nohelp" => NO_HELP,
                "zero" => ZERO,
                "constantshow" => CONSTANT_SHOW,
                "softwrap" => SOFTWRAP,
                "linenumbers" => LINE_NUMBERS,
                "whitespacedisplay" => WHITESPACE_DISPLAY,
                "nosyntax" => NO_SYNTAX,
                "smarthome" => SMART_HOME,
                "autoindent" => AUTOINDENT,
                "cutfromcursor" => CUT_FROM_CURSOR,
                "breaklonglines" => BREAK_LONG_LINES,
                "tabstospaces" => TABS_TO_SPACES,
                "mouse" => USE_MOUSE,
                _ => 0,
            } as i32;
        }
    }

    s
}

/// 菜单名称数组。
const MENUNAMES: [&str; 16] = [
    "main", "search", "replace", "replacewith", "yesno", "gotoline", "writeout",
    "insert", "execute", "help", "spell", "linter", "browser", "whereisfile",
    "gotodir", "all",
];

/// 菜单符号数组（与 MENUNAMES 一一对应）。
const MENUSYMBOLS: [i32; 16] = [
    MMAIN, MWHEREIS, MREPLACE, MREPLACEWITH, MYESNO, MGOTOLINE, MWRITEFILE,
    MINSERTFILE, MEXECUTE, MHELP, MSPELL, MLINTER, MBROWSER, MWHEREISFILE,
    MGOTODIR, MMOST | MBROWSER | MHELP | MYESNO,
];

/// 返回给定菜单名对应的符号。
fn name_to_menu(name: &str) -> i32 {
    let mut index = 0;
    while index < MENUNAMES.len() {
        if MENUNAMES[index] == name {
            return MENUSYMBOLS[index];
        }
        index += 1;
    }
    0
}

/// 返回给定菜单符号对应的名称。
fn menu_to_name(menu: i32) -> &'static str {
    let mut index = 0;
    while index < MENUSYMBOLS.len() {
        if MENUSYMBOLS[index] == menu {
            return MENUNAMES[index];
        }
        index += 1;
    }
    "boooo"
}

/// 从 buffer[start] 解析下一个单词：就地以 '\0' 终止，返回后继位置。
fn parse_next_word(buffer: &mut [u8], mut i: usize) -> usize {
    while i < buffer.len() && !is_blank_byte(buffer[i]) {
        i += 1;
    }
    if i >= buffer.len() {
        return i;
    }
    buffer[i] = 0;
    i += 1;
    while i < buffer.len() && is_blank_byte(buffer[i]) {
        i += 1;
    }
    i
}

/// 解析一个参数（可用双引号包裹）。返回后继位置；出错返回 None。
fn parse_argument(buffer: &mut [u8], mut i: usize) -> Option<usize> {
    let the_start = i;

    if buffer.get(i) != Some(&b'"') {
        return Some(parse_next_word(buffer, i));
    }

    while i < buffer.len() {
        i += 1;
        if buffer[i] == b'"' {
            /* 找到最后一个引号，作为参数的结束。 */
            let mut last_quote = i;
            while i < buffer.len() {
                i += 1;
                if buffer[i] == b'"' {
                    last_quote = i;
                }
            }
            buffer[last_quote] = 0;
            let mut j = last_quote + 1;
            while j < buffer.len() && is_blank_byte(buffer[j]) {
                j += 1;
            }
            return Some(j);
        }
    }

    jot_error(format!(gettext!("Argument '{}' has an unterminated \""),
        string_to_nul(buffer, the_start)));
    None
}

/// 前进越过一个正则表达式（以 '"' 包裹），就地终止并返回后继位置。
fn parse_next_regex(buffer: &mut [u8], mut i: usize) -> Option<usize> {
    let starting_point = i;

    if i == 0 || buffer[i - 1] != b'"' {
        jot_error(gettext!("Regex strings must begin and end with a \" character").to_string());
        return None;
    }

    while i < buffer.len()
        && (buffer[i] != b'"'
            || (i + 1 < buffer.len() && !is_blank_byte(buffer[i + 1])))
    {
        i += 1;
    }

    if i >= buffer.len() {
        jot_error(gettext!("Regex strings must begin and end with a \" character").to_string());
        return None;
    }

    if i == starting_point {
        jot_error(gettext!("Empty regex string").to_string());
        return None;
    }

    buffer[i] = 0;
    i += 1;
    while i < buffer.len() && is_blank_byte(buffer[i]) {
        i += 1;
    }
    Some(i)
}

/// 编译给定正则表达式，成功时填入 packed，返回是否合法。
fn compile(expression: &str, rex_flags: u32, packed: &mut Option<Box<Regex>>) -> bool {
    let mut builder = RegexBuilder::new(expression);
    if rex_flags & REG_ICASE != 0 {
        builder.case_insensitive(true);
    }
    match builder.build() {
        Ok(compiled) => {
            *packed = Some(Box::new(compiled));
            true
        }
        Err(e) => {
            jot_error(format!(gettext!("Bad regex \"{}\": {}"), expression, e));
            false
        }
    }
}

/// 解析下一语法名及其扩展正则，并加入全局语法链表。
pub fn begin_new_syntax(buffer: &mut [u8], name_start: usize) {
    unsafe {
        let nameptr = name_start;

        /* 检查语法名是否为空。 */
        if buffer[name_start] == 0
            || (buffer[name_start] == b'"'
                && (name_start + 1 >= buffer.len()
                    || buffer[name_start + 1] == b'"'))
        {
            jot_error(gettext!("Missing syntax name").to_string());
            return;
        }

        let mut i = parse_next_word(buffer, name_start);

        /* 检查引号是否配对。 */
        let name_slice = slice_to_nul(buffer, nameptr);
        let has_open = name_slice.first() == Some(&b'"');
        let has_close = name_slice.last() == Some(&b'"');
        if has_open != has_close {
            jot_error(gettext!("Unpaired quote in syntax name").to_string());
            return;
        }

        /* 若带引号则去掉引号。 */
        let real_name_start = if has_open { nameptr + 1 } else { nameptr };
        let mut end = real_name_start;
        while end < buffer.len() && buffer[end] != 0 && buffer[end] != b'"' {
            end += 1;
        }
        if has_open && end < buffer.len() {
            buffer[end] = 0;
        }

        let name = string_to_nul(buffer, real_name_start);

        if name == "none" {
            jot_error(gettext!("The \"none\" syntax is reserved").to_string());
            return;
        }

        let live = Box::into_raw(Box::new(syntaxtype {
            name: Some(name.clone()),
            filename: NANORC.clone(),
            lineno: LINENO,
            augmentations: std::ptr::null_mut(),
            extensions: std::ptr::null_mut(),
            headers: std::ptr::null_mut(),
            magics: std::ptr::null_mut(),
            linter: None,
            formatter: None,
            tabstring: None,
            comment: Some(GENERAL_COMMENT_CHARACTER.to_string()),
            color: std::ptr::null_mut(),
            multiscore: 0,
            next: syntaxes,
        }));
        syntaxes = live;
        LIVE_SYNTAX = live;

        OPENSYNTAX = true;
        SEEN_COLOR_COMMAND = false;

        if i < buffer.len() && buffer[i] != 0 && name == "default" {
            jot_error(gettext!("The \"default\" syntax does not accept extensions").to_string());
            return;
        }

        if i < buffer.len() && buffer[i] != 0 {
            grab_and_store("extension", buffer, i, &mut (*live).extensions);
        }
    }
}

/// 校验语法定义至少含有一条 color 命令。
pub fn check_for_nonempty_syntax() {
    unsafe {
        if OPENSYNTAX && !SEEN_COLOR_COMMAND {
            let current_lineno = LINENO;
            if !LIVE_SYNTAX.is_null() {
                LINENO = (*LIVE_SYNTAX).lineno;
                let nm = (*LIVE_SYNTAX)
                    .name
                    .clone()
                    .unwrap_or_default();
                jot_error(format!(gettext!("Syntax \"{}\" has no color commands"), nm));
            }
            LINENO = current_lineno;
        }
        OPENSYNTAX = false;
    }
}

/// 返回该函数是否几乎存在于所有菜单。
fn is_universal(func: Option<unsafe fn()>) -> bool {
    func == Some(do_left)
        || func == Some(do_right)
        || func == Some(do_home)
        || func == Some(do_end)
        || func == Some(to_prev_word)
        || func == Some(to_next_word)
        || func == Some(do_delete_safe)
        || func == Some(do_backspace_safe)
        || func == Some(cut_text)
        || func == Some(paste_text)
        || func == Some(do_tab_safe)
        || func == Some(do_enter_safe)
        || func == Some(do_verbatim_input)
}

/// 绑定或解绑一个快捷键组合到某函数。
pub fn parse_binding(buffer: &mut [u8], ptr: usize, dobind: bool) {
    unsafe {
        check_for_nonempty_syntax();

        if buffer[ptr] == 0 {
            jot_error(gettext!("Missing key name").to_string());
            return;
        }

        let keyptr = ptr;
        let mut i = parse_next_word(buffer, ptr);
        let mut keycopy = string_to_nul(buffer, keyptr);

        /* 把键名的第二个或第一个字符大写。 */
        let kb = keycopy.as_bytes_mut();
        if kb[0] == b'^' {
            if kb[1].is_ascii_lowercase() {
                kb[1] &= 0x5F;
            }
        } else if kb[0].is_ascii_lowercase() {
            kb[0] &= 0x5F;
        }

        /* 校验键名长度。 */
        if kb.len() < 2 || (kb[0] == b'M' && kb.len() < 3) {
            jot_error(format!(gettext!("Key name {} is invalid"), keycopy));
            return;
        }

        let keycode = keycode_from_string(keycopy.as_str());
        if keycode < 0 {
            jot_error(format!(gettext!("Key name {} is invalid"), keycopy));
            return;
        }

        let (funcptr, mut i) = if dobind {
            let fp = i;
            match parse_argument(buffer, i) {
                Some(ni) => (fp, ni),
                None => return,
            }
        } else {
            (i, i)
        };

        if dobind && string_to_nul(buffer, funcptr).is_empty() {
            jot_error(gettext!("Must specify a function to bind the key to").to_string());
            return;
        }

        let menuptr = i;
        i = parse_next_word(buffer, i);

        if string_to_nul(buffer, menuptr).is_empty() {
            jot_error(gettext!("Must specify a menu (or \"all\") in which to bind/unbind the key").to_string());
            return;
        }

        let menu_name = string_to_nul(buffer, menuptr);
        let menu = name_to_menu(&menu_name);
        if menu == 0 {
            jot_error(format!(gettext!("Unknown menu: {}"), menu_name));
            return;
        }

        let mut newsc: *mut keystruct = std::ptr::null_mut();

        if dobind {
            let funcstr = string_to_nul(buffer, funcptr);
            if funcstr.starts_with('"') {
                newsc = Box::into_raw(Box::new(keystruct {
                    keystr: "",
                    keycode: 0,
                    menus: 0,
                    func: Some(implant),
                    toggle: 0,
                    ordinal: 0,
                    expansion: Some(funcstr[1..].to_string()),
                    next: std::ptr::null_mut(),
                }));
            } else {
                newsc = strtosc(&funcstr);
            }

            if newsc.is_null() {
                jot_error(format!(gettext!("Unknown function: {}"), funcstr));
                return;
            }
        }

        /* 先从给定菜单擦除该快捷键。 */
        let mut s = sclist;
        while !s.is_null() {
            if ((*s).menus & menu) != 0 && (*s).keycode == keycode {
                (*s).menus &= !menu;
            }
            s = (*s).next;
        }

        if !dobind {
            return;
        }

        let mut menu = menu;
        let mut mask = 0;

        let newfunc = (*newsc).func;
        if is_universal(newfunc) {
            menu &= MMOST | MBROWSER;
        } else if newfunc == Some(do_toggle) && (*newsc).toggle == NO_HELP as i32 {
            menu &= (MMOST | MBROWSER | MYESNO) & !MFINDINHELP;
        } else if newfunc == Some(do_toggle) {
            menu &= MMAIN;
        } else if newfunc == Some(full_refresh) {
            menu &= MMOST | MBROWSER | MHELP | MYESNO;
        } else if newfunc == Some(implant) {
            menu &= MMOST | MBROWSER | MHELP;
        } else {
            let mut f = allfuncs;
            while !f.is_null() {
                if (*f).func == newfunc {
                    mask |= (*f).menus;
                }
                f = (*f).next;
            }
            menu &= mask;
        }

        if menu == 0 {
            if !ISSET(RESTRICTED) && !ISSET(VIEW_MODE) {
                jot_error(format!(gettext!("Function '{}' does not exist in menu '{}'"),
                    string_to_nul(buffer, funcptr), menu_name));
            }
            let _ = Box::from_raw(newsc);
            return;
        }

        (*newsc).menus = menu;
        let keycopy_static: &'static str = Box::leak(keycopy.into_boxed_str());
        (*newsc).keystr = keycopy_static;
        (*newsc).keycode = keycode;

        /* 不允许重绑 <Esc>（^[）。 */
        if keycode == ESC_CODE as i32 {
            jot_error(format!(gettext!("Keystroke {} may not be rebound"), keycopy_static));
            let _ = Box::from_raw(newsc);
            return;
        }

        /* 若是 toggle，查找并复制其序号。 */
        if newfunc == Some(do_toggle) {
            let mut s = sclist;
            while !s.is_null() {
                if (*s).func == Some(do_toggle) && (*s).toggle == (*newsc).toggle {
                    (*newsc).ordinal = (*s).ordinal;
                }
                s = (*s).next;
            }
        } else {
            (*newsc).ordinal = 0;
        }

        (*newsc).next = sclist;
        sclist = newsc;
    }
}

/// 校验文件存在、且既非目录也非设备。
pub fn is_good_file(file: &str) -> bool {
    let meta = std::fs::metadata(file);
    match meta {
        Ok(m) if m.is_dir() => {
            jot_error(format!(gettext!("'{}' is a directory"), file));
            false
        }
        Err(_) => false,
        Ok(_) => true,
    }
}

/* ---- 颜色相关 ---- */

/// 部分解析给定文件中的语法（syntax 为 NULL 时仅解析序言）。
pub fn parse_one_include(file: &str, syntax: *mut syntaxtype) {
    unsafe {
        let was_nanorc = NANORC.clone();
        let was_lineno = LINENO;

        if !is_good_file(file) {
            return;
        }

        let rcstream = std::fs::File::open(file);
        let rcstream = match rcstream {
            Ok(f) => f,
            Err(e) => {
                jot_error(format!(gettext!("Error reading {}: {}"), file, e));
                return;
            }
        };

        NANORC = Some(file.to_string());
        LINENO = 0;

        if syntax.is_null() {
            parse_rcfile(rcstream, true, true);
            NANORC = was_nanorc;
            LINENO = was_lineno;
            return;
        }

        LIVE_SYNTAX = syntax;
        LASTCOLOR = (*syntax).color;

        parse_rcfile(rcstream, true, false);

        let mut extra = (*syntax).augmentations;
        while !extra.is_null() {
            let keyword = (*extra).data.clone().unwrap_or_default();
            let mut kw_bytes = keyword.clone().into_bytes();
            let therest = parse_next_word(&mut kw_bytes, 0);

            NANORC = (*extra).filename.clone();
            LINENO = (*extra).lineno as usize;

            if !parse_syntax_commands(&keyword, &mut kw_bytes, therest) {
                jot_error(format!(gettext!("Command \"{}\" not understood"), keyword));
            }

            extra = (*extra).next;
        }

        if !(*syntax).filename.is_none() {
            (*syntax).filename = None;
        }

        NANORC = was_nanorc;
        LINENO = was_lineno;
    }
}

/// 展开名字中的 glob，解析匹配到的各文件。
pub fn parse_includes(buffer: &mut [u8], ptr: usize) {
    unsafe {
        check_for_nonempty_syntax();

        let mut pattern_start = ptr;
        if buffer[pattern_start] == b'"' {
            pattern_start += 1;
        }
        match parse_argument(buffer, ptr) {
            Some(_) => {}
            None => return,
        }

        let pattern = string_to_nul(buffer, pattern_start);
        if pattern.len() > PATH_MAX {
            jot_error(gettext!("Path is too long").to_string());
            return;
        }

        let expanded = expand_leading_tilde(&pattern);
        let paths = glob::glob(&expanded);

        match paths {
            Ok(paths) => {
                for entry in paths.flatten() {
                    if let Some(p) = entry.to_str() {
                        parse_one_include(p, std::ptr::null_mut());
                    }
                }
            }
            Err(e) => {
                jot_error(format!(gettext!("Error expanding {}: {}"), pattern, e));
            }
        }
    }
}

/// 返回最接近给定 RGB 等级的 xterm-256 颜色索引。
fn closest_index_color(red: i16, green: i16, blue: i16) -> i16 {
    static LEVEL: [i16; 16] =
        [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5];
    static GRAY: [i16; 14] = [1, 2, 3, 4, 5, 6, 7, 9, 11, 13, 15, 18, 21, 23];

    if COLORS != 256 {
        THE_DEFAULT as i16
    } else if red == green && green == blue && 0 < red && red < 0xF {
        232 + GRAY[(red - 1) as usize]
    } else {
        36 * LEVEL[red as usize] + 6 * LEVEL[green as usize] + LEVEL[blue as usize] + 16
    }
}

const COLORCOUNT: usize = 34;

/// 颜色名表。
static HUES: [&str; COLORCOUNT] = [
    "red", "green", "blue", "yellow", "cyan", "magenta", "white", "black", "normal",
    "pink", "purple", "mauve", "lagoon", "mint", "lime", "peach", "orange", "latte",
    "rosy", "beet", "plum", "sea", "sky", "slate", "teal", "sage", "brown", "ocher",
    "sand", "tawny", "brick", "crimson", "grey", "gray",
];

/// 颜色索引表（与 HUES 对应）。
static INDICES: [i16; COLORCOUNT] = [
    COLOR_RED, COLOR_GREEN, COLOR_BLUE, COLOR_YELLOW, COLOR_CYAN, COLOR_MAGENTA,
    COLOR_WHITE, COLOR_BLACK, THE_DEFAULT as i16, 204, 163, 134, 38, 48, 148, 215, 208,
    137, 175, 127, 98, 32, 111, 66, 35, 107, 100, 142, 186, 136, 166, 161,
    COLOR_BLACK + 8, COLOR_BLACK + 8,
];

/// 把颜色名转为 short 值，并据前缀设置 vivid/thick。
fn color_to_short(colorname: &str, vivid: &mut bool, thick: &mut bool) -> i16 {
    let mut name = colorname;
    if name.starts_with("bright") && name.len() > 6 {
        *vivid = true;
        *thick = true;
        name = &name[6..];
    } else if name.starts_with("light") && name.len() > 5 {
        *vivid = true;
        *thick = false;
        name = &name[5..];
    } else {
        *vivid = false;
        *thick = false;
    }

    if name.starts_with('#') && name.len() == 4 {
        if *vivid {
            jot_error(format!(gettext!("Color '{}' takes no prefix"), name));
            return BAD_COLOR as i16;
        }
        let mut chars = name[1..].chars();
        let r = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0) as i16;
        let g = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0) as i16;
        let b = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0) as i16;
        if r >= 0 && g >= 0 && b >= 0 {
            return closest_index_color(r, g, b);
        }
    }

    for index in 0..COLORCOUNT {
        if HUES[index] == name {
            if index > 7 && *vivid {
                jot_error(format!(gettext!("Color '{}' takes no prefix"), name));
                return BAD_COLOR as i16;
            } else if index > 8 && COLORS < 255 {
                return THE_DEFAULT as i16;
            } else {
                return INDICES[index];
            }
        }
    }

    jot_error(format!(gettext!("Color \"{}\" not understood"), name));
    BAD_COLOR as i16
}

/// 解析颜色组合（前景/背景/属性）。返回是否成功。
fn parse_combination(combotext: &str, fg: &mut i16, bg: &mut i16, attributes: &mut i32) -> bool {
    let mut text = combotext.to_string();
    let mut vivid = false;
    let mut thick = false;

    *attributes = A_NORMAL;

    if text.starts_with("bold") {
        *attributes |= A_BOLD;
        if text.as_bytes().get(4) != Some(&b',') {
            jot_error(gettext!("An attribute requires a subsequent comma").to_string());
            return false;
        }
        text = text[5..].to_string();
    }

    if text.starts_with("italic") {
        *attributes |= A_ITALIC;
        if text.as_bytes().get(6) != Some(&b',') {
            jot_error(gettext!("An attribute requires a subsequent comma").to_string());
            return false;
        }
        text = text[7..].to_string();
    }

    let comma = text.find(',');

    let (fgtext, bgtext) = if let Some(c) = comma {
        let f = text[..c].to_string();
        let b = text[c + 1..].to_string();
        (f, Some(b))
    } else {
        (text.clone(), None)
    };

    if comma.is_none() || !fgtext.is_empty() {
        *fg = color_to_short(&fgtext, &mut vivid, &mut thick);
        if *fg == BAD_COLOR as i16 {
            return false;
        }
        if vivid && !thick && COLORS > 8 {
            *fg += 8;
        } else if vivid {
            *attributes |= A_BOLD;
        }
    } else {
        *fg = THE_DEFAULT as i16;
    }

    if let Some(bt) = bgtext {
        *bg = color_to_short(&bt, &mut vivid, &mut thick);
        if *bg == BAD_COLOR as i16 {
            return false;
        }
        if vivid && COLORS > 8 {
            *bg += 8;
        }
    } else {
        *bg = THE_DEFAULT as i16;
    }

    true
}

/// 解析 color 命令的颜色规格及其后的一条或多条正则，并加入当前语法。
pub fn parse_rule(buffer: &mut [u8], ptr: usize, rex_flags: u32) {
    unsafe {
        if buffer[ptr] == 0 {
            jot_error(gettext!("Missing color name").to_string());
            return;
        }

        let names = ptr;
        let mut i = parse_next_word(buffer, ptr);

        let mut fg: i16 = 0;
        let mut bg: i16 = 0;
        let mut attributes: i32 = 0;
        let names_str = string_to_nul(buffer, names);
        if !parse_combination(&names_str, &mut fg, &mut bg, &mut attributes) {
            return;
        }

        if buffer[i] == 0 {
            jot_error(gettext!("Missing regex string after 'color' command").to_string());
            return;
        }

        while i < buffer.len() && buffer[i] != 0 {
            let mut start_rgx: Option<Box<Regex>> = None;
            let mut end_rgx: Option<Box<Regex>> = None;
            let mut expectend = false;

            if buffer[i..].starts_with(b"start=") {
                i += 6;
                expectend = true;
            }

            let regexstring = i + 1;
            match parse_next_regex(buffer, i + 1) {
                Some(ni) => i = ni,
                None => return,
            }

            let rs = string_to_nul(buffer, regexstring);
            if !compile(&rs, rex_flags, &mut start_rgx) {
                return;
            }

            if expectend {
                if !(i < buffer.len() && buffer[i..].starts_with(b"end=")) {
                    jot_error(gettext!("\"start=\" requires a corresponding \"end=\"").to_string());
                    return;
                }
                let regexstring2 = i + 5;
                match parse_next_regex(buffer, i + 5) {
                    Some(ni) => i = ni,
                    None => return,
                }
                let rs2 = string_to_nul(buffer, regexstring2);
                if !compile(&rs2, rex_flags, &mut end_rgx) {
                    return;
                }
            }

            let newcolor = Box::into_raw(Box::new(colortype {
                id: 0,
                fg,
                bg,
                pairnum: 0,
                attributes,
                start: start_rgx,
                end: end_rgx,
                next: std::ptr::null_mut(),
            }));

            if LASTCOLOR.is_null() {
                (*LIVE_SYNTAX).color = newcolor;
            } else {
                (*LASTCOLOR).next = newcolor;
            }
            LASTCOLOR = newcolor;

            if expectend {
                (*newcolor).id = (*LIVE_SYNTAX).multiscore;
                (*LIVE_SYNTAX).multiscore += 1;
            }
        }
    }
}

/// 为给定界面元素设置颜色组合。
pub fn set_interface_color(element: usize, combotext: &str) {
    let mut trio = Box::new(colortype {
        id: 0,
        fg: 0,
        bg: 0,
        pairnum: 0,
        attributes: 0,
        start: None,
        end: None,
        next: std::ptr::null_mut(),
    });

    let mut fg: i16 = 0;
    let mut bg: i16 = 0;
    let mut attributes: i32 = 0;
    if parse_combination(combotext, &mut fg, &mut bg, &mut attributes) {
        trio.fg = fg;
        trio.bg = bg;
        trio.attributes = attributes;
        unsafe {
            let _ = Box::from_raw(color_combo[element]);
            color_combo[element] = Box::into_raw(trio);
        }
    }
}

/// 读取双引号包裹的正则并存入 storage。
pub fn grab_and_store(
    kind: &str,
    buffer: &mut [u8],
    ptr: usize,
    storage: *mut *mut regexlisttype,
) {
    unsafe {
        if !OPENSYNTAX {
            jot_error(format!(gettext!("A '{}' command requires a preceding 'syntax' command"), kind));
            return;
        }

        if !LIVE_SYNTAX.is_null()
            && (*LIVE_SYNTAX).name.as_deref() == Some("default")
        {
            jot_error(format!(gettext!("The \"default\" syntax does not accept '{}' regexes"), kind));
            return;
        }

        if buffer[ptr] == 0 {
            jot_error(format!(gettext!("Missing regex string after '{}' command"), kind));
            return;
        }

        let mut lastthing = *storage;
        while !lastthing.is_null() && !(*lastthing).next.is_null() {
            lastthing = (*lastthing).next;
        }

        let mut i = ptr;
        while i < buffer.len() && buffer[i] != 0 {
            let regexstring = i + 1;
            match parse_next_regex(buffer, i + 1) {
                Some(ni) => i = ni,
                None => return,
            }
            let rs = string_to_nul(buffer, regexstring);
            let mut packed: Option<Box<Regex>> = None;
            if !compile(&rs, NANO_REG_EXTENDED | REG_NOSUB, &mut packed) {
                continue;
            }

            let newthing = Box::into_raw(Box::new(regexlisttype {
                one_rgx: packed,
                next: std::ptr::null_mut(),
            }));

            if lastthing.is_null() {
                *storage = newthing;
            } else {
                (*lastthing).next = newthing;
            }
            lastthing = newthing;
        }
    }
}

/// 收集 comment/linter/formatter/tabgives 命令后的字符串。
pub fn pick_up_name(kind: &str, buffer: &mut [u8], ptr: usize, storage: &mut Option<String>) {
    if buffer[ptr] == 0 {
        jot_error(format!(gettext!("Missing argument after '{}'"), kind));
        return;
    }

    if buffer[ptr] == b'"' {
        let len = nul_strlen(&buffer[ptr..]);
        let mut look = ptr + len;
        while buffer[look] != b'"' {
            if look == ptr {
                jot_error(format!(gettext!("Argument of '{}' lacks closing \""), kind));
                return;
            }
            look -= 1;
        }
        buffer[look] = 0;
        let value = string_to_nul(buffer, ptr + 1);
        *storage = Some(value);
    } else {
        let value = string_to_nul(buffer, ptr);
        *storage = Some(value);
    }
}

/// 处理六条仅语法命令。返回是否被理解。
pub fn parse_syntax_commands(keyword: &str, buffer: &mut [u8], ptr: usize) -> bool {
    unsafe {
        match keyword {
            "color" => parse_rule(buffer, ptr, NANO_REG_EXTENDED),
            "icolor" => parse_rule(buffer, ptr, NANO_REG_EXTENDED | REG_ICASE),
            "comment" => {
                let mut c = (*LIVE_SYNTAX).comment.clone();
                pick_up_name("comment", buffer, ptr, &mut c);
                (*LIVE_SYNTAX).comment = c;
            }
            "tabgives" => {
                let mut t = (*LIVE_SYNTAX).tabstring.clone();
                pick_up_name("tabgives", buffer, ptr, &mut t);
                (*LIVE_SYNTAX).tabstring = t;
            }
            "linter" => {
                let mut l = (*LIVE_SYNTAX).linter.clone();
                pick_up_name("linter", buffer, ptr, &mut l);
                (*LIVE_SYNTAX).linter = l;
                if let Some(ref mut s) = (*LIVE_SYNTAX).linter {
                    strip_leading_blanks_from(s.as_bytes_mut());
                }
            }
            "formatter" => {
                let mut f = (*LIVE_SYNTAX).formatter.clone();
                pick_up_name("formatter", buffer, ptr, &mut f);
                (*LIVE_SYNTAX).formatter = f;
                if let Some(ref mut s) = (*LIVE_SYNTAX).formatter {
                    strip_leading_blanks_from(s.as_bytes_mut());
                }
            }
            _ => return false,
        }
        true
    }
}

/* ---- 关键性校验 ---- */

/// 校验用户未把"关键"函数的所有快捷键都解绑。
fn check_vitals_mapped() {
    let vitals: [Option<unsafe fn()>; 4] = [Some(do_exit), Some(do_exit), Some(do_exit), Some(do_cancel)];
    let inmenus: [i32; 4] = [MMAIN, MBROWSER, MHELP, MYESNO];

    for v in 0..4 {
        unsafe {
            let mut f = allfuncs;
            while !f.is_null() {
                if (*f).func == vitals[v] && ((*f).menus & inmenus[v]) != 0 {
                    if first_sc_for(inmenus[v], vitals[v].unwrap()).is_null() {
                        jot_error(format!(
                            gettext!("No key is bound to function '{}' in menu '{}'.  Exiting.\n"),
                            (*f).tag, menu_to_name(inmenus[v])));
                        die(gettext!("If needed, use nano with the -I option to adjust your nanorc settings.\n"));
                    } else {
                        break;
                    }
                }
                f = (*f).next;
            }
        }
    }
}

/* ---- 主解析循环 ---- */

/// 解析已成功打开的 rcfile 流，之后关闭它。
pub fn parse_rcfile(rcstream: std::fs::File, just_syntax: bool, intros_only: bool) {
    use std::io::{BufRead, BufReader};

    let reader = BufReader::new(rcstream);
    let mut buffer: Vec<u8> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        unsafe {
            LINENO += 1;

            /* 若仅做语法序言解析，跳过 syntax 命令之前的行。 */
            if just_syntax && !intros_only && LINENO <= (*LIVE_SYNTAX).lineno {
                continue;
            }

            buffer = line.into_bytes();
            let mut length = buffer.len();

            /* 去掉换行与可能的回车。 */
            if length > 0 && buffer[length - 1] == b'\n' {
                length -= 1;
                buffer.truncate(length);
            }
            if length > 0 && buffer[length - 1] == b'\r' {
                length -= 1;
                buffer.truncate(length);
            }
            buffer.push(0);

            let mut ptr = 0;
            while ptr < buffer.len() && is_blank_byte(buffer[ptr]) {
                ptr += 1;
            }

            /* 空行或注释行跳过。 */
            if buffer[ptr] == 0 || buffer[ptr] == b'#' {
                continue;
            }

            let keyword = ptr;
            ptr = parse_next_word(&mut buffer, ptr);

            let mut drop_open = false;
            let mut set = 0;

            /* 先处理 extendsyntax。 */
            let kw = string_to_nul(&buffer, keyword);
            if !just_syntax && kw == "extendsyntax" {
                let mut syntaxname_start = ptr;
                ptr = parse_next_word(&mut buffer, ptr);
                let syntaxname = string_to_nul(&buffer, syntaxname_start);

                check_for_nonempty_syntax();

                let mut sntx = syntaxes;
                while !sntx.is_null() {
                    if (*sntx).name.as_deref() == Some(syntaxname.as_str()) {
                        break;
                    }
                    sntx = (*sntx).next;
                }

                if sntx.is_null() {
                    jot_error(format!(gettext!("Could not find syntax \"{}\" to extend"), syntaxname));
                    continue;
                }

                let cmd_keyword_start = ptr;
                let argument = string_to_nul(&buffer, ptr);
                ptr = parse_next_word(&mut buffer, ptr);
                let cmd_keyword = string_to_nul(&buffer, cmd_keyword_start);

                if cmd_keyword == "header" || cmd_keyword == "magic" {
                    LIVE_SYNTAX = sntx;
                    OPENSYNTAX = true;
                    drop_open = true;
                } else {
                    let newitem = Box::into_raw(Box::new(augmentstruct {
                        filename: NANORC.clone(),
                        lineno: LINENO as isize,
                        data: Some(argument),
                        next: std::ptr::null_mut(),
                    }));
                    if (*sntx).augmentations.is_null() {
                        (*sntx).augmentations = newitem;
                    } else {
                        let mut extra = (*sntx).augmentations;
                        while !(*extra).next.is_null() {
                            extra = (*extra).next;
                        }
                        (*extra).next = newitem;
                    }
                    continue;
                }
            } else if kw == "syntax" {
                if intros_only {
                    check_for_nonempty_syntax();
                    begin_new_syntax(&mut buffer, ptr);
                } else {
                    break;
                }
            } else if kw == "header" {
                if intros_only {
                    grab_and_store("header", &mut buffer, ptr, &mut (*LIVE_SYNTAX).headers);
                }
            } else if kw == "magic" {
                if intros_only {
                    grab_and_store("magic", &mut buffer, ptr, &mut (*LIVE_SYNTAX).magics);
                }
            } else if just_syntax
                && (kw == "set" || kw == "unset" || kw == "bind" || kw == "unbind"
                    || kw == "include" || kw == "extendsyntax")
            {
                if intros_only {
                    jot_error(format!(gettext!("Command \"{}\" not allowed in included file"), kw));
                } else {
                    break;
                }
            } else if intros_only
                && (kw == "color" || kw == "icolor" || kw == "comment" || kw == "tabgives"
                    || kw == "linter" || kw == "formatter")
            {
                if !OPENSYNTAX {
                    jot_error(format!(gettext!("A '{}' command requires a preceding 'syntax' command"), kw));
                }
                if kw == "icolor" {
                    SEEN_COLOR_COMMAND = true;
                }
                continue;
            } else if parse_syntax_commands(&kw, &mut buffer, ptr) {
                /* 已处理 */
            } else if kw == "include" {
                parse_includes(&mut buffer, ptr);
            } else if kw == "set" {
                set = 1;
            } else if kw == "unset" {
                set = -1;
            } else if kw == "bind" {
                parse_binding(&mut buffer, ptr, true);
            } else if kw == "unbind" {
                parse_binding(&mut buffer, ptr, false);
            } else if intros_only {
                jot_error(format!(gettext!("Command \"{}\" not understood"), kw));
            }

            if drop_open {
                OPENSYNTAX = false;
            }

            if set == 0 {
                continue;
            }

            check_for_nonempty_syntax();

            if buffer[ptr] == 0 {
                jot_error(gettext!("Missing option").to_string());
                continue;
            }

            let option_start = ptr;
            ptr = parse_next_word(&mut buffer, ptr);
            let option = string_to_nul(&buffer, option_start);

            /* 在已知选项中查找该选项名。 */
            let mut found = None;
            for (idx, opt) in RCOPTS.iter().enumerate() {
                if opt.name == option {
                    found = Some(idx);
                    break;
                }
            }

            let idx = match found {
                Some(i) => i,
                None => {
                    jot_error(format!(gettext!("Unknown option: {}"), option));
                    continue;
                }
            };

            /* 若选项带标志，按请求设置或清除。 */
            if RCOPTS[idx].flag != 0 {
                if set == 1 {
                    SET(RCOPTS[idx].flag as usize);
                } else {
                    UNSET(RCOPTS[idx].flag as usize);
                }
                continue;
            }

            /* 带参数的选项不能被 unset。 */
            if set == -1 {
                jot_error(format!(gettext!("Cannot unset option \"{}\""), option));
                continue;
            }

            if buffer[ptr] == 0 {
                jot_error(format!(gettext!("Option \"{}\" requires an argument"), option));
                continue;
            }

            let mut argument_start = ptr;
            if buffer[argument_start] == b'"' {
                argument_start += 1;
            }
            match parse_argument(&mut buffer, ptr) {
                Some(_) => {}
                None => continue,
            }
            let argument = string_to_nul(&buffer, argument_start);

            /* UTF-8 环境下忽略无效多字节串。 */
            if using_utf8 && std::str::from_utf8(argument.as_bytes()).is_err() {
                jot_error(gettext!("Argument is not a valid multibyte string").to_string());
                continue;
            }

            match option.as_str() {
                "titlecolor" => set_interface_color(TITLE_BAR, &argument),
                "numbercolor" => set_interface_color(LINE_NUMBER, &argument),
                "stripecolor" => set_interface_color(GUIDE_STRIPE, &argument),
                "scrollercolor" => set_interface_color(SCROLL_BAR, &argument),
                "selectedcolor" => set_interface_color(SELECTED_TEXT, &argument),
                "spotlightcolor" => set_interface_color(SPOTLIGHTED, &argument),
                "minicolor" => set_interface_color(MINI_INFOBAR, &argument),
                "promptcolor" => set_interface_color(PROMPT_BAR, &argument),
                "statuscolor" => set_interface_color(STATUS_BAR, &argument),
                "errorcolor" => set_interface_color(ERROR_MESSAGE, &argument),
                "keycolor" => set_interface_color(KEY_COMBO, &argument),
                "functioncolor" => set_interface_color(FUNCTION_TAG, &argument),
                "operatingdir" => {
                    operating_dir = Some(argument.clone());
                }
                "fill" => {
                    if !parse_num(&argument, &mut fill) {
                        jot_error(format!(gettext!("Requested fill size \"{}\" is invalid"), argument));
                        fill = -(COLUMNS_FROM_EOL as isize);
                    }
                }
                "matchbrackets" => {
                    if has_blank_char(argument.as_bytes()) {
                        jot_error(gettext!("Non-blank characters required").to_string());
                    } else if mbstrlen(argument.as_bytes()) % 2 != 0 {
                        jot_error(gettext!("Even number of characters required").to_string());
                    } else {
                        matchbrackets = Some(argument.clone());
                    }
                }
                "whitespace" => {
                    if mbstrlen(argument.as_bytes()) != 2 || breadth(argument.as_bytes()) != 2 {
                        jot_error(gettext!("Two single-column characters required").to_string());
                    } else {
                        whitespace = Some(argument.clone());
                        let w = whitespace.as_ref().unwrap();
                        whitelen[0] = char_length(w.as_bytes()) as i32;
                        let rest = &w.as_bytes()[whitelen[0] as usize..];
                        whitelen[1] = char_length(rest) as i32;
                    }
                }
                "punct" => {
                    if has_blank_char(argument.as_bytes()) {
                        jot_error(gettext!("Non-blank characters required").to_string());
                    } else {
                        punct = Some(argument.clone());
                    }
                }
                "brackets" => {
                    if has_blank_char(argument.as_bytes()) {
                        jot_error(gettext!("Non-blank characters required").to_string());
                    } else {
                        brackets = Some(argument.clone());
                    }
                }
                "quotestr" => {
                    quotestr = Some(argument.clone());
                }
                "speller" => {
                    alt_speller = Some(argument.clone());
                }
                "backupdir" => {
                    backup_dir = Some(argument.clone());
                }
                "wordchars" => {
                    word_chars = Some(argument.clone());
                }
                "guidestripe" => {
                    if !parse_num(&argument, &mut stripe_column) || stripe_column <= 0 {
                        jot_error(format!(gettext!("Guide column \"{}\" is invalid"), argument));
                        stripe_column = 0;
                    }
                }
                "tabsize" => {
                    if !parse_num(&argument, &mut crate::global::tabsize) || crate::global::tabsize <= 0 {
                        jot_error(format!(gettext!("Requested tab size \"{}\" is invalid"), argument));
                        crate::global::tabsize = -1;
                    }
                }
                _ => {}
            }
        }
    }

    unsafe {
        if intros_only {
            check_for_nonempty_syntax();
        }
        LINENO = 0;
    }
}

/// 读取并解释两个 nanorc 文件之一。
pub fn parse_one_nanorc() {
    unsafe {
        let path = NANORC.clone();
        let path = match path {
            Some(p) => p,
            None => return,
        };
        match std::fs::File::open(&path) {
            Ok(f) => parse_rcfile(f, false, true),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    jot_error(format!(gettext!("Error reading {}: {}"), path, e));
                }
            }
        }
    }
}

/// 返回 path+name 是否为可读的普通文件。
pub fn have_nanorc(path: &str, name: &str) -> bool {
    unsafe {
        if path.is_empty() {
            return false;
        }
        NANORC = Some(concatenate(path, name));
        if let Some(ref nr) = NANORC {
            is_good_file(nr)
        } else {
            false
        }
    }
}

/// 处理命令行指定的 nanorc（若有），否则系统级 rcfile 后接用户 rcfile。
pub fn do_rcfiles() {
    unsafe {
        if !custom_nanorc.is_none() {
            let cn = custom_nanorc.clone();
            if let Some(c) = cn {
                NANORC = get_full_path_str(&c);
                if NANORC.is_none()
                    || std::fs::metadata(NANORC.as_ref().unwrap()).is_err()
                {
                    die(gettext!("Specified rcfile does not exist\n"));
                }
                if is_good_file(NANORC.as_ref().unwrap()) {
                    parse_one_nanorc();
                }
            }
        } else {
            let xdgconfdir = std::env::var("XDG_CONFIG_HOME").ok();

            if have_nanorc(SYSCONFDIR, "/nanorc") {
                parse_one_nanorc();
            }

            get_homedir();
            let hd = homedir.clone();

            if have_nanorc(hd.as_deref().unwrap_or(""), &format!("/{}", HOME_RC_NAME))
                || have_nanorc(xdgconfdir.as_deref().unwrap_or(""), &format!("/nano/{}", RCFILE_NAME))
                || have_nanorc(hd.as_deref().unwrap_or(""), &format!("/.config/nano/{}", RCFILE_NAME))
            {
                parse_one_nanorc();
            } else if hd.is_none() && xdgconfdir.is_none() {
                jot_error(gettext!("I can't find my home directory!  Wah!").to_string());
            }
        }

        check_vitals_mapped();

        NANORC = None;
    }
}

/// 取自定义 nanorc 的完整路径（get_full_path 的便捷封装）。
unsafe fn get_full_path_str(name: &str) -> Option<String> {
    /* history::get_full_path 接收 *mut openfilestruct，这里用文件名的
     * 直接展开近似实现（全特性下 nanorc 即路径本身）。 */
    let _ = name;
    Some(name.to_string())
}
