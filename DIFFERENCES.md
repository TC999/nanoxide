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
| 规模 | `src/*.c` 约 2.0 万行（不含 gnulib） | `src/*.rs` 约 2.7 万行，模块基本一一对应 |
| 构建 | autotools | Cargo + `build.rs`（把 `locales/*.ftl` 复制到 `target/*/locales`） |
| 依赖 | ncurses、gettext、系统 libc | 仅 `crossterm`、`libc` |
| 条件编译 | 约 21 个 `ENABLE_*` / `NANO_TINY` 开关 | 无编译开关，所有功能始终编入（无 tiny 构建）；仅在注释中标出对应的 C 开关 |
| 全局状态 | 裸全局变量 + `prototypes.h` | `GlobalState` + `with_global` / `with_global_mut` + `Rc<RefCell<…>>` |
| 内存 | `malloc` / `free` | 所有权；`nmalloc` 等改为 `Vec` |
| 终端 | ncurses | crossterm，`color.rs` 里有 `init_pair` / `COLOR_PAIR` / `wattron` 适配层 |
| 平台 | 类 Unix | Linux 与 Windows；信号相关仅 Unix 生效 |
| 二进制名 | `nano` | `nax` |
| 测试 | 无 | `tests/` 下 54 个集成测试文件 |

### 正则

- 原版使用 POSIX `regcomp` / `regexec`。
- `DIFFERENCES.md` 旧版称 `regex.rs` 封装了 `regex` crate，**这是错的**：`Cargo.toml` 没有该依赖，`regex.rs`（约 683 行）是自写引擎。
- ⚠️ 与 POSIX 的语义一致性：`\<`（词首）/ `\>`（词尾）/ `\b`（任意词边界）
  已分别实现，并用 GNU grep 交叉验证 `\<word`、`word\>` 在词中/词尾的行为；
  模式内的反向引用（`\1`）未实现（替换文本的 `\1`..`\9` 已支持，见第五节）；
  交替的最左最长/最左优先与字符类边界情形仍未
  逐一验证。语法高亮和搜索替换都依赖它，是回归测试的重点。

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
| `-h` 说明文本 | `usage()` 里的 `_("...")` / `N_("...")` | 全部走 `t!("cmdline-*")`（`cmdline-usage`、`cmdline-opt_*`、带参旗标 `cmdline-short_/long_*`），中文取 `po/zh_CN.po` 的 msgstr |

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

已修复（2026-11，对照 nano 9.2 rcfile.c 静态核对 + `tests/rcfile_compat.rs`）：

- ✅ `extendsyntax` 存入 augmentations 的 data 此前丢掉命令名
  （`next_word` 截断后才取值），延迟应用必报 "not understood"；
  现按 C 版 rcfile.c:1448（`argument = copy_of(ptr)` 在截断前整段复制）
  存完整命令串，应用时再拆词分发。
- ✅ `syntax "x`（引号不配对）此前被误判为 "Missing syntax name"；
  现按 C 版 rcfile.c:646-651 在截词前检查原始行（`"` 后紧跟行尾或
  `"` 才算空名），`"x` 走 "Unpaired quote" 分支。
- ✅ `color.rs` 的 `INDICES` 表把 blue/yellow 颜色值写反（3/4 互换），
  导致 `color blue` 渲染为黄色；现按 C 版 rcfile.c:1041-1047 的
  ncurses 常量（COLOR_BLUE=4、COLOR_YELLOW=3）修正。

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
   插入文件、`M-R` 替换、`^Z` 提示挂起方式（`suggest_ctrlT_ctrlZ`）；
   `--preserve` 时不绑定 `^S`/`^Q`（`tests/keymap_reachability.rs`）。
   此前 `handle_input_key` 的硬编码把 `^S` 当挂起、`^Q`/`^L` 当刷新、
   `^T` 当拼写、`^Z` 当撤销、`Ins` 当标记。
10. ✅ **`--modernbindings`（`-/`）的键位表已实现**：`shortcut_init` 按
    该标志选择两套主菜单绑定，modern 表对应 `global.c:1243-1262`、`1301`
    （`^S` 保存、`^W` 写文件、`^O` 读文件、`^Q` 退出、`^D`/`^G` 查找、
    `^X` 剪切、`^C` 复制、`^V` 粘贴、`^A` 标记、`^Z`/`^Y` 撤销/重做、
    `^T` 跳行、`^E` 执行、`^H` 帮助、`^R` 替换、`^P` 报告位置）。
    `execute_function` 在该模式下先查 shortcut 表，避免 legacy 硬编码覆盖
    （`tests/keymap_reachability.rs`）。
