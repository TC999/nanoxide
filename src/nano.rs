/**************************************************************************
 * nano.rs  --  GNU nano 主逻辑（对应 nano.c）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 **************************************************************************/

//! 程序入口、命令行参数解析、终端初始化与编辑器主循环。
//! 转换说明：使用 `with_global` 安全访问全局状态。

use crate::definitions::*;
use crate::global;
use crate::files;
use crate::history;
use crate::search;
use crate::text;
use crate::winio;
use crate::utils;
use crate::winio::ERR;
use crate::movement;
use crate::cut;
use crate::rcfile;
use crate::color;
use crate::help;

/// 主函数入口。
pub fn main() {
    // 1. 初始化全局状态
    global::global_init();

    // 2. 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    parse_args(&args);

    // 3. 获取用户主目录
    utils::get_homedir();

    // 4. 初始化终端
    winio::initscr();

    // 5. 初始化颜色
    color::set_interface_colorpairs();

    // 6. 读取 rc 文件
    rcfile::do_rcfiles();

    // 7. 初始化快捷键
    global::shortcut_init();

    // 8. 加载历史记录
    history::history_init();
    history::load_history();

    // 9. 打开文件
    let filename = args.get(1).map(|s| s.as_str()).unwrap_or("");
    if !filename.is_empty() {
        files::open_buffer(filename);
    } else {
        // 创建空缓冲区
        files::open_buffer("");
    }

    // 10. 准备显示
    files::prepare_for_display();

    // 11. 标记正在运行
    with_global_mut(|g| g.we_are_running = true);

    // 12. 首次刷新
    winio::edit_refresh();

    // 13. 主事件循环
    main_loop();

    // 14. 退出清理
    history::save_history();
    winio::terminal_restore();
    with_global_mut(|g| g.we_are_running = false);
}

/// 解析命令行参数。
fn parse_args(args: &[String]) {
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') {
            match arg.as_str() {
                "-V" | "--version" => {
                    print_version();
                    std::process::exit(0);
                }
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                "-v" | "--view" => SET(VIEW_MODE),
                "-x" | "--nohelp" => SET(NO_HELP),
                "-S" | "--softwrap" => SET(SOFTWRAP),
                "-m" | "--mouse" => SET(USE_MOUSE),
                "-i" | "--autoindent" => SET(AUTOINDENT),
                "-k" | "--cutfromcursor" => SET(CUT_FROM_CURSOR),
                "-l" | "--linenumbers" => SET(LINE_NUMBERS),
                "-b" | "--boldtext" => SET(BOLD_TEXT),
                "-u" | "--unix" => SET(MAKE_IT_UNIX),
                "-w" | "--nowrap" => SET(NO_WRAP),
                "-c" | "--constantshow" => SET(CONSTANT_SHOW),
                "-p" | "--preserve" => SET(PRESERVE),
                "-A" | "--smarthome" => SET(SMART_HOME),
                "-E" | "--tabstospaces" => SET(TABS_TO_SPACES),
                "-Q" | "--quotestr" => {
                    i += 1;
                    if i < args.len() {
                        // 设置引用字符串模式
                    }
                }
                "-r" | "--fill" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(f) = args[i].parse::<isize>() {
                            with_global_mut(|g| g.fill = f);
                        }
                    }
                }
                "-T" | "--tabsize" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(s) = args[i].parse::<usize>() {
                            with_global_mut(|g| g.tabsize = s);
                        }
                    }
                }
                "-R" | "--restricted" => SET(RESTRICTED),
                "-o" | "--operatingdir" => {
                    i += 1;
                    // 设置操作目录
                }
                "-f" | "--rcfile" => {
                    i += 1;
                    // 指定 rc 文件
                }
                "-K" | "--rebinddelete" => SET(REBIND_DELETE),
                "-s" | "--speller" => {
                    i += 1;
                    // 设置拼写检查器
                }
                "-Y" | "--syntax" => {
                    i += 1;
                    // 设置语法
                }
                "-g" | "--positionlog" => SET(POSITIONLOG),
                "-Z" | "--locking" => SET(LOCKING),
                "-U" | "--quickblank" => SET(QUICK_BLANK),
                "-j" | "--jumpyscrolling" => SET(JUMPY_SCROLLING),
                "-e" | "--emptyline" => SET(EMPTY_LINE),
                "-J" | "--guidestripe" => {
                    i += 1;
                    // 设置引导线
                }
                "-t" | "--saveonexit" => SET(SAVE_ON_EXIT),
                "-0" | "--zero" => SET(ZERO),
                "-M" | "--modernbindings" => SET(MODERN_BINDINGS),
                "-H" | "--historylog" => SET(HISTORYLOG),
                "-B" | "--backup" => SET(MAKE_BACKUP),
                "-C" | "--backupdir" => {
                    i += 1;
                    // 设置备份目录
                }
                "-I" | "--insecurebackup" => SET(INSECURE_BACKUP),
                "-N" | "--noconvert" => SET(NO_CONVERT),
                "-L" | "--nonewlines" => SET(NO_NEWLINES),
                "-X" | "--wordbounds" => SET(WORD_BOUNDS),
                "-W" | "--whitespacedisplay" => SET(WHITESPACE_DISPLAY),
                "-O" | "--colonparsing" => SET(COLON_PARSING),
                "-F" | "--multibuffer" => SET(NEW_BUFFER),
                _ => {
                    if arg == "--" {
                        break;
                    }
                }
            }
        } else {
            // 文件名参数
            break;
        }
        i += 1;
    }
}

