/**************************************************************************
 *   color.rs  --  GNU nano 的 color.c 翻译（Rust 版）。                     *
 *   对应 C 源：nano/src/color.c（377 行）。                                *
 *   全特性构建：所有 #ifdef ENABLE_COLOR / HAVE_LIBMAGIC 分支均视为启用。  *
 *   注意：HAVE_LIBMAGIC 相关逻辑（libmagic）在本仓库中不可用，故跳过。    *
 **************************************************************************/

use crate::chars::step_right;
use crate::definitions::*;
use crate::files::{get_full_path, statusline};
use crate::global::*;
use crate::rcfile::parse_one_include;
use crate::utils::nmalloc;

/// 颜色子系统所需的局部常量（占位：真实 ncurses 依赖将在后续步骤补齐）。
/// 与 rcfile.rs 中的桩保持一致。
pub const COLORS: i32 = 256;

pub const COLOR_RED: i16 = 1;
pub const COLOR_GREEN: i16 = 2;
pub const COLOR_BLUE: i16 = 4;
pub const COLOR_YELLOW: i16 = 3;
pub const COLOR_CYAN: i16 = 6;
pub const COLOR_MAGENTA: i16 = 5;
pub const COLOR_WHITE: i16 = 7;
pub const COLOR_BLACK: i16 = 0;

pub const A_NORMAL: i32 = 0;
pub const A_BOLD: i32 = 1;
pub const A_ITALIC: i32 = 2;
pub const A_REVERSE: i32 = 3;

/// 将"对编号"映射为一个属性值（占位：真实实现依赖 ncurses 的 COLOR_PAIR）。
pub fn COLOR_PAIR(n: i32) -> i32 {
    n << 8
}

/// 初始化一个颜色对（占位：真实实现调用 ncurses 的 init_pair）。
pub fn init_pair(_pair: i32, _fg: i16, _bg: i16) {}

/// 询问 ncurses 是否接受 -1 表示"默认颜色"（占位：默认不允许）。
pub fn use_default_colors() -> i32 {
    0
}

/// ncurses 的成功返回值。
pub const OK: i32 = 0;

/// 是否允许 ncurses 用 -1 表示"默认颜色"。
static mut defaults_allowed: bool = false;

/* 为 nano 界面的各个界面元素初始化颜色对（对应 C 的 set_interface_colorpairs）。 */
pub fn set_interface_colorpairs() {
    unsafe {
        /* 询问 ncurses 是否允许 -1 表示"默认颜色"。 */
        defaults_allowed = use_default_colors() == OK;

        /* 为 nano 界面元素初始化颜色对。 */
        for index in 0..NUMBER_OF_ELEMENTS {
            let combo = color_combo[index];

            if !combo.is_null() {
                if !defaults_allowed {
                    if (*combo).fg == THE_DEFAULT as i16 {
                        (*combo).fg = COLOR_WHITE;
                    }
                    if (*combo).bg == THE_DEFAULT as i16 {
                        (*combo).bg = COLOR_BLACK;
                    }
                }
                init_pair((index + 1) as i32, (*combo).fg, (*combo).bg);
                interface_color_pair[index] = COLOR_PAIR((index + 1) as i32) | (*combo).attributes;
                rescind_colors = false;
            } else if index == FUNCTION_TAG || index == SCROLL_BAR {
                interface_color_pair[index] = A_NORMAL;
            } else if index == GUIDE_STRIPE {
                interface_color_pair[index] = A_REVERSE;
            } else if index == SPOTLIGHTED {
                init_pair(
                    (index + 1) as i32,
                    COLOR_BLACK,
                    (COLOR_YELLOW + if COLORS > 15 { 8 } else { 0 }) as i16,
                );
                interface_color_pair[index] = COLOR_PAIR((index + 1) as i32);
            } else if index == MINI_INFOBAR || index == PROMPT_BAR {
                interface_color_pair[index] = interface_color_pair[TITLE_BAR];
            } else if index == ERROR_MESSAGE {
                init_pair((index + 1) as i32, COLOR_WHITE, COLOR_RED);
                interface_color_pair[index] = COLOR_PAIR((index + 1) as i32) | A_BOLD;
            } else {
                interface_color_pair[index] = hilite_attribute;
            }

            /* 释放 color_combo 中临时分配的 colortype（对应 C 的 free）。 */
            if !color_combo[index].is_null() {
                let _ = Box::from_raw(color_combo[index]);
                color_combo[index] = std::ptr::null_mut();
            }
        }

        if rescind_colors {
            interface_color_pair[SPOTLIGHTED] = A_REVERSE;
            interface_color_pair[ERROR_MESSAGE] = A_REVERSE;
        }
    }
}

