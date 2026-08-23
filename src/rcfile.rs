/**************************************************************************
 * rcfile.rs  --  GNU nano 配置文件解析（对应 rcfile.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! nanorc 配置文件解析。对应原版 nano 的 `rcfile.c`。
//! 转换说明：使用 `MatchPattern` 替代 POSIX regex；解析流程（两阶段：
//! intro 扫描 + 语法完整加载、extendsyntax 延迟应用、include glob）与
//! 原版 rcfile.c 的 `parse_rcfile(FILE*, just_syntax, intros_only)` 对齐。

use crate::definitions::*;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use crate::{chars, color, files, global, utils};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// ======================== 会话状态（对应 rcfile.c 的 static 变量） ========================

// syntax 解析的会话状态（对应 rcfile.c 的 static 变量）。
thread_local! {
    static LIVE_SYNTAX: RefCell<Option<SyntaxRef>> = const { RefCell::new(None) };
    static LAST_COLOR: RefCell<Option<ColorRef>> = const { RefCell::new(None) };
    static OPEN_SYNTAX: Cell<bool> = const { Cell::new(false) };
    static SEEN_COLOR_COMMAND: Cell<bool> = const { Cell::new(false) };
}

fn get_live_syntax() -> Option<SyntaxRef> {
    LIVE_SYNTAX.with(|s| s.borrow().clone())
}
fn set_live_syntax(s: Option<SyntaxRef>) {
    LIVE_SYNTAX.with(|x| *x.borrow_mut() = s);
}
fn get_last_color() -> Option<ColorRef> {
    LAST_COLOR.with(|c| c.borrow().clone())
}
fn set_last_color(c: Option<ColorRef>) {
    LAST_COLOR.with(|x| *x.borrow_mut() = c);
}
fn open_syntax() -> bool {
    OPEN_SYNTAX.with(|x| x.get())
}
fn set_open_syntax(v: bool) {
    OPEN_SYNTAX.with(|x| x.set(v));
}
fn set_seen_color_command(v: bool) {
    SEEN_COLOR_COMMAND.with(|x| x.set(v));
}
fn seen_color_command() -> bool {
    SEEN_COLOR_COMMAND.with(|x| x.get())
}

// nanorc 文件状态（对应 rcfile.c 的 static 变量 nanorc、lineno）。
thread_local! {
    static ERRORS_HEAD: RefCell<Option<LineRef>> = const { RefCell::new(None) };
    static ERRORS_TAIL: RefCell<Option<LineRef>> = const { RefCell::new(None) };
    static NANORC_FILE: RefCell<Option<String>> = const { RefCell::new(None) };
    static NANORC_LINENO: Cell<usize> = const { Cell::new(0) };
}

/// 设置当前正在解析的 nanorc 文件名（供 jot_error 使用）。
pub(crate) fn set_nanorc(name: Option<String>) {
    NANORC_FILE.with(|n| *n.borrow_mut() = name);
}

/// 设置当前解析行号（供 jot_error 使用）。
pub(crate) fn set_rcfile_lineno(lineno: usize) {
    NANORC_LINENO.with(|l| l.set(lineno));
}

fn get_rcfile_lineno() -> usize {
    NANORC_LINENO.with(|l| l.get())
}

/// 当前 nanorc 文件名。
fn get_nanorc() -> Option<String> {
    NANORC_FILE.with(|n| n.borrow().clone())
}

// ======================== 字符串解析辅助（对应 rcfile.c） ========================

/// 从字符串开头取第一个词：返回 (词, 剩余部分)（对应 `parse_next_word`）。
fn next_word(s: &str) -> (&str, &str) {
    let s = s.trim_start_matches([' ', '\t']);
    let end = s.find(|c: char| c == ' ' || c == '\t').unwrap_or(s.len());
    (&s[..end], s[end..].trim_start_matches([' ', '\t']))
}

/// 解析一个参数（可选双引号包裹），返回 (参数内容, 剩余部分)。
/// 以 `"` 开头时，行内最后一个 `"` 指示其结束（对应 `parse_argument`）。
/// 失败（引号未闭合）返回 None 并已报错。
fn parse_argument(ptr: &str) -> Option<(&str, &str)> {
    if !ptr.starts_with('"') {
        let (word, rest) = next_word(ptr);
        return Some((word, rest));
    }
    match ptr.rfind('"') {
        Some(idx) if idx > 0 => Some((&ptr[1..idx], ptr[idx + 1..].trim_start_matches([' ', '\t']))),
        _ => {
            jot_error(&crate::t!("rcfile-missing_quote", kind = "argument"));
            None
        }
    }
}

/// 解析由 `"` 包裹的一个正则串，返回 (内容, 后续位置)。
/// 引号后必须是空白或行尾（对应 `parse_next_regex`）。
fn read_regex(rest: &str, pos: usize) -> Option<(String, usize)> {
    let bytes = rest.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'"' {
        jot_error(&crate::t!("rcfile-regex_quotes"));
        return None;
    }
    let start = pos + 1;
    let mut i = start;
    loop {
        if i >= bytes.len() {
            jot_error(&crate::t!("rcfile-regex_quotes"));
            return None;
        }
        if bytes[i] == b'"' && (i + 1 >= bytes.len() || bytes[i + 1].is_ascii_whitespace()) {
            break;
        }
        i += 1;
    }
    if i == start {
        jot_error(&crate::t!("rcfile-empty_regex"));
        return None;
    }
    Some((rest[start..i].to_string(), i + 1))
}

/// 解析一行中的正则列表（每个以 `"` 包裹）。
fn parse_regex_list(rest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = rest.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        match read_regex(rest, pos) {
            Some((rgx, np)) => {
                out.push(rgx);
                pos = np;
            }
            None => break,
        }
    }
    out
}

// ======================== 选项表（对应 rcfile.c 的 rcopts） ========================

/// 单个 rc 选项：flag 非 0 表示开关选项（对应 SET/UNSET），0 表示带参选项。
struct RcOption {
    name: &'static str,
    flag: usize,
}

#[rustfmt::skip]
const RCOPTS: &[RcOption] = &[
    RcOption { name: "boldtext", flag: BOLD_TEXT },
    RcOption { name: "brackets", flag: 0 },
    RcOption { name: "breaklonglines", flag: BREAK_LONG_LINES },
    RcOption { name: "casesensitive", flag: CASE_SENSITIVE },
    RcOption { name: "constantshow", flag: CONSTANT_SHOW },
    RcOption { name: "fill", flag: 0 },
    RcOption { name: "historylog", flag: HISTORYLOG },
    RcOption { name: "linenumbers", flag: LINE_NUMBERS },
    RcOption { name: "magic", flag: USE_MAGIC },
    RcOption { name: "mouse", flag: USE_MOUSE },
    RcOption { name: "multibuffer", flag: NEW_BUFFER },
    RcOption { name: "newbuffer", flag: NEW_BUFFER },
    RcOption { name: "nohelp", flag: NO_HELP },
    RcOption { name: "nonewlines", flag: NO_NEWLINES },
    RcOption { name: "nowrap", flag: NO_WRAP },
    RcOption { name: "operatingdir", flag: 0 },
    RcOption { name: "positionlog", flag: POSITIONLOG },
    RcOption { name: "preserve", flag: PRESERVE },
    RcOption { name: "punct", flag: 0 },
    RcOption { name: "quotestr", flag: 0 },
    RcOption { name: "quickblank", flag: QUICK_BLANK },
    RcOption { name: "rawsequences", flag: RAW_SEQUENCES },
    RcOption { name: "rebinddelete", flag: REBIND_DELETE },
    RcOption { name: "regexp", flag: USE_REGEXP },
    RcOption { name: "saveonexit", flag: SAVE_ON_EXIT },
    RcOption { name: "speller", flag: 0 },
    RcOption { name: "afterends", flag: AFTER_ENDS },
    RcOption { name: "allow_insecure_backup", flag: INSECURE_BACKUP },
    RcOption { name: "atblanks", flag: AT_BLANKS },
    RcOption { name: "autoindent", flag: AUTOINDENT },
    RcOption { name: "backup", flag: MAKE_BACKUP },
    RcOption { name: "backupdir", flag: 0 },
    RcOption { name: "bookstyle", flag: BOOKSTYLE },
    RcOption { name: "colonparsing", flag: COLON_PARSING },
    RcOption { name: "cutfromcursor", flag: CUT_FROM_CURSOR },
    RcOption { name: "emptyline", flag: EMPTY_LINE },
    RcOption { name: "guidestripe", flag: 0 },
    RcOption { name: "indicator", flag: INDICATOR },
    RcOption { name: "jumpyscrolling", flag: JUMPY_SCROLLING },
    RcOption { name: "locking", flag: LOCKING },
    RcOption { name: "matchbrackets", flag: 0 },
    RcOption { name: "minibar", flag: MINIBAR },
    RcOption { name: "noconvert", flag: NO_CONVERT },
    RcOption { name: "showcursor", flag: SHOW_CURSOR },
    RcOption { name: "smarthome", flag: SMART_HOME },
    RcOption { name: "softwrap", flag: SOFTWRAP },
    RcOption { name: "solosidescroll", flag: SOLO_SIDESCROLL },
    RcOption { name: "stateflags", flag: STATEFLAGS },
    RcOption { name: "tabsize", flag: 0 },
    RcOption { name: "tabstospaces", flag: TABS_TO_SPACES },
    RcOption { name: "trimblanks", flag: TRIM_BLANKS },
    RcOption { name: "unix", flag: MAKE_IT_UNIX },
    RcOption { name: "whitespace", flag: 0 },
    RcOption { name: "whitespacedisplay", flag: WHITESPACE_DISPLAY },
    RcOption { name: "wordbounds", flag: WORD_BOUNDS },
    RcOption { name: "wordchars", flag: 0 },
    RcOption { name: "zap", flag: LET_THEM_ZAP },
    RcOption { name: "zero", flag: ZERO },
    RcOption { name: "titlecolor", flag: 0 },
    RcOption { name: "numbercolor", flag: 0 },
    RcOption { name: "stripecolor", flag: 0 },
    RcOption { name: "scrollercolor", flag: 0 },
    RcOption { name: "selectedcolor", flag: 0 },
    RcOption { name: "spotlightcolor", flag: 0 },
    RcOption { name: "minicolor", flag: 0 },
    RcOption { name: "promptcolor", flag: 0 },
    RcOption { name: "statuscolor", flag: 0 },
    RcOption { name: "errorcolor", flag: 0 },
    RcOption { name: "keycolor", flag: 0 },
    RcOption { name: "functioncolor", flag: 0 },
];

/// 处理带参数的 set 选项（对应 rcfile.c parse_rcfile 中的各选项分支）。
fn parse_valued_option(option: &str, argument: &str) {
    match option {
        "titlecolor" => color::set_interface_color(TITLE_BAR, argument),
        "numbercolor" => color::set_interface_color(LINE_NUMBER, argument),
        "stripecolor" => color::set_interface_color(GUIDE_STRIPE, argument),
        "scrollercolor" => color::set_interface_color(SCROLL_BAR, argument),
        "selectedcolor" => color::set_interface_color(SELECTED_TEXT, argument),
        "spotlightcolor" => color::set_interface_color(SPOTLIGHTED, argument),
        "minicolor" => color::set_interface_color(MINI_INFOBAR, argument),
        "promptcolor" => color::set_interface_color(PROMPT_BAR, argument),
        "statuscolor" => color::set_interface_color(STATUS_BAR, argument),
        "errorcolor" => color::set_interface_color(ERROR_MESSAGE, argument),
        "keycolor" => color::set_interface_color(KEY_COMBO, argument),
        "functioncolor" => color::set_interface_color(FUNCTION_TAG, argument),
        "operatingdir" => with_global_mut(|g| g.operating_dir = Some(argument.to_string())),
        "fill" => {
            let mut f: isize = 0;
            if utils::parse_num(argument, &mut f) {
                with_global_mut(|g| g.fill = f);
            } else {
                jot_error(&crate::t!("rcfile-fill_invalid", arg = argument));
                with_global_mut(|g| g.fill = -(COLUMNS_FROM_EOL as isize));
            }
        }
        "matchbrackets" => {
            if chars::has_blank_char(argument.as_bytes()) {
                jot_error(&crate::t!("rcfile-non_blank"));
            } else if chars::mbstrlen(argument.as_bytes()) % 2 != 0 {
                jot_error(&crate::t!("rcfile-even_chars"));
            } else {
                with_global_mut(|g| g.matchbrackets = Some(argument.to_string()));
            }
        }
        "whitespace" => {
            if chars::mbstrlen(argument.as_bytes()) != 2 || utils::breadth(argument.as_bytes()) != 2 {
                jot_error(&crate::t!("rcfile-two_single_col"));
            } else {
                let bytes = argument.as_bytes().to_vec();
                let l0 = chars::char_length(bytes.as_slice());
                let l1 = chars::char_length(&bytes[l0..]);
                with_global_mut(|g| {
                    g.whitespace = Some(bytes);
                    g.whitelen = (l0, l1);
                });
            }
        }
        "punct" => {
            if chars::has_blank_char(argument.as_bytes()) {
                jot_error(&crate::t!("rcfile-non_blank"));
            } else {
                with_global_mut(|g| g.punct = Some(argument.to_string()));
            }
        }
        "brackets" => {
            if chars::has_blank_char(argument.as_bytes()) {
                jot_error(&crate::t!("rcfile-non_blank"));
            } else {
                with_global_mut(|g| g.brackets = Some(argument.to_string()));
            }
        }
        "quotestr" => with_global_mut(|g| g.quotestr = Some(argument.to_string())),
        "speller" => with_global_mut(|g| g.speller = Some(argument.to_string())),
        "backupdir" => with_global_mut(|g| g.backup_dir = Some(argument.to_string())),
        "wordchars" => {
            with_global_mut(|g| g.word_chars = Some(argument.to_string()));
            WORD_CHARS_VALUE.with(|w| *w.borrow_mut() = Some(argument.to_string()));
        }
        "guidestripe" => {
            let mut n: isize = 0;
            if utils::parse_num(argument, &mut n) && n > 0 {
                with_global_mut(|g| g.stripe_column = n as usize);
            } else {
                jot_error(&crate::t!("rcfile-guide_invalid", arg = argument));
                with_global_mut(|g| g.stripe_column = 0);
            }
        }
        "tabsize" => {
            let mut n: isize = 0;
            if utils::parse_num(argument, &mut n) && n > 0 {
                with_global_mut(|g| g.tabsize = n as usize);
                set_tabsize_independent(n as usize);
            } else {
                jot_error(&crate::t!("rcfile-tabsize_invalid", arg = argument));
            }
        }
        _ => {}
    }
}

// ======================== 语法命令（对应 rcfile.c 的 ENABLE_COLOR 部分） ========================

/// 语法存储目标（extensions/headers/magics）。
#[derive(Clone, Copy)]
enum StorageKind {
    Extensions,
    Headers,
    Magics,
}

/// syntax 的字符串字段目标（comment/tabgives/linter/formatter）。
#[derive(Clone, Copy)]
enum CommentTarget {
    Comment,
    Tabstring,
    Linter,
    Formatter,
}

/// 按名字查找已注册的语法。
fn find_syntax_by_name(name: &str) -> Option<SyntaxRef> {
    with_global(|g| {
        let mut cur = g.syntaxes.clone();
        while let Some(s) = cur {
            if s.borrow().name.as_deref() == Some(name) {
                return Some(s.clone());
            }
            let next = { let r = s.borrow(); r.next.clone() };
            cur = next;
        }
        None
    })
}

/// 收集自 before 之后新登记的语法（syntax 链表为头插，新语法在头部）。
fn added_syntaxes(before: &Option<SyntaxRef>) -> Vec<SyntaxRef> {
    let mut out = Vec::new();
    with_global(|g| {
        let mut cur = g.syntaxes.clone();
        while let Some(s) = cur {
            let is_before = before
                .as_ref()
                .map(|b| std::rc::Rc::ptr_eq(b, &s))
                .unwrap_or(false);
            if is_before {
                break;
            }
            out.push(s.clone());
            let next = { let r = s.borrow(); r.next.clone() };
            cur = next;
        }
    });
    out
}

/// 验证当前语法定义非空，并关闭 opensyntax（对应 `check_for_nonempty_syntax`）。
fn check_for_nonempty_syntax() {
    if open_syntax() && !seen_color_command() {
        let current_lineno = get_rcfile_lineno();
        let name = get_live_syntax()
            .and_then(|s| s.borrow().name.clone())
            .unwrap_or_default();
        let ls_lineno = get_live_syntax().map(|s| s.borrow().lineno).unwrap_or(0);

        set_rcfile_lineno(ls_lineno);
        jot_error(&crate::t!("rcfile-no_color_commands", name = name));
        set_rcfile_lineno(current_lineno);
    }

    set_open_syntax(false);
}

/// 开始一个新的 syntax 定义（对应 `begin_new_syntax`）。
fn begin_new_syntax(ptr: &str, filename: &str) {
    let (nameptr, rest) = next_word(ptr);

    /* 检查语法名不为空。 */
    if nameptr.is_empty() || (nameptr.starts_with('"') && (nameptr.len() <= 1 || nameptr.len() == 2)) {
        jot_error(&crate::t!("rcfile-missing_syntax_name"));
        return;
    }

    /* 检查引号配对。 */
    if nameptr.starts_with('"') != nameptr.ends_with('"') {
        jot_error(&crate::t!("rcfile-unpaired_quote"));
        return;
    }

    let name = if nameptr.starts_with('"') {
        &nameptr[1..nameptr.len() - 1]
    } else {
        nameptr
    };

    /* "none" 语法保留。 */
    if name == "none" {
        jot_error(&crate::t!("rcfile-none_reserved"));
        return;
    }

    let syntax = SyntaxType {
        name: Some(name.to_string()),
        filename: Some(filename.to_string()),
        lineno: get_rcfile_lineno(),
        augmentations: None,
        extensions: None,
        headers: None,
        magics: None,
        linter: None,
        formatter: None,
        tabstring: None,
        comment: Some(GENERAL_COMMENT_CHARACTER.to_string()),
        color: None,
        multiscore: 0,
        next: None,
    };

    let new_syntax = Rc::new(RefCell::new(syntax));
    with_global_mut(|g| {
        new_syntax.borrow_mut().next = g.syntaxes.take();
        g.syntaxes = Some(new_syntax.clone());
    });

    set_live_syntax(Some(new_syntax.clone()));
    set_last_color(None);
    set_open_syntax(true);
    set_seen_color_command(false);

    /* default 语法不接受扩展名。 */
    if name == "default" && !rest.is_empty() {
        jot_error(&crate::t!("rcfile-default_no_ext"));
        return;
    }

    /* 若有 extension 正则，收集它们。 */
    if !rest.is_empty() {
        grab_and_store("extension", rest, StorageKind::Extensions);
    }
}

