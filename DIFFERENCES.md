# nanoxide 与 GNU nano 9.2 的差异清单

- 对比对象：nanoxide 的 `src/`（持续更新）与 GNU nano 9.2 的 `src/`
- 核对方式：**静态阅读源码**与函数名、选项表、快捷键表的脚本比对；行为变更均配集成测试。
  除非条目里另有说明，**没有在真实终端里逐项手工操作验证**。
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
| 规模 | `src/*.c` 约 2.0 万行（不含 gnulib） | `src/*.rs` 约 2.3 万行，模块基本一一对应 |
| 构建 | autotools | Cargo + `build.rs`（把 `locales/*.ftl` 复制到 `target/*/locales`） |
| 依赖 | ncurses、gettext、系统 libc | 仅 `crossterm`、`libc` |
| 条件编译 | 约 21 个 `ENABLE_*` / `NANO_TINY` 开关 | 无编译开关，所有功能始终编入（无 tiny 构建）；仅在注释中标出对应的 C 开关 |
| 全局状态 | 裸全局变量 + `prototypes.h` | `GlobalState` + `with_global` / `with_global_mut` + `Rc<RefCell<…>>` |
| 内存 | `malloc` / `free` | 所有权；`nmalloc` 等改为 `Vec` |
| 终端 | ncurses | crossterm，`color.rs` 里有 `init_pair` / `COLOR_PAIR` / `wattron` 适配层 |
| 平台 | 类 Unix | Linux 与 Windows；信号相关仅 Unix 生效 |
| 二进制名 | `nano` | `nax` |
| 测试 | 无 | `tests/` 下 44 个集成测试文件 |

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
| 复数 | `ngettext` | `_one` / `_many` 双键，由代码按数量选择（`files-wrote_*`、`files-lines_*`、`search-replaced_*`、`winio-minibar_line_*`、`count-{line,word,char}_*`） |
| 缺失文案 | — | 无（`tests/i18n_keys.rs` 扫描 `src/` 里的 `t!` 字面量键，校验两种语言都有） |
| 底部栏标签 | `add_to_funcs` 的 tag | 全部走 `t!("key-*")`，文本按 9.2 `global.c` 的 tag 对齐 |

README 写的是 fluent，但项目并未依赖 fluent crate，格式只是类 ftl。

---

## 三、命令行

解析器仍是手写 `global.rs::parse_command_line`，但已按 GNU `getopt_long` 的语义实现：

- ✅ 短选项合并（`-ES`）、参数粘连（`-T4`）或分开（`-T 4`）
- ✅ `--opt=value`、`--opt value`、长选项唯一前缀缩写（如 `--tabstospac`）
- ✅ 选项可出现在文件名之后（等价于 getopt 的 argv 重排）、`--` 结束选项
- ✅ 单独的 `-` 作为 stdin 文件名；`+LINE[,COLUMN]` / `+/pattern` 仍由文件参数逻辑处理
- ✅ 未知/歧义/缺参选项：打印 getopt 风格错误 + `Type '<prog>' -h ...`，退出码 1
- ✅ `-T`、`-J` 要求 >0，`-r` 要求可解析；错误文案复用 `rcfile-*_invalid`
- ✅ `-I` 真正跳过 rc；`-z` 列出语法；`-Y` 设置 `syntaxstr`
- ⚠️ getopt 自身的错误文本仍为英文，未走 `t!`

### 短选项映射

已对齐 9.2 的 getopt 字符串
`ABC:DEFGHIJ:KLMNOPQ:RST:UVWX:Y:Zabcdef:ghijklmno:pqr:s:tuvwxyz!@%_01/`
与 `long_options[]`。此前含义错位的字母已修正：

| 功能 | 9.2 |
|------|-----|
| boldtext | `-D` |
| rebinddelete | `-d` |
| locking | `-G` |
| positionlog | `-P` |
| colonparsing | `-@` |
| modernbindings | `-/` |
| wordbounds | `-W` |
| wordchars | `-X <string>` |
| rawsequences | `-K` |
| zap | `-Z` |
| trimblanks | `-M` |
| bookstyle | `-O` |
| breaklonglines | `-b` |
| nowrap | `-w` |

