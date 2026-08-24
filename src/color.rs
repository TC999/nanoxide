/**************************************************************************
 * color.rs  --  GNU nano 颜色管理（对应 color.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 颜色管理，替代 ncurses 颜色对。对应原版 nano 的 `color.c`。
//! 转换说明：使用 crossterm 颜色 API 替代 ncurses。

use crate::definitions::*;
use crate::chars;
use crate::winio;
use crossterm::style::Color;
use std::cell::RefCell;
use std::rc::Rc;

/// 颜色常量。
pub const A_NORMAL: i32 = 0;
pub const A_REVERSE: i32 = 1;
pub const A_BOLD: i32 = 2;
pub const A_UNDERLINE: i32 = 4;
pub const A_ITALIC: i32 = 8;
pub const A_BLINK: i32 = 8;

/// 无效颜色（对应 C 的 BAD_COLOR）。
pub const BAD_COLOR: i16 = -2;

// 颜色对表：pairnum → (fg, bg)。由 init_pair 填充，渲染时查表。
thread_local! {
    static COLOR_PAIRS: RefCell<std::collections::HashMap<i32, (i16, i16)>> =
        RefCell::new(std::collections::HashMap::new());
}

/// 将 nano 颜色值转换为 crossterm Color。
/// 0-7 为基础色，8-255 为 256 色（AnsiValue），-1 为默认。
pub fn nano_to_crossterm_color(color: i16) -> Color {
    match color {
        -1 => Color::Reset,           // THE_DEFAULT
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        8 => Color::DarkGrey,
        9 => Color::Red,
        10 => Color::Green,
        11 => Color::Yellow,
        12 => Color::Blue,
        13 => Color::Magenta,
        14 => Color::Cyan,
        15 => Color::White,
        16..=255 => Color::AnsiValue(color as u8),
        _ => Color::Reset,
    }
}

/// 设置界面颜色对（对应 color.c 的 `set_interface_colorpairs`）。
/// 在 rcfile 解析完成后调用：用 set color 命令登记的 color_combo 初始化界面颜色对。
pub fn set_interface_colorpairs() {
    set_interface_colorpairs_full();
}

/// 设置给定界面元素的颜色组合（对应 rcfile.c 的 `set_interface_color`）。
/// 该函数把解析结果存入 color_combo，待 set_interface_colorpairs 时生效。
pub fn set_interface_color(element: usize, combotext: &str) {
    let Some((fg, bg, attributes)) = parse_combination(combotext) else {
        return;
    };
    with_global_mut(|g| {
        if element < g.color_combo.len() {
            let trio = Rc::new(RefCell::new(ColorType {
                id: 0,
                fg,
                bg,
                pairnum: 0,
                attributes,
                start: None,
                end: None,
                next: None,
            }));
            g.color_combo[element] = Some(trio);
        }
    });
}

/// 初始化颜色对（对应 init_pair）：记录 pairnum → (fg, bg)，渲染时查表。
pub fn init_pair(pairnum: i16, fg: i16, bg: i16) {
    COLOR_PAIRS.with(|m| m.borrow_mut().insert(pairnum as i32, (fg, bg)));
}

/// 颜色对编号编码进属性值的高 16 位（对应 COLOR_PAIR）。
pub fn COLOR_PAIR(pairnum: i32) -> i32 {
    pairnum << 16
}

/// 从属性值中提取颜色对编号。
pub fn pairnum_from(attr: i32) -> i32 {
    attr >> 16
}

/// 查询颜色对 (fg, bg)；无则返回 None。
pub fn lookup_pair(pairnum: i32) -> Option<(i16, i16)> {
    COLOR_PAIRS.with(|m| m.borrow().get(&pairnum).copied())
}

/// 启用颜色属性。
pub fn wattron(attr: i32) -> i32 {
    attr
}

/// 禁用颜色属性。
pub fn wattroff(attr: i32) -> i32 {
    attr
}

/// 设置颜色属性。
pub fn set_attributes(attr: i32) -> i32 {
    attr
}

/// 检查终端是否支持颜色。
pub fn has_colors() -> bool {
    true // 现代终端基本都支持颜色
}

/// 初始化颜色系统。
pub fn start_color() {
    // crossterm 无需显式初始化
}

/// 获取界面元素的颜色对编号。
pub fn interface_color_pair(element: usize) -> i32 {
    with_global(|g| {
        g.interface_color_pair.get(element).copied().unwrap_or(0)
    })
}

/// 设置界面元素的颜色对编号。
pub fn set_interface_color_pair(element: usize, pair: i32) {
    with_global_mut(|g| {
        if element < g.interface_color_pair.len() {
            g.interface_color_pair[element] = pair;
        }
    });
}

/// 将颜色字符串解析为颜色值。
pub fn color_name_to_number(name: &str) -> i16 {
    match name.to_lowercase().as_str() {
        "black" => 0,
        "red" => 1,
        "green" => 2,
        "yellow" => 3,
        "blue" => 4,
        "magenta" => 5,
        "cyan" => 6,
        "white" => 7,
        "default" => -1,
        _ => -1,
    }
}

/// 获取颜色名称。
pub fn color_number_to_name(color: i16) -> &'static str {
    match color {
        0 => "black",
        1 => "red",
        2 => "green",
        3 => "yellow",
        4 => "blue",
        5 => "magenta",
        6 => "cyan",
        7 => "white",
        -1 => "default",
        _ => "unknown",
    }
}

/// 准备颜色对并返回组合属性。
pub fn prepare_color_pair(fg: i16, bg: i16, attributes: i32) -> i32 {
    // 组合颜色属性
    attributes | ((fg as i32 + 1) << 8) | ((bg as i32 + 1) << 12)
}

// ======================== 界面颜色对（对应 color.c） ========================

/// 初始化 nano 界面元素的颜色对（对应 `set_interface_colorpairs`）。
/// crossterm 架构下颜色对映射为属性值。
pub fn set_interface_colorpairs_full() {
    /* crossterm 下默认颜色由 Color::Reset 表示，与 C 版 use_default_colors()
     * 成功时的行为一致：保持 THE_DEFAULT (-1) 不变。 */
    let elements = with_global(|g| g.color_combo.clone());

    for (index, combo) in elements.iter().enumerate() {
        if let Some(c) = combo {
            let c = c.borrow();
            /* 保持 THE_DEFAULT (-1) 不变，由 nano_to_crossterm_color(-1) → Color::Reset
             * 使用终端默认颜色。 */
            init_pair(index as i16 + 1, c.fg, c.bg);
            set_interface_color_pair(index, COLOR_PAIR(index as i32 + 1) | c.attributes);
            with_global_mut(|g| g.rescind_colors = false);
        } else {
            if index == FUNCTION_TAG || index == SCROLL_BAR {
                set_interface_color_pair(index, A_NORMAL);
            } else if index == GUIDE_STRIPE {
                set_interface_color_pair(index, A_REVERSE);
            } else if index == SPOTLIGHTED {
                init_pair(index as i16 + 1, 0, 3 + if 16 > 15 { 8 } else { 0 });
                set_interface_color_pair(index, COLOR_PAIR(index as i32 + 1));
            } else if index == MINI_INFOBAR || index == PROMPT_BAR {
                let tb = with_global(|g| g.interface_color_pair[TITLE_BAR]);
                set_interface_color_pair(index, tb);
            } else if index == ERROR_MESSAGE {
                init_pair(index as i16 + 1, 7, 1);
                set_interface_color_pair(index, COLOR_PAIR(index as i32 + 1) | A_BOLD);
            } else {
                let ha = with_global(|g| g.hilite_attribute);
                set_interface_color_pair(index, ha);
            }
        }
    }

    /* 清理 color_combo。 */
    with_global_mut(|g| g.color_combo = vec![None; NUMBER_OF_ELEMENTS]);

    if with_global(|g| g.rescind_colors) {
        set_interface_color_pair(SPOTLIGHTED, A_REVERSE);
        set_interface_color_pair(ERROR_MESSAGE, A_REVERSE);
    }
}