/// 编译正则并追加到 storage 链表（对应 `grab_and_store`）。
fn grab_and_store(kind: &str, ptr: &str, target: StorageKind) {
    if !open_syntax() {
        jot_error(&crate::t!("rcfile-missing_command", kind = kind));
        return;
    }

    let is_default = get_live_syntax()
        .map(|s| s.borrow().name.as_deref() == Some("default"))
        .unwrap_or(false);
    if is_default && !ptr.trim().is_empty() {
        jot_error(&crate::t!("rcfile-default_no_regex", kind = kind));
        return;
    }

    if ptr.trim().is_empty() {
        jot_error(&crate::t!("rcfile-missing_regex", kind = kind));
        return;
    }

    let regexes = parse_regex_list(ptr);
    let Some(live) = get_live_syntax() else { return };

    for rgx_str in regexes {
        let pat = match MatchPattern::from_regex(&rgx_str, false) {
            Ok(p) => p,
            Err(msg) => {
                jot_error(&crate::t!("rcfile-bad_regex", expr = rgx_str, msg = msg));
                return;
            }
        };
        let item = Rc::new(RefCell::new(RegexListType {
            one_rgx: Some(pat),
            next: None,
        }));
        let mut live_ref = live.borrow_mut();
        let storage = match target {
            StorageKind::Extensions => &mut live_ref.extensions,
            StorageKind::Headers => &mut live_ref.headers,
            StorageKind::Magics => &mut live_ref.magics,
        };
        let mut tail = storage.clone();
        let mut prev: Option<RegexListRef> = None;
        while let Some(t) = tail {
            let next = { let r = t.borrow(); r.next.clone() };
            prev = Some(t);
            tail = next;
        }
        match prev {
            Some(p) => p.borrow_mut().next = Some(item.clone()),
            None => *storage = Some(item.clone()),
        }
    }
}

