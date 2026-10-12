//! 集成测试：直接打真实的 `app()`，不起端口。
//!
//! 这正是教程 §8.1 讲的 `tower::ServiceExt::oneshot` 模式：
//! `oneshot` 把一个 `Router` 当成 `Service` 手工调用一次，**不经过 TCP**，
//! 所以测试又快又不会抢端口。
//!
//! 注意这个文件之所以能 `use axum_todo::app`，是因为项目已经拆成了 lib + bin：
//! 二进制 crate 的内部对 `tests/` **不可见**。这一点本身就是本章要学的工程结构。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum_todo::{app, AppState, SharedState};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;

fn fresh_state() -> SharedState {
    Arc::new(RwLock::new(AppState::new()))
}

/// 发一个请求，返回 (状态码, 解析后的 JSON；空 body 记为 `Value::Null`)。
async fn call(state: SharedState, req: Request<Body>) -> (StatusCode, Value) {
    let res = app(state).oneshot(req).await.expect("Router 调用失败");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("读 body 失败");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn post_todo(title: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/todos")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "title": title }).to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap()
}

fn patch(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn delete(uri: &str) -> Request<Body> {
    Request::builder().method("DELETE").uri(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn health_ok() {
    let (status, _) = call(fresh_state(), get("/health")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn full_crud_flow() {
    let state = fresh_state();

    // ① 初始为空
    let (status, body) = call(state.clone(), get("/todos")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]), "新建的列表应该是空的");

    // ② 创建 → 201，字段是 camelCase
    let (status, created) = call(state.clone(), post_todo("学 Rust")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["title"], "学 Rust");
    assert_eq!(created["done"], false);
    assert!(created.get("createdAt").is_some(), "应该有 camelCase 的 createdAt");
    let id = created["id"].as_u64().expect("应该返回 id");

    // ③ 列表里有 1 条
    let (_, list) = call(state.clone(), get("/todos")).await;
    assert_eq!(list.as_array().map(Vec::len), Some(1));

    // ④ 按 id 查单条
    let (status, one) = call(state.clone(), get(&format!("/todos/{id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["title"], "学 Rust");

    // ⑤ PATCH 切换 done
    let (status, patched) = call(
        state.clone(),
        patch(&format!("/todos/{id}"), json!({ "done": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["done"], true, "PATCH 之后 done 应该变成 true");

    // ⑥ DELETE → 204 且没有 body
    let (status, body) = call(state.clone(), delete(&format!("/todos/{id}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null, "204 不应该有 body");

    // ⑦ 删完再查列表 → 空
    let (_, list) = call(state.clone(), get("/todos")).await;
    assert_eq!(list, json!([]));
}

#[tokio::test]
async fn not_found_is_json_404() {
    let (status, body) = call(fresh_state(), get("/todos/999")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found", "错误体应该是统一 JSON 格式");
    assert!(body["message"].is_string(), "应该有可读的 message");
}

#[tokio::test]
async fn bad_json_is_json_400() {
    // 语法就不是 JSON
    let req = Request::builder()
        .method("POST")
        .uri("/todos")
        .header("content-type", "application/json")
        .body(Body::from("not json"))
        .unwrap();
    let (status, body) = call(fresh_state(), req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "bad_request");

    // 缺 content-type：axum 原生会给 415，这里被 JsonBody 统一成 400
    let req = Request::builder()
        .method("POST")
        .uri("/todos")
        .body(Body::from(r#"{"title":"x"}"#))
        .unwrap();
    let (status, body) = call(fresh_state(), req).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺 content-type 也要统一成 400 JSON"
    );
    assert_eq!(body["error"], "bad_request");
}

#[tokio::test]
async fn path_id_must_be_a_number() {
    let (status, body) = call(fresh_state(), get("/todos/abc")).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "路径参数不是数字应该 400 而不是 404"
    );
    assert_eq!(body["error"], "bad_request");
}
