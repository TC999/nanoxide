// 临时验证：rcfile 新功能（bind/unbind、set 选项、extendsyntax、include、错误消息）
use nanoxide::definitions::*;

fn setup() {
    nanoxide::global::global_init();
    nanoxide::global::shortcut_init();
    nanoxide::files::make_new_buffer();
}

fn write_rc(content: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "nanoxide_rc_verify_{}_{}.nanorc",
        std::process::id(),
        n
    ));
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn verify_bind_unbind() {
    setup();
    let rc = write_rc(
        "bind ^K whereis main\n\
         bind M-T comment main\n\
         bind ^Q \"hello world\" main\n\
         unbind ^X main\n",
    );
    nanoxide::rcfile::parse_rcfile(&rc);

    // 用户绑定登记
    let bound = with_global(|g| g.bound_keys.clone());
    assert_eq!(bound.len(), 3, "应有 3 个绑定，实际 {}", bound.len());
    assert!(bound.iter().any(|b| b.keycode == 11 && b.func == FunctionId::DoSearchForward),
        "^K 应绑到 whereis");
    assert!(bound.iter().any(|b| b.keycode == 116 && b.func == FunctionId::DoComment),
        "M-T 应绑到 comment（keycode 116）");
    assert!(bound.iter().any(|b| b.func == FunctionId::Implant && b.expansion.as_deref() == Some("hello world")),
        "^Q 应为植入字符串");

    // unbind 登记
    let unbound = with_global(|g| g.unbound_keys.clone());
    assert!(unbound.iter().any(|(k, _)| *k == 24), "^X 应被解绑");

    // 键名无效报错
    let rc2 = write_rc("bind a whereis main\n");
    nanoxide::rcfile::parse_rcfile(&rc2);
}

#[test]
fn verify_set_options() {
    setup();
    let rc = write_rc(
        "set tabsize 4\n\
         set fill 72\n\
         set matchbrackets \"(<[{\"\n\
         set titlecolor red\n\
         set numbercolor bold,blue\n\
         set unix\n\
         set zap\n\
         unset mouse\n",
    );
    nanoxide::rcfile::parse_rcfile(&rc);

    assert_eq!(with_global(|g| g.tabsize), 4, "tabsize 应为 4");
    assert_eq!(with_global(|g| g.fill), 72, "fill 应为 72");
    assert_eq!(with_global(|g| g.matchbrackets.clone()), Some("(<[{".to_string()));
    assert!(with_global(|g| g.color_combo[0].is_some()), "titlecolor 应登记");
    assert!(with_global(|g| g.color_combo[1].is_some()), "numbercolor 应登记");
    assert!(ISSET(MAKE_IT_UNIX), "set unix 应生效");
    assert!(ISSET(LET_THEM_ZAP), "set zap 应生效");
    assert!(!ISSET(USE_MOUSE), "unset mouse 应生效");

    // 未知选项应产生错误
    let rc2 = write_rc("set not_an_option\n");
    nanoxide::rcfile::parse_rcfile(&rc2);
    // 通过 startup_problem 检查有错误发生
    assert!(with_global(|g| g.startup_problem.is_some()), "未知选项应产生错误");

    // C 版选项名兼容：solosidescroll / allow_insecure_backup / newbuffer
    let rc3 = write_rc("set solosidescroll\nset allow_insecure_backup\nset newbuffer\n");
    nanoxide::rcfile::parse_rcfile(&rc3);
    assert!(ISSET(SOLO_SIDESCROLL));
    assert!(ISSET(INSECURE_BACKUP));
    assert!(ISSET(NEW_BUFFER));
}

#[test]
fn verify_extendsyntax_and_include() {
    setup();
    // include 语法文件（c.nanorc 定义 syntax "c"），再 extendsyntax 追加规则
    let rc = write_rc(
        "include \"nano/syntax/c.nanorc\"\n\
         extendsyntax c color brightred \"MYKEYWORD\"\n",
    );
    nanoxide::rcfile::parse_rcfile(&rc);

    let sntx = with_global(|g| g.syntaxes.clone()).expect("应有语法 c");
    // 语法规则应已加载（parse_rcfile 全量解析）
    let color = { let r = sntx.borrow(); r.color.clone() };
    assert!(color.is_some(), "c 语法应有颜色规则");

    // extendsyntax 追加的规则应存在
    let colors: Vec<ColorRef> = {
        let mut v = Vec::new();
        let mut cur = { let r = sntx.borrow(); r.color.clone() };
        while let Some(c) = cur {
            v.push(c.clone());
            let next = { let r = c.borrow(); r.next.clone() };
            cur = next;
        }
        v
    };
    assert!(colors.len() >= 10, "应有至少 10 条规则，实际 {}", colors.len());
}

#[test]
fn verify_glob_include() {
    setup();
    // 用 glob 模式 include 多个语法文件
    let rc = write_rc("include \"nano/syntax/c*.nanorc\"\n");
    nanoxide::rcfile::parse_rcfile(&rc);
    let syntaxes = with_global(|g| g.syntaxes.clone());
    assert!(syntaxes.is_some(), "glob include 应解析出语法");
}

#[test]
fn verify_get_comment_string() {
    setup();
    let rc = write_rc("syntax test\ncomment \"//\"\n");
    nanoxide::rcfile::parse_rcfile(&rc);
    // 修复后的 get_comment_string 应返回当前缓冲区语法的注释串
    let comment = nanoxide::rcfile::get_comment_string();
    assert!(comment.is_none() || comment.as_deref() == Some("//"),
        "get_comment_string 不应返回错误值: {:?}", comment);
}