/// 收集 comment/tabgives/linter/formatter 后的字符串（对应 `pick_up_name`）。
fn pick_up_name(kind: &str, ptr: &str, target: CommentTarget) {
    let s = ptr.trim();
    if s.is_empty() {
        jot_error(&crate::t!("rcfile-missing_arg", kind = kind));
        return;
    }
    let val = if s.starts_with('"') {
        match s.rfind('"') {
            Some(idx) if idx > 0 => s[1..idx].to_string(),
            _ => {
                jot_error(&crate::t!("rcfile-missing_quote", kind = kind));
                return;
            }
        }
    } else {
        s.to_string()
    };

    if let Some(live) = get_live_syntax() {
        let mut l = live.borrow_mut();
        match target {
            CommentTarget::Comment => l.comment = Some(val),
            CommentTarget::Tabstring => l.tabstring = Some(val),
            CommentTarget::Linter => l.linter = Some(val.trim_start().to_string()),
            CommentTarget::Formatter => l.formatter = Some(val.trim_start().to_string()),
        }
    }
}

/// 解析 color/icolor 命令：颜色组合 + 一个或多个正则（对应 `parse_rule`）。
fn parse_rule(ptr: &str, icase: bool) {
    if ptr.trim().is_empty() {
        jot_error(&crate::t!("rcfile-missing_color_name"));
        return;
    }

    let (names, rest) = next_word(ptr);

    let (fg, bg, attributes) = match color::parse_combination(names) {
        Some(v) => v,
        None => return,
    };

    if rest.is_empty() {
        jot_error(&crate::t!("rcfile-missing_regex_after", command = "color"));
        return;
    }

    let bytes = rest.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let mut expectend = false;
        if rest[pos..].starts_with("start=") {
            pos += 6;
            expectend = true;
        }

        let (rgx_str, np) = match read_regex(rest, pos) {
            Some(v) => v,
            None => return,
        };
        pos = np;

        let start_rgx = match MatchPattern::from_regex(&rgx_str, icase) {
            Ok(r) => r,
            Err(msg) => {
                jot_error(&crate::t!("rcfile-bad_regex", expr = rgx_str, msg = msg));
                return;
            }
        };

        let mut end_rgx = None;
        if expectend {
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if !rest[pos..].starts_with("end=") {
                jot_error(&crate::t!("rcfile-start_requires_end"));
                return;
            }
            pos += 4;
            let (ergx, np) = match read_regex(rest, pos) {
                Some(v) => v,
                None => return,
            };
            pos = np;
            end_rgx = Some(match MatchPattern::from_regex(&ergx, icase) {
                Ok(r) => r,
                Err(msg) => {
                    jot_error(&crate::t!("rcfile-bad_regex", expr = ergx, msg = msg));
                    return;
                }
            });
        }

        /* 建规则并链接到当前语法的颜色链尾。 */
        let mut color_type = ColorType {
            id: 0,
            fg,
            bg,
            pairnum: 0,
            attributes,
            start: Some(start_rgx),
            end: end_rgx,
            next: None,
        };

        let Some(live) = get_live_syntax() else { return };
        let new_color = {
            if expectend {
                color_type.id = live.borrow().multiscore;
                live.borrow_mut().multiscore += 1;
            }
            Rc::new(RefCell::new(color_type))
        };
        {
            let mut live_ref = live.borrow_mut();
            let last = get_last_color();
            match last {
                None => live_ref.color = Some(new_color.clone()),
                Some(l) => l.borrow_mut().next = Some(new_color.clone()),
            }
        }
        set_last_color(Some(new_color));
        set_seen_color_command(true);
    }
}