/// 打印版本信息。
fn print_version() {
    println!("nano-rs version {}", VERSION);
    println!("(Rust translation of GNU nano)");
    println!("Compiled options: --enable-utf8");
}

/// 打印使用说明。
fn print_usage() {
    println!("Usage: nano [OPTIONS] [FILE]");
    println!("");
    println!("GNU nano - a small, friendly text editor");
    println!("");
    println!("Basic options:");
    println!("  -V, --version          Print version information");
    println!("  -h, --help             Print this help message");
    println!("  -v, --view             View mode (read-only)");
    println!("  -x, --nohelp           Hide the help lines");
    println!("  -S, --softwrap         Soft wrap lines");
    println!("  -m, --mouse            Enable mouse");
    println!("  -i, --autoindent       Auto-indent new lines");
    println!("  -k, --cutfromcursor     Cut from cursor to end of line");
    println!("  -l, --linenumbers      Show line numbers");
    println!("  -b, --boldtext         Use bold text");
    println!("  -u, --unix             Save in Unix format");
    println!("  -w, --nowrap           Don't wrap long lines");
    println!("  -c, --constantshow     Constantly show cursor position");
    println!("  -p, --preserve         Preserve XON/XOFF");
    println!("  -A, --smarthome        Smart home key");
    println!("  -E, --tabstospaces     Convert typed tabs to spaces");
    println!("  -T, --tabsize=N        Tab size (default 8)");
    println!("  -r, --fill=N           Target width for wrap (default -2)");
}

/// 主事件循环。
fn main_loop() {
    with_global_mut(|g| g.we_are_running = true);

    while with_global(|g| g.we_are_running) {
        // 检查是否需要刷新
        if with_global(|g| g.refresh_needed) {
            with_global_mut(|g| g.refresh_needed = false);
            winio::edit_refresh();
        }

        // 获取按键
        let key = winio::wgetch();

        // 处理特殊键码
        if key == THE_WINDOW_RESIZED {
            winio::update_screen_size();
            winio::edit_refresh();
            continue;
        }

        if key == ERR || key == FOREIGN_SEQUENCE {
            continue;
        }

        // 查找快捷键并执行
        let menu = with_global(|g| g.currmenu);
        let handled = execute_function(key, menu);

        if !handled {
            // 处理普通字符输入
            if key > 0 && key < 256 && key != ESC_CODE as i32 {
                let ch = char::from_u32(key as u32);
                if let Some(c) = ch {
                    if !with_global(|g| g.flags.isset(VIEW_MODE)) {
                        text::insert_char(c);
                        winio::edit_refresh();
                    }
                }
            }
        }
    }
}

