# ch08_iterators_closures — 第 8 章 · 闭包与迭代器

配套文档：[`docs/01-syntax/ch08-iterators-closures.md`](../../docs/01-syntax/ch08-iterators-closures.md)

本章练的是 Rust 里最常用的两件事：**闭包**（能捕获环境的匿名函数）和
**迭代器链**（`iter().filter(..).map(..).sum()` 这种写法）。
学会它们之后，你会发现大部分手写 `for` 循环都可以删掉，而且性能不会变差——
迭代器适配器会被编译成和内联循环几乎一样的机器码。

## 三题

| 题号 | 函数 | 考点 |
|---|---|---|
| 8-1 | `sum_of_squares_of_evens(nums: &[i32]) -> i64` | `filter` + `map` + `sum` 适配器链、`&i32` 的模式匹配、先转 `i64` 再平方避免溢出 |
| 8-2 | `apply_n_times<F: Fn(i32) -> i32>(f: F, x: i32, n: u32) -> i32` | 泛型闭包参数、`Fn` 可以被多次调用、把函数当参数传 |
| 8-3 | `count_by_length(words: &[&str]) -> std::collections::BTreeMap<usize, usize>` | 迭代器 + `entry` API 统计频次、`BTreeMap` 的 key 自动升序 |

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch08_iterators_closures
```

或者直接用 cargo：

```bash
cargo test -p ch08_iterators_closures
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch08_iterators_closures --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值
  （空切片、全奇数、`n = 0`、负数平方、空字符串长度、重复长度）。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- 写的时候留意：`nums.iter()` 产出的是 `&i32` 而不是 `i32`，闭包参数要么写成 `|&&x|`
  用模式解构，要么先 `.copied()`；平方前先 `as i64`，否则 `i32` 的平方会溢出。
