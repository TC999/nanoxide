/**************************************************************************
 * browser.rs  --  GNU nano 文件浏览器（对应 browser.c）
 * 版权 (C) 2001-2011, 2013-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 文件浏览器：列目录、选择文件、搜索文件名、进入目录。
//!
//! 转换说明：
//! - `readdir`/`opendir` → `std::fs::read_dir`；`qsort` → `sort_by`；
//! - `filelist`/`list_length`/`selected` 等静态放入 [`GlobalState`]；
//! - 渲染由 `browser_refresh` 直接写入终端（crossterm）。

use crate::definitions::*;
use crate::files;
use crate::global;
use crate::history;
use crate::prompt;
use crate::utils;
use crate::winio;
use std::io::Write;

fn get_selected() -> usize {
    with_global(|g| g.selected)
}

fn set_selected(v: usize) {
    with_global_mut(|g| g.selected = v);
}

fn get_gauge() -> i32 {
    with_global(|g| g.gauge)
}

fn set_gauge(v: i32) {
    with_global_mut(|g| g.gauge = v);
}

fn get_piles() -> i32 {
    with_global(|g| g.piles)
}

fn set_piles(v: i32) {
    with_global_mut(|g| g.piles = v);
}

fn get_list_length() -> usize {
    with_global(|g| g.list_length)
}

fn get_filelist() -> Vec<String> {
    with_global(|g| g.filelist.clone())
}

fn set_filelist(v: Vec<String>) {
    with_global_mut(|g| {
        g.list_length = v.len();
        g.filelist = v;
    });
}

fn get_usable_rows() -> usize {
    with_global(|g| g.usable_rows)
}

// ======================== 列表读取（对应 read_the_list） ========================

/// 用给定目录中的文件名填充 filelist，设置 list_length、gauge、piles，
/// 并排序（对应 `read_the_list`）。
fn read_the_list(path: &str) {
    let cols = with_global(|g| g.COLS);
    let mut entries: Vec<String> = Vec::new();
    let mut widest = 0;

    /* 找出当前文件夹中最宽文件名的宽度。 */
    if let Ok(rd) = std::fs::read_dir(path) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let span = utils::breadth(name.as_bytes());
            if span > widest {
                widest = span;
            }
            entries.push(name);
        }
    }

    /* 为空白和文件大小预留十列。 */
    let mut gauge = widest + 10;

    /* 需要时为 ".. (parent dir)" 腾出空间。 */
    if gauge < 15 {
        gauge = 15;
    }
    /* 确保不宽于窗口。 */
    if gauge > cols {
        gauge = cols;
    }

    /* 构造完整路径列表（跳过 "."）。 */
    let mut filelist: Vec<String> = Vec::new();
    for name in &entries {
        if name == "." {
            continue;
        }
        filelist.push(format!("{}{}", path, name));
    }

    /* 排序。 */
    filelist.sort_by(|a, b| {
        let ta = utils::tail(a);
        let tb = utils::tail(b);
        ta.to_lowercase().cmp(&tb.to_lowercase())
    });

    let list_length = filelist.len();

    /* 计算一行能放多少文件。 */
    let piles = (cols as i32 + 2) / (gauge as i32 + 2);

    let (editwinrows, lines, zero) = with_global(|g| {
        (g.editwinrows, g.LINES, ISSET(ZERO))
    });
    let usable_rows = (editwinrows - if zero && lines > 1 { 1 } else { 0 }) as usize;

    set_gauge(gauge as i32);
    set_piles(piles);
    set_filelist(filelist);
    with_global_mut(|g| {
        g.usable_rows = usable_rows;
        let _ = list_length;
    });
}

/// 若给定文件或目录名仍存在，重新选择它（对应 `reselect`）。
fn reselect(name: &str) {
    let list = get_filelist();
    let mut looking_at = 0;

    while looking_at < list.len() && list[looking_at] != name {
        looking_at += 1;
    }

    /* 找到则选择；否则移动高亮使变化被注意，但保持在当前范围。 */
    let selected = get_selected();
    if looking_at < list.len() {
        set_selected(looking_at);
    } else if selected > list.len() {
        set_selected(list.len().saturating_sub(1));
    } else {
        set_selected(selected.saturating_sub(1));
    }
}

// ======================== 浏览器渲染（对应 browser_refresh） ========================