/// 处理六个语法专属命令（color/icolor/comment/tabgives/linter/formatter）。
/// 返回 FALSE 表示不是这些命令（对应 `parse_syntax_commands`）。
fn parse_syntax_commands(keyword: &str, ptr: &str) -> bool {
    match keyword {
        "color" => parse_rule(ptr, false),
        "icolor" => parse_rule(ptr, true),
        "comment" => pick_up_name("comment", ptr, CommentTarget::Comment),
        "tabgives" => pick_up_name("tabgives", ptr, CommentTarget::Tabstring),
        "linter" => pick_up_name("linter", ptr, CommentTarget::Linter),
        "formatter" => pick_up_name("formatter", ptr, CommentTarget::Formatter),
        _ => return false,
    }
    true
}

/// 应用一个语法累积的 extendsyntax 命令（对应 parse_one_include 尾部的 extra 循环）。
fn apply_augmentations(sntx: &SyntaxRef) {
    let extras = { let r = sntx.borrow(); r.augmentations.clone() };
    let mut extra = extras;
    while let Some(e) = extra {
        let (data, filename, lno) = {
            let r = e.borrow();
            (r.data.clone(), r.filename.clone(), r.lineno)
        };
        if let Some(f) = filename {
            set_nanorc(Some(f));
        }
        set_rcfile_lineno(lno.max(0) as usize);
        let data = data.unwrap_or_default();
        let (kw, rest) = next_word(&data);
        if !parse_syntax_commands(&kw, rest) {
            jot_error(&crate::t!("rcfile-not_understood", command = kw));
        }
        let next = { let r = e.borrow(); r.next.clone() };
        extra = next;
    }
}

// ======================== bind / unbind（对应 rcfile.c 的 parse_binding） ========================