11. ✅ **执行菜单（MEXECUTE）快捷键已补齐**：`^S`/`^T` 拼写、`^O` 格式化、
    `^V` 剪切到末尾、`^J` 全文对齐、`^Y` linter、`^Z` 挂起、`M-\` 管道
    （`global.c:1304-1305、1315、1449、1458`；`tests/keymap_reachability.rs`）。
12. ✅ **帮助查看器与滚动键已对齐**：帮助页 `^G` 退出、`Home`/`End`/
    `M-\`/`M-/`/`^Home`/`^End` 首末行、`Space` 翻页；主菜单与帮助页
    `M-Up`/`M-Down`/`M--`/`M-_`/`M-+`/`M-=` 滚动屏幕——此前
    `handle_input_key` 把 `M-Up`/`M-Down` 当段落移动
    （`global.c:1552-1565`；`tests/keymap_reachability.rs`）。
13. ✅ **`M-X` 切换帮助行**：绑定 `(MMOST|MBROWSER|MYESNO) & ~MFINDINHELP`
    → `DoToggle NO_HELP`，帮助查看器内不绑定（`tests/keymap_reachability.rs`）。
14. ✅ **Shift+方向键选择文本**：`translate_keycode` 按 `parse_kbinput`
    把 Shift+方向/Home/End/PgUp/PgDn 解析成普通 `KEY_*` 并置
    `shift_held`（对应 `KEY_SLEFT`/`KEY_SRIGHT`/`KEY_SR`/`KEY_SF`、
    `KEY_SHOME`/`KEY_SEND`/`KEY_SPREVIOUS`/`KEY_SNEXT` 分支）；
    `handle_input_key` 移植 `process_a_keystroke` 的软标记逻辑：执行
    Shift 移动前建立 `softmark`，松开 Shift 后的移动丢弃它。
    此前 Shift+Up/Down 返回 `SHIFT_UP`/`SHIFT_DOWN` 码点（显示为乱码
    字符），Shift+Left/Right 只移动光标而不建立标记。
    选中区域由 `draw_row` 末尾的 `paint_selection` 按
    `interface_color_pair[SELECTED_TEXT]` 覆盖绘制（默认反显），
    `selection_paint_range` 把标记区域与当前可视块（含水平滚动/软换行）
    求交；`hilite_attribute` 默认值改为 C 版的 `A_REVERSE`。
    证据：`tests/shift_selection.rs`。

15. ✅ **保存时的覆盖/改名/磁盘变更确认**：`do_writeout`（对应 C 版
    `write_it_out`）此前在 `method == OVERWRITE` 时直接落盘，会静默覆盖已有
    文件。现按 C 补齐三段检查：
    - `do_warning`（`needs_name_warning`）：无名缓冲区要写已存在的文件，
      或已有名字的缓冲区改了名（两侧都能 `get_full_path` 时比较规范路径，
      否则比较原始字符串）；
    - `confirm_namechange`：先问 `Save file under DIFFERENT NAME?`，
      否/取消则回到提示；
    - 目标已存在：问 `File "<name>" exists; OVERWRITE?`（文件名按
      `crop_to_fit` 截断到 `COLS`，`overwrite_question`）；
    - `else if` 分支：名字未变但磁盘更新（`disk_changed_since_open`）时
      警告 `File on disk has changed` 并问 `File was modified since you
      opened it; continue saving?`；`-t` 下按 C 就地收尾（是→写、否→返回
      2 放弃缓冲区、取消→返回 0）。
    受限模式下只警告 `File exists -- cannot overwrite`（C 直接用该文案，
    不走 `in_restricted_mode()` 的通用提示）。同时补 `write_file` 收尾的
    `stat_with_alloc`：常规覆盖保存后刷新 `statinfo`（`annotate` 且
    `OVERWRITE`），否则磁盘变更检查会拿打开时的旧时间误报。
    `-t` 且有文件名时也不再询问文件名（C 的
    `ISSET(SAVE_ON_EXIT) && exiting` 分支），直接进入上述检查。
    差异：C 还比较 `st_dev` / `st_ino`，Rust 标准库跨平台取不到稳定值，
    只比较 `st_mtime`（同内容、同时间但被换成另一个文件时检测不到）。
    另有既有偏差：`^X` → "是" 且缓冲区已有名字时，C 的
    `write_it_out(TRUE, TRUE)` 仍会显示 "Write to File: <名字>" 让用户确认
    （`withprompt` 为真、且不满足 `SAVE_ON_EXIT && exiting`），Rust 的
    `do_exit` 在该情形走 `write_it_out` 直接静默保存、不提示也不做上述
    覆盖/磁盘检查。要完全对齐需把 `do_exit` 的该分支也改为进入
    `do_writeout(exiting = true)`，会同时影响 `tests/save_onexit.rs`、
    `tests/exit_confirm.rs` 的既有预期，本次未动。⚠️
    证据：`tests/overwrite_confirm.rs`（判定函数、问题文本截断、
    mtime 比较）；交互循环需真实终端，未手工验证。

16. ✅ **Alt 键（软件内写作 `M-`）在 Linux 与 Windows 上一律失效已修复**。
    现象是"按 Alt+任意组合没有任何反应"。根因不在终端层，而是本项目把
    C 的「键码 + 全局 `meta_key` 标志」二元表达压成一个整数键码时，
    **绑定表与输入层用了两套互不相交的约定**（因此与平台无关，两个系统
    同时中招）。具体是下面七处叠加：
    1. `add_to_sclist` 漏掉 C 的键名回退。C 是
       `sc->keycode = keycode ? keycode : keycode_from_string(scstring)`
       （global.c），而内置表沿用 C 的书写习惯大量传 `0`。Rust 直接存 `0`，
       于是 `M-U`（撤销）、`M-E`（重做）、`M-A`（设标记）、`M-6`（复制）
       四条成为键码恒为 0 的死绑定，任何按键都匹配不到。已补回退。
    2. `keycode_from_string` 的 `M-` 分支仍照 C 返回老约定
       （`'D' | 0x20` = 0x64），而输入层产出 `0x200 + 字符`（0x264）。
       这使 **nanorc 里任何 `bind M-…` 都永远匹配不到真实按键**。
       现在新增 `alt_keycode_raw` / `alt_keycode_for` /
       `runtime_alt_keycode` 作为唯一出处：绑定名解析无条件把字母折成小写
       （C 的 `keycode_from_string` 不做该判断），输入层按 `shifted_metas`
       决定是否折叠（C 的 `parse_kbinput`），`Sh-M-<字母>` 保持大写以免与
       `M-<字母>` 撞码。
    3. `meta_key` 字段自始至终没有被任何代码置位（只在 `prompt.rs` 被读）。
       现在 `translate_keycode` 在开头按 C 的 `parse_kbinput` 对称地
       清 `meta_key`/`shift_held`，Alt 组合置位 `meta_key`。
       副作用（正向）：提示栏 `absorb_character` 的
       `input < 0x20 || meta_key` 判断从此生效——此前 Alt+d 的 0x264 会被
       当成普通码点注入搜索框，实测表现为凭空出现一个 U+0264（`ɤ`）。
    4. 未命中的 Alt 组合此前被**静默吞掉**，用户得不到任何反馈。已按
       C 的 `unbound_key`（nano.c:1288）重写 `global::unbound_key`：
       区分 `Unknown sequence` / `Unbound key: F%i` /
       `Unbindable key: M-^%c` / `Unbound key: Sh-M-%c` /
       `Unbound key: M-%c` / `Unbindable key: ^[` / `Unbound key: ^%c`，
       文案取自 nano 的 `po/`（新增 6 个 `global-*` 键，en/zh 同步），
       并把消息倒计时设为 1。
    5. 补齐 C 内置表里存在、此前完全缺失的 Alt 绑定（30 条）：
       段落/块跳转 `M-9 M-( M-0 M-) M-7 M-8`、`M-Space`（上一词）、
       `M-G`、`M-\` / `M-/`（首末行）、`M-6` / `M-^`（复制，MMOST）、
       `M-T`（剪到文件尾）、`M-3`（注释）、`M-%`（列循环）、
       `M-:` / `M-;`（宏录制/回放），以及外观/行为开关组
       `M-Z M-C M-S M-$ M-N M-# M-P M-Y M-H M-I M-K M-L M-O`。
    6. 输入层功能键分支此前丢弃 Alt：`Insert`/`Delete`/`PageUp`/`PageDown`
       不判断 alt，导致 `M-Ins`/`M-Del`/`M-PgUp`/`M-PgDn` 四条绑定拿不到
       键码（`M-Home`/`M-End` 因走 `KeyCode::Home/End` 分支而幸免）。
       现按 C 的 `parse_escape_sequence`（winio.c:647/679/707/731/757）
       补齐 `ALT_INSERT`/`ALT_DELETE`/`ALT_PAGEUP`/`ALT_PAGEDOWN`，以及
       Shift+Alt 的翻页变体。
    7. 修掉两处臆造/死值：`Alt+h` 曾被特判成 `KEY_HOME`（C 无此规则，
       导致 `M-H` 智能主页开关不可达）；`ALT_BACKSPACE`(0x40A) 是我方
       自造常量、输入层永不产出，而 C 的 `M-Bsp` 键码其实是
       `CONTROL_SHIFT_DELETE`（global.c:1352），且 `parse_kbinput` 对
       「ESC + Backspace」确实返回该值（winio.c:1049），现两处对齐。
    差异（未实现，非缺陷）：C 的 `M-&`（`show_curses_version`）在
    `ENABLE_EXTRA && NCURSES_VERSION_PATCH` 下才编译，报告的是 ncurses
    版本；本项目用 crossterm，无对应物，且没有可忠实转写的取值，
    故不臆造绑定。另外 C 的 4 条 UTF-8 三角别名
    （`M-◢◣▴▾`，键码与 `M-Left/Right/Up/Down` 完全相同）未移植。
    **既有映射偏离（本次只让 Alt 能被识别，未改以下选择）**：
    - 主菜单里单独按 `Esc`：C 会显示 `Unbindable key: ^[`
      （`process_a_keystroke` 的 `input < 0x20` → `unbound_key`），
      而本项目在 `execute_function` 的兼容分支里提前 `return true`
      静默忽略（winio.rs 的「忽略单独的 Esc」）。重写的 `unbound_key`
      已含该分支，但要生效需删掉那条硬编码，属行为变更，故仅登记。
    - `M-Del`：C 绑 `zap_text`（global.c:1355），Rust 绑 `DoCut`
      （shortcut_init 的 mmi 组）；且 `FunctionId::Zap` 目前没有
      `execute_by_id` 分发，绑过去也只是无操作。
    - `M-Left` / `M-Right`：C 在 MMAIN 绑 `switch_to_prev/next_buffer`
      （global.c:1393-1394），Rust 绑上一词/下一词。注意 C 另有
      NANO_TINY 分支把 `M-D`/`M-N` 绑到词移动，可能是本项目的取材来源。
      改这两条会连带影响 `tests/alt_keys.rs` 的期望，需要产品决策。
    证据：`tests/alt_keys.rs`（18 项：逐键名验证"真实按键产出的键码
    能命中期望功能"、大小写折叠、`shifted_metas` 区分、rc 解析值与
    运行时一致、绑定表无 0/负键码、`meta_key` 不残留、未绑定组合的
    提示文本、端到端 Alt+a/Alt+u 真的改状态、Alt+非 ASCII 不注入）、
    `tests/alt_in_prompt.rs`（3 项）。
    **未在真实终端里手工验证**：本环境无可用 TTY，键码链路靠
    `translate_keycode(真实 KeyEvent)` 驱动，等价于 crossterm 上报给
    编辑器的字节流，但 ESC 前缀与转义序列的**实际抓包行为**依赖终端，
    仍需用户在真实终端复核。
17. ✅ **`M-6` 复制标记区域会损坏复制内容已修复**：`copy_marked_region`
    （cut.c:607-639）没有照 C 版临时把首行前移 `top_x`、把末行在 `bot_x`
    处截断，再对整条链表调用 `copy_buffer`，而是自行拼了一个只含首行片段
    的单节点：
    - 单行选区复制成 `"world\0"`，多出一个 NUL 字节；
    - 跨行选区只复制首行的选区片段（选 `one`/`two`/`three` 的 `ne`..`th`
      只得到 `ne`）。
    无标记时 `M-6` 复制整行走的是 `copy_text` 的另一分支，仍然正常，所以
    用户看到的现象是"选中文本后按 Alt+6 复制不出来"。
    现逐句按 C 重写：`botline->next = NULL`、
    `botline->data[bot_x] = '\0'`（Rust 的 `String` 不能只改一个字节，改为记下
    完整内容后整体恢复，效果等价）、`topline->data += top_x`，
    `copy_buffer` 之后再恢复原状；`copy_text` 的其余逻辑未改。
    证据：`tests/copy_text.rs`（当前行、单行选区、跨行选区、复制后粘回原处
    四项；均通过真实的 Alt+6 `KeyEvent` → `translate_keycode` →
    `handle_input_key` 分发）。已在 Windows 实机验证修复有效（用户确认）；
    Linux 终端未实机验证。
18. ⚠️ **rcfile 的语法加载时机与 C 版不同（有意偏离，静态阅读对照）**：
    - **急切全量加载**：C 版两阶段——启动只做 intro 扫描（rcfile.c
      `parse_rcfile(..., FALSE, TRUE)`），打开文件命中某语法时才
      `parse_one_include(sntx->filename, sntx)` 懒加载其颜色规则
      （color.c:225-228）。Rust 版在 `parse_rcfile` 里 intro 扫描后立即对
      每个新语法做第二遍完整解析，`filename` 恒置 None（color.rs 注释）。
      可见影响：语法文件里的非法正则在**启动时全部报错**（C 版只报实际
      用到的语法）；启动开销随语法文件规模线性增长。
    - **glob 无匹配/不可读目录静默跳过**：C 版 `glob(GLOB_ERR|GLOB_NOCHECK)`
      遇不可读目录报 "Error expanding"（rcfile.c:993-1001）；Rust 版
      `read_dir` 失败即忽略该段（rcfile.rs `expand_glob_dir`），无匹配时
      GLOB_NOCHECK 语义（返回原模式）已对齐。Windows 目录权限语义不同，
      静默降级算合理降级，但未对照原版。
    - **UTF-8 校验范围更宽**：C 版只对 `set` 选项参数做 `mbstowcs` 校验
      （rcfile.c:1584-1590）；Rust 对整行做 `from_utf8` 预检
      （`had_invalid`），行内任何非法字节都会拒绝参数。极端行上错误分类
      可能不同，属实现层保守化。

19. ✅ **帮助页的快捷键描述已翻译**（对应用户报告的症状
    `^G              (F1)help_gist`）。对照 `help.c` 的 `help_init`：
    C 版是 `sprintf(ptr, "%s\n", _(f->phrase))`，即每条描述都要过
    gettext；本移植的 `FuncStruct::phrase` 存的是 ftl 键名
    （`help_gist` 之类），`help_init` 却直接 `push_str(&phrase)`，于是
    键名原样上了屏幕。同时 `locales/*.ftl` 里**根本没有**这些描述条目，
    等于没有译文可查。
    修法：
    - `help_init` 改用新增的 `i18n::tx_or_msgid(f_ref.phrase)` 查表。
      该函数按 gettext 语义处理——键不存在时返回 msgid 原文，而不是
      `[[key]]` 占位；这样帮助查看器自身那两条 `phrase = "x"` 的条目
      （C 里本就不该翻译）不会显示成 `[[x]]`。
    - 键名统一改为 `gist-*`，并把 `global.c` 里 104 条 `*_gist` 的 msgid
      逐条加入 `en-US.ftl`，译文逐条取自 `po/zh_CN.po`（如 `help_gist`
      → `gist-help` = 「显示帮助」；Rust 侧命名与 C 的对应关系：
      `casesens`→`case`、`backwards`→`reverse`、`dosformat`→`dos`、
      `insert`→`readfile`、`location`→`cursorpos`、`complete`→`completion`、
      `linter`→`lint`、`prevbuf`/`nextbuf`→`prevfile`/`nextfile`、
      `flip*`→对应被翻转的原 gist、`left`/`right`→`back`/`forward`）。
      Rust 独有的 `gist-tosearch`（MGOTOLINE 的 To Search）沿用
      `locales` 里原有的「跳至文字」。
    - 帮助列表列宽按 C 的注释「Make the first column 7 cells wide and the
      second 10」重排（此前两键条目用 `{:<16}` 再拼 `(键)`，少一格；
      单键条目描述提前到第 11 列）。修正后与原版逐字对齐：
      `^G     (F1)      Display this help text`。
    证据：`tests/help_viewer.rs`（描述已翻译/无键名泄漏/列宽）、
    `tests/help_i18n.rs`（zh-CN 下取 po 译文）。静态阅读 + 单元测试，
    未做交互式手工验证。

20. ✅ **帮助页不再画行号**（对应用户报告的症状：编辑器开了行号时，
    帮助页每行前面出现 `30`、`31` 这样的行号）。对照 `help.c` 的
    `show_help`：C 版是
    `was_margin = margin; editwincols = COLS - sidebar; margin = 0;`
    （help.c:402、437-439），退出时
    `margin = was_margin; editwincols = COLS - margin - sidebar;`
    （help.c:544-545）。本移植这两组动作都没实现，`margin` 只由主循环的
    `confirm_margin()` 改写，于是帮助页沿用了编辑器的行号边距。
    另有一层原因：绘制时用的是"每次按行数重算"的边距，即使把 `margin`
    置 0 也不会反映到屏幕上。
    修法：
    - `show_help` 补齐上述四行的保存 / 清零 / 恢复（含 `editwincols`
      同步，否则折行宽度也会跟着带边距）。
    - 绘制改用保存值：新增 `winio::margin()`（对应 C 的全局 `margin`），
      `place_the_cursor`、`draw_row`、`update_line`、
      `update_softwrapped_line`、鼠标点击换算都改读它；
      `current_margin()` 保留为 `confirm_margin()` 的"应有值"计算，
      语义与 C 的 `digits(filebot->lineno) + 1` 一致。
    证据：`tests/help_viewer.rs`（`drawing_margin_follows_saved_value`、
    `help_viewer_has_no_line_number_margin`）。另在 Windows 上以 `tests/`
    之外的临时用例直接调用 `show_help()`（注入 `^G` 退出）并抓取终端
    输出确认：帮助页各行均从第 1 列开始（无 `30`/`31` 行号），退出后
    编辑器行号恢复；该临时用例已删除，不是长期测试。Linux 未实机验证。

21. ✅ **帮助列表去掉 C 版没有的条目**。本移植的 `add_to_funcs` 有若干
    重复登记 / 错置分支，使主帮助页出现原版没有的行：
    - `add_to_funcs(DoRefresh, MMAIN|MREPLACE, "Refresh", "x", false)`
      在 C 里位于 `#ifndef ENABLE_HELP` 分支（global.c:781-786）——只有
      不编入帮助时才需要它撑住底栏。本移植始终启用帮助，故删除；否则
      帮助页多出一条描述为 "x" 的条目。
    - `shortcut_init_rest` 末尾重复登记了 `GetOlderItem`/`GetNewerItem`
      （MMAIN）、`ToFirstFile`/`ToLastFile`（MBROWSER）与
      `DoVerbatimInput`（MMAIN），这些函数在上方已按 C 的顺序登记过，
      于是帮助页出现重复的 `M-V`、`First File`/`Last File` 行。只保留
      C 里确实需要的 `GotoDir`（MBROWSER）。
    - `DoAnchor` 不是 C 的函数（C 只有 `put_or_lift_anchor`），此前与
      `PutOrLiftAnchor` 各登记一条，帮助页多出一行重复的
      "Place or remove an anchor at the current line"。删除 `DoAnchor`
      条目（`execute_by_id` 里的分发保留，rc 里 `bind ... anchor` 仍走
      `PutOrLiftAnchor`）。
    证据：`tests/help_viewer.rs`（`main_help_has_no_placeholder_refresh_entry`、
    `main_help_has_no_duplicate_entries`）。静态阅读 `global.c:761-1234`
    逐条比对。未做交互式手工验证。

22. ✅ **新增：读取程序同目录的 nanorc**（nanoxide 扩展，有意偏离原版；
    对应用户要求）。原版 9.2 的 `do_rcfiles`（rcfile.c:1727-1758）只读
    系统级 `SYSCONFDIR/nanorc` 与用户级 `~/.nanorc` / XDG。本移植在
    系统级之后、用户级之前增加一个候选——可执行文件所在目录下的
    `nanorc`（Windows 下即 `nax.exe` 旁的 `nanorc`）：
    - 顺序：`-f` 指定 → 系统级 → **程序同目录** → 用户级（三选一）；
    - 读到后**继续**读用户级，用户配置可覆盖程序目录配置（分层语义）；
    - `-I` 仍然完全不读任何 rc；`-f` 仍然只读指定文件；
    - 路径由 `std::env::current_exe()` 的父目录拼接 `nanorc` 得到
      （rcfile.rs `exe_dir_nanorc`），取不到时静默跳过；Linux 上符号链接
      已由内核解析为真实可执行文件路径；
    - 文件合法性沿用 `is_good_file`（可读、非目录/设备），与系统级一致。
    顺带修复一个移植遗漏：用户级三选一的 `have_nanorc()` 检查之后原本
    没有调用 `parse_one_nanorc()`（C 版是
    `if (have_nanorc(...) || ...) parse_one_nanorc();`），导致 `~/.nanorc`
    只被探测、从未解析。新用例“程序目录 + 用户级都要生效”暴露了它，
    现已补上，行为与 C 版一致。
    证据：`tests/exe_nanorc.rs`（把构建出的 nax 复制到临时目录，旁边放
    `nanorc` 后以 `-z` 运行，断言语法被列出；另覆盖“没有程序目录
    nanorc 时不误读”和“程序目录之后仍读 `~/.nanorc`”）。已在 Windows
    实机跑通；Linux 未实机验证。

---

## 五、功能模块状态

### 1. 信号与终端（`signals.rs`）

- ✅ `set_up_signal_handlers` / `set_up_sigwinch_handler` / `block_sigwinch`（Unix；handler 只置原子标志，主循环消费）
- ✅ SIGHUP / SIGTERM 紧急保存退出；SIGWINCH 重绘；SIGTSTP / SIGCONT 挂起与恢复
- ✅ `install_handler_for_Ctrl_C` / `restore_handler_for_Ctrl_C`
- ✅ `make_a_note`：Ctrl+C 读取期间只置原子标志（不请求退出），由读取
  循环通过 `take_ctrl_c_pressed` 消费。
- ✅ `reconnect_and_store_state`：Unix 打开 `/dev/tty` 并 `dup2` 到
  `STDIN_FILENO`；Windows 用 `CONIN$` + `SetStdHandle`（⚠️ 未在 Windows
  实机验证“管道读入后回到键盘”的完整交互）。
- ✅ `handle_crash`：SIGSEGV/SIGABRT 处理器先恢复默认动作，再置崩溃码与
  终止标志；主循环执行 `emergency_save_all`、删除锁文件并退出
  （对应 C 的 `die` 路径）。
- ✅ 鼠标：`mouse_init`（启停 crossterm 捕获）、编辑区左键点击定位
  （`handle_mouse_click`）、底部栏点击执行快捷键（`footer_index` /
  `shortcut_entries`，对应 C 的 `get_mouseinput` footer 分支）、滚轮上下
  滚动两行；`tests/mouse_support.rs`、`tests/mouse_footer.rs` 覆盖。
  ✅ `-q` 侧边滚动条点击跳行（C 的 sidebar 分支）：主菜单下点击编辑区
  最右列，按 `filebot->lineno * mouse_y / editwinrows + 1` 换算目标行，
  再 `goto_line_and_column(..., TRUE)`；`tests/mouse_scrollbar.rs` 覆盖
  顶部、中部、底部与越界钳制。
  另修正鼠标事件处理后的返回值：已直接执行的动作返回
  `FOREIGN_SEQUENCE`，不再返回 `1`（否则主循环会把它当作 `^A`）。

### 2. 文件（`files.rs`）

- ✅ `open_buffer` / `save_to` / `do_writeout` / `make_new_buffer` / `close_buffer` / `emergency_save_all`
- ✅ `open_buffer` 的两种路径与 C 一致：新缓冲区（`fresh=true`，对应
  `open_buffer(..., TRUE)`）才做 `has_valid_path`、才创建锁文件、文件不存在
  时按新文件处理；非新缓冲区打开不存在的文件报
  `File "<name>" not found`（`ErrorRead`），不建缓冲区、不创建锁
  （`tests/open_buffer.rs`、`tests/lockfile_lifecycle.rs`）。
- ✅ `read_file` 的 "Read N lines" 提示：按 C 的 `num_lines` 计行（空文件
  为 0），DOS 格式用 "(converted from DOS format)" 变体；打开文件在
  ZERO/MINIBAR 下静默、插入文件时仍显示（`tests/read_message.rs`）。
- ✅ 打开文件时检查可写性：不可写则优先提示
  `File '<name>' is unwritable`（Unix 用 `access(W_OK)`，Windows 用追加
  打开探测；查看模式不检查；`tests/read_message.rs`）。
- ✅ 打开文件时的类型检查：目录、字符/块设备（Unix）、以分隔符结尾却
  不存在的无效文件名都会被拒绝并提示；根用户以新缓冲区打开无写位的文件
  时提示 `'<name>' is meant to be read-only`（对应 C 的 `open_buffer`
  检查；`tests/open_buffer.rs`）。
- ✅ `has_valid_path`：新缓冲区打开前检查父目录是否存在/是目录/可进入
  （`access(X_OK)`）/可写（不可写只在 LOCKING 且非查看模式时给 MILD
  提示），当前工作目录消失时提示；无效时不创建缓冲区（`tests/open_buffer.rs`）。
- ✅ 命令行启动：每个文件都按 C 的 `open_buffer(filename, TRUE)` 打开
  （新缓冲区 + `has_valid_path`），首个文件的父目录无效时同样拒绝，
  不会残留空缓冲区（`tests/open_buffer.rs`）。
- ✅ `execute_command` / `insert_a_file_or` / `do_insertfile` / `do_execute`（用 `std::process::Command` 替代 fork/pipe，支持 `|cmd` 与 `||cmd`）
  插入/执行提示支持 `^X`/`M-F`/`M-N`/`M-\`/`^T` 翻转键（`tests/insert_flip.rs`）。
  插入到新缓冲区改用 `open_another_buffer`（保留当前缓冲区，此前会替换它），
  失败时不误清原缓冲区的修改标记（`tests/open_buffer.rs`）。
- ✅ 锁文件（`write_lockfile` / `delete_lockfile` / `lock_filename_for`，1024 字节 vim/nano 兼容格式）
- ✅ 多缓冲区切换（`M-,` / `M-.`）
- ✅ `make_backup_of`：默认生成 `文件名~`；设置 `backupdir` 时在指定目录
  生成编号备份（完整路径的分隔符换成 `!`，Windows 的盘符冒号也替换），
  `INSECURE_BACKUP` 时截断覆盖、否则用 `O_EXCL`；Unix 下备份先按 0600
  创建、再复制原文件权限。简化：省略 C 在失败后重试到 HOME 目录的逻辑
  （`tests/write_methods.rs`）。
- ✅ `save_to` 支持覆盖/追加/前置（`WritingType`），行尾按 DOS/Unix 格式转换；
  `do_writeout` 的提示循环支持 M-A/M-P/M-D/M-B/^Q/^T；`exiting` 为真时
  忽略标记、整块保存（对应 C 版 `write_it_out` 的 `exiting` 参数）。
  证据：`tests/file_format.rs`、`tests/write_methods.rs`
- ✅ 写入反馈与落盘：正常保存前显示 "Writing..."；目标是命名管道时显示
  "Writing to FIFO..."，读取命名管道时显示 "Reading from FIFO..."；写入后
  用 `sync_all` 确保数据落盘（FIFO 跳过，对应 C 的 `fflush + fsync` 与
  `!S_ISFIFO(...)` 条件）；SPECIAL（紧急保存）新建文件权限为 0600
  （C 的 `S_IRUSR|S_IWUSR`）；磁盘满（Unix ENOSPC / Windows
  ERROR_DISK_FULL）时额外警告文件可能已被截断并提示恢复步骤，同时刷新
  statinfo（`tests/write_methods.rs`）。
- ✅ `write_region_to_file`（写选区，`tests/write_region.rs`；区域也可追加/前置）、`do_savefile`
- ✅ `scoop_stdin`（命令行 `-`，`tests/stdin_args.rs`）：读取前恢复终端；
  stdin 是终端时提示用 ^D 结束；读取期间 ^C 只中止读取，结束后显示
  "Interrupted"（`read_stdin_bytes` 有单元测试）；从管道读入后用
  `reconnect_and_store_state` 把 stdin 接回键盘。
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
- ✅ 搜索/替换选项键：`M-C` / `M-R` / `M-B` 分别切换大小写/正则/反向；
  whereis 提示里 `^R` 切换到替换（`FlipReplace`，对应 `search.c:150-155`），
  不再误绑成正则（`tests/keymap_reachability.rs`）。
- ✅ 提示历史与首/末文件键：六个提示菜单的 `^P`/`^N`/`Up`/`Down` 切换历史；
  查找文件提示的 `^Y`/`^V` 与浏览器的 `M-\`/`M-/` 跳到首/末文件
  （`global.c:884-895、1529-1533`；`tests/keymap_reachability.rs`）。
- ✅ `replace_line` / `findnextstr` 的匹配长度取正则的真实匹配
  （`regmatches[0]` 的 `eo - so`）；`regexp_init` / `regexp_init_real`
  按 ERE 编译（此前误把正则当字面量/glob），错误时显示带原因的
  `Bad regex`。测试见 `tests/regex_replace.rs`。
- ✅ `replace_regexp` 的子表达式引用（`\1`..`\9`）：自研引擎现返回捕获组
  边界，`regexp_match_len` 在每次正则命中后填写 `regmatches`；`replace_line`
  在 `USE_REGEXP` 下用展开后的替换文本。组号超过 `re_nsub` 或该组未参与
  匹配时按字面量保留。证据：`tests/regex_replace.rs`（展开、越界、非正则
  回退）与 `regex.rs` 的捕获组单元测试。
- ✅ 锚点：`text.rs` 的 `do_anchor` / `to_prev_anchor` / `to_next_anchor`
  由 `execute_by_id` 分发；`M-Ins` / `M-PgUp` / `M-PgDn` / `M-"` / `M-'`
  已按 `global.c:1356-1362` 绑定，`M-Home` / `M-End` 绑定到
  `to_top_row` / `to_bottom_row`（`tests/keymap_reachability.rs`）。
  `search.rs` 没有 `put_or_lift_anchor` 同名函数（C 的锚点逻辑已拆到
  `text.rs`）。

### 5. 文件浏览器（`browser.rs`）

- ✅ `browse` / `browse_in` / `read_the_list` / `reselect` / `findfile` / `search_filename` / `research_filename` / `to_first_file` / `to_last_file`
- ✅ 键位按 `global.c:1537-1556` 对齐：`^T` 退出浏览器、`Home`/`End`/
  `^Home`/`^End` 跳首/末文件、`^/` 与 `M-G` 切换目录、`Space` 下一页、
  `Bsp`/`Sh-Del` 上一页；移除误绑的 `^H`（原是切换目录）与 `^S`（原是 Enter）。

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
- ⚠️ `get_mouseinput`：编辑区点击、底部栏点击、滚轮与 `-q` 滚动条点击跳行
  已实现（`handle_mouse_event`，见第五节第 1 部分）；提示栏的点击定位
  （C 版 prompt.c `process_prompt_click`：按点击列换算 `typing_x`）未移植，
  提示期间点鼠标不会把编辑位置移到点中的字符处
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
- ✅ `--listsyntaxes`（`list_syntax_names`）；`-h` 的选项表已按 9.2 的 `usage()` 重写，
  且全部说明文本走 `t!("cmdline-*")`：英文取 po 的 msgid，中文逐字取
  `nano/po/zh_CN.po` 的 msgstr；`print_opt` 的列对齐用 `utils::breadth`
  （wcwidth 语义，中文旗标如 `-C <目录>` 不再错位），带参数的短/长旗标
  （`-C <dir>`、`--quotestr=<regex>` 等）在 po 里是独立 msgid，同样走 `t!`
  （`tests/command_line.rs::binary_help_is_translated`，实际运行验证）
- ✅ BOM 提示：光标停在 UTF-8 BOM 上时显示 "Byte Order Mark"，倒计时 1
  （`winio::cursor_is_on_bom` / `set_blankdelay_to_one`，`tests/bom_notice.rs`）
- ✅ `ZERO` 模式的布局与重绘：无标题栏、编辑区延伸到状态行
  （`compute_editwinrows`/`toprows_for`/`bottomrows_for`），状态行位置随
  底部栏行数变化；主循环在有消息时重绘整屏、次要消息时重绘编辑区
  （`tests/zero_mode.rs`）。简化：C 只重画状态行/最后一行，这里用整屏
  重绘代替（结果相同，多画几行）。
- ✅ `-h` 输出与 C 版 `usage()` 的文案对齐：所有 msgid（含带参旗标）已接入
  i18n；与 C 版的残余差异仅两点——程序名显示为 `nax`（C 显示 `nano`）、
  无 libmagic 故无 `-!`/`--magic` 行（与未编译 libmagic 的 nano 一致）

### 8. 覆盖较完整的模块

`cut.rs`、`chars.rs`、`movement.rs`、`history.rs`、`help.rs`、`color.rs`、`utils.rs`、`rcfile.rs` 的函数基本对等。`history.rs` 的位置寄存器（`-P`）已接入打开/关闭流程，并修正了 Windows 绝对路径的解析。`rcfile.rs` 当前约 1640 行；另新增程序同目录 nanorc 读取（见四.22），并修复了用户级 rc 漏调用 `parse_one_nanorc()` 的移植遗漏。`chars.rs` 比原版多 `mbstrcasestr` / `revstrstr`，`movement.rs` 多 `do_first_line` / `do_last_line`。

---

## 六、总结

nanoxide 已移植 nano 绝大多数编辑功能，且有 Windows 支持。当前最需要处理的是：

1. 命令行选项已全部处理：`-q`/`-%` 已接线，`-!` 因无 libmagic 被明确拒绝。
2. `execute_by_id` 已审计：主菜单绑定全部可达；未分发的 rc 占位函数会显示
   “Function not implemented”而不是静默吞键。写文件提示的
   DosFormat/BackItUp/Append/Prepend/Discard 循环已移植。
3. 显示层基本完成：`do_credits` 的动画未在真实终端里手工验证；锚点键位已按
   `global.c:1356-1362` 绑定（见第五节搜索条目）。剩余缺口：提示栏的鼠标
   点击定位（C 版 `process_prompt_click`）未移植。
4. i18n 只有两种语言；单复数已用双键处理。
5. 按键链路已补齐 Alt（`M-`）：绑定表键名回退、键码约定统一、`meta_key`
   置位、30 条缺失的 C 内置 Alt 绑定、功能键的 Alt 变体、以及未绑定
   组合的 `Unbound key: M-X` 反馈（见第四节第 16 条）。仍未在真实终端
   手工验证。

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
- 本轮：实现 `--modernbindings` 的主菜单键位表（`shortcut_init` 按标志
  分支，modern 下 `execute_function` 先查表），并补它的绑定与分发测试。
- 本轮：legacy 的 `^Z` 改为 `suggest_ctrlT_ctrlZ`（仅当 `^T` 是 Execute
  且执行菜单 `^Z` 是 Suspend 时提示），补执行菜单的 `^Z` → Suspend 绑定
  与 `global-suspend_hint` 翻译；`execute_by_id` 接入 `SuggestSuspend`。
- 本轮：执行菜单（MEXECUTE）补齐 `^S`/`^T` 拼写、`^O` 格式化、`^V`
  剪切到末尾、`^J` 全文对齐等快捷键，与 `global.c` 对齐；
  `tests/keymap_reachability.rs` 增加对应断言。
- 本轮：搜索/替换菜单选项键修正——`M-C`/`M-R`/`M-B` 可达（此前缺失），
  whereis 的 `^R` 改为 `FlipReplace`（此前误绑正则），`search_init` 处理
  `FlipReplace` 切换提示；`tests/keymap_reachability.rs` 增加断言。
- 本轮：提示历史与首/末文件键补齐——六菜单的 `^P`/`^N`/`Up`/`Down`
  历史翻页、`MWHEREISFILE` 的 `^Y`/`^V`、浏览器的 `M-\`/`M-/`；
  `tests/keymap_reachability.rs` 增加逐菜单断言。
- 本轮：浏览器菜单键位按 `global.c:1537-1556` 对齐（`^T` 退出、
  `Home`/`End`/`^Home`/`^End` 首末文件、`^/`/`M-G` 切换目录、`Space`
  翻页、`Bsp`/`Sh-Del` 上一页），移除误绑的 `^H`/`^S`；写/插入提示的
  `^T` 统一绑到浏览。
- 本轮：帮助查看器与滚动键对齐——`^G` 退出、首末行键、`Space` 翻页；
  `M-Up`/`M-Down`/`M--`/`M-_`/`M-+`/`M-=` 改为滚动屏幕（原先
  `M-Up`/`M-Down` 被硬编码成段落移动）；`tests/keymap_reachability.rs`
  增加断言。
- 本轮：补 `M-X`（切换帮助行，`DoToggle NO_HELP`）绑定，帮助查看器内
  不绑定；`tests/keymap_reachability.rs` 验证绑定与切换。
- 本轮：写文件补 FIFO 识别与 "Writing to FIFO..." 提示，SPECIAL（紧急
  保存）新建文件权限收紧为 0600（Unix）；`tests/write_methods.rs` 增加
  FIFO 识别与权限断言（仅 Unix 运行）。
- 本轮：`open_buffer` 补齐类型检查——设备文件（Unix）与以分隔符结尾的
  无效文件名会被拒绝（新增 `OpenBufferResult::Unsupported` 与翻译键）；
  `tests/open_buffer.rs` 增加用例。
- 本轮：补根用户只读提示（`should_warn_read_only`）与 `has_valid_path`
  的父目录检查（不存在/不是目录/不可写），无效时不创建缓冲区；
  `tests/open_buffer.rs` 增加判定与父目录检查用例。
- 本轮：`has_valid_path` 补齐 `access(X_OK)` 与“工作目录已消失”检查
  （Unix），新增 `files-path_not_accessible` / `files-working_dir_gone`
  翻译；`tests/open_buffer.rs` 增加不可进入目录用例（root 下跳过）。
- 本轮：命令行首个文件改用 `open_another_buffer`（等价 C 对每个文件都传
  TRUE），使 `has_valid_path` 对首个文件也生效；补测试验证失败后不残留
  空缓冲区。
- 本轮：`insert_a_file_or` 的“插入到新缓冲区”也改用 `open_another_buffer`
  （此前会替换当前缓冲区），并只在成功载入/新建时清修改标记；
  `tests/open_buffer.rs` 增加旧缓冲区保留的测试。
- 本轮：打开命名管道时显示 "Reading from FIFO..."（与写入侧的提示对称），
  新增 `files-reading_from_fifo` 翻译。
- 本轮：`open_buffer` 对齐 C 的两条路径——非新缓冲区打开不存在的文件报
  `File "<name>" not found`；锁文件只在 `new_one`（`fresh`）时创建。相应
  把测试里“打开不存在文件”的场景改用 `open_another_buffer`，并新增
  not-found 断言。
- 本轮：`make_backup_of` 补齐 `backupdir` 编号备份、`INSECURE_BACKUP`、
  0600 创建与权限复制（Windows 处理 `\\?\` 前缀与盘符冒号）；新增
  `files-cannot_read_original`；`tests/write_methods.rs` 增加默认/目录/
  insecure 备份测试。
- 本轮：正则搜索/替换修正——`regexp_init`/`regexp_init_real` 改按 ERE
  编译（此前按字面量/glob），`findnextstr` 与 `replace_line` 用真实匹配
  长度；`search-bad_regex` 补错误原因；新增 `tests/regex_replace.rs`。
- 本轮：主循环补 BOM 提示（光标在行首且该行以 EF BB BF 开头、UTF-8 模式
  显示 "Byte Order Mark"，倒计时 1）；新增 `tests/bom_notice.rs`。
- 本轮：ZERO 模式布局——`compute_editwinrows`/`toprows_for`/`bottomrows_for`
  按 C 的 `window_init` 规则计算（无标题、编辑区延伸、NO_HELP 缩底部），
  状态行位置与编辑区起点随之调整，主循环补 ZERO 消息重绘；新增
  `tests/zero_mode.rs`。
- 本轮：regex 引擎把 `\<`/`\>`/`\b` 拆成词首/词尾/任意边界三种节点
  （此前三者等价），并用 GNU grep 交叉验证；修正依赖旧行为的
  `trailing_paren_is_literal` 断言。
- 本轮：鼠标补底部栏点击（`footer_index`/`shortcut_entries`，快捷键执行）
  与滚轮滚动两行（`handle_mouse_event`）；底部栏绘制复用同一份条目收集；
  新增 `tests/mouse_footer.rs`。
- 本轮：`-q` 侧边滚动条点击跳行已接线（对应 `get_mouseinput` 的 sidebar
  分支：主菜单下点击编辑区最右列，按 `filebot->lineno * mouse_y /
  editwinrows + 1` 换算并 `goto_line_and_column(..., TRUE)`）；同时把鼠标
  点击底部栏、滚轮处理后返回的 `1` 改为 `FOREIGN_SEQUENCE`，避免主循环
  把它当成 `^A` 再执行一次；新增 `tests/mouse_scrollbar.rs`。
- 本轮：正则替换支持 `\1`..`\9` 子表达式——`Regex` 记录捕获组边界
  （`nsub`、`find_bytes_captures`），`regexp_match_len` 填写 `regmatches`，
  `replace_line` 展开替换文本；组号越界或组未参与匹配时保留字面量。
  新增 `tests/regex_replace.rs` 用例与 `regex.rs` 的捕获组单元测试。
- 本轮：补齐 stdin 读取的 Ctrl+C/终端处理——`make_a_note` 只置中断标志，
  `scoop_stdin` 读取前 `restore_terminal`、读取期间可被 ^C 中止并在结束
  后显示 "Interrupted"，从管道读入后 `reconnect_and_store_state` 把 stdin
  接回键盘（Unix `/dev/tty`，Windows `CONIN$`）；新增 `files.rs` 的
  `read_stdin_bytes` 单元测试与 Unix 信号测试。
- 本轮：修复 Shift+方向键选择——`translate_keycode` 把带 Shift 的
  方向/Home/End/PgUp/PgDn 解析为普通 KEY_* 并置 `shift_held`，
  `handle_input_key` 按 `process_a_keystroke` 建立/丢弃软标记；此前
  Shift+Up/Down 会插入 `SHIFT_UP`/`SHIFT_DOWN` 码点（显示为乱码）。
  新增 `tests/shift_selection.rs`。
- 本轮：标题栏超长文件名截断改按 UTF-8 字符边界，避免中文路径触发
  `byte index ... is not a char boundary` 崩溃；`tests/titlebar_ranking.rs`
  增加回归测试。
- 本轮：选中文本高亮——`draw_row` 末尾按 C 版重绘
  `interface_color_pair[SELECTED_TEXT]` 覆盖标记区域，
  `selection_paint_range` 处理跨行/水平滚动/软换行求交；
  `hilite_attribute` 默认改为 `A_REVERSE`（C 的 global.c 默认），
  `--boldtext` 仍为 `A_BOLD`。新增 `selection_paint_range` 用例。
- 本轮：保存的覆盖确认已接线（`do_writeout` 对应 C 版 `write_it_out` 的
  `method == OVERWRITE` 分支）——`needs_name_warning` 判定改名/无名覆盖、
  `confirm_namechange` 询问改名、`overwrite_question` 询问覆盖（名字按
  `crop_to_fit` 截到 COLS）、`disk_changed_since_open` 比对 mtime 决定是否
  提示"磁盘已改变"；受限模式只警告不允许覆盖/改名；常规覆盖保存后刷新
  `statinfo`（对应 C 的 `stat_with_alloc`）；`-t` 且有名字时跳过文件名提示。
  新增 5 个 `files-*` 翻译键（en/zh）与 `tests/overwrite_confirm.rs`。
  差异：不比较 `st_dev`/`st_ino`（跨平台取不到），只比 `st_mtime`；
  交互循环本身需真实终端，未手工验证。
- 本轮：Alt（`M-`）组合键在 Linux 与 Windows 上一律失效已修复（见第四节
  第 16 条）。`add_to_sclist` 补 C 的键名回退；`alt_keycode_raw` /
  `alt_keycode_for` / `runtime_alt_keycode` 统一绑定表、rc `bind`、输入层
  三方的键码约定；`meta_key` 真正置位/清除；未绑定组合改走重写的
  `unbound_key`（新增 6 个 `global-*` 键，en/zh）；补 30 条 C 内置表里
  存在而本项目缺失的 Alt 绑定（另修正 `M-V` 的键码与 `M-Bsp` 的写法）；
  输入层补 Alt+Insert/Delete/PageUp/
  PageDown 与 Alt+Backspace；移除 `Alt+h → KEY_HOME` 与
  `ALT_BACKSPACE = 0x40A` 两处臆造。
  新增 `tests/alt_keys.rs`（18 项）与 `tests/alt_in_prompt.rs`（3 项）；
  `tests/rcfile_compat.rs` 里"`bind M-T` 键码应为 116"的断言原样固化了
  上述约定分叉缺陷，改为断言"rc 解析值必须等于输入层实际产出值"。
  `M-Del`、`M-Left`/`M-Right` 的目标函数与 C 不一致属既有选择，仅登记
  未改。真实终端下的抓包行为仍需用户复核。
- 复查：文档统计更新为 `src/*.rs` 约 2.7 万行、`tests/` 51 个文件、
  `rcfile.rs` 约 1611 行；第五节显示部分修掉一条过时描述（`-q` 滚动条
  点击跳行其实已完成），改为记录真正的剩余缺口 `process_prompt_click`；
  第六节总结里"锚点键位尚未绑定"也已过时，改为指向已完成的记录。

- 本轮：保存的覆盖确认已接线（`do_writeout` 对应 C 版 `write_it_out` 的
  `method == OVERWRITE` 分支）——`needs_name_warning` 判定改名/无名覆盖、
  `confirm_namechange` 询问改名、`overwrite_question` 询问覆盖（名字按
  `crop_to_fit` 截到 COLS）、`disk_changed_since_open` 比对 mtime 决定是否
  提示"磁盘已改变"；受限模式只警告不允许覆盖/改名；常规覆盖保存后刷新
  `statinfo`（对应 C 的 `stat_with_alloc`）；`-t` 且有名字时跳过文件名提示。
  新增 5 个 `files-*` 翻译键（en/zh）与 `tests/overwrite_confirm.rs`。
  差异：不比较 `st_dev`/`st_ino`（跨平台取不到），只比 `st_mtime`；
  交互循环本身需真实终端，未手工验证。