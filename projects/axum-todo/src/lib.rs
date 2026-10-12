//! # axum-todo —— 第 18 章配套 demo：内存版 todo REST API
//!
//! 运行：
//! ```bash
//! cargo run
//! ```
//! 然后另开一个终端：
//! ```bash
//! curl -s localhost:3000/health
//! curl -s -X POST localhost:3000/todos -H 'content-type: application/json' -d '{"title":"学 Rust"}'
//! curl -s localhost:3000/todos
//! ```
//!
//! 端点一览：
//!
//! | 方法 | 路径 | 说明 | 成功状态码 |
//! |---|---|---|---|
//! | GET | `/health` | 存活探针 | 200 |
//! | GET | `/todos` | 列出全部 | 200 |
//! | POST | `/todos` | 新建 | 201 |
//! | GET | `/todos/{id}` | 取一条 | 200 |
//! | PATCH | `/todos/{id}` | 改标题 / 切换完成状态 | 200 |
//! | DELETE | `/todos/{id}` | 删除 | 204 |
//!
//! 数据全部放在内存里，进程退出即丢——这是 demo 的取舍，不是 axum 的限制。
//!
//! ---
//!
//! ## 阅读指引（对应教程第 18 章）
//!
//! 1. [`AppState`]：为什么共享状态要包 `Arc`，以及为什么选 `tokio::sync::RwLock`。
//! 2. [`AppError`]：`IntoResponse` 是 axum 的统一出口，自定义错误就是实现它。
//! 3. [`JsonBody`]：包一层 `Json<T>`，把 axum 默认的 400/415/422 三种码统一成 400。
//! 4. [`app`]：路由表怎么写，`{id}` 这种占位符在 0.8 里才长这样。
//! 5. `main`：`#[tokio::main]` 之后，`axum::serve` 拿到的是 `TcpListener`。
//!
//! ⚠️ 本文件用到的提取器顺序规则、`Json` 必须最后一个等易错点，都在对应函数上方的
//! 注释里说明；教程文档 `docs/04-frameworks/ch18-web-axum.md` 有更完整的展开。

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// 1. 领域模型：请求体 / 响应体
// ---------------------------------------------------------------------------

/// 一条 todo。
///
/// `#[serde(rename_all = "camelCase")]` 让 Rust 的 snake_case 字段在 JSON 里变成
/// camelCase：`created_at` <-> `createdAt`。**这是 API 契约的一部分**，前端不用
/// 关心你后端用什么语言写的，所以别让 Rust 的命名习惯泄漏到 JSON 里。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
    /// 用「插入序号」当创建时间戳，避免为了 demo 拉一个时间库。
    pub created_at: u64,
}

/// `POST /todos` 的请求体。
///
/// 这里故意把 `title` 写成 `Option<String>`：这样 `{}` 和 `{"title": ""}` 都会
/// 反序列化成功，然后由我们自己的校验返回 **400 + 统一错误体**；如果写成
/// `String`，serde 会先抛 `JsonDataError`，虽然也能映射成 400，但错误消息是
/// serde 的措辞，不如自己写的好控制。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTodo {
    pub title: Option<String>,
}

/// `PATCH /todos/{id}` 的请求体。
///
/// 字段全 `Option`：**缺省 = 不改这一项**。注意区分「字段不存在」和「字段是 null」——
/// 两者对 `Option<T>` 都是 `None`，真要区分得用 `Option<Option<T>>`。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub done: Option<bool>,
}

/// 统一错误响应体：所有 4xx/5xx 都是这个形状。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: String,
    message: String,
}

// ---------------------------------------------------------------------------
// 2. 共享状态：Arc + tokio::sync::RwLock
// ---------------------------------------------------------------------------