/// 绑定或解绑一个键（对应 `parse_binding`）。
fn parse_binding(ptr: &str, dobind: bool) {
    check_for_nonempty_syntax();

    if ptr.is_empty() {
        jot_error(&crate::t!("rcfile-missing_key_name"));
        return;
    }

    let (keyptr, rest) = next_word(ptr);
    let mut keycopy = keyptr.to_string();

    /* 大写化 ^ 后的第二个字符或第一个字符。 */
    if keycopy.starts_with('^') {
        if keycopy.len() > 1 {
            let b = keycopy.as_bytes()[1];
            if b.is_ascii_lowercase() {
                keycopy.replace_range(1..2, &((b & 0x5F) as char).to_string());
            }
        }
    } else if let Some(b) = keycopy.as_bytes().first() {
        if b.is_ascii_lowercase() {
            keycopy.replace_range(0..1, &((b & 0x5F) as char).to_string());
        }
    }

    /* 键名不能太短。 */
    let len = keycopy.len();
    if len < 2 || (keycopy.starts_with('M') && len < 3) {
        jot_error(&crate::t!("rcfile-invalid_key_name", name = keycopy));
        return;
    }

    let keycode = global::keycode_from_string(&keycopy);
    if keycode < 0 {
        jot_error(&crate::t!("rcfile-invalid_key_name", name = keycopy));
        return;
    }

    /* 解析要绑定的函数（若为绑定）。C 版 funcptr 指向原始参数（含引号），
     * parse_argument 只负责截断尾部引号并前进指针。 */
    let (funcptr, rest2) = if dobind {
        match parse_argument(rest) {
            Some(v) => v,
            None => return,
        }
    } else {
        ("", rest)
    };
    let funcptr_is_quoted = rest.starts_with('"');

    if dobind && funcptr.is_empty() {
        jot_error(&crate::t!("rcfile-must_specify_function"));
        return;
    }

    let (menuptr, _rest3) = next_word(rest2);
    if menuptr.is_empty() {
        jot_error(&crate::t!("rcfile-missing_menu"));
        return;
    }

    let mut menu = global::name_to_menu(menuptr);
    if menu == 0 {
        jot_error(&crate::t!("rcfile-unknown_menu", name = menuptr));
        return;
    }

    let mut new_func = FunctionId::DoNothing;
    let mut new_toggle: i32 = 0;
    let mut new_expansion: Option<String> = None;

    if dobind {
        /* 以双引号开头的是字符串（植入），否则是函数名。 */
        if funcptr_is_quoted {
            new_func = FunctionId::Implant;
            new_expansion = Some(funcptr.to_string());
        } else {
            match global::strtosc(funcptr) {
                Some((f, tg)) => {
                    new_func = f;
                    new_toggle = tg;
                }
                None => {
                    jot_error(&crate::t!("rcfile-unknown_function", name = funcptr));
                    return;
                }
            }
        }
    }

    /* 从给定菜单清除同键码的旧绑定。 */
    with_global_mut(|g| {
        let mut cur = g.shortcuts.clone();
        while let Some(s) = cur {
            let next = { let r = s.borrow(); r.next.clone() };
            let mut sr = s.borrow_mut();
            if (sr.menus & menu) != 0 && sr.keycode == keycode {
                sr.menus &= !menu;
            }
            drop(sr);
            cur = next;
        }
    });

    /* 解绑：结束（记录到 unbound_keys 供按键分发）。 */
    if !dobind {
        with_global_mut(|g| {
            g.bound_keys.retain(|b| b.keycode != keycode || (b.menus & menu) == 0);
            g.unbound_keys.push((keycode, menu));
        });
        return;
    }

    /* 把菜单限制到函数确实存在的那些。 */
    let limited = if global::is_universal(new_func) {
        menu & (MMOST | MBROWSER)
    } else if new_func == FunctionId::DoToggle && new_toggle == NO_HELP as i32 {
        menu & ((MMOST | MBROWSER | MYESNO) & !MFINDINHELP)
    } else if new_func == FunctionId::DoToggle {
        menu & MMAIN
    } else if new_func == FunctionId::DoFullRefresh {
        menu & (MMOST | MBROWSER | MHELP | MYESNO)
    } else if new_func == FunctionId::Implant {
        menu & (MMOST | MBROWSER | MHELP)
    } else {
        let mut mask = 0;
        for f in global::iter_funcs() {
            let (ff, fm) = { let r = f.borrow(); (r.func, r.menus) };
            if ff == new_func {
                mask |= fm;
            }
        }
        menu & mask
    };

    if limited == 0 {
        if !ISSET(RESTRICTED) && !ISSET(VIEW_MODE) {
            jot_error(&crate::t!("rcfile-function_not_in_menu", func = funcptr, menu = menuptr));
        }
        return;
    }
    menu = limited;

    /* 禁止重绑 <Esc>。 */
    if keycode == ESC_CODE as i32 {
        jot_error(&crate::t!("rcfile-no_rebind", key = keycopy));
        return;
    }

    /* 若是 toggle，寻找并复制其序号。 */
    let mut ordinal: i32 = 0;
    if new_func == FunctionId::DoToggle {
        with_global(|g| {
            let mut cur = g.shortcuts.clone();
            while let Some(s) = cur {
                let sr = s.borrow();
                if sr.func == FunctionId::DoToggle && sr.toggle == new_toggle {
                    ordinal = sr.ordinal;
                    break;
                }
                let next = sr.next.clone();
                drop(sr);
                cur = next;
            }
        });
    }

    /* 头插到快捷键列表，并登记用户绑定。 */
    with_global_mut(|g| {
        let new_key = Rc::new(RefCell::new(KeyStruct {
            keystr: keycopy.clone(),
            keycode,
            menus: menu,
            func: new_func,
            toggle: new_toggle,
            ordinal,
            expansion: new_expansion.clone(),
            next: g.shortcuts.take(),
        }));
        g.shortcuts = Some(new_key);
        g.bound_keys.push(BoundKey {
            keystr: keycopy.clone(),
            keycode,
            menus: menu,
            func: new_func,
            toggle: new_toggle,
            ordinal,
            expansion: new_expansion,
        });
        g.unbound_keys.retain(|(k, _)| *k != keycode);
    });
}

// ======================== 文件检查与 glob（对应 rcfile.c） ========================

/// 检查文件存在、可读且非目录/设备（对应 `is_good_file`）。
#[cfg(unix)]
fn is_good_file(file: &str) -> bool {
    use std::ffi::CString;
    let Ok(c) = CString::new(file) else {
        return false;
    };
    let acc = unsafe { libc::access(c.as_ptr(), libc::R_OK) };
    if acc < 0 {
        return false;
    }
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let stat_ok = unsafe { libc::stat(c.as_ptr(), &mut st) };
    if stat_ok == 0 {
        let mode = st.st_mode & libc::S_IFMT;
        if mode == libc::S_IFDIR {
            jot_error(&crate::t!("rcfile-is_directory", file = file));
            return false;
        }
        if mode == libc::S_IFCHR || mode == libc::S_IFBLK {
            jot_error(&crate::t!("rcfile-is_device", file = file));
            return false;
        }
    }
    true
}

/// 检查文件存在、可读且非目录（非 Unix 简化版）。
#[cfg(not(unix))]
fn is_good_file(file: &str) -> bool {
    let p = Path::new(file);
    if !p.exists() {
        return false;
    }
    if p.is_dir() {
        jot_error(&crate::t!("rcfile-is_directory", file = file));
        return false;
    }
    true
}

/// 文件是否可读（对应 access(file, R_OK)）。
#[cfg(unix)]
fn access_ok(file: &str) -> bool {
    use std::ffi::CString;
    let Ok(c) = CString::new(file) else {
        return false;
    };
    let ok = unsafe { libc::access(c.as_ptr(), libc::R_OK) };
    ok == 0
}

