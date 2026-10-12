# axum-todo —— 第 18 章配套 demo

一个**内存版 todo REST API**，用 `axum 0.8` + `tokio` 写成，单文件 `src/main.rs`（约 550 行，含大量注释）。
它对应教程 [`docs/04-frameworks/ch18-web-axum.md`](../../docs/04-frameworks/ch18-web-axum.md)。

**它演示了 8 件事**：路由表、`State` 共享状态、`Path`/`Json` 提取器与顺序规则、
`serde` 的 `camelCase` 契约、自定义 `AppError` + `IntoResponse` 的统一错误体、
`Json` 的 400/415/422 三种默认码怎么归一、`201`/`204` 这些非 200 成功码、
以及 `#[tokio::main]` + `axum::serve` 的启动方式。

---

## 怎么跑

> 需要网络：第一次会下载 axum / tokio / serde 等约 50 个 crate。

```bash
cd /home/yxb/code/rs-study/projects/axum-todo

# ⚠️ 本仓库的会话环境里 ~/.cargo 只读，必须把 CARGO_HOME 指到可写目录
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home

cargo run
```

正常的话你会看到：

```text
axum-todo 已启动
  监听地址 : http://127.0.0.1:3000（也就是 http://localhost:3000）
  健康检查 : http://localhost:3000/health

试试这些命令：
  curl -s http://localhost:3000/health
  curl -s http://localhost:3000/todos
  curl -s -X POST http://localhost:3000/todos -H 'content-type: application/json' -d '{"title":"学 Rust"}'
  ...
按 Ctrl-C 退出。
```

`http://localhost:3000` 在多数终端里可以直接点击打开（浏览器会看到 404，因为我们没做首页，
只有 `/health` 和 `/todos` 两个路径——这很正常）。

### 依赖版本（实际解析结果）

| crate | 版本 | 为什么需要 |
|---|---|---|
| `axum` | 0.8.9 | 路由 + 提取器 + 响应 |
| `tokio` | 1.53.2（`macros`, `rt-multi-thread`） | `#[tokio::main]`、异步 `TcpListener`、`RwLock` |
| `serde` | 1.0.229（`derive`） | `Serialize`/`Deserialize` 派生 |
| `serde_json` | 1.0.154 | axum 的 `Json` 提取器/响应内部用它 |
| `tower`（dev） | 0.5.3 | 只给 `tests/api.rs` 用：`ServiceExt::oneshot` 不起端口直接打 Router |

> 若直连 crates.io 很慢，`projects/axum-todo/.cargo/config.toml` 已把源换成 rsproxy 镜像。
> 你的网络能直连 crates.io 时，删掉这个文件即可（它不影响 `Cargo.lock` 的版本号）。

---

## 接口一览

| 方法 | 路径 | 请求体 | 成功码 | 失败码 |
|---|---|---|---|---|
| GET | `/health` | — | 200 `ok` | — |
| GET | `/todos` | — | 200 `[Todo]` | — |
| POST | `/todos` | `{"title":"..."}` | 201 `Todo` | 400 |
| GET | `/todos/{id}` | — | 200 `Todo` | 400 路径非数字 / 404 |
| PATCH | `/todos/{id}` | `{"title"?:..,"done"?:..}` | 200 `Todo` | 400 / 404 |
| DELETE | `/todos/{id}` | — | 204 空 body | 400 / 404 |

`Todo` 的 JSON 形状（字段名是 **camelCase**，由 `#[serde(rename_all = "camelCase")]` 决定）：

```json
{"id":1,"title":"学 Rust","done":false,"createdAt":1}
```

所有错误都是同一种形状：

```json
{"error":"not_found","message":"todo 999 不存在"}
```

---

## 预期看到什么（真机实测输出）

下面这段是**在本机实际跑出来的**，不是手写的。建议你照着敲一遍，逐条对上。

```console
$ curl -s localhost:3000/health
ok                                       # http 200

$ curl -s localhost:3000/todos
[]                                       # http 200（启动时是空数组）

$ curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"学 Rust"}'
{"id":1,"title":"学 Rust","done":false,"createdAt":1}   # http 201

$ curl -s localhost:3000/todos
[{"id":1,"title":"学 Rust","done":false,"createdAt":1}] # http 200

$ curl -s localhost:3000/todos/1
{"id":1,"title":"学 Rust","done":false,"createdAt":1}   # http 200

$ curl -s -X PATCH localhost:3000/todos/1 -H 'content-type: application/json' -d '{"done":true}'
{"id":1,"title":"学 Rust","done":true,"createdAt":1}    # http 200，done 变 true

$ curl -s -o /dev/null -w '%{http_code}\n' -X DELETE localhost:3000/todos/1
204                                      # 删掉了

$ curl -s localhost:3000/todos
[]                                       # 又空了
```