/// 为给定语法中的每个前景/背景颜色组合分配对编号，相同组合用相同编号
/// （对应 `set_syntax_colorpairs`）。
pub fn set_syntax_colorpairs(sntx: &SyntaxRef) {
    let mut number = NUMBER_OF_ELEMENTS as i16;

    /* 收集颜色列表。 */
    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = sntx.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    for ink in &colors {
        /* 保持 THE_DEFAULT (-1) 不变，由 apply_attributes 中的
         * nano_to_crossterm_color(-1) → Color::Reset 使用终端默认颜色。
         * 对应 C 版中 use_default_colors() 成功时的行为：不替换默认色。 */

        /* 找相同组合的旧颜色。 */
        let mut older_pair: Option<i16> = None;
        for older in &colors {
            if Rc::ptr_eq(older, ink) {
                break;
            }
            let o = older.borrow();
            let i = ink.borrow();
            if o.fg == i.fg && o.bg == i.bg {
                older_pair = Some(o.pairnum);
                break;
            }
        }

        let mut ink_ref = ink.borrow_mut();
        ink_ref.pairnum = match older_pair {
            Some(p) => p,
            None => {
                number += 1;
                number
            }
        };
        let pn = ink_ref.pairnum;
        ink_ref.attributes |= COLOR_PAIR(pn as i32);
    }
}

