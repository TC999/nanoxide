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
    /// 捕获组数量（对应 POSIX `re_nsub`）。
    nsub: usize,
}

/// 一次匹配的结果（对应 C 的 `regmatch_t` 数组）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    /// 整体匹配的起点/终点。
    pub so: usize,
    pub eo: usize,
    /// 第 0 项为整体匹配，1..=nsub 为捕获组；未参与的组为 None。
    pub groups: Vec<Option<(usize, usize)>>,
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
    /// 词边界：`\b` 任意边界，`\<` 词首，`\>` 词尾。
    WordBoundary,
    /// 词首 `\<`（前一字符非单词、当前是单词字符）。
    WordStart,
    /// 词尾 `\>`（前一字符是单词、当前非单词或行尾）。
    WordEnd,
    /// 顺序连接。
    Seq(Vec<Node>),
    /// 交替 `|`（优先左侧）。
    Alt(Vec<Node>),
    /// 重复：min 次到 max 次（None 表示不限）。
    Rep(Box<Node>, usize, Option<usize>),
    /// 捕获组 `(...)`：内部节点与组号（从 1 开始）。
    Group(Box<Node>, usize),
}

impl Regex {
    /// 编译正则表达式。非法模式返回 Err(描述)。
    pub fn compile(pattern: &str, icase: bool) -> Result<Regex, String> {
        let mut parser = Parser {
            chars: pattern.as_bytes(),
            pos: 0,
            icase,
            groups: 0,
        };
        let prog = parser.parse_alt(false)?;
        if parser.pos != parser.chars.len() {
            return Err(crate::t!("regex-unexpected", ch = (parser.chars[parser.pos] as char).to_string()));
        }
        Ok(Regex {
            prog,
            nsub: parser.groups,
        })
    }

    /// 捕获组数量（对应 POSIX `re_nsub`）。
    pub fn nsub(&self) -> usize {
        self.nsub
    }

    /// 在 text 中从 start 起查找第一个匹配（部分匹配），返回 (起点, 终点)。
    /// notbol 为真时 `^` 不匹配（对应 REG_NOTBOL）。
    pub fn find(&self, text: &str, start: usize, notbol: bool) -> Option<(usize, usize)> {
        self.find_bytes(text.as_bytes(), start, notbol)
    }

    /// 字节串版本的 find（nano 文本为字节级存储）。
    /// POSIX 语义：最左优先、同起点取最长匹配（leftmost-longest）。
    pub fn find_bytes(&self, text: &[u8], start: usize, notbol: bool) -> Option<(usize, usize)> {
        self.find_bytes_captures(text, start, notbol)
            .map(|m| (m.so, m.eo))
    }

