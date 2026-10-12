# solutions —— 参考答案（同时是仓库自检）

这里的每个 crate 是 `exercises/` 对应章节的**参考实现**。它存在的意义有两个：

1. **给学习者对照**（但请先自己写，再看教程文档 `<details>` 里的答案）。
2. **证明练习是可通过的**：`tests/chapter.rs` 通过 `#[path]` 复用练习工程里的那一份，
   所以测试只有一份真相源，参考答案与题目不可能漂移。

```bash
cd solutions
cargo test --workspace        # 全部参考实现 + 文档测试
cargo test -p ch03_ownership  # 单章
```

> ⚠️ 如果某个参考实现跑不通，说明**题目本身的期望值有问题**，要去改
> `exercises/<crate>/tests/chapter.rs`，而不是在这里迁就。