/// 初始化当前语法的颜色对（对应 `prepare_palette`）。
pub fn prepare_palette() {
    let mut number = NUMBER_OF_ELEMENTS as i16;
    let sntx = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().syntax.clone())
    });
    let Some(sntx) = sntx else { return };

    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = sntx.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    /* 为每个唯一对编号告诉渲染层颜色组合。 */
    for ink in &colors {
        let ink_ref = ink.borrow();
        if ink_ref.pairnum > number {
            init_pair(ink_ref.pairnum, ink_ref.fg, ink_ref.bg);
            number = ink_ref.pairnum;
        }
    }

    with_global_mut(|g| g.have_palette = true);
}

/// 尝试把给定字符串与 head 开始的列表中的某个正则匹配
/// （对应 `found_in_list`）。
pub fn found_in_list(head: Option<RegexListRef>, shibboleth: &str) -> bool {
    let mut item = head;
    while let Some(it) = item {
        let matched = {
            let r = it.borrow();
            r.one_rgx.as_ref().map(|rgx| rgx.matches(shibboleth)).unwrap_or(false)
        };
        if matched {
            return true;
        }
        let next = { let r = it.borrow(); r.next.clone() };
        item = next;
    }
    false
}

/// 找到适用于当前缓冲区的语法，必要时加载并初始化
/// （对应 `find_and_prime_applicable_syntax`）。
pub fn find_and_prime_applicable_syntax() {
    let mut sntx: Option<SyntaxRef> = None;
    let inhelp = with_global(|g| g.inhelp);

    /* 若 rc 文件未读或没有语法，退出。 */
    let syntaxes = with_global(|g| g.syntaxes.clone());
    if syntaxes.is_none() {
        return;
    }

    /* 若指定了语法覆盖字符串，使用它。 */
    let syntaxstr = with_global(|g| g.syntaxstr.clone());
    if let Some(sstr) = &syntaxstr {
        if sstr == "none" {
            return;
        }
        let mut cur = syntaxes.clone();
        while let Some(s) = cur {
            let name_matches = {
                let r = s.borrow();
                r.name.as_ref().map(|n| n == sstr).unwrap_or(false)
            };
            if name_matches {
                sntx = Some(s.clone());
                break;
            }
            let next = { let r = s.borrow(); r.next.clone() };
            cur = next;
        }
        if sntx.is_none() && !inhelp {
            winio::statusline(MessageType::Alert, &crate::t!("color-unknown_syntax", name = sstr));
        }
    }

    /* 未指定覆盖或未匹配时，按文件名（扩展名）查找。 */
    if sntx.is_none() && !inhelp {
        let filename = with_global(|g| {
            g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
        });
        if let Some(fname) = filename {
            let fullname = crate::files::get_full_path(&fname)
                .unwrap_or_else(|| fname.clone());
            let mut cur = syntaxes.clone();
            while let Some(s) = cur {
                let found = {
                    let r = s.borrow();
                    found_in_list(r.extensions.clone(), &fullname)
                };
                if found {
                    sntx = Some(s.clone());
                    break;
                }
                let next = { let r = s.borrow(); r.next.clone() };
                cur = next;
            }
        }
    }

    /* 文件名未匹配时，尝试首行。 */
    if sntx.is_none() && !inhelp {
        let firstline = with_global(|g| {
            g.openfile.as_ref().and_then(|of| {
                of.borrow().filetop.as_ref().map(|t| t.borrow().data.clone())
            })
        });
        if let Some(fl) = firstline {
            let mut cur = syntaxes.clone();
            while let Some(s) = cur {
                let found = {
                    let r = s.borrow();
                    found_in_list(r.headers.clone(), &fl)
                };
                if found {
                    sntx = Some(s.clone());
                    break;
                }
                let next = { let r = s.borrow(); r.next.clone() };
                cur = next;
            }
        }
    }

    /* 全部未匹配时，寻找 default 语法。 */
    if sntx.is_none() && !inhelp {
        let mut cur = syntaxes.clone();
        while let Some(s) = cur {
            let is_default = {
                let r = s.borrow();
                r.name.as_ref().map(|n| n == "default").unwrap_or(false)
            };
            if is_default {
                sntx = Some(s.clone());
                break;
            }
            let next = { let r = s.borrow(); r.next.clone() };
            cur = next;
        }
    }

    /* 为选定的语法分配颜色对编号（对应 C 的 parse_one_include + set_syntax_colorpairs）：
     * Rust 中语法已在启动时全量解析（filename 恒为 None），故直接无条件初始化。
     * set_syntax_colorpairs 幂等：相同组合复用同一 pairnum，重复调用无副作用。 */
    if let Some(s) = &sntx {
        set_syntax_colorpairs(s);
    }

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().syntax = sntx;
        }
    });

    /* 有语法时准备调色板（需在 syntax 写入 openfile 之后，填充颜色对表）。 */
    let has_syntax = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().syntax.clone()).is_some()
    });
    if has_syntax {
        prepare_palette();
    }
}

