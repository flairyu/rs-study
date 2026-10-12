# projects —— 框架入门 demo

第 18–21 章的配套可运行项目。**每个项目是独立的 crate（不在同一个 workspace 里）**，
因为 Bevy / sqlx / ort 的依赖很重，混在一个 workspace 里会让依赖解析互相牵制。

| 目录 | 章节 | 框架 | 内容 |
|---|---|---|---|
| `axum-todo/` | 第 18 章 Web | `axum` + `tokio` | 内存版 todo REST API（GET/POST/DELETE + JSON + 错误映射） |
| `bevy-square/` | 第 19 章 游戏 | `bevy` | 2D 小游戏：键盘移动方块、计分文本、碰撞判定 |
| `sqlx-notes/` | 第 20 章 数据库 | `sqlx` + SQLite | 建表、CRUD、`query_as!` 编译期校验、连接池、事务 |
| `ai-infer/` | 第 21 章 AI | `candle` / 纯 `std` | 不下载权重的推理 demo：张量运算、2 层 MLP 前向、`Result` 错误处理、matmul 计时 |

每个目录里：

```
<project>/
├── Cargo.toml
├── README.md      ← 怎么跑、预期看到什么、怎么改造
└── src/main.rs
```

## 重要提醒

- **`axum-todo` / `bevy-square` / `sqlx-notes` 都需要网络**，第一次 `cargo run` 会下载很多依赖（Bevy 尤其大，可能要几分钟）。
- **`ai-infer` 是例外**：它的默认入口零依赖（`dependencies` 是空的）、离线可跑、秒级编译，
  因为带宽受限的环境里 candle 的依赖解析都跑不完（详见 [`ai-infer/README.md`](ai-infer/README.md)）。
- **它们不属于自动评测**：没有 `tests/chapter.rs`，验收标准写在各自的 `README.md` 里。
- 建议顺序：先跑 `axum-todo`（最实用），再 `sqlx-notes`（和前者能组合成完整后端），
  然后按兴趣选 `bevy-square` 或 `ai-infer`。
- 跑之前先确认 `cargo` 可用（见 [../docs/01-setup.md](../docs/01-setup.md)）。

```bash
cd /home/yxb/code/rs-study/projects/axum-todo
cargo run
# 另开一个终端：
curl localhost:3000/todos
curl -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"学 Rust"}'
```
