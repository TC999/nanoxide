/**************************************************************************
 *   help.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分（对应 help.c）
 *   版权 (C) 2000-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2017 Rishabh Dave
 *   版权 (C) 2014-2019 Benno Schulenberg
 *   版权 (C) 2015-2022, 2024, 2026 Benno Schulenberg
 **************************************************************************/

//! 帮助查看器：组装、显示并滚动浏览帮助文本。对应原始 help.c。
//! 全功能构建：所有条件编译块均按已启用翻译。
use crate::definitions::*;
use crate::definitions::{KEY_MOUSE, START_OF_PASTE, END_OF_PASTE, THE_WINDOW_RESIZED};
use crate::global::*;
use crate::browser::{functionptrtype, interpret, unbound_key, browser_refresh};
use crate::rcfile::implant as implant_stub;
use crate::files::{COLS, LINES, make_new_buffer, find_and_prime_applicable_syntax,
    prepare_for_display, close_buffer};
use crate::text::{break_line, remove_magicline};
use crate::winio::{bottombars, edit_refresh, titlebar, blank_statusbar, window_init,
    curs_set, get_kbinput, statusline, get_mouseinput, implant, first_sc_for, full_refresh,
    ERR};
use crate::gettext;

/* 帮助窗口中显示的文本。 */
static mut help_text: Option<String> = None;
/* 标题之后、正文开始处的指针。 */
static mut start_of_body: Option<String> = None;
/* 快捷键描述开始处的指针。 */
static mut end_of_intro: usize = 0;
/* 已显示帮助文本顶左角（以字节计）的偏移。 */
static mut location: usize = 0;

