/**************************************************************************
 * cut.rs  --  GNU nano 剪切/粘贴（对应 cut.c）
 * 版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 剪切、删除、粘贴与拷贝文本，完整移植自 `cut.c`。
//!
//! 转换说明：
//! - 行链表用 `Rc<RefCell<LineStruct>>`，`cutbuffer`/`cutbottom` 等全局
//!   指针放入 [`GlobalState`]；
//! - `memmove`/`strcat` 等字节操作改用 `String`/`Vec<u8>` 的等价操作；
//! - undo 相关调用（`add_undo`/`update_undo`）委托给 [`crate::text`]；
//! - `check_the_multis`/`precalc_multicolorinfo` 委托给 [`crate::color`]；
//! - `do_left`/`do_prev_word`/`do_next_word` 委托给 [`crate::movement`]。

use crate::definitions::*;
use crate::chars;
use crate::movement;
use crate::files;
use crate::utils;
use crate::winio;
use std::rc::Rc;

/// 获取当前打开的缓冲区引用（克隆 Rc，释放全局借用）。
fn openfile_ref() -> OpenFileRef {
    with_global(|g| g.openfile.as_ref().expect("no open file").clone())
}

/// 删除当前字符，并为给定动作添加或更新一个 undo 项
/// （对应 `expunge`）。
pub fn expunge(action: UndoType) {
    let of = openfile_ref();
    let current = { let r = of.borrow(); r.current.clone().unwrap() };

    /* 读取需要的当前状态（短借用）。 */
    let (current_x, last_action, undo_head_lineno, totsize) = {
        let r = of.borrow();
        (
            r.current_x,
            r.last_action,
            r.current_undo.as_ref().map(|u| u.borrow().head_lineno).unwrap_or(-1),
            r.totsize,
        )
    };
    let _ = totsize;

    let data = current.borrow().data.clone();
    let bytes = data.as_bytes();

    /* 位于行尾且不在缓冲区末尾：把本行与下一行连接。 */
    let at_eol = bytes.get(current_x).copied().unwrap_or(0) == 0;
    let joining: Option<LineRef> = if at_eol {
        let r = current.borrow();
        r.next.clone()
    } else {
        None
    };

    if !at_eol {
        /* 在行中间时，删除当前字符。 */
        let charlen = chars::char_length(&bytes[current_x..]);
        let old_amount = if ISSET(SOFTWRAP) {
            winio::extra_chunks_in(&current)
        } else {
            0
        };

        /* 若动作类型改变或光标移到另一行，创建新 undo 项，否则更新。 */
        if action != last_action || current.borrow().lineno != undo_head_lineno {
            crate::text::add_undo(action, None);
        } else {
            crate::text::update_undo(action);
        }

        /* 将行的其余部分"移入"，覆盖当前字符。 */
        {
            let mut ndata = current.borrow().data.clone().into_bytes();
            ndata.drain(current_x..current_x + charlen);
            current.borrow_mut().data = String::from_utf8_lossy(&ndata).into_owned();
        }

        /* 软换行时块数改变需要刷新；平移时接近视口边缘也需刷新。 */
        let mut need_refresh = false;
        if ISSET(SOFTWRAP) && winio::extra_chunks_in(&current) != old_amount {
            need_refresh = true;
        }
        let placewewant = of.borrow().placewewant;
        let brink = of.borrow().brink;
        let united = with_global(|g| g.united_sidescroll);
        if !need_refresh && united && placewewant < brink + CUSHION {
            need_refresh = true;
        }
        if need_refresh {
            with_global_mut(|g| g.refresh_needed = true);
        }

        /* 调整光标之后同一行的标记。 */
        {
            let mut r = of.borrow_mut();
            if r.mark.as_ref().map(|m| Rc::ptr_eq(m, &current)).unwrap_or(false)
                && r.mark_x > r.current_x
            {
                r.mark_x -= charlen;
            }
            r.totsize -= 1;
            if let Some(u) = &r.current_undo {
                u.borrow_mut().newsize = r.totsize;
            }
        }
    } else if let Some(joining) = joining {
        let is_filebot = {
            let r = of.borrow();
            r.filebot.as_ref().map(|b| Rc::ptr_eq(b, &joining)).unwrap_or(false)
        };

        /* 若有魔法行且位于其前：不吞掉它。 */
        if is_filebot && current_x != 0 && !ISSET(NO_NEWLINES) {
            if action == UndoType::Back {
                crate::text::add_undo(UndoType::Back, None);
            }
            return;
        }

        crate::text::add_undo(action, None);

        /* 调整位于将被"吞掉"的行上的标记。 */
        {
            let mut r = of.borrow_mut();
            if r.mark.as_ref().map(|m| Rc::ptr_eq(m, &joining)).unwrap_or(false) {
                r.mark = Some(current.clone());
                r.mark_x += r.current_x;
            }
        }

        let cur_has_anchor = current.borrow().has_anchor;
        let join_has_anchor = joining.borrow().has_anchor;
        current.borrow_mut().has_anchor = cur_has_anchor || join_has_anchor;

        /* 将下一行的内容添加到当前行的内容。 */
        {
            let mut cdata = current.borrow().data.clone();
            let jdata = joining.borrow().data.clone();
            cdata.push_str(&jdata);
            current.borrow_mut().data = cdata;
        }

        files::unlink_node(&joining);

        /* 连接了两行，需要重新编号并刷新屏幕。 */
        files::renumber_from(&current);
        with_global_mut(|g| g.refresh_needed = true);

        /* 调整文件大小。 */
        {
            let mut r = of.borrow_mut();
            r.totsize -= 1;
            if let Some(u) = &r.current_undo {
                u.borrow_mut().newsize = r.totsize;
            }
        }
    } else {
        /* 位于文件末尾：无事可做。 */
        return;
    }

    files::set_modified();
}