/// 判断多行正则的匹配是否仍然相同；不同则安排屏幕刷新
/// （对应 `check_the_multis`）。
pub fn check_the_multis(line: &LineRef) {
    /* 若无语法或无多行正则，无事可做。 */
    let (syntax, multiscore) = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let s = of.syntax.clone();
            (s.clone(), s.as_ref().map(|s| s.borrow().multiscore).unwrap_or(0))
        }).unwrap_or((None, 0))
    });
    let Some(syntax) = syntax else { return };
    if multiscore == 0 {
        return;
    }

    if line.borrow().multidata.is_none() {
        with_global_mut(|g| g.refresh_needed = true);
        return;
    }

    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = syntax.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    let line_data = line.borrow().data.clone();
    let multidata = line.borrow().multidata.clone().unwrap_or_default();

    for ink in &colors {
        /* 若不是多行正则，跳过。 */
        let ink_ref = ink.borrow();
        if ink_ref.end.is_none() {
            continue;
        }
        let id = ink_ref.id;

        let astart = ink_ref.start.as_ref()
            .and_then(|s| s.find_match_bytes(line_data.as_bytes()))
            .map(|(_, eo)| eo)
            .unwrap_or(0);
        let astart_match = ink_ref.start.as_ref().map(|s| s.matches(&line_data)).unwrap_or(false);
        let afterstart = if astart_match { astart } else { 0 };

        let anend_match = ink_ref.end.as_ref()
            .and_then(|e| e.find_match_bytes(&line_data.as_bytes()[afterstart..]));
        let anend = anend_match.is_some();

        let md = multidata.get(id as usize).copied().unwrap_or(0);
        if md == NOTHING as i16 {
            if !astart_match {
                continue;
            }
        } else if md == WHOLELINE as i16 {
            /* 确保检测到的 start 匹配不是实际上的 end 匹配。 */
            let end_whole = ink_ref.end.as_ref()
                .map(|e| e.find_match_bytes(line_data.as_bytes()).is_none())
                .unwrap_or(true);
            if !anend && (!astart_match || end_whole) {
                continue;
            }
        } else if md == JUSTONTHIS as i16 {
            if astart_match && anend {
                /* 在 start 匹配之后再加上 end 匹配的终点，寻找第三个 start
                 * （对应 C：regexec(start, data + startmatch.rm_eo + endmatch.rm_eo, ...)）。 */
                let third = ink_ref.start.as_ref()
                    .and_then(|s| s.find_match_bytes(&line_data.as_bytes()[afterstart + anend_match.unwrap().1..]))
                    .is_none();
                if third {
                    continue;
                }
            }
        } else if md == STARTSHERE as i16 {
            if astart_match && !anend {
                continue;
            }
        } else if md == ENDSHERE as i16 {
            if !astart_match && anend {
                continue;
            }
        }

        /* 不匹配：有变化，重绘。 */
        with_global_mut(|g| {
            g.refresh_needed = true;
            g.perturbed = true;
        });
        return;
    }
}

