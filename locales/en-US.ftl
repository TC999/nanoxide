# ============================================================
# locales/en-US.ftl
# 默认（英文）语言文件。其他语言文件命名：zh-CN.ftl / ja-JP.ftl / ...
#
# 格式：<key> = <value>
# 支持 {argname} 占位符。
# 行首 # 为注释，空行忽略。
#
# 约定：<key> 命名以模块为前缀，尽量语义化。
# ============================================================

# ---------- 全局/欢迎 ----------
welcome-message = Welcome to nano.  For basic help, type Ctrl+G.
winio-modified =  Modified
winio-new_buffer = New Buffer
winio-cursor_position = line {lineno}/{filebot_lineno} ({linepct}%), col {column}/{fullwidth} ({colpct}%), char {sum}/{totsize} ({charpct}%)
winio-view_warning = Key is invalid in view mode
winio-nameless = (nameless)

# ---------- browser（文件浏览器） ----------
browser-search = Search
browser-backwards = Backwards
browser-search_wrapped = Search Wrapped
browser-only_occurrence = This is the only occurrence
browser-not_found = Not found: {needle}
browser-cancelled = Cancelled
browser-no_search_pattern = No current search pattern
browser-cannot_open_dir = Cannot open directory: {path}
browser-no_entries = No entries
browser-go_to_dir = Go To Directory
browser-cannot_go_up = Can't move up a directory
browser-error_reading = Error reading {item}
browser-dir_disappeared = The working directory has disappeared

# ---------- cut（剪切/复制/粘贴） ----------
cut-nothing_cut = Nothing was cut
cut-copied_nothing = Copied nothing
cut-buffer_empty = Cutbuffer is empty

# ---------- files（读/写文件） ----------
files-error_reading = Error reading {filename}: {err}
files-wrote_one_line = Wrote {count} line
files-wrote_lines = Wrote {count} lines
files-error_writing = Error writing {filename}: {err}
files-write_to_file = Write to File
files-cancelled = Cancelled
prompt-save_modified_buffer = Save modified buffer? 
files-restricted_mode = This function is disabled in restricted mode
files-is_a_directory = '{filename}' is a directory
files-new_file = New File
files-insert_file = File to insert
files-insert_new_buffer = File to read into new buffer
files-execute_command = Command to execute
files-execute_new_buffer = Command to execute in new buffer
files-executing = Executing...
files-pipe_failed = Piping failed
files-command_failed = Error: {err}
files-command_cancelled = Cancelled
files-error_reading_stdin = Error reading from pipe: {err}
files-no_temp_file = Error creating temp file: {err}
files-error_writing_temp = Error writing temp file: {err}
files-spell_check_failed = Spell checking failed: {err}
files-linter_failed = Linting failed: {err}
files-no_linter_defined = No linter is defined for this type of file
files-no_formatter_defined = No formatter is defined for this type of file
files-no_speller_defined = No spell checker is defined
files-no_more_open_buffers = No more open file buffers
files-lines_one = {name} -- {count} line
files-lines_many = {name} -- {count} lines
files-making_backup = Making backup...
files-too_many_backups = Too many existing backup files
files-cannot_write_backup = Error writing backup file {filename}: {err}
files-error_deleting_lockfile = Error deleting lock file {filename}: {err}
files-error_writing_lockfile = Error writing lock file {filename}: {err}
files-couldnt_determine_identity = Couldn't determine my identity for lock file
files-couldnt_determine_hostname = Couldn't determine hostname: {err}
files-someone_else_editing = Someone else is also editing this file
files-error_opening_lockfile = Error opening lock file {filename}: {err}
files-bad_lock_file = Bad lock file is ignored: {filename}
files-being_edited = File {filename} is being edited by {user} (with {prog}, PID {pid}); open anyway?

# ---------- movement（光标移动） ----------
movement-not_possible = Not possible with '{opt}'

