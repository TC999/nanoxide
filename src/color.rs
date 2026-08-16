/**************************************************************************
 * color.rs  --  GNU nano 颜色管理（对应 color.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 颜色管理，替代 ncurses 颜色对。对应原版 nano 的 `color.c`。
//! 转换说明：使用 crossterm 颜色 API 替代 ncurses。

use crate::definitions::*;
use crossterm::style::Color;

/// 颜色常量。
pub const A_NORMAL: i32 = 0;
pub const A_REVERSE: i32 = 1;
pub const A_BOLD: i32 = 2;
pub const A_UNDERLINE: i32 = 4;
pub const A_BLINK: i32 = 8;

/// 将 nano 颜色值转换为 crossterm Color。
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
        _ => Color::Reset,
    }
}

/// 设置界面颜色对。
pub fn set_interface_colorpairs() {
    // 使用默认颜色，在 crossterm 中动态设置
}

/// 初始化颜色对（对应 init_pair）。
pub fn init_pair(pairnum: i16, fg: i16, bg: i16) {
    // crossterm 中颜色对是动态的，不需要预初始化
}

/// 获取颜色对属性（对应 COLOR_PAIR）。
pub fn COLOR_PAIR(pairnum: i32) -> i32 {
    pairnum
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