#[cfg(not(unix))]
fn access_ok(file: &str) -> bool {
    Path::new(file).exists()
}

/// glob 单段模式匹配（*、?、[...]；对应 glob() 中单层目录匹配）。
fn glob_match(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.first() {
        None => text.is_empty(),
        Some(b'*') => {
            let mut i = 0;
            while i <= text.len() {
                if glob_match(&pattern[1..], &text[i..]) {
                    return true;
                }
                i += 1;
            }
            false
        }
        Some(b'?') => !text.is_empty() && glob_match(&pattern[1..], &text[1..]),
        Some(b'[') => {
            if text.is_empty() {
                return false;
            }
            let mut j = 1;
            let mut negated = false;
            if pattern.get(j) == Some(&b'!') || pattern.get(j) == Some(&b'^') {
                negated = true;
                j += 1;
            }
            let mut matched = false;
            let mut first = true;
            loop {
                if j >= pattern.len() {
                    /* 未闭合的 [ 按字面处理。 */
                    return text.first() == Some(&b'[') && glob_match(&pattern[1..], &text[1..]);
                }
                if pattern[j] == b']' && !first {
                    break;
                }
                first = false;
                if j + 2 < pattern.len() && pattern[j + 1] == b'-' && pattern[j + 2] != b']' {
                    if pattern[j] <= text[0] && text[0] <= pattern[j + 2] {
                        matched = true;
                    }
                    j += 3;
                } else {
                    if pattern[j] == text[0] {
                        matched = true;
                    }
                    j += 1;
                }
            }
            if negated {
                matched = !matched;
            }
            matched && glob_match(&pattern[j + 1..], &text[1..])
        }
        Some(c) => text.first() == Some(c) && glob_match(&pattern[1..], &text[1..]),
    }
}

fn join_path(prefix: &str, name: &str) -> String {
    match (prefix, name) {
        ("", n) => n.to_string(),
        ("/", n) => format!("/{}", n),
        (p, n) => format!("{}/{}", p, n),
    }
}

/// 递归展开 glob 目录段。
fn expand_glob_dir(prefix: &str, parts: &[&str], out: &mut Vec<String>) {
    let Some((head, tail)) = parts.split_first() else {
        out.push(prefix.to_string());
        return;
    };
    if !head.contains(['*', '?', '[']) {
        let next = join_path(prefix, head);
        expand_glob_dir(&next, tail, out);
        return;
    }
    let dir = if prefix.is_empty() { "." } else { prefix };
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if glob_match(head.as_bytes(), name.as_bytes()) {
                let next = join_path(prefix, &name);
                expand_glob_dir(&next, tail, out);
            }
        }
    }
}

/// 展开 glob 模式；无匹配时返回原模式（对应 GLOB_NOCHECK 语义）。
fn expand_glob(pattern: &str) -> Vec<String> {
    if !pattern.contains(['*', '?', '[']) {
        return vec![pattern.to_string()];
    }
    let mut out = Vec::new();
    let (prefix, body) = match pattern.strip_prefix('/') {
        Some(rest) => ("/", rest),
        None => ("", pattern),
    };
    let parts: Vec<&str> = body.split('/').collect();
    expand_glob_dir(prefix, &parts, &mut out);
    if out.is_empty() {
        out.push(pattern.to_string());
    }
    out
}

// ======================== include 处理（对应 rcfile.c 的 parse_includes / parse_one_include） ========================

/// 展开 include 参数中的 glob 并逐个解析（对应 `parse_includes`）。
fn parse_includes(ptr: &str) {
    check_for_nonempty_syntax();

    let trimmed = ptr.trim_start_matches([' ', '\t']);
    let pattern = if trimmed.starts_with('"') {
        match parse_argument(trimmed) {
            Some((p, _)) => p,
            None => return,
        }
    } else {
        let (p, _) = next_word(trimmed);
        p
    };

    if pattern.len() > PATH_MAX {
        jot_error(&crate::t!("rcfile-path_too_long"));
        return;
    }

    let expanded = files::expand_leading_tilde(pattern);
    let matches = expand_glob(&expanded);
    for f in matches {
        parse_one_include(&f, None);
    }
}

/// 部分解析 include 文件（syntax 为 None 时只收 intro），或完整解析一个语法
/// （对应 `parse_one_include`）。
fn parse_one_include(file: &str, syntax: Option<SyntaxRef>) {
    /* 目录/设备文件：is_good_file 已报错，直接返回。 */
    if access_ok(file) && !is_good_file(file) {
        return;
    }

    let Ok(stream) = File::open(file) else {
        jot_error(&crate::t!(
            "rcfile-error_reading",
            file = file,
            err = std::io::Error::last_os_error().to_string()
        ));
        return;
    };

    let was_nanorc = get_nanorc();
    let was_lineno = get_rcfile_lineno();
    set_nanorc(Some(file.to_string()));
    set_rcfile_lineno(0);

    match syntax {
        None => {
            let mut reader = BufReader::new(stream);
            parse_stream(&mut reader, true, true);
        }
        Some(sntx) => {
            set_live_syntax(Some(sntx.clone()));
            set_last_color(None);
            let mut reader = BufReader::new(stream);
            parse_stream(&mut reader, true, false);

            /* 应用存储的 extendsyntax 命令。 */
            apply_augmentations(&sntx);

            /* 标记该语法已加载。 */
            sntx.borrow_mut().filename = None;
        }
    }

    set_nanorc(was_nanorc);
    set_rcfile_lineno(was_lineno);
}

// ======================== 主解析循环（对应 rcfile.c 的 parse_rcfile） ========================

