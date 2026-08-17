/**************************************************************************
 * regex.rs  --  GNU 风格正则表达式引擎（替代 POSIX regexec）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 为 syntax 高亮、搜索与引用段落提供正则匹配。
//! 原版 nano 使用 POSIX 正则（`regcomp`/`regexec`）；本项目不引入
//! 外部 crate，这里实现一个 GNU 风格正则的子集引擎：
//!   - 支持 ERE 语法：`.`、`[...]`/`[^...]`、`*`、`+`、`?`、`{m,n}`、
//!     `(...)` 分组、`|` 交替、`^`/`$` 锚点
//!   - GNU 扩展：`\<`、`\>`、`\b` 词边界，`\(`/`\)`/`\|`/`\{`/`\}` 兼容
//!   - POSIX 字符类 `[[:alpha:]]` 等
//!   - 大小写不敏感选项（对应 REG_ICASE）
//! 匹配语义与 `regexec(text + start, REG_NOTBOL)` 一致：
//! 在 text[start..] 中查找最左优先（同起点贪婪取最长）的匹配。

/// 正则表达式（已编译的匹配树）。
#[derive(Debug, Clone)]
pub struct Regex {
    prog: Node,
}

/// 匹配树的节点。
#[derive(Debug, Clone)]
enum Node {
    /// 单个字符（含大小写折叠后的两个边界）。
    Char(u8, u8),
    /// 任意单字符 `.`。
    Any,
    /// 字符类：ranges 为 (lo, hi) 列表，negated 表示取反。
    Class(Vec<(u8, u8)>, bool),
    /// 行首锚点 `^`。
    Bol,
    /// 行尾锚点 `$`。
    Eol,
    /// 词边界 `\<`、`\>`、`\b`。
    WordBoundary,
    /// 顺序连接。
    Seq(Vec<Node>),
    /// 交替 `|`（优先左侧）。
    Alt(Vec<Node>),
    /// 重复：min 次到 max 次（None 表示不限）。
    Rep(Box<Node>, usize, Option<usize>),
}

impl Regex {
    /// 编译正则表达式。非法模式返回 Err(描述)。
    pub fn compile(pattern: &str, icase: bool) -> Result<Regex, String> {
        let mut parser = Parser {
            chars: pattern.as_bytes(),
            pos: 0,
            icase,
        };
        let prog = parser.parse_alt()?;
        if parser.pos != parser.chars.len() {
            return Err(crate::t!("regex-unexpected", ch = (parser.chars[parser.pos] as char).to_string()));
        }
        Ok(Regex { prog })
    }

    /// 在 text 中从 start 起查找第一个匹配（部分匹配），返回 (起点, 终点)。
    /// notbol 为真时 `^` 不匹配（对应 REG_NOTBOL）。
    pub fn find(&self, text: &str, start: usize, notbol: bool) -> Option<(usize, usize)> {
        self.find_bytes(text.as_bytes(), start, notbol)
    }

    /// 字节串版本的 find（nano 文本为字节级存储）。
    pub fn find_bytes(&self, text: &[u8], start: usize, notbol: bool) -> Option<(usize, usize)> {
        let len = text.len();
        let start = start.min(len);
        for so in start..=len {
            let notbol_here = notbol || so != start;
            let eo = match_node(&self.prog, text, so, so, len, notbol_here, &|eo| Some(eo));
            if let Some(eo) = eo {
                return Some((so, eo));
            }
        }
        None
    }

    /// 判断 text 中是否存在匹配（对应 regexec 的布尔用途）。
    pub fn is_match(&self, text: &str) -> bool {
        self.find_bytes(text.as_bytes(), 0, false).is_some()
    }

    /// 判断 text 是否整体匹配（从头到尾）。
    pub fn full_match(&self, text: &str) -> bool {
        match self.find_bytes(text.as_bytes(), 0, false) {
            Some((0, eo)) => eo == text.len(),
            _ => false,
        }
    }
}

/// 递归回溯（continuation 风格）：匹配 node 从 pos 开始，成功后调用
/// continuation k(新位置)；失败返回 None。量词贪婪并支持回退。
fn match_node(node: &Node, text: &[u8], pos: usize, bol_pos: usize, end: usize, notbol: bool, k: &dyn Fn(usize) -> Option<usize>) -> Option<usize> {
    match node {
        Node::Char(lo, hi) => {
            if pos < end {
                let c = text[pos];
                let c2 = if c.is_ascii_uppercase() { c + 32 } else { c };
                if c2 >= *lo && c2 <= *hi {
                    return k(pos + 1);
                }
            }
            None
        }
        Node::Any => {
            if pos < end {
                k(pos + 1)
            } else {
                None
            }
        }
        Node::Class(ranges, negated) => {
            if pos >= end {
                return None;
            }
            let c = text[pos];
            let c2 = if c.is_ascii_uppercase() { c + 32 } else { c };
            let mut in_class = false;
            for (lo, hi) in ranges {
                if c2 >= *lo && c2 <= *hi {
                    in_class = true;
                    break;
                }
            }
            if in_class != *negated {
                k(pos + 1)
            } else {
                None
            }
        }
        Node::Bol => {
            if !notbol && pos == bol_pos {
                k(pos)
            } else {
                None
            }
        }
        Node::Eol => {
            if pos == end {
                k(pos)
            } else {
                None
            }
        }
        Node::WordBoundary => {
            let before = if pos > 0 { is_word_char(text[pos - 1]) } else { false };
            let after = if pos < end { is_word_char(text[pos]) } else { false };
            if before != after {
                k(pos)
            } else {
                None
            }
        }
        Node::Seq(items) => {
            // 依次匹配每一项，最后一项的 continuation 是外层 k。
            fn walk(items: &[Node], idx: usize, text: &[u8], pos: usize, bol: usize, end: usize, notbol: bool, k: &dyn Fn(usize) -> Option<usize>) -> Option<usize> {
                if idx == items.len() {
                    return k(pos);
                }
                let cont = |np: usize| walk(items, idx + 1, text, np, bol, end, notbol, k);
                match_node(&items[idx], text, pos, bol, end, notbol, &cont)
            }
            walk(items, 0, text, pos, bol_pos, end, notbol, k)
        }
        Node::Alt(branches) => {
            for branch in branches {
                if let Some(np) = match_node(branch, text, pos, bol_pos, end, notbol, k) {
                    return Some(np);
                }
            }
            None
        }
        Node::Rep(inner, min, max) => {
            // 第一阶段：贪婪扩展，记录每一步的位置（含起点）。
            let mut positions: Vec<usize> = vec![pos];
            let mut p = pos;
            loop {
                let count = positions.len() - 1;
                if let Some(mv) = max {
                    if count >= *mv {
                        break;
                    }
                }
                let found = match_node(inner, text, p, bol_pos, end, notbol, &|np| {
                    if np == p {
                        None // 空匹配：停止扩展
                    } else {
                        Some(np)
                    }
                });
                match found {
                    Some(np) => {
                        p = np;
                        positions.push(p);
                    }
                    None => break,
                }
            }
            // 第二阶段：从多到少尝试次数，调用外层 continuation。
            let mut count = positions.len() - 1;
            loop {
                if count >= *min {
                    if let Some(r) = k(positions[count]) {
                        return Some(r);
                    }
                }
                if count == 0 {
                    break;
                }
                count -= 1;
            }
            None
        }
    }
}

fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// 解析器：把模式串编译为 Node。
struct Parser<'a> {
    chars: &'a [u8],
    pos: usize,
    icase: bool,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    /// 解析交替：expr (| expr)*
    fn parse_alt(&mut self) -> Result<Node, String> {
        let mut branches = vec![self.parse_seq()?];
        loop {
            if self.peek() == Some(b'|') {
                self.pos += 1;
                branches.push(self.parse_seq()?);
            } else {
                break;
            }
        }
        if branches.len() == 1 {
            Ok(branches.remove(0))
        } else {
            Ok(Node::Alt(branches))
        }
    }

    /// 解析连接：piece*
    fn parse_seq(&mut self) -> Result<Node, String> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(b'|') | Some(b')') => break,
                _ => items.push(self.parse_piece()?),
            }
        }
        if items.is_empty() {
            // 空分支匹配空串。
            Ok(Node::Seq(vec![]))
        } else if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(Node::Seq(items))
        }
    }

    /// 解析一个原子及其量词。
    fn parse_piece(&mut self) -> Result<Node, String> {
        let atom = self.parse_atom()?;
        // 量词
        let mut min = 1usize;
        let mut max: Option<usize> = Some(1);
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    min = 0;
                    max = None;
                }
                Some(b'+') => {
                    self.pos += 1;
                    min = 1;
                    max = None;
                }
                Some(b'?') => {
                    self.pos += 1;
                    min = 0;
                    max = Some(1);
                }
                Some(b'{') => {
                    // 尝试 {m}, {m,}, {m,n}
                    let save = self.pos;
                    self.pos += 1;
                    if let Ok((lo, hi)) = self.parse_braces() {
                        min = lo;
                        max = hi;
                    } else {
                        self.pos = save;
                        break;
                    }
                }
                Some(b'\\') if self.chars.get(self.pos + 1) == Some(&b'{') => {
                    // BRE 形式 \{m,n\}
                    let save = self.pos;
                    self.pos += 2;
                    let inner = match self.parse_braces_bre() {
                        Ok(v) => v,
                        Err(_) => {
                            self.pos = save;
                            break;
                        }
                    };
                    if self.peek() == Some(b'\\') && self.chars.get(self.pos + 1) == Some(&b'}') {
                        self.pos += 2;
                        min = inner.0;
                        max = inner.1;
                    } else {
                        self.pos = save;
                        break;
                    }
                }
                _ => break,
            }
        }
        if min == 1 && max == Some(1) {
            Ok(atom)
        } else {
            Ok(Node::Rep(Box::new(atom), min, max))
        }
    }

    /// 解析 {m}, {m,}, {m,n}（不含花括号本身），返回 (min, max)。
    /// 失败时把位置恢复到 start 并返回 Err。
    fn parse_braces(&mut self) -> Result<(usize, Option<usize>), String> {
        let start = self.pos;
        let mut lo = 0usize;
        let mut seen = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                lo = lo * 10 + (c - b'0') as usize;
                seen = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if !seen {
            self.pos = start;
            return Err("no digits".into());
        }
        // 无上限形式 {m,}
        if self.peek() == Some(b',') {
            self.pos += 1;
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok((lo, None));
            }
            let mut hi = 0usize;
            let mut seen2 = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    hi = hi * 10 + (c - b'0') as usize;
                    seen2 = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if !seen2 || self.peek() != Some(b'}') {
                self.pos = start;
                return Err("bad braces".into());
            }
            self.pos += 1;
            return Ok((lo, Some(hi)));
        }
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok((lo, Some(lo)));
        }
        self.pos = start;
        Err("bad braces".into())
    }

    /// 解析 BRE 形式的 {m,n} 内部（数字、逗号），末尾需是 \}（由调用方处理）。
    fn parse_braces_bre(&mut self) -> Result<(usize, Option<usize>), String> {
        let start = self.pos;
        let mut lo = 0usize;
        let mut seen = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                lo = lo * 10 + (c - b'0') as usize;
                seen = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if !seen {
            self.pos = start;
            return Err("no digits".into());
        }
        if self.peek() == Some(b',') {
            self.pos += 1;
            let mut hi = 0usize;
            let mut seen2 = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    hi = hi * 10 + (c - b'0') as usize;
                    seen2 = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if !seen2 {
                // {m,} 无上限
                return Ok((lo, None));
            }
            return Ok((lo, Some(hi)));
        }
        Ok((lo, Some(lo)))
    }

    /// 解析一个原子（字符、类、分组、锚点、词边界、转义）。
    fn parse_atom(&mut self) -> Result<Node, String> {
        let c = self.eat().ok_or("unexpected end")?;
        match c {
            b'.' => Ok(Node::Any),
            b'^' => Ok(Node::Bol),
            b'$' => Ok(Node::Eol),
            b'[' => self.parse_class(),
            b'(' => {
                let inner = self.parse_alt()?;
                if self.eat() != Some(b')') {
                    return Err("missing ')'".into());
                }
                Ok(inner)
            }
            b'\\' => self.parse_escape(),
            b'*' | b'+' | b'?' => Err(crate::t!("regex-dangling", ch = (c as char).to_string())),
            b'|' | b')' | b'{' | b'}' => Err(crate::t!("regex-unexpected", ch = (c as char).to_string())),
            _ => Ok(self.char_node(c)),
        }
    }

    fn char_node(&self, c: u8) -> Node {
        if self.icase {
            let lo = if c.is_ascii_uppercase() { c + 32 } else { c };
            Node::Char(lo, lo)
        } else {
            Node::Char(c, c)
        }
    }

    /// 解析转义序列（\x、\(、\)、\|、\{、\}、\<、\>、\b 等）。
    fn parse_escape(&mut self) -> Result<Node, String> {
        let c = self.eat().ok_or("dangling '\\'")?;
        match c {
            b'<' | b'>' | b'b' => Ok(Node::WordBoundary),
            b'(' => {
                let inner = self.parse_alt()?;
                if self.eat() != Some(b')') {
                    return Err("missing '\\)'".into());
                }
                Ok(inner)
            }
            b'{' => Ok(self.char_node(b'{')), // \{ 视作字面 {（量词由 parse_piece 处理）
            _ => Ok(self.char_node(c)),
        }
    }

    /// 解析字符类 [...]。
    fn parse_class(&mut self) -> Result<Node, String> {
        let mut negated = false;
        if self.peek() == Some(b'^') {
            negated = true;
            self.pos += 1;
        }
        let mut ranges: Vec<(u8, u8)> = Vec::new();
        let mut first = true;
        loop {
            let c = match self.peek() {
                None => return Err("missing ']'".into()),
                Some(b']') if !first => {
                    self.pos += 1;
                    break;
                }
                Some(c) => {
                    self.pos += 1;
                    c
                }
            };
            first = false;
            // POSIX 字符类 [[:alpha:]]
            if c == b'[' && self.peek() == Some(b':') {
                self.pos += 1;
                let mut name = Vec::new();
                while let Some(n) = self.peek() {
                    if n == b':' {
                        break;
                    }
                    name.push(n);
                    self.pos += 1;
                }
                if self.peek() != Some(b':') {
                    return Err("unterminated [: :] class".into());
                }
                self.pos += 1; // :
                if self.peek() != Some(b']') {
                    return Err("unterminated [: :] class".into());
                }
                self.pos += 1; // ]
                ranges.extend(posix_class(&name));
                continue;
            }
            // 范围 a-z
            if self.peek() == Some(b'-') && self.chars.get(self.pos + 1).map(|&n| n != b']').unwrap_or(false) {
                self.pos += 1;
                let hi = self.eat().ok_or("bad range")?;
                let (lo2, hi2) = self.fold_range(c, hi);
                ranges.push((lo2, hi2));
            } else {
                let (lo2, hi2) = if self.icase {
                    let l = if c.is_ascii_uppercase() { c + 32 } else { c };
                    (l, l)
                } else {
                    (c, c)
                };
                ranges.push((lo2, hi2));
            }
        }
        Ok(Node::Class(ranges, negated))
    }

    fn fold_range(&self, lo: u8, hi: u8) -> (u8, u8) {
        if self.icase {
            let l = if lo.is_ascii_uppercase() { lo + 32 } else { lo };
            let h = if hi.is_ascii_uppercase() { hi + 32 } else { hi };
            (l.min(h), l.max(h))
        } else {
            (lo.min(hi), lo.max(hi))
        }
    }
}

/// 展开 POSIX 字符类名称为字节范围。
fn posix_class(name: &[u8]) -> Vec<(u8, u8)> {
    match name {
        b"alpha" => vec![(b'A', b'Z'), (b'a', b'z')],
        b"digit" => vec![(b'0', b'9')],
        b"alnum" => vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')],
        b"upper" => vec![(b'A', b'Z')],
        b"lower" => vec![(b'a', b'z')],
        b"space" => vec![(b'\t', b'\r'), (b' ', b' ')],
        b"blank" => vec![(b'\t', b'\t'), (b' ', b' ')],
        b"xdigit" => vec![(b'0', b'9'), (b'a', b'f'), (b'A', b'F')],
        b"punct" => vec![
            (b'!', b'/'), (b':', b'@'), (b'[', b'`'), (b'{', b'~'),
        ],
        b"cntrl" => vec![(0, 31), (127, 127)],
        b"graph" => vec![(33, 126)],
        b"print" => vec![(32, 126)],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(pattern: &str, text: &str) -> Option<(usize, usize)> {
        Regex::compile(pattern, false).unwrap().find(text, 0, false)
    }

    #[test]
    fn literal_and_anchor() {
        assert_eq!(find("abc", "xxabcyy"), Some((2, 5)));
        assert_eq!(find("^abc", "xxabc"), None);
        assert_eq!(find("^abc", "abcxx"), Some((0, 3)));
        assert_eq!(find("abc$", "xxabc"), Some((2, 5)));
    }

    #[test]
    fn any_and_class() {
        assert_eq!(find("a.c", "xabcy"), Some((1, 4)));
        assert_eq!(find("[0-9]+", "ab123cd"), Some((2, 5)));
        assert_eq!(find("[^0-9]+", "123abc456"), Some((3, 6)));
        assert_eq!(find("[[:alpha:]]+", "12abc34"), Some((2, 5)));
    }

    #[test]
    fn quantifiers_and_alt() {
        assert_eq!(find("ab*c", "abbbc"), Some((0, 5)));
        assert_eq!(find("ab*c", "ac"), Some((0, 2)));
        assert_eq!(find("(foo|bar)", "xxbarxx"), Some((2, 5)));
        assert_eq!(find("a{2,3}", "xaaaay"), Some((1, 4)));
    }

    #[test]
    fn backtracking() {
        // a* 必须回退才能匹配后面的 ab
        assert_eq!(find("a*ab", "aaab"), Some((0, 4)));
        assert_eq!(find("(a|ab)c", "abc"), Some((0, 3)));
    }

    #[test]
    fn word_boundary_and_icase() {
        assert_eq!(find(r"\<word\>", "xx word yy"), Some((3, 7)));
        assert_eq!(find(r"\bword\b", "xxwordy"), None);
        let re = Regex::compile("hello", true).unwrap();
        assert_eq!(re.find("xxHELLOyy", 0, false), Some((2, 7)));
    }

    #[test]
    fn notbol_and_offset() {
        let re = Regex::compile("^abc", false).unwrap();
        assert_eq!(re.find("zzabc", 0, false), None);
        assert_eq!(re.find("zzabc", 2, false), Some((2, 5)));
        assert_eq!(re.find("zzabc", 2, true), None);
    }

    #[test]
    fn glob_translation() {
        // glob 语义经翻译后仍可用
        let re = Regex::compile(".*\\.c$", false).unwrap();
        assert!(re.is_match("main.c"));
        assert!(!re.is_match("main.c.txt"));
    }

    #[test]
    fn zero_width_and_advance() {
        // 零宽匹配应能返回
        let re = Regex::compile("$", false).unwrap();
        assert_eq!(re.find("abc", 0, false), Some((3, 3)));
        // ^ 在中间位置（零宽）
        let re2 = Regex::compile("^", false).unwrap();
        assert_eq!(re2.find("abc", 2, false), Some((2, 2)));
        assert_eq!(re2.find("abc", 1, true), None);
    }
}