/// 进程内共享的状态。
///
/// **为什么用 `tokio::sync::RwLock` 而不是 `std::sync::Mutex`？**
/// 两个理由，按重要性排序：
///
/// 1. **不能跨 `.await` 持锁。** `std::sync::MutexGuard` 不是 `Send`——解锁必须
///    发生在同一个线程。而 axum 的 handler 是跑在 tokio 工作线程池上的 `async fn`，
///    跨 `.await` 持有 std 锁会让整个 Future 不满足 `Send`，直接编译失败；就算
///    绕过去，阻塞式等锁也会白占一个工作线程，让"非阻塞"失去意义。
/// 2. 读写分离。`GET /todos` 只读，用 `read()` 可以让多个请求真正并发；
///    `std::sync::Mutex` 会把它们全排成一队。
///
/// 代价是 `tokio::sync::RwLock` 每次加锁都要走一次原子操作、锁竞争时可能挂起任务，
/// 比 `std::sync::Mutex` 略慢。判断标准是：**锁的作用域里有没有 `.await`**，以及
/// 临界区是不是纯 CPU 的短操作。这里两种都行，选异步锁是为了示范 async 语境下的
/// 正确姿势。
#[derive(Debug)]
pub struct AppState {
    todos: Vec<Todo>,
    /// 单调递增的 id 生成器，避免「取 max(id)+1」在并发下撞车。
    next_id: u64,
    /// 单调递增的序号，充作 createdAt。
    clock: u64,
}

/// `Clone` 的只是 `Arc`（一次引用计数 +1），`RwLock` 里的数据是所有人共享的同一份。
pub type SharedState = Arc<RwLock<AppState>>;

impl AppState {
    pub fn new() -> Self {
        Self {
            todos: Vec::new(),
            next_id: 1,
            clock: 1,
        }
    }
}

// clippy 建议：`new()` 不接受参数时应当同时提供 `Default`，
// 这样 `AppState::default()`、`#[derive(Default)]` 的容器、以及各种泛型代码都能直接用。
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 3. 错误处理：自定义 AppError 实现 IntoResponse
// ---------------------------------------------------------------------------

/// 应用级错误。
///
/// 关键点：**handler 的返回类型必须实现 `IntoResponse`**。`Result<T, AppError>`
/// 之所以能直接 `return`，是因为 axum 为 `Result<T, E> where T: IntoResponse,
/// E: IntoResponse` 实现了 `IntoResponse`。所以你只要给自己的错误类型实现
/// `IntoResponse`，就能在 handler 里用 `?` 一把梭。
#[derive(Debug)]
pub enum AppError {
    /// 资源不存在 -> 404
    NotFound { resource: String, id: u64 },
    /// 请求体 / 参数不合法 -> 400
    BadRequest { message: String },
    /// 兜底 -> 500
    Internal { message: String },
}

