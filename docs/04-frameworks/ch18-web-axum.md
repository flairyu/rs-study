# 第 18 章 · Web：axum

> **一句话定位**：axum 不是「Rust 版 Django」，而是**建在 `tokio` / `hyper` / `tower` 之上的一层
> 「路由 + 提取器」**；学会提取器（Extractor）体系，就学会了 axum 的九成。
> **预计时间**：150 分钟（读 70 + 跑/改 demo 80）
> **前置**：第 12 章（并发：`Arc`）、第 13 章（async/await）、第 17 章（常用第三方库：`serde`）
> **配套 demo**：[`projects/axum-todo`](../../projects/axum-todo/README.md)（需要网络拉依赖，独立 crate，不在任何 workspace 里）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一句「把 JSON 请求体变成结构体」，抽象层次差别很大：
```python
# Flask：路由 + 解析 + 响应全包，但校验发生在运行时
@app.post("/todos")
def create_todo():
    data = request.get_json()      # key 写错照样跑，直到线上才炸
    return jsonify({"id": 1, "title": data["title"]}), 201
```

```rust
// axum：handler 的参数类型 = 校验规则 + 错误类型；返回值类型 = 响应构造
async fn create_todo(
    State(state): State<SharedState>,          // 从应用状态里取
    JsonBody(payload): JsonBody<CreateTodo>,   // 从 body 里取（必须放最后）
) -> Result<(StatusCode, JsonBody<Todo>), AppError> { ... }
```

| 维度 | Flask / FastAPI | Crow / Drogon（C++） | axum |
|---|---|---|---|
| 定位 | 全家桶（模板、ORM、依赖注入） | 轻量路由 + 手写 `load<>()` 解析 | **只有路由 + 提取器**；日志/压缩/CORS 靠 tower 中间件 |
| 校验时机 / 失败码 | 运行时，自己写 / 422 | 运行时，自己写 | **编译期**（不实现 `Deserialize` 就不编译）；码是 400/415/422 三选一，见 §6.1 |
| 异步 / 错误 / 中间件 | WSGI 或 ASGI；抛异常 + `errorhandler` | 线程池或协程；返回 `response` | `tokio` 的 `Future`（运行时由你选）；实现 `IntoResponse`；中间件是 `tower::Layer` |

### 0.1 axum 的定位：它是「积木」不是「房子」

**HTTP 协议本身**由 `hyper` 实现（0.8 用 hyper 1.x），**异步调度**由 `tokio` 提供
（可换别的运行时），**中间件（日志、超时、限流、CORS）**来自 `tower` 的
`Layer`/`Service`——它们**不是 axum 写的**，任何用 tower 的框架都能复用。
**axum 自己只做两件事**：把请求**路由**到 handler；把 handler 的**参数和返回值**与
HTTP 请求/响应互相**转换**（即提取器 + `IntoResponse`）。所以「axum 文档里怎么找不到
日志/静态文件/WebSocket 鉴权」——那分别是 `tracing`、`tower-http` 和其他 crate 的事。
**这不是缺功能，而是不重复造轮子。**

> ⚠️ **坑**：别在 axum 里找 `app.use(...)` 式的中间件列表，对应物是
> `Router::layer()` / `Router::route_layer()`，而 layer 来自 tower。

---

## 1. 先回到第 13 章：为什么需要异步运行时

axum 的 handler 是 `async fn`。第 13 章讲过：**调用 `async fn` 不执行函数体，只返回一台
惰性状态机，必须有人 poll 才会前进**；std 只给了 `Future`/`Waker`，没给执行器。

```toml
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
#                                     ^^^^^^   ^^^^^^^^^^^^^^^
#                            #[tokio::main]    多线程运行时
```

**为什么不自己写执行器？** 第 13 章那个玩具执行器只能跑 `YieldNow`；真实服务需要
epoll/kqueue/IOCP 事件循环、定时器、任务窃取调度和正确的 `Send` 边界——都在 `tokio` 里。
于是 handler 里可以直接 `tokio::spawn`、`tokio::time::sleep`，共享同一运行时。

