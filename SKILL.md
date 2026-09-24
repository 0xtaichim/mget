---
name: mget-maintainer
description: 为 mget Rust 命令行项目提供架构说明和维护约定。Use when 修改本仓库的 CLI、TMDB 请求、配置、NFO 生成或媒体文件处理。
---

# mget 项目维护指南

## 快速了解

`mget` 是 Rust 命令行程序，通过 TMDB 查询媒体资料，为电影和电视剧生成 Emby/Kodi NFO，并下载图片。改动前先沿着命令入口读到对应模块，保持 CLI 参数、文件命名和 README 说明一致。

| 文件 | 职责 |
| --- | --- |
| `src/main.rs` | Clap 命令定义、命令流程、输出汇总 |
| `src/config.rs` | 配置结构、默认值、读写和环境变量覆盖 |
| `src/tmdb/` | TMDB API 客户端和响应类型 |
| `src/nfo.rs` | 电影、电视剧和剧集 NFO/XML 生成 |
| `src/media.rs` | 本地媒体扫描、季集识别和输出路径 |
| `src/download.rs` | 图片下载、并发控制和重试 |
| `src/error.rs` | 错误类型和进程退出码 |

## 修改约定

- 新增或调整命令时，在 `src/main.rs` 同步更新 Clap 定义、处理流程和 `README.md` 示例。
- 新增配置项时，在 `src/config.rs` 同步维护结构体、默认值、`set`、`get` 和 `list`；配置入口由 `src/main.rs` 提供。
- 所有 TMDB 请求经 `TmdbClient` 发送。API key 从用户配置或 `TMDB_API_KEY` 环境变量读取，不要写入源码、示例输出或提交记录。
- 修改 NFO 字段或 XML 时，检查 `src/nfo.rs` 的对应生成函数，以及 `src/main.rs` 传入的数据和图片路径。
- 修改媒体命名或电视剧扫描行为时，检查 `src/media.rs` 的扩展名、季集解析和路径函数，并确认 `src/main.rs` 的调用方式仍匹配。
- 图片下载选项和覆盖逻辑会受 `src/config.rs` 默认值及 `fetch` 命令参数共同影响，修改时同时检查两处。
- `Cargo.lock` 属于这个可执行程序仓库，应随依赖变更保留并提交。

## 常用命令

```sh
cargo fmt --check
cargo check
cargo build --release
```
