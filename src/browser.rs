/**************************************************************************
 *   browser.rs  --  GNU nano 文件浏览器（对应原版 browser.c 的 Rust 翻译）。
 *
 *   版权 (C) 2001-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2015, 2016, 2020, 2022, 2025 Benno Schulenberg
 **************************************************************************/

//! 文件浏览器：在文件系统中浏览目录、搜索文件名、选择文件或目录。
//! 对应原版 `browser.c`。全功能构建：所有条件编译块均按已启用翻译。

use crate::definitions::*;
use crate::gettext;
use crate::chars::*;
use crate::utils::{tail, breadth, actual_x, free_chararray};

use crate::files::{
    COLS, LINES, statusline, statusbar, blank_edit, wipe_statusbar, edit_refresh, titlebar,
    opendir, readdir, closedir, realpath, wmove, wnoutrefresh, curs_set, display_string,
    get_full_path, outside_of_confinement, expand_leading_tilde, diralphasort,
};
use crate::prompt::{do_prompt, wattron, wattroff, mvwprintw, mvwaddstr, wmouse_trafo, get_mouseinput};
use crate::text::{do_enter};
use crate::winio::{bottombars, window_init, napms, get_kbinput};
use crate::global::{
    do_help, do_exit, full_refresh, do_toggle, get_shortcut, first_sc_for, do_search_backward,
    do_search_forward, do_findprevious, do_findnext, do_left, do_right, to_prev_word,
    to_next_word, do_up, do_down, to_prev_block, to_next_block, do_page_up, do_page_down,
    goto_dir, resized_for_browser, answer, last_search, present_path, search_history,
    searchbot, operating_dir, interface_color_pair, midwin, editwinrows, lastmessage,
};
use crate::rcfile::implant;
use crate::history::update_history;
use crate::search::not_found_msg;

/* ===== 模块级状态变量（对应 C 的静态变量） ===== */

static mut FILELIST: Vec<String> = Vec::new();
    /* 文件浏览器中要显示的文件名列表。 */
static mut LIST_LENGTH: usize = 0;
    /* 列表中的文件数量。 */
static mut USABLE_ROWS: usize = 0;
    /* 可用于显示列表的屏幕行数。 */
static mut PILES: i32 = 0;
    /* 每个屏幕行可显示的文件数。 */
static mut GAUGE: i32 = 0;
    /* 一个“桩”的宽度 —— 最宽文件名加十。 */
static mut SELECTED: usize = 0;
    /* 当前选中的文件名在列表中的索引（从零开始）。 */

/// 快捷键函数指针类型（对应 C 的 functionptrtype）。
pub type functionptrtype = Option<unsafe fn()>;

/* ===== 尚未翻译模块的辅助函数桩 ===== */

/// 文件状态（对应 C 的 struct stat，此处为占位）。
#[derive(Default)]
struct Stat {
    st_mode: u32,
    st_size: i64,
}

/// 获取文件状态（占位实现）。
fn stat(_path: &str, _st: &mut Stat) -> i32 {
    0
}

/// 获取文件状态（不跟随符号链接，占位实现）。
fn lstat(_path: &str, _st: &mut Stat) -> i32 {
    0
}

/// 判断模式是否为目录（占位实现）。
fn s_isdir(_mode: u32) -> bool {
    false
}

/// 判断模式是否为符号链接（占位实现）。
fn s_islnk(_mode: u32) -> bool {
    false
}

/// 获取 errno（占位实现）。
fn errno() -> i32 {
    0
}

/// 将错误码转换为字符串（占位实现）。
fn strerror(_e: i32) -> String {
    String::new()
}

/// 重置目录流（占位实现）。
fn rewinddir(_dir: ()) {}

/// 返回绑定到给定按键的函数（占位实现）。
pub fn interpret(_key: i32) -> functionptrtype {
    None
}

/// 处理未绑定的按键（占位实现）。
pub fn unbound_key(_key: i32) {}

/* ===== 浏览器函数 ===== */

/* 用给定目录中的文件名填充 'filelist'，将 'list_length' 设置为该列表中的
 * 名称数量，将 'gauge' 设置为最宽文件名加十的宽度，并将 'piles' 设置为每个
 * 屏幕行可显示的文件数。并对列表进行排序。 */