`-I` 不再是 `insecurebackup`（9.2 没有该命令行选项），而是 `ignorercfiles`；
`allow_insecure_backup` 只作为 rc 选项保留。

### 已接线与不支持的选项

9.2 的短/长选项现已全部处理：

- 已接线并有测试/静态证据：`-D`（粗体）、`-d`（Backspace/Delete 互换）、
  `-H`（历史记录，含 `have_statedir` 检查）、`-t`（有文件名时退出直接保存）、
  `-U`（状态栏消息 1 次按键后清除）、`-P`（记录并恢复光标位置）、
  `-u`/`-N`（DOS 行尾的读取/保存转换，见 `tests/file_format.rs`）、
  `-q`（右侧滚动条，软换行下按块精确计算，见 `tests/scrollbar.rs`）、
  `-%`（标题栏与 minibar 的状态字符 I/M/L/R/S）。
- `-!/--magic`：本项目没有 libmagic，因此与**未启用 libmagic 的 nano 9.2**
  一致——命令行 `-!` / `--magic` 与 rc 的 `set magic` 都按未知选项拒绝，
  不再“接受但静默置位”。

rc 文件方面：9.2 在未启用 libmagic 时的 68 个 `set` 选项名全部出现在
`rcfile.rs` 里，rc 指令（`set unset bind unbind color icolor syntax header
magic comment formatter linter tabgives include extendsyntax`）也都有处理。
❓ 但只做了字符串比对，**每个选项是否真正生效未逐一验证**。

---

## 四、行为差异

1. ✅ **默认硬换行已修正**：`main()` 在解析命令行前 `SET(NO_WRAP)`，rc 之后按
   `!ISSET(NO_WRAP)` 设置 `BREAK_LONG_LINES`；`-b` / `-w` 用 `hardwrap` 三态
   在 rc 处理之后覆盖。`fill` 默认值改为 C 版的 `-COLUMNS_FROM_EOL`，`wrap_at`
   按 `COLS + fill` 计算。
   证据：静态阅读 `main.rs` / `winio.rs`，`tests/command_line.rs`（三态与默认值）。
2. ✅ **`M-J`（全文对齐）可用**：`shortcut_init` 绑定 `M-J`，
   `execute_by_id` 分发 `DoFullJustify`；内置快捷键表兜底分发也已在
   `execute_function` 末尾接上。证据：`tests/keymap_reachability.rs`。
3. ✅ **`M-<` / `M->`（左右滚动）** 已在主菜单绑定并分发
   `DoScrollLeft` / `DoScrollRight`（`M-<` / `M->` 的键码统一为
   `0x200 + '<'` / `0x200 + '>'`）。证据：同上，测试断言 `brink` 变化。
4. ✅ **缺失的主菜单绑定已补齐**：`M-{`（反缩进）、`M-}`（缩进）、`Sh-Tab`
   （反缩进）、`M-W`（查找下一个）、`M-Q`（查找上一个）。注意 C 版里
   `M-}` 是缩进、`M-{` 是反缩进。证据：`tests/keymap_reachability.rs`。
   F1–F12 也已按 `global.c:1574-1595` 写入绑定表并删除旧的硬编码分支
   （F4=justify、F5=read file、F6=where is、F7/F8=翻页、F9/F10=剪切/粘贴、
   F11=报告位置、F12=拼写；帮助页 F1=退出）。证据：同一测试文件。
5. **输入批处理**：C 版 `process_a_keystroke` 把连续输入攒成 puddle 再一次注入，
   Rust 版逐键处理。大段粘贴的性能与撤销粒度可能不同。❓
