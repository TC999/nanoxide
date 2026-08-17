/**************************************************************************
 * help.rs  --  GNU nano 帮助查看器（对应 help.c）
 * 版权 (C) 2000-2011, 2013-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 帮助文本的组装、显示与滚动，完整移植自 `help.c`。
//!
//! 转换说明：
//! - `help_text`/`start_of_body`/`end_of_intro`/`location` 等静态状态
//!   放入 [`GlobalState`]；
//! - 各菜单的介绍文本通过 [`crate::t`] 宏读取外置 ftl 文件（en-US.ftl 默认）；
//! - `make_new_buffer`/`close_buffer` 委托给 [`crate::files`/`crate::nano`]，
//!   文本换行用 [`crate::text::break_line`]。

use crate::definitions::*;
use crate::global;
use crate::movement;
use crate::files;
use crate::text;
use crate::utils;
use crate::winio;

// ======================== 帮助文本组装（对应 help_init） ========================

/// 为当前菜单分配帮助文本空间，并把不同文本拼接进去
/// （对应 `help_init`）。
pub fn help_init() {
    let currmenu = with_global(|g| g.currmenu);

    /* 各菜单的帮助介绍文本，全部通过 i18n 宏外置加载。 */
    let (htx0, htx1, htx2): (String, String, String) = if currmenu & (MWHEREIS | MREPLACE) != 0 {
        (
            format!("{} \n\n {} \n\n {}",
                crate::t!("help-search_title"),
                crate::t!("help-search_body"),
                crate::t!("help-search_prev")),
            format!("{} \n\n {} \n\n {} \n\n",
                crate::t!("help-search_select"),
                "",
                crate::t!("help-search_fnkeys")),
            String::new()
        )
    } else if currmenu == MREPLACEWITH {
        (
            format!("{} \n\n ", crate::t!("help-replace_title")),
            format!("{} \n\n", crate::t!("help-replace_body")),
            format!(" {} \n\n", crate::t!("help-replace_fnkeys"))
        )
    } else if currmenu == MGOTOLINE {
        (
            format!("{} \n\n ", crate::t!("help-goto_line_title")),
            format!("{} \n\n", crate::t!("help-goto_body")),
            format!(" {} \n\n", crate::t!("help-goto_fnkeys"))
        )
    } else if currmenu == MINSERTFILE {
        (
            format!("{} \n\n ", crate::t!("help-insert_file_title")),
            format!("{} {} \n\n", crate::t!("help-insert_body"), crate::t!("help-insert_extra")),
            format!(" {} \n\n", crate::t!("help-insert_fnkeys"))
        )
    } else if currmenu == MWRITEFILE {
        (
            format!("{} \n\n ", crate::t!("help-write_file_title")),
            format!("{} \n\n", crate::t!("help-write_body")),
            format!(" {} \n\n", crate::t!("help-write_fnkeys"))
        )
    } else if currmenu == MBROWSER {
        (
            format!("{} \n\n ", crate::t!("help-browser_title")),
            format!("{} \n\n", crate::t!("help-browser_body")),
            format!(" {} \n\n", crate::t!("help-browser_fnkeys"))
        )
    } else if currmenu == MWHEREISFILE {
        (
            format!("{} \n\n ", crate::t!("help-browser_search_title")),
            format!("{} \n\n {}", crate::t!("help-bsearch_body"), crate::t!("help-bsearch_prev")),
            format!(" {} \n\n", crate::t!("help-replace_fnkeys"))
        )
    } else if currmenu == MGOTODIR {
        (
            format!("{} \n\n ", crate::t!("help-browser_gotodir_title")),
            format!("{} \n\n", crate::t!("help-bgotodir_body")),
            format!(" {} \n\n", crate::t!("help-bgotodir_fnkeys"))
        )
    } else if currmenu == MSPELL {
        (
            format!("{} \n\n ", crate::t!("help-spell_title")),
            format!(" {} \n\n", crate::t!("help-spell_fnkeys")),
            String::new()
        )
    } else if currmenu == MEXECUTE {
        (
            format!("{} \n\n ", crate::t!("help-execute_title")),
            format!(" {} \n\n", crate::t!("help-spell_fnkeys")),
            String::new()
        )
    } else if currmenu == MLINTER {
        (
            format!("{} \n\n ", crate::t!("help-linter_title")),
            format!(" {} \n\n", crate::t!("help-linter_fnkeys")),
            String::new()
        )
    } else {
        /* 默认使用主帮助列表。 */
        (
            format!("{} \n\n {}", crate::t!("help-main_title"), crate::t!("help-main_body")),
            format!("{} \n\n", crate::t!("help-main_keydesc")),
            format!("{} \n\n", crate::t!("help-main_extra"))
        )
    };

    let mut help_text = String::new();
    help_text.push_str(&htx0);
    help_text.push_str(&htx1);
    if !htx2.is_empty() {
        help_text.push_str(&htx2);
    }

    /* 记住"介绍结束、快捷键开始"的位置。 */
    let end_of_intro = help_text.len();

    /* 现在添加快捷键及其描述。 */
    let funcs = global::iter_funcs();
    for f in &funcs {
        let f_ref = f.borrow();
        if (f_ref.menus & currmenu) == 0 {
            continue;
        }
        let func = f_ref.func;
        let phrase = f_ref.phrase.to_string();
        let blank_after = f_ref.blank_after;
        drop(f_ref);

        let mut tally = 0;
        let mut first_keys = String::new();
        let shortcuts = global::iter_shortcuts();
        for s in &shortcuts {
            let s_ref = s.borrow();
            if (s_ref.menus & currmenu) != 0 && s_ref.func == func && !s_ref.keystr.is_empty() {
                if tally == 0 {
                    first_keys = format!("{:<16}", s_ref.keystr);
                    tally = 1;
                } else {
                    first_keys = format!("{}({})", first_keys, s_ref.keystr);
                    tally = 2;
                    break;
                }
            }
        }

        if tally == 0 {
            help_text.push_str("\t\t ");
        } else if tally == 1 {
            help_text.push_str(&format!("{:<10}", first_keys));
        } else {
            help_text.push_str(&first_keys);
        }

        help_text.push_str(&phrase);
        help_text.push('\n');
        if blank_after {
            help_text.push('\n');
        }
    }

    with_global_mut(|g| {
        g.help_text = Some(help_text);
        g.help_end_of_intro = end_of_intro;
    });
}

