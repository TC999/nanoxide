//! Linter 输出解析与 linter 模式绑定。
//!
//! 对应 text.c 的 do_linter：识别 "文件名:行[:列]: 消息"，
//! 列也可以写成 ",列"。

use nanoxide::definitions::*;

#[test]
fn parses_unix_and_windows_paths() {
    let messages = nanoxide::text::parse_linter_output(
        "file.c:12:3: syntax error\n\
         file.c:12: missing semicolon\n\
         C:\\src\\file.c:7:2: unknown type\n\
         C:\\src\\file.c:9: bad token\n",
    );

    assert_eq!(messages.len(), 4, "{messages:?}");
    assert_eq!(messages[0].filename, "file.c");
    assert_eq!((messages[0].line, messages[0].column), (12, 3));
    assert_eq!(messages[0].message, "syntax error");

    assert_eq!((messages[1].line, messages[1].column), (12, 1));
    assert_eq!(messages[1].message, "missing semicolon");

    assert_eq!(messages[2].filename, "C:\\src\\file.c");
    assert_eq!((messages[2].line, messages[2].column), (7, 2));

    assert_eq!(messages[3].filename, "C:\\src\\file.c");
    assert_eq!((messages[3].line, messages[3].column), (9, 1));
}

#[test]
fn parses_comma_column_and_ignores_unparsable_lines() {
    let messages = nanoxide::text::parse_linter_output(
        "/tmp/f.c:7,4: comma column\n\
         just some noise\n\
         f.c:0:1: zero line\n\
         f.c:3:2: expected ':' here\n",
    );

    assert_eq!(messages.len(), 2, "{messages:?}");
    assert_eq!((messages[0].line, messages[0].column), (7, 4));
    assert_eq!(messages[1].message, "expected ':' here");
}

#[test]
fn linter_mode_keys_are_bound() {
    nanoxide::global::global_init();
    nanoxide::global::shortcut_init();

    let exec = nanoxide::global::find_shortcut(25, MEXECUTE)
        .map(|sc| sc.borrow().func);
    assert_eq!(exec, Some(FunctionId::DoLinter), "执行菜单 ^Y 调用 linter");

    let cancel = nanoxide::global::find_shortcut(24, MLINTER)
        .map(|sc| sc.borrow().func);
    assert_eq!(cancel, Some(FunctionId::DoCancel), "linter 模式 ^X 退出");

    let prev = nanoxide::global::find_shortcut(25, MLINTER)
        .map(|sc| sc.borrow().func);
    assert_eq!(prev, Some(FunctionId::DoPageUp), "上一条消息");
    let next = nanoxide::global::find_shortcut(22, MLINTER)
        .map(|sc| sc.borrow().func);
    assert_eq!(next, Some(FunctionId::DoPageDown), "下一条消息");
}

/// linter 命令按 C 的 `construct_argument_list` 拆分：按空格断开、
/// 文件名追加为最后一个参数，不替换 `%s`、不经过 shell。
#[test]
fn linter_arguments_follow_construct_argument_list() {
    let (program, args) = nanoxide::text::build_linter_arguments("pyflakes", "a.py");
    assert_eq!(program, "pyflakes");
    assert_eq!(args, vec!["a.py"]);

    /* 连续空格被跳过（C 用 strtok）。 */
    let (program, args) = nanoxide::text::build_linter_arguments(
        "python  -m pylint   --output-format=text",
        "dir/a.py",
    );
    assert_eq!(program, "python");
    assert_eq!(args, vec!["-m", "pylint", "--output-format=text", "dir/a.py"]);

    /* C 不认识 %s：它作为普通参数保留，文件名仍追加在最后。 */
    let (program, args) = nanoxide::text::build_linter_arguments("lint --file=%s", "a.py");
    assert_eq!(program, "lint");
    assert_eq!(args, vec!["--file=%s", "a.py"]);
}