impl AppError {
    /// 唯一的「状态码表」：所有响应都从这里取码，不会出现同一个错误两种码。
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 机器可读的错误码（用于前端 switch），与 HTTP 状态码解耦。
    fn code(&self) -> &'static str {
        match self {
            AppError::NotFound { .. } => "not_found",
            AppError::BadRequest { .. } => "bad_request",
            AppError::Internal { .. } => "internal",
        }
    }

    /// 人类可读的说明。真实项目里 `Internal` 只应把细节写进日志，
    /// 返回给客户端的应该是模糊的一句话，避免泄漏内部信息。
    fn message(&self) -> String {
        match self {
            AppError::NotFound { resource, id } => format!("{resource} {id} 不存在"),
            AppError::BadRequest { message } => message.clone(),
            AppError::Internal { message } => format!("服务器内部错误: {message}"),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

/// 让 `AppError` 满足 `std::error::Error`，这样它才能进 `Box<dyn Error>`、
/// 被 `?` 自动转换、被 `anyhow`/`thiserror` 这类工具接管。
impl std::error::Error for AppError {}

/// **axum 错误处理的核心**：把领域错误翻译成 HTTP 响应。
///
/// 注意这里没有 `#[derive]`，也没有框架魔术——就是一个普通 trait。
/// 所有错误最终都变成「状态码 + JSON body」，客户端只需要解析一种结构。
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: self.code().to_string(),
            message: self.message(),
        };
        (self.status(), Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// 4. JsonBody / ApiPath：把提取失败也变成统一错误
// ---------------------------------------------------------------------------

/// `Json<T>` 的薄包装，只为了改一件事：**body 出问题时统一成 400 + 统一 JSON 体**。
///
/// 裸 `Json<T>` 在 0.8 里的三种失败其实是**三个不同的状态码**（本机实测，
/// axum 0.8.9）：
///
/// | 情况 | 裸 `Json` | 本包装 |
/// |---|---|---|
/// | body 不是合法 JSON | `400` + 纯文本 | `400` + 统一 JSON |
/// | JSON 合法但类型不符（`{"title":123}`） | `422` + 纯文本 | `400` + 统一 JSON |
/// | 忘了 `content-type: application/json` | `415` + 纯文本 | `400` + 统一 JSON |
///
/// 三种码、三种纯文本 body，会让客户端不得不写三套解析逻辑；统一成「400 + 一种
/// JSON 结构」后前端只写一套。有两种改法：
///
/// 1. 参数写 `Result<Json<T>, JsonRejection>`，在 handler 里手动 `match`；
/// 2. 像这里一样包一个 newtype，把拒绝类型在提取阶段就翻译成 `AppError`。
///
/// 第 2 种的好处是 **handler 签名保持干净**：拿到的就是 `T`，不需要每个 handler
/// 都写一遍样板 match。这也是实现自定义提取器的标准套路：
/// `impl FromRequest<S> for MyExtractor { type Rejection = ...; }`。
pub struct JsonBody<T>(pub T);

impl<S, T> FromRequest<S> for JsonBody<T>
where
    // `Json<T>: FromRequest` 意味着 `JsonBody<T>` 也消费 body，
    // 所以它必须出现在 handler 参数表的**最后一个**。
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(JsonBody(value)),
            Err(rejection) => Err(AppError::BadRequest {
                message: match rejection {
                    // 语法错 / 字段类型不对：都是「你发的 JSON 有问题」，统一 400。
                    JsonRejection::JsonSyntaxError(_) => "请求体不是合法的 JSON".to_string(),
                    JsonRejection::JsonDataError(_) => {
                        format!("JSON 结构与类型不匹配: {rejection}")
                    }
                    // 常见坑：忘了 `-H 'content-type: application/json'`。
                    // 没有这个头，axum 根本不会去读 body（默认给 415）。
                    JsonRejection::MissingJsonContentType(_) => {
                        "缺少 content-type: application/json".to_string()
                    }
                    // body 读取失败（超限、连接中断等），保留 axum 的原始措辞。
                    _ => rejection.body_text(),
                },
            }),
        }
    }
}

