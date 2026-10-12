//! # 第 1 章 · 变量、类型与可变性 —— **参考答案实现**
//!
//! 这个 crate 用来验证 `exercises/ch01_basics/tests/chapter.rs` 是**可通过**的。
//! 它不是给学习者抄的（答案在教程文档的 `<details>` 里），而是仓库自检的一部分：
//!
//! ```bash
//! cd solutions && cargo test -p ch01_basics
//! ```
#![allow(unused_variables)]

/// 练习 1-1：华氏温度转摄氏温度。
///
/// # 示例
/// ```
/// # use ch01_basics::fahrenheit_to_celsius;
/// assert!((fahrenheit_to_celsius(212.0) - 100.0).abs() < 1e-9);
/// ```
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    // `5.0 / 9.0` 而不是 `5 / 9`：后者是整数除法，结果是 0。
    (fahrenheit - 32.0) * 5.0 / 9.0
}

/// 练习 1-2：把总秒数拆成 `(小时, 分钟, 秒)`。
///
/// # 示例
/// ```
/// # use ch01_basics::split_duration;
/// assert_eq!(split_duration(3725), (1, 2, 5));
/// ```
pub fn split_duration(total_seconds: u32) -> (u32, u32, u32) {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    (hours, minutes, seconds)
}

/// 练习 1-3：生成一行成绩文本。
///
/// # 示例
/// ```
/// # use ch01_basics::format_score_line;
/// assert_eq!(format_score_line("alice", 7, 8), "alice: 7/8 (87.5%)");
/// ```
pub fn format_score_line(name: &str, score: u32, total: u32) -> String {
    // 先转 f64，否则 7 / 8 会是 0。
    let percent = score as f64 / total as f64 * 100.0;
    format!("{name}: {score}/{total} ({percent:.1}%)")
}