pub fn read_the_list(path: &str, _dir: ()) {
    let mut widest: usize = 0;
    let mut index: usize = 0;

    /* 找出当前文件夹中最宽文件名的宽度。 */
    while let Some(name) = readdir(()) {
        let span = breadth(name.as_bytes());
        if span > widest {
            widest = span;
        }
        index += 1;
    }

    /* 预留十个列用于空白加上文件大小。 */
    let mut gauge = widest as i32 + 10;

    /* 如果需要，为“..（父目录）”留出空间。 */
    if gauge < 15 {
        gauge = 15;
    }
    let cols = unsafe { COLS };
    /* 确保我们不超过窗口宽度。 */
    if gauge > cols {
        gauge = cols;
    }

    rewinddir(());

    let old = unsafe { std::mem::take(&mut FILELIST) };
    free_chararray(old);

    unsafe { LIST_LENGTH = index };
    index = 0;

    let mut newlist: Vec<String> = Vec::with_capacity(unsafe { LIST_LENGTH });

    while let Some(name) = readdir(()) {
        /* 不要显示无用的点项。 */
        if name == "." {
            continue;
        }

        newlist.push(format!("{}{}", path, name));
        index += 1;
    }

    /* 可能在第一次扫描和第二次扫描之间，目录中的文件数量减少了。 */
    unsafe { LIST_LENGTH = index };

    /* 对名称列表进行排序。 */
    newlist.sort_by(|a, b| unsafe { diralphasort(a, b) });

    /* 计算每行可以容纳的文件数 —— 在右侧预留两个空格，并在列之间添加两个
     * 空格的填充。 */
    unsafe {
        FILELIST = newlist;
        GAUGE = gauge;
        PILES = (cols + 2) / (gauge + 2);
        let lines = LINES;
        USABLE_ROWS = (editwinrows - if ISSET(ZERO) && lines > 1 { 1 } else { 0 }) as usize;
    }
}

/* 重新选择给定的文件或目录名（如果它仍然存在）。 */
pub unsafe fn reselect(name: &str) {
    let mut looking_at: usize = 0;

    while looking_at < LIST_LENGTH && FILELIST[looking_at] != name {
        looking_at += 1;
    }

    /* 如果找到了所寻找的名称，则选中它；否则，仅仅移动高亮条，以便
     * 改变的选择将被注意到，但要确保停留在当前可用范围内。 */
    if looking_at < LIST_LENGTH {
        SELECTED = looking_at;
    } else if SELECTED > LIST_LENGTH {
        SELECTED = LIST_LENGTH - 1;
    } else {
        SELECTED -= 1;
    }
}