/* 为给定语法中的每个前景/背景颜色组合分配一个对编号，
 * 使相同组合共享同一编号（对应 C 的 set_syntax_colorpairs）。 */
pub fn set_syntax_colorpairs(sntx: *mut syntaxtype) {
    let mut number: i16 = NUMBER_OF_ELEMENTS as i16;

    unsafe {
        let mut ink = (*sntx).color;
        while !ink.is_null() {
            if !defaults_allowed {
                if (*ink).fg == THE_DEFAULT as i16 {
                    (*ink).fg = COLOR_WHITE;
                }
                if (*ink).bg == THE_DEFAULT as i16 {
                    (*ink).bg = COLOR_BLACK;
                }
            }

            let mut older = (*sntx).color;

            while !older.is_null() && older != ink && ((*older).fg != (*ink).fg || (*older).bg != (*ink).bg) {
                older = (*older).next;
            }

            (*ink).pairnum = if older != ink { (*older).pairnum } else {
                number += 1;
                number
            };

            (*ink).attributes |= COLOR_PAIR((*ink).pairnum as i32);

            ink = (*ink).next;
        }
    }
}

/* 为当前语法初始化颜色对（对应 C 的 prepare_palette）。 */
pub fn prepare_palette() {
    let mut number: i16 = NUMBER_OF_ELEMENTS as i16;

    unsafe {
        /* 对每个唯一对编号，告诉 ncurses 颜色组合。 */
        let ink = (*openfile).syntax;
        if !ink.is_null() {
            let mut color = (*ink).color;
            while !color.is_null() {
                if (*color).pairnum > number {
                    init_pair((*color).pairnum as i32, (*color).fg, (*color).bg);
                    number = (*color).pairnum;
                }
                color = (*color).next;
            }
        }

        have_palette = true;
    }
}

/* 尝试匹配给定 shibboleth 字符串与正则列表中的某一个，成功返回 true。
 * 对应 C 的 found_in_list。 */
pub fn found_in_list(head: *mut regexlisttype, shibboleth: &str) -> bool {
    unsafe {
        let mut item = head;
        while !item.is_null() {
            if let Some(rgx) = (*item).one_rgx.as_ref() {
                if rgx.is_match(shibboleth) {
                    return true;
                }
            }
            item = (*item).next;
        }
    }
    false
}

/* 查找适用于当前缓冲区的语法，基于文件名或缓冲区内容，
 * 并在需要时加载并预备该语法（对应 C 的 find_and_prime_applicable_syntax）。 */
