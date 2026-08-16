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
//! - 各菜单的介绍文本保留（未翻译的原字符串）；
//! - `make_new_buffer`/`close_buffer` 委托给 [`crate::nano`]，
//!   文本换行用 [`crate::text::break_line`]。

use crate::definitions::*;
use crate::global;
use crate::movement;
use crate::nano;
use crate::text;
use crate::utils;
use crate::winio;

// ======================== 帮助文本组装（对应 help_init） ========================

/// 为当前菜单分配帮助文本空间，并把不同文本拼接进去
/// （对应 `help_init`）。
pub fn help_init() {
    let currmenu = with_global(|g| g.currmenu);

    /* 各菜单的帮助介绍文本。 */
    let (htx0, htx1, htx2): (&str, &str, &str) = if currmenu & (MWHEREIS | MREPLACE) != 0 {
        ("Search Command Help Text\n\n "
            ,
         "If you have selected text with the mark and then search to replace, \
          only matches in the selected text will be replaced.\n\n \
          The following function keys are available in Search mode:\n\n"
            ,
         "")
    } else if currmenu == MREPLACEWITH {
        ("=== Replacement ===\n\n ",
         " The following function keys are available at this prompt:\n\n",
         "")
    } else if currmenu == MGOTOLINE {
        ("Go To Line Help Text\n\n ",
         "",
         "")
    } else if currmenu == MINSERTFILE {
        ("Insert File Help Text\n\n ",
         " The following function keys are available in Insert File mode:\n\n",
         "")
    } else if currmenu == MWRITEFILE {
        ("Write File Help Text\n\n ",
         "",
         "")
    } else if currmenu == MBROWSER {
        ("File Browser Help Text\n\n ",
         "",
         "")
    } else if currmenu == MWHEREISFILE {
        ("Browser Search Command Help Text\n\n ",
         "",
         "")
    } else if currmenu == MGOTODIR {
        ("Browser Go To Directory Help Text\n\n ",
         "",
         "")
    } else if currmenu == MSPELL {
        ("=== Spelling correction ===\n\n ",
         " The following function keys are available at this prompt:\n\n",
         "")
    } else if currmenu == MEXECUTE {
        ("Execute Command Help Text\n\n ",
         " The following function keys are available at this prompt:\n\n",
         "")
    } else if currmenu == MLINTER {
        ("=== Linter ===\n\n ",
         " The following function keys are available in Linter mode:\n\n",
         "")
    } else {
        /* 默认使用主帮助列表。 */
        ("Main nano help text\n\n \
          The nano editor is designed to emulate the functionality and \
          ease-of-use of the UW Pico text editor.  There are four main \
          sections of the editor.  The top line shows the program version, \
          the current filename being edited, and whether or not the file \
          has been modified.  Next is the main editor window showing the \
          file being edited.  The status line is the third line from the \
          bottom and shows important messages.  ",
         "The bottom two lines show the most commonly used shortcuts in \
          the editor.\n\n Shortcuts are written as follows: Control-key \
          sequences are notated with a '^' and can be entered either by \
          using the Ctrl key or pressing the Esc key twice.  Meta-key \
          sequences are notated with 'M-' and can be entered using either \
          the Alt, Cmd, or Esc key, depending on your keyboard setup.  ",
         "Also, pressing Esc twice and then typing a three-digit decimal \
          number from 000 to 255 will enter the character with the \
          corresponding value.  The following keystrokes are available in \
          the main editor window.  Alternative keys are shown in \
          parentheses:\n\n")
    };

    let mut help_text = String::new();
    help_text.push_str(htx0);
    help_text.push_str(htx1);
    if !htx2.is_empty() {
        help_text.push_str(htx2);
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

        /* 显示每个函数的前两个快捷键（若有）。 */
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
    /* 对应 C 的 start_of_body：跳过标题行（标题单独显示在标题栏）。 */
    let start_of_body = with_global(|g| g.help_start_of_body);

    let mut ptr = start_of_body;
    let mut sum = 0;

    nano::make_new_buffer();

    /* 顶部确保有空白行（美学）。 */
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

    /* 把帮助文本复制到刚创建的新缓冲区。 */
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
        /* C: snprintf(oneline, length + shim, "%s", ptr) 最多写 length + shim - 1 字符。 */
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

                /* 创建新行，并为每个额外 \n 再创建一行。 */
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

        /* C: do { 创建新行 } while (*(++ptr) == '\n')——至少前进一次；
           每再遇到一个换行就为它创建一行空行。 */
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

    /* 移到之前所在的位置。 */
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

    /* 保存 flag 设置。 */
    let stash_flags = with_global(|g| clone_flags());
    let was_tabsize = with_global(|g| g.tabsize);
    /* 确保帮助屏幕的快捷键列表能显示。 */
    let no_help_or_zero = ISSET(NO_HELP) || ISSET(ZERO);
    if no_help_or_zero {
        UNSET(NO_HELP);
        UNSET(ZERO);
        winio::window_init();
    }

    /* 搜索时向前、不区分大小写、不使用正则。 */
    UNSET(BACKWARDS_SEARCH);
    UNSET(CASE_SENSITIVE);
    UNSET(USE_REGEXP);
    UNSET(WHITESPACE_DISPLAY);

    with_global_mut(|g| {
        g.tabsize = 8;
        set_tabsize_independent(8);
    });
    winio::curs_set(0);

    /* 从所有相关部分组装帮助文本。 */
    help_init();

    with_global_mut(|g| {
        g.inhelp = true;
        g.help_location = 0;
        g.didfind = 0;
        g.currmenu = MHELP;
    });

    winio::bottombars(with_global(|g| g.currmenu));

    /* 从帮助文本头部提取标题。 */
    let help_text = with_global(|g| g.help_text.clone()).unwrap_or_default();
    let length = text::break_line(help_text.as_bytes(), usize::MAX as isize >> 1, true) as usize;
    let title = help_text[..length].to_string();
    with_global_mut(|g| g.title = Some(title.clone()));

    winio::titlebar(Some(&title));

    /* 跳过标题指向正文开头。 */
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

        /* 显示光标（搜索并找到内容时）。 */
        let didfind = with_global(|g| g.didfind);
        let show_cursor = with_global(|g| ISSET(SHOW_CURSOR));
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

        /* 计算 edittop 在文件中的字节偏移。 */
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

    /* 丢弃帮助文本缓冲区。 */
    nano::close_buffer();

    /* 恢复 flag 设置。 */
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