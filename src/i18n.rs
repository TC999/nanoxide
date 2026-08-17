/**************************************************************************
 * i18n.rs  --  内置 i18n：基于外部 ftl 文件的轻量翻译支持
 *
 * 约定：
 *   - ftl 文件始终位于程序外部的 locales/ 目录（不嵌入二进制）；
 *   - 语言文件命名：en-US.ftl / zh-CN.ftl / ...
 *   - 默认语言：en-US，找不到对应文件时回退。
 *
 * ftl 格式（本模块使用的最简子集）：
 *   <key> = <value>
 *   支持 <value> 中的 {argname} 占位符，用 args 参数表填充。
 *   行首 # 视为注释，空行跳过。
 **************************************************************************/

use std::collections::HashMap;
use std::path::PathBuf;

// ================ ftl 文件解析（外部文件，绝不内嵌） ================

/// 一个编译好的 ftl 资源表：key -> 原始模板字符串（含 {argname} 占位符）。
#[derive(Debug, Default, Clone)]
pub struct Ftllib {
    pub entries: HashMap<String, String>,
}

impl Ftllib {
    /// 解析一行 "key = value"，返回 (key, value)。
    /// 跳过空行、注释行 (#)、解析失败行。
    fn parse_line(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let split = trimmed.find('=')?;
        let key = trimmed[..split].trim().to_string();
        let value = trimmed[split + 1..].trim().to_string();
        if key.is_empty() {
            return None;
        }
        Some((key, value))
    }

    /// 读取 ftl 文件到 HashMap。
    pub fn load(path: &PathBuf) -> Result<Self, std::io::Error> {
        let mut entries = HashMap::new();
        let content = std::fs::read_to_string(path)?;
        for line in content.lines() {
            if let Some((k, v)) = Self::parse_line(line) {
                entries.insert(k, v);
            }
        }
        Ok(Ftllib { entries })
    }

    /// 格式化单个键。{argname} 占位符由 args 提供；若 arg 缺失，则替换为 "<argname>"。
    pub fn format_message(&self, key: &str, args: &HashMap<&str, &str>) -> String {
        let template = self.entries.get(key).map(|s| s.as_str()).unwrap_or("");
        // 回退：找不到模板 → 返回键名本身（便于定位）。
        if template.is_empty() {
            return format!("[[{}]]", key);
        }
        let mut out = String::new();
        let bytes = template.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                // 找匹配的 '}'
                let rest_start = i + 1;
                let mut end = None;
                for j in rest_start..bytes.len() {
                    if bytes[j] == b'}' {
                        end = Some(j);
                        break;
                    }
                }
                if let Some(e) = end {
                    let arg = std::str::from_utf8(&bytes[rest_start..e]).unwrap_or("").trim();
                    out.push_str(match args.get(arg) {
                        Some(v) => v,
                        None => {
                            out.push_str("<");
                            out.push_str(arg);
                            out.push('>');
                            i = e + 1;
                            continue;
                        }
                    });
                    i = e + 1;
                } else {
                    out.push('{');
                    i += 1;
                }
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
        out
    }
}

// ================ 语言协商 ================

/// 从 LANG 环境变量解析出的语言代码（en-US、zh-CN 等），失败时回退到 "en-US"。
pub fn detect_lang() -> String {
    // 优先 LANG（形如 zh_CN.UTF-8、en_US、C、""）。
    if let Ok(lang) = std::env::var("LANG") {
        if let Some(code) = normalize_lang(&lang) {
            return code;
        }
    }
    // 也接受 LC_ALL / LC_MESSAGES（可选兜底）。
    for env in &["LC_ALL", "LC_MESSAGES"] {
        if let Ok(lang) = std::env::var(env) {
            if let Some(code) = normalize_lang(&lang) {
                return code;
            }
        }
    }
    "en-US".to_string()
}

/// 把 "zh_CN.UTF-8" / "zh_CN" / "zh" 规范化为 "zh-CN"。
/// 只取前两个字段（language 与 country），中间用 '-' 连接；大写 country 部分。
pub fn normalize_lang(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() || s == "C" || s == "POSIX" {
        return None;
    }
    // 按 '_' / '.' / '-' 拆分。
    let parts: Vec<&str> = s.split(|c: char| c == '_' || c == '.' || c == '-').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut result = String::new();
    result.push_str(&parts[0].to_lowercase());
    if parts.len() >= 2 {
        result.push('-');
        result.push_str(&parts[1].to_uppercase());
    }
    Some(result)
}

// ================ 加载器 ================

/// 当前进程的语言代码（由 init 确定一次）。
thread_local! {
    static CURRENT_LANG: std::cell::RefCell<String> = std::cell::RefCell::new("en-US".to_string());
    static LOCALE_MAP: std::cell::RefCell<HashMap<String, Ftllib>> = std::cell::RefCell::new(HashMap::new());
}