/// 预计算多行起始与结束正则信息以加速渲染
/// （对应 `precalc_multicolorinfo`）。
pub fn precalc_multicolorinfo() {
    let (syntax, multiscore) = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let s = of.syntax.clone();
            (s.clone(), s.as_ref().map(|s| s.borrow().multiscore).unwrap_or(0))
        }).unwrap_or((None, 0))
    });
    let Some(syntax) = syntax else { return };
    if multiscore == 0 || ISSET(NO_SYNTAX) {
        return;
    }

    /* 为每行分配多行正则信息的缓存空间。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let of = of.borrow_mut();
            let mut line = of.filetop.clone();
            while let Some(l) = line {
                if l.borrow().multidata.is_none() {
                    l.borrow_mut().multidata = Some(vec![0; multiscore as usize]);
                }
                let next = { let r = l.borrow(); r.next.clone() };
                line = next;
            }
        }
    });

    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = syntax.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    for ink in &colors {
        /* 不是多行正则则跳过。 */
        let ink_ref = ink.borrow();
        if ink_ref.end.is_none() {
            continue;
        }
        let (id, start_pat, end_pat) = (ink_ref.id, ink_ref.start.clone(), ink_ref.end.clone());
        drop(ink_ref);

        let lines: Vec<LineRef> = with_global(|g| {
            let mut v = Vec::new();
            let mut line = g.openfile.as_ref().unwrap().borrow().filetop.clone();
            while let Some(l) = line {
                v.push(l.clone());
                let next = { let r = l.borrow(); r.next.clone() };
                line = next;
            }
            v
        });

        let mut line_iter = 0;
        while line_iter < lines.len() {
            let mut line = lines[line_iter].clone();
            let mut index = 0;
            let mut data = line.borrow().data.clone();

            /* 假设开始时不适用任何内容。 */
            line.borrow_mut().multidata.as_mut().map(|m| m[id as usize] = NOTHING as i16);

            /* 当行中有 start 匹配时，寻找 end；找到后标记所有受影响的行。 */
            loop {
                let startmatch = start_pat.as_ref()
                    .and_then(|s| s.find_match_bytes(&data.as_bytes()[index..]))
                    .map(|(so, eo)| (index + so, index + eo));
                let Some((start_so, start_eo)) = startmatch else { break };

                /* 在 start 匹配之后开始寻找 end 匹配。 */
                index = start_eo;
                let end_search_start = index;

                /* 若同一行有 end 匹配，标记该行并继续找其他 start。 */
                let endmatch = end_pat.as_ref()
                    .and_then(|e| e.find_match_bytes(&data.as_bytes()[index..]))
                    .map(|(so, eo)| (index + so, index + eo));
                if let Some((_, end_eo)) = endmatch {
                    line.borrow_mut().multidata.as_mut().map(|m| m[id as usize] = JUSTONTHIS as i16);

                    /* 总匹配长度 = start 匹配长度 + end 匹配长度
                     * （对应 C 的 startmatch.rm_eo - startmatch.rm_so + endmatch.rm_eo）。 */
                    let total_len = (start_eo - start_so) + (end_eo - end_search_start);
                    index = end_eo;

                    /* 若总匹配长度为零，强制前进。 */
                    if total_len == 0 {
                        /* 位于行尾时没有其他 start。 */
                        if data.as_bytes().get(index).copied().unwrap_or(0) == 0 {
                            break;
                        }
                        index = chars::step_right(data.as_bytes(), index);
                    }
                    continue;
                }

                /* 在后续行中寻找 end 匹配。 */
                let mut tailline = line_iter + 1;
                let mut tail_eo: Option<usize> = None;
                while tailline < lines.len() {
                    let tdata = lines[tailline].borrow().data.clone();
                    if let Some((_, eo)) = end_pat.as_ref().and_then(|e| e.find_match_bytes(tdata.as_bytes())) {
                        tail_eo = Some(eo);
                        break;
                    }
                    tailline += 1;
                }

                line.borrow_mut().multidata.as_mut().map(|m| m[id as usize] = STARTSHERE as i16);

                /* 把中间各行标记为 WHOLELINE。 */
                for li in line_iter + 1..tailline {
                    lines[li].borrow_mut().multidata.as_mut().map(|m| m[id as usize] = WHOLELINE as i16);
                }

                if tail_eo.is_none() {
                    /* 跳到文件末尾。 */
                    line_iter = lines.len() - 1;
                    break;
                }

                lines[tailline].borrow_mut().multidata.as_mut().map(|m| m[id as usize] = ENDSHERE as i16);

                /* 在 end 匹配后继续寻找可能的新的 start：把工作行推进到
                 * tailline、从 end 匹配终点继续（对应 C 修改外层 line 指针
                 * 并用 index = endmatch.rm_eo 继续内层 while）。 */
                line_iter = tailline;
                line = lines[tailline].clone();
                data = line.borrow().data.clone();
                index = tail_eo.unwrap();
            }

            line_iter += 1;
        }
    }
}
// ======================== 颜色名解析（对应 rcfile.c 的 color_to_short/parse_combination） ========================

