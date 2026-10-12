# 第 17 章 · 常用第三方库

> **一句话定位**：`std` 只给地基，这一章带你认门——十几个「几乎每个 Rust 项目都会装」的 crate，各自解决什么问题、最小怎么用、什么时候**别**用。
> **预计时间**：120 分钟（读 60 + 动手 60）
> **前置**：第 16 章（标准库巡礼）、第 6 章（错误处理）、第 8 章（迭代器）、第 13 章（async/await）
> **配套实验**：`labs/`（8 个可运行小程序，本文档是它们的索引）

---

## 0. 先建立锚点

| 维度 | C / C++ | Python | Rust |
|---|---|---|---|
| 去哪找 | GitHub、发行版包、vendor 目录 | PyPI（`pip`） | **crates.io**（`cargo`） |
| 怎么装 | 编译进源码 / `apt install libfoo-dev` | `pip install requests` | `cargo add serde` |
| 版本/检查 | 祈祷 ABI 一致、链接期才报错 | `requirements.txt` 写死、运行时才炸 | `Cargo.toml` 写 `"1.0"`、`Cargo.lock` 锁定、编译期全查 |
| 文档/组织 | man page；无统一标准 | Read the Docs；包即目录 | **docs.rs**（每版本自动生成）；**小而专的 crate 组合** |

Rust 生态的原则是「**一个 crate 只做一件事**」：`serde` 只给两个 trait 和 derive，格式交给 `serde_json`/`toml`/`bincode`；`tokio` 给运行时，HTTP 交给 `reqwest`，Web 交给 `axum`。学生态 = 学一张**「谁和谁搭配」的关系图**。

**怎么挑 crate**：① **下载量**（周下载三位数以下别用在关键路径）；② **最近更新**（两年没发版但 issue 一堆 = 危险）；③ **unsafe 比例**（docs.rs 侧栏 `% of crate uses unsafe`）；④ **文档质量**（有没有可运行示例）；⑤ **许可证与维护者**（GitHub 人数、CI、issue 响应）；⑥ **feature 设计**（默认 feature 是否拖家带口，可 `default-features = false` 砍）；⑦ 有 **`no_std`** 需求时看它是否依赖 `std`。心法：**先看它是不是「事实标准」**——JSON → `serde_json`，正则 → `regex`，CLI → `clap`，异步 → `tokio`，随机数 → `rand`。

本文所有 `rust` 代码块都在 **rustc 1.98.1** 下实际编译运行过（版本只作参考）：`serde`/`serde_json` 1.0.229/1.0.154、`regex` 1.13.1、`time` 0.3.55、`clap` 4.6.7、`anyhow`/`thiserror` 1.0.104/2.0.21、`rayon`/`itertools` 1.12.0/0.14.0、`rand`/`uuid` 0.9.5/1.28.0、`tracing`/`tracing-subscriber` 0.1.44/0.3.23、`tokio` 1.53.2、`reqwest` 0.13.5（`rustls`）、`indexmap` 2.14.2、`once_cell` 1.21.4、`tempfile` 3.27.0。

> 💡 在受限环境（`~/.cargo` 只读）里跑 `labs/`，先设 `export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home`；首次编译要下载依赖，网络慢就先按 [docs/01-setup.md](../01-setup.md) 第 2 节配国内镜像。

---

## 1. `serde` + `serde_json`：序列化的事实标准

**解决什么问题**：结构体 ↔ JSON/TOML/YAML/二进制互转，不用手写解析。`serde` 是框架（trait + derive），`serde_json` 是格式实现。

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    name: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 { 8080 }

