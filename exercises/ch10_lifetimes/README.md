# ch10_lifetimes — 第 10 章 · 生命周期

配套文档：[`docs/01-syntax/ch10-lifetimes.md`](../../docs/01-syntax/ch10-lifetimes.md)

第 4 章借用了别人的数据，第 10 章回答「编译器怎么知道这份数据还活着」：
生命周期标注是一句**承诺**——「我返回的引用，保证不比输入活得更久」，
它不能让任何变量活得更久，只是把输入和输出的寿命绑在一起交给借用检查器验证。

## 三题

| 题号 | 函数 | 考点 |
|---|---|---|
| 10-1 | `longest<'a>(a: &'a str, b: &'a str) -> &'a str` | 显式 `'a`、两个输入共享同一生命周期、等长取第一个 |
| 10-2 | `struct Parser<'a>` + `rest` / `next_word` | 结构体持有引用必须标注、`&self` 返回 `&'a str`、用 `char_indices()` 定位边界 |
| 10-3 | `first_or<'a>(items: &'a [String], fallback: &'a str) -> &'a str` | 返回值生命周期来自切片元素、`Option<&String>` → `&str` |

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**——
   本章的签名（尤其 `'a` 写在哪里）就是练习内容本身。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch10_lifetimes
```

或者直接用 cargo：

```bash
cargo test -p ch10_lifetimes
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch10_lifetimes --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- `next_word` 的切片必须落在 `char` 边界上：用 `input[pos..].char_indices()` 拿到的
  字节偏移去切，不要自己按固定的字节数推进 `pos`。
