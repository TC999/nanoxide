/**************************************************************************
 * utils.rs  --  GNU nano 通用工具函数（对应 utils.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 通用工具函数，完整移植自 `utils.c`。
//!
//! 转换说明：
//! - 返回指针位置的函数返回 `Option<usize>`/元组而非裸指针；
//! - 链表节点用 `Rc<RefCell<LineStruct>>`，行号比较/指针相等用 `Rc::ptr_eq`；
//! - `strstrwrapper` 中的 POSIX 正则用 [`MatchPattern`]（glob/字面匹配）替代，
//!   反向正则搜索算法逐句对应原 `regexec` 循环；
//! - `getpwuid` 调用封装在 [`get_home_from_passwd`] 中（内部使用 `unsafe`
//!   调用 `libc`，对外仅返回 `Option<String>`）。

use crate::definitions::*;
use crate::chars;
use std::rc::Rc;

// ======================== 主目录 ========================

/// 设置全局变量 `homedir` 为用户的 home 目录（对应 `get_homedir`）。
/// 先尝试 $HOME，否则查询当前 UID 的密码数据库。
pub fn get_homedir() {
    with_global_mut(|g| {
        if g.homedir.is_none() {
            if let Ok(homenv) = std::env::var("HOME") {
                /* 仅当能确定 home 目录时才设置 `homedir`。 */
                if !homenv.is_empty() {
                    g.homedir = Some(homenv);
                    return;
                }
            }

            /* $HOME 未设置或为空时，尝试密码数据库。 */
            #[cfg(unix)]
            {
                if let Some(home) = get_home_from_passwd() {
                    g.homedir = Some(home);
                }
            }
            #[cfg(not(unix))]
            {
                if let Ok(home) = std::env::var("USERPROFILE") {
                    if !home.is_empty() {
                        g.homedir = Some(home);
                    }
                }
            }
        }
    });
}

/// 安全封装：根据当前 UID 查询密码数据库中的主目录。
/// （内部使用 `unsafe` 调用 `libc::geteuid`/`libc::getpwuid`，
/// 对外提供安全接口 `Option<String>`。）
#[cfg(unix)]
fn get_home_from_passwd() -> Option<String> {
    let pw = unsafe { libc::getpwuid(unsafe { libc::geteuid() }) };
    if pw.is_null() {
        return None;
    }
    let dir = unsafe { (*pw).pw_dir };
    if dir.is_null() {
        return None;
    }
    let home = unsafe { std::ffi::CStr::from_ptr(dir) }
        .to_string_lossy()
        .into_owned();
    if home.is_empty() {
        None
    } else {
        Some(home)
    }
}

/// 非 Unix 平台的无操作版本。
#[cfg(not(unix))]
fn get_home_from_passwd() -> Option<String> {
    None
}

// ======================== 路径与字符串 ========================

/// 返回给定路径的文件名部分（对应 `tail`）。
pub fn tail(path: &str) -> &str {
    match path.rfind('/') {
        None => path,
        Some(slash) => &path[slash + 1..],
    }
}

/// 返回两个给定字符串拼接后的拷贝（对应 `concatenate`）。
pub fn concatenate(path: &str, name: &str) -> String {
    format!("{}{}", path, name)
}

/// 返回给定整数 n 占用的位数（对应 `digits`）。
pub fn digits(n: isize) -> i32 {
    if n < 100000 {
        if n < 1000 {
            if n < 100 {
                2
            } else {
                3
            }
        } else if n < 10000 {
            4
        } else {
            5
        }
    } else if n < 10000000 {
        if n < 1000000 {
            6
        } else {
            7
        }
    } else if n < 100000000 {
        8
    } else {
        9
    }
}

// ======================== 数字解析 ========================

/// 从给定字符串读取一个整数。若解析成功，存入 *result 并返回 TRUE；
/// 否则返回 FALSE（对应 `parse_num`）。
pub fn parse_num(string: &str, result: &mut isize) -> bool {
    /* 清除错误号以便之后检查（Rust 的 parse 自带溢出检测）。 */
    let value = string.trim_start().parse::<isize>();

    /* strtol 语义：空串、溢出（ERANGE）或多余字符（excess）均为失败。 */
    match value {
        Ok(v) => {
            *result = v;
            true
        }
        Err(_) => false,
    }
}

/// 从给定字符串读取一个数字（或由逗号、句点、冒号分隔的两个数字），
/// 分别存入 *line（和 *column）。解析失败返回 FALSE，否则 TRUE
/// （对应 `parse_line_column`）。
pub fn parse_line_column(string: &str, line: &mut isize, column: &mut isize) -> bool {
    let mut s = string;

    while s.starts_with(' ') {
        s = &s[1..];
    }

    let comma = s.find(|c| c == ',' || c == '.' || c == ':');

    let comma = match comma {
        None => return parse_num(s, line),
        Some(i) => i,
    };

    let mut retval = parse_num(&s[comma + 1..], column);

    if comma == 0 {
        return retval;
    }

    let firstpart = &s[..comma];

    retval = parse_num(firstpart, line) && retval;

    retval
}

