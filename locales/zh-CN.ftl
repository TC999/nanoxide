# ============================================================
# locales/zh-CN.ftl
# 简体中文语言文件。翻译取自 nano/po/zh_CN.po（GNU nano 官方简体中文翻译）。
#
# 格式：<key> = <value>
# 支持 {argname} 占位符。
# 行首 # 为注释，空行忽略。
#
# 说明：
#   - po 中整段式 help 文本按 locales/en-US.ftl 的拆分方式拆为独立 key；
#   - 段落间的 \n\n 压为两个空格（与 en-US.ftl 的处理一致）；
#   - 少数 po 中不存在的消息（browser-not_found、search-view_replace_disabled、
#     regex-unexpected、regex-dangling、browser-error_reading 单参数版）按
#     nano 中文习惯直译。
# ============================================================

# ---------- 全局/欢迎 ----------
welcome-message = 欢迎使用 nano。  如需基本帮助信息，请按下 Ctrl+G。
winio-modified = 已更改
winio-new_buffer = 新缓冲区
winio-cursor_position = 行 {lineno}/{filebot_lineno} ({linepct}%)，列 {column}/{fullwidth} ({colpct}%)，字符 {sum}/{totsize} ({charpct}%)
winio-view_warning = 该键在查看模式中无效
winio-nameless = (未命名)

# ---------- browser（文件浏览器） ----------
browser-search = 搜索
browser-backwards = 向后搜索
browser-search_wrapped = 已从头搜索
browser-only_occurrence = 这是唯一出现之处
browser-not_found = 找不到：{needle}
browser-cancelled = 已取消
browser-no_search_pattern = 没有当前搜索模式
browser-cannot_open_dir = 无法打开目录：{path}
browser-no_entries = 没有条目
browser-go_to_dir = 跳至目录
browser-cannot_go_up = 无法上移一个目录
browser-error_reading = 读取 {item} 出错
browser-dir_disappeared = 工作目录已消失

# ---------- cut（剪切/复制/粘贴） ----------
cut-nothing_cut = 无剪切部分
cut-copied_nothing = 未复制任何内容
cut-buffer_empty = 剪贴缓冲区为空

# ---------- files（读/写文件） ----------
files-error_reading = 读取 {filename} 出错：{err}
files-wrote_one_line = 已写入 {count} 行
files-wrote_lines = 已写入 {count} 行
files-error_writing = 写入{filename} 出错：{err}
files-write_to_file = 写入到文件
files-cancelled = 已取消
prompt-save_modified_buffer = 保存已修改的缓冲区？ 
files-restricted_mode = 在限制模式中此功能被禁用
files-is_a_directory = '{filename}' 是一个目录
files-new_file = 新文件
files-insert_file = 插入文件
files-insert_new_buffer = 读取文件到新缓冲区
files-execute_command = 要执行的命令
files-execute_new_buffer = 在新缓冲区中执行的命令
files-executing = 正在执行...
files-pipe_failed = 管道传输失败
files-command_failed = 错误：{err}
files-command_cancelled = 已取消
files-error_reading_stdin = 从管道读取出错：{err}
files-no_temp_file = 创建临时文件出错：{err}
files-error_writing_temp = 写入临时文件出错：{err}
files-spell_check_failed = 拼写检查失败：{err}
files-linter_failed = 检查失败：{err}
files-no_linter_defined = 未为此文件类型定义检查器
files-no_formatter_defined = 未为此文件类型定义格式化器
files-no_speller_defined = 未定义拼写检查器
files-no_more_open_buffers = 没有更多打开的文件缓冲区
files-lines_one = {name} -- {count} 行
files-lines_many = {name} -- {count} 行
files-making_backup = 正在备份...
files-too_many_backups = 现有备份文件过多
files-cannot_write_backup = 写入备份文件 {filename} 出错：{err}
files-error_deleting_lockfile = 删除锁文件 {filename} 出错：{err}
files-error_writing_lockfile = 写入锁文件 {filename} 出错：{err}
files-couldnt_determine_identity = 无法确定我的身份以创建锁文件
files-couldnt_determine_hostname = 无法确定主机名：{err}
files-someone_else_editing = 其他人也在编辑此文件
files-error_opening_lockfile = 打开锁文件 {filename} 出错：{err}
files-bad_lock_file = 无效的锁文件被忽略：{filename}
files-being_edited = 文件 {filename} 正被 {user}（使用 {prog}，PID {pid}）编辑；仍要打开吗？