/// 删除光标下的字符及之后的零宽字符；
/// 或当标记开启且 --zap 生效时删除标记区域（对应 `do_delete`）。
pub fn do_delete() {
    if with_global(|g| g.openfile.as_ref().map(|of| {
        let of = of.borrow();
        of.mark.is_some() && ISSET(LET_THEM_ZAP)
    }).unwrap_or(false)) {
        zap_text();
    } else {
        expunge(UndoType::Del);
        /* 同时删除零宽（组合）字符。 */
        loop {
            let at_pos_is_zero = with_global(|g| {
                g.openfile.as_ref().map(|of| {
                    let of = of.borrow();
                    of.current.as_ref().map(|c| {
                        let data = c.borrow().data.clone();
                        let x = of.current_x;
                        let nonzero = data.as_bytes().get(x).copied().unwrap_or(0) != 0;
                        nonzero && chars::is_zerowidth(&data.as_bytes()[x..])
                    }).unwrap_or(false)
                }).unwrap_or(false)
            });
            if !at_pos_is_zero {
                break;
            }
            expunge(UndoType::Del);
        }
    }
}

/// 退格删除一个字符：光标左移一个字符并删除光标下的字符；
/// 或当标记开启且 --zap 生效时删除标记区域（对应 `do_backspace`）。
pub fn do_backspace() {
    if with_global(|g| g.openfile.as_ref().map(|of| {
        let of = of.borrow();
        of.mark.is_some() && ISSET(LET_THEM_ZAP)
    }).unwrap_or(false)) {
        zap_text();
    } else {
        let (x_gt_0, not_filetop) = with_global(|g| {
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                let x = of.current_x;
                let is_filetop = of.filetop.as_ref().map(|t| {
                    of.current.as_ref().map(|c| Rc::ptr_eq(t, c)).unwrap_or(false)
                }).unwrap_or(false);
                (x > 0, !is_filetop)
            }).unwrap_or((false, false))
        });

        if x_gt_0 {
            with_global_mut(|g| {
                let of = g.openfile.as_ref().expect("no open file").clone();
                let mut of = of.borrow_mut();
                let current = of.current.clone().unwrap();
                let data = current.borrow().data.clone();
                of.current_x = chars::step_left(data.as_bytes(), of.current_x);
            });
            expunge(UndoType::Back);
        } else if not_filetop {
            movement::do_left();
            expunge(UndoType::Back);
        }
    }
}

/// 返回 FALSE 当剪切命令实际上不会剪掉任何东西：位于文件末尾的空行、
/// 标记覆盖零个字符，或（test_cliff 为 TRUE 时）会剪掉魔法行
/// （对应 `is_cuttable`）。
pub fn is_cuttable(test_cliff: bool) -> bool {
    let cuttable = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let from = if test_cliff { of.current_x } else { 0 };
            let current = of.current.clone().unwrap();
            let data = current.borrow().data.clone();
            let is_last = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, &current)).unwrap_or(false);

            if (is_last && data.as_bytes().get(from).copied().unwrap_or(0) == 0 && of.mark.is_none())
                || (of.mark.as_ref().map(|m| Rc::ptr_eq(m, &current)).unwrap_or(false)
                    && of.mark_x == of.current_x)
                || (from > 0 && !ISSET(NO_NEWLINES)
                    && data.as_bytes().get(from).copied().unwrap_or(0) == 0
                    && current.borrow().next.as_ref().map(|n| {
                        of.filebot.as_ref().map(|b| Rc::ptr_eq(b, n)).unwrap_or(false)
                    }).unwrap_or(false))
            {
                false
            } else {
                true
            }
        }).unwrap_or(false)
    });

    if !cuttable {
        winio::statusbar("Nothing was cut");
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                of.borrow_mut().mark = None;
            }
        });
    }
    cuttable
}

/// 从光标处删除文本，直到左边（forward 为 FALSE）或右边（TRUE）
/// 下一个单词的起始处（对应 `chop_word`）。
pub fn chop_word(forward: bool) {
    /* 记住当前光标位置。 */
    let (was_current, was_x) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        (of.current.clone().unwrap(), of.current_x)
    });

    /* 记住 cutbuffer 的位置，然后使其看似空白。 */
    let is_cutbuffer = with_global(|g| g.cutbuffer.clone());
    with_global_mut(|g| g.cutbuffer = None);

    /* 将光标移动到单词起始处（左或右）。 */
    if !forward {
        movement::do_prev_word();
        let moved = with_global(|g| {
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                of.current.as_ref().map(|c| !Rc::ptr_eq(c, &was_current)).unwrap_or(false)
            }).unwrap_or(false)
        });
        if moved {
            let was_x_gt_0 = was_x > 0;
            if was_x_gt_0 {
                with_global_mut(|g| {
                    let of = g.openfile.as_ref().unwrap().clone();
                    let mut of = of.borrow_mut();
                    of.current = Some(was_current.clone());
                    of.current_x = 0;
                });
            } else {
                with_global_mut(|g| {
                    let of = g.openfile.as_ref().unwrap().clone();
                    let mut of = of.borrow_mut();
                    let current = of.current.clone().unwrap();
                    let len = current.borrow().data.len();
                    of.current_x = len;
                });
            }
        }
    } else {
        movement::do_next_word(ISSET(AFTER_ENDS));
        let moved = with_global(|g| {
            g.openfile.as_ref().map(|of| {
                let of = of.borrow();
                of.current.as_ref().map(|c| !Rc::ptr_eq(c, &was_current)).unwrap_or(false)
            }).unwrap_or(false)
        });
        if moved {
            let was_current_has_text = {
                let d = was_current.borrow().data.clone();
                d.as_bytes().get(was_x).copied().unwrap_or(0) != 0
            };
            if was_current_has_text {
                with_global_mut(|g| {
                    let of = g.openfile.as_ref().unwrap().clone();
                    let mut of = of.borrow_mut();
                    of.current = Some(was_current.clone());
                    let len = was_current.borrow().data.len();
                    of.current_x = len;
                });
            }
        }
    }

    /* 在该单词的起始处设置标记。 */
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.mark = of.current.clone();
        of.mark_x = of.current_x;
    });

    /* 把光标放回原处，这样 undo 也会把它放到那里。 */
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        let mut of = of.borrow_mut();
        of.current = Some(was_current.clone());
        of.current_x = was_x;
    });

    /* 现在删除标记区域，一个单词就消失了。 */
    crate::text::add_undo(UndoType::Cut, None);
    do_snip(true, false, false);
    crate::text::update_undo(UndoType::Cut);

    /* 丢弃剪下的单词并恢复 cutbuffer。 */
    files::free_lines(with_global(|g| g.cutbuffer.clone()));
    with_global_mut(|g| g.cutbuffer = is_cutbuffer);
}