/* 为当前菜单分配帮助文本空间，并把不同片段拼接进去。 */
pub unsafe fn help_init() {
    let mut allocsize: usize = 0;
    /* 帮助文本所需空间。 */
    let mut htx: [&str; 3] = ["", "", ""];
    /* 未翻译的帮助引言。拆成三块以防整串太长编译器无法处理。 */
    let mut f: *mut funcstruct;
    let mut s: *mut keystruct;

    /* 首先，为当前函数设置初始帮助文本。 */
    if currmenu & (MWHEREIS | MREPLACE) != 0 {
        htx[0] = N_("Search Command Help Text\n\n Enter the words or characters you would like to search for, and then press Enter.  If there is a match for the text you entered, the screen will be updated to the location of the nearest match for the search string.\n\n The previous search string will be shown in brackets after the search prompt.  Hitting Enter without entering any text will perform the previous search.  ");
        htx[1] = N_("If you have selected text with the mark and then search to replace, only matches in the selected text will be replaced.\n\n The following function keys are available in Search mode:\n\n");
        htx[2] = "";
    } else if currmenu == MREPLACEWITH {
        htx[0] = N_("=== Replacement ===\n\n Type the characters that should replace what you typed at the previous prompt, and press Enter.\n\n");
        htx[1] = N_(" The following function keys are available at this prompt:\n\n");
        htx[2] = "";
    } else if currmenu == MGOTOLINE {
        htx[0] = N_("Go To Line Help Text\n\n Enter the line number that you wish to go to and hit Enter.  If there are fewer lines of text than the number you entered, you will be brought to the last line of the file.\n\n The following function keys are available in Go To Line mode:\n\n");
        htx[1] = "";
        htx[2] = "";
    } else if currmenu == MINSERTFILE {
        htx[0] = N_("Insert File Help Text\n\n Type in the name of a file to be inserted into the current file buffer at the current cursor location.\n\n If you have compiled nano with multiple file buffer support, and enable multiple file buffers with the -F or --multibuffer command line flags, the Meta-F toggle, or a nanorc file, inserting a file will cause it to be loaded into a separate buffer (use Meta-< and > to switch between file buffers).  ");
        htx[1] = N_("If you need another blank buffer, do not enter any filename, or type in a nonexistent filename at the prompt and press Enter.\n\n The following function keys are available in Insert File mode:\n\n");
        htx[2] = "";
    } else if currmenu == MWRITEFILE {
        htx[0] = N_("Write File Help Text\n\n Type the name that you wish to save the current file as and press Enter to save the file.\n\n If you have selected text with the mark, you will be prompted to save only the selected portion to a separate file.  To reduce the chance of overwriting the current file with just a portion of it, the current filename is not the default in this mode.\n\n The following function keys are available in Write File mode:\n\n");
        htx[1] = "";
        htx[2] = "";
    } else if currmenu == MBROWSER {
        htx[0] = N_("File Browser Help Text\n\n The file browser is used to visually browse the directory structure to select a file for reading or writing.  You may use the arrow keys or Page Up/Down to browse through the files, and S or Enter to choose the selected file or enter the selected directory.  To move up one level, select the directory called \"..\" at the top of the file list.\n\n The following function keys are available in the file browser:\n\n");
        htx[1] = "";
        htx[2] = "";
    } else if currmenu == MWHEREISFILE {
        htx[0] = N_("Browser Search Command Help Text\n\n Enter the words or characters you would like to search for, and then press Enter.  If there is a match for the text you entered, the screen will be updated to the location of the nearest match for the search string.\n\n The previous search string will be shown in brackets after the search prompt.  Hitting Enter without entering any text will perform the previous search.\n\n");
        htx[1] = N_(" The following function keys are available at this prompt:\n\n");
        htx[2] = "";
    } else if currmenu == MGOTODIR {
        htx[0] = N_("Browser Go To Directory Help Text\n\n Enter the name of the directory you would like to browse to.\n\n If tab completion has not been disabled, you can use the Tab key to (attempt to) automatically complete the directory name.\n\n The following function keys are available in Browser Go To Directory mode:\n\n");
        htx[1] = "";
        htx[2] = "";
    } else if currmenu == MSPELL {
        htx[0] = N_("=== Spelling correction ===\n\n The spell checker has examined the spelling of all text in the current buffer or marked region.  An unknown word has been encountered -- it is highlighted and a replacement can now be edited.  After this you will be asked whether to replace each instance of that unknown word.\n\n");
        htx[1] = N_(" The following function keys are available at this prompt:\n\n");
        htx[2] = "";
    } else if currmenu == MEXECUTE {
        htx[0] = N_("Execute Command Help Text\n\n This mode allows you to insert the output of a command run by the shell into the current buffer (or into a new buffer).  If the command is preceded by '|' (the pipe symbol), the current contents of the buffer (or marked region) will be piped to the command.  ");
        htx[1] = N_("If you just need another blank buffer, do not enter any command.\n\n You can also pick one of four tools, or cut a large piece of the buffer, or put the editor to sleep.\n\n");
        htx[2] = N_(" The following function keys are available at this prompt:\n\n");
    } else if currmenu == MLINTER {
        htx[0] = N_("=== Linter ===\n\n In this mode, the status bar shows an error message or warning, and the cursor is put at the corresponding position in the file.  With PageUp and PageDown you can switch to earlier and later messages.\n\n");
        htx[1] = N_(" The following function keys are available in Linter mode:\n\n");
        htx[2] = "";
    } else {
        /* 默认使用主帮助列表。 */
        htx[0] = N_("Main nano help text\n\n The nano editor is designed to emulate the functionality and ease-of-use of the UW Pico text editor.  There are four main sections of the editor.  The top line shows the program version, the current filename being edited, and whether or not the file has been modified.  Next is the main editor window showing the file being edited.  The status line is the third line from the bottom and shows important messages.  ");
        htx[1] = N_("The bottom two lines show the most commonly used shortcuts in the editor.\n\n Shortcuts are written as follows: Control-key sequences are notated with a '^' and can be entered either by using the Ctrl key or pressing the Esc key twice.  Meta-key sequences are notated with 'M-' and can be entered using either the Alt, Cmd, or Esc key, depending on your keyboard setup.  ");
        htx[2] = N_("Also, pressing Esc twice and then typing a three-digit decimal number from 000 to 255 will enter the character with the corresponding value.  The following keystrokes are available in the main editor window.  Alternative keys are shown in parentheses:\n\n");
    }

    /* 计算快捷键描述的长度。每个条目有一到两个按键（约 17 格），
     * 加上翻译后的文本，再加一两个 \n。 */
    f = allfuncs;
    while !f.is_null() {
        if (*f).menus & currmenu != 0 {
            allocsize += (*(*f).phrase).len() + 21;
        }
        f = (*f).next;
    }

    /* 如果在主列表上，还要计入开关帮助文本。每个条目有 "M-%c\t\t " 六个字符
     * 约 17 格，加上两段翻译文本、一个空格和一两个 '\n'。 */
    if currmenu == MMAIN {
        let onoff_len = gettext!("enable/disable").len();

        s = sclist;
        while !s.is_null() {
            if (*s).func == Some(do_toggle) {
                allocsize += epithet_of_flag((*s).toggle as usize).len() + onoff_len + 9;
            }
            s = (*s).next;
        }
    }

    /* 拼接帮助文本。 */
    let mut text = String::new();
    for h in htx.iter() {
        text.push_str(h);
    }

    /* 记住引言结束、快捷键开始的位置。 */
    end_of_intro = text.len();

    /* 现在加入快捷键及其描述。 */
    f = allfuncs;
    while !f.is_null() {
        let mut tally = 0;

        if (*f).menus & currmenu == 0 {
            f = (*f).next;
            continue;
        }

        /* 显示每个函数的前两个快捷键（若有）。 */
        s = sclist;
        while !s.is_null() {
            if (*s).menus & currmenu != 0 && (*s).func == (*f).func && !(*s).keystr.is_empty() {
                /* 第一列宽 7 格，第二列宽 10 格。 */
                if tally == 0 {
                    if (*s).keystr.contains('\u{E2}') {
                        text.push_str(&format!("{:width$}", (*s).keystr, width = 9));
                    } else {
                        text.push_str(&format!("{:width$}", (*s).keystr, width = 7));
                    }
                    tally += 1;
                } else {
                    if (*s).keystr.contains('\u{E2}') {
                        text.push_str(&format!("({:width$})", (*s).keystr, width = 12));
                    } else {
                        text.push_str(&format!("({:width$})", (*s).keystr, width = 10));
                    }
                    tally += 1;
                    break;
                }
            }
            s = (*s).next;
        }

        if tally == 0 {
            text.push_str("\t\t ");
        } else if tally == 1 {
            text.push_str("          ");
        }

        /* 快捷键的描述。 */
        text.push_str(&format!("{}\n", (*f).phrase));

        if (*f).blank_after {
            text.push('\n');
        }

        f = (*f).next;
    }

    /* 以及开关…… */
    if currmenu == MMAIN {
        let mut maximum = 0;
        let mut counter = 0;

        /* 先看看有多少个开关。 */
        s = sclist;
        while !s.is_null() {
            if (*s).toggle != 0 && (*s).ordinal > maximum {
                maximum = (*s).ordinal;
            }
            s = (*s).next;
        }

        /* 现在按原始顺序显示它们。 */
        while counter < maximum {
            counter += 1;
            s = sclist;
            while !s.is_null() {
                if (*s).toggle != 0 && (*s).ordinal == counter {
                    text.push_str(&format!(
                        "{}\t\t {} {}\n",
                        if (*s).menus & MMAIN != 0 { (*s).keystr } else { "" },
                        epithet_of_flag((*s).toggle as usize),
                        gettext!("enable/disable")
                    ));
                    /* 两组之间加一个空行。 */
                    if (*s).toggle == NO_SYNTAX as i32 {
                        text.push('\n');
                    }
                    break;
                }
                s = (*s).next;
            }
        }
    }

    help_text = Some(text);
}