6. **`execute_by_id` 兜底与分发审计**：FunctionId 共 125 个，其中 83 个有
   `execute_by_id` 分支。审计结果：
   - 主菜单（MMAIN）的可达绑定都能命中硬编码分支或 `execute_by_id`；
   - 未分发的其余名字（`AppendIt`、`PrependIt`、`DosFormat`、`BackItUp`、
     `DiscardBuffer`、`Flip*`、`*Void` 等）多来自 rc 的 `bind` 占位名。
     C 版里这些函数本身是空函数（global.c 的 `{;}`），真正行为在写文件/
     插入/执行提示循环中；这两个循环已分别移植到 `do_writeout` 与
     `insert_a_file_or`。
   - 从主菜单直接 bind 这些占位名时，兜底分支会显示
     `Function not implemented: <名字>`（`tests/function_dispatch.rs`）。
     这是对 C 的偏离（C 中空函数无提示），用于避免“按键被吞”的错觉。⚠️
7. ✅ **插入/执行提示的翻转键**：`insert_a_file_or` 改为循环，
   支持 `^X`（插入/执行互换）、`M-F`（新缓冲区）、`M-N`（不转换）、
   `M-\`（命令前加/去管道）与 `^T`（浏览）；`NEW_BUFFER` 在提示结束后
   恢复原值（C 的 `was_multibuffer`），提示文本随 `NO_CONVERT` 与
   `NEW_BUFFER` 切换。测试见 `tests/insert_flip.rs`。
8. ✅ **`-t` / `-U` / `-P` 已接线**：
   - `-t`：`do_exit` 在有文件名时直接 `write_it_out`，无文件名时先提示
     "No file name" 再询问（`tests/save_onexit.rs`）。
   - `-U`：状态栏消息倒计时为 1，否则 20（`blank_it_when_expired`，
     `tests/quickblank.rs`）。
   - `-P`：`load_positions_register` / `restore_cursor_position_if_any` /
     `update_positions_register` 已接入启动、打开与关闭流程；位置寄存器
     解析改为按“数字+空格”扫描锚点，兼容 Windows 的 `C:\...` 路径
     （`tests/positionlog.rs`）。
9. ✅ **主菜单控制键按 9.2 的 legacy 绑定修正**：`^S` 保存、`^Q` 反向搜索、
   `^W` 正向搜索、`^L` 居中（其余菜单全屏刷新）、`^T` 执行命令、`Ins`
   插入文件、`M-R` 替换；`--preserve` 时不绑定 `^S`/`^Q`
   （`tests/keymap_reachability.rs`）。此前 `handle_input_key` 的硬编码把
   `^S` 当挂起、`^Q`/`^L` 当刷新、`^T` 当拼写、`Ins` 当标记。
10. ⚠️ **`--modernbindings`（`-/`）尚未切换键位表**：目前只处理
    `--preserve` 与提示的取消键；C 的 modern 表（`^S` 保存、`^W` 写文件、
    `^O` 读文件、`^Q` 退出、`^D`/`^G` 查找等）未实现。

---

## 五、功能模块状态

### 1. 信号与终端（`signals.rs`）

- ✅ `set_up_signal_handlers` / `set_up_sigwinch_handler` / `block_sigwinch`（Unix；handler 只置原子标志，主循环消费）
- ✅ SIGHUP / SIGTERM 紧急保存退出；SIGWINCH 重绘；SIGTSTP / SIGCONT 挂起与恢复
- ✅ `install_handler_for_Ctrl_C` / `restore_handler_for_Ctrl_C`
- ❌ `make_a_note` / `reconnect_and_store_state` / `handle_crash`
- ⚠️ 鼠标：`mouse_init`（启停 crossterm 捕获）与编辑区左键点击（`handle_mouse_click`）
  已实现，`tests/mouse_support.rs` 覆盖；原版 `get_mouseinput`（浏览器/帮助区的
  点击定位）未移植。

### 2. 文件（`files.rs`）

- ✅ `open_buffer` / `save_to` / `do_writeout` / `make_new_buffer` / `close_buffer` / `emergency_save_all`
- ✅ `read_file` 的 "Read N lines" 提示：按 C 的 `num_lines` 计行（空文件
  为 0），DOS 格式用 "(converted from DOS format)" 变体；打开文件在
  ZERO/MINIBAR 下静默、插入文件时仍显示（`tests/read_message.rs`）。
- ✅ 打开文件时检查可写性：不可写则优先提示
  `File '<name>' is unwritable`（Unix 用 `access(W_OK)`，Windows 用追加
  打开探测；查看模式不检查；`tests/read_message.rs`）。
- ✅ `execute_command` / `insert_a_file_or` / `do_insertfile` / `do_execute`（用 `std::process::Command` 替代 fork/pipe，支持 `|cmd` 与 `||cmd`）
  插入/执行提示支持 `^X`/`M-F`/`M-N`/`M-\`/`^T` 翻转键（`tests/insert_flip.rs`）
- ✅ 锁文件（`write_lockfile` / `delete_lockfile` / `lock_filename_for`，1024 字节 vim/nano 兼容格式）
- ✅ 多缓冲区切换（`M-,` / `M-.`）
- ⚠️ `make_backup_of`：只做 `文件名~`，没有 `backupdir`、`insecure_backup` 的完整逻辑
- ✅ `save_to` 支持覆盖/追加/前置（`WritingType`），行尾按 DOS/Unix 格式转换；
  `do_writeout` 的提示循环支持 M-A/M-P/M-D/M-B/^Q/^T；`exiting` 为真时
  忽略标记、整块保存（对应 C 版 `write_it_out` 的 `exiting` 参数）。
  证据：`tests/file_format.rs`、`tests/write_methods.rs`
- ✅ 写入反馈与落盘：正常保存前显示 "Writing..."；写入后用 `sync_all`
  确保数据落盘（FIFO 跳过，对应 C 的 `fflush + fsync` 与
  `!S_ISFIFO(...)` 条件）；磁盘满（Unix ENOSPC / Windows
  ERROR_DISK_FULL）时额外警告文件可能已被截断并提示恢复步骤，同时刷新
  statinfo（`tests/write_methods.rs`）。
- ✅ `write_region_to_file`（写选区，`tests/write_region.rs`；区域也可追加/前置）、`do_savefile`
- ✅ `scoop_stdin`（命令行 `-`，`tests/stdin_args.rs`）
- ✅ `diralphasort`、`init_backup_dir`（`tests/file_tools.rs`）
- ✅ `outside_of_confinement` / `init_operating_dir`（`--operatingdir`：
  启动 chdir、打开/保存/插入与文件浏览器的越界检查；`tests/operating_dir.rs`）
- ✅ `filename_completion` / `input_tab` / `username_completion`：
  文件名补全支持公共前缀、唯一目录补分隔符、`~user`、操作目录过滤
  与 `MGOTODIR` 只补目录（`tests/filename_completion.rs`）。
  多候选按 C 的网格在编辑区分栏列出（`completion_list_layout` /
  `draw_completion_list`，行数不足时显示 "(more)"，`tests/completion_list.rs`）。
  `username_completion` 仅在 Unix 生效（Windows 无 passwd）。
- ✅ `check_writable_directory` / `safe_tempfile`：`$TMPDIR` → 系统临时目录 →
  当前目录依次尝试，保留缓冲区扩展名；临时文件用 create_new 保证唯一。
  证据：`tests/tempfile.rs`

### 3. 文本编辑（`text.rs`）

- ✅ `do_mark` / `do_tab` / `do_indent` / `do_unindent` / `do_comment` / `do_undo` / `do_redo` / `do_enter` / `do_wrap` / `do_exit` / `inject`
- ✅ `do_justify`（`^J`）、`do_verbatim_input`（`M-V`）、`do_word_completion`（`^]`）、`do_spell`、`do_formatter`、`zap_all_cutbuffer`
- ✅ `do_full_justify`（`M-J`，见第四节）
- ✅ `do_linter`：执行菜单 `^Y` 调用；已修改时先询问是否保存
  （`linter-save_before`，有文件名直接保存、无名则进入提示），清除标记，
  按 `construct_argument_list` 的方式拆分命令并直接在真实文件上运行
  （不经 shell、不替换 `%s`）；跨文件消息会切到对应缓冲区，未打开时询问
  “在新缓冲区打开吗？”（是→打开、否→丢弃该文件的消息、取消→退出）；
  `^Y`/`^V` 切换消息、`^X`/Enter 退出（`tests/linter.rs`）。
  简化：用规范化路径代替 C 的 inode 比较。
- ✅ `count_lines_words_and_characters`（`M-D`）：行/词/字符按 C 的格式
  显示，三个名词用 `count-*_one` / `count-*_many` 双键；有标记时只统计
  选区并加 "In Selection:" 前缀；行数按 C 的公式；词数用 `do_next_word`
  从起点逐步前进到终点（`tests/count.rs`）。
- 🔧 绑定修正：`M-D` 之前误绑到 `ALT_DELETE` 的剪切，导致字数统计不可达；
  现按 `global.c` 改为 `M-D` → 字数统计、`M-Del` → 剪切整行。
- ❓ `suck_up_input_and_paste_it`、`copy_character`、`replace_buffer` 在 Rust 中找不到同名函数，可能已改名或合并，未核对语义

### 4. 搜索（`search.rs`）

- ✅ 前后向搜索、`do_research`、`do_findnext` / `do_findprevious`、`do_replace`、`goto_line_and_column`、`do_gotolinecolumn`（`^/`，含 `++` / `--` 相对跳转）、`do_find_bracket`（`M-]`）、`not_found_msg`
- ⚠️ `replace_regexp` / `replace_line` 是简化版：匹配长度直接取 needle 长度
- ✅ 锚点：`text.rs` 的 `do_anchor` / `to_prev_anchor` / `to_next_anchor`
  由 `execute_by_id` 分发；`M-Ins` / `M-PgUp` / `M-PgDn` / `M-"` / `M-'`
  已按 `global.c:1356-1362` 绑定，`M-Home` / `M-End` 绑定到
  `to_top_row` / `to_bottom_row`（`tests/keymap_reachability.rs`）。
  `search.rs` 没有 `put_or_lift_anchor` 同名函数（C 的锚点逻辑已拆到
  `text.rs`）。

