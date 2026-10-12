# ch02_control_flow · 第 2 章 · 函数与控制流

本 crate 是《第 2 章 · 函数与控制流》的自动评测工程，**只依赖标准库**，可以完全离线编译、测试。

## 你要做什么

打开 `src/lib.rs`，把 3 个函数体里的 `unimplemented!()`
换成你自己的实现（`unimplemented!()` 的类型是 `!`，所以骨架永远能编译通过，
你看到的失败一定是断言失败，而不是编译错误）：

| 函数 | 语义 | 考点 |
|---|---|---|
| `fizzbuzz(n: u32) -> String` | 被 3 和 5 同时整除 → `"FizzBuzz"`；只被 3 整除 → `"Fizz"`；只被 5 整除 → `"Buzz"`；否则返回 `n` 的十进制字符串 | `if` 是表达式、取模、`String` vs `&str` |
| `collatz_steps(n: u64) -> u32` | 偶数除 2、奇数乘 3 加 1，返回到达 1 的步数；`n == 0` 和 `n == 1` 都返回 `0` | `loop` + `break` 带值、提前返回、边界值 |
| `membership_level(years: u32, points: u32) -> &'static str` | 按顺序判断：`years >= 5 \|\| points >= 1000` → `"钻石"`；`years >= 3 \|\| points >= 500` → `"黄金"`；`years >= 1 \|\| points >= 100` → `"白银"`；其余 → `"青铜"` | `match (years, points)` + `if` 守卫、分支顺序 |

## 运行评测

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch02_control_flow
```

等价于直接跑：

```bash
cargo test -p ch02_control_flow
```

`tests/chapter.rs` 是裁判（**不要修改它**）：它恰好有 3 个 `#[test]`，每个测试内部
都覆盖了正常值和边界值（`0`、`1`、`u32::MAX` / 接近极大的 `u64`）。

## 卡住了

先读 `docs/01-syntax/ch02-control-flow.md` 的对应小节；还想不出来就让 AI 只给提示：

```bash
python3 tools/ai_review.py ch02_control_flow --hint 2
```

## 建议顺序

1. 读文档的「0. 先建立锚点」和你要做的那道题对应的小节；
2. 自己写实现，跑评测，红了再回来读「常见坑」；
3. 全部通过后做文档最后的「自测清单」。
