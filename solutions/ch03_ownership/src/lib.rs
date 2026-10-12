//! # 第 3 章 · 所有权
//!
//! 本章 3 个练习都是"必须把所有权交出去"的小函数，跑一遍就能体会
//! 值在函数边界上是怎么移动的：传进去、还回来、以及不还回来时编译器怎么拦你。
//!
//! 运行评测：
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch03_ownership
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`shout`] | 传参即转移：`String` 按值传入后，调用方不再拥有它 |
//! | [`join_and_count`] | 消费 `Vec<String>`：用 `into_iter()` 逐个拿走元素 |
//! | [`consume_and_give_back`] | 转移出去再还回来；`String::len()` 是字节数 |
//!
//! 提示：先读 `docs/01-syntax/ch03-ownership.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 3-1：把字符串变成全大写。
///
/// 参数 `s` 的类型是 `String`（拥有堆上字节的字符串），不是 `&str`。
/// 所以调用 `shout(text)` 时，`text` 的所有权被**移动**进函数：
/// 函数结束后 `s` 在这里被 `Drop`，堆内存随之释放。
///
/// 返回值是一个**新的** `String`（`to_uppercase()` 会分配新内存），
/// 它的所有权随返回值交回调用方。
///
/// 边界：空字符串返回空字符串；含中文等多字节 UTF-8 的字符串按 Unicode 规则
/// 转换（汉字没有大小写，原样保留）。
///
/// # 示例
/// ```
/// # use ch03_ownership::shout;
/// assert_eq!(shout(String::from("hello")), "HELLO");
/// assert_eq!(shout(String::new()), "");
/// assert_eq!(shout(String::from("中文")), "中文");
/// ```
pub fn shout(s: String) -> String {
    // 按值接收即拥有所有权；to_uppercase() 分配一个新的 String 交回调用方。
    // 这里用的是 Unicode 大写映射，所以 "café" → "CAFÉ"，
    // 而汉字没有大小写，原样保留。
    s.to_uppercase()
}

/// 练习 3-2：统计单词个数，并用单个空格把它们连起来。
///
/// 输入是 `Vec<String>`，**按值**传入——这个 `Vec` 连同里面每个 `String`
/// 的所有权都归函数所有。要遍历时把元素取出来用，得用消费型迭代器
/// `into_iter()`（产出 `String`），而不是只借出 `&String` 的 `iter()`。
///
/// 返回 `(单词个数, 连接后的字符串)`；空 `Vec` 返回 `(0, "")`。
/// 「单词个数」就是元素个数，**不做**按空格二次切分，所以
/// `vec!["a b".to_string()]` 返回 `(1, "a b")`。
///
/// 提示：`words.join(" ")` 需要 `&[String]`，会借用 `words`；
/// 先 `let count = words.len();` 再 join，顺序反了虽然也能编过，
/// 但先取长度是更不容易出错的习惯。
///
/// # 示例
/// ```
/// # use ch03_ownership::join_and_count;
/// let words = vec!["hello".to_string(), "world".to_string()];
/// assert_eq!(join_and_count(words), (2, "hello world".to_string()));
/// assert_eq!(join_and_count(Vec::new()), (0, String::new()));
/// ```
pub fn join_and_count(words: Vec<String>) -> (usize, String) {
    // 先把元素个数存进 usize（Copy，不牵动 words），再让 join 借用 words。
    // count 就是元素个数，不做按空格二次切分。
    let count = words.len();
    let joined = words.join(" ");
    (count, joined)
}

/// 练习 3-3：把字符串原样还回来，同时报告它的**字节**长度。
///
/// 返回 `(原字符串, 字节长度)`。重点是"转移出去再还回来"这个模式：
/// 想把 `s` 交给函数又不丢失它，只能让它出现在返回值里，
/// 调用方再重新绑定：`let (s, n) = consume_and_give_back(s);`。
/// 第 4 章的借用就是为了免掉这种啰嗦。
///
/// 注意 `String::len()` 返回 UTF-8 **字节数**，不是字符数：
/// `"中文".len() == 6`，而 `"中文".chars().count() == 2`。
///
/// 提示：先 `let n = s.len();` 再构造 `(s, n)`。如果写成 `(s, s.len())`，
/// 元组会先求值 `s`（把它移动进去），再对已移动的 `s` 求长度，编译不过。
///
/// # 示例
/// ```
/// # use ch03_ownership::consume_and_give_back;
/// assert_eq!(consume_and_give_back(String::from("abc")), ("abc".to_string(), 3));
/// assert_eq!(consume_and_give_back(String::from("中文")), ("中文".to_string(), 6));
/// ```
pub fn consume_and_give_back(s: String) -> (String, usize) {
    // 顺序是关键：先取长度（usize 是 Copy），再把 s 移动进元组还回去。
    // 写成 (s, s.len()) 会先移动 s 再求长度，触发 E0382。
    // 注意 len() 数的是 UTF-8 字节数："中文".len() == 6。
    let n = s.len();
    (s, n)
}
