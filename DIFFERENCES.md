# Rust 版程序与原版 C 程序差异清单

对比范围：Rust 版 `src/` 与原版 C 程序 `nano/src/`，覆盖 17 个源文件模块、功能开关、i18n、构建系统、数据结构等维度。

---

## 一、架构与构建体系差异

| 维度 | C 版 (nano) | Rust 版 (rustnano) |
|------|------------|-------------------|
| 构建系统 | autotools：`configure.ac`/`Makefile.am`/`m4`/`aclocal.m4` | Cargo + `Cargo.toml` + `build.rs` |
| 条件编译 | 21 个 `ENABLE_*` / `NANO_TINY` 宏开关 | 仅保留 `ENABLE_COLOR`，其余功能开关未按 C 版方式条件编译（特性门控缺失） |
| 全局状态 | 裸全局变量 + `extern` 声明（`prototypes.h`） | `GlobalState` 结构体 + `with_global`/`with_global_mut` 闭包访问 + `Rc<RefCell<...>>` 引用计数 |
| 内存管理 | 手动 `malloc`/`free`/`nmalloc`/`nrealloc` | Rust 所有权 + `Rc`/`RefCell`，`nmalloc` 等改为 `Vec` 封装 |
| 终端库 | ncurses（`initscr`/`wgetch`/`wnoutrefresh` 等） | crossterm（`color.rs` 新增 `nano_to_crossterm_color`/`init_pair`/`COLOR_PAIR`/`wattron` 等适配层） |
| 正则 | 系统 POSIX `regex_t`/`regcomp`/`regexec` | 新增独立 `regex.rs` 模块，封装 `regex` crate |
| 测试 | 无测试目录 | 新增 `tests/` 目录（11 个集成测试：`input.rs`/`isolate.rs`/`keymap_check.rs`/`syntax_check.rs` 等） |

---

## 二、国际化 (i18n) 差异

| 维度 | C 版 | Rust 版 |
|------|------|---------|
| 机制 | GNU gettext（`bindtextdomain`/`textdomain`） | 自研 ftl 加载器（`i18n.rs`），运行时读 `locales/*.ftl` |
| 支持语言 | 40 种 `.po` 文件（ar/bg/...等） | 仅 2 种：`en-US.ftl`、`zh-CN.ftl` |
| 翻译宏 | `_("...")` 运行时查表 | `t!("key", arg=val)` 宏 + `HashMap` 参数（ftl 外置，编译期不读入） |
| 构建 | `po/` 目录由 gettext 工具链编译为 `.gmo` | `build.rs` 将 `locales/*.ftl` 复制到 `target/{debug,release}/locales` |

---

## 三、缺失或简化的功能模块

> 注：以下条目中标记 ✅ 的功能已在本仓库后续提交中补全（对应函数已实现，
> 并在 `tests/new_features.rs` 中有集成测试覆盖）。

### 1. 信号与终端处理（部分补全 ✅）

C 版 `nano.c` 有完整的信号处理，Rust 版新增 `signals.rs` 模块补全：

- ✅ `set_up_signal_handlers` / `set_up_sigwinch_handler` / `block_sigwinch`
  已实现（Unix 平台；handler 只设置原子标志，由主循环消费）
- ✅ `handle_hupterm` / `handle_sigwinch` 语义已实现（SIGHUP/SIGTERM → 紧急保存退出；
  SIGWINCH → 重新查询尺寸并重绘 `regenerate_screen`）
- ✅ `suspend_nano` / `continue_nano` / `do_suspend()` 已实现（Unix 上恢复终端后
  发送 SIGTSTP；Windows 无 POSIX 信号，仅恢复终端并重绘）
- ✅ `install_handler_for_Ctrl_C` / `restore_handler_for_Ctrl_C` 已实现
- `make_a_note` / `reconnect_and_store_state` / `handle_crash` 未实现
- 鼠标支持 `mouse_init`/`enable_mouse_support`/`disable_mouse_support` 仍缺失
  （虽有 `USE_MOUSE` 标志）

### 2. 文件操作 (`files.rs` vs `files.c`)

