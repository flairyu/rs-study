//! # 第 8 章 · 闭包与迭代器
//!
//! 本章 3 个练习只用到标准库。做法：把每个函数体里的 `unimplemented!()`
//! 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch08_iterators_closures
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`sum_of_squares_of_evens`] | `filter` + `map` + `sum` 迭代器链、`&i32` 的解构、先转 `i64` 再平方 |
//! | [`apply_n_times`] | 泛型参数 `F: Fn(i32) -> i32`、闭包作为参数、`Fn` 可重复调用 |
//! | [`count_by_length`] | 迭代器 + `BTreeMap` 的 `entry` API、按 key 升序遍历 |
//!
//! 提示：先读 `docs/01-syntax/ch08-iterators-closures.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 8-1：用迭代器链计算切片中**所有偶数**的平方和。
///
/// 输入是 `&[i32]`，输出是 `i64`。
/// 空切片、全奇数都返回 `0`；负数如果是偶数也要算（平方后是正数）。
///
/// 返回 `i64` 是刻意的：`i32` 的平方很容易溢出（`46341^2 > i32::MAX`），
/// 所以要在乘法**之前**把每个元素转成 `i64`，而不是最后才把和转成 `i64`。
///
/// 考点：`iter()` 产出的是 `&i32`，闭包参数要写成 `|&&x|` 用模式解构，
/// 或者先 `.copied()` 把 `&i32` 变成 `i32`。
///
/// # 示例
/// ```
/// # use ch08_iterators_closures::sum_of_squares_of_evens;
/// assert_eq!(sum_of_squares_of_evens(&[1, 2, 3, 4, -5]), 20); // 只有 2 和 4 是偶数：4 + 16
/// assert_eq!(sum_of_squares_of_evens(&[]), 0);
/// ```
pub fn sum_of_squares_of_evens(nums: &[i32]) -> i64 {
    unimplemented!("练习 8-1：把偶数挑出来、平方、求和（返回 i64）")
}

/// 练习 8-2：把闭包 `f` 在 `x` 上迭代应用 `n` 次。
///
/// `n == 0` 时返回原值 `x`（一次也不调用）。
/// `f` 的约束是 [`Fn`] 而不是 `FnOnce`：因为它会被调用多次。
/// 函数项（普通函数名、非捕获闭包）也满足 `Fn`，可以直接传进来。
///
/// 实现用 `for` 循环即可，注意让累加变量可变。
///
/// # 示例
/// ```
/// # use ch08_iterators_closures::apply_n_times;
/// assert_eq!(apply_n_times(|v| v + 1, 0, 5), 5);   // 加 1 加 5 次
/// assert_eq!(apply_n_times(|v| v * 2, 3, 0), 3);   // n = 0：原样返回
/// ```
pub fn apply_n_times<F: Fn(i32) -> i32>(f: F, x: i32, n: u32) -> i32 {
    unimplemented!("练习 8-2：用 for 循环把 f 应用 n 次，n = 0 时返回 x")
}

/// 练习 8-3：统计每个「单词长度」出现了多少次。
///
/// key 是长度（`usize`），value 是具有该长度的单词个数。
/// 空输入或全是空字符串时返回空 map。
/// 返回 `BTreeMap` 而不是 `HashMap`，是为了让 key 天然按升序排列，方便对比和打印。
///
/// 提示：`map` 成 `(长度, 1)` 再 `collect`，或者用 `entry(len).or_insert(0)` 累加；
/// 空字符串的长度是 `0`，也要被统计。
///
/// # 示例
/// ```
/// # use ch08_iterators_closures::count_by_length;
/// # use std::collections::BTreeMap;
/// let mut expected = BTreeMap::new();
/// expected.insert(3, 2);
/// expected.insert(4, 1);
/// assert_eq!(count_by_length(&["abc", "def", "ghij"]), expected);
/// assert_eq!(count_by_length(&[]), BTreeMap::new());
/// ```
pub fn count_by_length(words: &[&str]) -> std::collections::BTreeMap<usize, usize> {
    unimplemented!("练习 8-3：用 BTreeMap 统计每种单词长度出现的次数")
}
