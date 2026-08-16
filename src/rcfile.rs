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
    with_global(|g| {
        if let Some(ref home) = g.homedir {
            let user_rc = format!("{}/.nanorc", home);
            if Path::new(&user_rc).exists() {
                parse_rcfile(&user_rc);
            }
        }
    });
}

/// 解析 nanorc 文件。
pub fn parse_rcfile(filename: &str) -> bool {
    match File::open(filename) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for (lineno, line) in reader.lines().enumerate() {
                if let Ok(line) = line {
                    let line = line.trim().to_string();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    parse_rcfile_line(&line, filename, lineno + 1);
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
fn parse_rcfile_line(line: &str, _filename: &str, _lineno: usize) {
    let parts: Vec<&str> = line.splitn(2, |c: char| c.is_whitespace()).collect();
    if parts.is_empty() {
        return;
    }

    let command = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match command.as_str() {
        "set" => parse_set_command(args),
        "unset" => parse_unset_command(args),
        "syntax" => parse_syntax_command(args),
        "color" | "icolor" => parse_color_command(command == "icolor", args),
        "include" => parse_include_command(args),
        "extendsyntax" => {} // 简化
        "bind" => {} // 简化
        "unbind" => {} // 简化
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

/// 解析 syntax 命令。
fn parse_syntax_command(args: &str) {
    let parts: Vec<&str> = args.splitn(2, |c: char| c.is_whitespace()).collect();
    if parts.is_empty() {
        return;
    }
    let name = parts[0].trim();
    let filename = parts.get(1).map(|s| s.trim()).unwrap_or("");
    let syntax = SyntaxType {
        name: Some(name.to_string()),
        filename: if filename.is_empty() { None } else { Some(filename.to_string()) },
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
    with_global_mut(|g| {
        let new_syntax = Rc::new(RefCell::new(syntax));
        new_syntax.borrow_mut().next = g.syntaxes.take();
        g.syntaxes = Some(new_syntax);
    });
}

/// 解析 color 命令。
fn parse_color_command(_case_insensitive: bool, args: &str) {
    let parts: Vec<&str> = args.splitn(3, |c: char| c.is_whitespace()).collect();
    if parts.len() < 2 {
        return;
    }
    let fg_name = parts[0];
    let bg_name = parts.get(1).unwrap_or(&"default");
    let _regex_str = parts.get(2).map(|s| s.trim_matches('"')).unwrap_or("");

    let fg = color::color_name_to_number(fg_name);
    let bg = color::color_name_to_number(bg_name);

    // 简化：不解析正则表达式
    let color_type = ColorType {
        id: 0,
        fg, bg,
        pairnum: 0,
        attributes: color::A_NORMAL,
        start: None,
        end: None,
        next: None,
    };

    // 添加到最后一个语法
    with_global_mut(|g| {
        if let Some(ref syntax) = g.syntaxes {
            let new_color = Rc::new(RefCell::new(color_type));
            new_color.borrow_mut().next = syntax.borrow().color.clone();
            syntax.borrow_mut().color = Some(new_color);
        }
    });
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