fn main() {
    // 缺 port 字段走 default；rename_all/rename 可批量或单独换命名
    let cfg: Config = serde_json::from_str(r#"{"name":"demo"}"#).unwrap();
    assert_eq!(cfg.port, 8080);
    println!("{}", serde_json::to_string(&cfg).unwrap());
    // 结构未知/可变时用 Value 动态取，取不到返回 None 而不是 panic
    let v: serde_json::Value = serde_json::from_str(r#"{"a":{"b":[1,2]}}"#).unwrap();
    println!("{} {:?}", v["a"]["b"][0], v["missing"].as_str());   // 1 None
}
```

字段级属性是精华：`default`、`rename_all`、`skip_serializing_if = "Option::is_none"`、`flatten`；反序列化失败时错误带**行列号**。
**什么时候别用**：① 只解析一个数字，直接 `str::parse`；② 要保留 JSON 的**注释和格式**做往返修改，用 `json5`/`toml_edit`；③ 结构极动态且只要一两个字段，`Value` 足够。

📦 **实验**：[`labs/01_serde_json/`](../../labs/01_serde_json/)（包名 `l01_serde_json`）

---

## 2. `regex`：正则，但没有回溯

**解决什么问题**：匹配、捕获组、替换。`regex` 用有限自动机实现，**保证线性时间**——没有灾难性回溯，这是它和 PCRE 最大的区别。

```rust
fn main() {
    // 编译只做一次；(?P<name>...) 是命名捕获组
    let re = regex::Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2}) (?P<level>[A-Z]+) (?P<msg>.*)$")
        .unwrap();
    let caps = re.captures("2026-10-12 ERROR disk full").unwrap();
    println!("{} {} {}", &caps["date"], &caps["level"], &caps["msg"]);
    // 多行批量匹配；捕获组借的是局部值，要 to_string 拷出来
    let msgs: Vec<String> = re.captures_iter("2026-10-12 ERROR a\n2026-10-12 INFO b")
        .map(|c| c["msg"].to_string()).collect();
    println!("{msgs:?}");
}
```

**什么时候别用**：① **绝不在循环里 `Regex::new`**——编译很贵，用 `OnceLock`/`LazyLock` 缓存（第 14 节）；② 只判断子串用 `str::contains`，简单分隔用 `split`；③ 需要**回溯引用 `\1`、环视 `(?=...)`**——`regex` 明确不支持，改用 `fancy-regex`；④ 解析 HTML/JSON 用专门解析器。

📦 **实验**：[`labs/02_regex/`](../../labs/02_regex/)（包名 `l02_regex`）

---

## 3. `time`（或 `chrono`）：日期、时区与格式化

**解决什么问题**：`std` 只有 `Duration`/`Instant`/`SystemTime`，**没有日历**。算「三个月后」「解析用户输入」得靠 `time` 或 `chrono`；二选一，**不要同时依赖**——`time` 更轻、API 更严谨，`chrono` 历史更久。

```rust
fn main() {
    use time::macros::{format_description, offset};
    use time::{Duration, OffsetDateTime, PrimitiveDateTime};
    // 格式串在编译期检查，不受本地 locale 影响
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    // 无时区的时间要用 PrimitiveDateTime 解析（OffsetDateTime 没有 parse）
    let dt = PrimitiveDateTime::parse("2026-10-12 08:30:00", &fmt).unwrap()
        .assume_offset(offset!(+8));
    println!("{}", dt.unix_timestamp());                            // 1791765000
    println!("{}", (dt + Duration::days(7)).format(&fmt).unwrap());
    let now = OffsetDateTime::now_utc();
    println!("{} {}", now.format(&fmt).unwrap(), now.year());
}
```

**机器时间用 `unix_timestamp()`**（i64，存库/传网络），人类时间才格式化；跨时区用 `OffsetDateTime`，只关心「哪一天」用 `Date`。
**什么时候别用**：① 只测耗时用 `Instant`（第 16 章）；② 只要当前时间戳，`SystemTime` 足够；③ 需要**完整时区数据库**（历史夏令时规则）要 `time-tz`/`chrono-tz`，`time` 默认只带 UTC 偏移。

> ⚠️ 本节**不在 `labs/` 里演示**（8 个实验没有日期成员）。

---

## 4. `clap`：命令行参数解析

**解决什么问题**：手写 `env::args()` 要处理 `--key=value`、短选项合并、子命令、`--help`、错误退出。`clap` 的 derive 让你**用结构体描述参数**，其余全自动。

```rust
fn main() {
    use clap::{Parser, Subcommand};
    use std::path::PathBuf;
    #[derive(Debug, Parser)]
    #[command(name = "logtool", version, about = "从日志里抓点什么")]
    struct Cli {
        /// 输入文件；不写则读 stdin
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// 只要哪个级别；value_parser 限定取值
        #[arg(short, long, default_value = "error", value_parser = ["error", "warn", "info"])]
        level: String,
        /// 详细程度，可重复 -v -v
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
        #[command(subcommand)]
        command: Option<Cmd>,
    }
    #[derive(Debug, Subcommand)]
    enum Cmd {
        /// 统计各级别出现次数
        Count {
            #[arg(short, long, default_value_t = 10)]
            top: usize,
        },
        /// 打印前几行
        Head,
    }
    let cli = Cli::parse();          // 生产代码用 parse() 读真实 argv
    match cli.command {
        Some(Cmd::Count { top }) => println!("count {top}"),
        _ => println!("none"),
    }
}
```

**注释就是 `--help` 文本**：实测 `--help` 会列出参数、`[default: error]`、`[possible values: ...]`；传 `--level nope` 会在进 `main` 之前报错退出。`Option<T>` = 可选，`default_value_t` = 带默认值。
**什么时候别用**：① 参数固定且只有一两个，手写 `env::args()` 更省依赖；② 二进制体积敏感；③ 想完全控制帮助排版——先查 `clap_complete`（生成 shell 补全）能否满足。**`clap` 3 与 4 的 API 差别很大**。

📦 **实验**：[`labs/03_clap/`](../../labs/03_clap/)（包名 `l03_clap`，可跑 `cargo run -p l03_clap -- --help`）

---

## 5. `anyhow` + `thiserror`：错误处理的两把锤子

**解决什么问题**：第 16 章的 `Box<dyn Error>` 能传播错误却**抹掉类型**。社区惯例：**库用 `thiserror`** 定义具体错误枚举（调用者能 `match`），**应用用 `anyhow`** 把「什么错 + 在哪出错」串成链。

```rust
mod errors {
    use thiserror::Error;
    #[derive(Debug, Error)]
    pub enum AppError {
        #[error("读取文件 {path} 失败")]
        Io { path: String, #[source] source: std::io::Error },   // #[source] 保留根因
        #[error("端口 {0} 不合法")]
        BadPort(u16),
    }
}
fn main() {
    use anyhow::{bail, Context, Result};
    fn load(path: &str) -> Result<String> {
        let text = std::fs::read_to_string(path).with_context(|| format!("读配置 {path}"))?;
        if text.trim().is_empty() {
            bail!("配置 {path} 是空的");        // 直接构造并返回错误
        }
        Ok(text)
    }
    println!("{}", errors::AppError::BadPort(0));   // 库风格：具体类型可 match
    let err = load("/definitely/missing.json").unwrap_err();
    println!("{err}");            // 读配置 /definitely/missing.json
    println!("{err:#}");          // …: No such file or directory (os error 2)
    for cause in err.chain() {    // 逐层遍历根因
        println!("  - {cause}");
    }
}
```

**什么时候别用**：① `anyhow` **不该出现在库的公开 API**——调用者只能读字符串、无法区分错误种类；② 错误种类单一（如 `io::Result`）时直接返回原错误；③ 别给每个函数都套 `context`，只在**跨模块边界**加；④ `thiserror` 每个变体都要写 `#[error(...)]`。

📦 **实验**：[`labs/04_errors/`](../../labs/04_errors/)（包名 `l04_errors`）

---

## 6. `rayon`：把 `.iter()` 换成 `.par_iter()`

**解决什么问题**：数据并行。串行迭代器逻辑改成 `par_iter` 就用工作窃取线程池在多核上跑——**不引入 `async`，不改算法**。

```rust
fn main() {
    use rayon::prelude::*;
    fn heavy(n: u64) -> f64 {
        (0..n).map(|i| ((i as f64).sin().abs()).sqrt()).sum()
    }
    let data: Vec<u64> = (1..=200_000).collect();
    let t0 = std::time::Instant::now();
    let seq: f64 = data.iter().map(|&x| heavy(x % 1000 + 1)).sum();
    let t_seq = t0.elapsed();
    let t1 = std::time::Instant::now();
    let par: f64 = data.par_iter().map(|&x| heavy(x % 1000 + 1)).sum();
    let t_par = t1.elapsed();
    // 实测：seq 2.50s / par 0.42s；浮点并行求和有末位差异，不能断言完全相等
    println!("seq {t_seq:?} / par {t_par:?} (diff {:.2e})", (seq - par).abs());
    // 并行排序
    let mut v: Vec<u64> = (1..=1000).map(|x| x * 7 % 1000).collect();
    v.par_sort_unstable();
    println!("{:?}", &v[..3]);
}
```

`rayon` 要求闭包是 `Fn + Send + Sync`、元素能安全共享。**浮点求和结合顺序会变、结果可能有末位差异**；要逐位一致就用整数运算或固定分块。
**什么时候别用**：① 任务太短——线程调度比计算还贵，**先量再改**；② 任务涉及 I/O，那是 `tokio` 的领域；③ 需要浮点结果逐位一致；④ 单核环境只是白加依赖。

📦 **实验**：[`labs/05_rayon/`](../../labs/05_rayon/)（包名 `l05_rayon`）

---

## 7. `itertools`：迭代器的「缺失适配器」

**解决什么问题**：标准库 `Iterator` 缺了组合、去重、分组、排序、join，`itertools` 一次性补齐。

```rust
fn main() {
    use itertools::Itertools;
    let words = ["b", "a", "c", "a"];
    println!("{:?}", words.iter().copied().unique().collect::<Vec<_>>());   // ["b","a","c"]
    println!("{:?}", words.iter().copied().sorted().collect::<Vec<_>>());   // ["a","a","b","c"]
    println!("{} {:?}", words.iter().join(","), words.iter().counts());
    // chunk_by 按「相邻且键相同」分组，不是全局分组！
    let grouped: Vec<(usize, Vec<usize>)> = (1..=6).chunk_by(|n| n % 3)
        .into_iter().map(|(k, g)| (k, g.collect())).collect();
    println!("{grouped:?}");   // [(1,[1,4]), (2,[2,5]), (0,[3,6])]
}
```

**什么时候别用**：① 只用一两个适配器，手写 `fold`/`HashMap` 更透明；② 要「全局按 key 分组」得先排序或自己用 `HashMap`——`chunk_by` 只合并**相邻**元素，这是最常见的误用；③ 热路径上注意 `sorted` 会收集全部元素。

> ⚠️ 本节**不在 `labs/` 里演示**。

---

## 8. `rand`：随机数

**解决什么问题**：`std` 完全没有随机数。`rand 0.9` 的入口是 `rand::rng()`（旧版 `thread_rng()`），取范围用 `random_range`（旧版 `gen_range`）。

```rust
fn main() {
    use rand::{random, rng, Rng};
    let mut r = rng();                               // 线程局部的 CSPRNG
    println!("{} {} {}", r.random_range(0..10), r.random_bool(0.5), random::<u32>());
    // 非均匀分布要显式构造 Uniform；fill 生成随机字节
    let d = r.sample(rand::distr::Uniform::new(1, 7).unwrap());
    let mut bytes = [0u8; 4];
    r.fill(&mut bytes);
    println!("{d} {bytes:?}");
}
```

**什么时候别用**：① 生成密钥/会话 token 请明确用 `getrandom`/`OsRng`，别依赖「默认就是安全的」；② 需要**可复现**的实验用 `StdRng::seed_from_u64`；③ 只为打散哈希冲突——`HashMap` 内部已经做了；④ **版本陷阱**：`gen_range`/`thread_rng` 是 0.8 及以前。

📦 **实验**：[`labs/06_rand_uuid/`](../../labs/06_rand_uuid/)（包名 `l06_rand_uuid`）

---

## 9. `uuid`：唯一 ID

**解决什么问题**：数据库主键、请求 ID、节点标识。**v4 纯随机**（无序、索引不友好）、**v7 时间前缀有序**（插入 B-tree 索引性能好，新项目优先）。

```rust
fn main() {
    use uuid::Uuid;
    let v4 = Uuid::new_v4();
    println!("{v4} {}", v4.simple());          // 默认带连字符；simple() 是 32 位 hex
    println!("{}", Uuid::now_v7());            // 前 48 位是毫秒时间戳
    // 解析失败返回 Err，不 panic
    let parsed = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    println!("{} {}", parsed.get_version_num(), parsed.is_nil());
}
```

**什么时候别用**：① 单机自增 `u64` 就够，更省空间；② 要更短、仍按时间有序的 ID 用 ULID；③ 要「人能读」的订单号，自己设计编码；④ 把 UUID 当**安全令牌**要谨慎：v4 的 122 位随机性够，但格式可枚举，涉及权限要另配签名。

📦 **实验**：[`labs/06_rand_uuid/`](../../labs/06_rand_uuid/)（包名 `l06_rand_uuid`）

---

## 10. `tracing` + `tracing-subscriber`：结构化日志

**解决什么问题**：`println!` 没有级别、字段、span。`tracing` 是结构化、可组合的诊断门面（`log` 的升级版）；`tracing-subscriber` 决定「输出到哪、怎么过滤」，`fmt` 是默认文本订阅器。

```rust
fn main() {
    use tracing::{debug, info, info_span, warn};
    use tracing_subscriber::EnvFilter;
    // RUST_LOG 存在就用它，否则默认 info；env-filter 要显式开 feature
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
    info!(user = "alice", port = 8080, "服务启动");   // 结构化字段
    warn!("磁盘剩余 {:.1}%", 3.5);                    // 也能 format 插值
    debug!("这行只在 RUST_LOG=debug 时出现");
    let span = info_span!("handle_request", method = "GET", path = "/health");
    let _guard = span.enter();                        // guard 存活期间，日志都带这个 span
    let child = info_span!("db_query");
    let _g = child.enter();
    info!(rows = 3, "查询完成");
}
```

实测输出里 `handle_request{method="GET" path="/health"}:db_query:` 就是 span 嵌套前缀：

```
2026-10-12T07:36:04.298869Z  INFO 服务启动 user="alice" port=8080
2026-10-12T07:36:04.299007Z  INFO handle_request{...}:db_query: 查询完成 rows=3
```

**什么时候别用**：① 一次性脚本 `println!` 就够；② **库不要自己 `init()` 订阅器**，只发事件，让应用的 `main` 决定输出去哪；③ 同步 CLI 要的是进度条，那是 `indicatif`；④ 以为 `env-filter` 默认就有；⑤ guard 别写成 `_`（立刻 drop，span 失效），要写 `_guard`。

📦 **实验**：[`labs/07_tracing/`](../../labs/07_tracing/)（包名 `l07_tracing`，可跑 `RUST_LOG=debug cargo run -p l07_tracing`）

---

## 11. `tokio`：异步运行时

**解决什么问题**：第 13 章说过 `std` 只有 `Future` 接口、没有调度器。`tokio` 就是调度器：多线程工作窃取、`spawn`、定时器、channel，外加异步 I/O。**`async fn` 只是描述，`tokio` 才让它跑起来**。

```rust
fn main() {
    // 生产代码通常用 #[tokio::main]；这里显式建运行时，看清层次
    let rt = tokio::runtime::Runtime::new().unwrap();     // 等价于多线程运行时
    rt.block_on(async {
        let h = tokio::spawn(async { 1 + 1 });            // 任务并发跑
        println!("{}", h.await.unwrap());
        let (tx, mut rx) = tokio::sync::mpsc::channel(4); // 异步 channel
        tokio::spawn(async move { tx.send("hello").await.unwrap() });
        println!("{:?}", rx.recv().await);
        // 给可能卡住的异步操作套超时（下面这个几乎必然超时）
        let slow = tokio::time::sleep(std::time::Duration::from_secs(10));
        let timed_out = tokio::time::timeout(std::time::Duration::from_millis(1), slow)
            .await.is_err();
        println!("{timed_out}");                          // true
    });
}
```
`#[tokio::main]` 只是这段的语法糖（feature `macros` + `rt-multi-thread`）。**异步是「等待」场景的优化，不是并发加速器**；CPU 密集任务丢进 `spawn_blocking`，否则会堵住执行器。
**什么时候别用**：① 纯 CLI/批处理/计算密集——同步代码更快更简单；② 只有一个网络调用，`ureq` 就够；③ 库想支持多个运行时，就别硬绑 `tokio`；④ **feature 默认全空**：`rt-multi-thread`、`macros`、`time`、`sync`、`fs` 按需开；⑤ 在 `async` 里调用阻塞 API（`std::fs`、`std::thread::sleep`）是最常踩的坑。

📦 **实验**：[`labs/08_reqwest/`](../../labs/08_reqwest/)（包名 `l08_reqwest`）

---

## 12. `reqwest`：HTTP 客户端

**解决什么问题**：`std` 只有裸 TCP。`reqwest` 是生态里最常用的 HTTP 客户端：连接池、超时、重定向、JSON、TLS 开箱即用。

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Post { id: u32, title: String }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 客户端要复用（内部有连接池），不要每次请求都新建
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))    // 超时是必选项
        .build()?;
    let post: Post = client.get("https://jsonplaceholder.typicode.com/posts/1")
        .send().await?
        .error_for_status()?          // 4xx/5xx 转成 Err，别只看连没连上
        .json()                       // 需要 feature "json"
        .await?;
    println!("{} {}", post.id, post.title);
    let resp = client.post("https://jsonplaceholder.typicode.com/posts")
        .json(&serde_json::json!({ "title": "hi", "userId": 1 }))
        .send().await?;
    println!("{}", resp.status());
    Ok(())
}
```

**什么时候别用**：① 同步程序不想引入 `tokio`，用 `ureq`；② 要细粒度控制 HTTP/2 帧与连接生命周期，用 `hyper`；③ 只解析 URL 字符串，用 `url`；④ **feature 陷阱**：默认走 `native-tls`（依赖系统 OpenSSL），换 rustls 要 `default-features = false, features = ["rustls", "json"]`——0.13 里叫 `rustls`，不是旧文档里的 `rustls-tls`。

📦 **实验**：[`labs/08_reqwest/`](../../labs/08_reqwest/)（包名 `l08_reqwest`）

---

## 13. `indexmap`：保持插入顺序的哈希表

**解决什么问题**：`std::HashMap` **不保证顺序**（迭代顺序随随机种子变）。要「按加入顺序输出」或「取第 n 个」，用 `IndexMap`（`Vec` + 哈希索引，查找仍是 O(1)）。

```rust
fn main() {
    use indexmap::{IndexMap, IndexSet};
    let mut m = IndexMap::new();
    m.insert("b", 2);
    m.insert("a", 1);
    m.insert("b", 20);                               // 覆盖旧值，保持原位置
    println!("{:?} {:?}", m.keys().collect::<Vec<_>>(), m.get_index(0));
    // 删除：shift_remove 保序（O(n)），swap_remove 是 O(1) 但打乱顺序
    println!("{:?} {:?}", m.shift_remove("b"), m.keys().collect::<Vec<_>>());
    let mut s = IndexSet::new();
    s.insert("z");
    s.insert("a");
    s.insert("z");                                   // 重复不插入
    println!("{:?}", s.iter().collect::<Vec<_>>());  // ["z", "a"]
}
```

**什么时候别用**：① 并不真的需要顺序——`HashMap` 更快更省内存；② 需要**按 key 排序**或范围查询——用 `BTreeMap`；③ 想用 `serde` 保留顺序要打开 `serde` feature；④ 循环里大量删除时注意 `shift_remove` 是 O(n)。

> ⚠️ 本节**不在 `labs/` 里演示**。

---

## 14. `once_cell` / `OnceLock`：全局只初始化一次

**解决什么问题**：`static` 只能放常量。要「第一次用时才构造、之后共享」的全局值（配置、编译好的正则、连接池），就需要懒初始化。**Rust 1.80 起 `std` 自带 `OnceLock`/`LazyLock`**，`once_cell` 主要留给旧工具链和它更丰富的 API。

```rust
fn main() {
    use once_cell::sync::Lazy;         // std 对应物：std::sync::LazyLock
    use std::collections::HashMap;
    use std::sync::{LazyLock, OnceLock};
    // Lazy/LazyLock：声明时绑定初始化逻辑，第一次解引用时执行
    static CONFIG: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| HashMap::from([("timeout", 30)]));
    static NAME: LazyLock<&'static str> = LazyLock::new(|| "std 1.80+ 自带");
    // OnceLock：初始化时机分散在多处时用它，get_or_init 幂等
    static PORT: OnceLock<u16> = OnceLock::new();
    println!("{:?} {} {}", CONFIG.get("timeout"), *NAME, PORT.get_or_init(|| 8080));
    println!("{}", PORT.get().unwrap());      // 第二次拿到同一个值
}
```

**什么时候别用**：① 能**显式传参**就别用全局——依赖注入比全局状态好测试；② 需要「可变全局」要 `Mutex`/`RwLock` 包起来，先想清楚为什么；③ 1.80+ 新项目直接用 `LazyLock`/`OnceLock`，不必引 `once_cell`；④ `lazy_static!` 老代码能跑，新代码别再用；⑤ `Lazy` 初始化里**别放可能 panic 的重活**。

> ⚠️ 本节**不在 `labs/` 里演示**。

---

## 15. `tempfile`：临时文件与目录

**解决什么问题**：`std::env::temp_dir()` 只给路径，**不清理、不防命名冲突、不管权限**。`tempfile` 在 drop 时自动删除，创建时就带随机名和「只有自己能访问」的权限。

```rust
fn main() -> std::io::Result<()> {
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};
    let mut f = NamedTempFile::new()?;          // 具名临时文件：能拿到 path
    f.write_all(b"hello")?;
    f.flush()?;
    println!("{}", std::fs::read_to_string(f.path())?);   // hello
    let dir = tempdir()?;                       // 临时目录：适合一次生成很多文件
    let inner = dir.path().join("a.txt");
    std::fs::write(&inner, "x")?;
    let existed = inner.exists();
    drop(dir);                                  // 目录（含内容）被删
    println!("{existed} {}", inner.exists());   // true false
    Ok(())
}
```

想要「退出后还留着」的临时文件，用 `persist()`/`keep()`。
**什么时候别用**：① 只想要系统临时目录的路径，`env::temp_dir()` 足够；② 测试里能用内存（`Vec<u8>`/`Cursor`）就别落盘；③ 生产里「跨进程、可恢复的临时文件」是正式存储，不是 `tempfile`；④ drop 之后还要用那个路径，必须先 `persist`。

> ⚠️ 本节**不在 `labs/` 里演示**。

---

## 16. Cargo 生态工具

| 工具 | 用途 | 命令 |
|---|---|---|
| `cargo add` | 加依赖并自动选版本、写入 `Cargo.toml` | `cargo add serde --features derive` |
| `cargo clippy` | 官方 lint：找出「能编译但不像人写的」代码 | `cargo clippy --all-targets -- -D warnings` |
| `cargo fmt` | 官方格式化（rustfmt），团队统一风格 | `cargo fmt --all` |
| `cargo doc` | 由文档注释生成本地 API 文档 | `cargo doc --open --no-deps` |
| `cargo audit` | 对照 RustSec 库检查依赖漏洞 | `cargo audit` |
| `cargo deny` | 检查许可证白名单、重复依赖、已知漏洞 | `cargo deny check licenses` |
| `cargo expand` | 展开宏和 derive，看 `#[derive(Serialize)]` 生成了什么 | `cargo expand --bin myapp` |
| `cargo watch` | 文件一变就重跑（开发循环） | `cargo watch -x test -x run` |
| `cargo tree` | 打印依赖树，查「这个 crate 是谁拉进来的」 | `cargo tree -i openssl` |
| `cargo update` | 在 `Cargo.toml` 允许范围内更新 `Cargo.lock` | `cargo update -p serde` |

