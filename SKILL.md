---
name: mget-metadata
description: 使用 mget 查询 TMDB，并为用户指定的本地电影或电视剧生成 Emby/Kodi 可读取的 NFO 和图片。适用于影视条目匹配与本地媒体资料整理。
---

# 使用 mget 整理媒体资料

`mget` 通过 TMDB ID 抓取资料，写入本地媒体目录。先确认条目和目标路径，再执行抓取；以命令的实际结果报告写入、跳过和失败项。

## 准备

- 用 `mget --help` 检查命令是否可用。若命令不存在且本仓库可用，可在仓库中运行 `cargo build --release`，再使用 `./target/release/mget`。不要为完成一次媒体整理而运行会使用 `sudo` 的 `install.sh`。
- 查询和抓取需要 TMDB API key。若尚未配置，请用户在本机设置 `TMDB_API_KEY`。不要索取、读取或回显密钥；`mget config list` 会遮蔽密钥，`mget config get tmdb.api_key` 和 `mget config set tmdb.api_key ...` 则可能将其显示在终端。

## 匹配条目

没有确定的 TMDB ID 时，用 `mget search "片名" --type movie` 或 `mget search "剧名" --type tv` 搜索；需要时添加 `--year`、`--lang`。根据标题、年份和媒体类型选择 ID。有多个合理候选且无法从用户提供的信息中确定时，请用户确认。

## 抓取到本地

电影的 `--output` 指向实际媒体文件（视频或 `.strm`），电视剧的 `--output` 指向剧集根目录。先从用户给出的路径或目录内容确认目标，不要猜测。电视剧会递归扫描文件名中的 `S01E02` 等季集编号，只为匹配的本地剧集生成剧集资料。

```sh
mget fetch movie --id 123456 --output "/media/movies/片名 (2024)/片名 (2024).mkv"
mget fetch tv --id 123456 --output "/media/tv/剧名"
```

示例 ID 和路径必须换成已确认的实际值。按用户要求选用 `--no-images`（只生成 NFO）、`--images-only`（只下载图片）或电视剧的 `--season`（只处理指定季；不处理剧集目录级文件）。其他参数以 `mget fetch --help` 为准。

## 写入边界与核对

- 目标范围不确定时，先加 `--dry-run` 查看计划写入和因已存在而会跳过的路径；它仍会请求 TMDB，但不会写文件。计划输出不等于实际写入结果。
- 默认会跳过已有文件。`--force` 或配置中的 `defaults.overwrite=true` 会覆盖它们；若已有目标文件且用户未授权覆盖，不要执行会覆盖的抓取。可用 `mget config list` 检查该配置。
- 执行后根据命令输出报告实际写入、跳过和失败的文件。需要结构化摘要时使用 `--format json`；不要把预览结果当作已完成的写入。退出码 `5` 表示部分文件失败，应查看 `errors` 并如实报告。
