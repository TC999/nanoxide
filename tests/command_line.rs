//! 命令行解析：短选项映射、getopt_long 语义与 9.2 缺失选项。
//!
//! 对照 nano 9.2 的 nano.c：getopt 字符串
//! `ABC:DEFGHIJ:KLMNOPQ:RST:UVWX:Y:Zabcdef:ghijklmno:pqr:s:tuvwxyz!@%_01/`
//! 与 long_options[] 表。

use nanoxide::definitions::*;
use nanoxide::global::{parse_args, parse_command_line, parse_file_args};
use std::process::Command;

fn args(list: &[&str]) -> Vec<String> {
    std::iter::once("nax".to_string())
        .chain(list.iter().map(|s| s.to_string()))
        .collect()
}

/// 重置全局状态，避免同一测试进程内的标志互相干扰。
fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
}

/// 每个标志型短选项都应设置 9.2 中对应的标志。
#[test]
fn short_options_match_nano_92() {
    let cases: &[(char, usize)] = &[
        ('A', SMART_HOME),
        ('B', MAKE_BACKUP),
        ('D', BOLD_TEXT),
        ('E', TABS_TO_SPACES),
        ('F', NEW_BUFFER),
        ('G', LOCKING),
        ('H', HISTORYLOG),
        ('K', RAW_SEQUENCES),
        ('L', NO_NEWLINES),
        ('M', TRIM_BLANKS),
        ('N', NO_CONVERT),
        ('O', BOOKSTYLE),
        ('P', POSITIONLOG),
        ('R', RESTRICTED),
        ('S', SOFTWRAP),
        ('U', QUICK_BLANK),
        ('W', WORD_BOUNDS),
        ('Z', LET_THEM_ZAP),
        ('a', AT_BLANKS),
        ('c', CONSTANT_SHOW),
        ('d', REBIND_DELETE),
        ('e', EMPTY_LINE),
        ('g', SHOW_CURSOR),
        ('i', AUTOINDENT),
        ('j', JUMPY_SCROLLING),
        ('k', CUT_FROM_CURSOR),
        ('l', LINE_NUMBERS),
        ('m', USE_MOUSE),
        ('n', NOREAD_MODE),
        ('p', PRESERVE),
        ('q', INDICATOR),
        ('t', SAVE_ON_EXIT),
        ('u', MAKE_IT_UNIX),
        ('v', VIEW_MODE),
        ('x', NO_HELP),
        ('y', AFTER_ENDS),
        ('@', COLON_PARSING),
        ('%', STATEFLAGS),
        ('_', MINIBAR),
        ('0', ZERO),
        ('1', SOLO_SIDESCROLL),
        ('/', MODERN_BINDINGS),
    ];

    for &(option, flag) in cases {
        setup();
        parse_command_line(&args(&[&format!("-{}", option)]));
        assert!(ISSET(flag), "-{option} 应置位标志 {flag}");
    }
}

