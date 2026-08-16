/**************************************************************************
 * chars.rs  --  GNU nano 字符处理函数（对应 chars.c）
 * 版权 (C) 2001-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 字符处理模块，完整移植自 `chars.c`。
//!
//! 转换说明：
//! - `char *`（NUL 结尾字节串）→ `&[u8]`，字节索引语义与原 C 完全一致；
//! - C 中"读取到 NUL"的语义用辅助函数 [`byte_at`] 模拟（越界读取返回 0），
//!   因此对短切片的安全访问与原 C 对 NUL 结尾字符串的访问等价；
//! - 返回指针位置的函数返回 `Option<usize>`（字节偏移）而非裸指针；
//! - `iswalpha/iswalnum/iswblank/iswpunct/towlower/wcwidth` 等宽字符函数
//!   受"仅允许 std/crossterm/libc"约束，用 `std` 的 Unicode 属性与内置
//!   宽度表替代（见 [`iswblank`]、[`iswpunct`]、[`wcwidth`]）；
//! - `mbstrchr` 中 C 源码对非法 UTF-8 序列存在指针后退甚至死循环的缺陷，
//!   这里用"已访问位置集合"检测并安全退出，正常路径行为完全一致。

use crate::definitions::*;
use std::collections::HashSet;

// ======================== 全局状态访问 ========================

/// 是否使用 UTF-8（对应 C 全局 `using_utf8`）。
pub fn using_utf8() -> bool {
    with_global(|g| g.using_utf8)
}

/// 设置 UTF-8 状态。
pub fn set_using_utf8(val: bool) {
    with_global_mut(|g| g.using_utf8 = val);
}

/// 获取单词字符集（对应 C 全局 `word_chars`）。
pub fn word_chars() -> Option<String> {
    with_global(|g| g.word_chars.clone())
}

/// 获取 `as_an_at` 标志。
pub fn as_an_at() -> bool {
    with_global(|g| g.as_an_at)
}

/// 获取制表符宽度（对应 C 全局 `tabsize`）。
pub fn tabsize() -> usize {
    with_global(|g| g.tabsize)
}

/// 越界读取返回 0，模拟 C 的 NUL 结尾字符串。
#[inline]
pub(crate) fn byte_at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

// ======================== 宽字符工具（替代 wchar 函数） ========================

/// 替代 `iswalpha()`。
fn iswalpha(wc: u32) -> bool {
    char::from_u32(wc).map_or(false, |c| c.is_alphabetic())
}

/// 替代 `iswalnum()`。
fn iswalnum(wc: u32) -> bool {
    char::from_u32(wc).map_or(false, |c| c.is_alphanumeric())
}

/// 替代 `iswblank()`：匹配制表符、空格及 Unicode 空白类字符。
fn iswblank(wc: u32) -> bool {
    matches!(wc, 0x0009 | 0x0020 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000)
}

/// 替代 `iswgraph()`：可见且非空白、非控制。
fn iswgraph(wc: u32) -> bool {
    let c = match char::from_u32(wc) {
        Some(c) => c,
        None => return false,
    };
    wc != 0 && !c.is_whitespace() && !c.is_control()
}

/// 替代 `iswpunct()`：graph 且非字母数字且非符号。
/// （符号类别用常见符号区间近似，见 [`is_symbol`]。）
fn iswpunct(wc: u32) -> bool {
    iswgraph(wc) && !iswalnum(wc) && !is_symbol(wc)
}

/// 常见符号（Unicode 类别 S*）区间表，用于 `iswpunct` 近似。
const SYMBOL_RANGES: &[(u32, u32)] = &[
    (0x20A0, 0x20CF), // 货币符号
    (0x2100, 0x214F), // 字母式符号
    (0x2190, 0x21FF), // 箭头
    (0x2200, 0x22FF), // 数学运算符
    (0x2300, 0x23FF), // 杂项技术符号
    (0x2460, 0x24FF), // 带圈字母数字
    (0x25A0, 0x25FF), // 几何形状
    (0x2600, 0x26FF), // 杂项符号
    (0x2700, 0x27BF), // 装饰符号
    (0x27C0, 0x27EF), // 杂项数学符号 A
    (0x27F0, 0x27FF), // 补充箭头 A
    (0x2900, 0x297F), // 补充箭头 B
    (0x2980, 0x29FF), // 杂项数学符号 B
    (0x2A00, 0x2AFF), // 补充数学运算符
    (0x2B00, 0x2BFF), // 杂项符号和箭头
    (0x1F000, 0x1FAFF), // 表情符号
    (0xFFE0, 0xFFE6), // 全角符号
];

/// 判断宽字符是否为符号（近似）。
fn is_symbol(wc: u32) -> bool {
    SYMBOL_RANGES
        .binary_search_by(|&(lo, hi)| {
            if wc < lo { std::cmp::Ordering::Greater } else if wc > hi { std::cmp::Ordering::Less } else { std::cmp::Ordering::Equal }
        })
        .is_ok()
}

/// 替代 `towlower()`：宽字符转小写（取第一个小写映射）。
fn towlower(wc: u32) -> u32 {
    char::from_u32(wc).map_or(wc, |c| c.to_lowercase().next().map_or(wc, |l| l as u32))
}

/// 零宽（组合字符等）区间表，替代 `wcwidth` 返回 0 的情况。
const ZERO_WIDTH_RANGES: &[(u32, u32)] = &[
    (0x0300, 0x036F), (0x0483, 0x0489), (0x0591, 0x05BD), (0x05BF, 0x05BF),
    (0x05C1, 0x05C2), (0x05C4, 0x05C5), (0x05C7, 0x05C7), (0x0610, 0x061A),
    (0x064B, 0x065F), (0x0670, 0x0670), (0x06D6, 0x06DC), (0x06DF, 0x06E4),
    (0x06E7, 0x06E8), (0x06EA, 0x06ED), (0x0711, 0x0711), (0x0730, 0x074A),
    (0x07A6, 0x07B0), (0x07EB, 0x07F3), (0x07FD, 0x07FD), (0x0816, 0x0819),
    (0x081B, 0x0823), (0x0825, 0x0827), (0x0829, 0x082D), (0x0859, 0x085B),
    (0x08D3, 0x08E1), (0x08E3, 0x0902), (0x093A, 0x093A), (0x093C, 0x093C),
    (0x0941, 0x0948), (0x094D, 0x094D), (0x0951, 0x0957), (0x0962, 0x0963),
    (0x0981, 0x0981), (0x09BC, 0x09BC), (0x09C1, 0x09C4), (0x09CD, 0x09CD),
    (0x09E2, 0x09E3), (0x09FE, 0x09FE), (0x0A01, 0x0A02), (0x0A3C, 0x0A3C),
    (0x0A41, 0x0A42), (0x0A47, 0x0A48), (0x0A4B, 0x0A4D), (0x0A51, 0x0A51),
    (0x0A70, 0x0A71), (0x0A75, 0x0A75), (0x0A81, 0x0A82), (0x0ABC, 0x0ABC),
    (0x0AC1, 0x0AC5), (0x0AC7, 0x0AC8), (0x0ACD, 0x0ACD), (0x0AE2, 0x0AE3),
    (0x0AFA, 0x0AFF), (0x0B01, 0x0B01), (0x0B3C, 0x0B3C), (0x0B3F, 0x0B3F),
    (0x0B41, 0x0B44), (0x0B4D, 0x0B4D), (0x0B55, 0x0B56), (0x0B62, 0x0B63),
    (0x0B82, 0x0B82), (0x0BC0, 0x0BC0), (0x0BCD, 0x0BCD), (0x0C00, 0x0C00),
    (0x0C04, 0x0C04), (0x0C3E, 0x0C40), (0x0C46, 0x0C48), (0x0C4A, 0x0C4D),
    (0x0C55, 0x0C56), (0x0C62, 0x0C63), (0x0C81, 0x0C81), (0x0CBC, 0x0CBC),
    (0x0CBF, 0x0CBF), (0x0CC6, 0x0CC6), (0x0CCC, 0x0CCD), (0x0CE2, 0x0CE3),
    (0x0D00, 0x0D01), (0x0D3B, 0x0D3C), (0x0D41, 0x0D44), (0x0D4D, 0x0D4D),
    (0x0D62, 0x0D63), (0x0D81, 0x0D81), (0x0DCA, 0x0DCA), (0x0DD2, 0x0DD4),
    (0x0DD6, 0x0DD6), (0x0E31, 0x0E31), (0x0E34, 0x0E3A), (0x0E47, 0x0E4E),
    (0x0EB1, 0x0EB1), (0x0EB4, 0x0EBC), (0x0EC8, 0x0ECD), (0x0F18, 0x0F19),
    (0x0F35, 0x0F35), (0x0F37, 0x0F37), (0x0F39, 0x0F39), (0x0F71, 0x0F7E),
    (0x0F80, 0x0F84), (0x0F86, 0x0F87), (0x0F8D, 0x0F97), (0x0F99, 0x0FBC),
    (0x0FC6, 0x0FC6), (0x102D, 0x1030), (0x1032, 0x1037), (0x1039, 0x103A),
    (0x103D, 0x103E), (0x1058, 0x1059), (0x105E, 0x1060), (0x1071, 0x1074),
    (0x1082, 0x1082), (0x1085, 0x1086), (0x108D, 0x108D), (0x109D, 0x109D),
    (0x135D, 0x135F), (0x1712, 0x1714), (0x1732, 0x1734), (0x1752, 0x1753),
    (0x1772, 0x1773), (0x17B4, 0x17B5), (0x17B7, 0x17BD), (0x17C6, 0x17C6),
    (0x17C9, 0x17D3), (0x17DD, 0x17DD), (0x180B, 0x180D), (0x1885, 0x1886),
    (0x18A9, 0x18A9), (0x1920, 0x1922), (0x1927, 0x1928), (0x1932, 0x1932),
    (0x1939, 0x193B), (0x1A17, 0x1A18), (0x1A1B, 0x1A1B), (0x1A56, 0x1A56),
    (0x1A58, 0x1A5E), (0x1A60, 0x1A60), (0x1A62, 0x1A62), (0x1A65, 0x1A6C),
    (0x1A73, 0x1A7C), (0x1A7F, 0x1A7F), (0x1AB0, 0x1AC0), (0x1B00, 0x1B03),
    (0x1B34, 0x1B34), (0x1B36, 0x1B3A), (0x1B3C, 0x1B3C), (0x1B42, 0x1B42),
    (0x1B6B, 0x1B73), (0x1B80, 0x1B81), (0x1BA2, 0x1BA5), (0x1BA8, 0x1BA9),
    (0x1BAB, 0x1BAD), (0x1BE6, 0x1BE6), (0x1BE8, 0x1BE9), (0x1BED, 0x1BED),
    (0x1BEF, 0x1BF1), (0x1C2C, 0x1C33), (0x1C36, 0x1C37), (0x1CD0, 0x1CD2),
    (0x1CD4, 0x1CE0), (0x1CE2, 0x1CE8), (0x1CED, 0x1CED), (0x1CF4, 0x1CF4),
    (0x1CF8, 0x1CF9), (0x1DC0, 0x1DF9), (0x1DFB, 0x1DFF), (0x200B, 0x200F),
    (0x202A, 0x202E), (0x2060, 0x2064), (0x2066, 0x206F), (0x20D0, 0x20F0),
    (0x2CEF, 0x2CF1), (0x2D7F, 0x2D7F), (0x2DE0, 0x2DFF), (0x302A, 0x302D),
    (0x3099, 0x309A), (0xA66F, 0xA672), (0xA674, 0xA67D), (0xA69E, 0xA69F),
    (0xA6F0, 0xA6F1), (0xA802, 0xA802), (0xA806, 0xA806), (0xA80B, 0xA80B),
    (0xA825, 0xA826), (0xA8C4, 0xA8C5), (0xA8E0, 0xA8F1), (0xA8FF, 0xA8FF),
    (0xA926, 0xA92D), (0xA947, 0xA951), (0xA980, 0xA982), (0xA9B3, 0xA9B3),
    (0xA9B6, 0xA9B9), (0xA9BC, 0xA9BC), (0xA9E5, 0xA9E5), (0xAA29, 0xAA2E),
    (0xAA31, 0xAA32), (0xAA35, 0xAA36), (0xAA43, 0xAA43), (0xAA4C, 0xAA4C),
    (0xAA7C, 0xAA7C), (0xAAB0, 0xAAB0), (0xAAB2, 0xAAB4), (0xAAB7, 0xAAB8),
    (0xAABE, 0xAABF), (0xAAC1, 0xAAC1), (0xAAEC, 0xAAED), (0xAAF6, 0xAAF6),
    (0xABE5, 0xABE5), (0xABE8, 0xABE8), (0xABED, 0xABED), (0xFB1E, 0xFB1E),
    (0xFE00, 0xFE0F), (0xFE20, 0xFE2F), (0xFEFF, 0xFEFF), (0xFFF9, 0xFFFB),
    (0x101FD, 0x101FD), (0x102E0, 0x102E0), (0x10376, 0x1037A), (0x10A01, 0x10A0F),
    (0x10A38, 0x10A3F), (0x10AE5, 0x10AE6), (0x11001, 0x11001), (0x11038, 0x11046),
    (0x1107F, 0x11081), (0x110B3, 0x110B6), (0x110B9, 0x110BA), (0x11100, 0x11102),
    (0x11127, 0x1112B), (0x1112D, 0x11134), (0x11173, 0x11173), (0x11180, 0x11181),
    (0x111B6, 0x111BE), (0x111C9, 0x111CC), (0x1122F, 0x11231), (0x11234, 0x11234),
    (0x11236, 0x11237), (0x1123E, 0x1123E), (0x112DF, 0x112DF), (0x112E3, 0x112EA),
    (0x11300, 0x11301), (0x1133B, 0x1133C), (0x11340, 0x11340), (0x11366, 0x1136C),
    (0x11370, 0x11374), (0x11438, 0x1143F), (0x11442, 0x11444), (0x11446, 0x11446),
    (0x1145E, 0x1145E), (0x114B3, 0x114B8), (0x114BA, 0x114BA), (0x114BF, 0x114C0),
    (0x114C2, 0x114C3), (0x115B2, 0x115B5), (0x115BC, 0x115BD), (0x115BF, 0x115C0),
    (0x115DC, 0x115DD), (0x11633, 0x1163A), (0x1163D, 0x1163D), (0x1163F, 0x11640),
    (0x116AB, 0x116AB), (0x116AD, 0x116AD), (0x116B0, 0x116B5), (0x116B7, 0x116B7),
    (0x1171D, 0x1171F), (0x11722, 0x11725), (0x11727, 0x1172B), (0x1182F, 0x11837),
    (0x11839, 0x1183A), (0x119D4, 0x119D7), (0x119DA, 0x119DB), (0x119E0, 0x119E0),
    (0x11A01, 0x11A0A), (0x11A33, 0x11A38), (0x11A3B, 0x11A3E), (0x11A47, 0x11A47),
    (0x11A51, 0x11A56), (0x11A59, 0x11A5B), (0x11A8A, 0x11A96), (0x11A98, 0x11A99),
    (0x11C30, 0x11C36), (0x11C38, 0x11C3D), (0x11C3F, 0x11C3F), (0x11C92, 0x11CA7),
    (0x11CAA, 0x11CB0), (0x11CB2, 0x11CB3), (0x11CB5, 0x11CB6), (0x11D31, 0x11D36),
    (0x11D3A, 0x11D3A), (0x11D3C, 0x11D3D), (0x11D3F, 0x11D45), (0x11D47, 0x11D47),
    (0x11D90, 0x11D91), (0x11D95, 0x11D95), (0x11D97, 0x11D97), (0x11EF3, 0x11EF4),
    (0x16AF0, 0x16AF4), (0x16B30, 0x16B36), (0x16F8F, 0x16F92), (0x1BC9D, 0x1BC9E),
    (0x1D165, 0x1D169), (0x1D16D, 0x1D182), (0x1D185, 0x1D18B), (0x1D1AA, 0x1D1AD),
    (0x1D242, 0x1D244), (0x1DA00, 0x1DA36), (0x1DA3B, 0x1DA6C), (0x1DA75, 0x1DA75),
    (0x1DA84, 0x1DA84), (0x1DA9B, 0x1DA9F), (0x1DAA1, 0x1DAAF), (0x1E000, 0x1E006),
    (0x1E008, 0x1E018), (0x1E01B, 0x1E021), (0x1E023, 0x1E024), (0x1E026, 0x1E02A),
    (0x1E130, 0x1E136), (0x1E2EC, 0x1E2EF), (0x1E8D0, 0x1E8D6), (0x1E944, 0x1E94A),
    (0xE0001, 0xE0001), (0xE0020, 0xE007F), (0xE0100, 0xE01EF),
];

/// 东亚宽/全角字符区间表，替代 `wcwidth` 返回 2 的情况。
const WIDE_RANGES: &[(u32, u32)] = &[
    (0x1100, 0x115F), (0x231A, 0x231B), (0x2329, 0x232A), (0x23E9, 0x23EC),
    (0x23F0, 0x23F0), (0x23F3, 0x23F3), (0x25FD, 0x25FE), (0x2614, 0x2615),
    (0x2648, 0x2653), (0x267F, 0x267F), (0x2693, 0x2693), (0x26A1, 0x26A1),
    (0x26AA, 0x26AB), (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26CE, 0x26CE),
    (0x26D4, 0x26D4), (0x26EA, 0x26EA), (0x26F2, 0x26F3), (0x26F5, 0x26F5),
    (0x26FA, 0x26FA), (0x26FD, 0x26FD), (0x2705, 0x2705), (0x270A, 0x270B),
    (0x2728, 0x2728), (0x274C, 0x274C), (0x274E, 0x274E), (0x2753, 0x2755),
    (0x2757, 0x2757), (0x2795, 0x2797), (0x27B0, 0x27B0), (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C), (0x2B50, 0x2B50), (0x2B55, 0x2B55), (0x2E80, 0x2E99),
    (0x2E9B, 0x2EF3), (0x2F00, 0x2FD5), (0x2FF0, 0x2FFB), (0x3000, 0x303E),
    (0x3041, 0x3096), (0x3099, 0x30FF), (0x3105, 0x312F), (0x3131, 0x318E),
    (0x3190, 0x31E3), (0x31F0, 0x321E), (0x3220, 0x3247), (0x3250, 0x4DBF),
    (0x4E00, 0xA48C), (0xA490, 0xA4C6), (0xA960, 0xA97C), (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF), (0xFE10, 0xFE19), (0xFE30, 0xFE52), (0xFE54, 0xFE66),
    (0xFE68, 0xFE6B), (0xFF00, 0xFF60), (0xFFE0, 0xFFE6), (0x16FE0, 0x16FE4),
    (0x16FF0, 0x16FF1), (0x17000, 0x187F7), (0x18800, 0x18CD5), (0x18D00, 0x18D08),
    (0x1B000, 0x1B11E), (0x1B150, 0x1B152), (0x1B164, 0x1B167), (0x1B170, 0x1B2FB),
    (0x1F004, 0x1F004), (0x1F0CF, 0x1F0CF), (0x1F18E, 0x1F18E), (0x1F191, 0x1F19A),
    (0x1F200, 0x1F202), (0x1F210, 0x1F23B), (0x1F240, 0x1F248), (0x1F250, 0x1F251),
    (0x1F260, 0x1F265), (0x1F300, 0x1F320), (0x1F32D, 0x1F335), (0x1F337, 0x1F37C),
    (0x1F37E, 0x1F393), (0x1F3A0, 0x1F3CA), (0x1F3CF, 0x1F3D3), (0x1F3E0, 0x1F3F0),
    (0x1F3F4, 0x1F3F4), (0x1F3F8, 0x1F43E), (0x1F440, 0x1F440), (0x1F442, 0x1F4FC),
    (0x1F4FF, 0x1F53D), (0x1F54B, 0x1F54E), (0x1F550, 0x1F567), (0x1F57A, 0x1F57A),
    (0x1F595, 0x1F596), (0x1F5A4, 0x1F5A4), (0x1F5FB, 0x1F64F), (0x1F680, 0x1F6C5),
    (0x1F6CC, 0x1F6CC), (0x1F6D0, 0x1F6D2), (0x1F6D5, 0x1F6D7), (0x1F6DC, 0x1F6DF),
    (0x1F6EB, 0x1F6EC), (0x1F6F4, 0x1F6FC), (0x1F7E0, 0x1F7EB), (0x1F7F0, 0x1F7F0),
    (0x1F90C, 0x1F93A), (0x1F93C, 0x1F945), (0x1F947, 0x1F9FF), (0x1FA70, 0x1FA7C),
    (0x1FA80, 0x1FA88), (0x1FA90, 0x1FABD), (0x1FABF, 0x1FAC5), (0x1FACE, 0x1FADB),
    (0x1FAE0, 0x1FAE8), (0x1FAF0, 0x1FAF8), (0x20000, 0x2FFFD), (0x30000, 0x3FFFD),
];

/// 区间表查询。
fn in_ranges(ranges: &[(u32, u32)], wc: u32) -> bool {
    ranges
        .binary_search_by(|&(lo, hi)| {
            if wc < lo {
                std::cmp::Ordering::Greater
            } else if wc > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// 替代 `wcwidth()`：返回字符在终端中占用的列数。
/// 控制字符返回 -1，组合字符返回 0，宽字符返回 2，其余返回 1。
pub fn wcwidth(wc: u32) -> i32 {
    if wc == 0 {
        return 0;
    }
    // C0 与 C1 控制字符
    if wc < 32 || (0x7F..=0xA0).contains(&wc) {
        return -1;
    }
    if in_ranges(ZERO_WIDTH_RANGES, wc) {
        return 0;
    }
    if in_ranges(WIDE_RANGES, wc) {
        return 2;
    }
    1
}

// ======================== 字符分类 ========================

/// 返回 TRUE 当给定字符是某种字母（对应 `is_alpha_char`，SPELLER 用）。
pub fn is_alpha_char(c: &[u8]) -> bool {
    match mbtowide(c) {
        Err(()) => false,
        Ok((wc, _)) => iswalpha(wc),
    }
}

/// 返回 TRUE 当给定字符是字母或数字（对应 `is_alnum_char`）。
pub fn is_alnum_char(c: &[u8]) -> bool {
    match mbtowide(c) {
        Err(()) => false,
        Ok((wc, _)) => iswalnum(wc),
    }
}

/// 返回 TRUE 当给定字符是空格、制表符或其他空白（对应 `is_blank_char`）。
pub fn is_blank_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return false;
    }
    if (c[0] as i8) >= 0 {
        return c[0] == b' ' || c[0] == b'\t';
    }
    match mbtowide(c) {
        Err(()) => false,
        Ok((wc, _)) => iswblank(wc),
    }
}

/// 返回 TRUE 当给定字符是控制字符（对应 `is_cntrl_char`）。
pub fn is_cntrl_char(c: &[u8]) -> bool {
    if c.is_empty() {
        return true; // C 中 NUL 满足 (c[0] & 0xE0) == 0
    }
    if using_utf8() {
        (c[0] & 0xE0) == 0 || c[0] == DEL_CODE
            || ((c[0] as i8) == -62 && (byte_at(c, 1) as i8) < -96)
    } else {
        (c[0] & 0x60) == 0 || c[0] == DEL_CODE
    }
}

/// 返回 TRUE 当给定字符是标点字符（对应 `is_punct_char`）。
pub fn is_punct_char(c: &[u8]) -> bool {
    match mbtowide(c) {
        Err(()) => false,
        Ok((wc, _)) => iswpunct(wc),
    }
}

/// 返回 TRUE 当给定字符是构词字符：字母数字，或（allow_punct 时）标点，
/// 或在 'wordchars' 中指定（对应 `is_word_char`）。
pub fn is_word_char(c: &[u8], allow_punct: bool) -> bool {
    if c.is_empty() || c[0] == 0 {
        return false;
    }
    if is_alnum_char(c) {
        return true;
    }
    if allow_punct && is_punct_char(c) {
        return true;
    }
    if let Some(wc) = word_chars() {
        if !wc.is_empty() {
            let (symlen, symbol) = collect_char(c);
            if symlen == 0 {
                return false;
            }
            let wbytes = wc.as_bytes();
            if symlen <= wbytes.len() {
                for k in 0..=(wbytes.len() - symlen) {
                    if wbytes[k..k + symlen] == symbol[..symlen] {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ======================== 控制字符的可见表示 ========================

/// 返回控制字符 c 的可见表示（对应 `control_rep`）。
pub fn control_rep(c: i8) -> u8 {
    if c == DEL_CODE as i8 {
        b'?'
    } else if c == -97 {
        b'='
    } else if c < 0 {
        (c as i32 + 224) as u8
    } else {
        (c as i32 + 64) as u8
    }
}

/// 返回多字节控制字符 c 的可见表示（对应 `control_mbrep`）。
pub fn control_mbrep(c: &[u8], isdata: bool) -> u8 {
    /* 行内嵌入的换行，若它是数据则显示为编码的 NUL。 */
    if !c.is_empty() && c[0] == b'\n' && (isdata || as_an_at()) {
        return b'@';
    }
    if using_utf8() {
        if !c.is_empty() && c[0] < 128 {
            return control_rep(c[0] as i8);
        } else {
            return control_rep(byte_at(c, 1) as i8);
        }
    }
    control_rep(if c.is_empty() { 0 } else { c[0] as i8 })
}