/// 向左删除一个单词（对应 `chop_previous_word`）。
pub fn chop_previous_word() {
    let at_top_left = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let is_top = of.filetop.as_ref().map(|t| {
                of.current.as_ref().map(|c| Rc::ptr_eq(t, c)).unwrap_or(false)
            }).unwrap_or(false);
            is_top && of.current_x == 0
        }).unwrap_or(false)
    });
    if at_top_left {
        winio::statusbar("Nothing was cut");
    } else {
        chop_word(false);
    }
}

/// 向右删除一个单词（对应 `chop_next_word`）。
pub fn chop_next_word() {
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().mark = None;
        }
    });
    if is_cuttable(true) {
        chop_word(true);
    }
}

/// 取出给定两点之间的文本并将其添加到 cutbuffer
/// （对应 `extract_segment`）。
pub fn extract_segment(top: &LineRef, top_x: usize, bot: &LineRef, bot_x: usize) {
    let (edittop_lineno, top_lineno, bot_lineno) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let et = of.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(0);
        (et, top.borrow().lineno, bot.borrow().lineno)
    });
    let edittop_inside = edittop_lineno >= top_lineno && edittop_lineno <= bot_lineno;

    let same_line = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            of.borrow().mark.as_ref().map(|m| Rc::ptr_eq(m, top)).unwrap_or(false)
        }).unwrap_or(false)
    });
    let post_marked = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.mark.as_ref().map(|m| {
                let m_line = m.borrow().lineno;
                m_line > top_lineno || (same_line && of.mark_x > top_x)
            }).unwrap_or(false)
        }).unwrap_or(false)
    });

    /* 计算标记区域起点处的锚点状态（top 到 bot 各行）。 */
    let mut had_anchor = top.borrow().has_anchor;
    if !Rc::ptr_eq(top, bot) {
        let mut line = { let r = top.borrow(); r.next.clone() };
        while let Some(l) = line {
            had_anchor |= l.borrow().has_anchor;
            if Rc::ptr_eq(&l, bot) {
                break;
            }
            let next = { let r = l.borrow(); r.next.clone() };
            line = next;
        }
    }

    /* 三种情况：(1) 单行；(2) 整行区域；(3) 一般区域。 */
    let mut taken: LineRef;
    let last: LineRef;

    if Rc::ptr_eq(top, bot) && top_x == bot_x {
        return;
    }

    if Rc::ptr_eq(top, bot) {
        taken = make_new_node(None);
        {
            let d = top.borrow().data.clone();
            let taken_data = String::from_utf8_lossy(&d.as_bytes()[top_x..bot_x]).into_owned();
            let mut bytes = d.into_bytes();
            let mid = bytes.split_off(bot_x);
            let _ = bytes.split_off(top_x);
            let mut newdata = bytes;
            newdata.extend_from_slice(&mid);
            top.borrow_mut().data = String::from_utf8_lossy(&newdata).into_owned();
            taken.borrow_mut().data = taken_data;
        }
        last = taken.clone();
    } else if top_x == 0 && bot_x == 0 {
        taken = top.clone();
        last = make_new_node(None);
        last.borrow_mut().data = String::new();
        last.borrow_mut().has_anchor = bot.borrow().has_anchor;

        /* 摘除 [top, bot) 之间的各行。 */
        let bot_prev = { let r = bot.borrow(); r.prev.clone() };
        if let Some(bp) = bot_prev.as_ref().and_then(|w| w.upgrade()) {
            bp.borrow_mut().next = None;
        }
        last.borrow_mut().prev = bot_prev;

        let top_prev = { let r = top.borrow(); r.prev.clone() };
        bot.borrow_mut().prev = top_prev.clone();
        if let Some(tp) = top_prev.as_ref().and_then(|w| w.upgrade()) {
            tp.borrow_mut().next = Some(bot.clone());
        } else {
            with_global_mut(|g| {
                if let Some(of) = &g.openfile {
                    of.borrow_mut().filetop = Some(bot.clone());
                }
            });
        }

        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                of.borrow_mut().current = Some(bot.clone());
            }
        });
    } else {
        taken = make_new_node(None);
        taken.borrow_mut().data = {
            let d = top.borrow().data.clone();
            d[top_x..].to_string()
        };

        let top_next = { let r = top.borrow(); r.next.clone() };
        taken.borrow_mut().next = top_next.clone();
        if let Some(tn) = &top_next {
            tn.borrow_mut().prev = Some(Rc::downgrade(&taken));
        }

        let bot_next = { let r = bot.borrow(); r.next.clone() };
        top.borrow_mut().next = bot_next.clone();
        if let Some(bn) = &bot_next {
            bn.borrow_mut().prev = Some(Rc::downgrade(top));
        }

        /* 将 top 行截短到 top_x，并把 bot 行的尾部拼接到其后。 */
        {
            let mut td = top.borrow().data.clone().into_bytes();
            td.truncate(top_x);
            let bd = bot.borrow().data.clone().into_bytes();
            td.extend_from_slice(&bd[bot_x..]);
            top.borrow_mut().data = String::from_utf8_lossy(&td).into_owned();
        }

        last = bot.clone();
        let mut ld = bot.borrow().data.clone().into_bytes();
        ld.truncate(bot_x);
        bot.borrow_mut().data = String::from_utf8_lossy(&ld).into_owned();
        bot.borrow_mut().next = None;

        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                of.borrow_mut().current = Some(top.clone());
            }
        });
    }

    /* 从缓冲区大小中减去被取出文本的大小。 */
    let taken_size = utils::number_of_characters_in(&taken, &last);
    let totsize = with_global(|g| {
        g.openfile.as_ref().map(|of| of.borrow().totsize).unwrap_or(0)
    });
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().totsize = totsize.saturating_sub(taken_size);
        }
    });

    /* 若 cutbuffer 当前为空，直接把所有文本移入其中；
     * 否则把文本追加到已有内容之后。 */
    let cutbuffer_empty = with_global(|g| g.cutbuffer.is_none());
    if cutbuffer_empty {
        with_global_mut(|g| {
            g.cutbuffer = Some(taken.clone());
            g.cutbottom = Some(last.clone());
        });
    } else {
        let (cutbottom, inherited_anchor, taken_anchor, taken_next) = with_global(|g| {
            let cutbottom = g.cutbottom.clone().unwrap();
            let inherited_anchor = g.inherited_anchor;
            let taken_anchor = taken.borrow().has_anchor;
            let taken_next = { let r = taken.borrow(); r.next.clone() };
            (cutbottom, inherited_anchor, taken_anchor, taken_next)
        });

        /* 把 taken 的文本合并到 cutbottom。 */
        {
            let taken_data = taken.borrow().data.clone();
            let mut cb = cutbottom.borrow().data.clone();
            cb.push_str(&taken_data);
            cutbottom.borrow_mut().data = cb;
            cutbottom.borrow_mut().has_anchor = taken_anchor && !inherited_anchor;
            cutbottom.borrow_mut().next = taken_next.clone();
        }

        files::delete_node(&taken);

        with_global_mut(|g| {
            g.inherited_anchor = inherited_anchor || taken_anchor;
            if let Some(tn) = taken_next {
                tn.borrow_mut().prev = Some(Rc::downgrade(&cutbottom));
                g.cutbottom = Some(last.clone());
            }
        });
    }

    let of = openfile_ref();
    {
        let mut of = of.borrow_mut();
        of.current_x = top_x;

        of.current.as_ref().map(|c| c.borrow_mut().has_anchor = had_anchor);

        if post_marked || same_line {
            of.mark = of.current.clone();
        }
        if post_marked {
            of.mark_x = of.current_x;
        }

        let is_filebot = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, bot)).unwrap_or(false);
        if is_filebot {
            of.filebot = of.current.clone();
        }
    }

    /* 闭包外调用（避免持有 GLOBAL 借用）。 */
    let current = { let r = of.borrow(); r.current.clone().unwrap() };
    files::renumber_from(&current);

    /* 视口起点在被取区域内时，调整视口。 */
    if edittop_inside {
        winio::adjust_viewport(UpdateType::Stationary);
        with_global_mut(|g| g.refresh_needed = true);
    }

    /* 若文本不以换行结尾而应当有换行，则添加一个。 */
    if !ISSET(NO_NEWLINES) {
        let filebot_empty = {
            let r = of.borrow();
            r.filebot.as_ref().map(|b| {
                b.borrow().data.as_bytes().first().copied().unwrap_or(0) == 0
            }).unwrap_or(false)
        };
        if !filebot_empty {
            utils::new_magicline();
        }
    }
}