// ======================== 帮助文本换行（对应 wrap_help_text_into_buffer） ========================

/// 将拼接的帮助文本硬换行，并写入新缓冲区（对应 `wrap_help_text_into_buffer`）。
pub fn wrap_help_text_into_buffer() {
    let (cols, sidebar) = with_global(|g| (g.COLS, g.sidebar));
    let mut wrapping_point = (if cols < 40 { 40 } else if cols > 74 { 74 } else { cols }) - sidebar as usize;

    let help_text = with_global(|g| g.help_text.clone()).unwrap_or_default();
    let end_of_intro = with_global(|g| g.help_end_of_intro);
    let start_of_body = with_global(|g| g.help_start_of_body);

    let mut ptr = start_of_body;
    let mut sum = 0;

    files::make_new_buffer();

    if !ISSET(MINIBAR) || !ISSET(EMPTY_LINE) {
        let lines = with_global(|g| g.LINES);
        if lines > 6 {
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    let mut of = of.borrow_mut();
                    let cur = of.current.clone().unwrap();
                    cur.borrow_mut().data = " ".to_string();
                    let newnode = make_new_node(Some(&*cur.borrow()));
                    newnode.borrow_mut().prev = Some(std::rc::Rc::downgrade(&cur));
                    cur.borrow_mut().next = Some(newnode.clone());
                    of.current = Some(newnode);
                }
            });
        }
    }

    let bytes = help_text.as_bytes();
    while ptr < bytes.len() {
        if ptr == end_of_intro {
            wrapping_point = (if cols < 40 { 40 } else { cols }) - sidebar as usize;
        }

        let is_intro = ptr < end_of_intro || (ptr > 0 && bytes[ptr - 1] == b'\n');
        let length;
        if is_intro {
            length = text::break_line(&bytes[ptr..], wrapping_point as isize, true) as usize;
        } else {
            length = text::break_line(&bytes[ptr..], ((if cols < 40 { 22 } else { cols - 18 }) - sidebar as usize) as isize, true) as usize;
        }

        let shim = if bytes.get(ptr + length.saturating_sub(1)).copied().unwrap_or(0) == b' ' { 0 } else { 1 };
        let copylen = (length + shim).saturating_sub(1).min(bytes.len() - ptr);
        let oneline: String = if is_intro {
            String::from_utf8_lossy(&bytes[ptr..ptr + copylen]).into_owned()
        } else {
            format!("\t\t  {}", String::from_utf8_lossy(&bytes[ptr..ptr + copylen]))
        };

        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                let cur = of.current.clone().unwrap();
                cur.borrow_mut().data = oneline;

                let newnode = make_new_node(Some(&*cur.borrow()));
                newnode.borrow_mut().prev = Some(std::rc::Rc::downgrade(&cur));
                cur.borrow_mut().next = Some(newnode.clone());
                of.current = Some(newnode);
            }
        });

        ptr += length;
        if bytes.get(ptr).copied().unwrap_or(0) != b'\n' {
            ptr = ptr.saturating_sub(1);
        }

        loop {
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    let mut of = of.borrow_mut();
                    let cur = of.current.clone().unwrap();
                    let newnode = make_new_node(Some(&*cur.borrow()));
                    newnode.borrow_mut().prev = Some(std::rc::Rc::downgrade(&cur));
                    cur.borrow_mut().next = Some(newnode.clone());
                    of.current = Some(newnode);
                }
            });
            ptr += 1;
            if bytes.get(ptr).copied().unwrap_or(0) != b'\n' {
                break;
            }
        }
    }

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.filebot = of.current.clone();
            of.current = of.filetop.clone();
        }
    });

    utils::remove_magicline();
    crate::color::find_and_prime_applicable_syntax();
    crate::files::prepare_for_display();

    let location = with_global(|g| g.help_location);
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let mut cur = of.filetop.clone().unwrap();
            loop {
                let dlen = cur.borrow().data.len();
                sum += dlen;
                if sum > location {
                    break;
                }
                let next = { let r = cur.borrow(); r.next.clone() };
                match next {
                    Some(n) => cur = n,
                    None => break,
                }
            }
            of.current = Some(cur.clone());
            of.edittop = Some(cur);
        }
    });
}