// ======================== 宽字符转换与长度 ========================

/// 将给定的多字节序列 c 转换为宽字符 wc，返回 `Ok((wc, 字节数))`，
/// 非法序列返回 `Err(())`（对应 `mbtowide`）。
pub fn mbtowide(c: &[u8]) -> Result<(u32, usize), ()> {
    if !c.is_empty() && (c[0] as i8) < 0 && using_utf8() {
        let v1 = c[0];
        let v2 = byte_at(c, 1) ^ 0x80;

        if v2 > 0x3F || v1 < 0xC2 {
            return Err(());
        }

        if v1 < 0xE0 {
            return Ok(((((v1 & 0x1F) as u32) << 6) | v2 as u32, 2));
        }

        let v3 = byte_at(c, 2) ^ 0x80;

        if v3 > 0x3F {
            return Err(());
        }

        if v1 < 0xF0 {
            if (v1 > 0xE0 || v2 >= 0x20) && (v1 != 0xED || v2 < 0x20) {
                return Ok(((((v1 & 0x0F) as u32) << 12) | ((v2 as u32) << 6) | v3 as u32, 3));
            } else {
                return Err(());
            }
        }

        let v4 = byte_at(c, 3) ^ 0x80;

        if v4 > 0x3F || v1 > 0xF4 {
            return Err(());
        }

        if (v1 > 0xF0 || v2 >= 0x10) && (v1 != 0xF4 || v2 < 0x10) {
            return Ok(((((v1 & 0x07) as u32) << 18) | ((v2 as u32) << 12)
                | ((v3 as u32) << 6) | v4 as u32, 4));
        } else {
            return Err(());
        }
    }

    Ok((if c.is_empty() { 0 } else { c[0] as u32 }, 1))
}