---

## 2. `Router` 与 handler

handler 就是「**参数都实现提取器 trait，返回值实现 `IntoResponse`**」的普通 `async fn`，
没有宏、没有基类——`route` 只是把「路径 + 方法」映射到这个函数上：

```rust
pub fn app(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", get(get_todo).patch(update_todo).delete(delete_todo))
        .with_state(state)     // 必须最后：Router<SharedState> -> Router<()>
}
```

三个要点：**① 同一路径多方法用 method-router 链式注册**（`get(..).post(..)`），一眼看出这个
路径支持什么；**② `.with_state()` 必须在最后**，它把状态「焊死」进路由、把
`Router<SharedState>` 变成 `axum::serve` 要的 `Router<()>`；**③ `{id}` 是 0.8 写法**（0.7 是
`:id`，见 §常见坑 1）。把 `app()` 拆成独立函数是刻意的：集成测试能复用同一个 `Router`（§8）。

---

## 3. 提取器（Extractor）体系 —— 本章最重要的一节

提取器就是「能从请求里取出东西」的类型，由两个 trait 定义：

| trait | 能力 | 典型类型 | 能放哪 |
|---|---|---|---|
| `FromRequestParts` | 只看请求**元数据**（方法、头、URI、路径参数） | `State`、`Path`、`Query`、`HeaderMap` | 参数表**任意位置** |
| `FromRequest` | 能**消费 body**（body 是流，只能读一次） | `Json`、`String`、`Bytes`、`Form` | **只能是最后一个** |

### 3.1 顺序规则

handler 的参数表被编译器当成一个**元组**处理，axum 生成的元组实现规则是：

> 除最后一个之外，其余参数类型必须实现 `FromRequestParts`；
> 只有最后一个参数可以是 `FromRequest`（消费 body 的那个）。

```rust
// ✅ 对：State 和 Path 都是 FromRequestParts，谁先谁后都行；Json 在最后
async fn ok(State(s): State<SharedState>, Path(id): Path<u64>, Json(b): Json<Body>) { }
async fn ok2(Path(id): Path<u64>, State(s): State<SharedState>, Json(b): Json<Body>) { }

// ❌ 错：Json 后面还有参数
async fn bad(Json(b): Json<Body>, State(s): State<SharedState>) { }
```

**为什么 body 只能读一次？** 请求体是**流**（hyper 的 `Body`），背后可能是一个还没读完的
TCP 连接，读过的字节不会重放。让「消费者」唯一且必须在最后，编译器就能保证没人能在
body 被读走之后还去读它——这是第 4 章「move 语义」在网络层的直接体现。

### 3.2 报错看不懂？加 `#[axum::debug_handler]`

上面 `bad` 的报错**完全没提顺序**（本机 axum 0.8.9 实测）：
`` error[E0277]: the trait bound `fn(Json<Body>, State<Arc<u32>>) -> ...: Handler<_, _>` is not satisfied
= note: Consider using `#[axum::debug_handler]` to improve the error message ``。
照着提示加上 `#[axum::debug_handler]`（需要
`axum = { version = "0.8", features = ["macros"] }`），同一处错误立刻变成人话：
`` error: `Json<_>` consumes the request body and thus must be the last argument to the handler function ``。

> ⚠️ **坑**：`debug_handler` 在 0.8 里**不是默认 feature**。看到那条 note 却用不了宏，
> 就是因为没开 `macros`。

---

## 4. `State`：把共享状态注入 handler

`State<T>` 从 router 里取出应用状态，要求 `T: Clone`——所以惯例是**状态本身是便宜的
`Arc`，`Clone` 只加一次引用计数**：

```rust
pub struct AppState { todos: Vec<Todo>, next_id: u64, clock: u64 }
pub type SharedState = Arc<RwLock<AppState>>;      // 注意是 tokio 的 RwLock
async fn list_todos(State(state): State<SharedState>) -> JsonBody<Vec<Todo>> {
    let guard = state.read().await;                // Arc -> RwLock -> 读锁
    JsonBody(guard.todos.clone())                  // 先 clone 快照，锁随函数结束释放
}
```