错误路径（重点看**状态码**和**统一 JSON 体**）：

```console
$ curl -s localhost:3000/todos/999
{"error":"not_found","message":"todo 999 不存在"}        # http 404

$ curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d 'not json'
{"error":"bad_request","message":"请求体不是合法的 JSON"} # http 400

$ curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d '{}'
{"error":"bad_request","message":"title 不能为空"}        # http 400

$ curl -s localhost:3000/todos/abc
{"error":"bad_request","message":"路径参数不合法: Cannot parse `abc` to a `u64`"}  # http 400

$ curl -s -X POST localhost:3000/todos -d '{"title":"x"}'
{"error":"bad_request","message":"缺少 content-type: application/json"}            # http 400
```

> **注意 id 从 1 开始**，不是 0。所以上面示例里如果写 `/todos/0` 会得到 404——
> 这不是 bug，是本 demo 的 id 生成规则（`next_id` 初值 1）。教程示例里出现过 `/todos/0`，
> 那是笔误，以本 README 为准。

---

## 3 道改造题（做完再去看教程的「本章练习」）

每题都给了**验收标准**，能自己判断过没过。改完记得 `cargo build` 无警告、`cargo clippy` 干净。

### 18-A　给 `GET /todos` 加过滤：`?done=true|false`

**要做什么**：`GET /todos?done=true` 只返回已完成的，`?done=false` 只返回未完成的，
不带查询参数时返回全部。`?done=maybe` 这种非法值要返回 400 + 统一错误体。

**要用的东西**：`axum::extract::Query` + `#[derive(Deserialize)]` 的查询参数结构体。
提示：`done: Option<bool>` 直接用 serde 解析 `true`/`false` 字符串；非法值会走
`QueryRejection`——参照 `src/main.rs` 里 `JsonBody` 的写法包一个 `ApiQuery<T>`，
把拒绝类型也映射成 `AppError::BadRequest`。

**验收标准**

```bash
curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"a"}'
curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"b"}'
curl -s -X PATCH localhost:3000/todos/1 -H 'content-type: application/json' -d '{"done":true}'

curl -s 'localhost:3000/todos?done=true'    # 只有 id=1，长度 1
curl -s 'localhost:3000/todos?done=false'   # 只有 id=2，长度 1
curl -s 'localhost:3000/todos'              # 两条都在
curl -s -o /dev/null -w '%{http_code}\n' 'localhost:3000/todos?done=maybe'   # 期望 400
```

### 18-B　持久化到 JSON 文件（重启不丢数据）

**要做什么**：把 `Vec<Todo>` 存到 `todos.json`；进程启动时读回来（文件不存在就当空列表），
每次增删改后写回去。要处理「文件内容是坏 JSON」的情况（不能 panic）。

**要用的东西**：`std::fs::read_to_string` / `std::fs::write`、`serde_json::from_str`/
`serde_json::to_string_pretty`。可选进阶：用 `AppState` 里额外存一个文件路径，
让测试可以用临时文件。

**验收标准**

1. 连续 `POST` 两条 → `cat todos.json` 能看到两条的 JSON 数组；`createdAt`/`done` 都在。
2. `Ctrl-C` 杀掉进程再 `cargo run` → `GET /todos` 仍返回那两条，且 `createdAt` 不变。
3. 把 `todos.json` 内容改成 `not json` 再启动 → **服务仍能起来**（当作空列表或打印警告），
   `GET /health` 返回 200。
4. `DELETE` 一条后 `todos.json` 里也少一条（重启验证最简单）。
5. 写文件失败时（比如目录只读）不能让整个进程崩掉——要么返回 500，要么记日志继续跑。

> 提示：写入用「先写临时文件再 `rename`」可以避免写一半崩溃导致文件损坏；
> 另外注意别把 `next_id` 忘了恢复（否则重启后新 id 会撞车）。

### 18-B+　（可选）把 demo 拆成 lib + bin，好写集成测试

**背景**：现在 `src/main.rs` 是一个 binary crate，`tests/` 里的集成测试**没法**用
`axum_todo::app(...)` —— binary crate 的内部对集成测试不可见（这是 Rust 的硬规则，
不是 axum 的问题）。

