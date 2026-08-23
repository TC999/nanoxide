// main.rs - GNU nano Rust 翻译版入口（对应 C 的 nano.c 主函数）
//
// 程序初始化顺序与 nano.c 的 main() 一致：
// 全局状态 -> 命令行参数 -> 主目录 -> 终端 -> 颜色 -> rc 文件 ->
// 快捷键 -> 历史 -> 打开文件 -> 显示 -> 主事件循环 -> 退出清理。

use nanoxide::definitions::{
    with_global, with_global_mut, ISSET, SET, MessageType, CONSTANT_SHOW, MINIBAR, ZERO,
    RESTRICTED, NO_WRAP, BREAK_LONG_LINES,
    FOREIGN_SEQUENCE, THE_WINDOW_RESIZED,
};
use nanoxide::global::parse_args;
use nanoxide::winio::{handle_input_key, show_welcome_message, ERR};
use nanoxide::{color, files, global, history, rcfile, signals, text, utils, winio};

fn main() {
    // 1. 初始化全局状态
    global::global_init();

    // 1b. 初始化 i18n（按 LANG 协商语言，加载外置 ftl，放在 global_init 之后、
    //     首次使用 i18n 之前即可；不依赖 home dir，因为 locales/ 默认位于 exe 旁）
    nanoxide::i18n::init();

    // 2. 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    parse_args(&args);

    // 3. 获取用户主目录
    utils::get_homedir();

    // 4. 初始化终端
    winio::initscr();

    // 4b. 注册信号处理器（SIGHUP/SIGTERM 紧急保存、SIGTSTP 挂起、SIGWINCH 尺寸变化等）。
    signals::set_up_signal_handlers();

    // 5. 初始化颜色（原版顺序：do_rcfiles 之后 set_interface_colorpairs）
    color::start_color();

    // 5b. 初始化快捷键与函数列表（原版顺序：shortcut_init 在读取 rcfile 之前，
    //     以便 rcfile 能 rebind/unbind 键，且 check_vitals_mapped 依赖函数列表）。
    global::shortcut_init();

    // 5c. 备份命令行选项（对应 nano.c do_rcfiles 前的 *-cmdline 备份与清空）。
    with_global_mut(|g| {
        g.cmdline_flags = g.flags;
        g.backup_dir = None;
        g.word_chars = None;
        g.operating_dir = None;
        g.quotestr = None;
        g.speller = None;
    });

    // 6. 读取 rc 文件
    rcfile::do_rcfiles();

    // 6a. 恢复命令行选项（对应 nano.c do_rcfiles 后的恢复：命令行优先）。
    with_global_mut(|g| {
        if let Some(f) = g.cmdline_fill {
            g.fill = f;
        }
        if let Some(t) = g.cmdline_tabsize {
            g.tabsize = t;
        }
        if let Some(s) = g.cmdline_stripe_column {
            g.stripe_column = s;
        }
        if let Some(b) = g.cmdline_backup_dir.clone() {
            g.backup_dir = Some(b);
        }
        if let Some(w) = g.cmdline_word_chars.clone() {
            g.word_chars = Some(w);
        }
        if let Some(o) = g.cmdline_operating_dir.clone() {
            g.operating_dir = Some(o);
        } else if ISSET(RESTRICTED) {
            g.operating_dir = None;
        }
        if let Some(q) = g.cmdline_quotestr.clone() {
            g.quotestr = Some(q);
        }
        if let Some(s) = g.cmdline_speller.clone() {
            g.speller = Some(s);
        }
        /* 命令行 flags 与 rcfile flags 按位 OR：rcfile 不能取消命令行选项。 */
        g.flags.or_with(&g.cmdline_flags);
        /* 若 rcfile 未取消 nowrap，保持 breaklonglines。 */
        if !ISSET(NO_WRAP) {
            SET(BREAK_LONG_LINES);
        }
    });

    // 6b. 用 rcfile 中的 set <element>color 初始化界面颜色对
    color::set_interface_colorpairs();

    // 8. 加载历史记录
    history::history_init();
    history::load_history();

    // 9. 打开文件（支持多文件与 +LINE,COLUMN 定位，对应 C 版 main 的循环）
    let args: Vec<String> = std::env::args().collect();
    let files = global::parse_file_args(&args);
    let has_filename = !files.is_empty();
    for (idx, (name, line, col)) in files.iter().enumerate() {
        let result = if idx == 0 {
            files::open_buffer(name)
        } else {
            files::open_another_buffer(name)
        };
        match result {
            files::OpenBufferResult::NewFile => {
                // 文件不存在：在原来显示 welcome-message 的位置显示 "[ New File ]"
                winio::statusbar_centered(&format!("[ {} ]", nanoxide::t!("files-new_file")));
            }
            files::OpenBufferResult::Directory => {
                // 与原版 nano.c 一致：目录不加载（open_buffer 返回 FALSE 后
                // main 继续处理下一个文件），最终打开空白缓冲区让编辑器可用。
                // 状态栏已显示 "[ '目录' is a directory ]"（对应 statusline(ALERT)）。
            }
            files::OpenBufferResult::ErrorRead => {
                // 读取失败：创建空缓冲区保证编辑器可继续工作
            }
            files::OpenBufferResult::FileLoaded => {}
        }

        /* 命令行给出的位置：跳到对应行/列（对应 C 的 goto_line_and_column）。 */
        if *line != 0 || *col != 0 {
            nanoxide::search::goto_line_and_column(*line, *col, true);
        }
    }

    // 若没有成功打开任何缓冲区，打开空白缓冲区。
    if with_global(|g| g.openfile.is_none()) {
        files::open_buffer("");
    }

    // 多文件时切回第一个缓冲区（对应 C：openfile = openfile->next）。
    if files.len() > 1 {
        files::switch_to_prev_buffer();
    }

    // 10. 准备显示
    files::prepare_for_display();

    // 10.5 仅在不带文件名时显示欢迎消息（对应 nano.c 的 statusbar 欢迎提示）。
    // 已带文件名且判定为新文件 / 目录时，状态栏信息已在 open_buffer 分支中输出，
    // 避免欢迎消息覆盖。
    if !has_filename {
        show_welcome_message();
    }

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

    // 14b. 输出 rcfile 解析中累积的错误（对应 display_rcfile_errors）
    rcfile::print_errors();
}