### 5. 文件浏览器（`browser.rs`）

- ✅ `browse` / `browse_in` / `read_the_list` / `reselect` / `findfile` / `search_filename` / `research_filename` / `to_first_file` / `to_last_file`

### 6. 显示（`winio.rs`）

- ✅ `minibar`、`confirm_margin`、`print_view_warning`
- ✅ 超长行横向滚动的 `<` / `>` 截断标记
- ✅ 软换行分块绘制 `update_softwrapped_line`
- ✅ `draw_scrollbar`（`-q`；软换行下用 `scrollbar_covered_lines` 按块
  累计覆盖行数，与 C 的 `coveredlines` 循环一致，`tests/scrollbar.rs`）、
  `show_states_at`
  （`states_string`，`-%` 的标题栏与 minibar 状态字符，位置按 C 的
  `COLS - 11 - padding` / `COLS - 5 - padding` / `COLS - 4 - padding`，
  `tests/indicator_stateflags.rs`）
- ✅ `do_credits`（`^X` → "Y" → 无名缓冲区输入 `zzy` 的彩蛋；
  `tests/credits.rs` 校验 52 行文本表与翻译键）：全屏向上滚动字幕，
  任一按键中止；期间临时置位 ZERO/NO_HELP，结束后恢复并重绘。
  差异：版本行显示本程序版本（非 C 版编译期写死的 9.2）；滚动用等高的
  虚拟窗口重绘模拟 `wscrl`；`window_init` 的 ncurses 窗口重建省略。
  ⚠️ 动画本身依赖真实终端，仅静态核对与数据表测试