# ---------- movement（光标移动） ----------
movement-not_possible = 使用 "{opt}" 时无法做到

# ---------- search（搜索/替换） ----------
search-search = 搜索
search-case_sensitive = 区分大小写
search-regexp = 正则表达式
search-backwards = 向后搜索
search-to_replace = (替换)
search-bad_regex = 非法的正则表达式 "{regexp}"
search-search_wrapped = 已从头搜索
search-cancelled = 已取消
search-searching = 正在搜索...
search-not_found = 找不到 "{pattern}"
search-only_occurrence = 这是唯一出现之处
search-no_search_pattern = 没有当前搜索模式
search-replace_instance = 替换这个？
search-replace_with = 替换为
search-view_replace_disabled = 查看模式：已禁用替换
search-replaced_one = 已替换 {count} 处
search-replaced_many = 已替换 {count} 处
search-enter_line_column = 输入列号，行号
search-invalid_line_or_column = 无效的行号或列号
search-not_a_bracket = 不是括号
search-no_matching_bracket = 没有匹配的括号

# ---------- text（编辑操作） ----------
text-mark_unset = 标记解除
text-mark_set = 标记设定
text-no_comment_syntax = 该文件类型不支持做注释
text-no_comment_past_eof = 无法注释越过文件末尾
text-nothing_to_undo = 没有可撤销的操作
text-nothing_to_redo = 没有可重做的操作
text-undid = 已撤销 {action}
text-redid = 已重做 {action}
text-verbatim_input = 逐字输入
text-invalid_code = 无效代码
text-buffer_is_empty = 缓冲区为空
text-selection_is_empty = 选区为空
text-nothing_changed = 没有变化
text-invoking_formatter = 正在调用格式化器...
text-finished_checking_spelling = 拼写检查完成
text-error_invoking = 调用 '{program}' 出错
text-no_word_fragment = 光标左侧没有单词片段
text-no_further_matches = 没有更多匹配
text-no_matches = 没有匹配
text-justified = 已对齐
text-justified_paragraph = 已对齐段落
text-justified_file = 已对齐文件
text-justified_selection = 已对齐选区
text-no_paragraph = 没有可对齐的段落
text-use_fg = 使用 "fg" 返回 nano。
text-redid = 已重做 {action}

# ---------- color（颜色/语法） ----------
color-unknown_syntax = 未知语法名称：{name}
color-no_prefix_allowed = 颜色“{name}”不接受前缀
color-unknown_color = 无法识别颜色“{name}”
color-attr_needs_comma = 属性需要后接一个逗号

