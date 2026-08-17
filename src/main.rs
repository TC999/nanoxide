// main.rs - GNU nano Rust 翻译版入口（对应 C 的 nano.c 主函数）
//
// 程序初始化顺序与 nano.c 的 main() 一致：
// 全局状态 -> 命令行参数 -> 主目录 -> 终端 -> 颜色 -> rc 文件 ->
// 快捷键 -> 历史 -> 打开文件 -> 显示 -> 主事件循环 -> 退出清理。

use nano_rs::definitions::{with_global, with_global_mut, FOREIGN_SEQUENCE, THE_WINDOW_RESIZED};
use nano_rs::global::parse_args;
use nano_rs::winio::{handle_input_key, show_welcome_message, ERR};
use nano_rs::{color, files, global, history, rcfile, utils, winio};

fn main() {
    // 1. 初始化全局状态
    global::global_init();

    // 1b. 初始化 i18n（按 LANG 协商语言，加载外置 ftl，放在 global_init 之后、
    //     首次使用 i18n 之前即可；不依赖 home dir，因为 locales/ 默认位于 exe 旁）
    nano_rs::i18n::init();

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
    let filename = parse_args(&args);
    match filename {
        Some(f) => {
            let opened = files::open_buffer(&f);
            if !opened {
                // 读取失败时也创建空缓冲区，保证编辑器始终有可编辑目标
                files::open_buffer("");
            }
        }
        None => {
            // 创建空缓冲区
            files::open_buffer("");
        }
    }

    // 10. 准备显示
    files::prepare_for_display();

    // 10.5 不带文件名且缓冲区为空时显示欢迎消息（对应 nano.c 的 statusbar 欢迎提示）
    show_welcome_message();

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

/// 主事件循环。
pub fn main_loop() {
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

        // 查找快捷键并执行（或作为普通字符输入）
        handle_input_key(key);
    }
}

/// 处理 Ctrl+C 中断。
#[allow(dead_code)]
fn handle_sigint() {
    with_global_mut(|g| g.control_C_was_pressed = true);
}