> 💡 `cargo clippy -- -D warnings` 把 lint 升级成错误；`cargo audit`/`cargo deny` 应该进 CI，而不是出事后再跑。

---

## 17. 选型速查表

| 我要做 X | 用哪个 crate | 备注 |
|---|---|---|
| HTTP 客户端 | `reqwest`（异步）/ `ureq`（阻塞） | 别忘超时；TLS backend 要选 `rustls`/`native-tls` |
| Web 框架 | `axum`（tokio 系）/ `actix-web` | 见第 18 章 |
| 异步运行时 | `tokio` | 事实标准；库应尽量不硬绑 |
| JSON | `serde_json` | 动态结构配 `serde_json::Value` |
| 时间 | `time` 或 `chrono` | 二选一；时区库 `time-tz`/`chrono-tz` |
| CLI | `clap`（derive） | 补全用 `clap_complete`，交互提示用 `dialoguer` |
| 日志/诊断 | `tracing` + `tracing-subscriber` | 要兼容 `log` 再加 `tracing-log` |
| 错误 | 库 `thiserror` / 应用 `anyhow` | 公开 API 别泄漏 `anyhow::Error` |
| 并发 | `rayon`（数据并行）/ `crossbeam`（手写线程 + channel） | I/O 并发用 `tokio` |
| 随机数 | `rand` | 密码学用途考虑 `getrandom` |
| UUID | `uuid`（v4/v7） | 新项目优先 v7 |
| 正则 | `regex` | 要环视/反向引用 → `fancy-regex` |
| 数据库 | `sqlx`（异步、编译期校验）/ `diesel`（ORM） | 单文件 SQLite 也可 `rusqlite` |
| 序列化 | `serde` + 格式 crate（`serde_json`/`toml`/`bincode`） | 一个结构体，多种格式 |
| 机器学习 | `candle` / `ort`（ONNX Runtime） | 见第 21 章 |
| 临时文件 | `tempfile` | drop 即删 |
| 有序 map | `indexmap` / `BTreeMap` | 前者按插入序，后者按 key 排序 |

