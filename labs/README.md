# labs —— 常用第三方库实验工作区

这里的东西**需要网络**拉取依赖（第一次 `cargo run` 会下载 crates）。
每个成员是独立的小程序，配一篇说明；第 17 章的文档就是它们的索引。

```bash
cd /home/yxb/code/rs-study/labs
cargo run -p l01_serde_json        # 跑某个实验
cargo build --workspace           # 一次编译全部（第一次比较慢）
```

> 如果网络慢，先按 [../docs/01-setup.md](../docs/01-setup.md) 第 2 节配国内镜像。
>
> ⚠️ **在受限环境里跑（例如只读的 `~/.cargo`）**：把 cargo 缓存指到工作区内，否则报
> `failed to create directory ... Read-only file system`：
> ```bash
> export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home
> ```
>
> 📌 **关于命名**：目录名带序号只是为了在文件浏览器里排序；cargo 的**包名不允许以数字开头**，
> 所以包名是 `lXX_` 前缀。`cargo run -p <包名>` 用的是包名（下表第一列）。

| 包名 | 覆盖的 crate | 学到的能力 |
|---|---|---|
| `l01_serde_json` | `serde`, `serde_json` | 序列化/反序列化、`#[derive(Serialize, Deserialize)]`、`serde_json::Value` |
| `l02_regex` | `regex` | 正则匹配、捕获组、`Captures`、性能陷阱（别在循环里编译） |
| `l03_clap` | `clap` | derive 风格 CLI、子命令、参数校验、自动 `--help` |
| `l04_errors` | `anyhow`, `thiserror` | 库用 `thiserror`、应用用 `anyhow`、错误链与 `context` |
| `l05_rayon` | `rayon` | 一行 `.par_iter()` 把顺序代码变并行 |
| `l06_rand_uuid` | `rand`, `uuid` | 随机数、分布、UUID v4/v7 生成 |
| `l07_tracing` | `tracing`, `tracing-subscriber` | 结构化日志、span、`RUST_LOG` 过滤 |
| `l08_reqwest` | `tokio`, `reqwest` | 异步 HTTP 请求、JSON 收发、超时与错误处理 |

## 约定

- 每个成员是 `bin` crate，`src/main.rs` 里 `fn main()` 直接跑，输出可读。
- 每个成员必须有 `README.md`：讲清「它解决什么问题」「什么时候别用它」「怎么跑」。
- 依赖版本写在各自的 `Cargo.toml` 里（不要用 workspace 依赖，方便单独升级）。
- 这些小程序的目的是**跑通 + 看懂**，不是生产代码；不要为了优雅牺牲可读性。