// ======================== 帮助显示（对应 show_help / do_help） ========================

/// 组装帮助文本，显示它，并允许滚动（对应 `show_help`）。
pub fn show_help() {
    let oldmenu = with_global(|g| g.currmenu);

    let stash_flags = with_global(|_g| clone_flags());
    let was_tabsize = with_global(|g| g.tabsize);
    let no_help_or_zero = ISSET(NO_HELP) || ISSET(ZERO);
    if no_help_or_zero {
        UNSET(NO_HELP);
        UNSET(ZERO);
        winio::window_init();
    }

    UNSET(BACKWARDS_SEARCH);
    UNSET(CASE_SENSITIVE);
    UNSET(USE_REGEXP);
    UNSET(WHITESPACE_DISPLAY);

    with_global_mut(|g| {
        g.tabsize = 8;
        set_tabsize_independent(8);
    });
    winio::curs_set(0);

    help_init();

    with_global_mut(|g| {
        g.inhelp = true;
        g.help_location = 0;
        g.didfind = 0;
        g.currmenu = MHELP;
    });

    winio::bottombars(with_global(|g| g.currmenu));

    let help_text = with_global(|g| g.help_text.clone()).unwrap_or_default();
    let length = text::break_line(help_text.as_bytes(), usize::MAX as isize >> 1, true) as usize;
    let title = help_text[..length].to_string();
    with_global_mut(|g| g.title = Some(title.clone()));

    winio::titlebar(Some(&title));

    let mut start_of_body = length;
    while help_text.as_bytes().get(start_of_body).copied().unwrap_or(0) == b'\n' {
        start_of_body += 1;
    }
    with_global_mut(|g| g.help_start_of_body = start_of_body);

    wrap_help_text_into_buffer();
    winio::edit_refresh();

    loop {
        with_global_mut(|g| {
            g.lastmessage = MessageType::Vacuum;
            g.focusing = true;
        });

        let _didfind = with_global(|g| g.didfind);
        let show_cursor = with_global(|_g| ISSET(SHOW_CURSOR));
        let kbinput = winio::get_kbinput();
        with_global_mut(|g| g.didfind = 0);

        let function = global::interpret(kbinput);

        match function {
            Some(FunctionId::DoLeft) | Some(FunctionId::DoRight) => {
                if show_cursor {
                    crate::prompt::run_function(function.unwrap());
                }
            }
            Some(FunctionId::DoUp) | Some(FunctionId::DoScrollUp) => movement::do_scroll_up(),
            Some(FunctionId::DoDown) | Some(FunctionId::DoScrollDown) => {
                let (et, fb, rows) = with_global(|g| {
                    let of = g.openfile.as_ref().unwrap().borrow();
                    let et = of.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(0);
                    let fb = of.filebot.as_ref().map(|b| b.borrow().lineno).unwrap_or(0);
                    (et, fb, g.editwinrows)
                });
                if et + rows as isize - 1 < fb {
                    movement::do_scroll_down();
                }
            }
            Some(FunctionId::DoPageUp) | Some(FunctionId::DoPageDown)
            | Some(FunctionId::DoFirstLine) | Some(FunctionId::DoLastLine) => {
                crate::prompt::run_function(function.unwrap());
            }
            Some(FunctionId::DoSearchForward) | Some(FunctionId::DoSearchBackward)
            | Some(FunctionId::DoFindNext) | Some(FunctionId::DoFindPrevious) => {
                crate::prompt::run_function(function.unwrap());
                winio::bottombars(with_global(|g| g.currmenu));
            }
            Some(FunctionId::DoFullRefresh) => winio::full_refresh(),
            Some(FunctionId::DoExit) => break,
            _ => global::unbound_key(kbinput),
        }

        winio::edit_refresh();

        with_global_mut(|g| {
            g.help_location = 0;
            if let Some(of) = &g.openfile {
                let of_ref = of.borrow();
                let mut line = of_ref.filetop.clone();
                let edittop = of_ref.edittop.clone();
                while let Some(l) = line {
                    if edittop.as_ref().map(|e| std::rc::Rc::ptr_eq(e, &l)).unwrap_or(false) {
                        break;
                    }
                    g.help_location += l.borrow().data.len();
                    let next = { let r = l.borrow(); r.next.clone() };
                    line = next;
                }
            }
        });
    }

    files::close_buffer();

    with_global_mut(|g| {
        restore_flags(stash_flags);
        g.tabsize = was_tabsize;
        set_tabsize_independent(was_tabsize);
        g.title = None;
        g.inhelp = false;
        g.currmenu = oldmenu;
    });

    winio::curs_set(0);

    if no_help_or_zero {
        winio::window_init();
    }
    winio::bottombars(with_global(|g| g.currmenu));
    winio::titlebar(None);
    winio::edit_refresh();
}

/// 启动帮助查看器（对应 `do_help`）。
pub fn do_help() {
    show_help();
}