// ======================== 字节重编码 ========================

/// 在给定字符串中，将每个内嵌 NUL 重编码为换行（对应 `recode_NUL_to_LF`）。
pub fn recode_NUL_to_LF(string: &mut [u8], length: usize) {
    let len = length.min(string.len());
    for b in &mut string[..len] {
        if *b == 0 {
            *b = b'\n';
        }
    }
}

/// 在给定字符串中，将每个内嵌换行重编码为 NUL，
/// 并返回字符串中的字节数（对应 `recode_LF_to_NUL`）。
pub fn recode_LF_to_NUL(string: &mut [u8]) -> usize {
    let mut i = 0;
    while i < string.len() && string[i] != 0 {
        if string[i] == b'\n' {
            string[i] = 0;
        }
        i += 1;
    }
    i
}

// ======================== 单词判断（SPELLER） ========================

/// 给定位置处、给定长度的单词在 `text` 中是否是一个独立的词？
/// 即：它不是某个更长词的一部分（对应 `is_separate_word`）。
pub fn is_separate_word(position: usize, length: usize, text: &[u8]) -> bool {
    let before = chars::step_left(text, position);
    let after = position + length;

    (position == 0 || !chars::is_alpha_char(&text[before..]))
        && (after >= text.len() || text[after] == 0 || !chars::is_alpha_char(&text[after..]))
}

// ======================== 搜索包装（对应 strstrwrapper） ========================

/// 在 haystack 中查找 needle，返回匹配位置的字节偏移，未找到返回 None。
/// 反向搜索时找到"开始位置不晚于 start 的最后一个匹配"；
/// 否则找到"开始位置不早于 start 的第一个匹配"。
/// 若使用正则（USE_REGEXP），匹配逻辑由 [`MatchPattern`] 替代
/// 原 `regexec`，并尽量保持原算法的扫描顺序（对应 `strstrwrapper`）。
pub fn strstrwrapper(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if ISSET(USE_REGEXP) {
        let search_regexp = with_global(|g| g.search_regexp.clone());
        let re = match search_regexp {
            Some(r) => r,
            None => return None,
        };

        if ISSET(BACKWARDS_SEARCH) {
            /* 反向：找到最后一个匹配（起点不晚于 start）。 */
            let mut floor: usize = 0;
            let mut next_rung: usize = 0;
            let mut current_so: usize;

            match re.find_match_bytes(haystack) {
                None => return None,
                Some((so, _)) => current_so = so,
            }

            let far_end = haystack.len();
            let ceiling = start;
            let mut last_find = current_so;

            /* 超出搜索范围的结果同样表示：无匹配。 */
            if last_find > ceiling {
                return None;
            }

            /* 将搜索范围起点向前推进直到不再有匹配；
             * 则最后找到的匹配就是反向搜索的第一个匹配。 */
            while current_so <= ceiling {
                floor = next_rung;
                last_find = current_so;
                /* 若这是最后一个可能的匹配，则不再尝试前进。 */
                if last_find == ceiling {
                    break;
                }
                next_rung = chars::step_right(haystack, last_find);
                match re.find_match_bytes(&haystack[next_rung..far_end]) {
                    Some((so, _)) => current_so = next_rung + so,
                    None => break,
                }
            }

            /* 再次找到最后匹配（原代码为获取可能的子匹配，此处略）。 */
            re.find_match_bytes(&haystack[floor..]).map(|(so, _)| floor + so)
        } else {
            /* 从起始点做前向正则搜索。 */
            let s = start.min(haystack.len());
            re.find_match_bytes(&haystack[s..]).map(|(so, _)| s + so)
        }
    } else if ISSET(CASE_SENSITIVE) {
        if ISSET(BACKWARDS_SEARCH) {
            chars::revstrstr(haystack, needle, start)
        } else {
            /* strstr(start, needle)。 */
            let s = start.min(haystack.len());
            let nlen = needle.len();
            if nlen == 0 {
                return Some(s);
            }
            haystack[s..]
                .windows(nlen)
                .position(|w| w == needle)
                .map(|p| s + p)
        }
    } else if ISSET(BACKWARDS_SEARCH) {
        chars::mbrevstrcasestr(haystack, needle, start)
    } else {
        chars::mbstrcasestr(&haystack[start.min(haystack.len())..], needle)
            .map(|p| start + p)
    }
}

// ======================== 内存分配（Rust 中由 Vec/String 自动管理） ========================

