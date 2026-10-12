//! # 第 1 章 · 变量、类型与可变性
//!
//! 本章 3 个练习只用到标准库。做法：把每个函数体里的 `unimplemented!()`
//! 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch01_basics
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`fahrenheit_to_celsius`] | 浮点类型、算术表达式、函数返回值 |
//! | [`split_duration`] | 整数除法与取模、元组返回、`let` 不可变绑定 |
//! | [`format_score_line`] | `as` 显式转换、浮点精度、`format!` 格式化 |
//!
//! 提示：先读 `docs/01-syntax/ch01-basics.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 1-1：华氏温度转摄氏温度。
///
/// 公式：`C = (F - 32) * 5 / 9`
///
/// 注意 Rust 里 `5 / 9` 是**整数**除法，结果是 `0`。
/// 要得到浮点结果，至少有一个操作数必须是浮点字面量：`5.0 / 9.0`。
///
/// # 示例
/// ```
/// # use ch01_basics::fahrenheit_to_celsius;
/// assert!((fahrenheit_to_celsius(212.0) - 100.0).abs() < 1e-9);
/// ```
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    unimplemented!("练习 1-1：实现华氏转摄氏")
}

/// 练习 1-2：把总秒数拆成 `(小时, 分钟, 秒)`。
///
/// 例如 `3725` 秒 = 1 小时 2 分 5 秒，返回 `(1, 2, 5)`。
///
/// 用 `/` 和 `%` 就能完成；注意 Rust 的整数除法是**向零截断**的。
/// 返回多值用元组，这在 Rust 里是零成本的值传递，不需要指针出参。
///
/// # 示例
/// ```
/// # use ch01_basics::split_duration;
/// assert_eq!(split_duration(3725), (1, 2, 5));
/// ```
pub fn split_duration(total_seconds: u32) -> (u32, u32, u32) {
    unimplemented!("练习 1-2：拆分总秒数为 (时, 分, 秒)")
}

/// 练习 1-3：生成一行成绩文本。
///
/// 输出格式固定为：`"{名字}: {得分}/{总分} ({百分比:.1}%)"`
///
/// 例如 `format_score_line("alice", 7, 8)` 必须返回 `"alice: 7/8 (87.5%)"`。
///
/// 考点：`u32` 之间相除还是整数，必须先 `as f64` 转成浮点，再算百分比；
/// `{:.1}` 表示保留 1 位小数（Python 的 `{:.1f}`，C 的 `%.1f`）。
///
/// # 示例
/// ```
/// # use ch01_basics::format_score_line;
/// assert_eq!(format_score_line("alice", 7, 8), "alice: 7/8 (87.5%)");
/// ```
pub fn format_score_line(name: &str, score: u32, total: u32) -> String {
    unimplemented!("练习 1-3：按指定格式生成成绩行")
}