/// 9.2 与旧实现的短选项含义冲突：-b/-d/-K/-M/-O/-W/-X 等必须按 9.2 解释。
#[test]
fn conflicting_short_options_follow_nano_92() {
    setup();
    let parsed = parse_command_line(&args(&["-b"]));
    assert_eq!(parsed.hardwrap, 1, "-b 应请求 breaklonglines");
    assert!(
        !ISSET(BOLD_TEXT),
        "-b 在 9.2 不是 boldtext（boldtext 是 -D）"
    );
    assert!(
        !ISSET(BREAK_LONG_LINES),
        "BREAK_LONG_LINES 由 main 在 rc 之后应用"
    );

    setup();
    parse_command_line(&args(&["-D"]));
    assert!(ISSET(BOLD_TEXT), "-D 才是 boldtext");
    assert!(!ISSET(BREAK_LONG_LINES));

    setup();
    parse_command_line(&args(&["-d"]));
    assert!(ISSET(REBIND_DELETE), "-d 才是 rebinddelete");
    assert!(!ISSET(RAW_SEQUENCES), "-d 不应设置 rawsequences");

    setup();
    parse_command_line(&args(&["-K"]));
    assert!(ISSET(RAW_SEQUENCES), "-K 是 rawsequences");
    assert!(!ISSET(REBIND_DELETE));

    setup();
    parse_command_line(&args(&["-G"]));
    assert!(ISSET(LOCKING), "-G 是 locking");
    assert!(!ISSET(LET_THEM_ZAP), "-G 不应设置 zap");

    setup();
    parse_command_line(&args(&["-Z"]));
    assert!(ISSET(LET_THEM_ZAP), "-Z 是 zap");
    assert!(!ISSET(LOCKING));

    setup();
    parse_command_line(&args(&["-g"]));
    assert!(ISSET(SHOW_CURSOR), "-g 是 showcursor");
    assert!(!ISSET(POSITIONLOG));

    setup();
    parse_command_line(&args(&["-P"]));
    assert!(ISSET(POSITIONLOG), "-P 是 positionlog");
    assert!(!ISSET(SHOW_CURSOR));

    setup();
    parse_command_line(&args(&["-O"]));
    assert!(ISSET(BOOKSTYLE), "-O 是 bookstyle");
    assert!(!ISSET(COLON_PARSING), "-O 不是 colonparsing（是 -@）");

    setup();
    parse_command_line(&args(&["-@"]));
    assert!(ISSET(COLON_PARSING), "-@ 是 colonparsing");
    assert!(!ISSET(BOOKSTYLE));

    setup();
    parse_command_line(&args(&["-M"]));
    assert!(ISSET(TRIM_BLANKS), "-M 是 trimblanks");
    assert!(!ISSET(MODERN_BINDINGS), "modernbindings 是 -/");

    setup();
    parse_command_line(&args(&["-/"]));
    assert!(ISSET(MODERN_BINDINGS), "-/ 是 modernbindings");
    assert!(!ISSET(TRIM_BLANKS));

    setup();
    parse_command_line(&args(&["-W"]));
    assert!(ISSET(WORD_BOUNDS), "-W 是 wordbounds");
    assert!(!ISSET(WHITESPACE_DISPLAY), "whitespacedisplay 只有长选项");

    setup();
    parse_command_line(&args(&["-X", "abc"]));
    assert_eq!(
        with_global(|g| g.word_chars.clone()),
        Some("abc".to_string())
    );
    assert!(!ISSET(WORD_BOUNDS));
}

/// -w/-b 的三态：默认 -2，后出现的选项覆盖先出现的。
#[test]
fn hardwrap_three_state() {
    setup();
    assert_eq!(parse_command_line(&args(&[])).hardwrap, -2);
    assert_eq!(parse_command_line(&args(&["-b"])).hardwrap, 1);
    assert_eq!(parse_command_line(&args(&["-w"])).hardwrap, 0);
    assert_eq!(parse_command_line(&args(&["-b", "-w"])).hardwrap, 0);
    assert_eq!(parse_command_line(&args(&["-w", "-b"])).hardwrap, 1);
}

/// 短选项可以合并（-ES），带参选项可粘连（-T4）或分开（-T 4）。
#[test]
fn grouped_and_attached_short_options() {
    setup();
    parse_command_line(&args(&["-ES"]));
    assert!(ISSET(TABS_TO_SPACES));
    assert!(ISSET(SOFTWRAP));

    setup();
    parse_command_line(&args(&["-T4"]));
    assert_eq!(with_global(|g| g.tabsize), 4);

    setup();
    parse_command_line(&args(&["-T", "6"]));
    assert_eq!(with_global(|g| g.tabsize), 6);

    setup();
    parse_command_line(&args(&["-r", "40"]));
    assert_eq!(with_global(|g| g.fill), 40);

    setup();
    let parsed = parse_command_line(&args(&["-T", "4", "a.txt"]));
    assert_eq!(with_global(|g| g.tabsize), 4);
    assert_eq!(parsed.files.len(), 1, "选项参数 4 不应被当作文件名");
    assert_eq!(parsed.files[0].name, "a.txt");
}

