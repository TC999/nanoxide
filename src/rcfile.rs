/**************************************************************************
 * rcfile.rs  --  GNU nano 配置文件解析（对应 rcfile.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! nanorc 配置文件解析。对应原版 nano 的 `rcfile.c`。
//! 转换说明：使用 `MatchPattern` 替代 POSIX regex。

use crate::definitions::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::color;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

/// 解析颜色名称。
pub fn strtosc(color_name: &str) -> i16 {
    color::color_name_to_number(color_name)
}

/// 读取并处理 rc 文件。
pub fn do_rcfiles() {
    // 读取系统级和用户级 nanorc 文件
    let paths = [
        "/etc/nanorc",
        "/usr/local/etc/nanorc",
    ];

    for path in &paths {
        if Path::new(path).exists() {
            parse_rcfile(path);
        }
    }

    // 读取用户级 rc 文件
    let user_rc = with_global(|g| g.homedir.clone().map(|h| format!("{}/.nanorc", h)));
    if let Some(urc) = user_rc {
        if Path::new(&urc).exists() {
            parse_rcfile(&urc);
        }
    }
}

/// 解析 nanorc 文件。
pub fn parse_rcfile(filename: &str) -> bool {
    match File::open(filename) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for (lineno, line) in reader.lines().enumerate() {
                if let Ok(line) = line {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    parse_rcfile_line(line, filename, lineno + 1);
                }
            }
            true
        }
        Err(e) => {
            eprintln!("Error reading {}: {}", filename, e);
            false
        }
    }
}

/// 解析 nanorc 文件的一行。
pub fn parse_rcfile_line(line: &str, filename: &str, _lineno: usize) {
    let (command, rest) = next_word(line);
    if command.is_empty() {
        return;
    }

    match command.to_lowercase().as_str() {
        "set" => parse_set_command(rest),
        "unset" => parse_unset_command(rest),
        "syntax" => begin_new_syntax(rest, filename),
        "color" => parse_rule(rest, false),
        "icolor" => parse_rule(rest, true),
        "header" => grab_and_store("header", rest, StorageKind::Headers),
        "magic" => grab_and_store("magic", rest, StorageKind::Magics),
        "comment" => pick_up_name("comment", rest, CommentTarget::Comment),
        "tabgives" => pick_up_name("tabgives", rest, CommentTarget::Tabstring),
        "linter" => pick_up_name("linter", rest, CommentTarget::Linter),
        "formatter" => pick_up_name("formatter", rest, CommentTarget::Formatter),
        "extendsyntax" => parse_extendsyntax(rest),
        "include" => parse_include_command(rest),
        "bind" | "unbind" => {} // 简化
        _ => {}
    }
}

/// 解析 set 命令。
fn parse_set_command(args: &str) {
    let option = args.trim().to_lowercase();
    match option.as_str() {
        "autoindent" => SET(AUTOINDENT),
        "backup" => SET(MAKE_BACKUP),
        "boldtext" => SET(BOLD_TEXT),
        "casesensitive" => SET(CASE_SENSITIVE),
        "constantshow" => SET(CONSTANT_SHOW),
        "cutfromcursor" => SET(CUT_FROM_CURSOR),
        "historylog" => SET(HISTORYLOG),
        "linenumbers" => SET(LINE_NUMBERS),
        "locking" => SET(LOCKING),
        "minibar" => SET(MINIBAR),
        "mouse" => SET(USE_MOUSE),
        "multibuffer" => SET(NEW_BUFFER),
        "noconvert" => SET(NO_CONVERT),
        "nohelp" => SET(NO_HELP),
        "nonewlines" => SET(NO_NEWLINES),
        "nowrap" => SET(NO_WRAP),
        "positionlog" => SET(POSITIONLOG),
        "preserve" => SET(PRESERVE),
        "quickblank" => SET(QUICK_BLANK),
        "rebinddelete" => SET(REBIND_DELETE),
        "restricted" => SET(RESTRICTED),
        "saveonexit" => SET(SAVE_ON_EXIT),
        "showcursor" => SET(SHOW_CURSOR),
        "smarthome" => SET(SMART_HOME),
        "softwrap" => SET(SOFTWRAP),
        "tabstospaces" => SET(TABS_TO_SPACES),
        "trimblanks" => SET(TRIM_BLANKS),
        "view" => SET(VIEW_MODE),
        "whitespacedisplay" => SET(WHITESPACE_DISPLAY),
        "wordbounds" => SET(WORD_BOUNDS),
        "zero" => SET(ZERO),
        "modernbindings" => SET(MODERN_BINDINGS),
        "solo" => SET(SOLO_SIDESCROLL),
        "jumpyscrolling" => SET(JUMPY_SCROLLING),
        "emptyline" => SET(EMPTY_LINE),
        "indicator" => SET(INDICATOR),
        "bookstyle" => SET(BOOKSTYLE),
        "colonparsing" => SET(COLON_PARSING),
        "stateflags" => SET(STATEFLAGS),
        "usemagic" => SET(USE_MAGIC),
        "afterends" => SET(AFTER_ENDS),
        "atblanks" => SET(AT_BLANKS),
        "breaklonglines" => SET(BREAK_LONG_LINES),
        "letthemzap" => SET(LET_THEM_ZAP),
        "noread" => SET(NOREAD_MODE),
        "makeitunix" => SET(MAKE_IT_UNIX),
        "insecurebackup" => SET(INSECURE_BACKUP),
        _ => {
            // 处理带值的选项
            let val_parts: Vec<&str> = option.splitn(2, |c: char| c == ' ' || c == '=').collect();
            if val_parts.len() == 2 {
                let val = val_parts[1].trim().trim_matches('"');
                match val_parts[0] {
                    "tabsize" => {
                        if let Ok(s) = val.parse::<usize>() {
                            with_global_mut(|g| g.tabsize = s);
        set_tabsize_independent(s);
                        }
                    }
                    "fill" => {
                        if let Ok(f) = val.parse::<isize>() {
                            with_global_mut(|g| g.fill = f);
                        }
                    }
                    "whitespace" => {} // 简化
                    "matchbrackets" => {
                        with_global_mut(|g| g.matchbrackets = Some(val.to_string()));
                    }
                    "titlecolor" => {} // 简化
                    "numbercolor" => {}
                    "selectedcolor" => {}
                    "stripecolor" => {}
                    "scrollbarcor" => {}
                    "textcolor" => {}
                    "promptcolor" => {}
                    "statuscolor" => {}
                    "errorcolor" => {}
                    "spotlightcolor" => {}
                    _ => {}
                }
            }
        }
    }
}

/// 解析 unset 命令。
fn parse_unset_command(args: &str) {
    let option = args.trim().to_lowercase();
    match option.as_str() {
        "autoindent" => UNSET(AUTOINDENT),
        "cutfromcursor" => UNSET(CUT_FROM_CURSOR),
        "linenumbers" => UNSET(LINE_NUMBERS),
        "mouse" => UNSET(USE_MOUSE),
        "nohelp" => UNSET(NO_HELP),
        "softwrap" => UNSET(SOFTWRAP),
        "tabstospaces" => UNSET(TABS_TO_SPACES),
        "view" => UNSET(VIEW_MODE),
        "whitespacedisplay" => UNSET(WHITESPACE_DISPLAY),
        _ => {}
    }
}

// ======================== 语法命令解析（对应 rcfile.c） ========================

/// syntax 解析的会话状态（对应 rcfile.c 的 static 变量）。
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
fn set_open_syntax(v: bool) {
    OPEN_SYNTAX.with(|x| x.set(v));
}
fn set_seen_color_command(v: bool) {
    SEEN_COLOR_COMMAND.with(|x| x.set(v));
}
fn seen_color_command() -> bool {
    SEEN_COLOR_COMMAND.with(|x| x.get())
}

/// 从字符串开头取第一个词：返回 (词, 剩余部分)。
fn next_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    let end = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
    (&s[..end], s[end..].trim_start())
}

/// 解析由 `"` 包裹的一个正则串，返回 (内容, 后续位置)。
/// 引号后必须是空白或行尾（对应 `parse_next_regex`）。
fn read_regex(rest: &str, pos: usize) -> Option<(String, usize)> {
    let bytes = rest.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'"' {
        jot_error("Regex strings must begin and end with a \" character");
        return None;
    }
    let start = pos + 1;
    let mut i = start;
    loop {
        if i >= bytes.len() {
            jot_error("Regex strings must begin and end with a \" character");
            return None;
        }
        if bytes[i] == b'"' && (i + 1 >= bytes.len() || bytes[i + 1].is_ascii_whitespace()) {
            break;
        }
        i += 1;
    }
    if i == start {
        jot_error("Empty regex string");
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

/// 开始一个新的 syntax 定义（对应 `begin_new_syntax`）。
fn begin_new_syntax(ptr: &str, filename: &str) {
    let (nameptr, rest) = next_word(ptr);

    /* 检查语法名不为空。 */
    if nameptr.is_empty() || (nameptr.starts_with('"') && (nameptr.len() <= 1 || nameptr.len() == 2)) {
        jot_error("Missing syntax name");
        return;
    }

    /* 检查引号配对。 */
    if nameptr.starts_with('"') != nameptr.ends_with('"') {
        jot_error("Unpaired quote in syntax name");
        return;
    }

    let name = if nameptr.starts_with('"') {
        &nameptr[1..nameptr.len() - 1]
    } else {
        nameptr
    };

    /* "none" 语法保留。 */
    if name == "none" {
        jot_error("The \"none\" syntax is reserved");
        return;
    }

    let syntax = SyntaxType {
        name: Some(name.to_string()),
        filename: Some(filename.to_string()),
        lineno: 0,
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
        jot_error("The \"default\" syntax does not accept extensions");
        return;
    }

    /* 若有 extension 正则，收集它们。 */
    if !rest.is_empty() {
        grab_and_store("extension", rest, StorageKind::Extensions);
    }
}

