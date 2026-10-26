# AGENTS.md —— AI 协作者行为规范

本文件约束所有在 nanoxide 仓库中工作的 AI 代理（Claude Code、Copilot、Cursor 等）。人类贡献者也可参考。

## 1. 项目定位

nanoxide（二进制名 `nax`）是 **GNU nano 9.2 的语义级 Rust 移植**。目标是行为、按键、命令行、rc 文件与 nano 9.2 兼容，不是"受 nano 启发的新编辑器"。

- 对照基准：GNU nano **9.2** 的 `nano/`，不要拿其他版本或自己的记忆当依据。
- 终端：ncurses → `crossterm`。国际化：gettext → `locales/*.ftl` + `t!` 宏。
- 平台：Linux 与 Windows 都是一等公民，改动不得破坏任一平台。
- 许可证：GPL-3.0。移植自 GPL 代码，保留原有版权声明，不得引入与 GPL-3.0 不兼容的代码或依赖。

## 2. 最高原则：忠实于原版，不臆造

1. **先读 C 源码再动手。** 修改或新增任何行为前，找到 nano 9.2 对应的 C 函数并读完，不要凭印象改。
2. **不确定就标注，不要编造。** 无法对照原版时，在注释和 PR 说明里写明"未对照原版"，不要把猜测写成事实。
3. **不要"顺手改进"。** 发现原版行为不合理也先保持一致；确需偏离，必须在 `DIFFERENCES.md` 中登记，并说明原因。
4. **每个移植函数必须能追溯到 C 函数。** 注释写明对应关系，沿用现有风格，例如：
   ```rust
   /// 对应 C 版 `do_justify`（text.c）。
   ```
   有简化时直接写"简化版"并说明省略了什么。
5. **不得静默吞掉功能。** 新增 `FunctionId` 时，必须在 `shortcut_init`（绑定）、`execute_by_id`（分发）、`strtosc`（rc 里的 bind 名）三处都接上。`execute_by_id` 的 `_ => {}` 不是实现。
6. **全程使用中文思考和回答。**

## 3. "完成"的定义

声明一个功能"已实现 ✅"，必须同时满足：

- [ ] 与 C 版行为一致（或差异已写入 `DIFFERENCES.md`）
- [ ] **用户能触达**：有默认快捷键 / 命令行选项 / rc 指令之一，且已接入分发
- [ ] 有测试覆盖（`tests/` 下集成测试）
- [ ] `cargo build`、`cargo test`、`cargo clippy` 在 Linux 上通过；涉及终端或文件系统的改动尽量在 Windows 上验证，不能验证要说明

函数写出来但没接线的，只能叫 ⚠️，不能叫 ✅。

## 4. 优先处理的已知问题

详见 [DIFFERENCES.md](DIFFERENCES.md)，AI 在没有明确任务时可优先处理：

1. 命令行短选项映射与 9.2 不一致（`-b` `-K` `-Z` `-g` `-O` `-M` `-X` `-I`），以及缺失的选项。目标是对齐 9.2 的 getopt 字符串 `ABC:DEFGHIJ:KLMNOPQ:RST:UVWX:Y:Zabcdef:ghijklmno:pqr:s:tuvwxyz!@%_01/` 及长选项表（`nano.c` 的 `long_options[]`）。
2. 启动时缺少 `SET(NO_WRAP)`，导致默认硬换行是开的。
3. `M-J`、`M-<`、`M->`、`M-{`、`M-}`、`M-W`、`M-Q`、`Sh-Tab` 等绑定缺失或未分发。
4. 鼠标支持、`write_region_to_file`、stdin 读取、操作目录限制。

## 5. 代码规范

### 5.1 结构

- 模块与 C 文件一一对应（见 `DIFFERENCES.md` 的模块映射）。新增函数放进对应模块，不要随意新建模块。
- 全局状态通过 `with_global` / `with_global_mut` 访问。**不要**在闭包嵌套里再次借用同一个 `GlobalState`（`RefCell` 会 panic）；需要时先取出值再调用。
- 不引入新的第三方依赖，除非先在 issue 里讨论。当前只有 `crossterm` 和 `libc`。
- `unsafe` 只允许出现在信号处理和 libc 调用处，并附上安全性说明。

### 5.2 平台

- Unix 专有代码用 `#[cfg(unix)]` 隔离，并给 Windows 提供合理的降级实现或明确的"不支持"。
- 路径处理不要假设 `/`；文件锁、权限、符号链接等在 Windows 上的行为要单独考虑。

### 5.3 文本与国际化

- 所有面向用户的文本走 `t!("key")`，key 沿用 nano `.po` 的 msgid 对应关系。
- 新增 key 必须同时加入 `locales/en-US.ftl` 和 `locales/zh-CN.ftl`。
- 翻译文本来自 nano 的 `.po` 文件，不要自己意译已有条目。
- 复数形式要处理，不要两个分支返回同一个字符串。
- 源码注释用中文，与现有代码保持一致。

### 5.4 终端与输入

- 不要在 crossterm 层之外直接写转义序列。
- 按键映射保持与 C 版 `shortcut_init` 一致；新增或修改后跑 `tests/keymap_check.rs`。

## 6. 测试

- 集成测试放在 `tests/`，文件名按主题命名。
- 修 bug 时先写能复现的测试，再改代码（参考现有 `crash_repro.rs`、`remaining_fixes.rs`）。
- 涉及渲染、按键序列的改动，补充 `render_check.rs` / `key_sequence.rs` 类测试。
- 不要为了让测试通过而削弱断言或删除测试。
- 运行：
  ```bash
  cargo build
  cargo test
  cargo clippy --all-targets
  cargo fmt --check
  ```

## 7. 文档同步

- 任何改变行为、选项、绑定、已知缺口的提交，都要同步更新 `DIFFERENCES.md`。
- 更新时遵守其状态图例（✅ ⚠️ ❌ ❓），并写清证据来源（静态阅读 / 实际运行 / 测试）。
- 不要把没验证过的结论写成 ✅。
- 修改用户可见行为时，同步 `README.md` 与 `doc/README.zh_CN.md`。

## 8. 提交与沟通

- 一次提交只做一件事，提交信息说明"对应 C 版的哪个函数 / 哪个行为"。
- 改动说明里写清：**做了什么、对照了原版哪里、怎么验证的、哪些没验证**。
- 遇到需求与原版冲突、或原版行为不明确时，**停下来提问**，不要自行决定。
- 不要在没有被要求时重构、重命名、批量格式化无关代码。
- 不要删除或改写他人的注释与版权声明。

## 9. 禁止事项

- 禁止凭记忆编造 nano 的行为、选项、快捷键或函数名。
- 禁止为了"看起来完整"添加空函数或桩函数而不标注。
- 禁止在文档中把部分实现标记为完整实现。
- 禁止引入遥测、网络请求或自动更新。
- 禁止提交密钥、个人路径、构建产物（`target/`）。
- 禁止修改 `LICENSE`，或移除 GPL 相关声明。