# ---------- search（搜索/替换） ----------
search-search = Search
search-case_sensitive = Case sensitive
search-regexp = Reg.exp.
search-backwards = Backwards
search-to_replace = (to replace)
search-bad_regex = Bad regex "{regexp}"
search-search_wrapped = Search Wrapped
search-cancelled = Cancelled
search-searching = Searching...
search-not_found = "{pattern}" not found
search-only_occurrence = This is the only occurrence
search-no_search_pattern = No current search pattern
search-replace_instance = Replace this instance?
search-replace_with = Replace with
search-view_replace_disabled = View mode: Replace disabled
search-replaced_one = Replaced {count} occurrence
search-replaced_many = Replaced {count} occurrences
search-enter_line_column = Enter line number, column number
search-invalid_line_or_column = Invalid line or column number
search-not_a_bracket = Not a bracket
search-no_matching_bracket = No matching bracket

# ---------- text（编辑操作） ----------
text-mark_unset = Mark Unset
text-mark_set = Mark Set
text-no_comment_syntax = Commenting is not supported for this file type
text-no_comment_past_eof = Cannot comment past end of file
text-nothing_to_undo = Nothing to undo
text-nothing_to_redo = Nothing to redo
text-undid = Undid {action}
text-redid = Redid {action}
text-verbatim_input = Verbatim Input
text-invalid_code = Invalid code
text-buffer_is_empty = Buffer is empty
text-selection_is_empty = Selection is empty
text-nothing_changed = Nothing changed
text-invoking_formatter = Invoking formatter...
text-finished_checking_spelling = Finished checking spelling
text-error_invoking = Error invoking '{program}'
text-no_word_fragment = No word fragment
text-no_further_matches = No further matches
text-no_matches = No matches
text-justified = Justified
text-justified_paragraph = Justified paragraph
text-justified_file = Justified file
text-justified_selection = Justified selection
text-no_paragraph = No paragraph to justify
text-use_fg = Use "fg" to return to nano.

# ---------- color（颜色/语法） ----------
color-unknown_syntax = Unknown syntax name: {name}
color-no_prefix_allowed = Color '{name}' takes no prefix
color-unknown_color = Color "{name}" not understood
color-attr_needs_comma = An attribute requires a subsequent comma

# ---------- rcfile（nanorc 配置解析） ----------
rcfile-missing_command = A '{kind}' command requires a preceding 'syntax' command
rcfile-default_no_regex = The "default" syntax does not accept '{kind}' regexes
rcfile-missing_regex = Missing regex string after '{kind}' command
rcfile-bad_regex = Bad regex "{expr}": {msg}
rcfile-missing_arg = Missing argument after '{kind}'
rcfile-missing_quote = Argument of '{kind}' lacks closing "
rcfile-syntax_not_found = Could not find syntax "{name}" to extend
rcfile-mistakes_in = Mistakes in '{name}'
rcfile-error_in = Error in {file} on line {line}: {msg}
rcfile-missing_key_name = Missing key name
rcfile-invalid_key_name = Key name {name} is invalid
rcfile-must_specify_function = Must specify a function to bind the key to
rcfile-missing_menu = Must specify a menu (or "all") in which to bind/unbind the key
rcfile-unknown_menu = Unknown menu: {name}
rcfile-unknown_function = Unknown function: {name}
rcfile-function_not_in_menu = Function '{func}' does not exist in menu '{menu}'
rcfile-no_rebind = Keystroke {key} may not be rebound
rcfile-cannot_unset = Cannot unset option "{option}"
rcfile-requires_argument = Option "{option}" requires an argument
rcfile-unknown_option = Unknown option: {option}
rcfile-missing_option = Missing option
rcfile-not_allowed_include = Command "{command}" not allowed in included file
rcfile-not_understood = Command "{command}" not understood
rcfile-no_color_commands = Syntax "{name}" has no color commands
rcfile-missing_color_name = Missing color name
rcfile-missing_regex_after = Missing regex string after '{command}' command
rcfile-start_requires_end = "start=" requires a corresponding "end="
rcfile-error_reading = Error reading {file}: {err}
rcfile-is_directory = '{file}' is a directory
rcfile-is_device = '{file}' is a device file
rcfile-path_too_long = Path is too long
rcfile-error_expanding = Error expanding {pattern}: {err}
rcfile-missing_syntax_name = Missing syntax name
rcfile-unpaired_quote = Unpaired quote in syntax name
rcfile-none_reserved = The "none" syntax is reserved
rcfile-default_no_ext = The "default" syntax does not accept extensions
rcfile-requires_preceding = A '{kind}' command requires a preceding 'syntax' command
rcfile-regex_quotes = Regex strings must begin and end with a " character
rcfile-empty_regex = Empty regex string
rcfile-non_blank = Non-blank characters required
rcfile-even_chars = Even number of characters required
rcfile-two_single_col = Two single-column characters required
rcfile-fill_invalid = Requested fill size "{arg}" is invalid
rcfile-tabsize_invalid = Requested tab size "{arg}" is invalid
rcfile-guide_invalid = Guide column "{arg}" is invalid
rcfile-no_home = I can't find my home directory!  Wah!
rcfile-specified_not_exist = Specified rcfile does not exist
rcfile-no_key_bound = No key is bound to function '{func}' in menu '{menu}'.  Exiting.
rcfile-die_hint = If needed, use nano with the -I option to adjust your nanorc settings.
rcfile-invalid_multibyte = Argument is not a valid multibyte string