- ❌ `get_mouseinput`（编辑器点击路径见 `handle_mouse_click`）
- ✅ minibar 的行数提示与多缓冲区 `[x/n]` 计数：`report_size` 在读取/保存后
  置位、绘制后被消费；`tallywidth` 参与状态字符的空间判断；`buffer_number`
  从第一个缓冲区计数（`tests/minibar_tally.rs`）。
- ✅ minibar 的十六进制码：`CONSTANT_SHOW` 下按 UTF-8 显示 `U+XXXX`、
  非 UTF-8 显示 `  0xXX`，行尾显示 `U+000A`（有下一行）或 `  ----`，
  并附加最多两个零宽后继码；显示后继码时状态字符让位（`tests/minibar_hex.rs`）。
- ✅ 标题栏左上角的多缓冲区 `[x/n]`：`more_than_one` 在创建第二个缓冲区时
  置位，关闭缓冲区时不重置（与 C 版一致），序号按 `buffer_number` 计算
  （`tests/titlebar_ranking.rs`）。
- ✅ 多缓冲区时底部栏 `^X` 的标签改为 "Close"，关闭到只剩一个缓冲区时
  改回 "Exit"（对应 C 版 install_buffer / close_buffer 对
  `exitfunc->tag` 的修改；同时作用于主菜单与文件浏览器，`tests/exit_label.rs`）。