/// 返回 TRUE 当给定字符占用两个单元格（对应 `is_doublewidth`）。
pub fn is_doublewidth(ch: &[u8]) -> bool {
    /* 只有从 U+1100 起才可能有双宽。 */
    if ch.is_empty() || ch[0] < 0xE1 || !using_utf8() {
        return false;
    }
    match mbtowide(ch) {
        Err(()) => false,
        Ok((wc, _)) => wcwidth(wc) == 2,
    }
}

/// 返回 TRUE 当给定字符占用零个单元格（对应 `is_zerowidth`）。
pub fn is_zerowidth(ch: &[u8]) -> bool {
    /* 只有从 U+0300 起才可能有零宽。 */
    if ch.is_empty() || ch[0] < 0xCC || !using_utf8() {
        return false;
    }
    match mbtowide(ch) {
        Err(()) => false,
        Ok((wc, _)) => wcwidth(wc) == 0,
    }
}

/// 返回从 *pointer 开始的字符的字节数（对应 `char_length`）。
pub fn char_length(pointer: &[u8]) -> usize {
    if !pointer.is_empty() && pointer[0] > 0xC1 && using_utf8() {
        let c1 = pointer[0];
        let c2 = byte_at(pointer, 1);

        if (c2 ^ 0x80) > 0x3F {
            return 1;
        }

        if c1 < 0xE0 {
            return 2;
        }

        if (byte_at(pointer, 2) ^ 0x80) > 0x3F {
            return 1;
        }

        if c1 < 0xF0 {
            if (c1 > 0xE0 || c2 >= 0xA0) && (c1 != 0xED || c2 < 0xA0) {
                return 3;
            } else {
                return 1;
            }
        }

        if (byte_at(pointer, 3) ^ 0x80) > 0x3F {
            return 1;
        }

        if c1 > 0xF4 {
            return 1;
        }

        if (c1 > 0xF0 || c2 >= 0x90) && (c1 != 0xF4 || c2 < 0x90) {
            return 4;
        }
    }

    1
}

