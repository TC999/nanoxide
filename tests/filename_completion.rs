//! 文件名补全（对应 files.c 的 filename_completion / input_tab）。

use nanoxide::definitions::*;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nanoxide_tab_{}_{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup() {
    nanoxide::global::global_init();
    restore_flags(GlobalFlags::new());
    with_global_mut(|g| {
        g.COLS = 80;
        g.LINES = 24;
        g.present_path = None;
        g.operating_dir = None;
    });
}

#[test]
fn lists_matching_files() {
    setup();
    let dir = temp_dir("list");
    std::fs::write(dir.join("alpha.txt"), "a").unwrap();
    std::fs::write(dir.join("alpine.txt"), "b").unwrap();
    std::fs::write(dir.join("beta.txt"), "c").unwrap();
    std::fs::create_dir(dir.join("alps")).unwrap();

    let prefix = format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR);
    let mut matches =
        nanoxide::files::filename_completion(&format!("{}al", prefix)).expect("应有匹配");
    matches.sort();
    assert_eq!(matches, vec!["alpha.txt", "alpine.txt", "alps"]);

    assert!(
        nanoxide::files::filename_completion(&format!("{}zz", prefix)).is_none(),
        "无匹配"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tab_completes_common_prefix_and_directory() {
    setup();
    let dir = temp_dir("prefix");
    std::fs::write(dir.join("alpha.txt"), "a").unwrap();
    std::fs::write(dir.join("alpine.txt"), "b").unwrap();
    std::fs::create_dir(dir.join("onlydir")).unwrap();
    let sep = std::path::MAIN_SEPARATOR;

    let mut answer = format!("{}{}al", dir.display(), sep);
    let mut place = answer.len();
    let mut listed = false;
    nanoxide::files::input_tab(&mut answer, &mut place, None, &mut listed);
    assert!(
        answer.ends_with(&format!("{}alp", sep)),
        "应补到公共前缀，实际 {answer:?}"
    );
    assert_eq!(place, answer.len());

    let mut answer = format!("{}{}only", dir.display(), sep);
    let mut place = answer.len();
    nanoxide::files::input_tab(&mut answer, &mut place, None, &mut listed);
    assert!(
        answer.ends_with(&format!("{}onlydir{}", sep, sep)),
        "唯一目录候选应补斜杠，实际 {answer:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn operating_dir_filters_matches() {
    setup();
    let dir = temp_dir("confine");
    std::fs::write(dir.join("alpha.txt"), "a").unwrap();
    let other = temp_dir("confine_other");

    with_global_mut(|g| {
        g.operating_dir = Some(other.to_string_lossy().into_owned());
    });

    let prefix = format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR);
    assert!(
        nanoxide::files::filename_completion(&format!("{}al", prefix)).is_none(),
        "操作目录外的候选应被过滤"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn tab_does_nothing_when_cursor_is_not_at_end() {
    setup();
    let mut answer = "some".to_string();
    let mut place = 2;
    let mut listed = false;
    nanoxide::files::input_tab(&mut answer, &mut place, None, &mut listed);
    assert_eq!(answer, "some");
    assert_eq!(place, 2);
}