---

## 18. 本章练习

> 本章**没有自动评测**（`tools/chapters.json` 里第 17 章的 `crate` 是 `null`）。三题按你机器上的实测结果判定。

**17-1 配置合并小工具（`serde` + `clap`）**：读 JSON 配置文件 `{"name":"app","port":8080,"tags":["a"]}`，命令行参数**覆盖**文件里的值；CLI 支持 `--config <path>`、`--name <s>`、`--port <n>`（`Option<T>`，不传就保留文件值）；文件不存在或 JSON 非法时打印带上下文的错误（`anyhow`）并以非零码退出。
**验收标准**：① `--help` 列出全部三个参数且都有说明文字；② `--config cfg.json` 输出与文件一致；③ `--config cfg.json --port 9000` 只有 `port` 变 9000；④ `--config nope.json` 退出码非 0，stderr 里同时有**文件名**和**根因**；⑤ 缺 `port` 字段的文件能正常读取（`#[serde(default)]`）。

**17-2 日志过滤 CLI（`regex`）**：参数 `--pattern <regex>`、`--file <path>`、可选 `--limit <n>`；支持按 `$1`/`$2` 重组输出；**正则只编译一次**；非法正则打印 `regex` 的原始错误并非零退出。
**验收标准**：① 正则缺省或非法时退出码非 0，且信息指出正则错在哪；② 100 万行日志（`2026-10-12 LEVEL msg`）过滤 `ERROR` 在 **3 秒内**跑完（含读文件、输出到 `/dev/null`）；③ `--limit 10` 只输出前 10 行就退出，不读完整文件；④ `$1` 重组结果与预期一致；⑤ `cargo clippy` 无 warning（尤其 `Regex::new` 不在循环里）。