/// 返回给定字符串中（多字节）字符的数量（对应 `mbstrlen`）。
pub fn mbstrlen(pointer: &[u8]) -> usize {
    let mut count = 0;
    let mut pos = 0;
    while byte_at(pointer, pos) != 0 {
        pos += char_length(&pointer[pos..]);
        count += 1;
    }
    count
}

/// 返回给定字符串开头字符的字节数，并复制该字符到返回的缓冲区
/// （对应 `collect_char`）。
pub fn collect_char(string: &[u8]) -> (usize, Vec<u8>) {
    let charlen = char_length(string);
    let mut thechar = vec![0u8; charlen];
    for i in 0..charlen {
        thechar[i] = byte_at(string, i);
    }
    (charlen, thechar)
}

/// 返回给定字符串开头字符的字节数，并将该字符的宽度加到 *column
/// （对应 `advance_over`）。
pub fn advance_over(string: &[u8], column: &mut usize) -> usize {
    if !string.is_empty() && (string[0] as i8) < 0 && using_utf8() {
        /* 一个 UTF-8 高控制码有两个字节、占两列。 */
        if string[0] == 0xC2 && (byte_at(string, 1) as i8) < -96 {
            *column += 2;
            return 2;
        } else {
            match mbtowide(string) {
                Err(()) => {
                    *column += 1;
                    return 1;
                }
                Ok((wc, charlen)) => {
                    let width = wcwidth(wc);
                    *column += if width < 0 { 1 } else { width as usize };
                    return charlen;
                }
            }
        }
    }

    if !string.is_empty() && string[0] < 0x20 {
        if string[0] == b'\t' {
            *column += tabsize() - *column % tabsize();
        } else {
            *column += 2;
        }
    } else if !string.is_empty() && 0x7E < string[0] && string[0] < 0xA0 {
        *column += 2;
    } else {
        *column += 1;
    }

    1
}

