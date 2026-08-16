/**************************************************************************
 * definitions.rs  --  GNU nano Rust 翻译版核心定义
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 * 本程序是自由软件：可根据 GPLv3+ 重新分发/修改。
 **************************************************************************/

//! 对应原版 definitions.h：常量、结构体、枚举及安全全局状态。
//! 转换说明：
//! - 裸指针 → `Rc<RefCell<T>>`（安全引用计数）
//! - `static mut` → `LazyLock<RefCell<GlobalState>>`
//! - 正则表达式 → `MatchPattern`（简单通配符匹配）
//! - 函数指针 → `FunctionId` 枚举

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::LazyLock;

use crate::regex::Regex;

// ======================== 常量 ========================

pub const ROOT_UID: u32 = 0;
pub const PATH_MAX: usize = 4096;
pub const MAXCHARLEN: usize = 4;
pub const WIDTH_OF_TAB: usize = 8;
pub const COLUMNS_FROM_EOL: usize = 8;
pub const CUSHION: usize = 3;
pub const GENERAL_COMMENT_CHARACTER: &str = "#";
pub const MAX_SEARCH_HISTORY: usize = 100;
pub const HIGHEST_POSITIVE: usize = usize::MAX >> 1;
pub const THE_DEFAULT: i32 = -1;
pub const BAD_COLOR: i32 = -2;
pub const NOTHING: i32 = 1 << 1;
pub const STARTSHERE: i32 = 1 << 2;
pub const WHOLELINE: i32 = 1 << 3;
pub const ENDSHERE: i32 = 1 << 4;
pub const JUSTONTHIS: i32 = 1 << 5;
pub const ESC_CODE: u8 = 0x1B;
pub const DEL_CODE: u8 = 0x7F;
pub const BACKWARD: bool = false;
pub const FORWARD: bool = true;
pub const YESORNO: bool = false;
pub const YESORALLORNO: bool = true;
pub const YES: i32 = 1;
pub const ALL: i32 = 2;
pub const NO: i32 = 0;
pub const CANCEL: i32 = -1;
pub const BLIND: bool = false;
pub const VISIBLE: bool = true;
pub const JUSTFIND: i32 = 0;
pub const REPLACING: i32 = 1;
pub const INREGION: i32 = 2;
pub const NONOTES: bool = false;
pub const PRUNE_DUPLICATE: bool = true;
pub const IGNORE_DUPLICATES: bool = false;

// 修饰键码
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
pub const START_OF_PASTE: i32 = 0x4B5;
pub const END_OF_PASTE: i32 = 0x4BE;
pub const MORE_PLANTS: i32 = 0x4EA;
pub const MISSING_BRACE: i32 = 0x4EB;
pub const PLANTED_A_COMMAND: i32 = 0x4EC;
pub const NO_SUCH_FUNCTION: i32 = 0x4EF;
pub const KEY_CENTER: i32 = 0x4F0;
pub const THE_WINDOW_RESIZED: i32 = 0x4F7;
pub const FOREIGN_SEQUENCE: i32 = 0x4FC;
pub const KEY_FRESH: i32 = 0x4FE;

// ncurses 键码
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
pub const KEY_F0: i32 = 0x108;
pub const KEY_CANCEL: i32 = 0x158;
pub const KEY_SIC: i32 = 0x14C;
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
pub const KEY_SEND: i32 = 0x169;
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

// 撤销标志
pub const WAS_BACKSPACE_AT_EOF: i32 = 1 << 1;
pub const WAS_WHOLE_LINE: i32 = 1 << 2;
pub const INCLUDED_LAST_LINE: i32 = 1 << 3;
pub const MARK_WAS_SET: i32 = 1 << 4;
pub const CURSOR_WAS_AT_HEAD: i32 = 1 << 5;
pub const HAD_ANCHOR_AT_START: i32 = 1 << 6;

