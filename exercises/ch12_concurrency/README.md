# ch12_concurrency · 第 12 章 并发

教程文档：[`docs/01-syntax/ch12-concurrency.md`](../../docs/01-syntax/ch12-concurrency.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch12_concurrency
```

| 练习 | 函数 | 考点 |
|---|---|---|
| 12-1 | `parallel_sum` | `thread::scope`、切片分片、空输入早返回 |
| 12-2 | `counter_with_mutex` | `Arc<Mutex<usize>>`、锁的作用域、`threads = 0` |
| 12-3 | `producer_consumer` | `mpsc::channel`、drop sender 关闭通道、收完排序 |

改 [`src/lib.rs`](src/lib.rs) 里的三个 `unimplemented!()`。只允许用标准库，不要加依赖。
卡住就跑 `python3 tools/ai_review.py ch12_concurrency --hint 1`。

> 评测里**不用 `sleep` 做同步**：线程之间只靠 `join` 和通道关闭语义对齐。
> 你的实现也应该这样——能用 `join` / `for v in rx` 等到的事，永远不要靠 `sleep` 猜。
