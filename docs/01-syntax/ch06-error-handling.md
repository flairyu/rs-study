# 第 6 章 · 错误处理

> **一句话定位**：把「可能失败」写进函数签名——`Result<T, E>` 让错误成为类型的一部分，`?` 让传播只花一个字符。
> **预计时间**：90 分钟（读 45 + 写 45）
> **前置**：第 5 章（枚举、`Option<T>`、`match`）
> **练习工程**：`exercises/ch06_error_handling`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一件事——把字符串解析成 0..=150 的年龄——Rust 是这么写的：

```rust
fn parse_age(s: &str) -> Result<u8, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("年龄不能为空".to_string());
    }
    let age: u8 = t.parse().map_err(|_| format!("不是合法数字: {s}"))?;
    if age > 150 {
        return Err("年龄超出范围".to_string());
    }
    Ok(age)
}
```

C 的版本会返回 `-1` 并把细节塞进全局 `errno`；Python 的版本会 `raise ValueError`，
调用方用 `try/except` 兜住，不写 `try` 就自动向上冒泡。对照着看：

| 话题 | C（返回码 + `errno`） | Python（`try/except`） | Rust（`Result` + `?`） |
|---|---|---|---|
| 失败写在哪 | 返回值 + 全局 `errno` | 异常对象 | 返回类型 `Result<T, E>` |
| 忘了处理 | 静默继续（最危险） | 异常继续冒泡，最后崩 | 编译期警告（`#[must_use]`），且取不到值 |
| 向上传播 | `if (ret < 0) return ret;` | 不写 `try` 就自动传 | `?` 一个字符 |
| 错误内容 | `errno` 整数，会被下次调用覆盖 | 异常对象 + traceback | 任意 `E`：`String` / 自定义枚举 / `Box<dyn Error>` |
| 崩溃 | 段错误、未定义行为 | 未捕获异常 | `panic!`，带消息、位置、可选 backtrace |

一句话：**C 的错误是约定，Python 的错误是控制流，Rust 的错误是类型**——
编译器知道它存在，你就没法假装它不存在。

## 1. `panic!` 和 `Result` 的边界

- **可恢复错误**：失败是预期内的。用户输入了 `abc`、文件不存在、端口号超范围。
  调用者有能力也有责任决定怎么办（重试 / 提示 / 用默认值）。→ 用 `Result<T, E>`。
- **不可恢复错误**：程序有 bug，或某个前提被破坏。索引越界、不可能走到的分支、
  内部不变量被违反，继续跑只会更糟。→ 用 `panic!`。

> **可恢复错误用 `Result`，不可恢复的 bug 用 `panic!`。**
> 判断标准只有一句：**这是调用者能合理处理的情况吗？**

```rust
// panic!：月份非法说明调用者写错了代码，没有合理的值可以返回
fn days_in_month(month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28,
        other => panic!("月份必须是 1..=12，收到 {other}"),
    }
}
```

同样的逻辑，如果 `month` 来自用户输入，就该返回 `Result<u32, String>`，
把非法分支写成 `Err(format!("月份必须是 1..=12，收到 {other}"))`。

| 场景 | 选择 |
|---|---|
| 解析用户输入、读文件、网络请求、解析配置 | `Result` |
| 库的公开 API 收到非法参数 | `Result`（别替调用者崩进程） |
| 内部不变量被破坏、`assert!`、不可能走到的分支 | `panic!` |
| 测试、一次性脚本、原型 | `panic!` / `unwrap()` 都行 |
| 给 C 调用的 `extern "C"` 函数 | 错误码（panic 不能穿出去） |

`panic!` 的细节：默认 **unwind**——栈会展开、`Drop` 会执行，资源正常释放（比 C 的 `abort()` 舒服）；
profile 里写 `panic = "abort"` 可以改成直接终止。`std::panic::catch_unwind` 只给 FFI 边界和
测试框架用，**不要当 `try/except` 使**：它要求闭包 `UnwindSafe`，而且 panic 之后程序状态
可能已经不一致了。想看栈回溯就设 `RUST_BACKTRACE=1`。

## 2. `Result<T, E>` 还是 `Option<T>`？

`Option<T>` 只回答「有没有」，`Result<T, E>` 还回答「为什么没有」。
当「为什么」对调用者有用（要打日志、要重试、要提示用户）时，就必须用 `Result`。