# ---------- history（历史记录/注册表） ----------
history-error_reading = Error reading {name}: {err}
history-error_writing = Error writing {name}: {err}

# ---------- regex 引擎 ----------
regex-unexpected = unexpected '{ch}'
regex-dangling = dangling '{ch}'

# ---------- help（帮助文本） ----------
# 各菜单介绍标题
help-search_title = Search Command Help Text
help-replace_title = === Replacement ===
help-goto_line_title = Go To Line Help Text
help-insert_file_title = Insert File Help Text
help-write_file_title = Write File Help Text
help-browser_title = File Browser Help Text
help-browser_search_title = Browser Search Command Help Text
help-browser_gotodir_title = Browser Go To Directory Help Text
help-spell_title = === Spelling correction ===
help-execute_title = Execute Command Help Text
help-linter_title = === Linter ===
help-main_title = Main nano help text

# 搜索帮助正文
help-search_body = Enter the words or characters you would like to search for, and then press Enter.  If there is a match for the text you entered, the screen will be updated to the location of the nearest match for the search string.
help-search_prev = The previous search string will be shown in brackets after the search prompt.  Hitting Enter without entering any text will perform the previous search.
help-search_select = If you have selected text with the mark and then search to replace, only matches in the selected text will be replaced.
help-search_fnkeys = The following function keys are available in Search mode

# 替换帮助正文
help-replace_body = Type the characters that should replace what you typed at this previous prompt, and press Enter.
help-replace_fnkeys = The following function keys are available at this prompt

# 转到行
help-goto_body = Enter the line number that you wish to go to and hit Enter.  If there are fewer lines of text than the number you entered, you will be brought to the last line of the file.
help-goto_fnkeys = The following function keys are available in Go To Line mode

# 插入文件
help-insert_body = Type in the name of a file to be inserted into the current file buffer at the current cursor location.  If you have compiled nano with multiple file buffer support, and enable multiple file buffers with the -F or --multibuffer command line flags, the Meta-F toggle, or a nanorc file, inserting a file will cause it to be loaded into a separate buffer (use Meta-< and > to switch between file buffers).
help-insert_extra = If you need another blank buffer, do not enter any filename, or type in a nonexistent filename at the prompt and press Enter.
help-insert_fnkeys = The following function keys are available in Insert File mode