/// 语言文件的候选根目录（按优先级依次尝试）：
///   1. 环境变量 `NANORS_LOCALES` 显式指定；
///   2. 可执行文件所在目录下的 locales/（发布时随程序分发）；
///   3. 当前工作目录下的 locales/（cargo run / 开发场景）。
/// 仅依赖 exe 旁目录会在 `cargo run`（exe 位于 target/debug/）时找不到
/// 项目根下的 locales/，导致 t() 回退成 [[key]]。
fn locales_dir_candidates() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(override_dir) = std::env::var("NANORS_LOCALES") {
        dirs.push(PathBuf::from(override_dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("locales"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("locales"));
    }
    dirs
}

/// 加载指定语言代码对应的 ftl 文件（按需），并缓存。
/// 依次尝试各候选目录，找到第一个可用的即加载；全部失败才返回错误。
fn ensure_loaded(lang: &str) -> Result<(), std::io::Error> {
    if LOCALE_MAP.with(|m| m.borrow().contains_key(lang)) {
        return Ok(());
    }
    let mut last_err = std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("no {}.ftl found in any locales dir", lang),
    );
    for dir in locales_dir_candidates() {
        let path = dir.join(format!("{}.ftl", lang));
        match Ftllib::load(&path) {
            Ok(lib) => {
                LOCALE_MAP.with(|m| {
                    m.borrow_mut().insert(lang.to_string(), lib);
                });
                return Ok(());
            }
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// 加载默认 en-US（若存在）。
fn ensure_loaded_default() {
    let _ = ensure_loaded("en-US");
}

// ================ 对外 API ================

/// 初始化 i18n（可被 main 调用一次，也可被宏调用时自动触发）。
pub fn init() {
    CURRENT_LANG.with(|l| {
        *l.borrow_mut() = detect_lang();
    });
    ensure_loaded_default();
}

/// 获取当前协商出的语言代码。
pub fn current_lang() -> String {
    CURRENT_LANG.with(|l| l.borrow().clone())
}

/// 用键和可选参数表格式化消息。
pub fn t(key: &str, args: &HashMap<&str, &str>) -> String {
    let mut found = None;
    // 优先当前语言。
    CURRENT_LANG.with(|l| {
        let lang = l.borrow().clone();
        let _ = ensure_loaded(&lang);
        LOCALE_MAP.with(|m| {
            if let Some(lib) = m.borrow().get(&lang) {
                if lib.entries.contains_key(key) {
                    found = Some(lib.format_message(key, args));
                }
            }
        });
    });
    if let Some(s) = found {
        return s;
    }
    // 回退到 en-US。
    ensure_loaded_default();
    LOCALE_MAP.with(|m| {
        if let Some(lib) = m.borrow().get("en-US") {
            lib.format_message(key, args)
        } else {
            format!("[[{}]]", key)
        }
    })
}

/// 不带参数的便捷版本。
pub fn tx(key: &str) -> String {
    t(key, &HashMap::new())
}

// ================ 宏 ================

/// 编译期转发的翻译宏。支持两种形式：
///   i18n::t!("key")
///   i18n::t!("key", arg1 = value, arg2 = other)
///
/// 注意：ftl 文件本身不在编译期读入（始终外置）；这里只做参数表的编译期构建。
#[macro_export]
macro_rules! t {
    ($key:literal) => {{
        $crate::i18n::tx($key)
    }};
    ($key:literal, $($arg:ident = $val:expr),+ $(,)?) => {{
        {
            let mut args: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
            $(
                let _v: String = $val.to_string();
                args.insert(stringify!($arg), &_v);
            )+
            $crate::i18n::t($key, &args)
        }
    }};
}

// ================ 单元测试 ================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang("zh_CN.UTF-8"), Some("zh-CN".to_string()));
        assert_eq!(normalize_lang("en_US"), Some("en-US".to_string()));
        assert_eq!(normalize_lang("zh_cn.utf8"), Some("zh-CN".to_string()));
        assert_eq!(normalize_lang("zh-cn"), Some("zh-CN".to_string()));
        assert_eq!(normalize_lang("C"), None);
        assert_eq!(normalize_lang("POSIX"), None);
        assert_eq!(normalize_lang(""), None);
    }

    #[test]
    fn test_parse_line() {
        assert!(Ftllib::parse_line("").is_none());
        assert!(Ftllib::parse_line("# comment").is_none());
        assert_eq!(
            Ftllib::parse_line("hello = world"),
            Some(("hello".to_string(), "world".to_string()))
        );
        assert_eq!(
            Ftllib::parse_line("greet = Hello {name}!"),
            Some(("greet".to_string(), "Hello {name}!".to_string()))
        );
    }

    #[test]
    fn test_format_message() {
        let mut entries = HashMap::new();
        entries.insert("greet".to_string(), "Hello {name}!".to_string());
        entries.insert("plain".to_string(), "OK".to_string());
        let lib = Ftllib { entries };

        let mut args = HashMap::new();
        args.insert("name", "Rust");
        assert_eq!(lib.format_message("greet", &args), "Hello Rust!");
        assert_eq!(lib.format_message("plain", &args), "OK");
        assert_eq!(lib.format_message("missing", &args), "[[missing]]");

        let empty: HashMap<&str, &str> = HashMap::new();
        assert_eq!(lib.format_message("greet", &empty), "Hello <name>!");
    }

    #[test]
    fn test_t_fallback() {
        let empty: HashMap<&str, &str> = HashMap::new();
        // 无 en-US 时仍返回键名标记。
        let result = t("does-not-exist", &empty);
        assert!(result.contains("does-not-exist"));
    }

    #[test]
    fn test_lang_negotiation_zh_cn_falls_back() {
        // 关键验收：即使 LANG 协商出 "zh-CN"，当无 zh-CN.ftl 时，所有消息回退到 en-US.ftl。
        let empty: HashMap<&str, &str> = HashMap::new();

        // 模拟 LANG=zh_CN.UTF-8，并调用 init() 触发语言协商（会刷新 CURRENT_LANG）。
        std::env::set_var("LANG", "zh_CN.UTF-8");
        super::init();
        assert_eq!(super::current_lang(), "zh-CN");

        // 由于无 zh-CN.ftl，welcome.message / cut.nothing_cut 都必须回退到 en-US.ftl。
        let welcome = super::t("welcome-message", &empty);
        assert!(welcome.contains("Welcome to nano"), "expected en-US fallback, got: {}", welcome);

        let cut = super::t("cut-nothing_cut", &empty);
        assert!(cut.contains("Nothing was cut"), "expected en-US fallback, got: {}", cut);

        // 恢复。
        std::env::remove_var("LANG");
    }
}
