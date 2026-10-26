# nanoxide 与 GNU nano 9.2 的差异清单

- 对比对象：nanoxide（commit `4e66630`，2026-08-31）的 `src/` 与 GNU nano 9.2 的 `src/`
- 核对方式：**静态阅读源码**与函数名、选项表、快捷键表的脚本比对。除非条目里另有说明，**没有编译运行**。
- 状态图例：
  - ✅ 已实现且可从用户入口触达（有绑定或命令行入口）
  - ⚠️ 函数存在，但**触达不了**或只是简化版
  - ❌ 未实现
  - ❓ 未验证
- 维护规则见 [AGENTS.md](AGENTS.md)：改动了行为，就要同步改本文件。

---

## 一、架构与构建

| 维度 | GNU nano 9.2 | nanoxide |
|------|--------------|----------|
| 规模 | `src/*.c` 约 2.4 万行（不含 gnulib） | `src/*.rs` 约 2.24 万行，模块基本一一对应 |
| 构建 | autotools | Cargo + `build.rs`（把 `locales/*.ftl` 复制到 `target/*/locales`） |
| 依赖 | ncurses、gettext、系统 libc | 仅 `crossterm`、`libc` |
| 条件编译 | 约 21 个 `ENABLE_*` / `NANO_TINY` 开关 | 只保留 `ENABLE_COLOR`，其余功能始终编入，无 tiny 构建 |
| 全局状态 | 裸全局变量 + `prototypes.h` | `GlobalState` + `with_global` / `with_global_mut` + `Rc<RefCell<…>>` |
| 内存 | `malloc` / `free` | 所有权；`nmalloc` 等改为 `Vec` |
| 终端 | ncurses | crossterm，`color.rs` 里有 `init_pair` / `COLOR_PAIR` / `wattron` 适配层 |
| 平台 | 类 Unix | Linux 与 Windows；信号相关仅 Unix 生效 |
| 二进制名 | `nano` | `nax` |
| 测试 | 无 | `tests/` 下 15 个集成测试文件 |

### 正则

- 原版使用 POSIX `regcomp` / `regexec`。
- `DIFFERENCES.md` 旧版称 `regex.rs` 封装了 `regex` crate，**这是错的**：`Cargo.toml` 没有该依赖，`regex.rs`（约 683 行）是自写引擎。
- ❓ 与 POSIX 的语义一致性未验证（交替的最左最长 / 最左优先、`\<` `\>`、反向引用、字符类）。语法高亮和搜索替换都依赖它，是回归测试的重点。

### 模块映射

| C | Rust | 备注 |
|---|------|------|
| `nano.c` | `main.rs` + `signals.rs` + 部分 `files.rs` / `text.rs` / `global.rs` | 链表操作在 `files.rs`，`do_exit` / `inject` 在 `text.rs`，`report_cursor_position` 在 `global.rs` |
| `move.c` | `movement.rs` | |
| 其余 | 同名 `.rs` | |
| 无 | `regex.rs`、`i18n.rs`、`signals.rs`、`lib.rs` | 新增 |

---

## 二、国际化

| 维度 | C 版 | Rust 版 |
|------|------|---------|
| 机制 | gettext，`_()` | 自研 ftl 加载器 `i18n.rs`，`t!("key", arg = val)`，运行时读 `locales/*.ftl` |
| 语言 | 40 种 `.po` | `en-US`、`zh-CN` |
| 复数 | `ngettext` | ⚠️ `text.rs` 两个分支都返回单数（有 TODO） |
| 缺失文案 | — | ⚠️ `search.rs` 有一处翻译被注释占位 |

README 写的是 fluent，但项目并未依赖 fluent crate，格式只是类 ftl。

---

## 三、命令行（与原版差异最大的部分）

解析器是手写 `match`（`global.rs::parse_args`），**不是 getopt_long**：

- ❌ 不支持短选项合并（`-ES`）、`--opt=value`、长选项缩写
- ❌ 未知选项被静默忽略，不报 usage 错误
- ❌ `-Y/--syntax` 是空操作
- ❌ `-`（读 stdin）被跳过
- ❌ `+/pattern`、`+?pattern` 等命令行搜索串未实现
- ✅ `+LINE[,COLUMN]` 与多文件已支持
- ✅ `--` 之后的参数当作文件名

### 短选项映射与 9.2 不一致

下面这些在 nanoxide 里**含义不同**，会让按 nano 习惯输入的用户触发别的功能：

