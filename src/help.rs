/**************************************************************************
 * help.rs  --  GNU nano 帮助系统（对应 help.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 帮助系统。对应原版 nano 的 `help.c`。

use crate::definitions::*;
use crate::global;
use crate::movement;
use crate::winio;
use std::io::Write;

/// 当前帮助页面的状态。
pub struct HelpState {
    pub start_of_body: Option<String>,
    pub end_of_intro: usize,
    pub location: usize,
}

/// 帮助文本行。
const HELP_TEXT: &[&str] = &[
    "GNU nano 帮助文本",
    "================",
    "",
    "nano 是一个小巧而友好的文本编辑器。",
    "",
    "基本快捷键：",
    "  Ctrl+G 或 F1    显示帮助",
    "  Ctrl+X 或 F2    退出 nano",
    "  Ctrl+O 或 F3    保存文件",
    "  Ctrl+W 或 F4    搜索文本",
    "  Ctrl+K 或 F9    剪切行",
    "  Ctrl+U 或 F10   粘贴",
    "  Ctrl+C 或 F11   显示光标位置",
    "  Ctrl+J 或 F12   对齐段落",
    "",
    "移动快捷键：",
    "  方向键          移动光标",
    "  Ctrl+A          行首",
    "  Ctrl+E          行尾",
    "  Ctrl+Y          上翻一页",
    "  Ctrl+V          下翻一页",
    "  Ctrl+Space      下一个单词",
    "  Alt+Space       上一个单词",
    "",
    "编辑快捷键：",
    "  Ctrl+D          删除光标下字符",
    "  Ctrl+H          退格删除",
    "  Ctrl+I          插入制表符",
    "  Ctrl+M          插入换行",
    "",
    "其他功能：",
    "  Ctrl+R          插入文件",
    "  Ctrl+T          拼写检查",
    "  Ctrl+\\          替换文本",
    "  Ctrl+6          设置/清除标记",
    "  Alt+A           设置/清除标记",
    "  Ctrl+_          跳转到行",
    "  Alt+U           撤销",
    "  Alt+E           重做",
    "",
    "文件浏览器快捷键：",
    "  Ctrl+S          跳转到目录",
    "  Ctrl+N          新建缓冲区",
    "",
    "按 Ctrl+X 关闭帮助",
];

/// 打开帮助。
pub fn do_help() {
    with_global_mut(|g| {
        g.inhelp = true;
        g.currmenu = MHELP;
    });

    // 显示帮助文本
    display_help();

    // 等待用户按键退出
    loop {
        let key = winio::wgetch();
        if key == 24 || key == KEY_ENTER || key == KEY_HOME { // Ctrl+X 或 Enter 退出
            break;
        }
        match key {
            KEY_UP | KEY_PPAGE => movement::do_page_up(),
            KEY_DOWN | KEY_NPAGE => movement::do_page_down(),
            KEY_HOME => movement::do_first_line(),
            KEY_END => movement::do_last_line(),
            _ => {}
        }
    }

    with_global_mut(|g| {
        g.inhelp = false;
        g.currmenu = MMAIN;
    });
}

/// 显示帮助文本。
fn display_help() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::All));

    for (i, line) in HELP_TEXT.iter().enumerate() {
        let _ = writeln!(stdout, "{}", line);
        if i >= 20 { // 一页显示 20 行
            break;
        }
    }
    let _ = stdout.flush();
}

/// 在帮助中搜索。
pub fn do_find_in_help() {
    // 简化：在帮助文本中搜索
}

/// 帮助文本中使用的函数（用于搜索、移动等）。
pub fn help_function(_func: FunctionId) -> bool {
    false
}

/// 获取帮助文本的行数。
pub fn help_lines() -> usize {
    HELP_TEXT.len()
}

/// 获取帮助文本的指定行。
pub fn help_line(n: usize) -> &'static str {
    if n < HELP_TEXT.len() {
        HELP_TEXT[n]
    } else {
        ""
    }
}