### 4.1 为什么是 `tokio::sync::RwLock` 而不是 `std::sync::Mutex`

- `std::sync::MutexGuard` **不是 `Send`**（解锁必须在同一线程）。tokio 的 handler 可能在
  线程 A 被 poll、`.await` 后在线程 B 继续，因此**跨 `.await` 持 std 锁**会让 Future 不满足
  `Send`，直接编译失败。就算写出不跨 await 的版本，std 锁竞争时是**阻塞等待**，会把
  tokio 的工作线程整个卡住；异步锁竞争时是「挂起任务，让出线程」。
- 代价：`tokio::sync::RwLock` 每次加锁更贵（原子操作 + 可能的任务挂起）。

判断标准是：**锁的作用域里有没有 `.await`？临界区是不是纯 CPU 的短操作？** 本 demo 两把锁
都不跨 await，用哪个都对；选异步锁是为了示范 async 语境下的正确姿势。

> ⚠️ **坑**：`std::sync::Mutex` + 短临界区在 async 里**很常见且推荐**，只要别把 guard 带过
> `.await`。反模式是「跨 await 持锁」，不是「用 std 锁」。

---

## 5. JSON 请求/响应与 `serde` 契约

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]      // created_at <-> createdAt
pub struct Todo { pub id: u64, pub title: String, pub done: bool, pub created_at: u64 }
```

`Json<T>` 同时是**提取器**（`T: DeserializeOwned`）和**响应**（`T: Serialize`），
所以 `Json<CreateTodo>` 当参数、`Json<Todo>` 当返回值都能用。

**为什么写 `rename_all = "camelCase"`？** JSON 字段名是对外契约，不该泄漏实现的命名习惯；
在结构体上声明一次，序列化和反序列化两边同时生效，比逐字段挂 `#[serde(rename = "...")]` 干净。

**请求体校验放哪？** 两种风格 demo 里都用了：**交给类型**（字段写 `String`/`bool`，
serde 拒绝类型不符的请求）；**留给自己**（demo 把 `CreateTodo.title` 写成 `Option<String>`，
于是 `{}` 和 `{"title":""}` 都能反序列化成功，再由 handler 返回**自己措辞的 400**，
消息可控、代价是多写几行校验）。

---

## 6. `IntoResponse` 与自定义错误

**axum 错误处理的全部秘密就是 `IntoResponse`**：「任何能变成 HTTP 响应」的 trait。
axum 为很多类型实现了它，还实现了这几条组合规则：

| 类型 | 含义 |
|---|---|
| `(StatusCode, T)` | 指定状态码 + 响应体（再加 `HeaderMap` 可补响应头） |
| `Result<T, E>`（两者都 `IntoResponse`） | **成功/失败二元组** |
| `impl IntoResponse for MyError` | 你自己的错误类型 |

最后两条合起来，就是「handler 里可以随便用 `?`」的原因：

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: self.code().to_string(), message: self.message() };
        (self.status(), Json(body)).into_response()    // 404 / 400 / 500 + 统一 JSON
    }
}
```

使用处就很干净：`guard.todos.iter().find(|t| t.id == id).cloned().map(JsonBody)
.ok_or(AppError::NotFound { resource: "todo".into(), id })`。

统一错误体 `{"error":"not_found","message":"todo 999 不存在"}` 里，`error` 是**机器可读**的
（前端 `switch` 它），`message` 是**人可读**的。别让前端正则匹配消息来决定跳哪个页面。

### 6.1 把 `Json` 的三种默认状态码归一

裸 `Json<T>` 失败时其实是**三个不同的码**（本机 axum 0.8.9 实测），三种纯文本 body
逼客户端写三套解析——用一个 newtype 包装即可全部归一成「400 + 统一 JSON」：

| 情况 | 裸 `Json` | 归一后 |
|---|---|---|
| 语法错 `not json` | `400` + `Failed to parse the request body as JSON: ...` | `400` + 统一 JSON |
| 合法 JSON 但类型不符 `{"title":123}` | `422` + `Failed to deserialize ...` | `400` + 统一 JSON |
| 忘了 `content-type: application/json` | `415` + `Expected request with Content-Type...` | `400` + 统一 JSON |

```rust
pub struct JsonBody<T>(pub T);                // 裸 Json<T> 换成这个，其余 handler 写法不变