**17-3 并行化并测加速比（`rayon`）**：挑一个真实点的 CPU 密集任务（大量字符串哈希、像素变换等），先写串行版，再写 `.par_iter()` 版；用 `Instant` 分别计时（**先预热一次**，别把首次分配算进去），打印 `seq`、`par`、加速比并断言结果一致；加速比 < 1.5 说明粒度不够，把单次迭代加重再测。
**验收标准**：① 串行与并行结果一致（整数必须完全相等，浮点要说明为什么允许误差）；② 多核机器上加速比 **≥ 2.0**（记录 CPU 型号和核数）；③ 能解释「为什么不是 8 倍」——至少提到线程池启动、任务粒度、内存带宽中的两项；④ 附实测数字（`seq 2.50s / par 0.42s` 这种量级即可）；⑤ 并行闭包里没有共享可变状态。

---

## 19. 常见坑

1. **照旧教程抄版本**：`rand` 0.8→0.9 改了 `thread_rng`→`rng`、`gen_range`→`random_range`；`reqwest` 0.13 的 TLS feature 叫 `rustls` 不是 `rustls-tls`。**以 docs.rs 上你实际用的版本为准**。
2. **`Regex::new` 在循环里**：编译一次几毫秒，循环一万次就是分钟级；用 `OnceLock`/`LazyLock` 缓存。
3. **依赖 `HashMap` 的顺序**：日志、序列化输出要用 `BTreeMap`（按 key）或 `IndexMap`（按插入）；断言 `HashMap` 迭代顺序的测试必然随机挂。
4. **`reqwest` 忘设超时**：请求会无限挂起，`.timeout()` 是必选项而不是优化项。
5. **`tokio` feature 没开**：`#[tokio::main]` 要 `macros` + `rt-multi-thread`，`tokio::time` 要 `time`，`tokio::fs` 要 `fs`。
6. **在 async 里做阻塞**：`std::fs::read`、`std::thread::sleep`、长时间持锁的同步 `Mutex` 都会拖垮执行器。
7. **`tracing-subscriber` 忘开 `env-filter`**：`EnvFilter` 找不到，或 `RUST_LOG` 不起作用。
8. **错误里丢掉根因**：`map_err(|_| "出错了")` 会抹掉 `source`，`err.chain()` 立刻断掉。
9. **`Box<dyn Error>` 跨线程**：需要 `Box<dyn Error + Send + Sync>`，否则 `tokio::spawn` 里编译不过。
10. **默认 feature 拖家带口**：`default-features = false` 常能砍掉一半依赖，但有些「核心功能」藏在默认 feature 里，砍之前看文档。

