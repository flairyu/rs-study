# ch14_modules_macros · 第 14 章 模块、crate 与宏

教程文档：[`docs/01-syntax/ch14-modules-macros.md`](../../docs/01-syntax/ch14-modules-macros.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch14_modules_macros
```

| 练习 | 目标 | 考点 |
|---|---|---|
| 14-1 | `geometry::Point` 的两个方法 + `pub use` 重导出 | `mod` 树、`pub` 可见性、重导出 |
| 14-2 | `hashmap_of!` 宏 | `macro_rules!`、`$k:expr`、`$(...),+`、尾逗号 `$(,)?` |
| 14-3 | `Temperature` 的 `Display` 与 `render` | 实现 std trait、`ToString` blanket impl |

改 [`src/lib.rs`](src/lib.rs) 里的 `unimplemented!()`（第 14-2 题连宏体也算）。
卡住就跑 `python3 tools/ai_review.py ch14_modules_macros --hint 1`。
