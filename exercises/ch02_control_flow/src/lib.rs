//! # 第 2 章 · 函数与控制流
//!
//! 本章把「函数怎么写、控制流怎么走」一次讲清：函数体最后一个表达式就是返回值，
//! `if` / `loop` / `match` 都是**表达式**（有值），而 `for` 是遍历集合的默认写法。
//!
//! 运行评测：
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch02_control_flow
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`fizzbuzz`] | `if` 是表达式、取模、`String` 与 `&str` 的分工 |
//! | [`collatz_steps`] | `loop` + `break value`、提前 `return`、边界值防死循环 |
//! | [`membership_level`] | `match (years, points)` + `if` 守卫、分支顺序与穷尽性 |
//!
//! 提示：先读 `docs/01-syntax/ch02-control-flow.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 2-1：FizzBuzz。
///
/// 规则（按优先级）：
///
/// - `n` 同时被 3 和 5 整除 → `"FizzBuzz"`
/// - 只被 3 整除 → `"Fizz"`
/// - 只被 5 整除 → `"Buzz"`
/// - 其余 → `n` 的十进制字符串
///
/// 注意边界：`0 % 3 == 0` 且 `0 % 5 == 0`，所以 `fizzbuzz(0) == "FizzBuzz"`。
///
/// # 示例
/// ```
/// # use ch02_control_flow::fizzbuzz;
/// assert_eq!(fizzbuzz(15), "FizzBuzz");
/// assert_eq!(fizzbuzz(3), "Fizz");
/// assert_eq!(fizzbuzz(5), "Buzz");
/// assert_eq!(fizzbuzz(7), "7");
/// assert_eq!(fizzbuzz(0), "FizzBuzz");
/// ```
pub fn fizzbuzz(n: u32) -> String {
    unimplemented!("练习 2-1：实现 FizzBuzz")
}

/// 练习 2-2：Collatz（角谷猜想）步数。
///
/// 从 `n` 出发：偶数就除以 2，奇数就乘 3 加 1，返回到达 `1` 所需要的步数。
///
/// 边界：`collatz_steps(0) == 0`，`collatz_steps(1) == 0`。
///
/// 提示：`0` 必须在进循环之前就返回，否则 `0 / 2` 还是 `0`，循环永远不会结束。
/// 另外注意：`n` 是 `u64`，接近 `u64::MAX` 的奇数做 `3 * n + 1` 会溢出。
///
/// # 示例
/// ```
/// # use ch02_control_flow::collatz_steps;
/// assert_eq!(collatz_steps(0), 0);
/// assert_eq!(collatz_steps(1), 0);
/// assert_eq!(collatz_steps(6), 8);
/// assert_eq!(collatz_steps(27), 111);
/// ```
pub fn collatz_steps(n: u64) -> u32 {
    unimplemented!("练习 2-2：计算 Collatz 步数")
}

/// 练习 2-3：会员等级。
///
/// 用 `match (years, points)` 加 `if` 守卫，**严格按下面的顺序**判断，
/// 第一个命中的分支就是结果：
///
/// 1. `years >= 5 || points >= 1000` → `"钻石"`
/// 2. `years >= 3 || points >= 500`  → `"黄金"`
/// 3. `years >= 1 || points >= 100`  → `"白银"`
/// 4. 其余                            → `"青铜"`
///
/// 返回类型是 `&'static str`，所以每个分支都返回字符串**字面量**，
/// 不要写 `String`（那需要 `.as_str()`，生命周期也不再是 `'static`）。
///
/// 提示：所有分支都带 `if` 守卫时，编译器不认为 `match` 已经穷尽，
/// 必须补一个 `_ => "青铜"` 兜底。
///
/// # 示例
/// ```
/// # use ch02_control_flow::membership_level;
/// assert_eq!(membership_level(6, 0), "钻石");
/// assert_eq!(membership_level(4, 0), "黄金");
/// assert_eq!(membership_level(1, 0), "白银");
/// assert_eq!(membership_level(0, 99), "青铜");
/// ```
pub fn membership_level(years: u32, points: u32) -> &'static str {
    unimplemented!("练习 2-3：按顺序判定会员等级")
}
