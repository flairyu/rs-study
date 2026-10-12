//! `axum-todo` 的可执行入口。
//!
//! 这里刻意**只留启动逻辑**，路由、状态、错误类型全在 [`axum_todo`] 这个库里：
//!
//! - 二进制 crate 的内部对 `tests/` 下的集成测试**不可见**，所以想测路由必须拆成 lib + bin；
//! - 这也是 Rust 项目的常见骨架：`lib.rs` 放业务，`main.rs` 只做「读配置 → 建运行时 → 起服务」。
//!
//! 运行：`cargo run`（监听 127.0.0.1:3000）

use std::sync::Arc;

use axum_todo::{app, print_bootstrap_hint, AppState, SharedState};
use tokio::sync::RwLock;

/// `#[tokio::main]` 把 `async fn main` 展开成「建运行时 -> block_on」。
/// 没有这个宏，`main` 里不能 `.await`（第 13 章讲过：Future 需要有人 poll）。
#[tokio::main]
async fn main() {
    let state: SharedState = Arc::new(RwLock::new(AppState::new()));

    // `TcpListener::bind` 是 tokio 的异步版本（std 的那个是阻塞的）。
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("端口 3000 绑定失败（是不是已经有实例在跑？）");
    let addr = listener.local_addr().expect("拿不到本地地址");

    print_bootstrap_hint(addr);

    // `axum::serve` 吃的是「实现了 axum::serve::Listener 的东西」+ `Router`。
    axum::serve(listener, app(state))
        .await
        .expect("server 异常退出");
}