/* 硬换行拼接好的帮助文本，并写入一个新缓冲区。 */
pub unsafe fn wrap_help_text_into_buffer() {
    /* 避免引言段落过紧或过宽。 */
    let sidebar_val = sidebar;
    let mut wrapping_point =
        if COLS < 40 { 40 } else if COLS > 74 { 74 } else { COLS } - sidebar_val;
    let ptr = help_text.as_ref().unwrap();
    let bytes = ptr.as_bytes();
    let mut sum: usize = 0;

    make_new_buffer();

    /* 为确保顶部有一空行（美观），在文本顶部留一行。 */
    if (ISSET(MINIBAR) || !ISSET(EMPTY_LINE)) && LINES > 6 {
        (*openfile).current = (*openfile).filetop;
        (*(*openfile).current).data = " ".to_string();
        (*(*openfile).current).next =
            Box::into_raw(make_new_node(&*(*openfile).current));
        (*openfile).current = (*(*openfile).current).next;
    }

    /* 把帮助文本复制到刚创建的新缓冲区中。 */
    let mut index: usize = 0;
    while index < bytes.len() {
        let length: isize;

        if index == end_of_intro {
            wrapping_point = if COLS < 40 { 40 } else { COLS } - sidebar_val;
        }

        if index < end_of_intro || index == 0 || bytes[index - 1] == b'\n' {
            length = break_line(&ptr[index..], wrapping_point as isize, true);
            let end = (index + length as usize).min(ptr.len());
            let oneline = copy_of(&ptr[index..end]);
            (*(*openfile).current).data = oneline;
        } else {
            length = break_line(
                &ptr[index..],
                ((if COLS < 40 { 22 } else { COLS - 18 }) - sidebar_val)
                    .try_into()
                    .unwrap(),
                true,
            );
            let end = (index + length as usize).min(ptr.len());
            let oneline = format!("\t\t  {}", &ptr[index..end]);
            (*(*openfile).current).data = oneline;
        }

        index += length as usize;
        if index < bytes.len() && bytes[index] != b'\n' {
            index -= 1;
        }

        /* 创建一个新行，并为每个额外的 \n 再创建一个。 */
        loop {
            (*(*openfile).current).next =
                Box::into_raw(make_new_node(&*(*openfile).current));
            (*openfile).current = (*(*openfile).current).next;
            (*(*openfile).current).data = copy_of("");
            if index >= bytes.len() || bytes[index] != b'\n' {
                break;
            }
            index += 1;
        }
    }

    (*openfile).filebot = (*openfile).current;
    (*openfile).current = (*openfile).filetop;

    remove_magicline();
    find_and_prime_applicable_syntax();
    prepare_for_display();

    /* 移动到我们之前所在的位置。 */
    let mut line = (*openfile).current;
    while !line.is_null() {
        sum += (*line).data.len();
        if (sum as isize) > location as isize {
            break;
        }
        line = (*line).next;
    }

    (*openfile).edittop = line;
}