/// 让 `JsonBody<T>` 同时能当**响应**用：handler 直接 `return JsonBody(todo)`。
/// 复用 `Json` 的序列化逻辑，只在 `T: Serialize` 时可用。
impl<T: Serialize> IntoResponse for JsonBody<T> {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

/// `Path<T>` 的薄包装，作用同上：把路径参数解析失败也变成统一的 400。
///
/// `Path<u64>` 对 `/todos/abc` 这种请求会失败——类型是 `u64`，`abc` 解析不出来。
/// 用裸 `Path<u64>` 时 axum 返回 400 但 body 是纯文本；包一层后 body 和其他
/// 错误保持一致。
///
/// 它是 `FromRequestParts`（不消费 body），所以**可以放在 `JsonBody` 前面**。
pub struct ApiPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    Path<T>: FromRequestParts<S, Rejection = axum::extract::rejection::PathRejection>,
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(ApiPath(value)),
            Err(rejection) => Err(AppError::BadRequest {
                message: format!("路径参数不合法: {rejection}"),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// 5. handlers
// ---------------------------------------------------------------------------

/// `GET /health` —— 不碰状态，最小 handler。
///
/// handler 就是「参数实现 `FromRequest`/`FromRequestParts`，返回值实现
/// `IntoResponse`」的普通 `async fn`。`&'static str` 本身实现了 `IntoResponse`，
/// 所以这里不用 `Json`。
async fn health() -> &'static str {
    "ok"
}

/// `GET /todos` —— 读锁 + 快照。
///
/// ## 提取器顺序规则（初学者第一大坑）
///
/// 参数表被编译器按 **元组** 处理，而元组的实现规则是：
///
/// - 除**最后一个**之外，其余参数必须是 `FromRequestParts`（不消费 body）；
/// - **最后一个**参数才可以是 `FromRequest`（消费 body，比如 `Json`/`String`/`Bytes`）。
///
/// 这就是「`Json` 必须放在参数表最后」的由来。写成
/// `async fn bad_order(Json(x): Json<CreateTodo>, State(s): State<SharedState>)`
/// 会得到一句**完全没提到顺序**的报错（本机实测）：
///
/// ```text
/// error[E0277]: the trait bound `fn(Json<CreateTodo>, State<...>) -> ...:
///               Handler<_, _>` is not satisfied
///    = note: Consider using `#[axum::debug_handler]` to improve the error message
/// ```
///
/// 照着提示给 handler 加上 `#[axum::debug_handler]`（需要开启 axum 的 `macros`
/// feature），同一处错误就会变成人话：
///
/// ```text
/// error: `Json<_>` consumes the request body and thus must be the last argument
///        to the handler function
/// ```
///
/// `State<AppState>` 和 `ApiPath<u64>` 都是 `FromRequestParts`，它们之间**可以任意
/// 换序**——唯一的约束是「消费 body 的那个必须在最后」。
async fn list_todos(State(state): State<SharedState>) -> JsonBody<Vec<Todo>> {
    let guard = state.read().await;
    // clone 出快照后再返回：锁在函数结束时释放，而响应体是独立的数据。
    JsonBody(guard.todos.clone())
}

/// `GET /todos/{id}` —— 路径参数 + 404 演示。
///
/// ⚠️ **axum 0.7 -> 0.8 的破坏性变更**：路径占位符从 `:id` 改成了 `{id}`。
/// 好消息是它不会让你排查半天——0.8 在 `Router` 构建时做了 v0.7 检查，
/// 写成 `"/todos/:id"` 的代码**能编译通过，但一启动就 panic**（本机实测）：
///
/// ```text
/// thread 'main' panicked at src/main.rs:...:
/// Path segments must not start with `:`. For capture groups, use `{capture}`.
/// If you meant to literally match a segment starting with a colon,
/// call `without_v07_checks` on the router.
/// ```
///
/// 所以报错信息会直接告诉你正确写法。注意 `Router::without_v07_checks()` 可以关掉
/// 这个检查（会退化成「`:id` 被当成字面量段，`GET /todos/1` 静默 404」的老行为），
/// 除非确实要匹配以冒号开头的路径，否则别用。
async fn get_todo(
    State(state): State<SharedState>,
    ApiPath(id): ApiPath<u64>,
) -> Result<JsonBody<Todo>, AppError> {
    let guard = state.read().await;
    guard
        .todos
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .map(JsonBody)
        .ok_or(AppError::NotFound {
            resource: "todo".to_string(),
            id,
        })
}

/// `POST /todos` —— 唯一一个「消费 body」的 handler，`JsonBody` 必须放最后。
///
/// 成功返回 **201 Created**；校验失败返回 **400**（由 `AppError` 统一渲染）。
async fn create_todo(
    State(state): State<SharedState>,
    JsonBody(payload): JsonBody<CreateTodo>,
) -> Result<(StatusCode, JsonBody<Todo>), AppError> {
    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest {
            message: "title 不能为空".to_string(),
        })?
        .to_string();

    let mut guard = state.write().await;
    let todo = Todo {
        id: guard.next_id,
        title,
        done: false,
        created_at: guard.clock,
    };
    guard.next_id += 1;
    guard.clock += 1;
    guard.todos.push(todo.clone());

    // (StatusCode, T) 这种元组也实现了 IntoResponse，用来覆写默认的 200。
    Ok((StatusCode::CREATED, JsonBody(todo)))
}

/// `PATCH /todos/{id}` —— 部分更新，演示「两个 parts 提取器 + 一个 body 提取器」。
///
/// 语义：`title` 给了就改标题，`done` 给了就改完成状态；两个都没给算 400。
async fn update_todo(
    State(state): State<SharedState>,
    ApiPath(id): ApiPath<u64>,
    JsonBody(payload): JsonBody<UpdateTodo>,
) -> Result<JsonBody<Todo>, AppError> {
    if payload.title.is_none() && payload.done.is_none() {
        return Err(AppError::BadRequest {
            message: "至少要提供 title 或 done 其中之一".to_string(),
        });
    }
    // 提前校验，避免「先写了一半再发现不合法」这种部分生效。
    let new_title = match payload.title.as_deref() {
        Some(t) if t.trim().is_empty() => {
            return Err(AppError::BadRequest {
                message: "title 不能为空".to_string(),
            })
        }
        Some(t) => Some(t.trim().to_string()),
        None => None,
    };

    let mut guard = state.write().await;
    let todo = guard
        .todos
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or(AppError::NotFound {
            resource: "todo".to_string(),
            id,
        })?;

    if let Some(title) = new_title {
        todo.title = title;
    }
    if let Some(done) = payload.done {
        todo.done = done;
    }
    Ok(JsonBody(todo.clone()))
}

/// `DELETE /todos/{id}` —— 成功返回 **204 No Content**，body 为空。
///
/// 返回 `StatusCode` 就够；如果返回 `Json(...)` 会附上 body，204 带 body 是违约的。
async fn delete_todo(
    State(state): State<SharedState>,
    ApiPath(id): ApiPath<u64>,
) -> Result<StatusCode, AppError> {
    let mut guard = state.write().await;
    let len_before = guard.todos.len();
    guard.todos.retain(|t| t.id != id);
    if guard.todos.len() == len_before {
        return Err(AppError::NotFound {
            resource: "todo".to_string(),
            id,
        });
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// 6. 路由表
// ---------------------------------------------------------------------------

/// 组装 `Router`。
///
/// 拆成独立函数（而不是全写在 `main` 里）的好处：**集成测试可以复用同一个
/// `Router`**，用 `tower::ServiceExt::oneshot` 直接打内存里的服务，不用起真端口。
///
/// 注意 `.with_state(state)` 放在最后：它把 `Router<SharedState>` 变成
/// `Router<()>`（状态已经被"焊死"在路由里）。之后 `axum::serve` 要的就是
/// `Router<()>`，所以顺序反了会编译不过。
pub fn app(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/todos", get(list_todos).post(create_todo))
        // 同一路径不同方法：method-router 链式注册，一眼看出支持哪些方法
        //（等价写法：`.route("/todos/{id}", patch(update_todo))` 再叠一层）。
        .route(
            "/todos/{id}",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// 7. 启动提示（供 bin 调用）
// ---------------------------------------------------------------------------

/// 启动时打印几条 curl，省得用户去翻 README。
///
/// 故意写 `localhost` 而不是 `127.0.0.1`：多数终端会把前者渲染成可点击链接。
pub fn print_bootstrap_hint(addr: std::net::SocketAddr) {
    let base = format!("http://localhost:{}", addr.port());
    println!("axum-todo 已启动");
    println!("  监听地址 : http://{addr}（也就是 {base}）");
    println!("  健康检查 : {base}/health");
    println!();
    println!("试试这些命令：");
    println!("  curl -s {base}/health");
    println!("  curl -s {base}/todos");
    println!(
        "  curl -s -X POST {base}/todos -H 'content-type: application/json' -d '{{\"title\":\"学 Rust\"}}'"
    );
    println!("  curl -s -X PATCH {base}/todos/1 -H 'content-type: application/json' -d '{{\"done\":true}}'");
    println!("  curl -s -o /dev/null -w '%{{http_code}}\\n' -X DELETE {base}/todos/1");
    println!("  curl -s {base}/todos/999   # 期望 404 + JSON 错误体");
    println!();
    println!("按 Ctrl-C 退出。");
}