| 功能 | nano 9.2 | nanoxide | 9.2 中 nanoxide 所用字母的实际含义 |
|------|----------|----------|--------------------------------------|
| boldtext | `-D` | `-b` | `-b` = breaklonglines |
| rebinddelete | `-d` | `-K` | `-K` = rawsequences |
| locking | `-G` | `-Z` | `-Z` = zap |
| positionlog | `-P` | `-g` | `-g` = showcursor |
| colonparsing | `-@` | `-O` | `-O` = bookstyle |
| modernbindings | `-/` | `-M` | `-M` = trimblanks |
| wordbounds | `-W` | `-X` | `-X` = wordchars |
| insecurebackup | 9.2 无此选项 | `-I` | `-I` = ignorercfiles |

### 缺失的选项

下列 9.2 选项在 `parse_args` 中没有对应：

`afterends`(-y) `atblanks`(-a) `bookstyle` `breaklonglines`(-b) `ignorercfiles`(-I) `indicator`(-q) `listsyntaxes`(-z) `magic`(-!) `minibar`(-_) `newbuffer`(仅有旧名 `multibuffer`) `noread`(-n) `rawsequences` `showcursor` `solosidescroll`(-1) `stateflags`(-%) `trimblanks` `wordchars`(-X) `zap`

rc 文件方面：9.2 的 69 个 `set` 选项名全部出现在 `rcfile.rs` 里，rc 指令（`set unset bind unbind color icolor syntax header magic comment formatter linter tabgives include extendsyntax`）也都有处理。❓ 但只做了字符串比对，**每个选项是否真正生效未逐一验证**。

---

## 四、行为差异（疑似 bug）

1. **默认硬换行是开的（与 9.2 相反）**
   C 版 `main()` 一开始 `SET(NO_WRAP)`，rc 之后才据 `!ISSET(NO_WRAP)` 设置 `BREAK_LONG_LINES`。nanoxide 没有初始的 `SET(NO_WRAP)`，`main.rs` 中 `!ISSET(NO_WRAP)` 恒为真，因此 `BREAK_LONG_LINES` 总是被设上。❓ 静态推断，未运行验证。
2. **`M-J`（全文对齐）不可用**：`do_full_justify` 已实现，但绑定表没有 `M-J`，`winio.rs::execute_by_id` 也没有 `DoFullJustify` 分支。
3. **`M-<` / `M->`（左右滚动）** 在主菜单未绑定（只绑给了文件浏览器），`execute_by_id` 也没有 `DoScrollLeft` / `DoScrollRight` 分支。
4. **缺失的主菜单绑定**：`M-{` `M-}`（缩进 / 反缩进）、`Sh-Tab`、`M-W` `M-Q`（查找下一个 / 上一个）在 `shortcut_init` 中找不到。F1–F12、Ctrl+方向键在绑定表里也没找到，❓ 可能由输入层单独处理，未深查。
   绑定表整体约 94 条 `add_to_sclist`，C 版 239 条（含大量备用键，不能直接比数量）。
5. **输入批处理**：C 版 `process_a_keystroke` 把连续输入攒成 puddle 再一次注入，Rust 版逐键处理。大段粘贴的性能与撤销粒度可能不同。❓
6. **`execute_by_id` 的兜底分支 `_ => {}`**：被绑定但未分发的功能会**静默吞键**，不会有任何提示。上面第 2、3 条就是这么来的。

---

## 五、功能模块状态

### 1. 信号与终端（`signals.rs`）

- ✅ `set_up_signal_handlers` / `set_up_sigwinch_handler` / `block_sigwinch`（Unix；handler 只置原子标志，主循环消费）
- ✅ SIGHUP / SIGTERM 紧急保存退出；SIGWINCH 重绘；SIGTSTP / SIGCONT 挂起与恢复
- ✅ `install_handler_for_Ctrl_C` / `restore_handler_for_Ctrl_C`
- ❌ `make_a_note` / `reconnect_and_store_state` / `handle_crash`
- ❌ 鼠标：只有 `USE_MOUSE` 标志，没有 `mouse_init` / `enable_mouse_support` / `process_click`

### 2. 文件（`files.rs`）

- ✅ `open_buffer` / `save_to` / `do_writeout` / `make_new_buffer` / `close_buffer` / `emergency_save_all`
- ✅ `execute_command` / `insert_a_file_or` / `do_insertfile` / `do_execute`（用 `std::process::Command` 替代 fork/pipe，支持 `|cmd` 与 `||cmd`）
- ✅ 锁文件（`write_lockfile` / `delete_lockfile` / `lock_filename_for`，1024 字节 vim/nano 兼容格式）
- ✅ 多缓冲区切换（`M-,` / `M-.`）
- ⚠️ `make_backup_of`：只做 `文件名~`，没有 `backupdir`、`insecure_backup` 的完整逻辑
- ⚠️ `save_to` 只做基础写入，没有 `write_file` 的追加 / 前置 / 格式转换
- ❌ `write_region_to_file`（写选区）、`do_savefile`
- ❌ `scoop_stdin`（从标准输入读）
- ❌ `diralphasort`、`init_backup_dir`、`check_writable_directory`
- ❌ `outside_of_confinement`（操作目录限制，没找到同名或等价实现，❓ 可能命名不同）
- ❌ `username_completion`；⚠️ `input_tab` 文件名补全是桩（`prompt.rs` 注释"简化：不执行文件补全"）