/// 处理一行 rc 配置（parse_rcfile 主循环的主体）。返回 true 表示应停止解析。
/// had_invalid 表示原始行含非法 UTF-8 字节（对应 C 版的 mbstowcs 校验前提）。
fn handle_rcfile_line(line: &str, just_syntax: bool, intros_only: bool, had_invalid: bool) -> bool {
    let line = line.trim_start_matches([' ', '\t']);
    if line.is_empty() || line.starts_with('#') {
        return false;
    }

    let (mut keyword, mut rest) = next_word(line);
    let mut drop_open = false;
    let mut set = 0i8;

    /* 先处理 extendsyntax... */
    if !just_syntax && keyword == "extendsyntax" {
        check_for_nonempty_syntax();

        let (syntaxname, rest2) = next_word(rest);
        let sntx = find_syntax_by_name(syntaxname);

        let Some(sntx) = sntx else {
            jot_error(&crate::t!("rcfile-could_not_find", name = syntaxname));
            return false;
        };

        let (kw2, argument) = next_word(rest2);

        /* 文件匹配命令立即处理（keyword 改为该命令并落入下方分支）；
         * 其余命令存入 augmentations 供以后应用。 */
        if kw2 == "header" || kw2 == "magic" {
            set_live_syntax(Some(sntx.clone()));
            set_open_syntax(true);
            drop_open = true;
            keyword = kw2;
            rest = argument;
        } else {
            let newitem = Rc::new(RefCell::new(AugmentStruct {
                filename: get_nanorc(),
                lineno: get_rcfile_lineno() as isize,
                data: Some(argument.to_string()),
                next: None,
            }));
            let mut l = sntx.borrow_mut();
            match &mut l.augmentations {
                None => l.augmentations = Some(newitem),
                Some(head) => {
                    let mut cur = head.clone();
                    while cur.borrow().next.is_some() {
                        let nx = cur.borrow().next.clone().unwrap();
                        cur = nx;
                    }
                    cur.borrow_mut().next = Some(newitem);
                }
            }
            return false;
        }
    }

    /* 尝试解析关键字。 */
    if keyword == "syntax" {
        if intros_only {
            check_for_nonempty_syntax();
            begin_new_syntax(rest, &get_nanorc().unwrap_or_default());
        } else {
            return true;
        }
    } else if keyword == "header" {
        if intros_only {
            grab_and_store("header", rest, StorageKind::Headers);
        }
    } else if keyword == "magic" {
        if intros_only {
            grab_and_store("magic", rest, StorageKind::Magics);
        }
    } else if just_syntax
        && matches!(keyword, "set" | "unset" | "bind" | "unbind" | "include" | "extendsyntax")
    {
        if intros_only {
            jot_error(&crate::t!("rcfile-not_allowed_include", command = keyword));
        } else {
            return true;
        }
    } else if intros_only
        && matches!(keyword, "color" | "icolor" | "comment" | "tabgives" | "linter" | "formatter")
    {
        if !open_syntax() {
            jot_error(&crate::t!("rcfile-requires_preceding", kind = keyword));
        }
        if keyword == "color" || keyword == "icolor" {
            set_seen_color_command(true);
        }
        return false;
    } else if parse_syntax_commands(keyword, rest) {
        /* 已处理。 */
    } else if keyword == "include" {
        parse_includes(rest);
    } else {
        if keyword == "set" {
            set = 1;
        } else if keyword == "unset" {
            set = -1;
        } else if keyword == "bind" {
            parse_binding(rest, true);
        } else if keyword == "unbind" {
            parse_binding(rest, false);
        } else if intros_only {
            jot_error(&crate::t!("rcfile-not_understood", command = keyword));
        }
    }

    /* 分派链之后统一处理（对应 C 版 drop_open / set==0 检查）。 */
    if drop_open {
        set_open_syntax(false);
    }
    if set == 0 {
        return false;
    }

    check_for_nonempty_syntax();

    if rest.is_empty() {
        jot_error(&crate::t!("rcfile-missing_option"));
        return false;
    }

    let (option, rest2) = next_word(rest);
    let rcopt = RCOPTS.iter().find(|o| o.name == option);
    let Some(rcopt) = rcopt else {
        jot_error(&crate::t!("rcfile-unknown_option", option = option));
        return false;
    };

    /* 开关选项：设置或取消。 */
    if rcopt.flag != 0 {
        if set == 1 {
            SET(rcopt.flag);
        } else {
            UNSET(rcopt.flag);
        }
        return false;
    }

    /* 带参选项不能被 unset。 */
    if set == -1 {
        jot_error(&crate::t!("rcfile-cannot_unset", option = option));
        return false;
    }

    if rest2.is_empty() {
        jot_error(&crate::t!("rcfile-requires_argument", option = option));
        return false;
    }

    /* 解析参数（去引号）。 */
    let Some((argument, _)) = parse_argument(rest2) else {
        return false;
    };

    /* UTF-8 模式下忽略含非法序列的参数（对应 C 版 mbstowcs 校验）。 */
    if had_invalid && with_global(|g| g.using_utf8) {
        jot_error(&crate::t!("rcfile-invalid_multibyte"));
        return false;
    }

    parse_valued_option(option, argument);

    false
}

/// 从流中解析（对应 parse_rcfile 主循环）。just_syntax 时允许文件只含
/// 颜色语法命令；intros_only 时只收集语法骨架（syntax/header/magic）。
fn parse_stream<R: BufRead>(reader: &mut R, just_syntax: bool, intros_only: bool) {
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        let lineno = get_rcfile_lineno() + 1;
        set_rcfile_lineno(lineno);

        /* 完整解析时，跳过语法定义行之前的内容。 */
        if just_syntax && !intros_only {
            let ls_lineno = get_live_syntax().map(|s| s.borrow().lineno).unwrap_or(0);
            if lineno <= ls_lineno {
                continue;
            }
        }

        /* 去掉结尾换行和可能的回车（字节级，对应 C 版 getline 处理）。 */
        while buffer.last() == Some(&b'\n') || buffer.last() == Some(&b'\r') {
            buffer.pop();
        }
        let had_invalid = std::str::from_utf8(&buffer).is_err();
        let line = String::from_utf8_lossy(&buffer);

        if handle_rcfile_line(&line, just_syntax, intros_only, had_invalid) {
            break;
        }
    }

    if intros_only {
        check_for_nonempty_syntax();
    }
    set_rcfile_lineno(0);
}

/// 打开文件并从流解析（管理 nanorc/lineno 的保存与恢复）。
fn parse_stream_file(filename: &str, just_syntax: bool, intros_only: bool) -> bool {
    let Ok(file) = File::open(filename) else {
        jot_error(&crate::t!(
            "rcfile-error_reading",
            file = filename,
            err = std::io::Error::last_os_error().to_string()
        ));
        return false;
    };
    let was_nanorc = get_nanorc();
    let was_lineno = get_rcfile_lineno();
    set_nanorc(Some(filename.to_string()));
    set_rcfile_lineno(0);

    let mut reader = BufReader::new(file);
    parse_stream(&mut reader, just_syntax, intros_only);

    set_nanorc(was_nanorc);
    set_rcfile_lineno(was_lineno);
    true
}

// ======================== 公开 API ========================

/// 解析 nanorc 文件（全量：先 intro 扫描，再逐个完整解析语法规则）。
/// 返回是否成功打开文件。
pub fn parse_rcfile(filename: &str) -> bool {
    let before = with_global(|g| g.syntaxes.clone());
    if !parse_stream_file(filename, false, true) {
        return false;
    }

    /* 对本次解析中登记的各语法做第二遍完整解析（对应 parse_one_include：
     * 用语法定义所在文件解析其规则）。 */
    let added = added_syntaxes(&before);
    for s in added {
        let fname = { let r = s.borrow(); r.filename.clone() };
        let Some(fname) = fname else { continue };
        set_live_syntax(Some(s.clone()));
        set_last_color(None);
        parse_stream_file(&fname, true, false);
        apply_augmentations(&s);
        s.borrow_mut().filename = None;
    }
    true
}