# ---------- rcfile（nanorc 配置解析） ----------
rcfile-missing_command = “{kind}”命令需要一个前导 "syntax" 命令
rcfile-default_no_regex = "default" 语法不接受“{kind}”正则表达式
rcfile-missing_regex = “{kind}”命令后缺少正则表达式字符串
rcfile-bad_regex = 非法的正则表达式 "{expr}": {msg}
rcfile-missing_arg = “{kind}”后缺少参数
rcfile-missing_quote = “{kind}”的参数缺少封闭的 "
rcfile-syntax_not_found = 无法找到要扩展的语法“{name}”
rcfile-mistakes_in = “{name}”中的错误
rcfile-error_in = 在 {file}（第 {line} 行）中发生错误：{msg}
rcfile-missing_key_name = 缺少键名
rcfile-invalid_key_name = 键名 {name} 无效
rcfile-must_specify_function = 必须指定要绑定到该键的函数
rcfile-missing_menu = 必须指定绑定/解绑键的菜单（或 "all"）
rcfile-unknown_menu = 未知菜单：{name}
rcfile-unknown_function = 未知函数：{name}
rcfile-function_not_in_menu = 函数“{func}”在菜单“{menu}”中不存在
rcfile-no_rebind = 键 {key} 不可重新绑定
rcfile-cannot_unset = 不能取消设置选项“{option}”
rcfile-requires_argument = 选项“{option}”需要一个参数
rcfile-unknown_option = 未知选项：{option}
rcfile-missing_option = 缺少选项
rcfile-not_allowed_include = 命令“{command}”不允许出现在被包含的文件中
rcfile-not_understood = 命令“{command}”无法识别
rcfile-no_color_commands = 语法“{name}”没有颜色命令
rcfile-missing_color_name = 缺少颜色名
rcfile-missing_regex_after = “{command}”命令后缺少正则表达式字符串
rcfile-start_requires_end = "start=" 需要一个对应的 "end="
rcfile-error_reading = 读取 {file} 出错：{err}
rcfile-is_directory = “{file}”是一个目录
rcfile-is_device = “{file}”是一个设备文件
rcfile-path_too_long = 路径过长
rcfile-error_expanding = 展开 {pattern} 出错：{err}
rcfile-missing_syntax_name = 缺少语法名
rcfile-unpaired_quote = 语法名中的引号不配对
rcfile-none_reserved = "none" 语法被保留
rcfile-default_no_ext = "default" 语法不接受扩展名
rcfile-requires_preceding = “{kind}”命令需要一个前导 'syntax' 命令
rcfile-regex_quotes = 正则表达式字符串必须以 " 字符开头和结尾
rcfile-empty_regex = 空的正则表达式字符串
rcfile-non_blank = 需要非空白字符
rcfile-even_chars = 需要偶数个字符
rcfile-two_single_col = 需要两个单列字符
rcfile-fill_invalid = 请求的填充宽度“{arg}”无效
rcfile-tabsize_invalid = 请求的制表符宽度“{arg}”无效
rcfile-guide_invalid = 引导列“{arg}”无效
rcfile-no_home = 找不到我的主目录！哇！
rcfile-specified_not_exist = 指定的 rcfile 不存在
rcfile-no_key_bound = 在菜单“{menu}”中没有键绑定到函数“{func}”。正在退出。
rcfile-die_hint = 如有必要，请使用带 -I 选项的 nano 调整您的 nanorc 设置。
rcfile-invalid_multibyte = 参数不是有效的多字节字符串

# ---------- history（历史记录/注册表） ----------
history-error_reading = 读取 {name} 出错：{err}
history-error_writing = 写入{name} 出错：{err}

# ---------- regex 引擎 ----------
regex-unexpected = 意外的 '{ch}'
regex-dangling = 悬空的 '{ch}'

# ---------- help（帮助文本） ----------
# 各菜单介绍标题
help-search_title = 搜索命令辅助说明
help-replace_title = === 替换 ===
help-goto_line_title = 跳行辅助说明
help-insert_file_title = 插入文件辅助说明
help-write_file_title = 写入文件辅助说明
help-browser_title = 文件选单辅助说明
help-browser_search_title = 搜索命令辅助说明
help-browser_gotodir_title = 跳至目录辅助说明
help-spell_title = === 拼写修正 ===
help-execute_title = 执行命令辅助说明
help-linter_title = === 代码语法检查 ===
help-main_title = nano 主帮助文档

# 搜索帮助正文
help-search_body = 首先输入您想要搜索的字符串或字符，然后按下回车键。如果存在着符合您所输入的文字，画面就会更新到最合乎搜索字符串的位置。
help-search_prev = 最近一次搜索的字符串将会显示在搜索提示符后的括号中。不输入任何文字而直接按下回车键则会重复使用最近一次的搜索条件。
help-search_select = 如果您已经用标记选择了一段文字并进行搜索替换，就只有在选择文字中符合者将会被替换。
help-search_fnkeys = 以下的功能键可用于搜索模式：

# 替换帮助正文
help-replace_body = 请输入用于替换您在上一个提示符处键入的内容的字符，然后按 Enter。
help-replace_fnkeys = 在此提示符下可使用如下的功能键：

# 转到行
help-goto_body = 首先输入您想去的行数编号并按下回车键。如果文章中的行数比您所输入少，您将会被带至文件的最后一行。
help-goto_fnkeys = 以下的功能键可用于跳行模式：

# 插入文件
help-insert_body = 先把文件的名称键入，它将会插入在当前缓冲区的游标所在之处。  如果您所编译的 nano 支持多重文件缓冲区，并且将此功能以命令列旗标-F 或--multibuffer、Meta-F 开关，或者 nanorc 文件来启动的话，所插入的文件 将会被载入另外的缓冲区中 (利用 Meta-< 和 > 在文件缓冲区间切换)。
help-insert_extra = 如果您需要另一个空的缓冲区，那就不要输入任何文件名，或是在提示符号后键入一个不存在的文件名，然后按下回车键。
help-insert_fnkeys = 以下的功能键可用于插入文件模式：