/* 最多显示一个屏幕的文件名。 */
pub unsafe fn browser_refresh() {
    let mut row: i32 = 0;
    let mut col: i32 = 0;
        /* 显示列表时当前的行和列。 */
    let mut the_row: i32 = 0;
    let mut the_column: i32 = 0;
        /* 选中项所在的行和列。 */

    titlebar(present_path.as_deref());
    blank_edit();

    let per_screen = USABLE_ROWS * PILES as usize;
    let start = if per_screen == 0 { 0 } else { SELECTED - SELECTED % per_screen };
    let usable = USABLE_ROWS;
    let gauge = GAUGE;
    let cols = COLS;

    let mut index = start;
    while index < LIST_LENGTH && row < usable as i32 {
        let thename = tail(&FILELIST[index]);
            /* 我们显示的文件名，去掉路径。 */
        let namelen = breadth(thename.as_bytes());
            /* 文件名的列宽。 */
        let mut infomaxlen: usize = 7;
            /* 文件信息的最大列宽：通常为七，对于“（父目录）”为十二。 */
        let dots = cols >= 15 && namelen >= gauge as usize - infomaxlen;
            /* 是否在文件名前放置省略号？当列数少于 15 时，我们不浪费空间。 */
        let disp = display_string(
            thename.as_bytes(),
            if dots {
                namelen + infomaxlen + 4 - gauge as usize
            } else {
                0
            },
            gauge as usize,
            false,
            false,
        );
            /* 以可显示格式显示的文件名（或片段）。当是片段时，
             * 计入省略号加一个空格的填充。 */

        /* 如果这是选中的项，提前绘制其高亮条，并记住其位置以便将光标放在上面。 */
        if index == SELECTED {
            wattron(midwin, interface_color_pair[SELECTED_TEXT]);
            mvwprintw(midwin, row, col, "%*s", gauge, " ");
            the_row = row;
            the_column = col;
        }

        /* 如果名称太长，我们显示类似“...ename”的内容。 */
        if dots {
            mvwaddstr(midwin, row, col, "...");
        }
        mvwaddstr(midwin, row, if dots { col + 3 } else { col }, disp.as_str());

        col += gauge;

        /* 显示关于文件的信息：“--”用于符号链接（除非它们指向目录）以及
         * 已消失的文件，对于目录显示“（dir）”，对于普通文件显示文件大小。 */
        let mut st = Stat::default();
        let mut info: String;
        if lstat(&FILELIST[index], &mut st) < 0 || s_islnk(st.st_mode) {
            let mut st2 = Stat::default();
            if stat(&FILELIST[index], &mut st2) < 0 || !s_isdir(st2.st_mode) {
                info = copy_of("--");
            } else {
                /* TRANSLATORS: 超过 7 个单元格的内容会被裁剪。 */
                info = copy_of(gettext!("(dir)"));
            }
        } else if s_isdir(st.st_mode) {
            if thename == ".." {
                /* TRANSLATORS: 超过 12 个单元格的内容会被裁剪。 */
                info = copy_of(gettext!("(parent dir)"));
                infomaxlen = 12;
            } else {
                info = copy_of(gettext!("(dir)"));
            }
        } else {
            let mut result = st.st_size;
            let modifier: char;

            /* 将文件大小转换为人类可读的形式。 */
            if st.st_size < (1 << 10) {
                modifier = ' '; /* 字节 */
            } else if st.st_size < (1 << 20) {
                result >>= 10;
                modifier = 'K'; /* 千字节 */
            } else if st.st_size < (1 << 30) {
                result >>= 20;
                modifier = 'M'; /* 兆字节 */
            } else {
                result >>= 30;
                modifier = 'G'; /* 吉字节 */
            }

            /* 如果小于一太字节，则显示大小，否则显示“（巨大）”。 */
            if result < (1 << 10) {
                info = format!("{:4} {}B", result, modifier);
            } else {
                /* TRANSLATORS: 超过 7 个单元格的内容会被裁剪。
                 * 如有必要，可以省略括号。 */
                info = copy_of(gettext!("(huge)"));
            }
        }

        /* 确保 info 占用的列数不超过 infomaxlen。 */
        let mut infolen = breadth(info.as_bytes());
        if infolen > infomaxlen {
            let idx = actual_x(info.as_bytes(), infomaxlen);
            info.remove(idx);
            infolen = infomaxlen;
        }

        mvwaddstr(midwin, row, col - infolen as i32, info.as_str());

        /* 如果是选中的项，则完成其高亮显示。 */
        if index == SELECTED {
            wattroff(midwin, interface_color_pair[SELECTED_TEXT]);
        }

        /* 在列之间添加一些空格。 */
        col += 2;

        /* 如果下一个项在当前行上放不下，则移动到下一行。 */
        if col > cols - gauge {
            row += 1;
            col = 0;
        }
        index += 1;
    }

    /* 如果请求，将光标放在选中的项上并打开它。 */
    if ISSET(SHOW_CURSOR) {
        wmove(midwin, the_row, the_column);
        curs_set(1);
    }

    wnoutrefresh(midwin);
}

