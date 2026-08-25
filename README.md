# nanoxide(nax, nano + oxide)

Language: English | [简体中文](doc/README.zh_CN.md) |

nanoxide is a small and simple TUI text editor, it copied the interface and key bindings of the GNU nano editor, based on 9.2 version. It has first class support on Windows and Linux, with binary downloads available for every release.

Mono-licensed under GPL-3.0.

## Documentation quick links
- [Installation](#Installation)
- [Translation](#Translation)
- [Building](#Building)

### Installation

The binary name for nanoxide is `nax`. Now you just go to the latest release.

- Windows: Download which name with `windows` zip, extract all files to a new folder and add it to `%PATH%`
- Linux:
  - Debian/Ubuntu: Download the `.deb` file and install.
  - Others: Download which name with `linux` zip, exract all files and run `./install.sh`

### Translation

nanoxide uses fluent lib to i18n. The Original language file is `locales/en-US.ftl`. All the translation extracted from nano's po file. Just PR.

### Building

nanoxide is written in Rust, so you'll need to install [Rust][rust-lang] to compile it.

To build nanoxide:
```bash
$ git clone https://github.com/TC999/nanoxide.git
$ cd nanoxide
$ cargo build --release
$ ./target/release/nax --version
nanoxide version 0.1.0
```

[rust-lang]: https://rust-lang.org/