/// 返回 buf 中 pos 处字符之前的那个多字节字符的起始索引
/// （对应 `step_left`）。
pub fn step_left(buf: &[u8], pos: usize) -> usize {
    if using_utf8() {
        let before;
        if pos < 4 {
            before = 0;
        } else {
            /* 在前四个字节中探测合法的起始字节。 */
            if (buf[pos - 1] as i8) > -65 {
                before = pos - 1;
            } else if (buf[pos - 2] as i8) > -65 {
                before = pos - 2;
            } else if (buf[pos - 3] as i8) > -65 {
                before = pos - 3;
            } else if (buf[pos - 4] as i8) > -65 {
                before = pos - 4;
            } else {
                before = pos - 1;
            }
        }

        /* 再向前推进直到到达原字符，从而得知其前一个字符的长度。 */
        let mut before = before;
        let mut charlen = 0;
        while before < pos {
            charlen = char_length(&buf[before..]);
            before += charlen;
        }

        return before - charlen;
    }

    if pos == 0 {
        0
    } else {
        pos - 1
    }
}

/// 返回 buf 中 pos 处字符之后的那个多字节字符的起始索引
/// （对应 `step_right`）。
pub fn step_right(buf: &[u8], pos: usize) -> usize {
    pos + char_length(&buf[pos.min(buf.len())..])
}