/* 在文件列表中向前或向后查找给定的针。 */
pub unsafe fn findfile(needle: &str, forwards: bool) {
    let mut began_at = SELECTED;

    /* 遍历文件名的列表，直到找到匹配项，或者我们已经回到开始点。 */
    loop {
        if forwards {
            SELECTED += 1;
            if SELECTED == LIST_LENGTH {
                SELECTED = 0;
                statusbar(gettext!("Search Wrapped"));
            }
        } else {
            if SELECTED == 0 {
                SELECTED = LIST_LENGTH - 1;
                statusbar(gettext!("Search Wrapped"));
            } else {
                SELECTED -= 1;
            }
        }

        /* 当针出现在文件的基名中时，我们就有了一个匹配项。 */
        if mbstrcasestr(tail(&FILELIST[SELECTED]).as_bytes(), needle.as_bytes()).is_some() {
            if SELECTED == began_at {
                statusbar(gettext!("This is the only occurrence"));
            }
            return;
        }

        /* 当我们回到起点而没有匹配项时…… */
        if SELECTED == began_at {
            not_found_msg(needle);
            return;
        }
    }
}

/* 准备提示并询问用户要搜索什么；然后搜索它。
 * 如果 forwards 为 TRUE，则向前搜索；否则向后搜索。 */
pub unsafe fn search_filename(forwards: bool) {
    let thedefault: String;

    /* 如果之前搜索过，则在方括号中显示它。 */
    if let Some(ref ls) = last_search {
        if !ls.is_empty() {
            let disp = display_string(ls.as_bytes(), 0, (COLS as usize) / 3, false, false);
            let mut d = String::with_capacity(disp.len() + 7);
            d.push_str(" [");
            if breadth(ls.as_bytes()) > (COLS as usize) / 3 {
                d.push_str("...");
            }
            d.push_str(&disp);
            d.push(']');
            thedefault = d;
        } else {
            thedefault = copy_of("");
        }
    } else {
        thedefault = copy_of("");
    }

    /* 现在询问要搜索什么。 */
    let mut msg = String::from(gettext!("Search"));
    if !forwards {
        msg.push_str(gettext!(" [Backwards]"));
    }
    msg.push_str(&thedefault);

    let response = do_prompt(MWHEREISFILE, "", search_history, browser_refresh, &msg);

    /* 如果用户取消，或者在没有搜索内容的情况下键入 <Enter> 且本次会话中
     * 还没有搜索过，则退出。 */
    if response == -1
        || (response == -2 && last_search.as_ref().map_or(true, |s| s.is_empty()))
    {
        statusbar(gettext!("Cancelled"));
        return;
    }

    /* 如果用户输入了答案，则记住它。 */
    if let Some(ref a) = answer {
        if !a.is_empty() {
            last_search = Some(a.clone());
            update_history(&mut search_history, a, PRUNE_DUPLICATE);
        }
    }

    if response == 0 || response == -2 {
        if let Some(ref ls) = last_search {
            findfile(ls, forwards);
        }
    }
}

/* 在不提示的情况下重复上次给定的搜索字符串，向前或向后。 */
pub unsafe fn research_filename(forwards: bool) {
    /* 如果还没有搜索过，则从历史记录中取最后一项。 */
    if last_search.as_ref().map_or(true, |s| s.is_empty()) && !searchbot.is_null() {
        let prev = (*searchbot).prev;
        if !prev.is_null() {
            last_search = Some((*prev).data.clone());
        }
    }

    if last_search.as_ref().map_or(true, |s| s.is_empty()) {
        statusbar(gettext!("No current search pattern"));
    } else {
        wipe_statusbar();
        if let Some(ref ls) = last_search {
            findfile(ls, forwards);
        }
    }
}

/* 选择列表中的第一个文件 —— 由 ^W^Y 直接调用。 */
pub unsafe fn to_first_file() {
    SELECTED = 0;
}

/* 选择列表中的最后一个文件 —— 由 ^W^V 直接调用。 */
pub unsafe fn to_last_file() {
    SELECTED = LIST_LENGTH - 1;
}

/* 供 interpret 比较用的安全包装（对应 C 的 functionptrtype 比较）。 */
fn safe_to_first_file() {
    unsafe { to_first_file() };
}
fn safe_to_last_file() {
    unsafe { to_last_file() };
}
fn safe_do_enter() {
    unsafe { do_enter() };
}

/* 从路径中剥离最后一个元素，并返回剥离后的路径。
 * 返回的字符串是动态分配的，应该被释放。 */
pub fn strip_last_component(path: &str) -> String {
    let mut copy = copy_of(path);
    if let Some(pos) = copy.rfind('/') {
        copy.replace_range(pos.., "");
    }
    copy
}

