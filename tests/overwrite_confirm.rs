//! 保存时的覆盖确认（对应 files.c `write_it_out` 的 do_warning 分支）。
//!
//! C 版在 method == OVERWRITE 时按顺序检查三件事：
//!   1. `do_warning`：改名保存（或无名缓冲区要写到一个已存在的文件）；
//!   2. `confirm_namechange`：先问 "Save file under DIFFERENT NAME?"；
//!   3. 目标已存在：问 "File "<name>" exists; OVERWRITE?"；
//!   4. 名字未变但 `statinfo` 与磁盘不一致：提示 "File on disk has changed"
//!      并问 "File was modified since you opened it; continue saving?"。
//!
//! 受限模式下（`ISSET(RESTRICTED)`）只警告 "File exists -- cannot overwrite"，
//! 不允许覆盖，也不允许改名。
//!
//! 交互循环本身需要真实终端，这里覆盖其判定函数与文案截断。

use nanoxide::definitions::*;

fn setup() {
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    std::env::set_var("NANORS_LOCALES", locales);
    std::env::set_var("LANG", "en-US");
    nanoxide::global::global_init();
    nanoxide::i18n::init();
    restore_flags(GlobalFlags::new());
    nanoxide::files::make_new_buffer();
    nanoxide::global::shortcut_init();
    with_global_mut(|g| {
        g.currmenu = MMAIN;
        g.COLS = 80;
        g.LINES = 24;
        g.editwincols = 80;
    });
}

fn temp_file(label: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "nanoxide_ow_{}_{}_{}",
        label,
        std::process::id(),
        std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
    ));
    std::fs::write(&path, content).unwrap();
    path
}

/// 无名缓冲区：目标已存在才需要警告，不存在则是普通的首次保存。
#[test]
fn nameless_warns_only_when_target_exists() {
    assert!(nanoxide::files::needs_name_warning(
        "out.txt", "", true, None, None
    ));
    assert!(
        !nanoxide::files::needs_name_warning("out.txt", "", false, None, None),
        "无名缓冲区写到不存在的文件不应警告"
    );
}

/// 有名缓冲区：两边都能规范化时比较规范路径（`./a.txt` 与 `a.txt` 同名）。
#[test]
fn compares_canonical_paths_when_available() {
    let same = nanoxide::files::needs_name_warning(
        "/tmp/x/a.txt",
        "/tmp/x/a.txt",
        true,
        Some("/tmp/x/a.txt/"),
        Some("/tmp/x/a.txt/"),
    );
    assert!(!same, "规范路径相同不算改名");

    let changed = nanoxide::files::needs_name_warning(
        "/tmp/x/b.txt",
        "/tmp/x/a.txt",
        true,
        Some("/tmp/x/b.txt/"),
        Some("/tmp/x/a.txt/"),
    );
    assert!(changed, "规范路径不同应警告");
}

/// 规范化失败时退回原始字符串比较（对应 C 的 `strcmp(answer, filename)`）。
#[test]
fn falls_back_to_raw_comparison() {
    assert!(
        !nanoxide::files::needs_name_warning("a.txt", "a.txt", true, None, Some("/x/a.txt/")),
        "任一侧取不到规范路径时按原始名字比较"
    );
    assert!(nanoxide::files::needs_name_warning(
        "b.txt", "a.txt", true, None, None
    ));
}

/// 磁盘变更判定：statinfo 缺失或时间不晚于打开时都视为未变更。
///
/// 用 `File::set_modified` 显式设定时间，避免依赖真实文件系统的时间戳
/// 粒度与目录项延迟刷新（Windows 上写完立刻 stat 可能拿不到新 mtime）。
#[test]
fn disk_change_requires_newer_mtime() {
    let base = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    let later = base + std::time::Duration::from_secs(60);

    assert!(
        !nanoxide::files::disk_changed_since_open(None, later),
        "没有打开时的 statinfo（新文件）不比较"
    );
    assert!(
        !nanoxide::files::disk_changed_since_open(Some(base), base),
        "时间相同不算变更"
    );
    assert!(
        nanoxide::files::disk_changed_since_open(Some(base), later),
        "磁盘变新时应报告变更"
    );
    assert!(
        !nanoxide::files::disk_changed_since_open(Some(later), base),
        "磁盘上的副本更旧时不应报警"
    );
}

/// 真实文件路径：写盘后取到的 mtime 应能作为"打开时"的基线，
/// 再次显式改晚时间则判定为已变更。
#[test]
fn real_file_mtime_comparison() {
    use std::time::Duration;
    let file = temp_file("mtime", "one\n");
    let at_open = std::fs::metadata(&file).unwrap().modified().unwrap();

    // 立刻再 stat：不应报告变更。
    let same = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(
        !nanoxide::files::disk_changed_since_open(Some(at_open), same),
        "未改动时不应报告磁盘变更"
    );

    let bumped = at_open + Duration::from_secs(120);
    let f = std::fs::OpenOptions::new().write(true).open(&file).unwrap();
    f.set_modified(bumped).unwrap();
    drop(f);
    let now = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(
        nanoxide::files::disk_changed_since_open(Some(at_open), now),
        "显式改晚 mtime 后应报告变更"
    );
    let _ = std::fs::remove_file(&file);
}

/// 覆盖确认的问题文本按 COLS 截断文件名，且宽度不超过状态栏。
#[test]
fn overwrite_question_crops_name_to_fit() {
    setup();
    let short = nanoxide::files::overwrite_question("a.txt", 80);
    assert!(short.contains("a.txt"), "放得下时应保留完整名字：{short}");
    assert!(
        short.contains("OVERWRITE") || t_helper::t_contains_overwrite(&short),
        "问题应保留覆盖询问的措辞：{short}"
    );

    let long_name = format!("{}.txt", "x".repeat(200));
    let long = nanoxide::files::overwrite_question(&long_name, 80);
    assert!(
        nanoxide::utils::breadth(long.as_bytes()) <= 80,
        "截断后应放进 80 列：{}",
        nanoxide::utils::breadth(long.as_bytes())
    );
    assert!(long.contains("..."), "超宽名字应以省略号开头：{long}");
}

/// 极窄终端下问题仍应可生成（C 的 crop_to_fit 在 room<4 时返回 "_"）。
#[test]
fn overwrite_question_survives_tiny_width() {
    setup();
    let q = nanoxide::files::overwrite_question("somewhat/long/name.txt", 10);
    assert!(!q.is_empty(), "窄屏也应给出问题文本");
    assert!(
        nanoxide::utils::breadth(q.as_bytes()) <= 40,
        "窄屏下问题不应无限膨胀：{q}"
    );
}

/// 受限模式：C 的检查在 `write_it_out` 层（`ISSET(RESTRICTED)` → 只警告
/// 不允许覆盖/改名），`write_file` 本身不判受限。判定函数与受限无关，
/// 因此改名警告照常计算——真正拦住写入的是循环里的 RESTRICTED 分支。
#[test]
fn name_warning_is_independent_of_restriction() {
    assert!(nanoxide::files::needs_name_warning("b.txt", "a.txt", true, None, None));
    assert!(nanoxide::files::needs_name_warning("b.txt", "a.txt", false, None, None));
}

mod t_helper {
    /// 判断截断后的问题是否仍含"覆盖"语义（en 为 OVERWRITE，zh 为"覆盖"）。
    pub fn t_contains_overwrite(s: &str) -> bool {
        s.contains("OVERWRITE") || s.contains("覆盖")
    }
}
