//! # 第 7 章 · 集合
//!
//! 前面几章处理的值都是一次一个（`i32`、`String`、`Option<T>`）。
//! 本章进入「一次处理一批」的世界：`Vec<T>` 是变长数组，`HashMap<K, V>` 是哈希表，
//! `HashSet<T>` 是哈希集合。它们都住在**堆**上，所以「所有权」在它们身上体现得最明显：
//! 往里放值是把所有权**移动**进去，从里面取值是把所有权**搬出来**。
//!
//! 本章 3 个练习只用到标准库。做法：把每个函数体里的 `unimplemented!()`
//! 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch07_collections
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`mean_and_median`] | `&[i32]` 借用切片、`Option` 表示空输入、偶数中位数、对副本排序 |
//! | [`word_count`] | `&str` 分词、`to_lowercase`、`HashMap` 的 `entry` API |
//! | [`dedup_preserve_order`] | `HashSet` 去重、保序、`String` 的所有权与克隆 |
//!
//! 提示：先读 `docs/01-syntax/ch07-collections.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

use std::collections::{HashMap, HashSet};

/// 练习 7-1：同时算出平均值与中位数。
///
/// 输入是一段 `i32` 的**共享借用** `&[i32]`（切片），函数不拥有它、也不修改它。
///
/// 规则：
/// - `nums` 为空 → 返回 `None`（空集合没有平均值，也没有中位数）；
/// - 均值 = 所有元素之和 / 元素个数，结果类型是 `f64`；
/// - 中位数：把元素**从小到大排序**后，奇数个取正中间那个；
///   偶数个取中间两个的平均值；
/// - 返回 `Some((均值, 中位数))`。
///
/// 考点：`nums` 是 `&[i32]` 而不是 `Vec<i32>`，所以你**不能**直接 `sort()` 它——
/// 那是 `&mut` 方法，而你现在只有共享借用。正确做法是先克隆一份：
/// `let mut sorted = nums.to_vec(); sorted.sort_unstable();`，排序副本、不动入参。
/// 另外注意均值里必须写 `as f64`，否则整数除法会把 3.5 变成 3。
///
/// # 示例
/// ```
/// # use ch07_collections::mean_and_median;
/// assert_eq!(mean_and_median(&[]), None);
/// let (mean, median) = mean_and_median(&[1, 2, 3, 4]).unwrap();
/// assert!((mean - 2.5).abs() < 1e-9);
/// assert!((median - 2.5).abs() < 1e-9);
/// ```
pub fn mean_and_median(nums: &[i32]) -> Option<(f64, f64)> {
    // 空集合既没有均值也没有中位数。
    if nums.is_empty() {
        return None;
    }
    // 用 i64 累加，避免元素多时 i32 溢出；最后再转 f64 做除法。
    let sum: i64 = nums.iter().map(|&x| x as i64).sum();
    let mean = sum as f64 / nums.len() as f64;
    // nums 是共享借用，不能直接 sort()——先 to_vec() 拷一份再排，入参保持不变。
    let mut sorted = nums.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    let median = if sorted.len() % 2 == 1 {
        sorted[mid] as f64
    } else {
        // 偶数个：中间两数的平均值；先转 f64，避免整数除法截断。
        (sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0
    };
    Some((mean, median))
}

/// 练习 7-2：统计词频。
///
/// 规则：
/// - 先把整段文本转成**小写**（`to_lowercase`），所以 `"The"` 和 `"the"` 算同一个词；
/// - 再按空白分词（空格、制表符、换行都算；`split_whitespace` 会自动跳过连续空白）；
/// - 返回 `词 -> 出现次数` 的 `HashMap`。
///
/// 边界：
/// - `""` 或全空白 → 返回**空 map**（不是 `None`）；
/// - 空串本身不是词，不进 map；
/// - 中文按 Unicode 小写转换处理，`"你好 你好"` 里 `"你好"` 出现 2 次。
///
/// 考点：`HashMap` 的 key 是 `String`（拥有所有权的），因为 `split_whitespace`
/// 给你的 `&str` 只是借用了 `text` 的片段，而返回的 map 要比 `text` 活得更久。
/// 计数用 `entry` API，一次查找完成「有就加一、没有就插入 0 再加一」：
/// `*counts.entry(word.to_string()).or_insert(0) += 1;`
///
/// # 示例
/// ```
/// # use ch07_collections::word_count;
/// let counts = word_count("Hello hello WORLD");
/// assert_eq!(counts.get("hello"), Some(&2));
/// assert_eq!(counts.get("world"), Some(&1));
/// assert!(word_count("   ").is_empty());
/// ```
pub fn word_count(text: &str) -> std::collections::HashMap<String, usize> {
    // 先转小写得到一个新 String，split_whitespace 借用的就是它（存活到函数结束）。
    let lowered = text.to_lowercase();
    let mut counts = HashMap::new();
    for word in lowered.split_whitespace() {
        // key 必须拥有所有权：word 只是 lowered 的片段，而 counts 要返回出去。
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
    counts
}

/// 练习 7-3：去重，但保留元素**首次出现**的顺序。
///
/// 规则：
/// - 结果里每个值只出现一次，顺序 = 它在 `items` 里第一次出现的顺序；
/// - `items` 为空 → 返回空 `Vec`；
/// - 输入里的重复项（包括不相邻的重复项）只保留第一个。
///
/// 例如 `["b", "a", "b", "c", "a"]` → `["b", "a", "c"]`。
///
/// 考点：`HashSet` 负责「这个值见过没有」的 O(1) 判断，`Vec` 负责记住顺序——
/// 单靠 `HashSet` 会丢掉顺序，单靠 `Vec` 会退化成 O(n²) 的双重循环。
///
/// 关于所有权：入参是 `&[String]`，返回的是 `Vec<String>`（拥有所有权的新集合），
/// 所以每个保留下的元素都要 `.clone()` 一份。
/// 一个小技巧是让 `HashSet` 借用元素本身（`HashSet<&str>`），
/// 这样集合里记录的是「已见」的借用，只有结果 Vec 需要真正克隆。
///
/// # 示例
/// ```
/// # use ch07_collections::dedup_preserve_order;
/// let items = vec!["b".to_string(), "a".to_string(), "b".to_string()];
/// assert_eq!(dedup_preserve_order(&items), vec!["b".to_string(), "a".to_string()]);
/// assert!(dedup_preserve_order(&[]).is_empty());
/// ```
pub fn dedup_preserve_order(items: &[String]) -> Vec<String> {
    // seen 借用 items 里的 &str（生命周期够用），只有结果才需要真正克隆。
    let mut seen: HashSet<&str> = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        // insert 返回 true 表示「第一次见到」，这时才收集，顺序即首次出现顺序。
        if seen.insert(item.as_str()) {
            result.push(item.clone());
        }
    }
    result
}