impl<S, T> FromRequest<S> for JsonBody<T>
where Json<T>: FromRequest<S, Rejection = JsonRejection>,   // 复用 axum 的解析，只接管错误
      S: Send + Sync {
    type Rejection = AppError;                // 关键：拒绝类型换成自己的
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(JsonBody(v)),
            Err(rej) => Err(AppError::BadRequest { message: /* 按 rej 分类 */ }),
        }
    }
}
```

另一种写法是参数直接写 `Result<Json<T>, JsonRejection>`——axum 为 `Result<T, T::Rejection>`
实现了提取器，失败时你拿到 `Err(rejection)` 自己 match。**好处是零新类型，坏处是每个
handler 都要写一遍样板**；包装一次、处处受益更划算。`ApiPath<T>` 是同一套路的第二个例子。

---

## 7. 中间件：`tower::ServiceBuilder` + `TraceLayer` 简介

axum 的中间件系统就是 **tower 的 `Layer`**：

```toml
# 注意：这几个不在 demo 的 Cargo.toml 里，是练习 18-C 要你自己加的
tower-http = { version = "0.6", features = ["trace"] }
tracing-subscriber = "0.3"
```

```rust
use tower_http::trace::TraceLayer;

let app = app(state).layer(TraceLayer::new_for_http());
// 多层写法（顺序 = 从外到内）：
// let app = app(state).layer(ServiceBuilder::new()
//     .layer(TraceLayer::new_for_http())
//     .layer(TimeoutLayer::new(Duration::from_secs(5))));
```

`TraceLayer` 给每个请求建一个 `tracing` span，记录方法、URI、状态码、耗时。要自定义格式就
`.on_response(...)`——`OnResponse<B>` 的签名是
`fn on_response(self, response: &Response<B>, latency: Duration, span: &Span)`，`latency` 已算好。
两条必须知道的规则：**`layer` 连 404（没匹配上路由）也会经过，`route_layer` 只作用于匹配上的
路由**（做鉴权时很要命）；**`.layer(A).layer(B)` 里 A 在外层**，请求先进 A。

---

## 8. 用 `#[tokio::test]` 做集成测试

### 8.1 `tower::ServiceExt::oneshot`（不起真端口，快）

```rust
use axum::body::Body; use axum::http::{Request, StatusCode}; use tower::ServiceExt;

#[tokio::test]                               // tower 要进 dev-dependencies；AppState::new 要改成 pub
async fn create_and_list() {
    let state = std::sync::Arc::new(tokio::sync::RwLock::new(axum_todo::AppState::new()));
    let res = axum_todo::app(state).oneshot(   // Router 是 Clone 的，内部 Arc，很便宜
        Request::builder().method("POST").uri("/todos")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"学 Rust"}"#)).unwrap(),
    ).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let todo: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(todo["title"], "学 Rust");
    assert_eq!(todo["createdAt"], 1);          // camelCase 也是契约，值得断言
}
```

**`oneshot` 会消费掉一个 `Service`**，循环发请求时每次都要 `app.clone()`。