/// 根据键码执行对应函数。
fn execute_function(key: i32, _menu: i32) -> bool {
    // 使用 if/else 链替代 match，避免表达式模式的问题
    if key == 1 { movement::do_home(); winio::edit_refresh(); return true; }           // Ctrl+A
    if key == 2 { movement::do_left(); winio::edit_refresh(); return true; }           // Ctrl+B
    if key == 3 { text::do_cancel(); return true; }                                    // Ctrl+C
    if key == 4 { cut::do_delete(); winio::edit_refresh(); return true; }              // Ctrl+D
    if key == 5 { movement::do_end(); winio::edit_refresh(); return true; }            // Ctrl+E
    if key == 6 { movement::do_right(); winio::edit_refresh(); return true; }          // Ctrl+F
    if key == 7 { help::do_help(); return true; }                                      // Ctrl+G
    if key == 8 { cut::do_backspace(); winio::edit_refresh(); return true; }           // Ctrl+H
    if key == 9 { text::do_tab(); winio::edit_refresh(); return true; }                // Ctrl+I (Tab)
    if key == 10 { return true; }                                                       // Ctrl+J
    if key == 11 { cut::do_cut(); winio::edit_refresh(); return true; }                // Ctrl+K
    if key == 12 { text::do_refresh(); winio::edit_refresh(); return true; }           // Ctrl+L
    if key == 13 { text::do_enter(); winio::edit_refresh(); return true; }             // Ctrl+M (Enter)
    if key == 14 { movement::do_down(); winio::edit_refresh(); return true; }          // Ctrl+N
    if key == 15 { files::do_writeout(); winio::edit_refresh(); return true; }         // Ctrl+O
    if key == 16 { movement::do_up(); winio::edit_refresh(); return true; }            // Ctrl+P
    if key == 17 { text::do_refresh(); return true; }                                  // Ctrl+Q
    if key == 18 { files::do_insertfile(); winio::edit_refresh(); return true; }       // Ctrl+R
    if key == 19 { text::do_suspend(); return true; }                                  // Ctrl+S
    if key == 20 { text::do_spell(); return true; }                                    // Ctrl+T
    if key == 21 { cut::do_paste(); winio::edit_refresh(); return true; }              // Ctrl+U
    if key == 22 { movement::do_page_down(); winio::edit_refresh(); return true; }     // Ctrl+V
    if key == 23 { search::do_search_forward(); winio::edit_refresh(); return true; }  // Ctrl+W
    if key == 24 {                                                                     // Ctrl+X
        if with_global(|g| g.inhelp) { /* 退出帮助 */ }
        text::do_exit();
        return true;
    }
    if key == 25 { movement::do_page_up(); winio::edit_refresh(); return true; }       // Ctrl+Y
    if key == 26 { text::do_undo(); winio::edit_refresh(); return true; }              // Ctrl+Z (Undo)

    // 功能键
    if key == KEY_F0 + 1 { help::do_help(); return true; }                             // F1
    if key == KEY_F0 + 2 { text::do_exit(); return true; }                             // F2
    if key == KEY_F0 + 3 { files::do_writeout(); return true; }                        // F3
    if key == KEY_F0 + 4 { search::do_search_forward(); return true; }                 // F4
    if key == KEY_F0 + 5 { text::do_refresh(); return true; }                          // F5
    if key == KEY_F0 + 6 { text::do_spell(); return true; }                            // F6
    if key == KEY_F0 + 7 { return true; }                                              // F7
    if key == KEY_F0 + 8 { return true; }                                              // F8
    if key == KEY_F0 + 9 { cut::do_cut(); winio::edit_refresh(); return true; }        // F9
    if key == KEY_F0 + 10 { cut::do_paste(); winio::edit_refresh(); return true; }     // F10
    if key == KEY_F0 + 11 { return true; }                                             // F11
    if key == KEY_F0 + 12 { return true; }                                             // F12

    // 方向键
    if key == KEY_LEFT { movement::do_left(); winio::edit_refresh(); return true; }
    if key == KEY_RIGHT { movement::do_right(); winio::edit_refresh(); return true; }
    if key == KEY_UP { movement::do_up(); winio::edit_refresh(); return true; }
    if key == KEY_DOWN { movement::do_down(); winio::edit_refresh(); return true; }
    if key == KEY_HOME { movement::do_home(); winio::edit_refresh(); return true; }
    if key == KEY_END { movement::do_end(); winio::edit_refresh(); return true; }
    if key == KEY_PPAGE { movement::do_page_up(); winio::edit_refresh(); return true; }
    if key == KEY_NPAGE { movement::do_page_down(); winio::edit_refresh(); return true; }
    if key == KEY_DC { cut::do_delete(); winio::edit_refresh(); return true; }
    if key == KEY_BACKSPACE { cut::do_backspace(); winio::edit_refresh(); return true; }
    if key == KEY_ENTER { text::do_enter(); winio::edit_refresh(); return true; }
    if key == 9 || key == KEY_BTAB { text::do_tab(); winio::edit_refresh(); return true; }

    // 修饰键
    if key == CONTROL_LEFT { movement::do_prev_word(); winio::edit_refresh(); return true; }
    if key == CONTROL_RIGHT { movement::do_next_word(); winio::edit_refresh(); return true; }
    if key == CONTROL_HOME { movement::do_first_line(); winio::edit_refresh(); return true; }
    if key == CONTROL_END { movement::do_last_line(); winio::edit_refresh(); return true; }
    if key == CONTROL_DELETE { cut::do_delete(); winio::edit_refresh(); return true; }
    if key == CONTROL_UP { movement::do_scroll_up(); winio::edit_refresh(); return true; }
    if key == CONTROL_DOWN { movement::do_scroll_down(); winio::edit_refresh(); return true; }

    // Alt 组合
    if key == ALT_LEFT { movement::do_prev_word(); winio::edit_refresh(); return true; }
    if key == ALT_RIGHT { movement::do_next_word(); winio::edit_refresh(); return true; }
    if key == ALT_UP { movement::to_para_begin(); winio::edit_refresh(); return true; }
    if key == ALT_DOWN { movement::to_para_end(); winio::edit_refresh(); return true; }
    if key == ALT_HOME { movement::do_first_line(); winio::edit_refresh(); return true; }
    if key == ALT_END { movement::do_last_line(); winio::edit_refresh(); return true; }
    if key == ALT_PAGEUP { movement::to_prev_block(); winio::edit_refresh(); return true; }
    if key == ALT_PAGEDOWN { movement::to_next_block(); winio::edit_refresh(); return true; }
    if key == ALT_INSERT { text::do_mark(); winio::edit_refresh(); return true; }

    // 其他
    if key == KEY_IC { text::do_mark(); winio::edit_refresh(); return true; }
    if key == KEY_SUSPEND { text::do_suspend(); return true; }
    if key == ESC_CODE as i32 { return true; } // 忽略单独的 Esc

    false
}

/// 处理 Ctrl+C 中断。
fn handle_sigint() {
    with_global_mut(|g| g.control_C_was_pressed = true);
}

/// 紧急保存所有缓冲区。
pub fn emergency_save_all() {
    let openfiles: Vec<OpenFileRef> = {
        let mut result = Vec::new();
        let mut current = with_global(|g| g.openfile.clone());
        loop {
            let next = match current {
                Some(ref ofile) => {
                    result.push(ofile.clone());
                    ofile.borrow().next.clone()
                }
                None => break,
            };
            current = next;
        }
        result
    };
    for openfile in openfiles {
        let filename = openfile.borrow().filename.clone().unwrap_or_default();
        let plainname = if filename.is_empty() {
            format!("nano.{}", std::process::id())
        } else {
            filename.clone()
        };
        let targetname = files::get_next_filename(&plainname, ".save");
        if !targetname.is_empty() {
            files::write_it_out(true, false);
        }
    }
}