/// 将 topline 开始的缓冲区融合到当前文件缓冲区的当前光标位置
/// （对应 `ingraft_buffer`）。
pub fn ingraft_buffer(topline: &LineRef) {
    let (line, xpos, length) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let line = of.current.clone().unwrap();
        let length = line.borrow().data.len();
        (line, of.current_x, length)
    });

    let extralen = topline.borrow().data.len();
    let tailtext = {
        let d = line.borrow().data.clone();
        d[xpos..].to_string()
    };

    let mark_follows = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.mark.as_ref().map(|m| Rc::ptr_eq(m, &line) && !utils::mark_is_before_cursor()).unwrap_or(false)
        }).unwrap_or(false)
    });

    /* 找 topline 链表的末尾。 */
    let mut botline = topline.clone();
    loop {
        let next = { let r = botline.borrow(); r.next.clone() };
        match next {
            Some(n) => botline = n,
            None => break,
        }
    }

    /* 将待嫁接文本的大小加到缓冲区大小。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.totsize += utils::number_of_characters_in(topline, &botline);
        }
    });

    let mut length = if !Rc::ptr_eq(topline, &botline) { xpos } else { length };

    if extralen > 0 {
        /* 在光标处插入 topline 的文本。 */
        let mut ndata = line.borrow().data.clone().into_bytes();
        let topdata = topline.borrow().data.clone().into_bytes();
        ndata.splice(xpos..xpos, topdata.iter().cloned());
        line.borrow_mut().data = String::from_utf8_lossy(&ndata).into_owned();
    }

    if !Rc::ptr_eq(topline, &botline) {
        /* 插入到缓冲区末尾时，更新相关指针。 */
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                let line_next = { let r = line.borrow(); r.next.clone() };
                if line_next.is_none() {
                    of.filebot = Some(botline.clone());
                }
            }
        });

        {
            let mut ld = line.borrow().data.clone().into_bytes();
            ld.truncate(xpos + extralen);
            line.borrow_mut().data = String::from_utf8_lossy(&ld).into_owned();
        }

        /* 将嫁接的各行挂到当前行之后。 */
        let cur_next = { let r = line.borrow(); r.next.clone() };
        botline.borrow_mut().next = cur_next.clone();
        if let Some(cn) = &cur_next {
            cn.borrow_mut().prev = Some(Rc::downgrade(&botline));
        }
        let top_next = { let r = topline.borrow(); r.next.clone() };
        line.borrow_mut().next = top_next.clone();
        if let Some(tn) = &top_next {
            tn.borrow_mut().prev = Some(Rc::downgrade(&line));
        }

        /* 将光标后的文本添加到 botline 末尾。 */
        let mut bd = botline.borrow().data.clone();
        bd.push_str(&tailtext);
        botline.borrow_mut().data = bd;

        /* 把光标放到嫁接文本的末尾。 */
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.current = Some(botline.clone());
                let len = botline.borrow().data.len();
                of.current_x = len;
            }
        });
    } else {
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                of.current_x += extralen;
            }
        });
    }

    /* 需要时更新标记的指针和位置。 */
    if mark_follows {
        with_global_mut(|g| {
            if let Some(of) = &g.openfile {
                let mut of = of.borrow_mut();
                if !Rc::ptr_eq(topline, &botline) {
                    of.mark = Some(botline.clone());
                    of.mark_x += length - xpos;
                } else {
                    of.mark_x += extralen;
                }
            }
        });
    }

    files::delete_node(topline);

    files::renumber_from(&line);

    /* 若文本不以换行结尾而应当有换行，则添加一个。 */
    let need_magicline = {
        let of = openfile_ref();
        let r = of.borrow();
        if !ISSET(NO_NEWLINES) {
            r.filebot.as_ref().map(|b| {
                b.borrow().data.as_bytes().first().copied().unwrap_or(0) != 0
            }).unwrap_or(false)
        } else {
            false
        }
    };
    if need_magicline {
        utils::new_magicline();
    }
}