已实现：`open_buffer`/`save_to`/`do_writeout`/`make_new_buffer`/`close_buffer`/`prepare_for_display`/`emergency_save_all`

缺失或空实现：

- ✅ `execute_command`（执行外部命令并插入输出）— 已实现（`std::process::Command`
  跨平台替代 fork/pipe；支持 `|cmd` 管道模式与 `||cmd` 直通终端）
- ✅ `insert_a_file_or`（插入文件/命令）— 已实现
- ✅ `do_insertfile` / `do_execute` — 已实现（含受限模式检查）
- ✅ `make_backup_of`（文件备份）— 已实现简化版（`文件名~`；`MAKE_BACKUP` 时保存前备份）
- `write_file`（完整写文件，含格式/追加/前置等）— Rust 版 `save_to` 仅基础写入
- `write_region_to_file`（写选定区域）— 未实现
- `do_savefile` — 未实现
- ✅ `write_lockfile` / `delete_lockfile` / `lock_filename_for`（文件锁）— 已实现
  （1024 字节 vim/nano 兼容格式；`LOCKING` 时打开/保存/关闭接入）
- `scoop_stdin`（从 stdin 读取）— 未实现
- ✅ `switch_to_prev_buffer` / `switch_to_next_buffer` / `mention_name_and_linecount`
  / `redecorate_after_switch` — 已实现（多缓冲区切换，M-, / M-. 绑定）
- `init_backup_dir` / `copy_file` / `is_dir` / `diralphasort` — 部分缺失

### 3. 文本编辑 (`text.rs` vs `text.c`)

已实现：`do_mark`/`do_tab`/`do_indent`/`do_unindent`/`do_comment`/`do_undo`/`do_redo`/`do_enter`/`do_wrap`/`do_exit`/`inject`

缺失或空实现：

- ✅ `do_justify` / `do_full_justify`（段落对齐）— 已实现（`justify_text`/`justify_paragraph`/
  `concat_paragraph`/`squeeze`/`rewrap_paragraph`/`find_paragraph`）
- `do_linter`（语法检查器）— 未实现（`DoLinter` 键位仍为空）
- ✅ `do_verbatim_input`（逐字输入）— 已实现（`M-V` 绑定，用 `get_verbatim_kbinput` 读取并注入）
- ✅ `do_word_completion` / `complete_a_word`（单词补全）— 已实现（`^]` 绑定）
- ✅ `do_spell()` — 已实现（写临时文件 → 调用 `-s/--speller` 或 `SPELL`/`spell` → 读回替换）
- ✅ `do_formatter()` — 已实现（syntax 的 formatter 命令处理整个缓冲区）
- ✅ `zap_all_cutbuffer()` — 已实现
- ✅ `do_suspend()` — 已实现（见信号处理一节）
- `break_case` — 空函数体（undo 特殊分支；与 C 版行为一致，C 版该分支只 `break`）

### 4. 搜索 (`search.rs` vs `search.c`)

已实现：`search_init`/`do_search_forward`/`do_search_backward`/`do_research`/`do_findprevious`/`do_findnext`/`do_replace`/`goto_line_and_column`/`findnextstr`

缺失：

- ✅ `not_found_msg`（未找到消息）— 已实现（`search.rs`）
- ✅ `replace_regexp` / `replace_line`（正则替换构造）— 已实现（简化版）
- ✅ `find_a_bracket` / `do_find_bracket`（括号匹配）— 已实现（`M-]` 绑定）
- ✅ `do_gotolinecolumn`（交互式跳转）— 已实现（`^/` 绑定；支持 `++`/`--` 相对跳转）

### 5. 浏览器 (`browser.rs` vs `browser.c`)

已实现：`browser_refresh`/`to_first_file`/`to_last_file`/`strip_last_component`/`browse`/`browse_in`
以及 `read_the_list`/`reselect`/`findfile`/`search_filename`/`research_filename`（DIFFERENCES 原列缺失项已实现）。

### 6. winio（显示）模块

缺失：