/// 解析 nanorc 文件的一行（单行兼容接口，测试使用）。
pub fn parse_rcfile_line(line: &str, filename: &str, lineno: usize) {
    let was_nanorc = get_nanorc();
    let was_lineno = get_rcfile_lineno();
    set_nanorc(Some(filename.to_string()));
    set_rcfile_lineno(lineno);

    /* 单行模式下语法专属命令立即解析（跳过 intro 扫描的拦截）。 */
    let trimmed = line.trim_start_matches([' ', '\t']);
    if !trimmed.is_empty() && !trimmed.starts_with('#') {
        let (kw, rest) = next_word(trimmed);
        if matches!(kw, "color" | "icolor" | "comment" | "tabgives" | "linter" | "formatter") {
            parse_syntax_commands(kw, rest);
        } else {
            handle_rcfile_line(line, false, true, false);
        }
    }

    set_nanorc(was_nanorc);
    set_rcfile_lineno(was_lineno);
}

/// 解析一个 nanorc 文件（打开已由 have_nanorc 确认的文件）。
fn parse_one_nanorc() {
    let name = get_nanorc();
    if let Some(name) = name {
        parse_rcfile(&name);
    }
}

/// 检查 path 下的 nanorc 候选：存在且合法则设置 nanorc 并返回 TRUE
/// （对应 `have_nanorc`）。
fn have_nanorc(path: &str) -> bool {
    set_nanorc(Some(path.to_string()));
    is_good_file(path)
}

/// 验证关键函数（Exit/Cancel）仍有键绑定（对应 `check_vitals_mapped`）。
fn check_vitals_mapped() {
    const VITALS: [FunctionId; 4] = [
        FunctionId::DoExit,
        FunctionId::DoExit,
        FunctionId::DoExit,
        FunctionId::DoCancel,
    ];
    const INMENUS: [i32; 4] = [MMAIN, MBROWSER, MHELP, MYESNO];

    for v in 0..4 {
        for f in global::iter_funcs() {
            let (ffunc, fmenus, ftag) = {
                let r = f.borrow();
                (r.func, r.menus, r.tag.clone())
            };
            if ffunc == VITALS[v] && (fmenus & INMENUS[v]) != 0 {
                if global::first_sc_for(INMENUS[v], ffunc).is_none() {
                    jot_error(&crate::t!(
                        "rcfile-no_key_bound",
                        func = ftag,
                        menu = global::menu_to_name(INMENUS[v])
                    ));
                    global::die(&crate::t!("rcfile-die_hint"));
                } else {
                    break;
                }
            }
        }
    }
}

/// 读取并处理 rc 文件（对应 `do_rcfiles`）。
pub fn do_rcfiles() {
    let custom = with_global(|g| g.custom_nanorc.clone());

    if let Some(custom) = custom {
        let full = files::get_full_path(&custom).unwrap_or(custom);
        if !Path::new(&full).exists() {
            global::die(&crate::t!("rcfile-specified_not_exist"));
        }
        if is_good_file(&full) {
            set_nanorc(Some(full));
            parse_one_nanorc();
        }
    } else {
        /* 系统级 nanorc（对应 C 版 SYSCONFDIR/nanorc，编译期确定）。 */
        let sys_rc = format!("{}/nanorc", env!("SYSCONFDIR"));
        if have_nanorc(&sys_rc) {
            parse_one_nanorc();
        }

        utils::get_homedir();
        let homedir = with_global(|g| g.homedir.clone());
        let xdgconfdir = std::env::var("XDG_CONFIG_HOME").ok();

        /* 依次尝试用户级候选：~/.nanorc、$XDG_CONFIG_HOME/nano/nanorc、
         * ~/.config/nano/nanorc，取第一个存在且合法者。 */
        let mut found = false;
        if let Some(h) = &homedir {
            if have_nanorc(&format!("{}/.nanorc", h)) {
                found = true;
            }
        }
        if !found {
            if let Some(x) = &xdgconfdir {
                if have_nanorc(&format!("{}/nano/nanorc", x)) {
                    found = true;
                }
            }
        }
        if !found {
            if let Some(h) = &homedir {
                if have_nanorc(&format!("{}/.config/nano/nanorc", h)) {
                    found = true;
                }
            }
        }
        if !found && homedir.is_none() && xdgconfdir.is_none() {
            jot_error(&crate::t!("rcfile-no_home"));
        }
    }

    check_vitals_mapped();

    set_nanorc(None);
}

/// 获取语法列表。
pub fn get_syntaxes() -> Option<SyntaxRef> {
    with_global(|g| g.syntaxes.clone())
}

/// 获取当前缓冲区的注释字符串。
pub fn get_comment_string() -> Option<String> {
    with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow()
                .syntax
                .clone()
                .and_then(|s| s.borrow().comment.clone())
        })
    })
}

// ======================== 错误消息（对应 rcfile.c 的 jot_error） ========================

/// 将给定错误消息存入链表，待退出时打印（对应 `jot_error`）。
pub fn jot_error(msg: &str) {
    let tail = ERRORS_TAIL.with(|t| t.borrow().clone());
    let error = match &tail {
        Some(r) => make_new_node(Some(&*r.borrow())),
        None => make_new_node(None),
    };

    if ERRORS_HEAD.with(|h| h.borrow().is_none()) {
        ERRORS_HEAD.with(|h| *h.borrow_mut() = Some(error.clone()));
    } else if let Some(t) = tail {
        t.borrow_mut().next = Some(error.clone());
    }
    ERRORS_TAIL.with(|t| *t.borrow_mut() = Some(error.clone()));

    /* 首次出错时记录 startup_problem 的概要。 */
    with_global_mut(|g| {
        if g.startup_problem.is_none() {
            let nanorc = get_nanorc();
            g.startup_problem = Some(match nanorc {
                Some(nr) => crate::t!("rcfile-mistakes_in", name = nr),
                None => "Problems with history file".to_string(),
            });
        }
    });

    /* 拼装完整错误文本。 */
    let lineno = get_rcfile_lineno();
    let nanorc = get_nanorc();
    let textbuf = if lineno > 0 {
        match &nanorc {
            Some(nr) => crate::t!("rcfile-error_in", file = nr, line = lineno.to_string(), msg = msg),
            None => msg.to_string(),
        }
    } else {
        msg.to_string()
    };

    error.borrow_mut().data = textbuf;
}

/// 打印累积的错误消息到 stderr（对应 rcfile.c 的 `display_rcfile_errors`）。
pub fn print_errors() {
    let mut item = ERRORS_HEAD.with(|h| h.borrow().clone());
    while let Some(e) = item {
        eprintln!("{}", e.borrow().data);
        let next = { let r = e.borrow(); r.next.clone() };
        item = next;
    }
}
