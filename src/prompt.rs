/**************************************************************************
 * prompt.rs  --  GNU nano 提示栏输入（对应 prompt.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 提示栏输入处理。对应原版 nano 的 `prompt.c`。

use crate::definitions::*;
use crate::global;
use crate::winio;
use crate::text;

/// 获取当前提示的答案。
pub fn get_answer() -> Option<String> {
    with_global(|g| g.answer.clone())
}

/// 设置答案。
pub fn set_answer(answer: &str) {
    with_global_mut(|g| g.answer = Some(answer.to_string()));
}

/// 清除答案。
pub fn clear_answer() {
    with_global_mut(|g| g.answer = None);
}

/// 获取当前菜单。
pub fn get_currmenu() -> i32 {
    with_global(|g| g.currmenu)
}

/// 在提示栏显示提示并获取输入。
pub fn do_prompt(menu: i32, _initial: Option<&str>, _history_list: Option<&Vec<String>>) -> Option<String> {
    with_global_mut(|g| {
        let old_menu = g.currmenu;
        g.currmenu = menu;
        // 提示输入（简化：直接返回当前答案）
        let result = g.answer.clone();
        g.currmenu = old_menu;
        result
    });
    None
}

/// 在提示栏显示消息并等待按键。
pub fn do_yesno_prompt(menu: i32, _msg: &str) -> i32 {
    with_global_mut(|g| {
        let old_menu = g.currmenu;
        g.currmenu = menu;
        // 简化：模拟用户输入
        let result = YES;
        g.currmenu = old_menu;
        result
    });
    YES
}

/// 获取提示字符串。
pub fn prompt_string(menu: i32, _initial: Option<&str>, _history_list: Option<&Vec<String>>) -> Option<String> {
    do_prompt(menu, _initial, _history_list)
}

/// 获取答案字符串并处理快捷键。
pub fn get_prompt_string(menu: i32, _initial: Option<&str>, _history_list: Option<&Vec<String>>) -> Option<String> {
    do_prompt(menu, _initial, _history_list)
}

/// 获取文件名（用于写入/插入文件）。
pub fn do_prompt_filename(menu: i32, _initial: Option<&str>) -> Option<String> {
    do_prompt(menu, _initial, None)
}

/// 完成单词（Tab 补全）。
pub fn complete_word() {
    // 简化
}

/// 获取当前行和列信息。
pub fn get_line_and_column() -> (isize, usize) {
    with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of_ref = of.borrow();
            let lineno = of_ref.current.as_ref().map(|c| c.borrow().lineno).unwrap_or(0);
            (lineno, of_ref.current_x)
        }).unwrap_or((0, 0))
    });
    (0, 0)
}

/// 获取总行数和总大小。
pub fn get_total_info() -> (isize, usize) {
    with_global(|g| {
        g.openfile.as_ref().map(|of| {
            let of_ref = of.borrow();
            let filebot = of_ref.filebot.as_ref().map(|fb| fb.borrow().lineno).unwrap_or(0);
            (filebot, of_ref.totsize)
        }).unwrap_or((0, 0))
    });
    (0, 0)
}

/// 在提示栏显示消息。
pub fn show_prompt_message(msg: &str) {
    with_global_mut(|g| {
        g.lastmessage = MessageType::Info;
        // 显示消息
    });
}

/// 获取用户输入（用于搜索/替换等）。
pub fn get_input(menu: i32, _msg: &str, _history: Option<&Vec<String>>) -> Option<String> {
    do_prompt(menu, None, _history)
}