/// 34 个可用的颜色名（对应 C 的 hues 表）。
const HUES: [&str; 34] = [
    "red", "green", "blue", "yellow", "cyan", "magenta",
    "white", "black", "normal",
    "pink", "purple", "mauve", "lagoon", "mint", "lime",
    "peach", "orange", "latte", "rosy", "beet", "plum",
    "sea", "sky", "slate", "teal", "sage", "brown",
    "ocher", "sand", "tawny", "brick", "crimson",
    "grey", "gray",
];

/// 对应每个色名的颜色值（对应 C 的 indices 表）。
const INDICES: [i16; 34] = [
    1, 2, 3, 4, 6, 5,
    7, 0, -1,
    204, 163, 134, 38, 48, 148,
    215, 208, 137, 175, 127, 98,
    32, 111, 66, 35, 107, 100,
    142, 186, 136, 166, 161,
    8, 8,
];

/// 把颜色名解析为颜色值；返回 (值, vivid, thick)（对应 `color_to_short`）。
/// 失败时返回 Err(错误消息)。
pub fn color_to_short(colorname: &str) -> Result<(i16, bool, bool), String> {
    let mut vivid = false;
    let mut thick = false;
    let mut name = colorname;

    if let Some(rest) = name.strip_prefix("bright") {
        if !rest.is_empty() {
            vivid = true;
            thick = true;
            name = rest;
        }
    } else if let Some(rest) = name.strip_prefix("light") {
        if !rest.is_empty() {
            vivid = true;
            thick = false;
            name = rest;
        }
    }

    // #RGB 形式（4 字符十六进制）
    if name.starts_with('#') && name.len() == 4 {
        if vivid {
            return Err(crate::t!("color-no_prefix_allowed", name = colorname));
        }
        let parse_hex = |c: char| c.to_digit(16);
        let chars: Vec<char> = name[1..].chars().collect();
        if chars.len() == 3 {
            let r = parse_hex(chars[0]);
            let g = parse_hex(chars[1]);
            let b = parse_hex(chars[2]);
            if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                /* 灰阶：红绿蓝相等时映射到 xterm 灰阶（对应 closest_index_color）。 */
                if r == g && g == b && r > 0 && r < 0xF {
                    const GRAY: [i32; 14] = [1, 2, 3, 4, 5, 6, 7, 9, 11, 13, 15, 18, 21, 23];
                    return Ok((232 + GRAY[(r - 1) as usize] as i16, false, false));
                }
                let level = |v: u32| -> i32 { match v { 0..=3 => 0, 4..=7 => 1, 8..=9 => 2, 10..=11 => 3, 12..=13 => 4, _ => 5 } };
                let value = 16 + 36 * level(r) + 6 * level(g) + level(b);
                return Ok((value as i16, false, false));
            }
        }
        return Err(crate::t!("color-unknown_color", name = colorname));
    }

    for (index, hue) in HUES.iter().enumerate() {
        if name == *hue {
            if index > 7 && vivid {
                return Err(crate::t!("color-no_prefix_allowed", name = colorname));
            }
            if index > 8 {
                // 扩展色在少色终端退化；crossterm 一律支持 256 色。
            }
            return Ok((INDICES[index], vivid, thick));
        }
    }

    Err(crate::t!("color-unknown_color", name = colorname))
}