# 写文件
help-write_body = 先键入您想要以此来储存当前文件的名称，并按下回车键来储存文件。  如果已经用标记选择了文字，那么您将会被提示，只储存标记部份到另外的档案。为了降低当前文件只被其中部份覆盖的机会，在此模式下，当前的文件名不会成为默认值。
help-write_fnkeys = 以下的功能键可用于写入文件模式：

# 浏览器帮助
help-browser_body = 文件选单是用来视觉化浏览目录结构，以选取要读出或写入的文件。您可以 使用上下左右键或上页/下页来浏览，并用S 或回车键来选取所要的文件或者进入所选的目录。要跳到上层时，选择在文件列表顶端名为 ".." 的目录。
help-browser_fnkeys = 以下的功能键可用于文件选单：

# 浏览器搜索
help-bsearch_body = 首先输入您想要搜索的字符串或字符，然后按下回车键。如果存在著 符合您所输入的文字，画面就会更新到最合乎搜索字符串的位置。
help-bsearch_prev = 最近一次搜索的字符串将会显示在搜索提示符号后面的括号中。不输入任何文字而直接按下回车键则会重复最近一次的搜索条件。

# 浏览器 Go To Directory
help-bgotodir_body = 先输入您想要浏览的目录名称。  如果制表符补全的功能没有被关闭，您可以利用制表符(尝试)自动补全目录名称。
help-bgotodir_fnkeys = 以下的功能键可用于跳至目录模式：

# 拼写
help-spell_fnkeys = 在此提示符下可使用如下的功能键：

# Linter
help-linter_fnkeys = 在代码语法检查模式可使用如下的功能键：

# 主帮助正文
help-main_body = nano 编辑器被设计用来模仿华盛顿大学 Pico 文本编辑器，  且具有类似的功能性与易用性。它包括四个主要部分：  顶行显示程序版本、当前被编辑的文件名以及该文件是否已被修改。  接着是主要编辑区，显示正在编辑的文件。  状态行位于倒数第三行，用来显示重要的信息。
help-main_keydesc = 底部的两行显示了编辑器中最常用的快捷键。  快捷键用如下方式进行表示：控制键序列使用一个“^”符号标记，它可以用 Ctrl 键或按 Esc 键两次的方式进行输入。Meta 键序列使用“M-”符号标记，它可以用 Alt、Cmd 或 Esc 键输入，具体取决于您的键盘设置。
help-main_extra = 另外，按 Esc 两次之后再键入从 000 到 255 之间的三位数字，则会输入该 ASCII 码对应的字符。下列按键组合可用于主要编辑区，替代按键则显示于括号内。

# ---------- key（快捷键/底部栏标签，来自 global.rs add_to_funcs 的 tag） ----------
key-help = 帮助
key-refresh = 刷新
key-close = 关闭
key-cancel = 取消
key-yes = 是
key-no = 否
key-exit = 离开
key-write_out = 写入
key-read_file = 读档
key-justify = 对齐
key-where_is = 搜索
key-where_was = 向前搜索
key-replace = 替换
key-previous = 上一个
key-next = 下一个
key-cut = 剪切
key-paste = 粘贴
key-execute = 执行命令
key-location = 位置
key-go_to_line = 跳行
key-undo = 撤销
key-redo = 重做
key-set_mark = 设置标记
key-copy = 复制
key-case_sens = 区分大小写
key-regexp = 正则表达式
key-backwards = 向后搜索
key-to_bracket = 至括号
key-left = 左
key-right = 右
key-prev_line = 上行
key-next_line = 下行
key-home = 顶端
key-end = 尾端
key-prev_page = 上页
key-next_page = 下页
key-delete = 删除
key-backspace = 退格
key-enter = 回车
key-tab = 制表符
key-get_older_item = 更旧项
key-get_newer_item = 更新项
key-start_of_paragraph = 段落开头
key-end_of_paragraph = 段落结尾
key-first_line = 首行
key-last_line = 尾行
key-to_search = 跳至文字
key-first_file = 首文件
key-last_file = 末文件
key-go_to_dir = 跳至目录
key-verbatim_input = 原形输入
key-unbound = 按键未绑定