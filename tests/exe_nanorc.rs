// tests/exe_nanorc.rs - 程序同目录 nanorc 的读取规则（nanoxide 扩展）
//
// 通过复制构建出的 nax 可执行文件到临时目录、在它旁边放/不放 `nanorc`，
// 再以 `-z` 运行副本，检查语法列表是否包含 rc 中定义的语法。
// 覆盖：程序目录 nanorc 被读取；与用户级 ~/.nanorc 同时生效（不短路）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// 每个测试独占的临时目录；Drop 时递归删除。
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 测试临时目录的根：优先用 Cargo 为集成测试提供的 `CARGO_TARGET_TMPDIR`
/// （位于 target/ 下，不污染源码树），直接运行测试可执行文件时回退系统临时目录。
fn temp_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_TMPDIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("tmp");
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/tmp")
}

/// 创建唯一临时目录（进程号 + 纳秒 + 计数器）。
fn temp_dir(tag: &str) -> TempDir {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = temp_root().join(format!(
        "nanoxide_exe_rc_{tag}_{}_{nanos}_{n}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create_dir_all({:?}) 失败：{e}", dir));
    TempDir(dir)
}

/// 复制 nax 可执行文件到 dir，返回副本路径。
fn copy_exe_to(dir: &Path) -> PathBuf {
    let src = PathBuf::from(env!("CARGO_BIN_EXE_nax"));
    let dst = dir.join(src.file_name().expect("nax 应有文件名"));
    fs::copy(&src, &dst).expect("应能复制 nax 可执行文件");
    dst
}

/// 以 `-z` 运行副本，返回 stdout+stderr 合并文本。
/// HOME 指向空的 home 目录并清掉 XDG_CONFIG_HOME，避免读到开发机上的用户级 rc。
fn run_list_syntaxes(exe: &Path, home: &Path) -> String {
    let out = Command::new(exe)
        .arg("-z")
        .env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .expect("应能运行 nax -z");
    assert!(out.status.success(), "nax -z 应成功退出：{:?}", out.status);
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text
}

fn prepare() -> (TempDir, PathBuf, PathBuf) {
    let base = temp_dir("case");
    let bin_dir = base.0.join("bin");
    let home_dir = base.0.join("home");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&home_dir).unwrap();
    (base, bin_dir, home_dir)
}

/// 程序目录的 nanorc 应被读取（对应 rcfile.rs 的 exe_dir_nanorc）。
#[test]
fn exe_directory_nanorc_is_read() {
    let (_base, bin_dir, home_dir) = prepare();
    fs::write(
        bin_dir.join("nanorc"),
        "syntax \"EXE_RC_TEST\" \"^exerc$\"\n",
    )
    .unwrap();

    let exe = copy_exe_to(&bin_dir);
    let text = run_list_syntaxes(&exe, &home_dir);
    assert!(
        text.contains("EXE_RC_TEST"),
        "程序同目录的 nanorc 应被读取，输出：{text}"
    );
}

/// 程序目录没有 nanorc 时，不应凭空出现该语法（对照用例）。
#[test]
fn missing_exe_nanorc_is_ignored() {
    let (_base, bin_dir, home_dir) = prepare();
    let exe = copy_exe_to(&bin_dir);
    let text = run_list_syntaxes(&exe, &home_dir);
    assert!(
        !text.contains("EXE_RC_TEST"),
        "没有程序目录 nanorc 时不应出现测试语法，输出：{text}"
    );
}

/// 程序目录 nanorc 读到后继续读用户级 ~/.nanorc（分层，而非短路）。
#[test]
fn exe_nanorc_does_not_shadow_user_nanorc() {
    let (_base, bin_dir, home_dir) = prepare();
    fs::write(
        bin_dir.join("nanorc"),
        "syntax \"EXE_RC_TEST\" \"^exerc$\"\n",
    )
    .unwrap();
    fs::write(
        home_dir.join(".nanorc"),
        "syntax \"USER_RC_TEST\" \"^userc$\"\n",
    )
    .unwrap();

    let exe = copy_exe_to(&bin_dir);
    let text = run_list_syntaxes(&exe, &home_dir);
    assert!(
        text.contains("EXE_RC_TEST"),
        "程序同目录的 nanorc 应被读取，输出：{text}"
    );
    assert!(
        text.contains("USER_RC_TEST"),
        "程序目录 nanorc 之后还应继续读用户级 ~/.nanorc，输出：{text}"
    );
}
