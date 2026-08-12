/**************************************************************************
 *   definitions.rs  --  这是 GNU nano 的 Rust 翻译版本的一部分。
 *
 *   版权 (C) 1999-2011, 2013-2026 Free Software Foundation, Inc.
 *   版权 (C) 2014-2017, 2020-2022, 2024 Benno Schulenberg
 *
 *   本程序是自由软件：你可以根据 GNU 通用公共许可证（第 3 版或更新版本）
 *   重新分发和/或修改它。
 **************************************************************************/

//! 此模块对应原版 nano 的 `definitions.h`，包含所有的结构体、枚举、
//! 常量定义以及全局类型。其余模块均依赖本模块。
//!
//! 链表字段一律使用裸指针 `*mut`，以忠实对应原版 C 代码中的指针语义。

use regex::Regex;

/// 根用户的 UID。
pub const ROOT_UID: u32 = 0;

/// 路径名的最大长度（若系统中未定义则取默认值）。
pub const PATH_MAX: usize = 4096;

/* 用于标记字符串的函数宏（在 Rust 中简化为恒等函数）。 */
/// 标记稍后会被 gettext() 调用的字符串（此处不做任何转换）。
#[allow(non_snake_case)]
pub fn N_(string: &'static str) -> &'static str {
    string
}

/* 翻译宏：此处不做任何转换，直接返回原字符串。 */
#[macro_export]
macro_rules! gettext {
    ($s:expr) => {
        $s
    };
}

/* 标志位的宏：在一个小数组中按位索引。 */
/// 标志数组每个元素的类型。
pub type flagword = u32;

/// 在标志数组中取出对应 `flag` 的数组元素。
#[inline]
pub fn FLAGS(flags_arr: &[flagword], flag: usize) -> flagword {
    flags_arr[flag / (std::mem::size_of::<flagword>() * 8)]
}
/// 计算 `flag` 对应的位掩码。
#[inline]
pub fn FLAGMASK(flag: usize) -> flagword {
    (1u32) << (flag % (std::mem::size_of::<flagword>() * 8))
}

/// 搜索方向：向后 / 向前。
pub const BACKWARD: bool = false;
pub const FORWARD: bool = true;

/// 是否带有 "all" 选项的确认对话框。
pub const YESORNO: bool = false;
pub const YESORALLORNO: bool = true;

/// 确认对话框的返回值。
pub const YES: i32 = 1;
pub const ALL: i32 = 2;
pub const NO: i32 = 0;
pub const CANCEL: i32 = -1;

/// 是否可见。
pub const BLIND: bool = false;
pub const VISIBLE: bool = true;

/// 搜索/替换模式。
pub const JUSTFIND: i32 = 0;
pub const REPLACING: i32 = 1;
pub const INREGION: i32 = 2;

/// 是否显示提示说明。
pub const NONOTES: bool = false;

/// 历史记录去重选项。
pub const PRUNE_DUPLICATE: bool = true;
pub const IGNORE_DUPLICATES: bool = false;

/* 在 UTF-8 下一个合法字符最多占用四个字节。 */
pub const MAXCHARLEN: usize = 4;

/// 制表符默认的空格宽度。
pub const WIDTH_OF_TAB: usize = 8;

/// 从行尾起多少列开始换行。
pub const COLUMNS_FROM_EOL: usize = 8;

/// 光标应远离边缘的列数。
pub const CUSHION: usize = 3;

/// 当某个语法未指定注释字符时使用的默认注释字符。
pub const GENERAL_COMMENT_CHARACTER: &str = "#";

/// 保存的搜索/替换历史字符串的最大数量。
pub const MAX_SEARCH_HISTORY: usize = 100;

/// 没有最高位被置位的最大 size_t 数值。
pub const HIGHEST_POSITIVE: usize = usize::MAX >> 1;

/* 启用颜色时的特殊值。 */
pub const THE_DEFAULT: i32 = -1;
pub const BAD_COLOR: i32 = -2;

/* 多行正则对某一行作用方式的标志。 */
pub const NOTHING: i32 = 1 << 1;
pub const STARTSHERE: i32 = 1 << 2;
pub const WHOLELINE: i32 = 1 << 3;
pub const ENDSHERE: i32 = 1 << 4;
pub const JUSTONTHIS: i32 = 1 << 5;

/* 基本控制码。 */
pub const ESC_CODE: u8 = 0x1B;
pub const DEL_CODE: u8 = 0x7F;

/* 超出 ncurses KEY_MAX 的"修饰"方向键代码。 */
pub const CONTROL_LEFT: i32 = 0x401;
pub const CONTROL_RIGHT: i32 = 0x402;
pub const CONTROL_UP: i32 = 0x403;
pub const CONTROL_DOWN: i32 = 0x404;
pub const CONTROL_HOME: i32 = 0x405;
pub const CONTROL_END: i32 = 0x406;
pub const CONTROL_DELETE: i32 = 0x40D;
pub const SHIFT_CONTROL_LEFT: i32 = 0x411;
pub const SHIFT_CONTROL_RIGHT: i32 = 0x412;
pub const SHIFT_CONTROL_UP: i32 = 0x413;
pub const SHIFT_CONTROL_DOWN: i32 = 0x414;
pub const SHIFT_CONTROL_HOME: i32 = 0x415;
pub const SHIFT_CONTROL_END: i32 = 0x416;
pub const CONTROL_SHIFT_DELETE: i32 = 0x41D;
pub const ALT_LEFT: i32 = 0x421;
pub const ALT_RIGHT: i32 = 0x422;
pub const ALT_UP: i32 = 0x423;
pub const ALT_DOWN: i32 = 0x424;
pub const ALT_HOME: i32 = 0x425;
pub const ALT_END: i32 = 0x426;
pub const ALT_PAGEUP: i32 = 0x427;
pub const ALT_PAGEDOWN: i32 = 0x428;
pub const ALT_INSERT: i32 = 0x42C;
pub const ALT_DELETE: i32 = 0x42D;
pub const SHIFT_ALT_LEFT: i32 = 0x431;
pub const SHIFT_ALT_RIGHT: i32 = 0x432;
pub const SHIFT_ALT_UP: i32 = 0x433;
pub const SHIFT_ALT_DOWN: i32 = 0x434;
pub const SHIFT_UP: i32 = 0x453;
pub const SHIFT_DOWN: i32 = 0x454;
pub const SHIFT_HOME: i32 = 0x455;
pub const SHIFT_END: i32 = 0x456;
pub const SHIFT_PAGEUP: i32 = 0x457;
pub const SHIFT_PAGEDOWN: i32 = 0x458;
pub const SHIFT_DELETE: i32 = 0x45D;
pub const SHIFT_TAB: i32 = 0x45F;

pub const FOCUS_IN: i32 = 0x491;
pub const FOCUS_OUT: i32 = 0x499;

/* 用于表示括号粘贴开始与结束的自定义键码。 */
pub const START_OF_PASTE: i32 = 0x4B5;
pub const END_OF_PASTE: i32 = 0x4BE;

/* 字符串绑定被部分植入、或存在不成对的左花括号、或字符串绑定中的函数
 * 需要执行、或指定的函数名无效时的特殊键码。 */
pub const MORE_PLANTS: i32 = 0x4EA;
pub const MISSING_BRACE: i32 = 0x4EB;
pub const PLANTED_A_COMMAND: i32 = 0x4EC;
pub const NO_SUCH_FUNCTION: i32 = 0x4EF;

/* Ctrl + 小键盘中央键的特殊键码。 */
pub const KEY_CENTER: i32 = 0x4F0;

/* 收到 SIGWINCH（窗口大小改变）时的特殊键码。 */
pub const THE_WINDOW_RESIZED: i32 = 0x4F7;

/* 某个按键产生未知转义序列时的特殊键码。 */
pub const FOREIGN_SEQUENCE: i32 = 0x4FC;

/* 挂起后用于插入输入流的特殊键码。 */
pub const KEY_FRESH: i32 = 0x4FE;

/* 来自 ncurses 的键码常量（用于快捷键绑定）。 */
pub const KEY_ENTER: i32 = 0x157;
pub const KEY_MOUSE: i32 = 0x4BB;
pub const KEY_BACKSPACE: i32 = 0x107;
pub const KEY_DC: i32 = 0x14B;
pub const KEY_IC: i32 = 0x14D;
pub const KEY_HOME: i32 = 0x106;
pub const KEY_END: i32 = 0x168;
pub const KEY_PPAGE: i32 = 0x149;
pub const KEY_NPAGE: i32 = 0x152;
pub const KEY_LEFT: i32 = 0x104;
pub const KEY_RIGHT: i32 = 0x105;
pub const KEY_UP: i32 = 0x103;
pub const KEY_DOWN: i32 = 0x102;
pub const KEY_F0: i32 = 0x101;
pub const KEY_CANCEL: i32 = 0x158;
pub const KEY_SIC: i32 = 0x14C;

/* 软换行/数字键盘/修改键相关的 ncurses 键码（按其数值顺序占位）。 */
pub const KEY_BTAB: i32 = 0x161;
pub const KEY_BEG: i32 = 0x171;
pub const KEY_SBEG: i32 = 0x172;
pub const KEY_B2: i32 = 0x179;
pub const KEY_A1: i32 = 0x176;
pub const KEY_A3: i32 = 0x178;
pub const KEY_C1: i32 = 0x17B;
pub const KEY_C3: i32 = 0x17D;
pub const KEY_SDC: i32 = 0x163;
pub const KEY_SHOME: i32 = 0x166;
pub const KEY_SEND: i32 = 0x168 + 1;
pub const KEY_SLEFT: i32 = 0x184;
pub const KEY_SRIGHT: i32 = 0x182;
pub const KEY_SR: i32 = 0x18A;
pub const KEY_SF: i32 = 0x189;
pub const KEY_SUP: i32 = 0x18B;
pub const KEY_SDOWN: i32 = 0x188;
pub const KEY_EOL: i32 = 0x187;
pub const KEY_SPREVIOUS: i32 = 0x185;
pub const KEY_SNEXT: i32 = 0x186;
pub const KEY_SCANCEL: i32 = 0x173;
pub const KEY_SSUSPEND: i32 = 0x175;
pub const KEY_SUSPEND: i32 = 0x159;
pub const KEY_RESIZE: i32 = 0x204;
pub const KEY_A2: i32 = 0x177;
pub const KEY_C2: i32 = 0x17C;
pub const KEY_SIC2: i32 = 0x164;
pub const KEY_SHELLO: i32 = 0x174;

/* 撤销功能的一些额外标志。 */
pub const WAS_BACKSPACE_AT_EOF: i32 = 1 << 1;
pub const WAS_WHOLE_LINE: i32 = 1 << 2;
pub const INCLUDED_LAST_LINE: i32 = 1 << 3;
pub const MARK_WAS_SET: i32 = 1 << 4;
pub const CURSOR_WAS_AT_HEAD: i32 = 1 << 5;
pub const HAD_ANCHOR_AT_START: i32 = 1 << 6;

/* 不同菜单的标识符。 */
pub const MMAIN: i32 = 1 << 0;
pub const MWHEREIS: i32 = 1 << 1;
pub const MREPLACE: i32 = 1 << 2;
pub const MREPLACEWITH: i32 = 1 << 3;
pub const MGOTOLINE: i32 = 1 << 4;
pub const MWRITEFILE: i32 = 1 << 5;
pub const MINSERTFILE: i32 = 1 << 6;
pub const MEXECUTE: i32 = 1 << 7;
pub const MHELP: i32 = 1 << 8;
pub const MSPELL: i32 = 1 << 9;
pub const MBROWSER: i32 = 1 << 10;
pub const MWHEREISFILE: i32 = 1 << 11;
pub const MGOTODIR: i32 = 1 << 12;
pub const MYESNO: i32 = 1 << 13;
pub const MLINTER: i32 = 1 << 14;
pub const MFINDINHELP: i32 = 1 << 15;
/* 除 Help、Browser 和 YesNo 之外所有菜单的缩写。 */
pub const MMOST: i32 = MMAIN | MWHEREIS | MREPLACE | MREPLACEWITH | MGOTOLINE | MWRITEFILE
    | MINSERTFILE | MEXECUTE | MWHEREISFILE | MGOTODIR | MFINDINHELP | MSPELL | MLINTER;
pub const MSOME: i32 = MMOST | MBROWSER;

/* 枚举类型。 */

/// 文件格式类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum format_type {
    UNSPECIFIED,
    NIX_FILE,
    DOS_FILE,
}

