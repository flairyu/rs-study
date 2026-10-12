# ch04_borrowing — 第 4 章 · 引用与借用

配套文档：[`docs/01-syntax/ch04-borrowing.md`](../../docs/01-syntax/ch04-borrowing.md)

本章练的是「借出去，而不是送出去」：用 `&T` 共享读取、用 `&mut T` 独占修改、
用切片 `&[T]` / `&str` 零拷贝地引用别人拥有的一段数据。

## 三题

| 题号 | 函数 | 考点 |
|---|---|---|
| 4-1 | `longest_word(text: &str) -> &str` | `&str` 进 `&str` 出、返回切片绑定输入生命周期、`split_whitespace` |
| 4-2 | `append_exclaim(s: &mut String)` | `&mut` 可变借用、`push` 原地修改 |
| 4-3 | `halves(nums: &[i32]) -> (&[i32], &[i32])` | 切片借用、`split_at`、两个共享借用可以同时存在 |

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch04_borrowing
```

或者直接用 cargo：

```bash
cargo test -p ch04_borrowing
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch04_borrowing --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- 写的时候留意：不要在同一个作用域里同时持有某个值的 `&T` 和 `&mut T`，那是本章要讲的编译错误。