    /// 在 text 中从 start 起查找第一个匹配，并返回捕获组边界
    /// （对应 POSIX `regexec` 填写 `regmatch_t` 数组）。
    ///
    /// 整体匹配仍取最左、同起点最长；捕获组取该条最长路径上的值。
    /// POSIX 对同一整体长度下多条路径的子表达式优先级规则更细，
    /// 这里保留最先枚举到的那条，未逐一复刻。
    #[allow(clippy::type_complexity)]
    pub fn find_bytes_captures(
        &self,
        text: &[u8],
        start: usize,
        notbol: bool,
    ) -> Option<MatchResult> {
        let len = text.len();
        let start = start.min(len);
        for so in start..=len {
            let notbol_here = notbol || so != start;
            let initial = vec![None; self.nsub + 1];

            /* 收集所有可能终点，取最长（对应 POSIX leftmost-longest），
             * 同时保留产生该终点时的捕获组快照。 */
            let mut best: Option<(usize, Vec<Option<(usize, usize)>>)> = None;
            {
                let mut collect = |eo: usize, caps: &[Option<(usize, usize)>]| {
                    let better = match &best {
                        None => true,
                        Some((prev, _)) => eo > *prev,
                    };
                    if better {
                        best = Some((eo, caps.to_vec()));
                    }
                    None // 返回 None 以继续枚举其它可能终点
                };
                match_node(
                    &self.prog,
                    text,
                    so,
                    so,
                    len,
                    notbol_here,
                    &initial,
                    &mut collect,
                );
            }

            if let Some((eo, mut groups)) = best {
                groups[0] = Some((so, eo));
                return Some(MatchResult { so, eo, groups });
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
/// continuation k(新位置, 当前捕获组)；失败返回 None。量词贪婪并支持回退。
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn match_node(
    node: &Node,
    text: &[u8],
    pos: usize,
    bol_pos: usize,
    end: usize,
    notbol: bool,
    caps: &[Option<(usize, usize)>],
    k: &mut dyn FnMut(usize, &[Option<(usize, usize)>]) -> Option<usize>,
) -> Option<usize> {
    match node {
        Node::Char(lo, hi) => {
            if pos < end {
                let c = text[pos];
                let c2 = if c.is_ascii_uppercase() { c + 32 } else { c };
                if c2 >= *lo && c2 <= *hi {
                    return k(pos + 1, caps);
                }
            }
            None
        }
        Node::Any => {
            if pos < end {
                k(pos + 1, caps)
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
                k(pos + 1, caps)
            } else {
                None
            }
        }
        Node::Bol => {
            if !notbol && pos == bol_pos {
                k(pos, caps)
            } else {
                None
            }
        }
        Node::Eol => {
            if pos == end {
                k(pos, caps)
            } else {
                None
            }
        }
        Node::WordBoundary => {
            let before = if pos > 0 { is_word_char(text[pos - 1]) } else { false };
            let after = if pos < end { is_word_char(text[pos]) } else { false };
            if before != after {
                k(pos, caps)
            } else {
                None
            }
        }
        Node::WordStart => {
            let before = if pos > 0 { is_word_char(text[pos - 1]) } else { false };
            let after = if pos < end { is_word_char(text[pos]) } else { false };
            if !before && after {
                k(pos, caps)
            } else {
                None
            }
        }
        Node::WordEnd => {
            let before = if pos > 0 { is_word_char(text[pos - 1]) } else { false };
            let after = if pos < end { is_word_char(text[pos]) } else { false };
            if before && !after {
                k(pos, caps)
            } else {
                None
            }
        }
        Node::Seq(items) => {
            // 依次匹配每一项，最后一项的 continuation 是外层 k。
            #[allow(clippy::type_complexity, clippy::too_many_arguments)]
            fn walk(
                items: &[Node],
                idx: usize,
                text: &[u8],
                pos: usize,
                bol: usize,
                end: usize,
                notbol: bool,
                caps: &[Option<(usize, usize)>],
                k: &mut dyn FnMut(usize, &[Option<(usize, usize)>]) -> Option<usize>,
            ) -> Option<usize> {
                if idx == items.len() {
                    return k(pos, caps);
                }
                let mut cont = |np: usize, c: &[Option<(usize, usize)>]| {
                    walk(items, idx + 1, text, np, bol, end, notbol, c, &mut *k)
                };
                match_node(&items[idx], text, pos, bol, end, notbol, caps, &mut cont)
            }
            walk(items, 0, text, pos, bol_pos, end, notbol, caps, &mut *k)
        }
        Node::Alt(branches) => {
            for branch in branches {
                if let Some(np) =
                    match_node(branch, text, pos, bol_pos, end, notbol, caps, &mut *k)
                {
                    return Some(np);
                }
            }
            None
        }
        Node::Rep(inner, min, max) => {
            // 第一阶段：贪婪扩展，记录每一步的位置与捕获快照（含起点）。
            let mut positions: Vec<(usize, Vec<Option<(usize, usize)>>)> =
                vec![(pos, caps.to_vec())];
            let mut p = pos;
            loop {
                let count = positions.len() - 1;
                if let Some(mv) = max {
                    if count >= *mv {
                        break;
                    }
                }
                let step_start = p;
                let step_caps = positions[count].1.clone();
                let mut next: Option<(usize, Vec<Option<(usize, usize)>>)> = None;
                {
                    let mut capture = |np: usize, c: &[Option<(usize, usize)>]| {
                        if np == step_start {
                            None // 空匹配：停止扩展
                        } else {
                            next = Some((np, c.to_vec()));
                            Some(np)
                        }
                    };
                    match_node(inner, text, p, bol_pos, end, notbol, &step_caps, &mut capture);
                }
                match next {
                    Some((np, c)) => {
                        p = np;
                        positions.push((np, c));
                    }
                    None => break,
                }
            }
            // 第二阶段：从多到少尝试次数，调用外层 continuation。
            let mut count = positions.len() - 1;
            loop {
                if count >= *min {
                    if let Some(r) = k(positions[count].0, &positions[count].1) {
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
        Node::Group(inner, index) => {
            let start = pos;
            let mut cont = |np: usize, inner_caps: &[Option<(usize, usize)>]| {
                let mut updated = inner_caps.to_vec();
                updated[*index] = Some((start, np));
                k(np, &updated)
            };
            match_node(inner, text, pos, bol_pos, end, notbol, caps, &mut cont)
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
    /// 已分配的捕获组数量（对应 re_nsub）。
    groups: usize,
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

    /// 解析交替：expr (| expr)*。stop_at_paren 为真时 `)` 结束表达式（分组内部）。
    fn parse_alt(&mut self, stop_at_paren: bool) -> Result<Node, String> {
        let mut branches = vec![self.parse_seq(stop_at_paren)?];
        loop {
            if self.peek() == Some(b'|') {
                self.pos += 1;
                branches.push(self.parse_seq(stop_at_paren)?);
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

    /// 解析连接：piece*。stop_at_paren 为真时 `)` 结束表达式。
    fn parse_seq(&mut self, stop_at_paren: bool) -> Result<Node, String> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(b'|') => break,
                Some(b')') if stop_at_paren => break,
                _ => items.push(self.parse_piece(stop_at_paren)?),
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
    fn parse_piece(&mut self, stop_at_paren: bool) -> Result<Node, String> {
        let atom = self.parse_atom(stop_at_paren)?;
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

    /// 解析一个原子（字符、类、分组、锚点、词边界、转义）。
    fn parse_atom(&mut self, _stop_at_paren: bool) -> Result<Node, String> {
        let c = self.eat().ok_or("unexpected end")?;
        match c {
            b'.' => Ok(Node::Any),
            b'^' => Ok(Node::Bol),
            b'$' => Ok(Node::Eol),
            b'[' => self.parse_class(),
            b'(' => {
                self.groups += 1;
                let index = self.groups;
                let inner = self.parse_alt(true)?;
                if self.eat() != Some(b')') {
                    return Err("missing ')'".into());
                }
                Ok(Node::Group(Box::new(inner), index))
            }
            b'\\' => self.parse_escape(),
            b'*' | b'+' | b'?' => Err(crate::t!("regex-dangling", ch = (c as char).to_string())),
            /* ERE 中无法构成重复的裸 {、} 及多余的分隔符/右括号按字面处理
             * （对应 glibc regcomp 的容错行为）。stop_at_paren 为真时 `)` 由
             * parse_seq 截断，不会到达此处。 */
            b'{' | b'}' | b'|' | b')' => Ok(self.char_node(c)),
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

    /// 解析转义序列。POSIX ERE 语义：转义的特殊字符是字面量
    /// （`\(` `\)` `\|` `\{` `\}` `\+` `\?` `\*` `\.` `\^` `\$`），
    /// 另支持 GNU 扩展的 `\<` `\>` `\b` 词边界与 `\s` `\S` `\w` `\W` `\d` `\D` 字符类。
    fn parse_escape(&mut self) -> Result<Node, String> {
        let c = self.eat().ok_or("dangling '\\'")?;
        match c {
            b'<' => Ok(Node::WordStart),
            b'>' => Ok(Node::WordEnd),
            b'b' => Ok(Node::WordBoundary),
            /* GNU 扩展字符类。 */
            b's' => Ok(Node::Class(vec![(b'\t', b'\r'), (b' ', b' ')], false)),
            b'S' => Ok(Node::Class(vec![(b'\t', b'\r'), (b' ', b' ')], true)),
            b'w' => Ok(Node::Class(
                vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z'), (b'_', b'_')],
                false,
            )),
            b'W' => Ok(Node::Class(
                vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z'), (b'_', b'_')],
                true,
            )),
            b'd' => Ok(Node::Class(vec![(b'0', b'9')], false)),
            b'D' => Ok(Node::Class(vec![(b'0', b'9')], true)),
            /* 转义的特殊字符：字面量（POSIX ERE）。 */
            b'(' | b')' | b'|' | b'{' | b'}' | b'+' | b'?' | b'*' | b'.' | b'^' | b'$' | b'[' | b']' => {
                Ok(self.char_node(c))
            }
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

/// 展开 POSIX 字符类名称为字节范围（含 GNU 扩展）。
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
        /* GNU 扩展：word = alnum + 下划线。 */
        b"word" => vec![(b'0', b'9'), (b'A', b'Z'), (b'a', b'z'), (b'_', b'_')],
        b"ascii" => vec![(0, 127)],
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
    fn word_start_and_end_differ() {
        /* `<` 只在词首匹配（在 xword 中不匹配，在后面的 word 匹配）。 */
        assert_eq!(find(r"\<word", "xword word"), Some((6, 10)));
        assert_eq!(find(r"\<word", "xword"), None);
        /* `>` 只在词尾匹配（在 wordy 中不匹配，在后面的 word 匹配）。 */
        assert_eq!(find(r"word\>", "wordy word"), Some((6, 10)));
        assert_eq!(find(r"word\>", "wordy"), None);
        /* `\b` 仍是任意边界。 */
        assert_eq!(find(r"\bword", "wordy"), Some((0, 4)));
        assert_eq!(find(r"word\b", "wordy"), None);
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

    #[test]
    fn leftmost_longest_semantics() {
        // POSIX leftmost-longest：同起点取最长匹配
        assert_eq!(find("a|aa", "aa"), Some((0, 2)));
        assert_eq!(find("aa|a", "aa"), Some((0, 2)));
        assert_eq!(find("(a|aa)b", "aab"), Some((0, 3)));
        assert_eq!(find("a*", "aaa"), Some((0, 3)));
        assert_eq!(find("a*|b", "aa"), Some((0, 2)));
        // 最左优先
        assert_eq!(find("b|a", "xxayy"), Some((2, 3)));
    }

    #[test]
    fn gnu_word_class() {
        // GNU 扩展 [ [:word:] ] = alnum + 下划线
        let re = Regex::compile("[[:word:]]+", false).unwrap();
        assert_eq!(re.find("abc_123", 0, false), Some((0, 7)));
        assert_eq!(re.find("a-b", 0, false), Some((0, 1)), "a-b 中只应匹配 a");
        let re2 = Regex::compile("\\<[[:word:]]+\\>", false).unwrap();
        assert_eq!(re2.find("hello_world", 0, false), Some((0, 11)));
        assert_eq!(re2.find("xx hello_world yy", 0, false), Some((0, 2)), "最左优先：先匹配 xx");
    }

    #[test]
    fn nested_repetition() {
        assert_eq!(find("(ab)*", "ababab"), Some((0, 6)));
        assert_eq!(find("a{2,4}b", "aaaab"), Some((0, 5)));
        assert_eq!(find("(a|b)*c", "ababc"), Some((0, 5)));
    }

    #[test]
    fn ere_escaped_literals() {
        // POSIX ERE：\( \) \| 等转义是字面量（不是分组/交替）
        assert_eq!(find(r"\(", "a(b"), Some((1, 2)));
        assert_eq!(find(r"\(|\)|\[|\]|\{|\}", "{"), Some((0, 1)));
        assert_eq!(find(r"\(x\)", "(x)"), Some((0, 3)));
        assert_eq!(find(r"\\f(.|\(..)", "\\f("), Some((0, 3)));
        // 未转义括号是分组
        assert_eq!(find("(ab)+", "abab"), Some((0, 4)));
        // 无法构成重复的裸 { 按字面
        assert_eq!(find("a{foo", "a{foo"), Some((0, 5)));
    }

    #[test]
    fn gnu_class_escapes() {
        // GNU 扩展：\s \S \w \W \d \D 字符类
        assert_eq!(find(r"\s+", "ab cd"), Some((2, 3)));
        assert_eq!(find(r"\S+", " ab "), Some((1, 3)));
        assert_eq!(find(r"\w+", "a_b-1"), Some((0, 3)));
        assert_eq!(find(r"\W+", "ab - cd"), Some((2, 5)));
        assert_eq!(find(r"\d+", "ab12cd"), Some((2, 4)));
        assert_eq!(find(r"\D+", "12ab34"), Some((2, 4)));
    }

    #[test]
    fn trailing_paren_is_literal() {
        // glibc 容错：多余右括号按字面处理（对应 objc.nanorc 的 "selector)\>"）
        assert_eq!(find("a)b", "a)b"), Some((0, 3)));
        // 分组括号与多余右括号字面共存
        assert_eq!(find("@(x)|y)\\>", "@x"), Some((0, 2)));
        /* `\>` 是词尾：要求前一字符是词字符。"y)" 的 ')' 之前是 'y'，
         * 但 ')' 之后（文本尾）不是词尾，因此两种文本都不匹配。 */
        assert_eq!(find("@(x)|y)\\>", "y)"), None);
        assert_eq!(find("@(x)|y)\\>", "y)a"), None);
        assert_eq!(find("a|b)", "b)"), Some((0, 2)));
    }

    #[test]
    fn capture_groups() {
        let re = Regex::compile("(a+)(b+)", false).unwrap();
        assert_eq!(re.nsub(), 2);
        let m = re.find_bytes_captures(b"aaabb", 0, false).unwrap();
        assert_eq!((m.so, m.eo), (0, 5));
        assert_eq!(m.groups[1], Some((0, 3)));
        assert_eq!(m.groups[2], Some((3, 5)));
    }

    #[test]
    fn capture_nested_and_optional() {
        let re = Regex::compile("((a)(b))?c", false).unwrap();
        assert_eq!(re.nsub(), 3);

        let m = re.find_bytes_captures(b"abc", 0, false).unwrap();
        assert_eq!((m.so, m.eo), (0, 3));
        assert_eq!(m.groups[1], Some((0, 2)));
        assert_eq!(m.groups[2], Some((0, 1)));
        assert_eq!(m.groups[3], Some((1, 2)));

        /* 可选组未参与匹配时为 None。 */
        let m2 = re.find_bytes_captures(b"c", 0, false).unwrap();
        assert_eq!(m2.groups[1], None);
        assert_eq!(m2.groups[2], None);
        assert_eq!(m2.groups[3], None);
    }

    #[test]
    fn capture_repetition_takes_last() {
        /* 重复的捕获组取最后一次迭代（POSIX 行为）。 */
        let re = Regex::compile("(a|b)+", false).unwrap();
        let m = re.find_bytes_captures(b"abab", 0, false).unwrap();
        assert_eq!(m.groups[1], Some((3, 4)));
    }

    #[test]
    fn nsub_counts_only_capturing_parens() {
        assert_eq!(Regex::compile("(a)(b)(c)", false).unwrap().nsub(), 3);
        /* `\(` 是字面量，不算捕获组。 */
        assert_eq!(Regex::compile(r"\(a\)(b)", false).unwrap().nsub(), 1);
        assert_eq!(Regex::compile("a|b", false).unwrap().nsub(), 0);
    }
}