/// 从 filelist 显示最多一屏文件名（对应 `browser_refresh`）。
pub fn browser_refresh() {
    let present_path = with_global(|g| g.present_path.clone());
    winio::titlebar(present_path.as_deref());

    let (cols, editwinrows, zero, lines) = with_global(|g| {
        (g.COLS, g.editwinrows, ISSET(ZERO), g.LINES)
    });
    let usable_rows = (editwinrows - if zero && lines > 1 { 1 } else { 0 }) as usize;
    let piles = get_piles() as usize;
    let gauge = get_gauge() as usize;
    let list = get_filelist();
    let selected = get_selected();

    let mut row = 0;
    let mut col = 0;
    let mut the_row = 0;
    let mut the_column = 0;

    let mut stdout = std::io::stdout();
    let start_index = selected - selected % (usable_rows * piles.max(1));

    let mut index = start_index;
    while index < list.len() && row < usable_rows {
        let thename = utils::tail(&list[index]).to_string();
        let namelen = utils::breadth(thename.as_bytes());
        let infomaxlen = 7;

        /* 检查文件类型与大小。 */
        let mut info = String::new();
        let mut infomaxlen = infomaxlen;
        if let Ok(meta) = std::fs::metadata(&list[index]) {
            if meta.is_dir() {
                if thename == ".." {
                    info = "(parent dir)".to_string();
                    infomaxlen = 12;
                } else {
                    info = "(dir)".to_string();
                }
            } else {
                let size = meta.len();
                if size < (1 << 10) {
                    info = format!("{:4} B", size);
                } else if size < (1 << 20) {
                    info = format!("{:4} KB", size >> 10);
                } else if size < (1 << 30) {
                    info = format!("{:4} MB", size >> 20);
                } else {
                    info = format!("{:4} GB", size >> 30);
                }
            }
        } else {
            info = "--".to_string();
        }

        /* 截断 info 到 infomaxlen。 */
        let info_trunc = {
            let actual = utils::actual_x(info.as_bytes(), infomaxlen);
            info[..actual].to_string()
        };
        let infolen = info_trunc.len();

        /* 若这是选中项，先绘制高亮条。 */
        if index == selected {
            let _ = execute_at(&mut stdout, row, col, &format!("{:width$}", "", width = gauge));
            the_row = row;
            the_column = col;
        }

        /* 名称太长时显示 "...ename"。 */
        let dots = cols >= 15 && namelen >= gauge.saturating_sub(infomaxlen);
        let mut display = String::new();
        if dots {
            let skip = namelen + infomaxlen + 4 - gauge;
            let start = utils::actual_x(thename.as_bytes(), skip);
            display = format!("...{}", &thename[start..]);
        } else {
            display = thename.clone();
        }

        let _ = execute_at(&mut stdout, row, col, &display);
        col += gauge;

        /* 在右侧显示文件信息。 */
        let _ = execute_at(&mut stdout, row, col.saturating_sub(infolen), &info_trunc);

        /* 列间加空格。 */
        col += 2;

        /* 若下一项放不下这一行，换行。 */
        if col > cols.saturating_sub(gauge) {
            row += 1;
            col = 0;
        }

        index += 1;
    }

    let _ = stdout.flush();
    let _ = (the_row, the_column);
}

/// 在 (row, col) 处写文本（crossterm 辅助）。
fn execute_at(stdout: &mut std::io::Stdout, row: usize, col: usize, text: &str) -> std::io::Result<()> {
    use crossterm::cursor::MoveTo;
    use crossterm::execute;
    execute!(stdout, MoveTo(col as u16, row as u16))?;
    write!(stdout, "{}", text)
}

// ======================== 文件名搜索（对应 findfile / search_filename / research_filename） ========================

/// 在文件列表中前后查找给定 needle（对应 `findfile`）。
fn findfile(needle: &str, forwards: bool) {
    let began_at = get_selected();
    let list_len = get_list_length();

    loop {
        let mut selected = get_selected();
        if forwards {
            if selected + 1 == list_len {
                set_selected(0);
                winio::statusbar(&crate::t!("browser-search_wrapped"));
            } else {
                set_selected(selected + 1);
            }
        } else {
            if selected == 0 {
                set_selected(list_len.saturating_sub(1));
                winio::statusbar(&crate::t!("browser-search_wrapped"));
            } else {
                set_selected(selected - 1);
            }
        }
        selected = get_selected();

        /* 当 needle 出现在文件基本名中，即匹配。 */
        let list = get_filelist();
        if let Some(t) = list.get(selected) {
            let basename = utils::tail(t);
            if crate::chars::mbstrcasestr(basename.as_bytes(), needle.as_bytes()).is_some() {
                if selected == began_at {
                    winio::statusbar(&crate::t!("browser-only_occurrence"));
                }
                return;
            }
        }

        /* 回到起点而无匹配时。 */
        if selected == began_at {
            winio::statusline(MessageType::Ahem, &crate::t!("browser-not_found", needle = needle));
            return;
        }
    }
}