### 3. 文本编辑（`text.rs`）

- ✅ `do_mark` / `do_tab` / `do_indent` / `do_unindent` / `do_comment` / `do_undo` / `do_redo` / `do_enter` / `do_wrap` / `do_exit` / `inject`
- ✅ `do_justify`（`^J`）、`do_verbatim_input`（`M-V`）、`do_word_completion`（`^]`）、`do_spell`、`do_formatter`、`zap_all_cutbuffer`
- ⚠️ `do_full_justify`：已实现但不可触达（见第四节）
- ❌ `do_linter`：键位在，函数为空
- ❓ `suck_up_input_and_paste_it`、`copy_character`、`replace_buffer` 在 Rust 中找不到同名函数，可能已改名或合并，未核对语义

### 4. 搜索（`search.rs`）

- ✅ 前后向搜索、`do_research`、`do_findnext` / `do_findprevious`、`do_replace`、`goto_line_and_column`、`do_gotolinecolumn`（`^/`，含 `++` / `--` 相对跳转）、`do_find_bracket`（`M-]`）、`not_found_msg`
- ⚠️ `replace_regexp` / `replace_line` 是简化版：匹配长度直接取 needle 长度
- ❌ `put_or_lift_anchor` 同名函数；❓ 锚点相关功能另有实现，未逐项核对

### 5. 文件浏览器（`browser.rs`）

- ✅ `browse` / `browse_in` / `read_the_list` / `reselect` / `findfile` / `search_filename` / `research_filename` / `to_first_file` / `to_last_file`

### 6. 显示（`winio.rs`）

- ✅ `minibar`、`confirm_margin`、`print_view_warning`
- ✅ 超长行横向滚动的 `<` / `>` 截断标记
- ✅ 软换行分块绘制 `update_softwrapped_line`
- ❌ `draw_scrollbar`、`do_credits`、`show_states_at`
- ❌ `get_mouseinput`
- ⚠️ 输入解析（`parse_kbinput` / `convert_CSI_sequence` 等）由 crossterm 事件替代，不是逐字节解析；原版对异常转义序列的容错行为不保证一致

### 7. 主循环与启动（`main.rs`）

- ✅ 初始化顺序与 `nano.c::main()` 一致；命令行优先于 rc 的合并逻辑已移植
- ✅ minibar 刷新、`CONSTANT_SHOW`、`confirm_margin`、信号标志消费
- ✅ 多命令行文件、`+LINE,COLUMN`
- ❌ colon-parsing（`file:line:col`）、命令行搜索串
- ❌ BOM 提示、`ZERO` 模式的特殊重绘
- ❌ `--listsyntaxes`；❓ `-h` 的 `print_usage` 与原版 `usage()` 的输出内容未逐项对比

### 8. 覆盖较完整的模块

`cut.rs`、`chars.rs`、`movement.rs`、`history.rs`、`help.rs`、`color.rs`、`utils.rs`、`rcfile.rs` 的函数基本对等。`rcfile.rs` 当前 1585 行（旧版文档写 718 行，已过时）。`chars.rs` 比原版多 `mbstrcasestr` / `revstrstr`，`movement.rs` 多 `do_first_line` / `do_last_line`。

---

## 六、总结

nanoxide 已移植 nano 绝大多数编辑功能，且有 Windows 支持。当前最需要处理的是：

1. 命令行层与 9.2 不兼容（短选项含义不同、无 getopt 语义、缺 17 个选项）。
2. 默认硬换行与原版相反。
3. 若干"已实现"的功能没有接到按键上，被 `execute_by_id` 静默吞掉。
4. 鼠标、选区写文件、stdin、目录限制、linter 仍缺失。
5. i18n 只有两种语言，且复数未处理。

## 七、与上一版文档的主要更正

- `regex.rs` 不是 `regex` crate 的封装。
- `rcfile.rs` 行数由 718 更正为 1585。
- 项目名由 "rustnano" 更正为 nanoxide。
- ✅ 的定义收紧为"用户可触达"；`do_full_justify` 由 ✅ 改为 ⚠️。
- 新增：命令行短选项映射、缺失选项、默认换行、绑定缺口、`execute_by_id` 兜底分支。
