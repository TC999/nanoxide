// tests/syntax_check.rs - 语法高亮（syntax）集成测试
//
// 用 GNU nano 源码自带的真实语法文件（nano/syntax/*.nanorc）验证：
// 1. rcfile 解析出语法、颜色规则、多行规则
// 2. 正则引擎能匹配真实规则
// 3. update_line 渲染不崩溃

use nano_rs::definitions::*;

/// 初始化全局状态与一个空缓冲。
fn setup() {
    nano_rs::global::global_init();
    nano_rs::files::make_new_buffer();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
    });
}

/// 解析 GNU nano 自带的 C 语法文件。
fn parse_c_syntax() -> Option<SyntaxRef> {
    let ok = nano_rs::rcfile::parse_rcfile("nano/syntax/c.nanorc");
    assert!(ok, "c.nanorc 应能读取");
    with_global(|g| g.syntaxes.clone())
}

/// rcfile 应解析出 C 语法（名字、颜色规则、多行规则、扩展名）。
#[test]
fn syntax_parse_works() {
    setup();
    let syntaxes = parse_c_syntax();
    let sntx = syntaxes.as_ref().expect("应有语法");
    let s = sntx.borrow();

    assert_eq!(s.name.as_deref(), Some("c"), "语法名应为 c");
    assert_eq!(s.comment.as_deref(), Some("//"), "注释字符应解析");

    // 统计颜色规则
    let mut count = 0;
    let mut cur = s.color.clone();
    while let Some(c) = cur {
        count += 1;
        let next = { let r = c.borrow(); r.next.clone() };
        cur = next;
    }
    assert!(count >= 10, "C 语法应有多个颜色规则，实际 {count}");

    // 多行规则（start=/end=）应计入 multiscore
    assert!(s.multiscore >= 2, "应有至少 2 个多行规则，实际 {}", s.multiscore);

    // 扩展名正则
    let extensions = s.extensions.clone().expect("应有扩展名正则");
    let e = extensions.borrow();
    assert!(e.one_rgx.as_ref().unwrap().matches("main.c"));
    assert!(e.one_rgx.as_ref().unwrap().matches("main.cpp"));
    assert!(!e.one_rgx.as_ref().unwrap().matches("main.txt"));
}

/// icolor 应大小写不敏感。
#[test]
fn syntax_icolor_case_insensitive() {
    setup();
    // 先建立语法，再添加 icolor 规则
    nano_rs::rcfile::parse_rcfile_line("syntax test", "test", 1);
    nano_rs::rcfile::parse_rcfile_line("icolor brightred \"hello\"", "test", 2);
    let sntx = with_global(|g| g.syntaxes.clone()).expect("应有语法");
    let first = sntx.borrow().color.clone().expect("应有颜色规则");
    let r = first.borrow();
    let pat = r.start.as_ref().expect("应有正则");
    assert!(pat.matches("xx HELLO yy"), "icolor 应忽略大小写");
    assert!(pat.matches("hello"));
}

/// 单行正则规则应能匹配 C 代码中的关键字与注释。
#[test]
fn syntax_regex_matches() {
    setup();
    let sntx = parse_c_syntax().expect("应有语法");

    // 收集所有颜色规则
    let rules: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = sntx.borrow().color.clone();
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };

    // 找一个匹配 "for" 关键字（词边界）的规则
    let line = "int main() { for (int i = 0; i < 10; i++) {} }";
    let mut hit_keyword = false;
    for r in &rules {
        let rr = r.borrow();
        if rr.end.is_none() {
            if let Some(p) = &rr.start {
                if p.matches(line) {
                    hit_keyword = true;
                    break;
                }
            }
        }
    }
    assert!(hit_keyword, "应有关键字规则匹配 C 代码行");
}

/// 设置语法后 update_line 渲染不应崩溃。
#[test]
fn update_line_with_syntax_renders() {
    setup();
    let sntx = parse_c_syntax().expect("应有语法");

    // 打开一个带 C 代码的缓冲并绑定语法
    nano_rs::text::inject(b"int main() { return 0; }", 24);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().syntax = Some(sntx);
    });
    nano_rs::files::prepare_for_display();

    let line = with_global(|g| g.openfile.as_ref().unwrap().borrow().current.clone().unwrap());
    nano_rs::winio::update_line(&line, 0);
}

/// 多行规则（/* ... */）应更新 multidata 状态。
#[test]
fn multiline_rule_sets_multidata() {
    setup();
    let sntx = parse_c_syntax().expect("应有语法");

    // 构造多行内容：注释跨行
    nano_rs::text::inject(b"/* comment start", 16);
    with_global_mut(|g| {
        let of = g.openfile.as_ref().unwrap().clone();
        of.borrow_mut().syntax = Some(sntx.clone());
    });
    nano_rs::files::prepare_for_display();

    // 预计算多行信息
    nano_rs::color::precalc_multicolorinfo();

    // 第一行应标记 STARTSHERE 或 WHOLELINE（取决于规则）
    let line1 = with_global(|g| g.openfile.as_ref().unwrap().borrow().filetop.clone().unwrap());
    let md = line1.borrow().multidata.clone().expect("应有 multidata");
    // 注释规则是最后一个多行规则（id = multiscore - 1）
    let id = with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().syntax.as_ref().unwrap().borrow().multiscore - 1
    });
    assert_eq!(md[id as usize] as i32, STARTSHERE, "注释起始行应标记 STARTSHERE");
}

/// 完整链路：按文件名匹配语法 → 建立颜色对 → 准备调色板。
#[test]
fn syntax_binding_primes_colorpairs() {
    setup();
    parse_c_syntax();

    // 打开一个 .c 文件并注入代码
    let ok = nano_rs::files::open_buffer("test_syntax.c");
    assert!(ok);
    nano_rs::text::inject(b"int main(void) { return 0; }", 28);
    nano_rs::files::prepare_for_display();

    // 按扩展名匹配语法并绑定
    nano_rs::color::find_and_prime_applicable_syntax();

    let bound = with_global(|g| {
        g.openfile.as_ref().unwrap().borrow().syntax.clone()
    });
    assert!(bound.is_some(), "应为 .c 文件绑定 C 语法");

    let sntx = bound.unwrap();
    assert_eq!(sntx.borrow().name.as_deref(), Some("c"));

    // 建立颜色对并准备调色板
    nano_rs::color::set_syntax_colorpairs(&sntx);
    nano_rs::color::prepare_palette();

    // 颜色规则应分配到非零 pairnum，且颜色对表有对应条目
    let first = sntx.borrow().color.clone().expect("应有颜色规则");
    let r = first.borrow();
    let pn = r.pairnum;
    assert!(pn > 0, "pairnum 应被分配，实际 {pn}");
    assert!(r.attributes & (pn as i32) != 0 || r.attributes >> 16 > 0,
            "attributes 应编码颜色对");
    let lookup = nano_rs::color::lookup_pair(pn as i32);
    assert!(lookup.is_some(), "颜色对表应有 pairnum={pn} 的条目");
}