- ✅ `minibar`（极简状态栏）— 已实现（`MINIBAR` 模式时主循环调用）
- ✅ `confirm_margin`（行号边距确认）— 已实现（原 `current_margin` 只读版升级为带副作用版本，主循环每轮调用）
- ✅ `print_view_warning`（只读模式警告）— 已实现（`winio::print_view_warning`）
- ✅ 超长行横向滚动的截断标记 — 已实现：`display_string` 返回 `(String, has_more)`，
  `update_line` 依据 `from_col > 0` 在行首画 `<`、依据 `has_more` 在行尾画 `>`
  （对应 C 版 `edit_draw` 两处 `waddch`）
- ✅ 软换行分块绘制 — 已实现：新增 `update_softwrapped_line`（逐块 `display_string` +
  `draw_row`，行号仅显示在第一块，聚光高亮跨块保留）；`update_line` 在 SOFTWRAP 下
  返回真实占用行数，`refresh_screen` 的行循环消费该返回值（原实现只返回
  `extra_chunks_in + 1` 而从不真正分块绘制）

### 7. 主循环差异 (`main.rs` vs `nano.c` main)

C 版 `process_a_keystroke` 有复杂的"输入 puddle 累积批量注入"机制（收集连续字节直到有命令或无待处理键时再 `inject`），Rust 版 `handle_input_key` 简化为逐键处理。C 版主循环还包含：鼠标点击处理(`process_click`)、BOM 检测提示、minibar 刷新、CONSTANT_SHOW 光标位置报告、零行模式(`ZERO`)特殊重绘等。Rust 版主循环已补全：✅ minibar 刷新、✅ CONSTANT_SHOW 光标位置报告、✅ confirm_margin、✅ 信号标志消费（SIGHUP/SIGTERM/SIGWINCH/SIGTSTP/SIGCONT）。

### 8. 多文件/多缓冲区

C 版 `main()` 支持：循环读取多个命令行文件、`+LINE,COLUMN` 定位参数、colon-parsing、命令行搜索串、`openfile = openfile->next` 切换到首个文件。Rust 版 `main.rs` 已补全：✅ 多命令行文件（`open_another_buffer`）、✅ `+LINE,COLUMN` 定位（`parse_file_args`）、✅ 切回首个缓冲区（`switch_to_prev_buffer`）。colon-parsing 与命令行搜索串未实现。

---

## 四、覆盖较完整的模块

以下模块函数覆盖基本对等：

- **cut.rs** ↔ cut.c（剪贴板：cut/copy/paste/chop/expunge/extract_segment/ingraft 全覆盖）
- **chars.rs** ↔ chars.c（字符处理全覆盖，Rust 版还多出 `mbstrcasestr`/`revstrstr` 等）
- **movement.rs** ↔ move.c（光标移动全覆盖，Rust 版多出 `do_first_line`/`do_last_line`）
- **history.rs** ↔ history.c（历史记录全覆盖）
- **help.rs** ↔ help.c（帮助全覆盖）
- **color.rs** ↔ color.c（语法着色全覆盖）
- **utils.rs** ↔ utils.c（工具函数全覆盖，缺 `mallocstrcpy`/`free_and_ass`，因 Rust 不需要）
- **rcfile.rs** ↔ rcfile.c（rc 文件解析：行数 718 vs 1760，Rust 版明显精简但核心 `do_rcfiles`/`parse_rcfile` 在）

---

## 五、模块映射关系

Rust 版把 C 版的 `nano.c`（主文件，73198 字节）拆分到了多处：链表操作(`splice_node`/`delete_node`/`renumber_from`)移到 `files.rs`，`do_exit`/`do_suspend`/`inject` 等移到 `text.rs`，`report_cursor_position` 移到 `global.rs`，主循环留在 `main.rs`。C 版的 `move.c` 对应 Rust 版 `movement.rs`。

---

## 六、总结

Rust 版完成了 nano 的核心编辑功能骨架（能编译运行，覆盖 cut/chars/movement/history/help/color/utils 等基础模块），但**大量进阶功能为空实现或完全缺失**——特别是信号处理、文件备份/锁、外部命令执行、拼写检查、格式化、段落对齐、括号匹配、多缓冲区切换、鼠标支持等。i18n 从 gettext 的 40 语言缩减为 2 语言。