**要做什么**：把 `src/main.rs` 拆成：

```
src/lib.rs    ← 所有 pub 的类型和 app()/handler，加 pub fn AppState::new()
src/main.rs   ← 只留 #[tokio::main] fn main()，use axum_todo::...
```

`Cargo.toml` 不用改（cargo 会自动把 `src/lib.rs` 认成库、`src/main.rs` 认成同名 binary）。

**验收标准**：仓库里已经放了一个 `tests/api.rs` 作为模板（它现在是自包含的最小 Router，
只演示 `oneshot` 怎么用）。拆分后把它改成真的打 `axum_todo::app(...)`，并断言：

1. `POST /todos` 返回 201，body 里 `title` 正确、`createdAt == 1`；
2. `GET /todos/999` 返回 404，且 body 是统一错误体（`error == "not_found"`）；
3. `POST /todos` 发 `not json` 返回 400（**不是** axum 默认的 422）。

`cargo test` 全绿即通过。`tower` 已经在 `[dev-dependencies]` 里（`oneshot` 靠它）。

### 18-C　加请求日志中间件（method / path / status / 耗时）

**要做什么**：每个请求打一行日志，形如：

```text
INFO GET /todos -> 200 in 1.2ms
INFO POST /todos -> 400 in 0.4ms
```

**要用的东西**：`tower::ServiceBuilder` + `tower_http::trace::TraceLayer`
（需要在 `Cargo.toml` 加 `tower-http = { version = "0.6", features = ["trace"] }`
和 `tracing-subscriber = "0.3"`，并在 `main` 开头初始化 subscriber）。
自定义格式用 `TraceLayer::new_for_http().on_response(...)`。

**验收标准**

1. 每发一个 curl，终端就多一行日志，且 `status` 与实际返回的 HTTP 码一致
   （404 的那条要打 404，不是 200）。
2. 日志里能看到耗时字段。
3. 加中间件后原有 16 条 curl 行为**完全不变**（`.layer()` 只包一层，不该改动业务）。
4. `cargo clippy` 仍无警告。

> 想更简单的话，也可以手写一个 `axum::middleware::from_fn` 函数来打日志，
> 然后再对比一下 `TraceLayer` 少写了多少代码。

---

## 常见问题

| 现象 | 原因 | 怎么办 |
|---|---|---|
| 启动 panic：`Path segments must not start with ':'` | 用了 axum 0.7 的 `:id` 写法 | 改成 `{id}`（0.8 会主动检查并报错，算好事） |
| `Address already in use` | 3000 端口被占（上次的进程没退） | `pkill -f axum-todo`，或改 `main` 里的端口 |
| `failed to get ... as a dependency` / 下载超时 | 网络问题 | 已配 .cargo/config.toml 镜像；仍失败就重试 `cargo fetch` |
| 编译报 `Handler<_, _> is not satisfied` | 提取器顺序错了（`Json` 没放最后） | 给 handler 加 `#[axum::debug_handler]` 看人话提示（需 axum 的 `macros` feature） |
| `use of unresolved module or unlinked crate tower` | 测试里用了 `tower` 但没声明 | 已加进 `[dev-dependencies]`；确认版本是 0.5（和 axum 0.8 配套） |
| `error: no default toolchain` | 没走指定工具链 | 按上面 `export PATH=...` 再跑 |

---

## 文件清单

```
projects/axum-todo/
├── Cargo.toml          # 4 个运行时依赖 + 1 个 dev 依赖（tower，仅测试用）+ 空的 [workspace]
├── Cargo.lock          # 已锁定版本（axum 0.8.9 / tokio 1.53.2 / serde 1.0.229）
├── .cargo/config.toml  # 可选的 crates.io 镜像配置，直连没问题时可删
├── README.md           # 本文件：怎么跑、预期输出、改造题与验收标准
├── src/main.rs         # 全部实现，约 550 行，含大量解释性注释
└── tests/api.rs        # 集成测试模板：演示 tower::ServiceExt::oneshot 的用法
```

---

## 和其它章节的关系

- 第 12 章（并发）解释了 `Arc`；第 13 章（async）解释了为什么 `main` 需要 `#[tokio::main]`。
- 第 17 章（常用第三方库）里的 `serde` 是这里 JSON 的基础。
- 第 20 章的 `sqlx-notes` 会把这里的「内存 `Vec`」换成 SQLite——路由和 handler 的骨架几乎不用动，
  这就是「把存储细节关在 `AppState` 里」的好处。