/// 将给定缓冲区的副本融合到当前文件缓冲区（对应 `copy_from_buffer`）。
pub fn copy_from_buffer(somebuffer: &LineRef) {
    let threshold = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.edittop.as_ref().map(|e| e.borrow().lineno).unwrap_or(0) + g.editwinrows as isize - 1
        }).unwrap_or(0)
    });

    let the_copy = files::copy_buffer(somebuffer);
    ingraft_buffer(&the_copy);

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let of = of.borrow();
            let current_lineno = of.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(0);
            if current_lineno > threshold || ISSET(SOFTWRAP) {
                g.recook = true;
            } else {
                g.perturbed = true;
            }
        }
    });
}

/// 将当前缓冲区中所有标记的文本移入 cutbuffer（对应 `cut_marked_region`）。
pub fn cut_marked_region() {
    let (top, top_x, bot, bot_x) = utils::get_region();
    extract_segment(&top, top_x, &bot, bot_x);
    let pww = utils::xplustabs();
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().placewewant = pww;
        }
    });
}

/// 将文本从当前缓冲区移入 cutbuffer。
/// until_eof 为 TRUE 时把从光标位置到文件末尾的文本移入 cutbuffer；
/// append 为 TRUE（zap 时）总是把剪切内容追加到 cutbuffer
/// （对应 `do_snip`）。
pub fn do_snip(marked: bool, until_eof: bool, append: bool) {
    let of = openfile_ref();

    /* 若剪切不连续，或剪切区域，则清空剪贴板。 */
    let last_action = { let r = of.borrow(); r.last_action };
    if last_action != UndoType::Copy {
        with_global_mut(|g| g.keep_cutbuffer = false);
    }
    let keep_cutbuffer = with_global(|g| g.keep_cutbuffer);
    if (marked || until_eof || !keep_cutbuffer) && !append {
        files::free_lines(with_global(|g| g.cutbuffer.clone()));
        with_global_mut(|g| g.cutbuffer = None);
    }

    /* 现在把相关文本移入 cutbuffer。 */
    let current = { let r = of.borrow(); r.current.clone().unwrap() };
    if until_eof {
        let (filebot, filebot_len, cur_x) = {
            let r = of.borrow();
            let fb = r.filebot.clone().unwrap();
            let len = fb.borrow().data.len();
            (fb, len, r.current_x)
        };
        extract_segment(&current, cur_x, &filebot, filebot_len);
    } else if { let r = of.borrow(); r.mark.is_some() } {
        cut_marked_region();
        with_global_mut(|g2| {
            if let Some(of2) = &g2.openfile {
                of2.borrow_mut().mark = None;
            }
        });
    } else if ISSET(CUT_FROM_CURSOR) {
        /* 不在行尾时，把该行剩余部分移入 cutbuffer；
         * 否则不在缓冲区末尾时，只把"行分隔符"移入。 */
        let cur_x = { let r = of.borrow(); r.current_x };
        let data = current.borrow().data.clone();
        let at_eol = data.as_bytes().get(cur_x).copied().unwrap_or(0) == 0;
        if !at_eol {
            let len = data.len();
            extract_segment(&current, cur_x, &current, len);
        } else {
            let is_filebot = {
                let r = of.borrow();
                r.filebot.as_ref().map(|b| Rc::ptr_eq(b, &current)).unwrap_or(false)
            };
            if !is_filebot {
                let next = { let r = current.borrow(); r.next.clone() }.unwrap();
                extract_segment(&current, cur_x, &next, 0);
                let pww = utils::xplustabs();
                with_global_mut(|g2| {
                    if let Some(of2) = &g2.openfile {
                        of2.borrow_mut().placewewant = pww;
                    }
                });
            }
        }
    } else {
        /* 不在缓冲区末尾时，把一整行移入 cutbuffer；
         * 否则把行尾之前的全部文本移入。 */
        let is_filebot = {
            let r = of.borrow();
            r.filebot.as_ref().map(|b| Rc::ptr_eq(b, &current)).unwrap_or(false)
        };
        if !is_filebot {
            let next = { let r = current.borrow(); r.next.clone() }.unwrap();
            extract_segment(&current, 0, &next, 0);
        } else {
            let len = current.borrow().data.len();
            extract_segment(&current, 0, &current, len);
        }
        with_global_mut(|g2| {
            if let Some(of2) = &g2.openfile {
                of2.borrow_mut().placewewant = 0;
            }
        });
    }

    /* 行操作之后，后续操作应添加到 cutbuffer。 */
    with_global_mut(|g| g.keep_cutbuffer = !marked && !until_eof);

    files::set_modified();
    with_global_mut(|g| {
        g.refresh_needed = true;
        g.perturbed = true;
    });
}

