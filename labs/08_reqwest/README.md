# 08_reqwest · 异步 HTTP 客户端（reqwest + tokio）

> 目录名带序号只为排序。**cargo 不允许包名以数字开头**，所以 `labs/` 里所有成员的包名
> 统一是 `lXX_` 前缀：目录 `08_reqwest`，包名 `l08_reqwest`。

| 项 | 值 |
|---|---|
| 包名 | `l08_reqwest` |
| 覆盖的 crate | `tokio` 1.x（`macros`, `rt-multi-thread`）、`reqwest` 0.13.x（`json`）、`serde` 1.x（`derive`）、`serde_json` 1.x |
| 运行 | `cargo run -p l08_reqwest`（拉依赖要联网，**运行时也要联网**） |

## 它解决什么问题

调别人的 HTTP API：发请求、解析 JSON、控制超时、把「超时 / 连不上 / 404」分清楚。
`reqwest` 是异步客户端（默认带 TLS），配 `tokio` 就能在一个线程里并发成百上千个请求；
`serde` 负责把 JSON 变成带类型的 struct。

## 程序演示了什么

1. **GET + 超时**：`#[tokio::main]` 里建一个复用连接池的 `Client`，
   `Client::builder().timeout(Duration::from_secs(10))` 统一设超时，GET `api.github.com`
   并打印状态码与响应文本片段。
2. **反序列化**：`#[derive(Deserialize)] struct Repo` 只声明关心的字段
   （`full_name` / `stargazers_count` / `language: Option<String>` …），
   先 `.error_for_status()` 再 `.json::<Repo>()`。
3. **POST JSON**：`client.post(url).json(&payload)` 自动序列化并设置
   `Content-Type: application/json`，再解析回显。
4. **错误处理**（重点，全程没有 `unwrap()`）：
   - **超时**：对保留网段 `192.0.2.1` 只给 500ms 预算 —— 保证最多等半秒就返回；
   - **连接失败**：`http://127.0.0.1:9/`（几乎肯定无人监听，连接被拒）；
   - **非 2xx**：请求一个不存在的 GitHub 仓库，先手动 `status().is_success()`
     （还能读错误响应体），再用 `.error_for_status()` 把 4xx/5xx 统一变成 `Err`；
   - 一个 `explain()` 帮助函数把 `reqwest::Error` 按 `is_timeout()` / `is_connect()` /
     `is_status()` / `is_decode()` 分类，并打印完整错误链（`source()` 逐层）。

**失败分支不会让程序崩**：每个请求的错误都只打印，`main` 继续往下走并正常结束。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
cargo run -p l08_reqwest
```

网络被限制时：GET/POST 会打印错误信息（且连接失败分支照样演示成功），
程序仍然以退出码 0 结束。纯离线环境下至少可以用 `cargo build -p l08_reqwest` 验证能编译。

## 预期输出（节选，star 数是实时值）

```text
== 实验 08 · reqwest 异步 HTTP ==

(1) Client 建好了：全局超时 10s，带 User-Agent

── ① GET（看状态码与原始文本）──
  GET https://api.github.com/repos/rust-lang/rust
  HTTP 200 OK（is_success = true）
  前 200 字符: {"id":724712,"node_id":"MDEwOlJlcG9zaXRvcnk3MjQ3MTI=","name":"rust","full_name":"rust-lang/rust","private":false,"owner":{"login":"rust-lang", …

── ② GET + 反序列化到自定义 struct ──
  仓库全名 : rust-lang/rust
  star 数  : 119724
  主语言   : Rust
  简介     : Empowering everyone to build reliable and efficient software.
  归档     : false
  主页     : https://github.com/rust-lang/rust
  （struct 里没写的字段自动忽略，网络流量照样收，只是不解析）

── ③ POST JSON（.json(&payload)）──
  POST https://httpbin.org/post
  发送: {"project":"rs-study labs","stars":108000,"comment":"hello from reqwest"}
  {
    "comment": "hello from reqwest",
    "project": "rs-study labs",
    "stars": 108000
  }

── ④ 错误处理：超时 / 连接失败 / 非 2xx ──
  (a) 超时: GET http://192.0.2.1:81/ ，超时设 500ms
      超时 (timeout) => error sending request for url (http://192.0.2.1:81/)  <-  operation timed out
  (b) 连接失败: GET http://127.0.0.1:9/
      连接失败 (connect：DNS/TCP/TLS) => error sending request for url (http://127.0.0.1:9/)  <-  client error (Connect)  <-  tcp connect error  <-  Connection refused (os error 111)
  (c) HTTP 非 2xx: GET https://api.github.com/repos/rust-lang/definitely-not-a-repo-xyz
      收到响应 HTTP 404 Not Found（is_success = false）
      手动读错误体: {"message":"Not Found",...
      error_for_status() 之后: HTTP 状态码非 2xx (status) => HTTP status client error (404 Not Found) for url (...)
```

> `(a)` 在有的网络里会以「连接失败」而不是「超时」返回（取决于路由是否直接报不可达）——
> 这正是要**按 `is_*()` 分类**而不是假设固定错误的原因，程序对两种结果都处理了。

## 什么时候别用它

- **阻塞式小程序**：只想同步发一个请求，用 `ureq` 更省事，不必拖进整个 tokio。
- **需要 HTTP/2 服务端、路由、中间件**：那是 `axum`/`hyper` 的活；`reqwest` 只是客户端。
- **超大规模爬取**：`reqwest` 不替你限速、重试、去重，这些要自己写或换专门的抓取框架。
- **WebAssembly**：默认 TLS 后端在 wasm 上用不了，需要专门的配置。
- **把 `Client::new()` 写在循环里**：会丢掉连接池复用，每个请求都重新握手（也更容易打满 fd）。

## 常见坑

- **不设超时**：`reqwest` 默认**没有总超时**，对端半死不活时请求会挂着。
  应该 `Client::builder().timeout(...)`（这是整次请求的总预算）或加 `.timeout()`。
- **默认 TLS 依赖系统 OpenSSL**：某些环境（比如精简镜像）缺 OpenSSL 开发库会编译失败，
  可改用 `features = ["rustls-tls"]` + `default-features = false`。
- **`reqwest::Error` 想早退**：直接 `?` 会把「404」也当成 Err；要读错误响应体就自己看
  `resp.status()`，只想统一处理就用 `.error_for_status()`（它在请求失败时不会覆盖原错误）。
- **反序列化字段对不上**：类型不匹配会得到 `is_decode()` 的 Err；
  字段名有 `snake_case`/`camelCase` 差异时用 `#[serde(rename_all = "camelCase")]` 或 `#[serde(rename = "...")]`。
- **忘记带 User-Agent**：GitHub API 等站点会直接 403。
- **在异步上下文里跑阻塞代码**（`std::thread::sleep`、同步文件 IO）：会卡住整个 runtime 线程，
  用 `tokio::time::sleep` / `tokio::fs`，CPU 密集任务用 `spawn_blocking`。
- **忘了 tokio 的 `rt-multi-thread` + `macros` feature**，`#[tokio::main]` 或 `main` 的运行时不存在。