// 菜单标识符
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
pub const MMOST: i32 = MMAIN | MWHEREIS | MREPLACE | MREPLACEWITH | MGOTOLINE | MWRITEFILE
    | MINSERTFILE | MEXECUTE | MWHEREISFILE | MGOTODIR | MFINDINHELP | MSPELL | MLINTER;
pub const MSOME: i32 = MMOST | MBROWSER;

// 界面元素
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

// 标志位枚举
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

// ======================== 类型别名 ========================

pub type Flagword = u32;

// 安全引用类型
pub type ColorRef = Rc<RefCell<ColorType>>;
pub type RegexListRef = Rc<RefCell<RegexListType>>;
pub type AugmentRef = Rc<RefCell<AugmentStruct>>;
pub type SyntaxRef = Rc<RefCell<SyntaxType>>;
pub type LintRef = Rc<RefCell<LintStruct>>;
pub type LineRef = Rc<RefCell<LineStruct>>;
pub type GroupRef = Rc<RefCell<GroupStruct>>;
pub type UndoRef = Rc<RefCell<UndoStruct>>;
pub type PositionRef = Rc<RefCell<PositionStruct>>;
pub type OpenFileRef = Rc<RefCell<OpenFileStruct>>;
pub type KeyRef = Rc<RefCell<KeyStruct>>;
pub type FuncRef = Rc<RefCell<FuncStruct>>;
pub type CompletionRef = Rc<RefCell<CompletionStruct>>;
pub type LineWeak = Weak<RefCell<LineStruct>>;
pub type OpenFileWeak = Weak<RefCell<OpenFileStruct>>;

// ======================== 简单模式匹配（替代 regex） ========================

/// 简单的模式匹配，替代 POSIX regex。
/// 基于内部正则引擎的模式（替代 POSIX regex）。
/// - `from_literal`：字面匹配（特殊字符转义）
/// - `from_glob`：glob 匹配（`*`/`?`）
/// - `from_regex`：真正的 GNU 风格正则（syntax 高亮等）
#[derive(Debug, Clone)]
pub struct MatchPattern {
    pattern: String,
    re: Regex,
}

impl MatchPattern {
    /// 字面模式：把正则特殊字符全部转义。
    pub fn from_literal(pattern: &str) -> Self {
        let escaped = escape_regex(pattern);
        let re = Regex::compile(&escaped, false).expect("escaped literal must compile");
        MatchPattern { pattern: pattern.to_string(), re }
    }
    /// glob 模式：`*` → `.*`，`?` → `.`，其余字符转义。
    pub fn from_glob(pattern: &str) -> Self {
        let mut out = String::new();
        for c in pattern.chars() {
            match c {
                '*' => out.push_str(".*"),
                '?' => out.push('.'),
                _ => push_escaped(&mut out, c),
            }
        }
        let re = Regex::compile(&out, false).expect("translated glob must compile");
        MatchPattern { pattern: pattern.to_string(), re }
    }
    /// 正则模式：按 GNU 风格 ERE 编译；icase 对应 REG_ICASE。
    pub fn from_regex(pattern: &str, icase: bool) -> Result<Self, String> {
        let re = Regex::compile(pattern, icase)?;
        Ok(MatchPattern { pattern: pattern.to_string(), re })
    }
    /// 判断 text 中是否存在匹配（对应 regexec 布尔用途）。
    pub fn matches(&self, text: &str) -> bool {
        self.re.is_match(text)
    }
    /// 在 text 中查找第一个匹配，返回 (起点, 终点)。
    pub fn find_match(&self, text: &str) -> Option<(usize, usize)> {
        self.re.find(text, 0, false)
    }
    /// 字节串版本：在 `text` 中查找模式的第一个匹配，返回 (起点, 终点) 字节偏移。
    /// （nano 的文本是字节级存储，可能含非法 UTF-8，故提供该版本。）
    pub fn find_match_bytes(&self, text: &[u8]) -> Option<(usize, usize)> {
        self.re.find_bytes(text, 0, false)
    }
    /// 从指定偏移开始查找（对应 regexec(text + start)），notbol 对应 REG_NOTBOL。
    pub fn find_from(&self, text: &[u8], start: usize, notbol: bool) -> Option<(usize, usize)> {
        self.re.find_bytes(text, start, notbol)
    }
    /// 原始模式字符串。
    pub fn pattern_str(&self) -> &str {
        &self.pattern
    }
}