pub fn find_and_prime_applicable_syntax() {
    let mut sntx: *mut syntaxtype = std::ptr::null_mut();

    unsafe {
        /* 若未读取 rcfile 或其中没有语法，退出。 */
        if syntaxes.is_null() {
            return;
        }

        /* 若指定了语法覆盖字符串，则使用它。 */
        if let Some(syntaxstr_ref) = syntaxstr.as_ref() {
            /* 覆盖为 "none" 等价于没有任何语法。 */
            if syntaxstr_ref == "none" {
                return;
            }

            let mut cur = syntaxes;
            while !cur.is_null() {
                if (*cur).name.as_deref() == Some(syntaxstr_ref.as_str()) {
                    sntx = cur;
                    break;
                }
                cur = (*cur).next;
            }

            if sntx.is_null() && !inhelp {
                statusline(
                    message_type::ALERT,
                    &format!("Unknown syntax name: {}", syntaxstr_ref),
                );
            }
        }

        /* 若未指定语法覆盖字符串，或它未匹配，则尝试基于文件名（扩展名）查找。 */
        if sntx.is_null() && !inhelp {
            let fullname = match (*openfile).filename.as_ref() {
                Some(name) => get_full_path(name).unwrap_or_else(|| name.clone()),
                None => String::new(),
            };

            let mut cur = syntaxes;
            while !cur.is_null() {
                if found_in_list((*cur).extensions, &fullname) {
                    sntx = cur;
                    break;
                }
                cur = (*cur).next;
            }
        }

        /* 若文件名未匹配任何内容，尝试第一行。 */
        if sntx.is_null() && !inhelp {
            let head = (*openfile).filetop;
            let data = if head.is_null() {
                ""
            } else {
                (*head).data.as_str()
            };
            let mut cur = syntaxes;
            while !cur.is_null() {
                if found_in_list((*cur).headers, data) {
                    sntx = cur;
                    break;
                }
                cur = (*cur).next;
            }
        }

        /* HAVE_LIBMAGIC 分支：本仓库当前不可用 libmagic，跳过。 */

        /* 若完全未匹配，查看是否存在默认语法。 */
        if sntx.is_null() && !inhelp {
            let mut cur = syntaxes;
            while !cur.is_null() {
                if (*cur).name.as_deref() == Some("default") {
                    sntx = cur;
                    break;
                }
                cur = (*cur).next;
            }
        }

        /* 当语法尚未加载时，解析它并初始化其颜色。 */
        if !sntx.is_null() && !(*sntx).filename.is_none() {
            parse_one_include((*sntx).filename.as_ref().unwrap(), sntx);
            set_syntax_colorpairs(sntx);
        }

        (*openfile).syntax = sntx;
    }
}

/* 判断多行正则的匹配是否仍然相同，若不同则调度屏幕刷新，
 * 以便重新绘制（对应 C 的 check_the_multis）。 */
pub fn check_the_multis(line: *mut linestruct) {
    unsafe {
        /* 若无语法或无多行正则，则无事可做。 */
        if (*openfile).syntax.is_null() || (*(*openfile).syntax).multiscore == 0 {
            return;
        }

        if (*line).multidata.is_none() {
            refresh_needed = true;
            return;
        }

        let mut ink = (*(*openfile).syntax).color;
        while !ink.is_null() {
            /* 若不是多行正则，跳过。 */
            if (*ink).end.is_none() {
                ink = (*ink).next;
                continue;
            }

            let astart = match (*ink).start.as_ref() {
                Some(rgx) => rgx.find((*line).data.as_str()).is_some(),
                None => false,
            };
            let afterstart = if astart {
                match (*ink).start.as_ref().unwrap().find((*line).data.as_str()) {
                    Some(m) => &(*line).data[m.end()..],
                    None => "",
                }
            } else {
                (*line).data.as_str()
            };
            let anend = match (*ink).end.as_ref() {
                Some(rgx) => rgx.find(afterstart).is_some(),
                None => false,
            };

            /* 检查 multidata 是否仍匹配当前情况。 */
            let id = (*ink).id as usize;
            let md = (*line).multidata.as_ref().unwrap()[id];
            if md == (NOTHING as i16) {
                if !astart {
                    ink = (*ink).next;
                    continue;
                }
            } else if md == (WHOLELINE as i16) {
                /* 确保检测到的开始匹配不是实际的结束匹配。 */
                let end_on_full = match (*ink).end.as_ref() {
                    Some(rgx) => rgx.find((*line).data.as_str()).is_some(),
                    None => false,
                };
                if !anend && (!astart || !end_on_full) {
                    ink = (*ink).next;
                    continue;
                }
            } else if md == (JUSTONTHIS as i16) {
                let start_on_after = match (*ink).start.as_ref() {
                    Some(rgx) => {
                        let base = (*ink).start.as_ref().unwrap().find((*line).data.as_str());
                        match base {
                            Some(m) => {
                                let rest = &(*line).data[m.end()..];
                                rgx.find(rest).is_some()
                            }
                            None => false,
                        }
                    }
                    None => false,
                };
                if astart && anend && !start_on_after {
                    ink = (*ink).next;
                    continue;
                }
            } else if md == (STARTSHERE as i16) {
                if astart && !anend {
                    ink = (*ink).next;
                    continue;
                }
            } else if md == (ENDSHERE as i16) {
                if !astart && anend {
                    ink = (*ink).next;
                    continue;
                }
            }

            /* 有差异，因此重新绘制。 */
            refresh_needed = true;
            perturbed = true;
            return;
        }
    }
}