/// 消息类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum message_type {
    VACUUM,
    HUSH,
    REMARK,
    INFO,
    NOTICE,
    AHEM,
    MILD,
    ALERT,
}

/// 写入文件的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum writing_type {
    OVERWRITE,
    APPEND,
    PREPEND,
    SPECIAL,
}

/// 更新类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum update_type {
    CENTERING,
    FLOWING,
    STATIONARY,
}

/// 撤销操作的类型。ADD...REPLACE 必须排在最前面。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum undo_type {
    ADD,
    ENTER,
    BACK,
    DEL,
    JOIN,
    REPLACE,
    SPLIT_BEGIN,
    SPLIT_END,
    INDENT,
    UNINDENT,
    COMMENT,
    UNCOMMENT,
    PREFLIGHT,
    ZAP,
    CUT,
    CUT_TO_EOF,
    COPY,
    PASTE,
    INSERT,
    COUPLE_BEGIN,
    COUPLE_END,
    OTHER,
}

/* 界面中可以被不同着色的元素。 */
pub const TITLE_BAR: usize = 0;
pub const LINE_NUMBER: usize = 1;
pub const GUIDE_STRIPE: usize = 2;
pub const SCROLL_BAR: usize = 3;
pub const SELECTED_TEXT: usize = 4;
pub const SPOTLIGHTED: usize = 5;
pub const MINI_INFOBAR: usize = 6;
pub const PROMPT_BAR: usize = 7;
pub const STATUS_BAR: usize = 8;
pub const ERROR_MESSAGE: usize = 9;
pub const KEY_COMBO: usize = 10;
pub const FUNCTION_TAG: usize = 11;
pub const NUMBER_OF_ELEMENTS: usize = 12;