/// 准备提示并询问要搜索什么（对应 `search_filename`）。
fn search_filename(forwards: bool) {
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();

    /* 若之前搜索过，显示在方括号中。 */
    let thedefault = if !last_search.is_empty() {
        let cols = with_global(|g| g.COLS);
        let disp = winio::display_string(last_search.as_bytes(), 0, cols / 3, false, false);
        let dots = utils::breadth(last_search.as_bytes()) > cols / 3;
        format!(" [{}{}]", disp, if dots { "..." } else { "" })
    } else {
        String::new()
    };

    let search_msg = crate::t!("browser-search");
    let msg = if !forwards {
        format!("{} [{}]{}", search_msg, crate::t!("browser-backwards"), thedefault)
    } else {
        format!("{}{}", search_msg, thedefault)
    };

    let mut search_history = with_global(|g| g.search_history.clone()).unwrap_or_else(|| make_new_node(None));
    let response = prompt::do_prompt(
        MWHEREISFILE,
        "",
        Some(&mut search_history),
        Some(browser_refresh),
        &msg,
    );
    with_global_mut(|g| g.search_history = Some(search_history));

    /* 用户取消，或空白回答且本次会话未搜索过时，退出。 */
    if response == -1 || (response == -2 && last_search.is_empty()) {
        winio::statusbar(&crate::t!("browser-cancelled"));
        return;
    }

    /* 若用户输入了回答，记住它。 */
    let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
    if !answer.is_empty() {
        with_global_mut(|g| g.last_search = Some(answer.clone()));
        let mut sh = with_global(|g| g.search_history.clone()).unwrap_or_else(|| make_new_node(None));
        history::update_history(&mut sh, &answer, true);
        with_global_mut(|g| g.search_history = Some(sh));
    }

    if response == 0 || response == -2 {
        let ls = with_global(|g| g.last_search.clone()).unwrap_or_default();
        findfile(&ls, forwards);
    }
}

/// 不提示地再次搜索最后给出的字符串（对应 `research_filename`）。
fn research_filename(forwards: bool) {
    let last_search = with_global(|g| g.last_search.clone()).unwrap_or_default();

    if last_search.is_empty() {
        winio::statusbar(&crate::t!("browser-no_search_pattern"));
    } else {
        winio::wipe_statusbar();
        findfile(&last_search, forwards);
    }
}

/// 选择列表中的第一个文件（对应 `to_first_file`）。
pub fn to_first_file() {
    set_selected(0);
}

/// 选择列表中的最后一个文件（对应 `to_last_file`）。
pub fn to_last_file() {
    set_selected(get_list_length().saturating_sub(1));
}

/// 从 path 末尾移除一个元素并返回（对应 `strip_last_component`）。
pub fn strip_last_component(path: &str) -> String {
    match path.rfind('/') {
        Some(slash) => path[..slash].to_string(),
        None => path.to_string(),
    }
}

// ======================== 浏览主循环（对应 browse / browse_in） ========================