/* 预计算多行开始和结束正则信息，以加速渲染
 * （对应 C 的 precalc_multicolorinfo）。 */
pub fn precalc_multicolorinfo() {
    unsafe {
        if (*openfile).syntax.is_null()
            || (*(*openfile).syntax).multiscore == 0
            || ISSET(NO_SYNTAX)
        {
            return;
        }

        /* 为每行分配多行正则信息的缓存空间。 */
        let mut line = (*openfile).filetop;
        while !line.is_null() {
            if (*line).multidata.is_none() {
                let size = (*(*openfile).syntax).multiscore as usize * std::mem::size_of::<i16>();
                let buf = nmalloc(size);
                (*line).multidata = Some(
                    buf.chunks_exact(std::mem::size_of::<i16>())
                        .map(|c| i16::from_ne_bytes([c[0], c[1]]))
                        .collect(),
                );
            }
            line = (*line).next;
        }

        let mut ink = (*(*openfile).syntax).color;
        while !ink.is_null() {
            /* 若不是多行正则，跳过。 */
            if (*ink).end.is_none() {
                ink = (*ink).next;
                continue;
            }

            line = (*openfile).filetop;
            while !line.is_null() {
                let id = (*ink).id as usize;
                let mut index: usize = 0;

                /* 假设没有任何匹配，直到下面证明。 */
                if let Some(ref mut md) = (*line).multidata {
                    md[id] = NOTHING as i16;
                }

                /* 当行包含开始匹配时，查找结束匹配，若找到则标记所有受影响的行。 */
                loop {
                    let start_match = match (*ink).start.as_ref() {
                        Some(rgx) => {
                            let rest = &(*line).data.as_str()[index..];
                            rgx.find(rest)
                        }
                        None => None,
                    };
                    if start_match.is_none() {
                        break;
                    }
                    let sm = start_match.unwrap();

                    /* 在开始匹配之后开始查找结束匹配。 */
                    index += sm.end();

                    let end_match = match (*ink).end.as_ref() {
                        Some(rgx) => {
                            let rest = &(*line).data.as_str()[index..];
                            rgx.find(rest)
                        }
                        None => None,
                    };

                    if let Some(em) = end_match {
                        /* 若同一行上有结束匹配，标记该行，但继续查找其后的其它开始。 */
                        if let Some(ref mut md) = (*line).multidata {
                            md[id] = JUSTONTHIS as i16;
                        }

                        index += em.end();

                        /* 若总匹配长度为 0，强制前进。 */
                        if sm.end() - sm.start() + em.end() == 0 {
                            /* 在行尾时，没有其它开始。 */
                            if index >= (*line).data.len() {
                                break;
                            }
                            index = step_right((*line).data.as_bytes(), index);
                        }

                        continue;
                    }

                    /* 在后续行上查找结束匹配。 */
                    let mut tailline = (*line).next;
                    while !tailline.is_null() {
                        let found = match (*ink).end.as_ref() {
                            Some(rgx) => rgx.find((*tailline).data.as_str()).is_some(),
                            None => false,
                        };
                        if found {
                            break;
                        }
                        tailline = (*tailline).next;
                    }

                    if let Some(ref mut md) = (*line).multidata {
                        md[id] = STARTSHERE as i16;
                    }

                    /* 注意：这也推进了主循环中的行。 */
                    let mut mid = (*line).next;
                    while !mid.is_null() && mid != tailline {
                        if let Some(ref mut md) = (*mid).multidata {
                            md[id] = WHOLELINE as i16;
                        }
                        mid = (*mid).next;
                    }

                    if tailline.is_null() {
                        line = (*openfile).filebot;
                        break;
                    }

                    if let Some(ref mut md) = (*tailline).multidata {
                        md[id] = ENDSHERE as i16;
                    }

                    /* 在结束匹配之后查找可能的新开始。 */
                    let em = match (*ink).end.as_ref() {
                        Some(rgx) => rgx.find(&(*tailline).data.as_str()[index..]),
                        None => None,
                    };
                    index = em.map_or(index, |m| m.end());
                }

                line = (*line).next;
            }

            ink = (*ink).next;
        }
    }
}