/* 标志数组中所用的枚举。参见 FLAGMASK 的定义。 */
pub const DONTUSE: usize = 0;
pub const CASE_SENSITIVE: usize = 1;
pub const CONSTANT_SHOW: usize = 2;
pub const NO_HELP: usize = 3;
pub const NO_WRAP: usize = 4;
pub const AUTOINDENT: usize = 5;
pub const VIEW_MODE: usize = 6;
pub const USE_MOUSE: usize = 7;
pub const USE_REGEXP: usize = 8;
pub const SAVE_ON_EXIT: usize = 9;
pub const CUT_FROM_CURSOR: usize = 10;
pub const BACKWARDS_SEARCH: usize = 11;
pub const NEW_BUFFER: usize = 12;
pub const REBIND_DELETE: usize = 13;
pub const RAW_SEQUENCES: usize = 14;
pub const NO_CONVERT: usize = 15;
pub const MAKE_BACKUP: usize = 16;
pub const INSECURE_BACKUP: usize = 17;
pub const NO_SYNTAX: usize = 18;
pub const PRESERVE: usize = 19;
pub const HISTORYLOG: usize = 20;
pub const RESTRICTED: usize = 21;
pub const SMART_HOME: usize = 22;
pub const WHITESPACE_DISPLAY: usize = 23;
pub const TABS_TO_SPACES: usize = 24;
pub const QUICK_BLANK: usize = 25;
pub const WORD_BOUNDS: usize = 26;
pub const NO_NEWLINES: usize = 27;
pub const BOLD_TEXT: usize = 28;
pub const SOFTWRAP: usize = 29;
pub const POSITIONLOG: usize = 30;
pub const LOCKING: usize = 31;
pub const NOREAD_MODE: usize = 32;
pub const MAKE_IT_UNIX: usize = 33;
pub const TRIM_BLANKS: usize = 34;
pub const SHOW_CURSOR: usize = 35;
pub const LINE_NUMBERS: usize = 36;
pub const AT_BLANKS: usize = 37;
pub const AFTER_ENDS: usize = 38;
pub const LET_THEM_ZAP: usize = 39;
pub const BREAK_LONG_LINES: usize = 40;
pub const JUMPY_SCROLLING: usize = 41;
pub const EMPTY_LINE: usize = 42;
pub const INDICATOR: usize = 43;
pub const BOOKSTYLE: usize = 44;
pub const COLON_PARSING: usize = 45;
pub const STATEFLAGS: usize = 46;
pub const USE_MAGIC: usize = 47;
pub const MINIBAR: usize = 48;
pub const ZERO: usize = 49;
pub const MODERN_BINDINGS: usize = 50;
pub const SOLO_SIDESCROLL: usize = 51;

