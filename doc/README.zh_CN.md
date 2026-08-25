# nanoxide(nax, nano + oxide)
语言： [English](../README.md) | 简体中文

nanoxide 是一个小而简单的终端文本编辑器，使用了 GNU nano 编辑器的界面以及键绑定，基于 9.2 版本。支持 Windows 和 Linux，并提供对应二进制下载。

许可证为 GPL-3.0。

## 文档目录
- [安装][#安装]
- [翻译][#翻译]
- [编译][#编译]

### 安装

`nanoxide`的二进制名称为`nax`。现在你只需要前往最新发行版。

- Windows：下载带`windows`的压缩包，全部解压到一个新目录下，并添加到`%PATH%`
- Linux：
  - Debian/Ubuntu: 下载`.deb`软件包并安装。
  - 其他：下载带`linux`的压缩包，全部解压后在该目录下运行`./install.sh`

### 翻译

nnanoxide 使用 fluent 库。原始语言文件在`locales/en-US.ftl`。所有的翻译都提取自 nano 的`.po`文件。只需 PR。

### 编译

nanoxide 使用 Rust 编写，所以你需要安装 [Rust][rust-lang] 后编译。

编译命令：
```bash
$ git clone https://github.com/TC999/nanoxide.git
$ cd nanoxide
$ cargo build --release
$ ./target/release/nax --version
nanoxide version 0.1.0
```

[rust-lang]: https://rust-lang.org/
