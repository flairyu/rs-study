//! 评测文件复用练习工程里的那一份（单一真相源，避免测试与参考实现漂移）。
//!
//! 这个 shim 让 `tests/chapter.rs` 与练习 crate **完全相同**：
//! 里面的 `use ch13_async::*` 会解析到本目录的参考答案实现。
#![allow(clippy::all)]
#[path = "../../../exercises/ch13_async/tests/chapter.rs"]
mod chapter;