/* 结构体类型。 */

/// 颜色组合。
pub struct colortype {
    pub id: i16,
    pub fg: i16,
    pub bg: i16,
    pub pairnum: i16,
    pub attributes: i32,
    pub start: Option<Box<Regex>>,
    pub end: Option<Box<Regex>>,
    pub next: *mut colortype,
}

/// 正则列表节点。
pub struct regexlisttype {
    pub one_rgx: Option<Box<Regex>>,
    pub next: *mut regexlisttype,
}

/// extendsyntax 命令的增强记录。
pub struct augmentstruct {
    pub filename: Option<String>,
    pub lineno: isize,
    pub data: Option<String>,
    pub next: *mut augmentstruct,
}

/// 语法类型。
pub struct syntaxtype {
    pub name: Option<String>,
    pub filename: Option<String>,
    pub lineno: usize,
    pub augmentations: *mut augmentstruct,
    pub extensions: *mut regexlisttype,
    pub headers: *mut regexlisttype,
    pub magics: *mut regexlisttype,
    pub linter: Option<String>,
    pub formatter: Option<String>,
    pub tabstring: Option<String>,
    pub comment: Option<String>,
    pub color: *mut colortype,
    pub multiscore: i16,
    pub next: *mut syntaxtype,
}

/// lint 错误信息。
pub struct lintstruct {
    pub lineno: isize,
    pub colno: isize,
    pub msg: Option<String>,
    pub filename: Option<String>,
    pub next: *mut lintstruct,
    pub prev: *mut lintstruct,
}