// ======================== 多字节字符串比较与查找 ========================

/// 替代 `strncasecmp()`（ASCII 不区分大小写，n 字节）。
fn strncasecmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    for k in 0..n {
        let ca = byte_at(a, k);
        let cb = byte_at(b, k);
        let la = (ca as char).to_ascii_lowercase() as u8;
        let lb = (cb as char).to_ascii_lowercase() as u8;
        if la != lb || ca == 0 {
            return la as i32 - lb as i32;
        }
    }
    0
}

/// 替代 `strncmp()`（n 字节）。
fn strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    for k in 0..n {
        let ca = byte_at(a, k);
        let cb = byte_at(b, k);
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
    }
    0
}

/// 等价于多字节字符串的 `strcasecmp()`（对应 `mbstrcasecmp`）。
pub fn mbstrcasecmp(s1: &[u8], s2: &[u8]) -> i32 {
    mbstrncasecmp(s1, s2, HIGHEST_POSITIVE)
}

/// 等价于多字节字符串的 `strncasecmp()`（对应 `mbstrncasecmp`）。
pub fn mbstrncasecmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    if using_utf8() {
        let mut i = 0;
        let mut j = 0;
        let mut remaining = n;

        loop {
            let c1 = byte_at(s1, i);
            let c2 = byte_at(s2, j);

            if c1 == 0 || c2 == 0 || remaining == 0 {
                break;
            }

            if (c1 as i8) >= 0 && (c2 as i8) >= 0 {
                if (b'A'..=b'Z').contains(&(c1 & 0x5F)) {
                    if (b'A'..=b'Z').contains(&(c2 & 0x5F)) {
                        if (c1 & 0x5F) != (c2 & 0x5F) {
                            return (c1 & 0x5F) as i32 - (c2 & 0x5F) as i32;
                        }
                    } else {
                        return (c1 | 0x20) as i32 - c2 as i32;
                    }
                } else if (b'A'..=b'Z').contains(&(c2 & 0x5F)) {
                    return c1 as i32 - (c2 | 0x20) as i32;
                } else if c1 != c2 {
                    return c1 as i32 - c2 as i32;
                }

                i += 1;
                j += 1;
                remaining -= 1;
                continue;
            }

            let wc1 = mbtowide(&s1[i..]);
            let wc2 = mbtowide(&s2[j..]);
            let bad1 = wc1.is_err();
            let bad2 = wc2.is_err();

            if bad1 || bad2 {
                if c1 != c2 {
                    return c1 as i32 - c2 as i32;
                }
                if bad1 != bad2 {
                    return if bad1 { 1 } else { -1 };
                }
            } else {
                let difference = towlower(wc1.unwrap().0) as i32 - towlower(wc2.unwrap().0) as i32;
                if difference != 0 {
                    return difference;
                }
            }

            i += char_length(&s1[i..]);
            j += char_length(&s2[j..]);
            remaining -= 1;
        }

        if remaining > 0 {
            byte_at(s1, i) as i32 - byte_at(s2, j) as i32
        } else {
            0
        }
    } else {
        strncasecmp(s1, s2, n)
    }
}