| 场景 | 用哪个 |
|---|---|
| 缓存未命中、`HashMap` 查不到、`find` 没找到 | `Option<T>` |
| 解析失败、I/O 失败、校验不通过 | `Result<T, E>` |
| 调用者只需要知道「成没成」 | `Option<T>` |
| 调用者需要区分失败原因 | `Result<T, E>` |

互转各一个方法：

```rust
fn find_user(id: u32) -> Option<String> {
    if id == 1 { Some("alice".to_string()) } else { None }
}

fn main() {
    // Option -> Result：把「没有」升级成带原因的错误
    assert_eq!(find_user(1).ok_or("用户不存在"), Ok("alice".to_string()));
    assert_eq!(find_user(2).ok_or("用户不存在"), Err("用户不存在"));
}
```

`.ok_or(err)` 会**立即**求值 `err`，`.ok_or_else(|| ...)` 只在需要时报错，所以更常用；
反方向用 `.ok()`：`"42".parse::<u32>().ok()` 是 `Some(42)`，失败时给 `None`。

## 3. `unwrap` / `expect` 什么时候可以用

`unwrap()` = 「成功就取值，失败就 panic」；`expect("...")` 一样，只是 panic 消息里带上你的说明：

```rust
let port: u16 = "8080".parse().unwrap();                 // 失败时 panic: called `Result::unwrap()` on an `Err` value: ...
let port: u16 = "8080".parse().expect("端口配置写错了");   // 失败时 panic: 端口配置写错了: ParseIntError { ... }
```

**可以用**：测试代码（失败本来就该炸）；示例 / 原型 / `main` 里的一次性脚本；
你已经**证明**不可能失败、并把理由写进 `expect` 消息的地方。

**不要用**：库的公开 API（等于把用户乱填的输入变成你的进程崩溃）；任何处理外部输入的地方
（命令行参数、HTTP body、配置文件）；热路径（上层若用 `catch_unwind` 兜住，panic 就成了拒绝服务）。
不想 panic 又想要默认值，用 `unwrap_or(0)` / `unwrap_or_default()` / `unwrap_or_else(|e| ...)`
（最后一个是惰性的，只在失败时执行闭包）。

## 4. `?` 运算符

`?` 的语义只有两条：

1. `Ok(v)` → 取出 `v`，继续往下走；
2. `Err(e)` → **提前 return**，而且是 `return Err(From::from(e))`。

展开后大致是（`x?` 就是这段的语法糖）：

```rust
fn parse_number(s: &str) -> Result<u32, std::num::ParseIntError> {
    let n: u32 = match s.trim().parse() {
        Ok(v) => v,
        Err(e) => return Err(std::convert::From::from(e)),   // 注意这个 From::from
    };
    Ok(n)
}
```

第 2 条里的 `From::from` 就是 `?` 能同时处理多种错误类型的原因（见第 5 节）。

### 4.1 铁律一：`?` 只能用在返回 `Result` / `Option` 的函数里

```rust
fn bad() -> i32 {
    let n: i32 = "42".parse()?;   // ❌ E0277：`?` 只能用在返回 Result 或 Option 的函数里
    n
}
```

`fn main()` 返回 `()`，所以也不能直接写 `?`；但 `main` 可以改成返回 `Result`：

```rust
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let n: i64 = " 42 ".trim().parse()?;   // 返回 Err 时打印 `Error: ...`，退出码 1
    println!("n = {n}");                    // 输出: n = 42
    Ok(())
}
```

### 4.2 铁律二：错误类型必须能转成函数的错误类型

类型相同自然可以；不同就要实现 `From`（第 5 节），或者用 `map_err` 手动换——
练习 6-1 / 6-2 用的就是这招：`t.parse().map_err(|_| format!("不是合法数字: {s}"))?`。

### 4.3 铁律三：`Option` 上的 `?` 提前返回 `None`

```rust
fn first_word_len(text: &str) -> Option<usize> {
    let word = text.split_whitespace().next()?;   // 没有词就 return None
    Some(word.len())
}
```

两种 `?` 不能混用（提前返回的东西不一样）；在返回 `Result` 的函数里对 `Option` 用 `?` 编译不过，
必须先转：

```rust
fn fixed(text: &str) -> Result<usize, String> {
    let word = text.split_whitespace().next().ok_or("输入为空")?;   // Option -> Result
    Ok(word.len())
}
```

## 5. `From` 驱动的错误转换

`?` 的第二条语义是 `return Err(From::from(e))`。只要你的错误类型实现了 `From<底层错误>`，
同一个函数里就能用 `?` 混合多种底层错误：