/// 行结构。
pub struct linestruct {
    pub data: String,
    pub lineno: isize,
    pub next: *mut linestruct,
    pub prev: *mut linestruct,
    pub multidata: Option<Vec<i16>>,
    pub has_anchor: bool,
}

/// 行组结构（用于缩进/反缩进等成组操作）。
pub struct groupstruct {
    pub top_line: isize,
    pub bottom_line: isize,
    pub indentations: Vec<Option<String>>,
    pub next: *mut groupstruct,
}

/// 撤销结构。
pub struct undostruct {
    pub type_: undo_type,
    pub xflags: i32,
    pub head_lineno: isize,
    pub head_x: usize,
    pub strdata: Option<String>,
    pub wassize: usize,
    pub newsize: usize,
    pub grouping: *mut groupstruct,
    pub cutbuffer: *mut linestruct,
    pub tail_lineno: isize,
    pub tail_x: usize,
    pub next: *mut undostruct,
}

/// 位置记录（用于保存/恢复光标位置）。
pub struct positionstruct {
    pub filename: Option<String>,
    pub linenumber: isize,
    pub columnnumber: isize,
    pub anchors: Option<String>,
    pub next: *mut positionstruct,
}

/// 已打开文件的结构。
pub struct openfilestruct {
    pub filename: Option<String>,
    pub filetop: *mut linestruct,
    pub filebot: *mut linestruct,
    pub edittop: *mut linestruct,
    pub current: *mut linestruct,
    pub totsize: usize,
    pub firstcolumn: usize,
    pub current_x: usize,
    pub placewewant: usize,
    pub brink: usize,
    pub cursor_row: isize,
    pub statinfo: Option<Box<std::fs::Metadata>>,
    pub spillage_line: *mut linestruct,
    pub mark: *mut linestruct,
    pub mark_x: usize,
    pub softmark: bool,
    pub fmt: format_type,
    pub lock_filename: Option<String>,
    pub undotop: *mut undostruct,
    pub current_undo: *mut undostruct,
    pub last_saved: *mut undostruct,
    pub last_action: undo_type,
    pub modified: bool,
    pub syntax: *mut syntaxtype,
    pub errormessage: Option<String>,
    pub next: *mut openfilestruct,
    pub prev: *mut openfilestruct,
}