/// 转义单个字符为字面量（正则引擎的转义规则）。
fn push_escaped(out: &mut String, c: char) {
    match c {
        '.' | '^' | '$' | '*' | '+' | '?' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\' => {
            out.push('\\');
            out.push(c);
        }
        _ => out.push(c),
    }
}

/// 把整串转义为正则字面量。
fn escape_regex(pattern: &str) -> String {
    let mut out = String::new();
    for c in pattern.chars() {
        push_escaped(&mut out, c);
    }
    out
}

// ======================== 枚举类型 ========================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType { Unspecified, NixFile, DosFile }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType { Vacuum, Hush, Remark, Info, Notice, Ahem, Mild, Alert }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritingType { Overwrite, Append, Prepend, Special }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType { Centering, Flowing, Stationary }

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoType {
    Add, Enter, Back, Del, Join, Replace, SplitBegin, SplitEnd,
    Indent, Unindent, Comment, Uncomment, Preflight,
    Zap, Cut, CutToEof, Copy, Paste, Insert, CoupleBegin, CoupleEnd, Other,
}

/// 函数 ID 枚举，替代 unsafe 函数指针。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionId {
    None, DoCancel, DoExit, DoHelp, DoLeft, DoRight, DoUp, DoDown,
    DoHome, DoEnd, DoPageUp, DoPageDown, DoDelete, DoBackspace,
    DoEnter, DoTab, DoCut, DoCopy, DoPaste, DoCutToEof,
    DoSearchForward, DoSearchBackward, DoFindNext, DoFindPrevious,
    DoReplace, DoGoToLine, DoWriteOut, DoInsertFile, DoExecute,
    DoSpell, DoLinter, DoFormatter, DoIndent, DoUnindent,
    DoComment, DoUncomment, DoUndo, DoRedo, DoRefresh,
    DoSuspend, DoToggle, DoToggleCaseSensitive, DoToggleRegexp,
    DoToggleBackwards, DoToggleNoHelp, DoToggleConstantShow,
    DoToggleAutoIndent, DoToggleCutFromCursor, DoToggleSoftWrap,
    DoToggleLineNumbers, DoToggleWhiteSpace, DoToggleTabsToSpaces,
    DoToggleMouse, DoToggleViewMode, DoToggleNoWrap, DoToggleSmarthome,
    DoToggleBoldText, DoMakeItUnix, DoScrollUp, DoScrollDown,
    DoPrevBlock, DoNextBlock, DoParaBegin, DoParaEnd,
    DoFirstLine, DoLastLine, DoNextWord, DoPrevWord,
    DoMark, DoAnchor, DoGotoDir, DoBrowserUp, DoBrowserDown,
    DoBrowserEnter, DoWhereIsFile, DoFindInHelp, DoFullRefresh,
    DoImplantStub, DoRecordMacro, DoRunMacro, DoJustify,
    DoFindBracket, DoReportLocation, DoToggleModern,
    DoNothing, DoVerbatimInput, GetOlderItem, GetNewerItem,
    ToFirstFile, ToLastFile, Implant, FlipGoto,
    Other(u32),
}

// ======================== 结构体类型 ========================

#[derive(Debug, Clone)]
pub struct ColorType {
    pub id: i16, pub fg: i16, pub bg: i16, pub pairnum: i16, pub attributes: i32,
    pub start: Option<MatchPattern>, pub end: Option<MatchPattern>,
    pub next: Option<ColorRef>,
}

#[derive(Debug, Clone)]
pub struct RegexListType {
    pub one_rgx: Option<MatchPattern>, pub next: Option<RegexListRef>,
}