/// 将文本从当前缓冲区移入 cutbuffer（对应 `cut_text`）。
pub fn cut_text() {
    let cuttable = is_cuttable(ISSET(CUT_FROM_CURSOR)
        && with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_none()).unwrap_or(false)));

    if !cuttable {
        return;
    }

    /* 仅当当前项不是 CUT 或当前剪切不与上次剪切连续时才添加新 undo 项。 */
    let need_new = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.last_action != UndoType::Cut || !g.keep_cutbuffer
        }).unwrap_or(true)
    });
    if need_new {
        with_global_mut(|g| g.keep_cutbuffer = false);
        crate::text::add_undo(UndoType::Cut, None);
    }

    let marked = with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false));
    do_snip(marked, false, false);

    crate::text::update_undo(UndoType::Cut);
    winio::wipe_statusbar();
}

/// 从当前光标位置剪切到文件末尾（对应 `cut_till_eof`）。
pub fn cut_till_eof() {
    with_global_mut(|g| g.ran_a_tool = true);

    let nothing = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            let current = of.current.clone().unwrap();
            let data = current.borrow().data.clone();
            let at_eol = data.as_bytes().get(of.current_x).copied().unwrap_or(0) == 0;
            let is_filebot = of.filebot.as_ref().map(|b| Rc::ptr_eq(b, &current)).unwrap_or(false);
            let is_before_magic = !ISSET(NO_NEWLINES) && of.current_x > 0
                && current.borrow().next.as_ref().map(|n| {
                    of.filebot.as_ref().map(|b| Rc::ptr_eq(b, n)).unwrap_or(false)
                }).unwrap_or(false);
            at_eol && (is_filebot || is_before_magic)
        }).unwrap_or(false)
    });

    if nothing {
        winio::statusbar("Nothing was cut");
        return;
    }

    crate::text::add_undo(UndoType::CutToEof, None);
    do_snip(false, true, false);
    crate::text::update_undo(UndoType::CutToEof);
    winio::wipe_statusbar();
}

/// 擦除文本（当前行或标记区域），送入遗忘（对应 `zap_text`）。
pub fn zap_text() {
    /* 记住当前 cutbuffer 以便 zap 后恢复。 */
    let was_cutbuffer = with_global(|g| g.cutbuffer.clone());

    if !is_cuttable(ISSET(CUT_FROM_CURSOR)
        && with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_none()).unwrap_or(false)))
    {
        return;
    }

    /* 仅当当前项不是 ZAP 或当前 zap 不与上次 zap 连续时才添加新 undo 项。 */
    let need_new = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.last_action != UndoType::Zap || !g.keep_cutbuffer
        }).unwrap_or(true)
    });
    if need_new {
        crate::text::add_undo(UndoType::Zap, None);
    }

    /* 使用 ZAP undo 项中的 cutbuffer，以便剪切可被撤销。 */
    with_global_mut(|g| {
        g.cutbuffer = g.openfile.as_ref().and_then(|of| {
            of.borrow().current_undo.clone().and_then(|u| u.borrow().cutbuffer.clone())
        });
    });

    let marked = with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false));
    do_snip(marked, false, true);

    crate::text::update_undo(UndoType::Zap);
    winio::wipe_statusbar();

    with_global_mut(|g| g.cutbuffer = was_cutbuffer);
}