---

## 20. 自测清单

- [ ] 我能说出「挑 crate」要看的至少四件事（下载量、更新时间、unsafe、文档/许可证/feature）
- [ ] 我能用 `serde` 的 `default`/`rename_all` 写「字段可缺省」的配置结构体，并知道 `Value` 何时更合适
- [ ] 我知道 `Regex` 要缓存、`regex` 不支持环视与反向引用、以及它为什么没有回溯
- [ ] 我能说清 `Instant`/`SystemTime`/`time::OffsetDateTime` 的分工，知道 `OffsetDateTime` 没有 `parse`
- [ ] 我能用 `clap` derive 写带子命令和 `default_value_t` 的 CLI，并解释参数里 `Option<T>` 的含义
- [ ] 我能说出「库用 `thiserror`、应用用 `anyhow`」的理由，会用 `context`/`bail`/`err.chain()`
- [ ] 我知道 `rayon` 什么时候反而更慢，以及浮点并行求和为什么可能和串行结果不同
- [ ] 我分得清 `chunk_by`（相邻分组）和「全局分组」，也知道 `IndexMap`/`BTreeMap`/`HashMap` 各自的适用场景
- [ ] 我知道 Rust 1.80+ 用 `LazyLock`/`OnceLock` 替代 `once_cell` 的多数场景
- [ ] 我能说全 `reqwest` 的错误处理链：`send` → `error_for_status` → `json`/`text`，并且设了超时
- [ ] 我能列出至少 6 个 Cargo 生态工具并说出用途
- [ ] 我完成了 3 道练习，并能拿出实测数据（`--help` 输出、过滤耗时、加速比）