/* 允许用户在文件系统中浏览，从给定的路径开始。 */
pub unsafe fn browse(initial: &str) -> Option<String> {
    let mut path: Option<String> = Some(initial.to_string());
        /* 当前正在显示的目录。 */
    let mut present_name: Option<String> = None;
        /* 当前选中文件的名称，或者在我们备份到“..”之前的目录。 */
    let mut old_selected: usize;
        /* 当前选中文件之前的选中文件的数量。 */
    let mut chosen: Option<String> = None;
        /* 用户选择的文件名，如果没有则选 NULL。 */

    'outer: loop {
        /* 当用户刷新或选择一个新目录时，会回到这里。 */

        path = get_full_path(path.as_deref().unwrap_or(""));

        let dir = if path.is_some() {
            opendir(path.as_deref().unwrap())
        } else {
            None
        };

        if path.is_none() || dir.is_none() {
            statusline(
                message_type::ALERT,
                &format!("Cannot open directory: {}", strerror(errno())),
            );
            /* 如果没有文件列表，则没有可显示的内容。 */
            if FILELIST.is_empty() {
                lastmessage = message_type::VACUUM;
                napms(1200);
                return None;
            }
            path = present_path.clone();
            present_name = Some(FILELIST[SELECTED].clone());
        }

        if let Some(d) = dir {
            /* 获取文件列表，并在此过程中设置 gauge 和 piles。 */
            read_the_list(path.as_deref().unwrap(), d);
            closedir(d);
        }

        resized_for_browser = false;

        /* 如果之前选择了某项，则重新选择它；否则，仅选择第一项（..）。 */
        if let Some(name) = present_name.take() {
            reselect(&name);
        } else {
            SELECTED = 0;
        }

        old_selected = usize::MAX;

        present_path = path.clone();

        titlebar(path.as_deref());

        if LIST_LENGTH == 0 {
            statusline(message_type::ALERT, gettext!("No entries"));
            napms(1200);
        } else {
            'inner: loop {
                let mut kbinput = get_kbinput(midwin, ISSET(SHOW_CURSOR));

                /* 当用户点击文件列表时，选择一个文件名。 */
                if kbinput == KEY_MOUSE {
                    let mut mouse_x: i32 = 0;
                    let mut mouse_y: i32 = 0;

                    if get_mouseinput(&mut mouse_y, &mut mouse_x) == 0
                        && wmouse_trafo(midwin, &mut mouse_y, &mut mouse_x, false)
                    {
                        let per_screen = (USABLE_ROWS * PILES as usize) as i32;
                        SELECTED = (SELECTED - SELECTED % per_screen as usize)
                            + (mouse_y * PILES) as usize
                            + (mouse_x / (GAUGE + 2)) as usize;

                        /* 如果超出行尾，选择前一个文件名。 */
                        if mouse_x > PILES * (GAUGE + 2) {
                            SELECTED = SELECTED.wrapping_sub(1);
                        }

                        /* 如果超出列表末尾，选择最后一个文件名。 */
                        if SELECTED > LIST_LENGTH - 1 {
                            SELECTED = LIST_LENGTH - 1;
                        }

                        /* 如果第二次点击文件名，则选择它。 */
                        if old_selected == SELECTED {
                            kbinput = KEY_ENTER;
                        }
                    }

                    if kbinput == KEY_MOUSE {
                        continue;
                    }
                }

                let function: functionptrtype = interpret(kbinput);

                if function == Some(do_help) {
                    do_help();
                } else if function == Some(full_refresh) {
                    kbinput = THE_WINDOW_RESIZED;
                } else if function == Some(do_toggle)
                    && !get_shortcut(kbinput).is_null()
                    && unsafe { (*get_shortcut(kbinput)).toggle } == NO_HELP as i32
                {
                    TOGGLE(NO_HELP);
                    window_init();
                    kbinput = THE_WINDOW_RESIZED;
                } else if function == Some(do_search_backward) {
                    search_filename(BACKWARD);
                } else if function == Some(do_search_forward) {
                    search_filename(FORWARD);
                } else if function == Some(do_findprevious) {
                    research_filename(BACKWARD);
                } else if function == Some(do_findnext) {
                    research_filename(FORWARD);
                } else if function == Some(do_left) {
                    if SELECTED > 0 {
                        SELECTED -= 1;
                    }
                } else if function == Some(do_right) {
                    if SELECTED + 1 < LIST_LENGTH {
                        SELECTED += 1;
                    }
                } else if function == Some(to_prev_word) {
                    SELECTED -= SELECTED % PILES as usize;
                } else if function == Some(to_next_word) {
                    SELECTED += PILES as usize - 1 - (SELECTED % PILES as usize);
                    if SELECTED >= LIST_LENGTH {
                        SELECTED = LIST_LENGTH - 1;
                    }
                } else if function == Some(do_up) {
                    if SELECTED >= PILES as usize {
                        SELECTED -= PILES as usize;
                    }
                } else if function == Some(do_down) {
                    if SELECTED + PILES as usize <= LIST_LENGTH - 1 {
                        SELECTED += PILES as usize;
                    }
                } else if function == Some(to_prev_block) {
                    SELECTED = ((SELECTED / (USABLE_ROWS * PILES as usize)) * USABLE_ROWS * PILES as usize)
                        + SELECTED % PILES as usize;
                } else if function == Some(to_next_block) {
                    SELECTED = ((SELECTED / (USABLE_ROWS * PILES as usize)) * USABLE_ROWS * PILES as usize)
                        + SELECTED % PILES as usize
                        + USABLE_ROWS * PILES as usize
                        - PILES as usize;
                    if SELECTED >= LIST_LENGTH {
                        SELECTED = (LIST_LENGTH / PILES as usize) * PILES as usize
                            + SELECTED % PILES as usize;
                    }
                    if SELECTED >= LIST_LENGTH {
                        SELECTED -= PILES as usize;
                    }
                } else if function == Some(do_page_up) {
                    if SELECTED < PILES as usize {
                        SELECTED = 0;
                    } else if SELECTED < USABLE_ROWS * PILES as usize {
                        SELECTED = SELECTED % PILES as usize;
                    } else {
                        SELECTED -= USABLE_ROWS * PILES as usize;
                    }
                } else if function == Some(do_page_down) {
                    if SELECTED + PILES as usize >= LIST_LENGTH - 1 {
                        SELECTED = LIST_LENGTH - 1;
                    } else if SELECTED + USABLE_ROWS * PILES as usize >= LIST_LENGTH {
                        SELECTED = (SELECTED + USABLE_ROWS * PILES as usize - LIST_LENGTH)
                            % PILES as usize
                            + LIST_LENGTH
                            - PILES as usize;
                    } else {
                        SELECTED += USABLE_ROWS * PILES as usize;
                    }
                } else if function == Some(safe_to_first_file) {
                    to_first_file();
                } else if function == Some(safe_to_last_file) {
                    to_last_file();
                } else if function == Some(goto_dir) {
                    /* 询问要转到的目录。 */
                    if do_prompt(
                        MGOTODIR,
                        "",
                        std::ptr::null_mut(),
                        browser_refresh,
                        gettext!("Go To Directory"),
                    ) < 0
                    {
                        statusbar(gettext!("Cancelled"));
                        break;
                    }

                    let ans = answer.clone().unwrap_or_default();
                    path = Some(expand_leading_tilde(&ans));

                    /* 如果给定的路径是相对的，则将其与当前路径连接。 */
                    if !path.as_deref().unwrap_or("").starts_with('/') {
                        let pp = present_path.clone().unwrap_or_default();
                        path = Some(format!("{}{}", pp, ans));
                    }

                    if let Some(ref od) = operating_dir {
                        if outside_of_confinement(path.as_deref().unwrap_or(""), false) {
                            /* TRANSLATORS: 这指的是 --operatingdir 选项的
                             * 限制效果，而不是 --restricted。 */
                            statusline(
                                message_type::ALERT,
                                &format!("Can't go outside of {}", od),
                            );
                            path = present_path.clone();
                            break;
                        }
                    }

                    /* 去掉任何尾随斜杠，以便可以比较名称。 */
                    while path.as_ref().map_or(0, |p| p.len()) > 1
                        && path.as_deref().unwrap_or("").ends_with('/')
                    {
                        path.as_mut().unwrap().pop();
                    }

                    /* 如果指定的目录无法进入，则选中它（如果它在当前列表中）
                     * 以便突出显示。 */
                    for j in 0..LIST_LENGTH {
                        if FILELIST[j] == path.as_deref().unwrap_or("") {
                            SELECTED = j;
                        }
                    }

                    /* 尝试打开并读取指定的目录。 */
                    continue 'outer;
                } else if function == Some(safe_do_enter) {
                    /* 无法从根目录向上移动。 */
                    if FILELIST[SELECTED] == "/.." {
                        statusline(message_type::ALERT, gettext!("Can't move up a directory"));
                        continue;
                    }

                    if let Some(ref od) = operating_dir {
                        if outside_of_confinement(&FILELIST[SELECTED], false) {
                            statusline(
                                message_type::ALERT,
                                &format!("Can't go outside of {}", od),
                            );
                            continue;
                        }
                    }

                    /* 如果由于某种原因文件不可访问，则抱怨。 */
                    let mut st = Stat::default();
                    if stat(&FILELIST[SELECTED], &mut st) < 0 {
                        statusline(
                            message_type::ALERT,
                            &format!(
                                "Error reading {}: {}",
                                FILELIST[SELECTED],
                                strerror(errno())
                            ),
                        );
                        continue;
                    }

                    /* 如果不是目录，则选中了一个文件 —— 我们完成了。 */
                    if !s_isdir(st.st_mode) {
                        chosen = Some(FILELIST[SELECTED].clone());
                        break;
                    }

                    /* 如果我们正在向上移动一个级别，请记住我们来自哪里，以便
                     * 此目录可以被高亮显示并轻松重新进入。 */
                    if tail(&FILELIST[SELECTED]) == ".." {
                        present_name = Some(strip_last_component(&FILELIST[SELECTED]));
                    }

                    /* 尝试打开并读取所选目录。 */
                    path = Some(FILELIST[SELECTED].clone());
                    continue 'outer;
                } else if function == Some(implant) {
                    implant();
                } else if kbinput == START_OF_PASTE {
                    while get_kbinput(midwin, BLIND) != END_OF_PASTE {
                        /* 忽略粘贴内容。 */
                    }
                    statusline(message_type::AHEM, gettext!("Paste is ignored"));
                } else if kbinput == THE_WINDOW_RESIZED {
                    /* 在下方处理。 */
                } else if function == Some(do_exit) {
                    break;
                } else {
                    unbound_key(kbinput);
                }

                /* 如果终端已调整大小（或可能已调整大小），则刷新文件列表。 */
                if kbinput == THE_WINDOW_RESIZED || resized_for_browser {
                    /* 记住选中的文件，以便能够重新选择它。 */
                    present_name = Some(FILELIST[SELECTED].clone());
                    continue 'outer;
                }
            }
        }

        titlebar(None);
        edit_refresh();

        FILELIST.clear();
        LIST_LENGTH = 0;

        return chosen;
    }
}

/* 准备开始浏览。如果给定的路径有目录部分，则从该目录开始浏览，
 * 否则从当前目录开始浏览。 */
pub unsafe fn browse_in(inpath: &str) -> Option<String> {
    let mut path = expand_leading_tilde(inpath);
    let mut st = Stat::default();

    /* 如果路径不是目录，请尝试从其中剥离文件名；如果仍然不是目录，
     * 则使用当前工作目录。 */
    if stat(&path, &mut st) < 0 || !s_isdir(st.st_mode) {
        path = strip_last_component(&path);

        if stat(&path, &mut st) < 0 || !s_isdir(st.st_mode) {
            path = realpath(".").unwrap_or_default();

            if path.is_empty() {
                statusline(
                    message_type::ALERT,
                    gettext!("The working directory has disappeared"),
                );
                napms(1200);
                return None;
            }
        }
    }

    if let Some(ref od) = operating_dir {
        if outside_of_confinement(&path, false) {
            path = od.clone();
        }
    }

    browse(&path)
}
