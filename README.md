# RPlayer

基于 Rust 的终端音乐播放器，支持歌词显示和 Vim 风格快捷键。

## 功能特性

- 本地音乐播放，自动扫描媒体库
- 支持 MP3、FLAC、WAV、OGG、M4A、AAC 格式
- LRC 歌词同步显示
- Vim 风格快捷键操作
- 多列播放列表（序号、歌名、歌手、专辑、时长，窄屏自动隐藏专辑列）
- 实时搜索过滤（歌曲名/歌手/专辑/文件名）
- 多种排序方式（歌曲名/歌手/专辑/文件夹）
- 多种播放模式（顺序/单曲循环/列表循环/随机）
- 后台异步扫描，启动不阻塞界面
- 增量扫描 + 缓存，二次启动秒开
- 跨平台支持（Linux / Windows）

## 截图

界面布局：左侧歌曲列表，右侧歌词显示，底部状态栏。

## 安装

### 从源码编译

```bash
# 克隆项目
git clone <repo-url> rplayer
cd rplayer

# 编译
cargo build --release

# 运行
./target/release/rplayer
```

### 指定音乐目录

通过命令行参数指定音乐目录（首次运行后自动保存到配置）：

```bash
./target/release/rplayer -d /path/to/music
```

### 交叉编译到 Windows

需要安装 MinGW 工具链：

```bash
# Ubuntu/Debian
sudo apt install mingw-w64

# 添加 target
rustup target add x86_64-pc-windows-gnu

# 编译
cargo build --target x86_64-pc-windows-gnu --release
```

产物：`target/x86_64-pc-windows-gnu/release/rplayer.exe`

## 使用方法

### 启动

直接运行即可，程序会自动扫描音乐目录：

- 如果通过 `-d` 指定了目录，使用该目录
- 否则读取配置文件中的设置
- 如果都没有，使用系统默认音频目录

### 快捷键

#### 导航

所有导航命令支持数字前缀（如 `5j` 向下移动 5 行，`10g` 跳转到第 10 行）。

| 按键 | 功能 |
|------|------|
| `j` / `↓` | 向下移动 |
| `k` / `↑` | 向上移动 |
| `d` / `PgDn` / `→` | 向下翻页 |
| `u` / `PgUp` / `←` | 向上翻页 |
| `g` | 跳到顶部 |
| `数字+g` | 跳到指定序号行 |
| `G` | 跳到底部 |
| `` ` `` / `'` | 跳到当前播放歌曲 |

#### 播放控制

| 按键 | 功能 |
|------|------|
| `Enter` | 播放选中歌曲 |
| `Space` | 暂停/继续 |
| `n` | 下一首 |
| `p` | 上一首 |
| `s` | 停止 |
| `h` | 快退 10 秒 |
| `l` | 快进 10 秒 |

#### 音量

| 按键 | 功能 |
|------|------|
| `+` / `=` | 音量 +10% |
| `-` | 音量 -10% |

#### 排序

| 按键 | 功能 |
|------|------|
| `t` | 切换排序方式 |

排序方式循环：歌曲名 → 歌手 → 专辑 → 文件夹 → 歌曲名

#### 播放模式

| 按键 | 功能 |
|------|------|
| `r` | 切换播放模式 |

播放模式循环：顺序播放 → 单曲循环 → 列表循环 → 随机播放 → 顺序播放

#### 搜索

| 按键 | 功能 |
|------|------|
| `f` / `/` | 进入搜索模式 |
| `Ctrl+F` | 切换搜索字段 |
| `Esc` | 退出搜索（清除过滤） |
| `Enter` | 确认搜索 |
| `F` | 清除当前过滤 |

搜索字段循环：歌曲/歌手 → 歌手 → 专辑 → 文件名 → 歌曲/歌手

搜索支持实时过滤，输入时即时显示匹配结果。

#### 其他

| 按键 | 功能 |
|------|------|
| `R` | 重新扫描媒体库（后台执行） |
| `?` | 显示帮助 |
| `q` / `Ctrl+C` | 退出 |

## 配置

配置文件 `config.toml` 自动生成在可执行文件同目录下：

```toml
music_folder = "/path/to/music"
```

## 歌词

将 LRC 格式歌词文件放在音乐文件同目录下，文件名与音乐文件相同即可自动加载。

LRC 格式示例：

```
[ti:歌曲标题]
[ar:歌手]
[00:12.00]第一行歌词
[00:17.20]第二行歌词
[01:23.45]第三行歌词
```

## 缓存机制

为加快启动速度，RPlayer 使用 JSON 缓存：

- 首次启动：全量扫描，解析所有文件元数据
- 后续启动：加载缓存 → 后台增量扫描（仅解析新增/修改的文件）
- 缓存文件存储在可执行文件同目录的 `cache/` 文件夹中
- 通过文件修改时间（mtime）判断是否需要重新解析

## 技术栈

| 组件 | 技术 |
|------|------|
| TUI 框架 | [ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm) |
| 音频解码 | [rodio](https://github.com/RustAudio/rodio) (symphonia) |
| 元数据解析 | [lofty](https://github.com/Serial-ATA/lofty) |
| 并行处理 | [rayon](https://github.com/rayon-rs/rayon) |
| 命令行解析 | [clap](https://github.com/clap-rs/clap) |

## 依赖

- Rust 1.75+
- ALSA (Linux，通常系统自带)

## License

MIT