/// 等价于多字节字符串的 `strcasestr()`（对应 `mbstrcasestr`）。
pub fn mbstrcasestr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if using_utf8() {
        let needle_len = mbstrlen(needle);
        let mut pos = 0;
        while byte_at(haystack, pos) != 0 {
            if mbstrncasecmp(&haystack[pos..], needle, needle_len) == 0 {
                return Some(pos);
            }
            pos += char_length(&haystack[pos..]);
        }
        None
    } else {
        /* strcasestr：ASCII 不区分大小写。 */
        let needle_len = needle.len();
        let mut pos = 0;
        while byte_at(haystack, pos) != 0 {
            if strncasecmp(&haystack[pos..], needle, needle_len) == 0 {
                return Some(pos);
            }
            pos += 1;
        }
        None
    }
}

/// 等价于 `strstr()`，但从 pointer 处反向扫描（对应 `revstrstr`）。
pub fn revstrstr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    let needle_len = needle.len();
    let pp = pointer.min(haystack.len());
    let tail_len = haystack[pp..].iter().take_while(|&&b| b != 0).count();
    let mut p: isize = if tail_len < needle_len {
        pointer as isize - (needle_len - tail_len) as isize
    } else {
        pointer as isize
    };

    while p >= 0 {
        if strncmp(&haystack[p as usize..], needle, needle_len) == 0 {
            return Some(p as usize);
        }
        p -= 1;
    }

    None
}

/// 等价于 `strcasestr()`，但从 pointer 处反向扫描（对应 `revstrcasestr`）。
pub fn revstrcasestr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    let needle_len = needle.len();
    let pp = pointer.min(haystack.len());
    let tail_len = haystack[pp..].iter().take_while(|&&b| b != 0).count();
    let mut p: isize = if tail_len < needle_len {
        pointer as isize - (needle_len - tail_len) as isize
    } else {
        pointer as isize
    };

    while p >= 0 {
        if strncasecmp(&haystack[p as usize..], needle, needle_len) == 0 {
            return Some(p as usize);
        }
        p -= 1;
    }

    None
}