# 写文件
help-write_body = Type the name that you wish to save the current file as and press Enter to save the file.  If you have selected text with the mark, you will be prompted to save only the selected portion to a separate file.  To reduce the chance of overwriting the current file with just a portion of it, the current filename is not the default in this mode.
help-write_fnkeys = The following function keys are available in Write File mode

# 浏览器帮助
help-browser_body = The file browser is used to visually browse the directory structure to select a file for reading or writing.  You may use the arrow keys or Page Up/Down to browse through the files, and S or Enter to choose the selected file or enter the selected directory.  To move up one level, select the directory called ".." at the top of the file list.
help-browser_fnkeys = The following function keys are available in the file browser

# 浏览器搜索
help-bsearch_body = Enter the words or characters you would like to search for, and then press Enter.  If there is a match for the text you entered, the screen will be updated to the location of the nearest match for the search string.
help-bsearch_prev = The previous search string will be shown in brackets after the search prompt.  Hitting Enter without entering any text will perform the previous search.

# 浏览器 Go To Directory
help-bgotodir_body = Enter the name of the directory you would like to browse to.  If tab completion has not been disabled, you can use the Tab key to (attempt to) automatically complete the directory name.
help-bgotodir_fnkeys = The following function keys are available in Browser Go To Directory mode

# 拼写
help-spell_fnkeys = The following function keys are available at this prompt

# Linter
help-linter_fnkeys = The following function keys are available in Linter mode

# 主帮助正文
help-main_body = The nano editor is designed to emulate the functionality and ease-of-use of the UW Pico text editor.  There are four main sections of the editor.  The top line shows the program version, the current filename being edited, and whether or not the file has been modified.  Next is the main editor window showing the file being edited.  The status line is the third line from the bottom and shows important messages.
help-main_keydesc = The bottom two lines show the most commonly used shortcuts in the editor.  Shortcuts are written as follows: Control-key sequences are notated with a '^' and can be entered either by using the Ctrl key or pressing the Esc key twice.  Meta-key sequences are notated with 'M-' and can be entered using either the Alt, Cmd, or Esc key, depending on your keyboard setup.
help-main_extra = Also, pressing Esc twice and then typing a three-digit decimal number from 000 to 255 will enter the character with the corresponding value.  The following keystrokes are available in the main editor window.  Alternative keys are shown in parentheses.

# ---------- key（快捷键/底部栏标签，来自 global.rs add_to_funcs 的 tag） ----------
key-help = Help
key-refresh = Refresh
key-close = Close
key-cancel = Cancel
key-yes = Yes
key-no = No
key-exit = Exit
key-write_out = Write Out
key-read_file = Read File
key-justify = Justify
key-where_is = Where Is
key-where_was = Where Was
key-replace = Replace
key-previous = Previous
key-next = Next
key-cut = Cut
key-paste = Paste
key-execute = Execute
key-location = Location
key-go_to_line = Go To Line
key-undo = Undo
key-redo = Redo
key-set_mark = Set Mark
key-copy = Copy
key-case_sens = Case Sens
key-regexp = Regexp
key-backwards = Backwards
key-to_bracket = To Bracket
key-left = Left
key-right = Right
key-prev_line = Prev Line
key-next_line = Next Line
key-home = Home
key-end = End
key-prev_page = Prev Page
key-next_page = Next Page
key-delete = Delete
key-backspace = Backspace
key-enter = Enter
key-tab = Tab
key-get_older_item = Get Older Item
key-get_newer_item = Get Newer Item
key-start_of_paragraph = Start of Paragraph
key-end_of_paragraph = End of Paragraph
key-first_line = First Line
key-last_line = Last Line
key-to_search = To Search
key-first_file = First File
key-last_file = Last File
key-go_to_dir = Go To Dir
key-verbatim_input = Verbatim Input
key-unbound = Unbound Key