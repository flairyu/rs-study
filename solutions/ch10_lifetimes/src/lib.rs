//! # 第 10 章 · 生命周期
//!
//! 第 4 章的 `longest_word(text: &str) -> &str` 之所以能编译，是因为**省略规则**
//! 悄悄替你写上了生命周期。本章把省略掉的东西显式写出来：为什么需要 `'a`、
//! 它到底向编译器承诺了什么、什么时候必须自己写、以及 `'static` 的真正含义。
//!
//! 运行评测：
//! ```bash
//! python3 tools/check.py ch10_lifetimes
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`longest`] | 显式生命周期参数、把两个输入绑到同一个 `'a`、等长取第一个 |
//! | [`Parser`] | 结构体持有引用必须标注、`&self` 返回 `&'a str`、按 `char_indices` 切片 |
//! | [`first_or`] | 返回值生命周期来自切片元素、`Option<&String>` → `&str` |
//!
//! 提示：先读 `docs/01-syntax/ch10-lifetimes.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 10-1：返回两个字符串里**较长**的那一个（返回切片，不拷贝）。
///
/// 两个参数都是 `&'a str`：它们被**绑定到同一个生命周期** `'a`，
/// 因此返回值也被约束在 `'a` 内——只有 `a` 和 `b` 都还有效时，结果才能用。
///
/// 规则：
/// - 长度不同 → 返回较长者；
/// - 长度相同（包括两个都是 `""`）→ 返回第一个参数 `a`；
/// - 比较的是**字节长度**（`str::len`）。
///
/// 考点：这是"两个输入引用、要返回引用"的典型场景。省略规则在这里失效
/// （编译器不知道返回值借用谁），必须显式写 `'a`。注意 `'a` 不是"让变量活更久"，
/// 而是一句承诺：输入的寿命和输出的寿命是同一个。
///
/// # 示例
/// ```
/// # use ch10_lifetimes::longest;
/// assert_eq!(longest("hello", "hi"), "hello");
/// assert_eq!(longest("ab", "cde"), "cde");
/// assert_eq!(longest("abc", "xyz"), "abc"); // 等长返回 a
/// ```
pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    // `>=` 而不是 `>`：等长时走 `a` 分支，满足「等长返回 a」的约定。
    if a.len() >= b.len() {
        a
    } else {
        b
    }
}

/// 练习 10-2：一个"借用输入"的极简分词器。
///
/// `input` 是**借来的**，不是拥有的——所以结构体必须写成 `Parser<'a>`。
/// `pos` 是字节下标，永远落在 `char` 边界上。
///
/// 方法语义：
/// - `new(input)`：`pos` 初始化为 `0`；
/// - `rest()`：返回 `input[pos..]`，类型是 `&'a str`——借用的是 **input 的数据**，
///   不是 `self`；
/// - `next_word()`：先跳过前导空白（空格 / 制表符 / 换行 / 回车都算空白），
///   把 `pos` 推进到该单词之后的第一个空白处（后面没有空白就推进到输入末尾），
///   并返回这个单词；如果 `pos` 之后只剩空白（或已到末尾），返回 `None` 且 `pos` 不变。
///
/// 规则：
/// - `""` → `next_word()` 返回 `None`，`rest()` 返回 `""`；
/// - 全空白输入 → `next_word()` 返回 `None`，`rest()` 原样返回整段（`pos` 仍是 0）；
/// - 连续多个空格不会被当成"空单词"；
/// - 末尾有没有空白都可以（`"alpha beta"` 的第二个单词末尾就是输入末尾）。
///
/// ⚠️ 坑：切片必须落在 **char 边界**上。不要用固定的字节数推进 `pos`，
/// 用 `input[pos..].char_indices()` 找到下一个空白字符的字节偏移再切。
///
/// # 示例
/// ```
/// # use ch10_lifetimes::Parser;
/// let mut p = Parser::new("  hello   world ");
/// assert_eq!(p.next_word(), Some("hello"));
/// assert_eq!(p.rest(), "   world ");
/// assert_eq!(p.next_word(), Some("world"));
/// assert_eq!(p.next_word(), None);
/// ```
pub struct Parser<'a> {
    pub input: &'a str,
    pub pos: usize,
}