/* 组装帮助文本、显示它，并允许在其中滚动。 */
pub unsafe fn show_help() {
    let mut kbinput: i32 = ERR;
    let mut function: functionptrtype = None;
    /* 用户键入按键对应的函数。 */
    let oldmenu = currmenu;
    /* 调用我们时的菜单。 */
    let was_margin = margin;
    let was_tabsize = tabsize;
    let was_lighted = spotlighted;
    let was_syntax = syntaxstr.clone();
    /* 调用帮助时提示处的当前答案。 */
    let saved_answer = match answer.as_ref() {
        Some(a) => Some(copy_of(a)),
        None => None,
    };
    let mut stash: [flagword; 2] = [0; 2];
    /* 当前标志设置的存储位置。 */
    let mut line: *mut linestruct;
    let mut length: isize;

    /* 保存所有标志的设置。 */
    stash.copy_from_slice(&flags);

    /* 确保帮助屏幕的快捷键列表可以显示。 */
    if ISSET(NO_HELP) || ISSET(ZERO) {
        UNSET(NO_HELP);
        UNSET(ZERO);
        window_init();
    } else {
        blank_statusbar();
    }

    /* 搜索时：向前、大小写不敏感、不使用正则。 */
    UNSET(BACKWARDS_SEARCH);
    UNSET(CASE_SENSITIVE);
    UNSET(USE_REGEXP);

    UNSET(WHITESPACE_DISPLAY);

    editwincols = (COLS - sidebar) as usize;
    margin = 0;
    tabsize = 8;
    spotlighted = false;
    syntaxstr = Some("nanohelp".to_string());
    curs_set(0);

    /* 从所有相关片段组合帮助文本。 */
    help_init();

    inhelp = true;
    location = 0;
    didfind = 0;

    bottombars(MHELP);

    /* 从帮助文本头部提取标题。 */
    length = break_line(help_text.as_ref().unwrap(), HIGHEST_POSITIVE as isize, true);
    title = Some(copy_of(
        &help_text.as_ref().unwrap()[..length as usize],
    ));

    titlebar(title.as_deref());

    /* 跳过标题，指向正文开始处。 */
    start_of_body = Some(help_text.as_ref().unwrap()[length as usize..].to_string());
    while let Some(sb) = start_of_body.as_ref() {
        if !sb.is_empty() && sb.as_bytes()[0] == b'\n' {
            start_of_body = Some(sb[1..].to_string());
        } else {
            break;
        }
    }

    wrap_help_text_into_buffer();
    edit_refresh();

    while true {
        lastmessage = message_type::VACUUM;
        focusing = true;

        /* 当我们搜索并找到内容时显示光标。 */
        kbinput = get_kbinput(midwin, didfind == 1 || ISSET(SHOW_CURSOR));

        didfind = 0;

        spotlighted = false;
        function = interpret(kbinput);

        if ISSET(SHOW_CURSOR)
            && (function == Some(do_left)
                || function == Some(do_right)
                || function == Some(do_up)
                || function == Some(do_down))
        {
            if let Some(f) = function {
                f();
            }
        } else if function == Some(do_up) || function == Some(do_scroll_up) {
            do_scroll_up();
        } else if function == Some(do_down) || function == Some(do_scroll_down) {
            if (*(*openfile).edittop).lineno + editwinrows as isize - 1
                < (*(*openfile).filebot).lineno
            {
                do_scroll_down();
            }
        } else if function == Some(do_page_up)
            || function == Some(do_page_down)
            || function == Some(to_first_line)
            || function == Some(to_last_line)
        {
            if let Some(f) = function {
                f();
            }
        } else if function == Some(do_search_backward)
            || function == Some(do_search_forward)
            || function == Some(do_findprevious)
            || function == Some(do_findnext)
        {
            if let Some(f) = function {
                f();
            }
            bottombars(MHELP);
        } else if function == Some(implant_stub) {
            let sc = first_sc_for(MHELP, implant_stub);
            if !sc.is_null() {
                let expansion = (*sc).expansion.clone();
                if let Some(e) = expansion {
                    implant(&e);
                }
            }
        } else if kbinput == KEY_MOUSE {
            let mut dummy_row: i32 = 0;
            let mut dummy_col: i32 = 0;
            get_mouseinput(&mut dummy_row, &mut dummy_col);
        } else if kbinput == START_OF_PASTE {
            while get_kbinput(midwin, false) != END_OF_PASTE {
                /* 空循环。 */
            }
            statusline(message_type::AHEM, gettext!("Paste is ignored"));
        } else if kbinput == THE_WINDOW_RESIZED {
            /* 什么都不做。 */
        } else if function == Some(full_refresh) {
            full_refresh();
        } else if function == Some(do_exit) {
            break;
        } else {
            unbound_key(kbinput);
        }

        edit_refresh();

        location = 0;
        line = (*openfile).filetop;

        /* 统计 edittop 在文件中（以字节计）深入多少。 */
        while line != (*openfile).edittop {
            location += (*line).data.len();
            line = (*line).next;
        }
    }

    /* 丢弃帮助文本缓冲区。 */
    close_buffer();

    /* 恢复所有标志的设置。 */
    flags.copy_from_slice(&stash);

    margin = was_margin;
    editwincols = (COLS - margin - sidebar) as usize;
    tabsize = was_tabsize;
    spotlighted = was_lighted;
    syntaxstr = was_syntax;
    have_palette = false;

    title = None;
    answer = saved_answer;
    help_text = None;
    inhelp = false;

    curs_set(0);

    if ISSET(NO_HELP) || ISSET(ZERO) {
        window_init();
    } else {
        blank_statusbar();
    }

    bottombars(oldmenu);

    if oldmenu & (MBROWSER | MGOTODIR | MWHEREISFILE) != 0 {
        browser_refresh();
    } else {
        titlebar(None);
        edit_refresh();
    }
}

/* 启动帮助查看器，或指示没有帮助。 */
pub unsafe fn do_help() {
    show_help();
}