- ⚠️ 输入解析（`parse_kbinput` / `convert_CSI_sequence` 等）由 crossterm 事件替代，不是逐字节解析；原版对异常转义序列的容错行为不保证一致
- ✅ `waiting_keycodes` 只统计按下/重复事件，并缓存预读事件供 `wgetch` 消费（修复 Windows 上按键释放事件导致提示栏输入迟迟不注入的问题）
- ✅ `warn_and_briefly_pause`（`-t` 无文件名时的提示）与 `blank_it_when_expired`
  （`--quickblank` 的 1/20 次按键倒计时）

### 7. 主循环与启动（`main.rs`）

- ✅ 初始化顺序与 `nano.c::main()` 一致；命令行优先于 rc 的合并逻辑已移植
  （标志位经 `clone_flags` / `restore_flags` 合并，`-I` 整块跳过 rc）
- ✅ minibar 刷新、`CONSTANT_SHOW`、`confirm_margin`、信号标志消费
- ✅ 多命令行文件、`+LINE,COLUMN`、colon-parsing（`file:line:col`）、命令行搜索串
- ✅ `--listsyntaxes`（`list_syntax_names`）；`-h` 的选项表已按 9.2 的 `usage()` 重写
- ❌ BOM 提示、`ZERO` 模式的特殊重绘
- ❓ `-h` 输出与 C 版 `usage()` 的逐字对齐（受限于未接入的 gettext 文案）

### 8. 覆盖较完整的模块

`cut.rs`、`chars.rs`、`movement.rs`、`history.rs`、`help.rs`、`color.rs`、`utils.rs`、`rcfile.rs` 的函数基本对等。`history.rs` 的位置寄存器（`-P`）已接入打开/关闭流程，并修正了 Windows 绝对路径的解析。`rcfile.rs` 当前约 1455 行。`chars.rs` 比原版多 `mbstrcasestr` / `revstrstr`，`movement.rs` 多 `do_first_line` / `do_last_line`。

---

## 六、总结

nanoxide 已移植 nano 绝大多数编辑功能，且有 Windows 支持。当前最需要处理的是：

1. 命令行选项已全部处理：`-q`/`-%` 已接线，`-!` 因无 libmagic 被明确拒绝。
2. `execute_by_id` 已审计：主菜单绑定全部可达；未分发的 rc 占位函数会显示
   “Function not implemented”而不是静默吞键。写文件提示的
   DosFormat/BackItUp/Append/Prepend/Discard 循环已移植。
3. 显示层基本完成：`do_credits` 的动画未在真实终端里手工验证；锚点键位
   尚未按 `global.c` 绑定（见第五节搜索条目）。
4. i18n 只有两种语言；单复数已用双键处理。

## 七、与上一版文档的主要更正

