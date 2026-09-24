---
name: mget-media-operator
description: 指导 AI 助手使用 mget 查询 TMDB，并为用户指定的本地电影和电视剧生成 NFO、下载图片。Use when 用户要求查找影视条目、整理本地媒体资料或生成 Emby/Kodi 元数据。
---

# 使用 mget 处理媒体资料

## 快速开始

先确认 `mget` 可用：

```sh
mget --help
```

如果命令不存在且当前仓库可用，在仓库目录运行 `cargo build --release`，之后用 `./target/release/mget` 执行。不要为了单次任务擅自运行 `install.sh`，它会用 `sudo` 将程序安装到 `/usr/local/bin`。

搜索或抓取需要 TMDB API key。缺少 key 时，告诉用户在本机设置 `TMDB_API_KEY`，不要让用户把 key 发到对话里。`mget config list` 会隐藏 key；不要运行 `mget config get tmdb.api_key`。`mget config set tmdb.api_key ...` 会把输入值回显到标准错误，不要通过工具传入真实 key。

## 搜索条目

```sh
mget search "片名" --type movie --year 2024 --lang zh-CN
mget search "剧名" --type tv --lang zh-CN
```

搜索默认类型是 `movie`，结果以 JSON 输出。根据标题、年份和类型匹配用户要找的条目；有多个合理匹配时，先把候选项列给用户确认，再用对应 TMDB ID 抓取。

## 处理电影

`--output` 必须是本地媒体文件路径，不是目录。程序会在媒体文件旁生成同名 `.nfo`，并保存可用的海报、背景图和 logo：

```sh
mget fetch movie --id 123456 --output "/media/movies/片名 (2024)/片名 (2024).mkv" --dry-run
mget fetch movie --id 123456 --output "/media/movies/片名 (2024)/片名 (2024).mkv"
```

示例中的数字 ID 要替换成搜索结果里的 ID。如果用户只要 NFO，添加 `--no-images`；如果只要图片，添加 `--images-only`。先从用户提供的路径或目录内容确定实际媒体文件，不要猜测路径。仅在用户要求生成或更新媒体资料时执行实际写入。

## 处理电视剧

`--output` 是电视剧目录。程序会递归扫描媒体文件，并从文件名识别 `S01E02` 形式的季集编号：

```sh
mget fetch tv --id 123456 --output "/media/tv/剧名" --dry-run
mget fetch tv --id 123456 --output "/media/tv/剧名"
```

完整抓取会处理 `tvshow.nfo`、匹配到的剧集 NFO 和图片。`--season 1` 只处理指定季，也会跳过电视剧目录级文件。没有匹配到本地剧集文件时，程序会跳过对应剧集。

## 写入和覆盖

- 写文件前确认 `--output` 指向用户指定的媒体文件或电视剧目录。`--dry-run` 会查询 TMDB 并显示计划路径，但不会写入文件。
- 默认情况下已存在的输出会跳过。若 `defaults.overwrite` 为 `true` 且目标文件已存在，未经用户明确同意不要执行抓取，因为程序会覆盖这些文件。用户明确要求覆盖时传 `--force`。
- `--lang` 和 `--image-size` 可覆盖单次命令的默认值。`--format json` 可让 `fetch` 将写入、跳过和错误摘要输出为 JSON。
- 完成后报告实际写入、跳过和失败的文件；不要把未执行的 dry-run 说成已完成。