/// 分配指定数量的元素（对应 C 的 `nmalloc`）。
pub fn nmalloc<T: Default>(size: usize) -> Vec<T> {
    let mut v = Vec::with_capacity(size);
    for _ in 0..size {
        v.push(T::default());
    }
    v
}

/// 重新分配内存（对应 C 的 `nrealloc`）。
pub fn nrealloc<T: Clone>(vec: &mut Vec<T>, new_size: usize, default: T) {
    vec.resize(new_size, default);
}

/// 分配并清零（对应 C 的 `calloc`）。
pub fn ncalloc<T: Default>(count: usize) -> Vec<T> {
    nmalloc(count)
}

/// 释放内存（对应 C 的 `free`；Rust 中由作用域自动管理）。
pub fn nfree<T>(_ptr: Vec<T>) {
    // Vec 自动释放
}

// ======================== 列与宽度计算 ========================

/// 非软换行时，nano 按块（"页"）水平滚动当前行。
/// 返回光标位于给定列时编辑窗口显示的第一个字符的列号
/// （对应 `get_page_start`）。
pub fn get_page_start(column: usize) -> usize {
    with_global(|g| {
        if g.united_sidescroll {
            if let Some(of) = &g.openfile {
                let of = of.borrow();
                let brink = of.brink;
                let ew = g.editwincols;
                if column < CUSHION {
                    0
                } else if column < brink + CUSHION {
                    if ISSET(JUMPY_SCROLLING) {
                        if column > ew / 2 {
                            column.saturating_sub(ew / 2)
                        } else {
                            0
                        }
                    } else {
                        column.saturating_sub(CUSHION)
                    }
                } else if column > brink + ew.saturating_sub(CUSHION + 1) {
                    column
                        .saturating_sub(ew)
                        .saturating_add(if ISSET(JUMPY_SCROLLING) { ew / 2 } else { CUSHION })
                        .saturating_add(1)
                } else {
                    brink
                }
            } else {
                0
            }
        } else if column == 0 || column + 2 < g.editwincols || ISSET(SOFTWRAP) {
            0
        } else if g.editwincols > 8 {
            column
                .saturating_sub(6)
                .saturating_sub(column.saturating_sub(6) % g.editwincols.saturating_sub(8))
        } else {
            column.saturating_sub(g.editwincols.saturating_sub(2))
        }
    })
}

/// 返回给定文本中"显示时不会越过给定列"的字符的字节索引
/// （对应 `actual_x`）。
pub fn actual_x(text: &[u8], column: usize) -> usize {
    let mut pos = 0;
    let mut width = 0;

    while pos < text.len() && text[pos] != 0 {
        let charlen = chars::advance_over(&text[pos..], &mut width);

        if width > column {
            break;
        }

        pos += charlen;
    }

    pos
}

/// 返回 text 前 count 个字节所占的列数（对应 `wideness`）。
pub fn wideness(text: &[u8], count: usize) -> usize {
    let mut width = 0;

    if count == 0 {
        return 0;
    }

    let mut pos = 0;
    let mut remaining = count;

    while pos < text.len() && text[pos] != 0 {
        let charlen = chars::advance_over(&text[pos..], &mut width);

        if remaining <= charlen {
            break;
        }

        remaining -= charlen;
        pos += charlen;
    }

    width
}

/// 返回给定文本所占的列数（对应 `breadth`）。
pub fn breadth(text: &[u8]) -> usize {
    let mut span = 0;
    let mut pos = 0;

    while pos < text.len() && text[pos] != 0 {
        pos += chars::advance_over(&text[pos..], &mut span);
    }

    span
}

/// 返回光标的（零基）列位置（对应 `xplustabs`）。
pub fn xplustabs() -> usize {
    with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.current.as_ref().map(|c| wideness(c.borrow().data.as_bytes(), of.current_x)).unwrap_or(0)
        }).unwrap_or(0)
    })
}

// ======================== 缓冲区魔法行 ========================

/// 在缓冲区末尾追加一个新的魔法行（对应 `new_magicline`）。
pub fn new_magicline() {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let filebot = of.filebot.clone();
            if let Some(fb) = filebot {
                let given = fb.borrow();
                let newnode = make_new_node(Some(&*given));
                drop(given);
                newnode.borrow_mut().prev = Some(Rc::downgrade(&fb));
                fb.borrow_mut().next = Some(newnode.clone());
                of.filebot = Some(newnode);
                of.totsize += 1;
            }
        }
    });
}

/// 若缓冲区末尾有魔法行且它不是唯一一行，则移除之
/// （对应 `remove_magicline`）。
pub fn remove_magicline() {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let filebot = of.filebot.clone();
            let filetop = of.filetop.clone();
            if let (Some(fb), Some(ft)) = (filebot, filetop) {
                let is_empty = fb.borrow().data.is_empty();
                let not_only = !Rc::ptr_eq(&fb, &ft);

                if is_empty && not_only {
                    if of.current.as_ref().map(|c| Rc::ptr_eq(c, &fb)).unwrap_or(false) {
                        let prev = fb.borrow().prev.clone().and_then(|w| w.upgrade());
                        of.current = prev;
                    }
                    let prev = fb.borrow().prev.clone().and_then(|w| w.upgrade());
                    if let Some(p) = prev {
                        p.borrow_mut().next = None;
                        of.filebot = Some(p);
                        of.totsize -= 1;
                    }
                }
            }
        }
    });
}