```rust
use std::num::ParseIntError;

#[derive(Debug, PartialEq, Eq)]
enum MyError {
    Parse(ParseIntError),
    TooLarge(u32),
}

// 有了这个 impl，`?` 会自动把 ParseIntError 转成 MyError
impl From<ParseIntError> for MyError {
    fn from(e: ParseIntError) -> Self {
        MyError::Parse(e)
    }
}

fn double_checked(s: &str) -> Result<u32, MyError> {
    let n: u32 = s.trim().parse()?;        // 内部做了 MyError::from(e)
    if n > 1000 {
        return Err(MyError::TooLarge(n));
    }
    Ok(n * 2)
}
```

为什么值得这么麻烦？因为**错误类型保住了信息**：调用者可以 `match MyError::Parse(_)`
决定「重试」还是「提示用户改输入」。另外 `impl From<X> for Y` 要求 `X` 或 `Y` 至少有一个是你
crate 里的类型（孤儿规则，第 9 章）——所以 `String` 当错误类型时只能逐处 `map_err`。

## 6. 自定义错误枚举：把「为什么失败」写进类型

`String` 当错误类型省事，但调用者只能打印它，**没法 match**。库 / 模块的公开 API 应该用枚举，
让调用者能区分「空输入」「不是数字」「超范围」并分别处理。练习 6-3 的 `PortError` 就是最小版本：

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum PortError {
    Empty,
    NotANumber(String),
    TooLarge(u32),
}
```

`Debug` 让 `assert_eq!` 失败时能打印（`unwrap()` 也要求它）；`PartialEq` / `Eq` 让测试能直接写
`assert_eq!(parse_port("65536"), Err(PortError::TooLarge(65536)))`，也方便以后把它当 `HashMap`
的 key（那还需要 `Hash`）。要让它在 `eprintln!("{e}")` 和 `Box<dyn Error>` 里都能用，还得手写
`std::fmt::Display` 并 `impl std::error::Error for PortError {}`（`Error` 的 supertrait 是
`Debug + Display`）；这个模式很啰嗦，第 17 章的 `thiserror` 就是给它做自动化的。

## 7. `Box<dyn Error>`：应用层的「万能错误」

在 `main`、脚本、CLI 入口这类地方，底层错误来自 I/O、解析、环境变量……每种都写枚举分支太累。
`Box<dyn Error>` 能一把装下：标准库为任何 `E: std::error::Error + 'static` 提供了
`impl From<E> for Box<dyn Error>`，所以 `?` 会自动装箱。

```rust
use std::error::Error;
use std::fs;

fn count_lines(path: &str) -> Result<usize, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;   // io::Error -> Box<dyn Error>
    Ok(text.lines().count())
}
```

代价：**类型信息丢了**（调用者只能用 `.downcast_ref::<T>()` 猜回去，不能 `match`）；
**不能比较**（没实现 `PartialEq`，测试里写不了 `assert_eq!`，所以练习 6-3 用枚举）；
**跨线程 / async 要加约束**（得写 `Box<dyn Error + Send + Sync>`）；
**库的公开 API 不要返回它**（把错误类型的选择权留给了别人）。
它默认用 `Debug` 打印——`main` 返回 `Err` 时输出的 `Error: ...` 就是 `{:?}`。

## 8. 常见坑

**坑 1：`?` 用在返回 `()` 的函数里。** 比如在 `fn main()` 里直接写 `let n: i64 = "42".parse()?;`
会报 E0277。改法：`fn main() -> Result<(), Box<dyn Error>> { ... Ok(()) }`，或显式 `match` / `unwrap`。

**坑 2：忘了把成功值包回 `Ok`。**

```rust
fn double(s: &str) -> Result<u8, String> {
    let n: u8 = s.trim().parse().map_err(|_| "不是数字".to_string())?;
    n * 2            // ❌ expected `Result<u8, String>`, found `u8`；应写 Ok(n * 2)
}
```

顺带一提：给 `Ok(n * 2)` 加上分号也一样错——函数体最后一个表达式不能带分号。

**坑 3：范围校验放在了「解析成功」之前。** 练习 6-3 的经典错误是用
`let v: u16 = t.parse()` 再判断 `v > 65535`：`"65536"` 在解析阶段就失败了，
`TooLarge` 永远构造不出来，测试会告诉你它拿到的是 `NotANumber("65536")`。
**先按足够宽的类型解析，再做范围校验。**