> ✅ **上面这段代码在仓库里是真的、可运行的**：
> [`projects/axum-todo/tests/api.rs`](../../projects/axum-todo/tests/api.rs) 就是这个模式，
> 共 5 个 `#[tokio::test]`（健康检查 / 完整 CRUD 流程 / 404 / 400 / 路径参数非法）。
> `cd projects/axum-todo && cargo test` 直接跑，全部通过、不起任何端口。
>
> 它能写出 `use axum_todo::app` 的前提是项目拆成了 **lib + bin**：
> `src/lib.rs` 放路由与类型，`src/main.rs` 只留「建运行时 → 起服务」。
> **二进制 crate 的内部对 `tests/` 不可见**，所以不拆就写不了集成测试——
> 这是「为什么要把 `app()` 单独拆出来」最实用的理由。

### 8.2 `reqwest` 打真实端口（更接近端到端）

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();      // 让内核给个空闲端口，并行测试不会互抢
tokio::spawn(async move { axum::serve(listener, axum_todo::app(state)).await.unwrap() });
let body: serde_json::Value = reqwest::Client::new()
    .post(format!("http://{addr}/todos")).json(&serde_json::json!({"title": "学 Rust"}))
    .send().await.unwrap().json().await.unwrap();
assert_eq!(body["title"], "学 Rust");
```

两种方式按需选：`oneshot` 快、不用端口；真端口能覆盖超时/断连等真实网络行为。

---

## 与 demo 的对照

打开 [`projects/axum-todo/src/main.rs`](../../projects/axum-todo/src/main.rs) 按段对照：

| demo 位置 | 对应 | 说明 |
|---|---|---|
| `main.rs:62-100` | §5 | `Todo`/`CreateTodo`/`UpdateTodo` + `rename_all` |
| `main.rs:123-141` | §4 | `AppState` + `SharedState`（注释解释了为什么不用 std 锁） |
| `main.rs:155-310` | §6, §6.1 | `AppError` + `IntoResponse`，以及 `JsonBody<T>`/`ApiPath<T>` 两个自定义提取器 |
| `main.rs:354-372` | §3.1 | `list_todos`：注释里完整写了顺序规则和 `debug_handler` |
| `main.rs:499-556` | §1, §2 | `app()` 路由表，以及 `#[tokio::main]` + `axum::serve` 启动 |
| `README.md` | 练习 | 三道改造题的验收标准与实测输出 |

三个最值得单独看的片段：**① 路由表（`app()`）** 注意 `"/todos/{id}"` 的花括号和
`get(..).patch(..).delete(..)`（一个路径、三个方法、三个 handler）；**② `AppError`** 它是
「领域错误」和「HTTP 表示」之间**唯一**的翻译层，业务代码只产出 `AppError`、不碰
`StatusCode`，想改状态码只改这一处；**③ `State` 的取用** `state.read().await` →
`guard.todos.clone()` → 返回，**先 clone 快照再返回**，锁随函数结束释放。

> 💡 demo 的 `Cargo.toml` **只列了 4 个依赖**（axum / tokio / serde / serde_json）。
> §7、§8.1 需要的 `tower-http` / `tracing-subscriber` / `tower` / `reqwest` 是练习 18-C
> 和写测试时你自己加的——这正是 axum「不全家桶」的体现。

---

## 本章练习

三道改造题，全部基于 demo 改，详细验收标准在
[`projects/axum-todo/README.md`](../../projects/axum-todo/README.md)。

| 题号 | 任务 | 考点 | 验收（一句话） |
|---|---|---|---|
| 18-A | `GET /todos?done=true\|false` 过滤 | `Query` 提取器 + 自定义拒绝映射 + 借用 | 三种查询返回正确子集，`?done=maybe` 返回 400 |
| 18-B | 持久化到 `todos.json` | `serde_json` + `std::fs` + 启动恢复状态 + 错误不 panic | 重启后数据还在；文件是坏 JSON 时服务仍能启动 |
| 18-C | 请求日志中间件 | `tower::ServiceBuilder` + `TraceLayer` + `tracing` | 每请求一行「方法 路径 状态码 耗时」，且原 curl 行为不变 |

**提示（不给答案）**

- **18-A**：`Query<T>` 和 `Path<T>` 一样是 `FromRequestParts`；非法 `bool` 会走
  `QueryRejection`，照 `JsonBody` 的样子包个 `ApiQuery<T>` 就统一了错误体。过滤时别把
  `Vec` move 走，用 `iter().filter(..)` 再 `cloned()`。