/// 允许用户在文件系统中浏览目录，从给定路径开始（对应 `browse`）。
pub fn browse(path: &str) -> Option<String> {
    let mut present_name: Option<String> = None;
    let mut old_selected: Option<usize> = None;
    let mut chosen: Option<String> = None;

    let mut path = path.to_string();

    'read_directory_contents: loop {
        /* 规范化路径。 */
        if let Some(fp) = files::get_full_path(&path) {
            path = fp;
        }

        let dir_ok = std::fs::read_dir(&path).is_ok();

        if !dir_ok {
            winio::statusline(MessageType::Alert, &crate::t!("browser-cannot_open_dir", path = path));
            let filelist_exists = !get_filelist().is_empty();
            if !filelist_exists {
                with_global_mut(|g| g.lastmessage = MessageType::Vacuum);
                present_name = None;
                winio::napms(1200);
                return None;
            }
            let pp = with_global(|g| g.present_path.clone()).unwrap_or_default();
            path = pp;
            let sel = get_selected();
            present_name = get_filelist().get(sel).cloned();
        }

        if dir_ok {
            read_the_list(&path);
        }

        /* 若之前选择了某物，重新选择它；否则选择第一项。 */
        match present_name.take() {
            Some(name) => reselect(&name),
            None => set_selected(0),
        }

        old_selected = None;
        with_global_mut(|g| g.present_path = Some(path.clone()));

        winio::titlebar(Some(&path));

        let list_len = get_list_length();
        if list_len == 0 {
            winio::statusline(MessageType::Alert, &crate::t!("browser-no_entries"));
            winio::napms(1200);
        } else {
            loop {
                with_global_mut(|g| g.lastmessage = MessageType::Vacuum);
                winio::bottombars(with_global(|g| g.currmenu));

                /* 列表本身或选中文件变化时显示列表。 */
                let selected = get_selected();
                let show_cursor = with_global(|_g| ISSET(SHOW_CURSOR));
                if old_selected != Some(selected) || show_cursor {
                    browser_refresh();
                }
                old_selected = Some(selected);

                let kbinput = winio::get_kbinput();
                let function = global::interpret(kbinput);

                match function {
                    Some(FunctionId::DoHelp) => crate::help::do_help(),
                    Some(FunctionId::DoFullRefresh) => {
                        let _ = kbinput;
                    }
                    Some(FunctionId::DoSearchBackward) => search_filename(false),
                    Some(FunctionId::DoSearchForward) => search_filename(true),
                    Some(FunctionId::DoFindPrevious) => research_filename(false),
                    Some(FunctionId::DoFindNext) => research_filename(true),
                    Some(FunctionId::DoLeft) => {
                        if get_selected() > 0 {
                            set_selected(get_selected() - 1);
                        }
                    }
                    Some(FunctionId::DoRight) => {
                        if get_selected() < get_list_length().saturating_sub(1) {
                            set_selected(get_selected() + 1);
                        }
                    }
                    Some(FunctionId::DoPrevWord) => {
                        let piles = get_piles().max(1) as usize;
                        set_selected(get_selected() - (get_selected() % piles));
                    }
                    Some(FunctionId::DoNextWord) => {
                        let piles = get_piles().max(1) as usize;
                        let sel = get_selected() + piles - 1 - (get_selected() % piles);
                        set_selected(if sel >= get_list_length() { get_list_length().saturating_sub(1) } else { sel });
                    }
                    Some(FunctionId::DoUp) => {
                        let piles = get_piles().max(1) as usize;
                        if get_selected() >= piles {
                            set_selected(get_selected() - piles);
                        }
                    }
                    Some(FunctionId::DoDown) => {
                        let piles = get_piles().max(1) as usize;
                        if get_selected() + piles <= get_list_length().saturating_sub(1) {
                            set_selected(get_selected() + piles);
                        }
                    }
                    Some(FunctionId::DoPrevBlock) => {
                        let piles = get_piles().max(1) as usize;
                        let block = get_usable_rows() * piles;
                        let sel = (get_selected() / block) * block + get_selected() % piles;
                        set_selected(sel);
                    }
                    Some(FunctionId::DoNextBlock) => {
                        let piles = get_piles().max(1) as usize;
                        let block = get_usable_rows() * piles;
                        let mut sel = (get_selected() / block) * block + get_selected() % piles
                            + block - piles;
                        if sel >= get_list_length() {
                            sel = (get_list_length() / piles) * piles + get_selected() % piles;
                        }
                        if sel >= get_list_length() {
                            sel = sel.saturating_sub(piles);
                        }
                        set_selected(sel);
                    }
                    Some(FunctionId::DoPageUp) => {
                        let piles = get_piles().max(1) as usize;
                        let block = get_usable_rows() * piles;
                        let sel = get_selected();
                        if sel < piles {
                            set_selected(0);
                        } else if sel < block {
                            set_selected(sel % piles);
                        } else {
                            set_selected(sel - block);
                        }
                    }
                    Some(FunctionId::DoPageDown) => {
                        let piles = get_piles().max(1) as usize;
                        let block = get_usable_rows() * piles;
                        let sel = get_selected();
                        let list_len = get_list_length();
                        if sel + piles >= list_len.saturating_sub(1) {
                            set_selected(list_len.saturating_sub(1));
                        } else if sel + block >= list_len {
                            set_selected((sel + block - list_len) % piles + list_len - piles);
                        } else {
                            set_selected(sel + block);
                        }
                    }
                    Some(FunctionId::ToFirstFile) | Some(FunctionId::ToLastFile) => {
                        crate::prompt::run_function(function.unwrap());
                    }
                    Some(FunctionId::DoGotoDir) => {
                        /* 询问要去的目录。 */
                        let response = prompt::do_prompt(
                            MGOTODIR,
                            "",
                            None,
                            Some(browser_refresh),
                            &crate::t!("browser-go_to_dir"),
                        );
                        if response < 0 {
                            winio::statusbar(&crate::t!("browser-cancelled"));
                            continue;
                        }
                        let answer = with_global(|g| g.answer.clone()).unwrap_or_default();
                        let mut newpath = files::expand_leading_tilde(&answer);

                        /* 相对路径时与当前路径连接。 */
                        if !newpath.starts_with('/') {
                            let pp = with_global(|g| g.present_path.clone()).unwrap_or_default();
                            newpath = format!("{}{}", pp, answer);
                        }

                        /* 去掉尾部斜杠。 */
                        while newpath.len() > 1 && newpath.ends_with('/') {
                            newpath.pop();
                        }

                        /* 若指定目录无法进入，在列表中选中它。 */
                        let list = get_filelist();
                        for (j, item) in list.iter().enumerate() {
                            if *item == newpath {
                                set_selected(j);
                            }
                        }

                        /* 尝试打开并读取指定目录。 */
                        path = newpath;
                        continue 'read_directory_contents;
                    }
                    Some(FunctionId::DoEnter) => {
                        let sel = get_selected();
                        let list = get_filelist();
                        let Some(item) = list.get(sel).cloned() else {
                            continue;
                        };

                        /* 无法从根目录向上移动。 */
                        if item == "/.." {
                            winio::statusline(MessageType::Alert, &crate::t!("browser-cannot_go_up"));
                            continue;
                        }

                        /* 文件不可访问时抱怨。 */
                        let Ok(meta) = std::fs::metadata(&item) else {
                            winio::statusline(MessageType::Alert, &crate::t!("browser-error_reading", item = item));
                            continue;
                        };

                        /* 不是目录时，选中了文件——完成。 */
                        if !meta.is_dir() {
                            chosen = Some(item);
                            break;
                        }

                        /* 上移一级时，记住来源目录。 */
                        if utils::tail(&item) == ".." {
                            present_name = Some(strip_last_component(&item));
                        }

                        /* 尝试打开并读取选中目录。 */
                        path = item;
                        continue 'read_directory_contents;
                    }
                    Some(FunctionId::DoExit) => break,
                    _ => global::unbound_key(kbinput),
                }
            }
        }

        break;
    }

    winio::titlebar(None);
    winio::edit_refresh();

    chosen
}