// ======================== 标记区域 ========================

/// 内部辅助：读取标记区域坐标（不访问全局，避免借用冲突）。
fn get_region_raw(of: &OpenFileStruct) -> (LineRef, usize, LineRef, usize) {
    let mark_before = match (&of.mark, &of.current) {
        (Some(m), Some(c)) => {
            let m_line = m.borrow().lineno;
            let c_line = c.borrow().lineno;
            m_line < c_line || (Rc::ptr_eq(m, c) && of.mark_x <= of.current_x)
        }
        _ => false,
    };

    if mark_before {
        (
            of.mark.clone().unwrap(),
            of.mark_x,
            of.current.clone().unwrap(),
            of.current_x,
        )
    } else {
        (
            of.current.clone().unwrap(),
            of.current_x,
            of.mark.clone().unwrap(),
            of.mark_x,
        )
    }
}

/// 返回 TRUE 当标记位于光标之前或与光标同处，否则 FALSE
/// （对应 `mark_is_before_cursor`）。
pub fn mark_is_before_cursor() -> bool {
    with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            match (&of.mark, &of.current) {
                (Some(m), Some(c)) => {
                    let m_line = m.borrow().lineno;
                    let c_line = c.borrow().lineno;
                    m_line < c_line || (Rc::ptr_eq(m, c) && of.mark_x <= of.current_x)
                }
                _ => false,
            }
        }).unwrap_or(false)
    })
}

/// 返回 (top, top_x, bot, bot_x)：标记区域的起点与终点"坐标"
/// （对应 `get_region`）。
pub fn get_region() -> (LineRef, usize, LineRef, usize) {
    with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        get_region_raw(&of)
    })
}

/// 返回 (top, bot)：要处理的行集合——或仅为当前行，或为标记区域的
/// 首行到末行。当光标（或标记）位于区域末行的行首时，排除该行
/// （对应 `get_range`）。
pub fn get_range() -> (LineRef, LineRef) {
    with_global_mut(|g| {
        let of = g.openfile.as_ref().expect("no open file").clone();
        let of = of.borrow();

        if of.mark.is_none() {
            (of.current.clone().unwrap(), of.current.clone().unwrap())
        } else {
            let (top, _top_x, bot, bot_x) = get_region_raw(&of);

            if bot_x == 0 && !Rc::ptr_eq(&bot, &top) && !g.also_the_last {
                let prev = bot.borrow().prev.clone().and_then(|w| w.upgrade()).unwrap();
                (top, prev)
            } else {
                g.also_the_last = true;
                (top, bot)
            }
        }
    })
}

// ======================== 行号查找与字符计数 ========================

/// 返回具有给定行号的那一行（对应 `line_from_number`）。
pub fn line_from_number(number: isize) -> LineRef {
    with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let line = of.current.clone().unwrap();

        if line.borrow().lineno > number {
            let mut l = line;
            loop {
                let lineno = l.borrow().lineno;
                if lineno == number {
                    return l;
                }
                let prev = { let r = l.borrow(); r.prev.clone() };
                l = prev.and_then(|w| w.upgrade()).unwrap();
            }
        } else {
            let mut l = line;
            loop {
                let lineno = l.borrow().lineno;
                if lineno == number {
                    return l;
                }
                let next = { let r = l.borrow(); r.next.clone() };
                l = next.unwrap();
            }
        }
    })
}

/// 计算从 begin 到 end 的字符数量并返回（对应 `number_of_characters_in`）。
pub fn number_of_characters_in(begin: &LineRef, end: &LineRef) -> usize {
    let mut count = 0;
    let mut line = begin.clone();

    /* 累计每行中的字符数（加上一个换行）。 */
    loop {
        count += chars::mbstrlen(line.borrow().data.as_bytes()) + 1;
        if Rc::ptr_eq(&line, end) {
            break;
        }
        let next = { let r = line.borrow(); r.next.clone() };
        match next {
            Some(n) => line = n,
            /* 对应 C 的 `line != end->next` 循环：到达 NULL 即停止。 */
            None => break,
        }
    }

    /* 不计最后一个换行。 */
    count - 1
}

/// 用于避免未使用告警的占位（对应 C 的 `free_chararray`；
/// Rust 中由 Vec 自动释放）。
pub fn free_chararray<T>(_array: Vec<Vec<T>>) {
    // Vec 自动释放
}