impl<'a> Parser<'a> {
    /// 创建一个从 `input` 开头开始扫描的解析器。
    pub fn new(input: &'a str) -> Self {
        Parser { input, pos: 0 }
    }

    /// 返回尚未消费的剩余输入。
    ///
    /// 注意签名是 `-> &'a str` 而不是 `-> &str`：返回值借用的是 **input 的数据**，
    /// 和 `self` 这个结构体本身无关（`Parser` 马上被 drop 也不影响结果）。
    /// 写成 `-> &str` 会被省略规则绑到 `&self` 上，比实际需要更严格。
    ///
    /// # 示例
    /// ```
    /// # use ch10_lifetimes::Parser;
    /// let mut p = Parser::new("hello world");
    /// assert_eq!(p.rest(), "hello world");
    /// let _ = p.next_word();
    /// assert_eq!(p.rest(), " world"); // pos 停在单词后的第一个空白处，空白保留在 rest 里
    /// ```
    pub fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    /// 取出下一个单词（输入的一段切片），并推进 `pos`。
    ///
    /// 没有剩余单词时返回 `None`，并且**不改变** `pos`。
    ///
    /// # 示例
    /// ```
    /// # use ch10_lifetimes::Parser;
    /// let mut p = Parser::new(" \t\n rust  lifetimes \r\n ");
    /// assert_eq!(p.next_word(), Some("rust"));
    /// assert_eq!(p.next_word(), Some("lifetimes"));
    /// assert_eq!(p.next_word(), None);
    /// ```
    pub fn next_word(&mut self) -> Option<&'a str> {
        // 先把 `&'a str` 拷出来（`&str` 是 `Copy`），让后续切片与 `&mut self` 脱钩。
        let input = self.input;

        // 1) 跳过前导空白：在 `input[pos..]` 上找第一个非空白字符。
        //    用 `char_indices()` 拿到的是该字符的**字节偏移**，天然落在 char 边界上。
        let start = match input[self.pos..]
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
        {
            Some((offset, _)) => self.pos + offset,
            // 只剩空白或已到末尾：返回 None 且**不推进** pos。
            None => return None,
        };

        // 2) 从单词开头往后找第一个空白字符；找不到说明单词一直到输入末尾。
        let end = match input[start..]
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
        {
            Some((offset, _)) => start + offset,
            None => input.len(),
        };

        self.pos = end;
        Some(&input[start..end])
    }
}

/// 练习 10-3：返回切片里的第一个元素；空切片返回 `fallback`。
///
/// 两个来源——`items` 里的元素和 `fallback`——被绑定到同一个 `'a`，
/// 所以返回值一定能活到调用点。
///
/// 规则：
/// - `items` 非空 → 返回第一个元素的 `&str`（**哪怕它是空串**，也不该用 fallback）；
/// - `items` 为空 → 返回 `fallback`。
///
/// 考点：`&[String]` 的 `first()` 给出的是 `Option<&String>`，
/// 用 `.as_str()` 得到 `&str`；这个 `&str` 的生命周期来自 `items`，
/// 而 `unwrap_or(fallback)` 两个分支的类型必须统一成同一个 `'a`。
///
/// # 示例
/// ```
/// # use ch10_lifetimes::first_or;
/// let items = vec!["alpha".to_string(), "beta".to_string()];
/// assert_eq!(first_or(&items, "fallback"), "alpha");
/// assert_eq!(first_or(&[], "fallback"), "fallback");
/// ```
pub fn first_or<'a>(items: &'a [String], fallback: &'a str) -> &'a str {
    // `first()` 借的是 `items`，`unwrap_or` 借的是 `fallback`，
    // 签名的同一个 `'a` 把两个分支统一起来；`Some(空串)` 不会走 fallback。
    items.first().map(|s| s.as_str()).unwrap_or(fallback)
}
