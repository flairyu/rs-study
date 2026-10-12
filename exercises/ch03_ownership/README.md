# ch03_ownership —— 第 3 章 · 所有权

这一章的三个练习都不难算，难的是**让编译器满意**。目标是把三件事变成本能：

1. `String` / `Vec<T>` 这类"拥有堆内存"的值，赋值和传参默认是**移动**（move），不是复制；
2. 想把值交给函数又要留着，只能 `clone`、重新构造，或者让它出现在返回值里带回来；
3. 作用域结束自动 `Drop`，所以既不用、也不能手写 `free` / `delete`。

配套讲义：`docs/01-syntax/ch03-ownership.md`（先读它，再动手）。

## 文件

| 文件 | 作用 | 能不能改 |
|---|---|---|
| `src/lib.rs` | 3 个函数的骨架，把 `unimplemented!()` 换成你的实现 | ✅ 只能改这个 |
| `tests/chapter.rs` | 自动评测（恰好 3 个测试） | ❌ 不要改，它是裁判 |
| `Cargo.toml` | 只依赖 `std` | ❌ 不要加任何依赖 |

## 运行

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch03_ownership
```

做题前先跑一次，看看「骨架必然编译通过、测试必然失败」长什么样。

## 三道题

| 题号 | 函数签名 | 要做什么 |
|---|---|---|
| 3-1 | `pub fn shout(s: String) -> String` | 返回 `s` 的全大写形式（新 `String`） |
| 3-2 | `pub fn join_and_count(words: Vec<String>) -> (usize, String)` | 消费 `Vec`，返回 `(单词个数, 空格连接结果)`；空 `Vec` → `(0, "")` |
| 3-3 | `pub fn consume_and_give_back(s: String) -> (String, usize)` | 返回 `(原字符串, 字节长度)` |

## 学到什么算过关

- 能解释 `let b = a;`（`a: String`）之后为什么 `a` 不能再用了；
- 知道 `let y = x;`（`x: i32`）为什么没有这个问题；
- 能说出 `"中文".len()` 返回 `6` 而不是 `2` 的原因；
- 知道为什么 `(s, s.len())` 编译不过，正确写法是什么；
- `python3 tools/check.py ch03_ownership` 全绿。