/// 长选项：--opt=value、--opt value、唯一前缀缩写与 deprecated 别名。
#[test]
fn long_options_and_unique_prefix() {
    setup();
    parse_command_line(&args(&["--tabsize=5"]));
    assert_eq!(with_global(|g| g.tabsize), 5);

    setup();
    parse_command_line(&args(&["--tabsize", "7"]));
    assert_eq!(with_global(|g| g.tabsize), 7);

    setup();
    parse_command_line(&args(&["--tabstospac"]));
    assert!(
        ISSET(TABS_TO_SPACES),
        "--tabstospac 是 --tabstospaces 的唯一前缀"
    );

    setup();
    parse_command_line(&args(&["--newbuffer"]));
    assert!(ISSET(NEW_BUFFER));

    setup();
    parse_command_line(&args(&["--multibuffer"]));
    assert!(ISSET(NEW_BUFFER), "--multibuffer 是 9.2 保留的旧名");

    setup();
    parse_command_line(&args(&["--whitespacedisplay"]));
    assert!(ISSET(WHITESPACE_DISPLAY));

    setup();
    parse_command_line(&args(&["--syntax=rust"]));
    assert_eq!(
        with_global(|g| g.syntaxstr.clone()),
        Some("rust".to_string())
    );
}

/// -I 跳过 rc；-z 记录列出语法的请求，并保留遇到 -z 时的 -I 状态。
#[test]
fn ignorercfiles_and_listsyntaxes() {
    setup();
    assert!(parse_command_line(&args(&["-I"])).ignore_rcfiles);
    assert!(!parse_command_line(&args(&[])).ignore_rcfiles);

    setup();
    let parsed = parse_command_line(&args(&["-z"]));
    assert!(parsed.list_syntaxes);
    assert!(parsed.list_syntaxes_reads_rc, "-z 单独出现时应先读 rc");

    setup();
    let parsed = parse_command_line(&args(&["-I", "-z"]));
    assert!(parsed.list_syntaxes);
    assert!(!parsed.list_syntaxes_reads_rc, "-I -z 不应读 rc");

    setup();
    let parsed = parse_command_line(&args(&["-z", "-I"]));
    assert!(parsed.list_syntaxes);
    assert!(parsed.list_syntaxes_reads_rc, "-z -I 在 C 版里已先读过 rc");
}

/// `--` 之后不再解析选项；单独的 `-` 是 stdin 文件名。
#[test]
fn double_dash_and_stdin_operand() {
    setup();
    let parsed = parse_command_line(&args(&["--", "-l"]));
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].name, "-l");
    assert!(!ISSET(LINE_NUMBERS), "-- 之后的 -l 是文件名");

    setup();
    let files = parse_file_args(&args(&["-"]));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "-");

    setup();
    let parsed = parse_command_line(&args(&["file.txt", "-l"]));
    assert_eq!(parsed.files[0].name, "file.txt");
    assert!(
        ISSET(LINE_NUMBERS),
        "GNU getopt 会重排 argv，选项在文件名后仍生效"
    );
    assert_eq!(
        parse_args(&args(&["file.txt", "-l"])).as_deref(),
        Some("file.txt")
    );
}

/// 可执行文件层面：未知/歧义/缺参选项退出码 1，并提示 -h。
#[test]
fn binary_rejects_bad_options() {
    let exe = env!("CARGO_BIN_EXE_nax");

    let out = Command::new(exe)
        .arg("--definitely-not-an-option")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("-h"));

    let out = Command::new(exe).arg("--tab").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "歧义前缀应报错退出");

    let out = Command::new(exe).arg("-Q").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "缺少参数应报错退出");

    let out = Command::new(exe).arg("-2").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "未知短选项应报错退出");

    /* 没有 libmagic 时，-! 与 --magic 应和未编译 libmagic 的 nano 一样被拒绝。 */
    let out = Command::new(exe).arg("-!").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "无 libmagic 时不接受 -!");
    let out = Command::new(exe).arg("--magic").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "无 libmagic 时不接受 --magic");
}

/// -z 可执行路径：-I -z 不读 rc，只打印语法标题后正常退出。
#[test]
fn binary_listsyntaxes_exits_cleanly() {
    let exe = env!("CARGO_BIN_EXE_nax");
    let out = Command::new(exe)
        .args(["-I", "-z"])
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(!out.stdout.is_empty(), "-z 应输出一行标题");
}