/// 主事件循环。
pub fn main_loop() {
    with_global_mut(|g| g.we_are_running = true);

    while with_global(|g| g.we_are_running) {
        // 收到 SIGHUP/SIGTERM：紧急保存所有缓冲区并退出
        // （对应 C 版 handle_hupterm 的 die + emergency_save_all）。
        if signals::terminate_requested() {
            files::emergency_save_all();
            /* 对应 C 版 die()：遍历删除所有缓冲区的锁文件。 */
            files::delete_all_lockfiles();
            with_global_mut(|g| g.we_are_running = false);
            break;
        }

        // 收到 SIGWINCH：重新查询尺寸并重绘（对应 regenerate_screen）。
        if signals::take_window_resized() {
            signals::regenerate_screen();
        }

        // 收到 SIGTSTP：请求挂起（对应 suspend_nano/continue_nano）。
        if signals::take_suspend_requested() {
            text::do_suspend();
        }
        if signals::take_resumed() {
            winio::edit_refresh();
        }

        // 确认行号边距（对应 C 主循环的 confirm_margin()）。
        winio::confirm_margin();

        // MINIBAR 模式：无重要消息时刷新极简状态栏
        // （对应 C 主循环的 minibar() 条件：lastmessage < REMARK）。
        let quiet = matches!(
            with_global(|g| g.lastmessage),
            MessageType::Vacuum | MessageType::Hush
        );
        if ISSET(MINIBAR) && !ISSET(ZERO) && with_global(|g| g.LINES) > 1 && quiet {
            winio::minibar();
        } else if ISSET(CONSTANT_SHOW) && with_global(|g| g.LINES) > 1 && !ISSET(ZERO)
            && quiet && winio::waiting_keycodes() == 0
        {
            // CONSTANT_SHOW：无消息且无待处理按键时报告光标位置
            // （对应 C 主循环的 report_cursor_position() 条件）。
            nanoxide::global::report_cursor_position();
        }

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

        // 查找快捷键并执行（或作为普通字符输入）
        handle_input_key(key);
    }
}

/// 处理 Ctrl+C 中断。
#[allow(dead_code)]
fn handle_sigint() {
    with_global_mut(|g| g.control_C_was_pressed = true);
}