/// 复制标记区域，将其放入 cutbuffer（对应 `copy_marked_region`）。
pub fn copy_marked_region() {
    let (topline, top_x, botline, bot_x) = utils::get_region();

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            of.last_action = UndoType::Other;
            g.keep_cutbuffer = false;
            of.mark = None;
            g.refresh_needed = true;
        }
    });

    if Rc::ptr_eq(&topline, &botline) && top_x == bot_x {
        winio::statusbar("Copied nothing");
        return;
    }

    /* 让被标记的区域看起来像一个独立的缓冲区。 */
    let afterline = { let r = botline.borrow(); r.next.clone() };
    botline.borrow_mut().next = None;
    let saved_byte = {
        let d = botline.borrow().data.clone().into_bytes();
        d.get(bot_x).copied().unwrap_or(0)
    };
    {
        let mut d = botline.borrow().data.clone().into_bytes();
        d.truncate(bot_x);
        d.push(0);
        botline.borrow_mut().data = String::from_utf8_lossy(&d).into_owned();
    }
    let mut topdata = topline.borrow().data.clone().into_bytes();
    let cutbuffer = {
        let shifted = topdata.split_off(top_x);
        topline.borrow_mut().data = String::from_utf8_lossy(&topdata).into_owned();
        // 用裁剪后的 top 行构造副本
        let mut tmp = make_new_node(None);
        tmp.borrow_mut().data = String::from_utf8_lossy(&shifted).into_owned();
        let mut nodes = vec![tmp];
        let mut n = { let r = nodes[0].borrow(); r.next.clone() };
        // 无后继（我们只构造单节点）
        files::copy_buffer(&nodes[0])
    };

    /* 恢复缓冲区的正确状态。 */
    topline.borrow_mut().data = String::from_utf8_lossy(&topdata).into_owned();
    {
        let mut d = botline.borrow().data.clone().into_bytes();
        if saved_byte != 0 {
            if d.len() <= bot_x {
                d.resize(bot_x + 1, 0);
            }
            d[bot_x] = saved_byte;
        }
        botline.borrow_mut().data = String::from_utf8_lossy(&d).into_owned();
    }
    botline.borrow_mut().next = afterline;

    with_global_mut(|g| g.cutbuffer = Some(cutbuffer));
}

/// 将文本从当前缓冲区复制到 cutbuffer。文本可以是标记区域、整行、
/// 从光标到行尾的文本、或只是行分隔符，取决于模式与光标位置
/// （对应 `copy_text`）。
pub fn copy_text() {
    let (at_eol, current, current_x, next_exists) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let current = of.current.clone().unwrap();
        let data = current.borrow().data.clone();
        let at_eol = data.as_bytes().get(of.current_x).copied().unwrap_or(0) == 0;
        let next_exists = { let r = current.borrow(); r.next.is_some() };
        (at_eol, current, of.current_x, next_exists)
    });
    let sans_newline = ISSET(NO_NEWLINES) && !next_exists;
    let from_x = if ISSET(CUT_FROM_CURSOR) { current_x } else { 0 };
    let was_current = current.clone();

    /* 标记开启或上次动作不是 COPY 时，清空 cutbuffer。 */
    let (marked, last_copy) = with_global(|g| {
        (
            g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false),
            g.openfile.as_ref().map(|of| of.borrow().last_action == UndoType::Copy).unwrap_or(false),
        )
    });
    if marked || !last_copy {
        with_global_mut(|g| g.keep_cutbuffer = false);
    }
    let keep = with_global(|g| g.keep_cutbuffer);
    if !keep {
        files::free_lines(with_global(|g| g.cutbuffer.clone()));
        with_global_mut(|g| g.cutbuffer = None);
    }

    winio::wipe_statusbar();

    let marked = with_global(|g| g.openfile.as_ref().map(|of| of.borrow().mark.is_some()).unwrap_or(false));
    if marked {
        copy_marked_region();
        return;
    }

    /* 位于缓冲区最末尾时，无事可做。 */
    if !next_exists && at_eol && (ISSET(CUT_FROM_CURSOR) || current_x == 0
        || with_global(|g| g.cutbuffer.is_some()))
    {
        winio::statusbar("Copied nothing");
        return;
    }

    let addition = make_new_node(None);
    {
        let d = current.borrow().data.clone();
        addition.borrow_mut().data = d[from_x..].to_string();
    }

    let mut sans_newline = sans_newline;
    if ISSET(CUT_FROM_CURSOR) {
        sans_newline = !at_eol;
    }

    let cutbuffer_empty = with_global(|g| g.cutbuffer.is_none());
    if cutbuffer_empty && sans_newline {
        with_global_mut(|g| {
            g.cutbuffer = Some(addition.clone());
            g.cutbottom = Some(addition.clone());
        });
    } else if cutbuffer_empty {
        with_global_mut(|g| {
            g.cutbuffer = Some(addition.clone());
            let cb = make_new_node(Some(&*addition.borrow()));
            cb.borrow_mut().data = String::new();
            cb.borrow_mut().prev = Some(Rc::downgrade(&addition));
            addition.borrow_mut().next = Some(cb.clone());
            g.cutbottom = Some(cb);
        });
    } else if sans_newline {
        let (cutbottom, cb_prev) = with_global(|g| {
            let cutbottom = g.cutbottom.clone().unwrap();
            let cb_prev = { let r = cutbottom.borrow(); r.prev.clone() };
            (cutbottom, cb_prev)
        });
        addition.borrow_mut().prev = cb_prev.clone();
        if let Some(p) = cb_prev.as_ref().and_then(|w| w.upgrade()) {
            p.borrow_mut().next = Some(addition.clone());
        }
        files::delete_node(&cutbottom);
        with_global_mut(|g| g.cutbottom = Some(addition.clone()));
    } else if ISSET(CUT_FROM_CURSOR) {
        with_global_mut(|g| {
            let cutbottom = g.cutbottom.clone().unwrap();
            addition.borrow_mut().prev = Some(Rc::downgrade(&cutbottom));
            cutbottom.borrow_mut().next = Some(addition.clone());
            g.cutbottom = Some(addition.clone());
        });
    } else {
        with_global_mut(|g| {
            let cutbottom = g.cutbottom.clone().unwrap();
            let cb_prev = { let r = cutbottom.borrow(); r.prev.clone() };
            addition.borrow_mut().prev = cb_prev.clone();
            if let Some(p) = cb_prev.as_ref().and_then(|w| w.upgrade()) {
                p.borrow_mut().next = Some(addition.clone());
            }
            addition.borrow_mut().next = Some(cutbottom.clone());
            cutbottom.borrow_mut().prev = Some(Rc::downgrade(&addition));
        });
    }

    /* 需要且可能时，把光标移到下一行。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            if (!ISSET(CUT_FROM_CURSOR) || at_eol) && {
                let next = { let r = of.current.as_ref().unwrap().borrow(); r.next.clone() };
                next.is_some()
            } {
                let next = { let r = of.current.as_ref().unwrap().borrow(); r.next.clone() }.unwrap();
                of.current = Some(next);
                of.current_x = 0;
            } else {
                let len = of.current.as_ref().unwrap().borrow().data.len();
                of.current_x = len;
            }
        }
    });

    winio::edit_redraw(&was_current, UpdateType::Flowing);

    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().last_action = UndoType::Copy;
        }
        g.keep_cutbuffer = true;
    });
}

/// 将 cutbuffer 中的文本复制到当前缓冲区（对应 `paste_text`）。
pub fn paste_text() {
    let was_current = with_global(|g| g.openfile.as_ref().expect("no open file").borrow().current.clone().unwrap());
    let had_anchor = was_current.borrow().has_anchor;
    let was_lineno = was_current.borrow().lineno;
    let mut was_leftedge = 0;

    if with_global(|g| g.cutbuffer.is_none()) {
        winio::statusline(MessageType::Ahem, "Cutbuffer is empty");
        return;
    }

    crate::text::add_undo(UndoType::Paste, None);

    if ISSET(SOFTWRAP) {
        was_leftedge = winio::leftedge_for(utils::xplustabs(), &was_current);
    }

    /* 在光标处把 cutbuffer 文本的副本添加到当前缓冲区。 */
    let cutbuffer = with_global(|g| g.cutbuffer.clone()).unwrap();
    copy_from_buffer(&cutbuffer);

    /* 擦除粘贴文本中的锚点，避免它们扩散。 */
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            let mut of = of.borrow_mut();
            let end = of.current.clone().unwrap();
            let mut line = Some(was_current.clone());
            loop {
                let Some(l) = line else { break };
                if Rc::ptr_eq(&l, &end) {
                    break;
                }
                l.borrow_mut().has_anchor = false;
                let next = { let r = l.borrow(); r.next.clone() };
                line = next;
            }
            was_current.borrow_mut().has_anchor = had_anchor;
        }
    });

    crate::text::update_undo(UndoType::Paste);

    /* 仍在同一行且进行硬换行时，限制宽度。 */
    let still_same_line = with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of = of.borrow();
            of.current.as_ref().map(|c| Rc::ptr_eq(c, &was_current)).unwrap_or(false)
        }).unwrap_or(false)
    });
    if still_same_line && ISSET(BREAK_LONG_LINES) {
        crate::text::do_wrap();
    }

    /* 若粘贴不足一屏，不将光标居中。 */
    if less_than_a_screenful(was_lineno, was_leftedge) {
        with_global_mut(|g| g.focusing = false);
    }

    /* 把期望的 x 位置设为粘贴文本的结束处。 */
    let pww = utils::xplustabs();
    with_global_mut(|g| {
        if let Some(of) = &g.openfile {
            of.borrow_mut().placewewant = pww;
        }
    });

    files::set_modified();
    winio::wipe_statusbar();
    with_global_mut(|g| g.refresh_needed = true);
}