- **18-B**：`AppState` 加 `path: PathBuf`，构造函数改成 `AppState::load(path)`；读用
  `serde_json::from_str::<Vec<Todo>>`，失败就 `eprintln!` 警告 + 空列表（**别 `unwrap`**）；
  写回先写 `todos.json.tmp` 再 `rename`，避免留下半个文件；别忘了恢复
  `next_id = max(id) + 1`，否则重启后新 id 撞车。
- **18-C**：`TraceLayer` 最省事；想看清原理就先用 `axum::middleware::from_fn` 手写一个，
  你会发现要自己处理「响应已生成但 body 还没读完」的时机问题——这就是现成 layer 的价值。

---

## 常见坑

1. **路径参数语法 0.7 → 0.8**：`:id` 变成 `{id}`。0.8 有 v0.7 检查，写 `"/todos/:id"`
   **能编译但一启动就 panic**（本机实测），且错误信息直接给出正确写法：
   ``Path segments must not start with `:`. For capture groups, use `{capture}`.``
2. **提取器顺序**：消费 body 的（`Json`/`Form`/`String`/`Bytes`）**必须是最后一个参数**，
   其余 `FromRequestParts` 随意换序；报错看不出问题时加 `#[axum::debug_handler]`（需 `macros` feature）。
3. **handler 返回类型必须实现 `IntoResponse`**。常见翻车：返回 `Vec<Todo>` → 包成
   `Json(todos)`；返回 `Result<T, String>` → `String` 不实现 `IntoResponse`（只有
   `&'static str` 实现），要么换成 `AppError`，要么包 `(StatusCode, String)`。
4. **`Arc` 跨 `await` 的坑**：`std` 的 guard 不能跨 `.await`（不 `Send`）；`tokio::spawn` 要求
   Future `'static`，不能借 `&state` 进去，要 `let s = Arc::clone(&state); spawn(async move { .. })`。
5. **body 只能消费一次**：两个提取器都想读 body 会编译失败（第二个不满足 `FromRequestParts`）；
   需要多处使用时先提成 `Bytes` 或解析成结构体，再用 `Extension`/`State` 传下去。
6. **`axum` 与 `tower` 版本要配套**：axum 0.8 配 tower 0.5 / hyper 1.x，手写 `tower = "0.4"`
   会引入第二份 `Service` trait，报错极其费解（明明实现了却说不满足）——**别手写版本号**。
   另外忘了 `content-type: application/json` 会得到 415、类型不符会得到 422（见 §6.1）。

---

## 自测清单

- [ ] 我能说清 axum 只负责哪两件事，以及 `hyper`/`tokio`/`tower` 各负责什么
- [ ] 我能写出 `Router::new().route(..).with_state(..)` 的正确顺序，并解释 `.with_state()`
      为什么必须在最后
- [ ] 我能说出 `FromRequestParts` 和 `FromRequest` 的区别，以及为什么 `Json` 必须放最后；
      报 `Handler<_, _> is not satisfied` 时知道该加 `#[axum::debug_handler]`（需 `macros` feature）
- [ ] 我能解释 `std::sync::MutexGuard` 为什么不能跨 `.await`、什么时候 `std::sync::Mutex`
      反而更好；能给自定义错误实现 `IntoResponse` 并用 `(StatusCode, Json<T>)` 控制状态码
- [ ] 我知道裸 `Json` 在语法错/类型错/缺 content-type 时分别是 400/422/415，以及怎么归一
- [ ] 我知道 0.8 的路径参数是 `{id}`，写 `:id` 会在**启动时 panic** 并给出正确提示
- [ ] 我能用 `oneshot` 写一个不起真端口的集成测试，知道 `bind("127.0.0.1:0")` 的用途，
      并且完成了 18-A/18-B/18-C 中至少两道、`cargo clippy` 无警告
