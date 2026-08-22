/**************************************************************************
 * signals.rs  --  GNU nano 信号处理（对应 nano.c 的信号部分）
 * 版权 (C) 1999-2026 Free Software Foundation, Inc.
 *
 * 转换说明：
 * - C 版在信号处理函数中直接做恢复终端、保存文件等操作；Rust 版出于
 *   async-signal-safety 考虑，处理函数只设置原子标志，由主循环消费。
 * - Windows 平台没有 POSIX 信号（SIGWINCH/SIGTSTP 等），crossterm 已
 *   通过事件机制处理窗口尺寸变化；此模块在这些平台提供空实现。
 **************************************************************************/

//! 信号处理：SIGINT/SIGQUIT 忽略，SIGHUP/SIGTERM 触发退出前的紧急保存，
//! SIGTSTP/SIGCONT 处理挂起与恢复，SIGWINCH 标记窗口尺寸变化。

use std::sync::atomic::{AtomicBool, Ordering};

/// 收到 SIGHUP 或 SIGTERM：请求优雅退出（先紧急保存）。
static TERMINATE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// 收到 SIGWINCH：窗口尺寸已变化，需要重新查询终端尺寸并重绘。
static WINDOW_RESIZED: AtomicBool = AtomicBool::new(false);

/// 收到 SIGTSTP：请求挂起（由主循环执行恢复终端 + SIGSTOP）。
static SUSPEND_REQUESTED: AtomicBool = AtomicBool::new(false);

/// 收到 SIGCONT：从挂起中恢复，需要重绘。
static RESUMED: AtomicBool = AtomicBool::new(false);

/// 是否请求了优雅退出（SIGHUP/SIGTERM）。
pub fn terminate_requested() -> bool {
    TERMINATE_REQUESTED.load(Ordering::SeqCst)
}

/// 消费"窗口已变化"标志。
pub fn take_window_resized() -> bool {
    WINDOW_RESIZED.swap(false, Ordering::SeqCst)
}

/// 消费"请求挂起"标志。
pub fn take_suspend_requested() -> bool {
    SUSPEND_REQUESTED.swap(false, Ordering::SeqCst)
}

/// 消费"已从挂起恢复"标志。
pub fn take_resumed() -> bool {
    RESUMED.swap(false, Ordering::SeqCst)
}

#[cfg(unix)]
extern "C" fn handle_terminate(_signal: libc::c_int) {
    TERMINATE_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_sigwinch(_signal: libc::c_int) {
    WINDOW_RESIZED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_sigstop(_signal: libc::c_int) {
    SUSPEND_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_sigcont(_signal: libc::c_int) {
    RESUMED.store(true, Ordering::SeqCst);
}

/// 注册 SIGWINCH 处理器（对应 `set_up_sigwinch_handler`）。
#[cfg(unix)]
pub fn set_up_sigwinch_handler() {
    unsafe {
        libc::signal(libc::SIGWINCH, handle_sigwinch as libc::sighandler_t);
    }
}

/// 注册全部信号处理器（对应 `set_up_signal_handlers`）：
/// - SIGINT/SIGQUIT：忽略；
/// - SIGHUP/SIGTERM：请求退出并紧急保存；
/// - SIGTSTP：请求挂起；SIGCONT：标记恢复；
/// - SIGWINCH：标记窗口尺寸变化。
#[cfg(unix)]
pub fn set_up_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGHUP, handle_terminate as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handle_terminate as libc::sighandler_t);
        libc::signal(libc::SIGTSTP, handle_sigstop as libc::sighandler_t);
        libc::signal(libc::SIGCONT, handle_sigcont as libc::sighandler_t);
    }
    set_up_sigwinch_handler();
}

/// 阻塞或解除阻塞 SIGWINCH（对应 `block_sigwinch`）。
/// 用于读写大文件期间避免窗口尺寸变化打断操作。
#[cfg(unix)]
pub fn block_sigwinch(blockit: bool) {
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGWINCH);
        let how = if blockit { libc::SIG_BLOCK } else { libc::SIG_UNBLOCK };
        libc::sigprocmask(how, &set, std::ptr::null_mut());
    }
}

/// 非 Unix 平台：无 POSIX 信号，空实现。
#[cfg(not(unix))]
pub fn set_up_sigwinch_handler() {}

/// 非 Unix 平台：无 POSIX 信号，空实现（窗口尺寸由 crossterm 事件处理）。
#[cfg(not(unix))]
pub fn set_up_signal_handlers() {}

/// 非 Unix 平台：空实现。
#[cfg(not(unix))]
pub fn block_sigwinch(_blockit: bool) {}

/// 重新初始化并完全重绘屏幕（对应 `regenerate_screen`）：
/// 重新查询终端尺寸、更新窗口变量并重绘。
pub fn regenerate_screen() {
    WINDOW_RESIZED.store(false, Ordering::SeqCst);
    crate::winio::update_screen_size();
    crate::winio::full_refresh();
}

/// 安装 Ctrl+C 处理（对应 `install_handler_for_Ctrl_C`）：
/// 读取文件等长操作期间，把 ^C 标记为"应中止"。
#[cfg(unix)]
pub fn install_handler_for_Ctrl_C() {
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_terminate as libc::sighandler_t,
        );
    }
}

/// 恢复 Ctrl+C 为忽略（对应 `restore_handler_for_Ctrl_C`）。
#[cfg(unix)]
pub fn restore_handler_for_Ctrl_C() {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }
}

/// 非 Unix 平台：空实现。
#[cfg(not(unix))]
pub fn install_handler_for_Ctrl_C() {}

/// 非 Unix 平台：空实现。
#[cfg(not(unix))]
pub fn restore_handler_for_Ctrl_C() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_default_to_false() {
        assert!(!terminate_requested());
        assert!(!take_window_resized());
        assert!(!take_suspend_requested());
        assert!(!take_resumed());
    }
}