/// 准备开始浏览。给定路径有目录部分时在其中浏览，
/// 否则在当前目录浏览（对应 `browse_in`）。
pub fn browse_in(inpath: &str) -> Option<String> {
    let mut path = files::expand_leading_tilde(inpath);
    let Ok(meta) = std::fs::metadata(&path) else {
        path = strip_last_component(&path);
        let Ok(_meta) = std::fs::metadata(&path) else {
            match std::fs::canonicalize(".") {
                Ok(cwd) => {
                    path = cwd.to_string_lossy().into_owned();
                }
                Err(_) => {
                    winio::statusline(MessageType::Alert, &crate::t!("browser-dir_disappeared"));
                    winio::napms(1200);
                    return None;
                }
            }
            return browse(&path);
        };
        return browse(&path);
    };
    if !meta.is_dir() {
        path = strip_last_component(&path);
        if let Ok(m) = std::fs::metadata(&path) {
            if !m.is_dir() {
                match std::fs::canonicalize(".") {
                    Ok(cwd) => path = cwd.to_string_lossy().into_owned(),
                    Err(_) => {
                        winio::statusline(MessageType::Alert, &crate::t!("browser-dir_disappeared"));
                        winio::napms(1200);
                        return None;
                    }
                }
            }
        } else {
            match std::fs::canonicalize(".") {
                Ok(cwd) => path = cwd.to_string_lossy().into_owned(),
                Err(_) => {
                    winio::statusline(MessageType::Alert, &crate::t!("browser-dir_disappeared"));
                    winio::napms(1200);
                    return None;
                }
            }
        }
    }

    browse(&path)
}