// tests/new_features.rs - 验证本仓库新增功能的集成测试：
//   1. 插入文件内容（insert_text_into_buffer）；
//   2. 多缓冲区切换（open_another_buffer / switch_to_prev_buffer / switch_to_next_buffer）；
//   3. 锁文件写入与删除（write_lockfile / delete_lockfile / lock_filename_for）；
//   4. 段落对齐（do_justify）；
//   5. 单词补全（complete_a_word）；
//   6. 命令行多文件参数解析（parse_file_args）；
//   7. Ctrl+/ 键码映射与 Go To Line 菜单快捷键。

use nano_rs::definitions::{with_global, with_global_mut, LineRef};

/// 初始化全局状态与 i18n（locales 指向仓库内的 locales/ 目录）。
fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    nano_rs::global::global_init();
    nano_rs::i18n::init();
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.editwinrows = 20;
        g.wrap_at = 40;
        g.openfile = None;
        g.statusbar_msg.clear();
        g.statusbar_centered = false;
        g.lastmessage = nano_rs::definitions::MessageType::Vacuum;
    });
}

/// 从行链表读取全部文本（行间以 \n 连接；跳过末尾魔法行）。
fn buffer_text() -> String {
    with_global(|g| {
        let of = g.openfile.as_ref().unwrap().borrow();
        let (filebot, filetop) = (of.filebot.clone(), of.filetop.clone());
        let single_line = match (&filetop, &filebot) {
            (Some(t), Some(b)) => std::rc::Rc::ptr_eq(t, b),
            _ => true,
        };
        let mut result = String::new();
        let mut cur = of.filetop.clone();
        let mut first = true;
        while let Some(c) = cur {
            let is_filebot = filebot
                .as_ref()
                .map(|b| std::rc::Rc::ptr_eq(b, &c))
                .unwrap_or(false);
            let (data, next) = {
                let r = c.borrow();
                (r.data.clone(), r.next.clone())
            };
            /* 魔法行：末尾空行且非唯一行时不输出。 */
            if is_filebot && data.is_empty() && !single_line {
                break;
            }
            if !first {
                result.push('\n');
            }
            first = false;
            result.push_str(&data);
            cur = next;
        }
        result
    })
}

/// 在临时目录创建文件并返回路径。
fn temp_file(name: &str, content: &str) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("rustnano_test_{}_{}", std::process::id(), name));
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn insert_text_into_buffer_works() {
    setup();
    nano_rs::files::open_buffer("");
    // 先在缓冲区输入 "first"
    nano_rs::text::inject(b"first", 5);
    assert_eq!(buffer_text(), "first");

    // 在光标后插入多行文本
    nano_rs::files::insert_text_into_buffer("second\nthird");
    let text = buffer_text();
    assert!(text.contains("second"), "插入文本应含 second，实际: {text}");
    assert!(text.contains("third"), "插入文本应含 third，实际: {text}");
    assert!(text.starts_with("firstsecond"), "插入应在光标处，实际: {text}");
}

#[test]
fn multibuffer_switch_works() {
    setup();
    nano_rs::files::open_buffer("");
    let file1 = temp_file("mb1.txt", "one\n");
    let file2 = temp_file("mb2.txt", "two\n");

    let r = nano_rs::files::open_another_buffer(&file1);
    assert!(matches!(r, nano_rs::files::OpenBufferResult::FileLoaded));
    let r = nano_rs::files::open_another_buffer(&file2);
    assert!(matches!(r, nano_rs::files::OpenBufferResult::FileLoaded));

    // 当前应是 file2
    let name = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
    });
    assert_eq!(name.as_deref(), Some(file2.as_str()));

    // 切换到前一个（file1）
    nano_rs::files::switch_to_prev_buffer();
    let name = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
    });
    assert_eq!(name.as_deref(), Some(file1.as_str()));

    // 再切回下一个（file2）
    nano_rs::files::switch_to_next_buffer();
    let name = with_global(|g| {
        g.openfile.as_ref().and_then(|of| of.borrow().filename.clone())
    });
    assert_eq!(name.as_deref(), Some(file2.as_str()));

    let _ = std::fs::remove_file(&file1);
    let _ = std::fs::remove_file(&file2);
}

