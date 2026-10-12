# ch15_testing_unsafe · 第 15 章 测试、unsafe 与 FFI

教程文档：[`docs/01-syntax/ch15-testing-unsafe.md`](../../docs/01-syntax/ch15-testing-unsafe.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch15_testing_unsafe
```

| 练习 | 函数 | 考点 |
|---|---|---|
| 15-1 | `unsafe fn read_raw` | 解引用裸指针；`unsafe fn` 的调用方也要写 `unsafe` |
| 15-2 | `abs_via_ffi` | `extern "C"` 声明并调用 C 运行库的 `abs` |
| 15-3 | `c_strlen` | `CString` + `strlen`；内部 `\0` 返回 0；`strlen` 数的是字节 |

改 [`src/lib.rs`](src/lib.rs) 里的三个 `unimplemented!()`。
本 crate 只依赖 `std`（`abs` / `strlen` 是宿主 C 运行库自带的，不需要任何第三方 crate）。
卡住就跑 `python3 tools/ai_review.py ch15_testing_unsafe --hint 1`。
