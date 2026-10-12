//! # 第 4 章 · 引用与借用
//!
//! 第 3 章的 `shout(s: String)` 会把 `String` 的所有权吞掉，调用完原变量就不能用了。
//! 本章解决这个问题：**把「访问权」借出去，而不是把「所有权」送出去**。
//!
//! 运行评测：
//! ```bash
//! python3 tools/check.py ch04_borrowing
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`longest_word`] | `&str` 入参 / `&str` 返回、省略规则、`split_whitespace` 切片 |
//! | [`append_exclaim`] | `&mut String` 可变借用、`push` 修改被借用的值 |
//! | [`halves`] | `&[i32]` 切片借用、`split_at` 返回两个共享借用 |
//!
//! 提示：先读 `docs/01-syntax/ch04-borrowing.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 4-1：返回第一个最长的单词（切片）。
///
/// 按空白（空格、制表符、换行都算）把 `text` 分词，返回**最长**的那个单词。
/// 返回的不是新 `String`，而是 `text` 内部的一段切片——所以不需要拷贝。
///
/// 规则：
/// - 空输入或全空白 → 返回 `""`（空的 `&str`，不是 `None`）；
/// - 多个单词长度相同 → 取**靠前**的那个；
/// - 单词之间的连续空白不会被算进任何单词。
///
/// 考点：`&str` 收 `&str` 返回时，返回值按**省略规则**与唯一的输入引用绑定——
/// 你返回的切片必须来自 `text` 本身，不能来自函数内新建的 `String`。
///
/// # 示例
/// ```
/// # use ch04_borrowing::longest_word;
/// assert_eq!(longest_word("the quick brown fox"), "quick");
/// assert_eq!(longest_word(""), "");
/// ```
pub fn longest_word(text: &str) -> &str {
    // 返回的全是 `text` 内部的切片，零拷贝。
    // `best` 初值 `""` 是 `'static`，赋成 `word` 后被借用检查器收窄为与 `text` 同寿命。
    let mut best = "";
    for word in text.split_whitespace() {
        // 用 `>` 而不是 `>=`：等长时保留靠前的那个。
        if word.len() > best.len() {
            best = word;
        }
    }
    // 空输入 / 全空白时循环体一次都不执行，`best` 仍是 `""`。
    best
}

/// 练习 4-2：在字符串末尾追加一个 `!`。
///
/// 参数是 `&mut String`：函数**不接管所有权**，但获得了修改它的独占权限。
/// 调用方传 `&mut s`，调用返回后 `s` 仍然归调用方所有（只是内容变了）。
///
/// 规则：每调用一次就多一个 `!`；空字符串调用一次得到 `"!"`；
/// 字符串里已经有 `!` 也不做特殊处理，继续追加。
///
/// 考点：同一作用域内，一个值要么有任意多个 `&T`，要么只有一个 `&mut T`。
/// 传 `&mut s` 的那段时间里，你不能再读 `s`。
///
/// # 示例
/// ```
/// # use ch04_borrowing::append_exclaim;
/// let mut s = String::from("hi");
/// append_exclaim(&mut s);
/// assert_eq!(s, "hi!");
/// ```
pub fn append_exclaim(s: &mut String) {
    // 借来的 `&mut String` 上调用 `push`：改的是调用方的那个 String。
    s.push('!');
}

/// 练习 4-3：从中间把切片切成两半。
///
/// 切点在 `nums.len() / 2`（整数除法、向下取整），返回 `(前半段, 后半段)`。
///
/// 规则：
/// - `[]` → `(&[], &[])`；
/// - `[7]` → `(&[], &[7])`（切点是 0，前半段为空）；
/// - `[1, 2, 3]` → `(&[1], &[2, 3])`（切点是 1）；
/// - `[1, 2, 3, 4]` → `(&[1, 2], &[3, 4])`。
///
/// 考点：返回的两个 `&[i32]` 都是对 `nums` 的**共享借用**，
/// 多个不可变借用可以同时存在，这正是它合法（且零拷贝）的原因。
///
/// # 示例
/// ```
/// # use ch04_borrowing::halves;
/// assert_eq!(halves(&[1, 2, 3, 4]), (&[1, 2][..], &[3, 4][..]));
///
/// let (empty, rest) = halves(&[]);
/// assert!(empty.is_empty());
/// assert!(rest.is_empty());
/// ```
pub fn halves(nums: &[i32]) -> (&[i32], &[i32]) {
    // `split_at` 返回的两段都是 `nums` 上的共享借用视图：
    // 前段的指针就是 `nums.as_ptr()`，没有任何拷贝。
    nums.split_at(nums.len() / 2)
}
