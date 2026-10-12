# ch13_async · 第 13 章 async / await 与 Future

教程文档：[`docs/01-syntax/ch13-async.md`](../../docs/01-syntax/ch13-async.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch13_async
```

| 练习 | 函数 | 考点 |
|---|---|---|
| 13-1 | `async_add` | `async fn` 调用后返回 Future；用 `block_on` 取结果 |
| 13-2 | `async_sum` | 切片借用跨过 `.await`；`async fn` 之间互相 `.await` |
| 13-3 | `async_count_down` | `YieldNow` 制造 `Poll::Pending`；`.await` 暂停后恢复 |

改 [`src/lib.rs`](src/lib.rs) 里的三个 `unimplemented!()`。
**不要动** [`src/lib.rs`](src/lib.rs) 顶部的 `block_on` 与 `YieldNow`——它们是给你用的工具。
卡住就跑 `python3 tools/ai_review.py ch13_async --hint 1`。
