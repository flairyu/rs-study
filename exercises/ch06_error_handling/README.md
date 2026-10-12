# ch06_error_handling · 第 6 章 · 错误处理

配套文档：`docs/01-syntax/ch06-error-handling.md`

本章 3 个练习只依赖标准库。做法：把 `src/lib.rs` 里每个函数体的 `unimplemented!()`
换成你自己的实现，然后跑评测。

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch06_error_handling     # 等价于 cargo test -p ch06_error_handling
```

| 练习 | 函数 | 你要做的事 |
|---|---|---|
| 6-1 | `parse_age` | 把 `&str` 解析成 `Result<u8, String>`：空 / 非法数字 / 超过 150 各有各的错误 |
| 6-2 | `sum_numbers` | 逗号分隔整数求和，空输入算 `0`，用 `?` 传播解析错误 |
| 6-3 | `parse_port` | 解析 `u16` 端口号，错误用自定义枚举 `PortError` 表达 |

## 规则

- `tests/chapter.rs` 是裁判，**不要修改**：恰好 3 个 `#[test]`，分别是
  `ex6_1_parse_age` / `ex6_2_sum_numbers` / `ex6_3_parse_port`。
- 只允许依赖 `std`。不要往 `Cargo.toml` 里加任何依赖。
- 不使用 nightly 特性；edition 由 workspace 固定为 2021。
- 测试会用 `assert_eq!` 直接比较 `Err(...)`，所以错误字符串（含中文标点）必须与规格**逐字**一致。
- 卡住了跑：`python3 tools/ai_review.py ch06_error_handling --hint 2`

## 本章考点

- `Result<T, E>` 与 `Option<T>` 的取舍，以及 `.ok_or_else(...)` / `.ok()` 互转
- `?` 的两条语义，以及它只能用在返回 `Result` / `Option` 的函数里
- `map_err` + `?`：把底层错误换成自己的错误类型
- 自定义错误枚举，配 `#[derive(Debug, PartialEq, Eq)]`，方便测试直接比较