/// 解析颜色组合（对应 `parse_combination`）：返回 (fg, bg, attributes)。
/// 组合形式：`[bold[,]][fg][,bg]`，如 "red"、"bold,red,blue"、"lightyellow"。
pub fn parse_combination(text: &str) -> Option<(i16, i16, i32)> {
    let mut attributes = A_NORMAL;
    let mut s = text;

    if let Some(rest) = s.strip_prefix("bold") {
        attributes |= A_BOLD;
        if !rest.starts_with(',') {
            crate::rcfile::jot_error(&crate::t!("color-attr_needs_comma"));
            return None;
        }
        s = &rest[1..];
    }
    if let Some(rest) = s.strip_prefix("italic") {
        attributes |= A_ITALIC;
        if !rest.starts_with(',') {
            crate::rcfile::jot_error(&crate::t!("color-attr_needs_comma"));
            return None;
        }
        s = &rest[1..];
    }

    let mut parts = s.splitn(2, ',');
    let fg_part = parts.next().unwrap_or("");
    let bg_part = parts.next();

    /* 逗号位于开头（如 ",blue"）时前景色为默认（对应 parse_combination）。 */
    let (fgv, fg_vivid, fg_thick) = if fg_part.is_empty() {
        (THE_DEFAULT as i16, false, false)
    } else {
        match color_to_short(fg_part) {
            Ok(v) => v,
            Err(msg) => {
                crate::rcfile::jot_error(&msg);
                return None;
            }
        }
    };
    let fg = if fg_vivid && !fg_thick { fgv + 8 } else { fgv };
    if fg_vivid && fg_thick {
        attributes |= A_BOLD;
    }

    let bg = match bg_part {
        None => THE_DEFAULT as i16,
        Some(b) => match color_to_short(b) {
            Ok((bv, b_vivid, _)) => {
                if b_vivid { bv + 8 } else { bv }
            }
            Err(msg) => {
                crate::rcfile::jot_error(&msg);
                return None;
            }
        },
    };

    Some((fg, bg, attributes))
}