**坑 4：在外部输入上 `unwrap()`。** `std::env::args().nth(1).unwrap().parse().unwrap()`：
用户不传参数就 panic，传 `abc` 也 panic——这不是「快速失败」，是把输入校验推给了崩溃。

**坑 5：让 panic 穿出 FFI 边界，或者丢掉错误信息。** 给 C 用的函数一旦 panic，进程会 `abort`，
C 侧完全无法处理，所以这种函数必须返回错误码（第 15 章）。另外
`Err("invalid input".to_string())` 对排查毫无帮助，`format!("不是合法数字: {s}")` 才有用。

## 9. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 6-1 | 解析年龄：空 / 非法 / 超范围各给不同错误 | 先 `trim`、先解析后校验 | `parse_age` |
| 6-2 | 逗号分隔整数求和，空输入算 0 | `?` + `map_err`、逐段 `trim` | `sum_numbers` |
| 6-3 | 解析端口号 0..=65535 | 自定义错误枚举、先按 `u32` 解析 | `parse_port` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch06_error_handling      # 做题前先跑一次，看它怎么失败
python3 tools/ai_review.py ch06_error_handling --hint 2   # 卡住了再跑
```

### 提示（不给答案）

- **6-1**：先把 `s.trim()` 存成一个变量（比如 `t`），判空和解析都用它；
  `> 150` 必须在解析**成功之后**；错误消息里的 `{s}` 用原始输入。
- **6-2**：判空放在 `split(',')` **之前**（`"   "` 要先被 `trim` 成空串）；
  每段 `trim` 后写 `seg.parse().map_err(|_| format!("不是合法数字: {seg}"))?`。
- **6-3**：类型注解必须写 `let v: u32 = ...`。用 `u16` 解析的话 `"65536"` 会变成
  `NotANumber`，`TooLarge` 就白定义了。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 6-1：trim 一次存起来；先解析成功，再判断 > 150。错误消息里带上原始输入 s。
pub fn parse_age(s: &str) -> Result<u8, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("年龄不能为空".to_string());
    }
    let age: u8 = t.parse().map_err(|_| format!("不是合法数字: {s}"))?;
    if age > 150 {
        return Err("年龄超出范围".to_string());
    }
    Ok(age)
}

// 6-2：整体为空先返回 Ok(0)，再逐段 trim + 解析；? 让第一次失败就直接返回。
pub fn sum_numbers(input: &str) -> Result<i64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    let mut total: i64 = 0;
    for seg in trimmed.split(',') {
        let seg = seg.trim();
        let n: i64 = seg.parse().map_err(|_| format!("不是合法数字: {seg}"))?;
        total += n;
    }
    Ok(total)
}

// 6-3：先按 u32 解析（宽到能装下 65536），再自己判断上界。
#[derive(Debug, PartialEq, Eq)]
pub enum PortError {
    Empty,
    NotANumber(String),
    TooLarge(u32),
}

pub fn parse_port(s: &str) -> Result<u16, PortError> {
    let t = s.trim();
    if t.is_empty() {
        return Err(PortError::Empty);
    }
    let v: u32 = t.parse().map_err(|_| PortError::NotANumber(s.to_string()))?;
    if v > 65535 {
        return Err(PortError::TooLarge(v));
    }
    Ok(v as u16)
}
```

- 6-1 的 `age > 150` 必须在 `parse` 之后：解析失败时根本没有 `age` 可比较。
- 6-2 里 `let seg = seg.trim();` 遮蔽了原值，所以 `"1, 2 ,x"` 报的是 `不是合法数字: x`；
  6-3 的 `v as u16` 安全（前面已保证 `v <= 65535`），先 `as` 再判断会把 `70000` 截成 `4464`。

</details>

---

## 自测清单

- [ ] 我能说出「什么时候用 `Result`、什么时候用 `panic!`」的判断标准
- [ ] 我知道 `Result<T, E>` 和 `Option<T>` 各自适合什么场景，以及 `.ok_or_else` / `.ok` 怎么互转
- [ ] 我能默写 `?` 的两条语义，包括 `Err` 分支上的 `Err(From::from(e))`
- [ ] 我知道 `?` 只能用在返回 `Result` / `Option` 的函数里，也知道 `main` 可以返回 `Result<(), Box<dyn Error>>`
- [ ] 我能解释 `unwrap` 和 `expect` 的区别，并说出至少两个「可以用 `unwrap`」的场景
- [ ] 我能给自定义错误实现 `From<底层错误>`，并说清 `Box<dyn Error>` 丢掉了什么信息
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch06_error_handling` 全绿
