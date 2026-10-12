//! # 第 6 章 · 错误处理
//!
//! 本章解决一个问题：**函数失败了，怎么把「失败」这件事告诉调用者。**
//! Rust 的答案是把「可能失败」写进类型：`Result<T, E>`，再用 `?` 把错误一层层往上送。
//!
//! 做法：把每个函数体里的 `unimplemented!()` 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch06_error_handling
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`parse_age`] | `Result<u8, String>`、先 `trim` 再解析、解析成功后才判断上界 |
//! | [`sum_numbers`] | `?` + `map_err`、空输入与全空白输入、逐段 `trim` |
//! | [`parse_port`] | 自定义错误枚举、先按 `u32` 解析再判断 `u16` 上界 |
//!
//! 提示：先读 `docs/01-syntax/ch06-error-handling.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 6-1：把用户输入的年龄解析成 `u8`。
///
/// 规则（按顺序执行，不要调换顺序）：
///
/// 1. 先对输入 `trim`（去掉首尾空白，得到下面说的「干净输入」）；
/// 2. 干净输入为空 → `Err("年龄不能为空".to_string())`；
/// 3. 把干净输入按 `u8` 解析，失败 → `Err(format!("不是合法数字: {s}"))`
///    （注意 `{s}` 是**原始**输入，方便定位用户到底填了什么）；
/// 4. 解析成功且大于 `150` → `Err("年龄超出范围".to_string())`；
/// 5. 其余情况 → `Ok(age)`。
///
/// 上界判断必须放在解析**成功之后**。另外 `u8` 最多到 255，
/// 所以像 `"256"` 这种输入会先在第 3 步解析失败，轮不到第 4 步。
///
/// # 示例
/// ```
/// # use ch06_error_handling::parse_age;
/// assert_eq!(parse_age(" 42 "), Ok(42));
/// assert_eq!(parse_age("0"), Ok(0));
/// assert_eq!(parse_age("150"), Ok(150));
/// assert_eq!(parse_age("151"), Err("年龄超出范围".to_string()));
/// assert_eq!(parse_age("   "), Err("年龄不能为空".to_string()));
/// assert_eq!(parse_age("abc"), Err("不是合法数字: abc".to_string()));
/// ```
pub fn parse_age(s: &str) -> Result<u8, String> {
    unimplemented!("练习 6-1：实现 parse_age")
}

/// 练习 6-2：把 `"1,2,3"` 这样的逗号分隔整数求和。
///
/// 规则：
///
/// 1. 先对整个输入 `trim`；为空（空串或全空白）→ `Ok(0)`；
/// 2. 按 `,` 切分，每一段再单独 `trim`；
/// 3. 任何一段解析成 `i64` 失败 → `Err(format!("不是合法数字: {seg}"))`，
///    其中 `seg` 是**该段 trim 之后**的内容；
/// 4. 全部成功 → `Ok(各段之和)`。
///
/// 用 `?` 把解析错误直接抛出去（配 `map_err` 换上自己的消息），不要写一长串 `match`。
///
/// # 示例
/// ```
/// # use ch06_error_handling::sum_numbers;
/// assert_eq!(sum_numbers("1,2,3"), Ok(6));
/// assert_eq!(sum_numbers(" 1 , 2 , 3 "), Ok(6));
/// assert_eq!(sum_numbers("   "), Ok(0));
/// assert_eq!(sum_numbers(""), Ok(0));
/// assert_eq!(sum_numbers("1,x"), Err("不是合法数字: x".to_string()));
/// ```
pub fn sum_numbers(input: &str) -> Result<i64, String> {
    unimplemented!("练习 6-2：实现 sum_numbers")
}

/// 练习 6-3 用的错误类型：一个端口号可能有三种失败原因。
///
/// 派生 `Debug`（`assert_eq!` 失败时要把它打印出来）和 `PartialEq` / `Eq`
/// （要能直接比较，测试才写得干净）。
#[derive(Debug, PartialEq, Eq)]
pub enum PortError {
    /// 输入为空，或只有空白字符。
    Empty,
    /// 不是合法数字，附带**原始**输入。
    NotANumber(String),
    /// 数值超出 `u16` 的范围，附带解析出来的那个数。
    TooLarge(u32),
}

/// 练习 6-3：解析端口号，失败原因用 [`PortError`] 表达。
///
/// 规则：
///
/// 1. `trim` 后为空 → `Err(PortError::Empty)`；
/// 2. **先按 `u32` 解析**（不是 `u16`！），失败 → `Err(PortError::NotANumber(s.to_string()))`；
/// 3. 数值大于 `65535` → `Err(PortError::TooLarge(v))`；
/// 4. 否则 → `Ok(v as u16)`。
///
/// 第 2 步如果直接按 `u16` 解析，`"65536"` 会在解析阶段就失败，
/// `TooLarge` 这个变体永远构造不出来——这正是本题要体会的坑。
///
/// # 示例
/// ```
/// # use ch06_error_handling::{parse_port, PortError};
/// assert_eq!(parse_port("8080"), Ok(8080));
/// assert_eq!(parse_port(" 443 "), Ok(443));
/// assert_eq!(parse_port("65535"), Ok(65535));
/// assert_eq!(parse_port("65536"), Err(PortError::TooLarge(65536)));
/// assert_eq!(parse_port(""), Err(PortError::Empty));
/// assert_eq!(parse_port("abc"), Err(PortError::NotANumber("abc".to_string())));
/// ```
pub fn parse_port(s: &str) -> Result<u16, PortError> {
    unimplemented!("练习 6-3：实现 parse_port")
}
