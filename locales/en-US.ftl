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
winio-view =  View
winio-restricted =  Restricted
winio-new_buffer = New Buffer
winio-cursor_position = line {lineno}/{filebot_lineno} ({linepct}%), col {column}/{fullwidth} ({colpct}%), char {sum}/{totsize} ({charpct}%)
winio-view_warning = Key is invalid in view mode
winio-function_not_implemented = Function not implemented: {func}
winio-nameless = (nameless)
global-byte_order_mark = Byte Order Mark
global-suspend_hint = To suspend, type ^T^Z
# 未绑定/不可绑定按键的提示（对应 nano.c unbound_key 与 po 的同名 msgid）
global-unknown_sequence = Unknown sequence
global-unbound_key = Unbound key
global-unbound_key_F = Unbound key: F{number}
global-unbound_key_name = Unbound key: {name}{letter}
global-unbindable_M = Unbindable key: M-^{letter}
global-unbindable_ctrl_bracket = Unbindable key: ^[

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
files-write_selection_to_file = Write Selection to File
# 保存时的覆盖确认（对应 files.c write_it_out 的三个提示）
files-exists_cannot_overwrite = File exists -- cannot overwrite
files-save_under_different_name = Save file under DIFFERENT NAME? 
files-exists_overwrite = File "{name}" exists; OVERWRITE? 
files-disk_changed = File on disk has changed
files-modified_since_open = File was modified since you opened it; continue saving? 
files-cancelled = Cancelled
prompt-save_modified_buffer = Save modified buffer? 
files-restricted_mode = This function is disabled in restricted mode
files-invalid_operating_dir = Invalid operating directory: {dir}
files-cant_read_outside = Can't read file from outside of {dir}
files-cant_write_outside = Can't write outside of {dir}
files-cant_go_outside = Can't go outside of {dir}
files-is_a_directory = '{filename}' is a directory
files-is_device = '{filename}' is a device file
files-invalid_filename = '{filename}' is an invalid filename
files-read_only_file = '{filename}' is meant to be read-only
files-directory_does_not_exist = Directory '{dir}' does not exist
files-path_error = Path '{path}': {err}
files-path_not_a_directory = Path '{path}' is not a directory
files-path_not_accessible = Path '{path}' is not accessible
files-working_dir_gone = The working directory has disappeared
files-directory_not_writable = Directory '{dir}' is not writable
files-new_file = New File
files-file_not_found = File "{filename}" not found
files-insert_file = File to insert
files-insert_unconverted = File to insert unconverted
files-insert_new_buffer = File to read into new buffer
files-insert_unconverted_new_buffer = File to read unconverted into new buffer
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
files-no_file_name = No file name
files-lines_one = {name} -- {count} line
files-lines_many = {name} -- {count} lines
files-read_one = Read {count} line
files-read_many = Read {count} lines
files-read_dos_one = Read {count} line (converted from DOS format)
files-read_dos_many = Read {count} lines (converted from DOS format)
files-unwritable = File '{filename}' is unwritable
files-writing = Writing...
files-writing_to_fifo = Writing to FIFO...
files-reading_from_fifo = Reading from FIFO...
files-disk_full = File on disk has been truncated!
files-disk_full_remedy = Maybe ^T^Z, make room on disk, resume, then ^S^X
files-making_backup = Making backup...
files-too_many_backups = Too many existing backup files
files-cannot_write_backup = Error writing backup file {filename}: {err}
files-cannot_read_original = Cannot read original file
files-invalid_backup_dir = Invalid backup directory: {dir}
files-append_to_file = Append to File
files-prepend_to_file = Prepend to File
files-append_selection = Append Selection to File
files-prepend_selection = Prepend Selection to File
files-dos_format_marker =  [DOS Format]
files-backup_marker =  [Backup]
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
search-bad_regex = Bad regex "{regexp}": {err}
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
rcfile-could_not_find = Could not find syntax "{name}" to extend

# ---------- command line（对应 nano.c 的 usage / list_syntax_names） ----------
cmdline-available_syntaxes = Available syntaxes:
cmdline-type_help = Type '{arg}' -h for a list of available options.
#
# -h 说明文本：英文取 nano 9.2 po 的 msgid（nano/src/nano.c usage()），
# 中文取 nano/po/zh_CN.po 的 msgstr，逐字照抄、不意译。msgid/msgstr 把
# 程序名写死为 nano；本移植二进制名是 nax，故值里写 nax（与移植版
# print_usage 既有行为一致）。
cmdline-usage = Usage: nax [OPTIONS] [[+LINE[,COLUMN]] FILE]...
cmdline-cursor_hint = To place the cursor on a specific line of a file, put the line number with
cmdline-cursor_hint_cont = a '+' before the filename.  The column number can be added after a comma.
cmdline-stdin_hint = When a filename is '-', nax reads data from standard input.
cmdline-header_short = Option
cmdline-header_long = Long option
cmdline-header_meaning = Meaning
# 带参数的旗标本身也过 gettext（对应 print_opt(_("-C <dir>"), ...)）。
cmdline-short_backupdir = -C <dir>
cmdline-long_backupdir = --backupdir=<dir>
cmdline-short_guidestripe = -J <number>
cmdline-long_guidestripe = --guidestripe=<number>
cmdline-short_quotestr = -Q <regex>
cmdline-long_quotestr = --quotestr=<regex>
cmdline-short_tabsize = -T <number>
cmdline-long_tabsize = --tabsize=<number>
cmdline-short_wordchars = -X <string>
cmdline-long_wordchars = --wordchars=<string>
cmdline-short_syntax = -Y <name>
cmdline-long_syntax = --syntax=<name>
cmdline-short_rcfile = -f <file>
cmdline-long_rcfile = --rcfile=<file>
cmdline-short_operatingdir = -o <dir>
cmdline-long_operatingdir = --operatingdir=<dir>
cmdline-short_fill = -r <number>
cmdline-long_fill = --fill=<number>
cmdline-short_speller = -s <program>
cmdline-long_speller = --speller=<program>
# 选项描述（key 按长选项名命名，可追溯到 C 版 usage() 的调用行）。
cmdline-opt_smarthome = Enable smart home key
cmdline-opt_backup = Save backups of existing files
cmdline-opt_backupdir = Directory for saving unique backup files
cmdline-opt_boldtext = Use bold instead of reverse video text
cmdline-opt_tabstospaces = Convert typed tabs to spaces
cmdline-opt_newbuffer = Read a file into a new buffer by default
cmdline-opt_locking = Use (vim-style) lock files
cmdline-opt_historylog = Save & reload old search/replace strings
cmdline-opt_ignorercfiles = Don't look at nanorc files
cmdline-opt_guidestripe = Show a guiding bar at this column
cmdline-opt_rawsequences = Fix numeric keypad key confusion problem
cmdline-opt_nonewlines = Don't add an automatic newline
cmdline-opt_trimblanks = Trim tail spaces when hard-wrapping
cmdline-opt_noconvert = Don't convert files from DOS format
cmdline-opt_bookstyle = Leading whitespace means new paragraph
cmdline-opt_positionlog = Save & restore position of the cursor
cmdline-opt_quotestr = Regular expression to match quoting
cmdline-opt_restricted = Restrict access to the filesystem
cmdline-opt_softwrap = Display overlong lines on multiple rows
cmdline-opt_tabsize = Make a tab this number of columns wide
cmdline-opt_quickblank = Wipe status bar upon next keystroke
cmdline-opt_version = Print version information and exit
cmdline-opt_wordbounds = Detect word boundaries more accurately
cmdline-opt_wordchars = Which other characters are word parts
cmdline-opt_syntax = Syntax definition to use for coloring
cmdline-opt_zap = Let Bsp and Del erase a marked region
cmdline-opt_atblanks = When soft-wrapping, do it at whitespace
cmdline-opt_breaklonglines = Automatically hard-wrap overlong lines
cmdline-opt_constantshow = Constantly show cursor position
cmdline-opt_rebinddelete = Fix Backspace/Delete confusion problem
cmdline-opt_emptyline = Keep the line below the title bar empty
cmdline-opt_rcfile = Use only this file for configuring nax
cmdline-opt_showcursor = Show cursor in file browser & help text
cmdline-opt_help = Show this help text and exit
cmdline-opt_autoindent = Automatically indent new lines
cmdline-opt_jumpyscrolling = Scroll per half-screen, not per line
cmdline-opt_cutfromcursor = Cut from cursor to end of line
cmdline-opt_linenumbers = Show line numbers in front of the text
cmdline-opt_mouse = Enable the use of the mouse
cmdline-opt_noread = Do not read the file (only write it)
cmdline-opt_operatingdir = Set operating directory
cmdline-opt_preserve = Preserve XON (^Q) and XOFF (^S) keys
cmdline-opt_indicator = Show a position+portion indicator
cmdline-opt_fill = Set width for hard-wrap and justify
cmdline-opt_speller = Use this alternative spell checker
cmdline-opt_saveonexit = Save changes on exit, don't prompt
cmdline-opt_unix = Save a file by default in Unix format
cmdline-opt_view = View mode (read-only)
cmdline-opt_nowrap = Don't hard-wrap long lines [default]
cmdline-opt_nohelp = Don't show the two help lines
cmdline-opt_afterends = Make Ctrl+Right stop at word ends
cmdline-opt_listsyntaxes = List the names of available syntaxes
cmdline-opt_colonparsing = Accept 'filename:linenumber' notation
cmdline-opt_stateflags = Show some states on the title bar
cmdline-opt_minibar = Show a feedback bar at the bottom
cmdline-opt_zero = Hide all bars, use whole terminal
cmdline-opt_solosidescroll = Scroll only the current line sideways
cmdline-opt_modernbindings = Use better-known key bindings

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

# ---------- gist（帮助页里每条快捷键的描述，取自 global.c 的 *_gist 与 po） ----------
gist-anchor = Place or remove an anchor at the current line
gist-append = Toggle appending
gist-backspace = Delete the character to the left of the cursor
gist-backup = Toggle backing up of the original file
gist-backwards = Reverse the direction of the search
gist-bracket = Go to the matching bracket
gist-cancel = Cancel the current function
gist-casesens = Toggle the case sensitivity of the search
gist-center = Center the line where the cursor is
gist-chopwordleft = Delete backward from cursor to word start
gist-chopwordright = Delete forward from cursor to next word start
gist-comment = Comment/uncomment the current line (or marked lines)
gist-complete = Try and complete the current word
gist-copy = Copy current line (or marked region) and store it in cutbuffer
gist-cut = Cut current line (or marked region) and store it in cutbuffer
gist-cycle = Push the cursor line to the center, then top, then bottom
gist-delete = Delete the character under the cursor
gist-dosformat = Toggle the use of DOS format
gist-end = Go to end of current line
gist-enter = Insert a newline at the cursor position
gist-execute = Execute a function or an external command
gist-exit = Close the current buffer / Exit from nano
gist-findnext = Search next occurrence forward
gist-findprev = Search next occurrence backward
gist-firstfile = Go to the first file in the list
gist-firstline = Go to the first line of the file
gist-flipconvert = Do not convert from DOS format
gist-flipexecute = Execute a function or an external command
gist-flipnewbuffer = Toggle the use of a new buffer
gist-flippipe = Pipe the current buffer (or marked region) to the command
gist-flipreplace = Replace a string or a regular expression
gist-formatter = Invoke a program to format/arrange/manipulate the buffer
gist-fulljustify = Justify the entire file
gist-gotodir = Go to directory
gist-gotoline = Go to line and column number
gist-help = Display this help text
gist-home = Go to beginning of current line
gist-indent = Indent the current line (or marked lines)
gist-insert = Insert another file into current buffer (or into new buffer)
gist-justify = Justify the current paragraph
gist-lastfile = Go to the last file in the list
gist-lastline = Go to the last line of the file
gist-left = Go back one character
gist-linter = Invoke the linter, if available
gist-location = Display the position of the cursor
gist-mark = Mark text starting from the cursor position
gist-newer = Recall the next search/replace string
gist-nextanchor = Jump forward to the nearest anchor
gist-nextbuf = Switch to the next file buffer
gist-nextline = Go to next line
gist-nextpage = Go one screenful down
gist-nextword = Go forward one word
gist-older = Recall the previous search/replace string
gist-parabegin = Go to beginning of paragraph; then of previous paragraph
gist-paraend = Go just beyond end of paragraph; then of next paragraph
gist-paste = Paste the contents of cutbuffer at current cursor position
gist-prepend = Toggle prepending
gist-prevanchor = Jump backward to the nearest anchor
gist-prevbuf = Switch to the previous file buffer
gist-prevline = Go to previous line
gist-prevpage = Go one screenful up
gist-prevword = Go back one word
gist-recordmacro = Start/stop recording a macro
gist-redo = Redo the last undone operation
gist-regexp = Toggle the use of regular expressions
gist-replace = Replace a string or a regular expression
gist-right = Go forward one character
gist-runmacro = Run the last recorded macro
gist-savefile = Save file without prompting
gist-scrollleft = Scroll the viewport a tabsize to the left
gist-scrollright = Scroll the viewport a tabsize to the right
gist-spell = Invoke the spell checker, if available
gist-suspend = Suspend the editor (return to the shell)
gist-tab = Insert a tab at the cursor position (or indent marked lines)
gist-tofiles = Go to file browser
gist-tosearch = To Search
gist-undo = Undo the last operation
gist-unindent = Unindent the current line (or marked lines)
gist-verbatim = Insert the next keystroke verbatim
gist-whereis = Search forward for a string or a regular expression
gist-wherewas = Search backward for a string or a regular expression
gist-wordcount = Count the number of lines, words, and characters
gist-writeout = Write the current buffer (or the marked region) to disk
gist-zap = Throw away the current line (or marked region)

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
key-case_sens = Case sensitive
key-regexp = Reg.expression
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
key-prevfile = Prev File
key-nextfile = Next File
key-go_to_dir = Go To Dir
key-verbatim_input = Verbatim
key-anchor = Anchor
key-center = Center
key-comment = Comment Lines
key-complete = Complete
key-cycle = Cycle
key-formatter = Formatter
key-full_justify = Full Justify
key-indent = Indent
key-linter = Linter
key-next_word = Next Word
key-prev_word = Prev Word
key-record_macro = Record
key-run_macro = Run Macro
key-savefile = Save
key-spell = Spell Check
key-suspend = Suspend
key-to_files = Browse
key-unindent = Unindent
key-zap = Zap
key-scroll_left = Scroll Left
key-scroll_right = Scroll Right
key-chop_left = Chop Left
key-chop_right = Chop Right
key-word_count = Word Count
key-up_to_anchor = Up to anchor
key-down_to_anchor = Down to anchor
key-older = Older
key-newer = Newer
key-dos_format = DOS Format
key-backup_file = Backup File
key-append = Append
key-prepend = Prepend
key-no_conversion = No Conversion
key-execute_command = Execute Command
key-new_buffer = New Buffer
key-pipe_text = Pipe Text
key-unbound = Unbound Key
# ---------- 宏录制（对应 winio.c record_macro / run_macro） ----------
macro-recording = Recording a macro...
macro-stopped = Stopped recording
macro-cancelled = Cancelled
macro-while_recording = Cannot run macro while recording
macro-empty = Macro is empty

# ---------- 锚点（对应 search.c put_or_lift_anchor 等） ----------
anchor-placed = Placed anchor
anchor-removed = Removed anchor
anchor-jumped = Jumped to anchor
anchor-only = This is the only anchor
anchor-none = There are no anchors

# ---------- 字数统计（对应 text.c count_lines_words_and_characters） ----------
count-in_selection = In Selection:
count-line_one = line
count-line_many = lines
count-word_one = word
count-word_many = words
count-char_one = character
count-char_many = characters
count-report = {prefix}{lines} {lines_label},  {words} {words_label},  {chars} {chars_label}

# ---------- linter（对应 text.c do_linter） ----------
linter-none = No linter is defined for this type of file
linter-invoking = Invoking linter...
linter-ok = No lint problems found
linter-failed = Could not invoke linter
linter-zero_parsable = Got 0 parsable lines from command: {cmd}
linter-at_first = At first message
linter-at_last = At last message
linter-save_before = Save modified buffer before linting?
linter-unopened_file = This message is for unopened file {filename}, open it in a new buffer?
linter-no_messages = No messages for this file
signals-crash_message = Sorry! Nano crashed!  Code: {code}.  Please report a bug.
files-buffer_written_to = Buffer written to {name}
files-too_many_savefiles = Too many .save files
files-failed_to_open_stdin = Failed to open stdin
files-interrupted = Interrupted
files-could_not_reconnect_stdin = Could not reconnect stdin to keyboard
files-reading_from_keyboard = Reading data from keyboard; type ^D or ^D^D to finish.
search-invalid_modifier = Invalid search modifier "{ch}"
search-empty_search_string = Empty search string
search-invalid_number = Invalid number

# ---------- 彩蛋（对应 winio.c 的 do_credits，ENABLE_EXTRA） ----------
files-too_tiny = Too tiny
winio-credits_the_nano_text_editor = The nano text editor
winio-credits_version = version
winio-credits_brought_to_you_by = Brought to you by:
winio-credits_special_thanks_to = Special thanks to:
winio-credits_the_free_software_foundation = The Free Software Foundation
winio-credits_the_many_translators = the many translators and the TP
winio-credits_for_ncurses = For ncurses:
winio-credits_and_anyone_else = and anyone else we forgot...
winio-credits_thank_you = Thank you for using nano!

# ---------- minibar 的行数与缓冲区计数（对应 winio.c 的 minibar） ----------
winio-dos = DOS
winio-minibar_line_one = ({count} line)
winio-minibar_line_many = ({count} lines)
winio-minibar_line_dos_one = ({count} line, {fmt})
winio-minibar_line_dos_many = ({count} lines, {fmt})

# ---------- 补全列表（对应 files.c input_tab 的 "(more)"） ----------
files-more = (more)