/// rcfile 选项。
pub struct rcoption {
    pub name: &'static str,
    pub flag: i64,
}

/// 键结构。
pub struct keystruct {
    pub keystr: &'static str,
    pub keycode: i32,
    pub menus: i32,
    pub func: Option<unsafe fn()>,
    pub toggle: i32,
    pub ordinal: i32,
    pub expansion: Option<String>,
    pub next: *mut keystruct,
}

/// 函数结构。
pub struct funcstruct {
    pub func: Option<unsafe fn()>,
    pub tag: &'static str,
    pub phrase: &'static str,
    pub blank_after: bool,
    pub menus: i32,
    pub next: *mut funcstruct,
}

/// 单词补全结构。
pub struct completionstruct {
    pub word: Option<String>,
    pub next: *mut completionstruct,
}

/* ===== 共享全局状态与基础辅助（供各翻译模块复用） ===== */

/// 用户主目录。
pub static mut homedir: Option<String> = None;

/// 标志数组（按位索引），长度足以容纳所有标志枚举。
pub static mut flags: [flagword; 2] = [0; 2];

/// 已打开文件链表的当前文件指针。
pub static mut openfile: *mut openfilestruct = std::ptr::null_mut();

/// 搜索用的正则表达式（已编译）。
pub static mut search_regexp: Option<Box<Regex>> = None;
pub static mut regexp_nsub: usize = 0;

/// 正则搜索匹配结果（最多 10 组，rm_so/rm_eo 为 Option<usize>）。
pub static mut regmatches: [(Option<usize>, Option<usize>); 10] = [(None, None); 10];

/// 是否将区域末尾的整行也纳入处理。
pub static mut also_the_last: bool = false;

/// 编辑窗口的列数。
pub static mut editwincols: usize = 0;

/// 是否采用统一的侧向滚动行为。
pub static mut united_sidescroll: bool = false;

/// 取标志数组中 `flag` 是否被置位。
#[inline]
pub fn ISSET(flag: usize) -> bool {
    unsafe { (FLAGS(&flags, flag) & FLAGMASK(flag)) != 0 }
}

/// 置位某个标志。
#[inline]
pub fn SET(flag: usize) {
    unsafe {
        let idx = flag / (std::mem::size_of::<flagword>() * 8);
        flags[idx] |= FLAGMASK(flag);
    }
}

/// 清除某个标志。
#[inline]
pub fn UNSET(flag: usize) {
    unsafe {
        let idx = flag / (std::mem::size_of::<flagword>() * 8);
        flags[idx] &= !FLAGMASK(flag);
    }
}

/// 翻转某个标志。
#[inline]
pub fn TOGGLE(flag: usize) {
    unsafe {
        let idx = flag / (std::mem::size_of::<flagword>() * 8);
        flags[idx] ^= FLAGMASK(flag);
    }
}

/// 致命错误退出（内存耗尽等）。
pub fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

/// 复制一份字符串（对应 C 的 copy_of / measured_copy）。
pub fn copy_of(string: &str) -> String {
    string.to_string()
}

/// 分配并复制 src 的前 count 个字节（对应 measured_copy）。
pub fn measured_copy(string: &[u8], count: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(count + 1);
    v.extend_from_slice(&string[..count.min(string.len())]);
    v.push(0);
    v
}

/// 创建一个新行节点（链表接线由调用方完成）。
pub fn make_new_node(given: &linestruct) -> Box<linestruct> {
    Box::new(linestruct {
        data: String::new(),
        lineno: given.lineno + 1,
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
        multidata: None,
        has_anchor: false,
    })
}

/// 删除给定节点（从链表中摘除并释放，接线由调用方处理）。
pub fn delete_node(_node: Box<linestruct>) {
    /* 在 Rust 中，摘除逻辑由调用方处理链表指针，此处仅消费 Box 以释放。 */
}

/* 版本号与构建信息（对应 C 的 VERSION / REVISION）。 */
pub const VERSION: &str = "8.5";
pub const REVISION: &str = "";