/// 编译正则并追加到 storage 链表（对应 `grab_and_store`）。
fn grab_and_store(kind: &str, ptr: &str, target: StorageKind) {
    if !OPEN_SYNTAX.with(|x| x.get()) {
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

/// syntax 的字符串字段目标（comment/tabgives/linter/formatter）。
#[derive(Clone, Copy)]
enum CommentTarget {
    Comment,
    Tabstring,
    Linter,
    Formatter,
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
        jot_error("Missing color name");
        return;
    }

    let (names, rest) = next_word(ptr);

    let (fg, bg, attributes) = match color::parse_combination(names) {
        Some(v) => v,
        None => return,
    };

    if rest.is_empty() {
        jot_error("Missing regex string after 'color' command");
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
                jot_error("\"start=\" requires a corresponding \"end=\"");
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
                    jot_error(&format!("Bad regex \"{}\": {}", ergx, msg));
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

/// 处理 extendsyntax 命令（对应 rcfile.c 的 extendsyntax 分支）。
fn parse_extendsyntax(ptr: &str) {
    let (syntaxname, rest) = next_word(ptr);
    if syntaxname.is_empty() {
        return;
    }

    let found = with_global(|g| {
        let mut cur = g.syntaxes.clone();
        while let Some(s) = cur {
            if s.borrow().name.as_deref() == Some(syntaxname) {
                return Some(s);
            }
            let next = { let r = s.borrow(); r.next.clone() };
            cur = next;
        }
        None
    });
    let Some(sntx) = found else {
        jot_error(&crate::t!("rcfile-syntax_not_found", name = syntaxname));
        return;
    };

    let (keyword, argument) = next_word(rest);
    if keyword.is_empty() {
        return;
    }

    /* 临时把当前语法切到目标语法，应用命令后再恢复。 */
    let saved = get_live_syntax();
    let saved_last = get_last_color();
    set_live_syntax(Some(sntx.clone()));
    set_last_color(None);

    match keyword {
        "color" => parse_rule(argument, false),
        "icolor" => parse_rule(argument, true),
        "header" => grab_and_store("header", argument, StorageKind::Headers),
        "magic" => grab_and_store("magic", argument, StorageKind::Magics),
        "extension" => grab_and_store("extension", argument, StorageKind::Extensions),
        "comment" => pick_up_name("comment", argument, CommentTarget::Comment),
        "tabgives" => pick_up_name("tabgives", argument, CommentTarget::Tabstring),
        "linter" => pick_up_name("linter", argument, CommentTarget::Linter),
        "formatter" => pick_up_name("formatter", argument, CommentTarget::Formatter),
        _ => {}
    }

    set_live_syntax(saved);
    set_last_color(saved_last);
}

/// syntax 存储目标（extensions/headers/magics）。
#[derive(Clone, Copy)]
enum StorageKind {
    Extensions,
    Headers,
    Magics,
}

/// 解析 include 命令。
fn parse_include_command(args: &str) {
    let path = args.trim().trim_matches('"');
    // 展开 ~
    let expanded = if path.starts_with('~') {
        with_global(|g| {
            g.homedir.clone().map(|h| {
                path.replacen('~', &h, 1)
            }).unwrap_or_else(|| path.to_string())
        })
    } else {
        path.to_string()
    };
    parse_rcfile(&expanded);
}

/// 获取语法列表。
pub fn get_syntaxes() -> Option<SyntaxRef> {
    with_global(|g| g.syntaxes.clone())
}

/// 获取注释字符串。
pub fn get_comment_string() -> Option<String> {
    with_global(|g| {
        g.openfile.as_ref().and_then(|of| {
            of.borrow().syntax.clone().and_then(|s| s.borrow().comment.clone())
        })
    });
    None
}

// ======================== 错误消息（对应 rcfile.c 的 jot_error） ========================

use std::cell::Cell;

/// nanorc 文件状态（对应 rcfile.c 的 static 变量 nanorc、lineno）。
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
            let nanorc = NANORC_FILE.with(|n| n.borrow().clone());
            g.startup_problem = Some(match nanorc {
                Some(nr) => crate::t!("rcfile-mistakes_in", name = nr),
                None => "Problems with history file".to_string(),
            });
        }
    });

    /* 拼装完整错误文本。 */
    let lineno = NANORC_LINENO.with(|l| l.get());
    let nanorc = NANORC_FILE.with(|n| n.borrow().clone());
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

/// 打印累积的错误消息到 stderr（对应 rcfile.c 的 `print_errors`）。
pub fn print_errors() {
    let mut item = ERRORS_HEAD.with(|h| h.borrow().clone());
    while let Some(e) = item {
        eprintln!("{}", e.borrow().data);
        let next = { let r = e.borrow(); r.next.clone() };
        item = next;
    }
}