//! # 第 16 章 · 标准库巡礼
//!
//! 本章 3 个练习只用到标准库，用来把「std 到底提供了什么」变成手感：
//! 文件 IO、路径、数字解析与格式化，以及 `io::Result` 这条错误通道。
//! 做法：把每个函数体里的 `unimplemented!()` 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch16_stdlib_tour
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`count_words_in_file`] | `std::fs` 与 `std::io`、`io::Result`、错误不加包装地传播 |
//! | [`file_stem_and_ext`] | `std::path::Path` 的解析规则、`Option` 边界、`PathBuf` vs `String` |
//! | [`round_trip_number`] | `FromStr` / `str::parse`、`ParseFloatError`、`{:.2}` 格式化 |
//!
//! 提示：先读 `docs/02-stdlib/ch16-stdlib-tour.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 16-1：统计一个文本文件里有多少个「单词」。
///
/// 规格：
/// - 用 `std::fs::read_to_string` 一次性读出整个文件；
/// - 用 `str::split_whitespace()` 分词并计数（任意空白分隔，连续空白不算空词）；
/// - 返回 `std::io::Result<usize>`：**文件不存在、没有权限、不是合法 UTF-8 时，
///   把 `io::Error` 原样返回**，不要 `unwrap`，也不要换成 `String`。
///
/// 边界：空文件返回 `Ok(0)`；只有空白（空格 / 换行 / 制表符）也返回 `Ok(0)`。
///
/// 注意：`?` 在这里可以直接用——函数的返回类型就是 `Result`，
/// `io::Error` 不需要任何转换（这是第 6 章 `?` 的延续）。
///
/// # 示例
/// ```
/// # use ch16_stdlib_tour::count_words_in_file;
/// let path = std::env::temp_dir().join(format!("ch16_doctest_{}.txt", std::process::id()));
/// std::fs::write(&path, "hello world\nfoo\tbar").unwrap();
/// assert_eq!(count_words_in_file(&path).unwrap(), 4, "四个单词");
/// std::fs::remove_file(&path).unwrap();
///
/// // 文件不存在时返回 Err，而不是 panic
/// let missing = std::path::Path::new("/definitely/not/here/ch16-missing.txt");
/// assert!(count_words_in_file(missing).is_err());
/// ```
pub fn count_words_in_file(path: &std::path::Path) -> std::io::Result<usize> {
    unimplemented!("练习 16-1：读文件并统计单词数")
}

/// 练习 16-2：把一个路径字符串拆成 `(主名, 扩展名)`。
///
/// 规格（用 `std::path::Path` 自带的规则，不要手写字符串切分）：
/// - 有主名、有扩展名：`"/tmp/notes.tar.gz"` → `("notes.tar".to_string(), Some("gz".to_string()))`
///   （扩展名只取**最后一个**点之后的部分）；
/// - 有主名、无扩展名：`"/tmp/README"` → `("README".to_string(), None)`；
/// - 无主名（空串、`"/"`、`"."`、`".."`）→ `("".to_string(), None)`。
///
/// 提示：`Path::file_stem()` / `Path::extension()` 返回的都是 `Option<&OsStr>`，
/// 用 `map(...)` + `unwrap_or_default()` 就能把 `Option` 折叠掉。
///
/// # 示例
/// ```
/// # use ch16_stdlib_tour::file_stem_and_ext;
/// assert_eq!(
///     file_stem_and_ext("/tmp/notes.tar.gz"),
///     ("notes.tar".to_string(), Some("gz".to_string()))
/// );
/// assert_eq!(file_stem_and_ext("/tmp/README"), ("README".to_string(), None));
/// assert_eq!(file_stem_and_ext(".."), ("".to_string(), None));
/// ```
pub fn file_stem_and_ext(path: &str) -> (String, Option<String>) {
    unimplemented!("练习 16-2：取出文件主名与扩展名")
}

/// 练习 16-3：把字符串解析成数字，四舍五入到 2 位小数后格式化输出。
///
/// 规格：
/// - 用 `str::parse::<f64>()`（也就是 `FromStr`）解析 `s`；
/// - 解析失败时把 `std::num::ParseFloatError` **原样**返回（不要转成 `String`，也不要 panic）；
/// - 成功后四舍五入到 2 位小数，再用 `{:.2}` 格式化：`"3.14159"` → `"3.14"`，
///   `"2.5"` → `"2.50"`（注意：`{:.2}` 会补零），`"42"` → `"42.00"`。
///
/// > ⚠️ 浮点陷阱：「四舍五入到 2 位」有两条路，它们**不总是等价**：
/// > `format!("{:.2}", 2.005_f64)` 得到 `"2.00"`，而
/// > `(2.005_f64 * 100.0).round() / 100.0` 得到 `2.01`。
/// > 原因是 `2.005` 在 f64 里其实是 `2.00499999999999989…`，但它乘 100 之后
/// > 恰好被舍入到 `200.5`，`round()` 就进位了；而 `{:.2}` 是直接对原值做正确舍入。
/// > 两条路在 `0.125`、`2.675` 这类「卡在中间」的数上也会给出不同答案。
/// > 这不是你的 bug——测试里请用不敏感的输入（例如 `"3.14159"`、`"2.5"`、`"42"`）。
///
/// # 示例
/// ```
/// # use ch16_stdlib_tour::round_trip_number;
/// assert_eq!(round_trip_number("3.14159").unwrap(), "3.14");
/// assert_eq!(round_trip_number("2.5").unwrap(), "2.50");
/// assert_eq!(round_trip_number("42").unwrap(), "42.00");
/// assert!(round_trip_number("abc").is_err());
/// ```
pub fn round_trip_number(s: &str) -> Result<String, std::num::ParseFloatError> {
    unimplemented!("练习 16-3：解析数字、保留两位小数、格式化")
}