- `regex.rs` 不是 `regex` crate 的封装。
- `rcfile.rs` 行数由 718 更正为 1585。
- 项目名由 "rustnano" 更正为 nanoxide。
- ✅ 的定义收紧为"用户可触达"；`do_full_justify` 由 ✅ 改为 ⚠️。
- 新增：命令行短选项映射、缺失选项、默认换行、绑定缺口、`execute_by_id` 兜底分支。
- 本版更新：命令行解析已按 `getopt_long` 重写；默认 nowrap、M-J/M-</M->/M-{/M-}/
  Sh-Tab/M-W/M-Q 已接线；上一版标记为缺失的 mouse / write_region / stdin /
  diralphasort / init_backup_dir 经测试确认已存在；修正 `waiting_keycodes` 的
  按键释放事件问题。
- 后续更新：`-t`、`-U`、`-P` 已接线；F1–F12 绑定已按 `global.c` 对齐；
  `-P` 的位置寄存器解析兼容 Windows 路径。
- 再后续：`-u`/`-N` 已接线（CRLF 检测、保存回写、NO_CONVERT 保留 `\r`），
  读取/插入/stdin 共用同一套分行逻辑；新增 `tests/file_format.rs`。
- 本轮：`-q`（draw_scrollbar）与 `-%`（标题栏 I/M/L/R/S）已接线；
  无 libmagic 时 `-!` / `--magic` / rc `set magic` 改为按未知选项拒绝。
- 审计：`execute_by_id` 的 125 个 FunctionId 中 83 个有分发；主菜单可达绑定
  全部覆盖，rc 占位名改为显示未实现提示（不再 `_ => {}` 静默）。
- 本轮：写文件提示循环已移植（M-A/M-P 追加/前置、M-D DOS、M-B 备份、
  ^Q 放弃、^T 浏览）；`save_to` 支持 `WritingType`，新增
  `tests/write_methods.rs`。
- 后续：插入/执行提示循环已移植（^X/M-F/M-N/M-\ 翻转键、NEW_BUFFER
  恢复、管道符号），新增 `tests/insert_flip.rs`。
- 再后续：`--operatingdir` 已接线（chdir、打开/保存/插入/浏览器越界检查，
  Windows 路径归一化），新增 `tests/operating_dir.rs`。
- 本轮：`do_linter` 已接线（执行菜单 ^Y；解析 file:line[:col]: msg，
  ^Y/^V 切换、^X/Enter 退出；临时副本运行），新增 `tests/linter.rs`。
- 后续：`do_linter` 对齐 C——已修改时询问保存、清除标记、按空格拆分
  命令并直接在真实文件上运行（不经 shell、不替换 %s）；
  `build_linter_arguments` 加入 `tests/linter.rs`。
- 再本轮：文件名补全已实现（`input_tab` / `filename_completion` /
  `username_completion`），新增 `tests/filename_completion.rs`。
