# 07_tracing · 结构化日志（tracing + tracing-subscriber）

> 目录名带序号只为排序。**cargo 不允许包名以数字开头**，所以 `labs/` 里所有成员的包名
> 统一是 `lXX_` 前缀：目录 `07_tracing`，包名 `l07_tracing`。

| 项 | 值 |
|---|---|
| 包名 | `l07_tracing` |
| 覆盖的 crate | `tracing` 0.1.x、`tracing-subscriber` 0.3.x（features: `env-filter`） |
| 运行 | `cargo run -p l07_tracing`；`RUST_LOG=debug cargo run -p l07_tracing` |
| 需要网络 | 只有第一次拉依赖需要 |

## 它解决什么问题

`println!` 的三宗罪：没有级别、没有上下文、没法过滤。
一次请求里打了 40 行日志，出了事却分不清哪几行属于同一个用户。

`tracing` 的模型是 **event（事件）+ span（时间段/上下文）**：

- **event** 就是一条日志，带级别和任意结构化字段（`user_id = 7`），
  交给下游可以变成 JSON 给 ELK/Loki，而不是只能当字符串 grep；
- **span** 表示「这段时间内发生的事都属于某个上下文」，可以嵌套，
  于是「哪几行属于同一次请求」这个问题由框架回答，不用手写拼接前缀；
- **宏只在事件真的会被记录时才做格式化**，被过滤掉时几乎零成本，
  所以库代码可以放心地撒 `debug!`，让使用者在部署时决定要不要看。

`tracing-subscriber` 负责「谁来记录、记到哪里、什么级别」：本实验用 `fmt()` 输出到终端，
`EnvFilter` 从 `RUST_LOG` 读过滤器。

## 程序演示了什么

1. **初始化**：`EnvFilter::try_from_default_env()` 读 `RUST_LOG`，读不到退回 `"info"`，
   再 `tracing_subscriber::fmt().with_env_filter(filter).init()`。
2. **级别与结构化字段**：`info!` / `warn!` / `error!` / `debug!` / `trace!`，
   以及 `info!(user_id = 7, "登录")`、`warn!(remaining = 3, limit = 5, "...")`。
3. **`#[instrument]`**：给函数自动建 span 并记录参数，`login()` 调用 `check_password()`
   形成嵌套 span；`skip(password)` 演示怎么避免把敏感字段写进日志；
   另外用 `info_span!` + `span.enter()` 手动开一个「批量导入」的 span。
4. **过滤级别**：`debug!` 默认不打印不是宏的问题，而是 filter 的问题 ——
   程序里用 `tracing::enabled!(Level::DEBUG)` 打印「当前是否启用」，
   再用 `with_default` 临时换一个允许 debug 的 subscriber，让同一行 `debug!` 真的出现。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
cargo run -p l07_tracing                 # 默认 info：debug! 看不到
RUST_LOG=debug cargo run -p l07_tracing  # 全部级别
RUST_LOG=warn cargo run -p l07_tracing   # 只看 warn 以上
RUST_LOG=warn,l07_tracing=debug cargo run -p l07_tracing  # 全局 warn + 本程序 debug
```

## 预期输出（节选）

`cargo run -p l07_tracing`（默认 info）：

```text
（提示：RUST_LOG = None）
（提示：debug! 当前是否启用 = false）

── ② 五个级别 + 结构化字段 ──
2026-10-12T07:32:35.800752Z  INFO 程序启动
2026-10-12T07:32:35.800795Z  INFO 登录 user_id=7 user="ada"
2026-10-12T07:32:35.800842Z  WARN API 配额快用完了 remaining=3 limit=5
2026-10-12T07:32:35.800863Z ERROR 数据库连接失败 code=500 retryable=false
  ↑ 上面每行日志都带级别、时间、字段；debug!/trace! 默认不出现

── ③ #[instrument]：函数即 span ──
2026-10-12T07:32:35.800967Z  INFO login{user="ada"}: 开始校验凭证
2026-10-12T07:32:35.806339Z  INFO login{user="ada"}: 登录成功 roles=["user", "reader"]
2026-10-12T07:32:35.806405Z  INFO login() 返回 ok=true
2026-10-12T07:32:35.811655Z  WARN login{user="bob"}: 登录失败，请检查用户名或密码
2026-10-12T07:32:35.811766Z  INFO batch_import{rows=3 source="csv"}: 导入一行 row=1
2026-10-12T07:32:35.811848Z  INFO span 已退出，之后的事件不再带 batch_import 字段

── ④ debug! 默认不打印，是因为过滤器而不是宏 ──
2026-10-12T07:32:35.812041Z DEBUG （临时 subscriber）这句 debug! 打印出来了
```

`login{user="ada"}:` 就是 `#[instrument]` 建出来的 span（参数自动变成 span 字段），
`span 已退出` 那行不再带 `batch_import` 字段。

`RUST_LOG=debug cargo run -p l07_tracing` 的差别：

```text
（提示：RUST_LOG = Some("debug")）
（提示：debug! 当前是否启用 = true）

2026-10-12T07:3x:xx.xxxxxxZ DEBUG detail="只有 RUST_LOG=debug 时才看得到这条"
2026-10-12T07:3x:xx.xxxxxxZ DEBUG login{user="ada"}:check_password{user="ada"}: 进入密码校验（默认级别下你看不到我） pwd_len=6
```

注意 `login{...}:check_password{...}:` 的双层前缀 —— 那就是**嵌套 span**，
事件自动带上了两层的字段。`password` 因为 `skip(password)` 没有出现在日志里，
只留了 `pwd_len`。

## 什么时候别用它

- **单文件脚本 / 一次性调试**：`println!` 或 `dbg!` 更直接，引入 subscriber 是负担。
- **只在库代码里 `init()`**：库自己初始化 subscriber 会抢走 App 的选择权（而且 `init()`
  重复调用会 panic）。库只发事件，初始化留给 `main`。
- **把日志当日志文件系统用**：需要检索就上 JSON 输出 + 集中式收集，别自己 rotate 文件
  （那属于 `tracing-appender`，而且生产一般是容器 stdout 交给平台收）。
- **要审计/指标**：日志不是审计流水，也不适合当 metrics；这些场景用专门的系统。
- **过度 instrument**：给每个小函数都加 `#[instrument]` 会让 span 树巨大、
  输出噪音变多；只标注「一次业务操作」这一层真正有价值的边界。

## 常见坑

- **没有 `env-filter` feature**：`tracing_subscriber::EnvFilter` 根本不存在，
  报错是「unresolved import」。要在 `Cargo.toml` 里写
  `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`。
- `RUST_LOG` 没设置时 `try_from_default_env()` 返回 `Err`，用 `unwrap_or_else` 兜底；
  直接 `unwrap()` 会在没设环境变量时 panic。
- `.init()` 只能调一次；测试里多次初始化要么用 `try_init()` 忽略错误，
  要么用 `tracing::subscriber::with_default` 限定作用域（本实验就是这么演示的）。
- 忘记 `#[instrument]` 里 `skip(...)` 敏感参数，密码/token 就进了日志。
- 字段语法：`info!(x)` 记录的是**变量名 x**，`info!(pid = id)` 才是自定义字段名，
  `info!(?obj)` 用 `Debug` 输出，`info!(%obj)` 用 `Display` 输出。