/// 等价于多字节字符串的 `strcasestr()`，但从 pointer 处反向扫描
/// （对应 `mbrevstrcasestr`）。
pub fn mbrevstrcasestr(haystack: &[u8], needle: &[u8], pointer: usize) -> Option<usize> {
    if using_utf8() {
        let needle_len = mbstrlen(needle);
        let tail_len = mbstrlen(&haystack[pointer.min(haystack.len())..]);
        let mut p: isize = if tail_len < needle_len {
            pointer as isize - (needle_len - tail_len) as isize
        } else {
            pointer as isize
        };

        if p < 0 {
            return None;
        }

        loop {
            if mbstrncasecmp(&haystack[p as usize..], needle, needle_len) == 0 {
                return Some(p as usize);
            }
            if p == 0 {
                return None;
            }
            p = step_left(haystack, p as usize) as isize;
        }
    } else {
        revstrcasestr(haystack, needle, pointer)
    }
}

/// 等价于多字节字符串的 `strchr()`（对应 `mbstrchr`）。
pub fn mbstrchr(string: &[u8], chr: &[u8]) -> Option<usize> {
    if using_utf8() {
        let mut bad_s = false;
        let (wc, bad_c) = match mbtowide(chr) {
            Ok((w, _)) => (w, false),
            Err(()) => (byte_at(chr, 0) as u32, true),
        };

        let mut pos: isize = 0;
        /* 安全化：C 源码在非法 UTF-8 字节处指针会后退甚至死循环，
         * 这里用已访问集合检测循环并退出，正常路径行为一致。 */
        let mut visited = HashSet::new();

        while byte_at(string, pos as usize) != 0 {
            if !visited.insert(pos as usize) {
                break;
            }
            let c = byte_at(string, pos as usize);
            let (ws, symlen) = match mbtowide(&string[pos as usize..]) {
                Ok((w, len)) => (w, len as isize),
                Err(()) => {
                    bad_s = true;
                    (c as u32, -1)
                }
            };

            if ws == wc && bad_s == bad_c {
                return Some(pos as usize);
            }

            pos += symlen;
            if pos < 0 {
                return None;
            }
        }

        None
    } else {
        /* strchr(string, *chr)。 */
        let target = byte_at(chr, 0);
        string.iter().position(|&b| b == target)
    }
}

/// 在给定字符串中，向前查找 accept 中任一字符的首次出现
/// （对应 `mbstrpbrk`）。
pub fn mbstrpbrk(string: &[u8], accept: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while byte_at(string, pos) != 0 {
        if mbstrchr(accept, &string[pos..]).is_some() {
            return Some(pos);
        }
        pos += char_length(&string[pos..]);
    }
    None
}

/// 在从 head 开始的字符串中，从 pointer 处反向查找 accept 中任一字符
/// （对应 `mbrevstrpbrk`）。
pub fn mbrevstrpbrk(head: &[u8], accept: &[u8], pointer: usize) -> Option<usize> {
    let mut p = pointer;
    if byte_at(head, p) == 0 {
        if p == 0 {
            return None;
        }
        p = step_left(head, p);
    }

    loop {
        if mbstrchr(accept, &head[p..]).is_some() {
            return Some(p);
        }
        /* 到达字符串头部仍未找到。 */
        if p == 0 {
            return None;
        }
        p = step_left(head, p);
    }
}

// ======================== 空白判断 ========================

/// 返回 TRUE 如果给定字符串包含至少一个空白字符
/// （对应 `has_blank_char`）。
pub fn has_blank_char(string: &[u8]) -> bool {
    let mut pos = 0;
    while byte_at(string, pos) != 0 && !is_blank_char(&string[pos..]) {
        pos += char_length(&string[pos..]);
    }
    byte_at(string, pos) != 0
}

/// 返回 TRUE 当给定字符串为空或只含空白（对应 `white_string`）。
pub fn white_string(string: &[u8]) -> bool {
    let mut pos = 0;
    while byte_at(string, pos) != 0
        && (is_blank_char(&string[pos..]) || byte_at(string, pos) == b'\r')
    {
        pos += char_length(&string[pos..]);
    }
    byte_at(string, pos) == 0
}

/// 移除给定字符串开头的空白（对应 `strip_leading_blanks_from`）。
pub fn strip_leading_blanks_from(string: &mut Vec<u8>) {
    while !string.is_empty() && (string[0] == b' ' || string[0] == b'\t') {
        string.remove(0);
    }
}

// ======================== 兼容 API（供其他模块使用） ========================

/// 当前字节位置处字符的字节数（兼容旧 API；对应 C 的 `char_length`）。
pub fn mb_cur_max(data: &[u8], pos: usize) -> usize {
    if data.is_empty() || pos >= data.len() {
        return 0;
    }
    char_length(&data[pos..])
}

/// 当前字节位置处字符的显示宽度（列数；兼容旧 API）。
pub fn char_width(data: &[u8], pos: usize) -> usize {
    if data.is_empty() || pos >= data.len() {
        return 0;
    }
    if data[pos] == b'\t' {
        return tabsize();
    }
    let mut column = 0;
    advance_over(&data[pos..], &mut column);
    column
}