- 本轮：`check_writable_directory` 与 `safe_tempfile` 已实现（临时目录
  探测、保留扩展名）；`get_full_path` 在 Windows 上用 `\` 结尾，修复
  verbatim 路径拼接。
- 本轮：`do_credits` 彩蛋已实现（退出时文件名输入 `zzy` 触发，临时置位
  ZERO/NO_HELP、全屏滚动、任一键中止）；`do_writeout` 增加 `exiting`
  参数，退出时忽略标记；新增 `tests/credits.rs`。
- 本轮：`-%` 的状态字符已显示在 minibar（`COLS - 11 - padding`，位置与
  C 一致）；minibar 的行/列位置改为只在 `CONSTANT_SHOW` 时显示；新增
  `minibar_right_positions` 与 `tests/indicator_stateflags.rs` 的位置断言。
- 本轮：minibar 已显示刚读/写文件的行数与多缓冲区 `[x/n]` 计数
  （`report_size` 的置位/消费、`buffer_number`）；保存后仅在非 MINIBAR 时
  写 "Wrote" 消息；新增 `tests/minibar_tally.rs`。
- 本轮：minibar 的十六进制码已实现（`CONSTANT_SHOW`；UTF-8 码点/非 UTF-8
  字节、行尾 `U+000A`/`----`、最多两个零宽后继、后继码让位状态字符）；
  新增 `tests/minibar_hex.rs`。
- 本轮：标题栏左上角的多缓冲区 `[x/n]` 已实现（`more_than_one` 置位、
  `buffer_number` 计数、关闭后保持）；新增 `tests/titlebar_ranking.rs`。
- 本轮：多缓冲区时底部栏 ^X 的标签在 "Exit"/"Close" 间切换
  （`set_exit_label` 修改主菜单与浏览器的 DoExit 项，关闭到只剩一个时
  改回）；新增 `tests/exit_label.rs`。
- 本轮：软换行下的滚动条改为按 C 的 `coveredlines` 循环精确计算
  （`scrollbar_covered_lines`），不再按行数近似；新增 `tests/scrollbar.rs`。
- 本轮：多候选文件名补全改为在编辑区按 C 的网格分栏列出
  （`completion_list_layout` / `draw_completion_list`，行数不足显示
  "(more)"），不再写状态栏；新增 `tests/completion_list.rs`。
- 本轮：字数统计改用 C 的格式与魔法行公式，行/词/字符用 `_one`/`_many`
  双键；修正 `M-D` 绑定（原先误绑 `ALT_DELETE`，字数统计不可达，
  `M-Del` 才是剪切整行）；新增 `tests/count.rs`。
- 本轮：读取文件后显示 `Read N lines`（含 DOS 变体与单复数；空文件 0 行），
  ZERO/MINIBAR 下打开文件静默、插入时仍显示；新增 `tests/read_message.rs`，
  并更新 `tests/open_buffer.rs` 的过时断言。
- 本轮：打开文件时检查可写性，不可写优先提示 `File '<name>' is unwritable`
  （Unix `access(W_OK)`、Windows 追加打开探测），补充到 `tests/read_message.rs`。
- 本轮：字数统计支持标记区域（`In Selection:` 前缀、C 的字符数公式），
  词数改用 `do_next_word` 逐步前进；`tests/count.rs` 增加跨行/单行/零长度
  选区与前缀断言。
- 本轮：`do_linter` 的跨文件消息分支已实现（按规范化路径切缓冲区，
  未打开时询问是否在新缓冲区打开、否则丢弃该文件的消息）；
  `tests/linter.rs` 增加路径匹配与缓冲区查找测试。
- 本轮：写入反馈对齐 C——保存前显示 "Writing..."，磁盘满时给出
  "文件已被截断" 警告与恢复建议并刷新 statinfo；新增 `is_disk_full`
  与 `tests/write_methods.rs` 的平台相关测试。
- 本轮：保存改为 `File::create + write_all + sync_all`（FIFO 跳过
  fsync），对应 C 版 write_file 的 fflush + fsync；现有写入测试全部通过。
- 本轮：底部栏标签全部改走 `t!`（21 处硬编码英文 tag、38 个缺失的
  `key-*`），文本按 `global.c` 对齐；新增 `tests/i18n_keys.rs` 校验
  `t!` 键在两种语言里都存在；删除 `search.rs` 的过时占位注释。
- 复查：更新文档中的对比对象、源码行数、测试文件数（44）、条件编译
  描述、`rcfile.rs` 行数（约 1455）与锚点现状（键位尚未绑定）。
- 本轮：锚点与视口行键位按 `global.c:1356-1362` 修正——`M-Ins`/`M-PgUp`/
  `M-PgDn`/`M-"`/`M-'` 绑定锚点，`M-Home`/`M-End` 改为
  `to_top_row`/`to_bottom_row`，移除 `handle_input_key` 里把
  `ALT_INSERT` 当标记、`ALT_PAGEUP/PAGEDOWN` 当块移动的误绑；
  `tests/keymap_reachability.rs` 增加绑定与 `M-Ins` 实际切换锚点的测试。
- 本轮：绑定审计——主菜单控制键按 `global.c` 的 legacy 表修正
  （`^S` 保存、`^Q` 反向搜索、`^W` 正向搜索、`^L` 居中、`^T` 执行命令、
  `Ins` 插入文件、`M-R` 替换），`--preserve` 时 `^S`/`^Q` 不绑定；
  记录 `--modernbindings` 键位表尚未实现。