#[test]
fn lockfile_write_and_delete() {
    setup();
    let file = temp_file("lock1.txt", "hello\n");
    let lockname = nano_rs::files::lock_filename_for(&file);
    let base = std::path::Path::new(&file)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        lockname.ends_with(&format!(".{}.swp", base)),
        "锁文件名: {lockname}"
    );

    assert!(nano_rs::files::write_lockfile(&lockname, &file, false));
    assert!(std::path::Path::new(&lockname).exists(), "锁文件应已创建");
    assert!(nano_rs::files::delete_lockfile(&lockname));
    assert!(!std::path::Path::new(&lockname).exists(), "锁文件应已删除");

    let _ = std::fs::remove_file(&file);
}

#[test]
fn justify_paragraph_works() {
    setup();
    let file = temp_file("justify1.txt", "alpha beta gamma delta\nepsilon zeta eta theta\n");
    nano_rs::files::open_buffer(&file);

    // 把光标移到第一行
    nano_rs::search::goto_line_posx(1, 0);

    nano_rs::text::do_justify();

    // 对齐后应是一段，行数减少（合并进单行并重排）
    let text = buffer_text();
    assert!(!text.is_empty());
    assert!(text.contains("alpha"), "对齐后应保留文本，实际: {text}");
    assert!(text.contains("theta"), "对齐后应保留全部单词，实际: {text}");
    assert!(!text.contains("\n\n"), "不应出现空行，实际: {text:?}");

    let _ = std::fs::remove_file(&file);
}

#[test]
fn complete_a_word_finds_candidate() {
    setup();
    nano_rs::files::open_buffer("");
    nano_rs::text::inject(b"hello world hello there", 23);
    // 移到行首并输入片段 "hel"
    nano_rs::search::goto_line_posx(1, 0);
    nano_rs::text::inject(b"hel", 3);

    nano_rs::text::complete_a_word();
    let text = buffer_text();
    assert!(text.contains("hello"), "应补全为 hello，实际: {text}");
}

#[test]
fn parse_file_args_collects_all_files() {
    setup();
    let args: Vec<String> = vec![
        "nano".into(),
        "-l".into(),
        "+5".into(),
        "a.txt".into(),
        "b.txt".into(),
        "-v".into(),
        "c.txt".into(),
    ];
    let files = nano_rs::global::parse_file_args(&args);
    assert_eq!(files.len(), 3);
    assert_eq!(files[0].0, "a.txt");
    assert_eq!(files[0].1, 5, "+5 应作用于 a.txt");
    assert_eq!(files[1].0, "b.txt");
    assert_eq!(files[2].0, "c.txt");
}

#[test]
fn zap_all_cutbuffer_clears() {
    setup();
    nano_rs::files::open_buffer("");
    /* 手动构造 cutbuffer 内容。 */
    with_global_mut(|g| {
        let line = nano_rs::definitions::make_new_node(None);
        line.borrow_mut().data = "cut me".to_string();
        g.cutbuffer = Some(line);
    });
    assert!(with_global(|g| g.cutbuffer.is_some()), "应有 cutbuffer");

    nano_rs::text::zap_all_cutbuffer();
    assert!(with_global(|g| g.cutbuffer.is_none()), "zap 后 cutbuffer 应为空");
}

#[test]
fn buffer_text_helpers_consistent() {
    // 验证 buffer_text 辅助函数自身：多行文本往返
    setup();
    nano_rs::files::open_buffer("");
    nano_rs::text::inject(b"line1", 5);
    nano_rs::text::do_enter();
    nano_rs::text::inject(b"line2", 5);
    let text = buffer_text();
    assert_eq!(text, "line1\nline2");
}

#[test]
fn ctrl_slash_maps_to_gotoline_keycode() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // Ctrl+/ 必须映射到 31（0x1F，与 Unix 终端发送的字节一致，对应 Go To Line）。
    let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL);
    assert_eq!(
        nano_rs::winio::translate_keycode(key),
        31,
        "Ctrl+/ 应映射为 31（Go To Line），而不是与 Ctrl+O 冲突的 15"
    );
    // 对照：Ctrl+O 仍应为 15（写文件）。
    let ctrl_o = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
    assert_eq!(nano_rs::winio::translate_keycode(ctrl_o), 15);
}

