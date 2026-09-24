# mget

`mget` 是一个 Rust 命令行工具，用 TMDB 元数据为本地电影和电视剧生成 Emby/Kodi 可读取的 NFO 文件，并下载海报、背景图和剧集缩略图。

## 安装

需要安装 Rust 和 Cargo。克隆仓库后可以构建并安装到 Cargo 的可执行文件目录：

```sh
git clone https://github.com/0xtaichim/mget.git
cd mget
cargo install --path .
```

也可以只构建程序，生成的二进制文件位于 `target/release/mget`：

```sh
cargo build --release
```

仓库提供的 `install.sh` 会构建 release 版本，并用 `sudo` 安装到 `/usr/local/bin/mget`。

## 配置 TMDB

查询和抓取元数据都需要 TMDB API key。可以通过环境变量提供：

```sh
export TMDB_API_KEY="YOUR_TMDB_API_KEY"
```

也可以用 `mget config set tmdb.api_key YOUR_TMDB_API_KEY` 保存到用户配置（输出中会遮蔽密钥）。v3 API key 和 v4 读取令牌（以 `eyJ` 开头）都可以使用。配置文件保存在操作系统的用户配置目录下（`mget config path` 可查看路径，Unix 上权限为 `600`），不需要放进媒体目录或仓库。`mget config list` 会隐藏 API key，`mget config get tmdb.api_key` 则会输出明文。

## 搜索

搜索默认查找电影，结果以 JSON 输出：

```sh
mget search "The Matrix"
mget search "The Matrix" --type movie --year 1999
mget search "Breaking Bad" --type tv --lang zh-CN
```

`--type` 支持 `movie` 和 `tv`，`--year` 按上映年份筛选电影或首播年份筛选电视剧，`--lang` 覆盖本次请求的语言。

## 抓取电影

`--output` 指向本地媒体文件。程序会在旁边生成同名 `.nfo`，并按 TMDB 返回的资源下载图片：

```sh
mget fetch movie --id 123456 --output "/media/movies/Movie (2024)/Movie (2024).mkv"
```

把示例里的数字 ID 换成搜索结果中的电影 ID。

可能生成 `Movie (2024).nfo`、`Movie (2024)-poster.jpg`、`Movie (2024)-fanart.jpg` 和 `Movie (2024)-clearlogo.png`。如果不需要图片，可以加 `--no-images`；如果只要图片，可以加 `--images-only`。

## 抓取电视剧

`--output` 指向电视剧目录。程序会递归扫描目录中的媒体文件，并从文件名中识别 `S01E02` 形式的季和集编号。以 `.` 开头的文件（如 macOS 的 `._` 文件）和 `@eaDir` 等 NAS 系统目录会被忽略；同一集有多个文件（不同版本）时，每个文件都会生成对应的 NFO 和缩略图：

```sh
mget fetch tv --id 123456 --output "/media/tv/Show Name"
mget fetch tv --id 123456 --output "/media/tv/Show Name" --season 1
```

把示例里的数字 ID 换成搜索结果中的电视剧 ID。

完整抓取会生成 `tvshow.nfo`、剧集 NFO、可用的剧集缩略图，以及剧集海报等图片。本地存在 `S00Exx` 文件时也会处理特别篇（季海报为 `season-specials-poster.jpg`）。指定 `--season` 时只处理该季，不生成电视剧目录级别的文件。

## 常用选项

电影和电视剧的 `fetch` 命令都支持以下选项：

| 选项 | 作用 |
| --- | --- |
| `--no-images` | 只生成 NFO，不下载图片 |
| `--images-only` | 只下载图片，不生成 NFO |
| `--force` | 覆盖已存在的输出文件 |
| `--dry-run` | 显示计划写入或下载的文件及会被跳过的已有文件，不写入文件 |
| `--lang LANG` | 覆盖本次请求的语言 |
| `--image-size SIZE` | 覆盖图片尺寸，例如 `w500`、`w780` 或 `original` |
| `--format json` | 将抓取结果以 JSON 写到标准输出 |

电视剧命令还支持 `--season NUMBER`，只处理指定季。`--dry-run` 仍会请求 TMDB 读取元数据。

所有文件都先写入临时文件再原子替换，中断不会留下半截文件。`--format json` 输出 `files_written`、`files_skipped`、`errors`；`--dry-run` 时另有 `files_planned`。

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功 |
| `1` | 未找到（搜索无结果、TMDB ID 不存在） |
| `2` | 网络或 TMDB API 错误 |
| `3` | 配置错误（如未设置 API key） |
| `4` | 文件系统错误 |
| `5` | 抓取完成但有部分文件失败（详见 `errors`） |

## 配置

`mget config list` 以 JSON 显示当前配置；`get`、`set` 和 `reset` 可读取、更新或恢复配置项。`set` 一个空字符串可清除可选项（如 `mget config set network.proxy ""`）：

```sh
mget config get defaults.language
mget config set defaults.language zh-CN
mget config list
```

| 配置项 | 默认值 | 说明 |
| --- | --- | --- |
| `tmdb.api_key` | 未设置 | TMDB API key，也可通过 `TMDB_API_KEY` 环境变量设置 |
| `defaults.language` | `zh-CN` | 元数据语言 |
| `defaults.fallback_language` | `en` | 简介等字段为空时尝试使用的语言 |
| `defaults.image_size` | `original` | 下载图片的尺寸 |
| `defaults.image_language` | 未设置 | 图片语言偏好，可用逗号分隔多个语言代码 |
| `defaults.overwrite` | `false` | 是否默认覆盖现有文件 |
| `network.proxy` | 未设置 | HTTP/HTTPS 代理地址 |
| `network.concurrent_downloads` | `4` | 并发图片下载数 |
| `network.timeout` | `30` | 网络请求超时秒数 |

单次命令传入的 `--lang`、`--image-size` 和 `--force` 会覆盖对应默认值。

## 开发

AI 助手使用 `mget` 搜索资料并处理本地媒体文件的流程见 [SKILL.md](SKILL.md)。

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets
```

代码结构：

| 模块 | 职责 |
| --- | --- |
| `cli` | 命令行参数定义 |
| `commands/` | 各子命令的编排：`search`、`fetch`（电影/电视剧）、`config` |
| `plan` | 收集待写文件，统一执行跳过判断、dry-run、NFO 写入与并发下载 |
| `tmdb/` | TMDB 客户端、响应类型、图片选择 |
| `nfo/` | NFO XML 生成 |
| `media` | 本地剧集扫描与 Emby/Kodi 文件命名 |
| `http` | 共享 HTTP 客户端与重试策略 |
| `config`、`error`、`fsutil` | 配置、错误与退出码、原子写文件 |