/// 判断粘贴的文本是否不足一屏（对应 winio.c 的 `less_than_a_screenful`）。
pub fn less_than_a_screenful(was_lineno: isize, was_leftedge: usize) -> bool {
    let (current, cur_lineno, edittop, firstcolumn, editwinrows, softwrap) = with_global(|g| {
        let of = g.openfile.as_ref().expect("no open file").borrow();
        let current = of.current.clone().unwrap();
        let cur_lineno = current.borrow().lineno;
        let edittop = of.edittop.clone().unwrap();
        let firstcolumn = of.firstcolumn;
        let editwinrows = g.editwinrows;
        let softwrap = g.flags.isset(SOFTWRAP);
        (current, cur_lineno, edittop, firstcolumn, editwinrows, softwrap)
    });
    let shim = if ISSET(ZERO) && with_global(|g| g.currmenu == MREPLACEWITH || g.currmenu == MYESNO) {
        1
    } else {
        0
    };
    let rows = (editwinrows - 1 - shim) as isize;

    if cur_lineno < was_lineno {
        return false;
    }
    if cur_lineno - was_lineno > rows {
        return false;
    }
    if softwrap {
        let mut line = edittop;
        let mut leftedge = firstcolumn;
        winio::go_forward_chunks(rows as i32, &mut line, &mut leftedge);
        let line_lineno = line.borrow().lineno;
        let mut cur = current;
        let pww = utils::xplustabs();
        if line_lineno < cur_lineno
            || (line_lineno == cur_lineno && leftedge < winio::leftedge_for(pww, &cur))
        {
            return false;
        }
        let _ = was_leftedge;
    }
    true
}

/// SHIM 宏值（用于底栏行数计算）。
fn shim_value_here(g: &GlobalState) -> i32 {
    if ISSET(ZERO) && (g.currmenu == MREPLACEWITH || g.currmenu == MYESNO) {
        1
    } else {
        0
    }
}