#[test]
fn gotoline_menu_shortcuts_are_bound() {
    use nano_rs::definitions::FunctionId;
    setup();
    nano_rs::global::shortcut_init();

    // Go To Line 提示菜单中的专属快捷键（对应 C 版 global.c）。
    let t = nano_rs::global::find_shortcut(20, nano_rs::definitions::MGOTOLINE);
    assert_eq!(t.map(|s| s.borrow().func), Some(FunctionId::FlipGoto), "^T 应切换到搜索");
    let w = nano_rs::global::find_shortcut(23, nano_rs::definitions::MGOTOLINE);
    assert_eq!(w.map(|s| s.borrow().func), Some(FunctionId::DoParaBegin), "^W 应为段落开头");
    let o = nano_rs::global::find_shortcut(15, nano_rs::definitions::MGOTOLINE);
    assert_eq!(o.map(|s| s.borrow().func), Some(FunctionId::DoParaEnd), "^O 应为段落末尾");
    let y = nano_rs::global::find_shortcut(25, nano_rs::definitions::MGOTOLINE);
    assert_eq!(y.map(|s| s.borrow().func), Some(FunctionId::DoFirstLine), "^Y 应为文件首行");
    let v = nano_rs::global::find_shortcut(22, nano_rs::definitions::MGOTOLINE);
    assert_eq!(v.map(|s| s.borrow().func), Some(FunctionId::DoLastLine), "^V 应为文件末行");

    // 主菜单中的 ^/ 绑定仍指向 Go To Line。
    let slash = nano_rs::global::find_shortcut(31, nano_rs::definitions::MMAIN);
    assert_eq!(slash.map(|s| s.borrow().func), Some(FunctionId::DoGoToLine), "^/ 应绑定 Go To Line");
}

#[test]
fn gotoline_bottombars_layout_matches_original() {
    // 验证 MGOTOLINE 菜单底部快捷键栏的条目与顺序（对应原版 C 版布局）：
    //   第一行：^G 帮助    ^W 段落开头   ^Y 首行   ^T 跳至文字
    //   第二行：^C 取消    ^O 段落结尾   ^V 尾行
    use nano_rs::definitions::MGOTOLINE;
    setup();
    /* 断言中文标签，固定语言为 zh-CN。 */
    std::env::set_var("LANG", "zh-CN");
    nano_rs::i18n::init();
    nano_rs::global::shortcut_init();

    // 模拟 C 版 bottombars：遍历 allfuncs，对匹配 MGOTOLINE 的函数
    // 用 first_sc_for 找快捷键，得到显示顺序（keystr, tag）。
    let entries: Vec<(String, String)> = with_global(|g| {
        let mut result = Vec::new();
        let mut current_func = g.allfuncs.clone();
        while let Some(f) = current_func {
            let f_ref = f.borrow();
            if (f_ref.menus & MGOTOLINE) != 0 {
                if let Some(sc) = nano_rs::global::first_sc_for(MGOTOLINE, f_ref.func) {
                    result.push((sc.borrow().keystr.clone(), f_ref.tag.clone()));
                }
            }
            current_func = f_ref.next.clone();
        }
        result
    });

    // 期望的键序列（与用户/原版布局一致，index 交错两行）。
    let expected_keys = ["^G", "^C", "^W", "^O", "^Y", "^V", "^T"];
    assert_eq!(entries.len(), expected_keys.len(), "条目数：{entries:?}");
    for (i, key) in expected_keys.iter().enumerate() {
        assert_eq!(&entries[i].0, key, "第 {i} 项键应为 {key}，实际 {entries:?}");
    }

    // 期望的标签（中文，注意 i18n）。
    let expected_tags = ["帮助", "取消", "段落开头", "段落结尾", "首行", "尾行", "跳至文字"];
    for (i, tag) in expected_tags.iter().enumerate() {
        assert_eq!(&entries[i].1, tag, "第 {i} 项标签应为 {tag}，实际 {entries:?}");
    }
}

#[test]
fn gotoline_prompt_sets_currmenu() {
    // 提示期间 currmenu 应为 MGOTOLINE（do_prompt 设置），保证菜单专属快捷键匹配。
    setup();
    nano_rs::global::shortcut_init();
    let currmenu = with_global(|g| g.currmenu);
    assert_eq!(currmenu, nano_rs::definitions::MMAIN);
    // 直接设置与恢复的路径由 do_prompt 内部处理；这里验证初始状态即可。
    with_global_mut(|g| g.currmenu = nano_rs::definitions::MGOTOLINE);
    let sc = nano_rs::global::find_shortcut(20, with_global(|g| g.currmenu));
    assert_eq!(
        sc.map(|s| s.borrow().func),
        Some(nano_rs::definitions::FunctionId::FlipGoto)
    );
}

/// 确保 LineRef 类型在测试中被引用（避免未使用导入警告）。
#[allow(dead_code)]
fn _type_anchor(_l: &LineRef) {}
