# ch01_basics · 第 1 章 变量、类型与可变性

教程文档：[`docs/01-syntax/ch01-basics.md`](../../docs/01-syntax/ch01-basics.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch01_basics
```

| 练习 | 函数 | 考点 |
|---|---|---|
| 1-1 | `fahrenheit_to_celsius` | 浮点算术、整数除法陷阱 |
| 1-2 | `split_duration` | 整数除与取模、元组返回 |
| 1-3 | `format_score_line` | `as` 转换、`{:.1}` 格式化 |

改 [`src/lib.rs`](src/lib.rs) 里的三个 `unimplemented!()`。
卡住就跑 `python3 tools/ai_review.py ch01_basics --hint 1`。
