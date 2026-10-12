# ch07_collections — 第 7 章 · 集合

配套文档：[`docs/01-syntax/ch07-collections.md`](../../docs/01-syntax/ch07-collections.md)

本章练的是标准库里最常用的三个集合：`Vec<T>`（变长数组）、
`HashMap<K, V>`（哈希表）、`HashSet<T>`（哈希集合），
以及字符串的三层关系 `&str` / `String` / `str`。

## 三题

| 题号 | 函数 | 考点 |
|---|---|---|
| 7-1 | `mean_and_median(nums: &[i32]) -> Option<(f64, f64)>` | 借用切片、`Option` 表达"空输入无意义"、偶数个元素的中位数、对**副本**排序不改入参 |
| 7-2 | `word_count(text: &str) -> std::collections::HashMap<String, usize>` | 大小写归一、`split_whitespace`、`entry(...).or_insert(0) += 1`、`String` 与 `&str` 的分工 |
| 7-3 | `dedup_preserve_order(items: &[String]) -> Vec<String>` | `HashSet` 记录已见、保序去重、`HashSet<&str>` 借用元素避免多余分配 |

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch07_collections
```

或者直接用 cargo：

```bash
cargo test -p ch07_collections
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch07_collections --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- 写的时候留意：`v[i]` 越界会 panic，`v.get(i)` 返回 `Option<&T>`；
  `HashMap` 的 key 需要 `Eq + Hash`，`f64` 两者都没有，所以 `word_count` 的 key 是 `String`。