#[derive(Debug, Clone)]
pub struct AugmentStruct {
    pub filename: Option<String>, pub lineno: isize, pub data: Option<String>,
    pub next: Option<AugmentRef>,
}

#[derive(Debug, Clone)]
pub struct SyntaxType {
    pub name: Option<String>, pub filename: Option<String>, pub lineno: usize,
    pub augmentations: Option<AugmentRef>,
    pub extensions: Option<RegexListRef>, pub headers: Option<RegexListRef>,
    pub magics: Option<RegexListRef>,
    pub linter: Option<String>, pub formatter: Option<String>,
    pub tabstring: Option<String>, pub comment: Option<String>,
    pub color: Option<ColorRef>, pub multiscore: i16, pub next: Option<SyntaxRef>,
}

impl SyntaxType {
    /// 创建默认的空语法结构。
    pub fn new() -> Self {
        SyntaxType {
            name: None,
            filename: None,
            lineno: 0,
            augmentations: None,
            extensions: None,
            headers: None,
            magics: None,
            linter: None,
            formatter: None,
            tabstring: None,
            comment: None,
            color: None,
            multiscore: 0,
            next: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintStruct {
    pub lineno: isize, pub colno: isize,
    pub msg: Option<String>, pub filename: Option<String>,
    pub next: Option<LintRef>, pub prev: Option<LintRef>,
}

#[derive(Debug, Clone)]
pub struct LineStruct {
    pub data: String, pub lineno: isize,
    pub next: Option<LineRef>, pub prev: Option<LineWeak>,
    pub multidata: Option<Vec<i16>>, pub has_anchor: bool,
}

#[derive(Debug, Clone)]
pub struct GroupStruct {
    pub top_line: isize, pub bottom_line: isize,
    pub indentations: Vec<Option<String>>, pub next: Option<GroupRef>,
}

#[derive(Debug, Clone)]
pub struct UndoStruct {
    pub type_: UndoType, pub xflags: i32,
    pub head_lineno: isize, pub head_x: usize,
    pub strdata: Option<String>, pub wassize: usize, pub newsize: usize,
    pub grouping: Option<GroupRef>, pub cutbuffer: Option<LineRef>,
    pub tail_lineno: isize, pub tail_x: usize, pub next: Option<UndoRef>,
}

#[derive(Debug, Clone)]
pub struct PositionStruct {
    pub filename: Option<String>, pub linenumber: isize, pub columnnumber: isize,
    pub anchors: Option<String>, pub next: Option<PositionRef>,
}

#[derive(Debug, Clone)]
pub struct OpenFileStruct {
    pub filename: Option<String>,
    pub filetop: Option<LineRef>, pub filebot: Option<LineRef>,
    pub edittop: Option<LineRef>, pub current: Option<LineRef>,
    pub totsize: usize, pub firstcolumn: usize, pub current_x: usize,
    pub placewewant: usize, pub brink: usize, pub cursor_row: isize,
    pub statinfo: Option<Box<std::fs::Metadata>>,
    pub spillage_line: Option<LineRef>,
    pub mark: Option<LineRef>, pub mark_x: usize, pub softmark: bool,
    pub fmt: FormatType, pub lock_filename: Option<String>,
    pub undotop: Option<UndoRef>, pub current_undo: Option<UndoRef>,
    pub last_saved: Option<UndoRef>, pub last_action: UndoType,
    pub modified: bool, pub syntax: Option<SyntaxRef>,
    pub errormessage: Option<String>,
    pub next: Option<OpenFileRef>, pub prev: Option<OpenFileRef>,
}

impl OpenFileStruct {
    /// 创建默认的空缓冲区结构。
    pub fn new() -> Self {
        OpenFileStruct {
            filename: None,
            filetop: None,
            filebot: None,
            edittop: None,
            current: None,
            totsize: 1,
            firstcolumn: 0,
            current_x: 0,
            placewewant: 0,
            brink: 0,
            cursor_row: 0,
            statinfo: None,
            spillage_line: None,
            mark: None,
            mark_x: 0,
            softmark: false,
            fmt: FormatType::Unspecified,
            lock_filename: None,
            undotop: None,
            current_undo: None,
            last_saved: None,
            last_action: UndoType::Other,
            modified: false,
            syntax: None,
            errormessage: None,
            next: None,
            prev: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RcOption {
    pub name: &'static str, pub flag: i64,
}

#[derive(Debug, Clone)]
pub struct KeyStruct {
    pub keystr: String, pub keycode: i32, pub menus: i32,
    pub func: FunctionId, pub toggle: i32, pub ordinal: i32,
    pub expansion: Option<String>, pub next: Option<KeyRef>,
}

#[derive(Debug, Clone)]
pub struct FuncStruct {
    pub func: FunctionId, pub tag: &'static str, pub phrase: &'static str,
    pub blank_after: bool, pub menus: i32, pub next: Option<FuncRef>,
}

#[derive(Debug, Clone)]
pub struct CompletionStruct {
    pub word: Option<String>, pub next: Option<CompletionRef>,
}

// ======================== 安全全局状态 ========================

/// 全局标志位容器。
#[derive(Debug, Clone)]
pub struct GlobalFlags {
    flags: [Flagword; 4],
}

impl GlobalFlags {
    pub const fn new() -> Self { GlobalFlags { flags: [0; 4] } }
    pub fn isset(&self, flag: usize) -> bool {
        (FLAGS(&self.flags, flag) & FLAGMASK(flag)) != 0
    }
    pub fn set(&mut self, flag: usize) {
        let idx = flag / (std::mem::size_of::<Flagword>() * 8);
        self.flags[idx] |= FLAGMASK(flag);
    }
    pub fn unset(&mut self, flag: usize) {
        let idx = flag / (std::mem::size_of::<Flagword>() * 8);
        self.flags[idx] &= !FLAGMASK(flag);
    }
    pub fn toggle(&mut self, flag: usize) {
        let idx = flag / (std::mem::size_of::<Flagword>() * 8);
        self.flags[idx] ^= FLAGMASK(flag);
    }
}

#[inline]
pub fn FLAGS(flags_arr: &[Flagword], flag: usize) -> Flagword {
    flags_arr[flag / (std::mem::size_of::<Flagword>() * 8)]
}
#[inline]
pub fn FLAGMASK(flag: usize) -> Flagword {
    (1u32) << (flag % (std::mem::size_of::<Flagword>() * 8))
}

/// 便捷函数，使用全局状态。
pub fn ISSET(flag: usize) -> bool { is_flag_set(flag) }
pub fn SET(flag: usize) { set_flag(flag) }
pub fn UNSET(flag: usize) { unset_flag(flag) }
pub fn TOGGLE(flag: usize) { toggle_flag(flag) }

/// 全局状态结构体。
pub struct GlobalState {
    pub flags: GlobalFlags,
    pub openfile: Option<OpenFileRef>,
    pub homedir: Option<String>,
    pub editwincols: usize,
    pub united_sidescroll: bool,
    pub also_the_last: bool,
    pub search_regexp: Option<MatchPattern>,
    pub regexp_nsub: usize,
    pub came_full_circle: bool,
    pub regexp_compiled: bool,
    pub regmatches: Vec<(Option<usize>, Option<usize>)>,
    pub we_are_running: bool,
    pub more_than_one: bool,
    pub report_size: bool,
    pub ran_a_tool: bool,
    pub foretext: Option<String>,
    pub final_status: i32,
    pub inhelp: bool,
    pub title: Option<String>,
    pub refresh_needed: bool,
    pub focusing: bool,
    pub control_C_was_pressed: bool,
    pub lastmessage: MessageType,
    /// 状态栏当前显示的消息文本（供重绘时保留，对应 C 中 curses 窗口的内容）。
    pub statusbar_msg: String,
    pub pletion_line: Option<LineRef>,
    pub answer: Option<String>,
    pub last_search: Option<String>,
    pub didfind: i32,
    pub present_path: Option<String>,
    pub on_a_vt: bool,
    pub shifted_metas: bool,
    pub meta_key: bool,
    pub shift_held: bool,
    pub mute_modifiers: bool,
    pub using_utf8: bool,
    pub word_chars: Option<String>,
    pub as_an_at: bool,
    pub tabsize: usize,
    pub quotereg: Option<MatchPattern>,
    // 窗口/键盘变量
    pub currmenu: i32,
    pub topwin: bool, pub midwin: bool, pub footwin: bool,
    pub editwinrows: i32,
    pub margin: i32,
    pub matchbrackets: Option<String>,
    pub perturbed: bool,
    pub recook: bool,
    pub searchbot: Option<LineRef>,
    pub spotlighted: bool,
    pub light_from_col: usize,
    pub light_to_col: usize,
    pub search_history: Option<LineRef>,
    pub searchtop: Option<LineRef>,
    pub replace_history: Option<LineRef>,
    pub replacetop: Option<LineRef>,
    pub replacebot: Option<LineRef>,
    pub execute_history: Option<LineRef>,
    pub executetop: Option<LineRef>,
    pub executebot: Option<LineRef>,
    pub statedir: Option<String>,
    pub registername: Option<String>,
    pub latest_timestamp: i64,
    pub positions_register: Option<PositionRef>,
    pub history_changed: bool,
    pub startup_problem: Option<String>,
    pub sidebar: bool,
    pub cutbuffer: Option<LineRef>,
    pub cutbottom: Option<LineRef>,
    pub keep_cutbuffer: bool,
    pub inherited_anchor: bool,
    pub cycling_aim: i32,
    pub typing_x: usize,
    pub prompt: Option<String>,
    pub whitespace: Option<Vec<u8>>,
    pub whitelen: (usize, usize),
    pub color_combo: Vec<Option<ColorRef>>,
    pub rescind_colors: bool,
    pub have_palette: bool,
    pub hilite_attribute: i32,
    pub syntaxstr: Option<String>,
    pub help_text: Option<String>,
    pub help_start_of_body: usize,
    pub help_end_of_intro: usize,
    pub help_location: usize,
    pub filelist: Vec<String>,
    pub list_length: usize,
    pub usable_rows: usize,
    pub piles: i32,
    pub gauge: i32,
    pub selected: usize,
    pub resized_for_browser: bool,
    pub interface_color_pair: Vec<i32>,
    pub allfuncs: Option<FuncRef>,
    pub shortcuts: Option<KeyRef>,
    pub syntaxes: Option<SyntaxRef>,
    pub commandname: Option<String>,
    pub planted_shortcut: Option<KeyRef>,
    // 窗口尺寸
    pub COLS: usize,
    pub LINES: usize,
    pub fill: isize,
    pub wrap_at: usize,
}

impl GlobalState {
    pub fn new() -> Self {
        GlobalState {
            flags: GlobalFlags::new(), openfile: None, homedir: None,
            editwincols: 0, united_sidescroll: false, also_the_last: false,
            search_regexp: None, regexp_nsub: 0,
            came_full_circle: false, regexp_compiled: false,
            regmatches: vec![(None, None); 10],
            we_are_running: false, more_than_one: false, report_size: true,
            ran_a_tool: false, foretext: None, final_status: 0,
            inhelp: false, title: None, refresh_needed: false,
            focusing: true, control_C_was_pressed: false,
            lastmessage: MessageType::Vacuum, statusbar_msg: String::new(),
            pletion_line: None,
            answer: None, last_search: None, didfind: 0, present_path: None,
            on_a_vt: false, shifted_metas: false, meta_key: false,
            shift_held: false, mute_modifiers: false,
            using_utf8: true, word_chars: None, as_an_at: false, tabsize: 8,
            quotereg: None,
            currmenu: MMAIN, topwin: false, midwin: false, footwin: false,
            editwinrows: 0, margin: 0, matchbrackets: None,
            perturbed: false, recook: false, searchbot: None,
            spotlighted: false, light_from_col: 0, light_to_col: 0,
            search_history: None, searchtop: None,
            replace_history: None, replacetop: None, replacebot: None,
            execute_history: None, executetop: None, executebot: None,
            statedir: None, registername: None, latest_timestamp: 942927132,
            positions_register: None, history_changed: false,
            startup_problem: None,
            sidebar: false,
            cutbuffer: None, cutbottom: None,
            keep_cutbuffer: false, inherited_anchor: false, cycling_aim: 0,
            typing_x: 0,
            prompt: None,
            whitespace: Some(vec![0xC2, 0xBB, 0xC2, 0xB7]),
            whitelen: (2, 2),
            color_combo: vec![None; NUMBER_OF_ELEMENTS],
            rescind_colors: true,
            have_palette: false,
            hilite_attribute: 0,
            syntaxstr: None,
            help_text: None, help_start_of_body: 0, help_end_of_intro: 0,
            help_location: 0,
            filelist: Vec::new(), list_length: 0, usable_rows: 0,
            piles: 0, gauge: 0, selected: 0, resized_for_browser: false,
            interface_color_pair: vec![0; NUMBER_OF_ELEMENTS],
            allfuncs: None, shortcuts: None, syntaxes: None,
            commandname: None, planted_shortcut: None,
            COLS: 80, LINES: 24, fill: -1, wrap_at: 0,
        }
    }
}

/// 全局状态单例（线程安全，单线程使用）。
thread_local! {
    pub static GLOBAL: RefCell<GlobalState> = RefCell::new(GlobalState::new());
}

// ======================== 独立于 GLOBAL 借用的字符层全局 ========================
// `chars` 模块的函数大量读取 `using_utf8`/`tabsize`/`word_chars`/`as_an_at`。
// 若这些值放在 `GLOBAL`（RefCell<GlobalState>）中，则在持有 `GLOBAL` 借用时
// 调用任何 `chars` 函数都会触发 "RefCell already borrowed"。
// 因此把它们放入独立的 thread_local（Cell/RefCell），与 `GLOBAL` 相互独立，
// 任何借用状态下都可安全访问。`GlobalState` 中的对应字段保留用于同步。

thread_local! {
    /// 是否使用 UTF-8（对应 C 全局 `using_utf8`）。
    pub static UTF8_FLAG: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    /// 制表符宽度（对应 C 全局 `tabsize`）。
    pub static TABSIZE_VALUE: std::cell::Cell<usize> = const { std::cell::Cell::new(8) };
    /// 单词字符集（对应 C 全局 `word_chars`）。
    pub static WORD_CHARS_VALUE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// 是否把回车显示为 @（对应 C 全局 `as_an_at`）。
    pub static AS_AN_AT_VALUE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 全局 flag 位（对应 C 全局 `flags`）。
    pub static FLAGS_VALUE: std::cell::RefCell<GlobalFlags> = const { std::cell::RefCell::new(GlobalFlags::new()) };
}

/// 读取 flag（独立于 GLOBAL 借用）。
pub fn is_flag_set(flag: usize) -> bool {
    FLAGS_VALUE.with(|f| f.borrow().isset(flag))
}

/// 设置 flag（独立于 GLOBAL 借用）。
pub fn set_flag(flag: usize) {
    FLAGS_VALUE.with(|f| f.borrow_mut().set(flag));
}

/// 清除 flag（独立于 GLOBAL 借用）。
pub fn unset_flag(flag: usize) {
    FLAGS_VALUE.with(|f| f.borrow_mut().unset(flag));
}

/// 切换 flag（独立于 GLOBAL 借用）。
pub fn toggle_flag(flag: usize) {
    FLAGS_VALUE.with(|f| f.borrow_mut().toggle(flag));
}

/// 克隆 flags（独立于 GLOBAL 借用）。
pub fn clone_flags() -> GlobalFlags {
    FLAGS_VALUE.with(|f| f.borrow().clone())
}

/// 恢复 flags（独立于 GLOBAL 借用）。
pub fn restore_flags(flags: GlobalFlags) {
    FLAGS_VALUE.with(|f| *f.borrow_mut() = flags);
}

/// 读取 using_utf8（独立于 GLOBAL 借用）。
pub fn using_utf8_independent() -> bool {
    UTF8_FLAG.with(|c| c.get())
}

/// 设置 using_utf8（独立于 GLOBAL 借用）。
pub fn set_using_utf8_independent(val: bool) {
    UTF8_FLAG.with(|c| c.set(val));
}

/// 读取 tabsize（独立于 GLOBAL 借用）。
pub fn tabsize_independent() -> usize {
    TABSIZE_VALUE.with(|c| c.get())
}

/// 设置 tabsize（独立于 GLOBAL 借用）。
pub fn set_tabsize_independent(val: usize) {
    TABSIZE_VALUE.with(|c| c.set(val));
}

/// 读取 word_chars（独立于 GLOBAL 借用）。
pub fn word_chars_independent() -> Option<String> {
    WORD_CHARS_VALUE.with(|c| c.borrow().clone())
}

/// 设置 word_chars（独立于 GLOBAL 借用）。
pub fn set_word_chars_independent(val: Option<String>) {
    WORD_CHARS_VALUE.with(|c| *c.borrow_mut() = val);
}

/// 读取 as_an_at（独立于 GLOBAL 借用）。
pub fn as_an_at_independent() -> bool {
    AS_AN_AT_VALUE.with(|c| c.get())
}

/// 设置 as_an_at（独立于 GLOBAL 借用）。
pub fn set_as_an_at_independent(val: bool) {
    AS_AN_AT_VALUE.with(|c| c.set(val));
}

/// 访问全局状态的便捷宏。
#[macro_export]
macro_rules! global {
    ($field:ident) => {
        $crate::definitions::GLOBAL.with(|g| g.borrow().$field.clone())
    };
    ($field:ident, $val:expr) => {
        $crate::definitions::GLOBAL.with(|g| g.borrow_mut().$field = $val.into())
    };
}

/// 对全局状态执行闭包的便捷函数。
pub fn with_global<F, R>(f: F) -> R where F: FnOnce(&GlobalState) -> R {
    GLOBAL.with(|g| f(&*g.borrow()))
}

pub fn with_global_mut<F, R>(f: F) -> R where F: FnOnce(&mut GlobalState) -> R {
    GLOBAL.with(|g| f(&mut *g.borrow_mut()))
}

// ======================== 辅助函数 ========================

/// 致命错误退出。
pub fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

/// 复制字符串。
pub fn copy_of(string: &str) -> String {
    string.to_string()
}

/// 分配并复制 src 的前 count 个字节。
pub fn measured_copy(string: &[u8], count: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(count + 1);
    let end = count.min(string.len());
    v.extend_from_slice(&string[..end]);
    v.push(0);
    v
}

/// 创建新行节点（对应 text.c 的 `make_new_node`）。
/// `given` 为 `None` 时相当于 C 的 `make_new_node(NULL)`，行号为 1。
/// 注意：与 C 一致，`prev`/`next`/`data` 由调用方随后设置。
pub fn make_new_node(given: Option<&LineStruct>) -> LineRef {
    let lineno = match given {
        Some(g) => g.lineno + 1,
        None => 1,
    };
    Rc::new(RefCell::new(LineStruct {
        data: String::new(),
        lineno,
        next: None,
        prev: None,
        multidata: None,
        has_anchor: false,
    }))
}

/// 删除节点（调用方负责从链表中摘除）。
pub fn delete_node(_node: LineRef) {
    // 当最后一个引用消失时，Rc 自动释放
}

/// 宏：标记稍后翻译的字符串。
#[allow(non_snake_case)]
pub fn N_(string: &'static str) -> &'static str { string }

/// 版本信息。
pub const VERSION: &str = "8.5";
pub const REVISION: &str = "";