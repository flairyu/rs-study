# 章节规格总表（CHAPTER SPECS）

> 本文件锁定每一章的**练习 API 签名**与**文档路径**。写章节内容时以本表为准，
> 格式契约见 [AUTHORING.md](AUTHORING.md)。
>
> 所有函数签名必须**逐字**使用。测试用例由章节作者设计，但必须与签名和语义一致。

---

## ch02_control_flow — 第 2 章 · 函数与控制流

文档：`docs/01-syntax/ch02-control-flow.md`

覆盖：函数定义与返回值、`表达式 vs 语句`（末尾无分号）、`if` 是表达式、`loop`/`while`/`for`、
`match` 的穷尽性与模式、`loop` 带返回值、`break`/`continue` + 标签。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn fizzbuzz(n: u32) -> String` | `n` 同时被 3 和 5 整除返回 `"FizzBuzz"`；只被 3 整除返回 `"Fizz"`；只被 5 整除返回 `"Buzz"`；否则返回 `n` 的十进制字符串 |
| 2 | `pub fn collatz_steps(n: u64) -> u32` | 从 `n` 出发，偶数除 2、奇数乘 3 加 1，返回到达 1 所需的步数。`n == 1` 返回 `0`；`n == 0` 返回 `0` |
| 3 | `pub fn membership_level(years: u32, points: u32) -> &'static str` | 用 `match (years, points)` + `if` 守卫，**按顺序**判断：`years >= 5 \|\| points >= 1000` → `"钻石"`；`years >= 3 \|\| points >= 500` → `"黄金"`；`years >= 1 \|\| points >= 100` → `"白银"`；其余 → `"青铜"` |

---

## ch03_ownership — 第 3 章 · 所有权

文档：`docs/01-syntax/ch03-ownership.md`

覆盖：栈/堆、`move` 语义、`Copy` 与 `Clone` 的分界、函数传参即转移、
返回值即转移、`Drop` 与作用域、为什么不需要 `free`/`delete`、与 C++ RAII/移动语义的对照。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn shout(s: String) -> String` | 接收 `String` 的所有权，返回其全大写形式（`to_uppercase`） |
| 2 | `pub fn join_and_count(words: Vec<String>) -> (usize, String)` | 消费 `Vec`，返回 `(单词个数, 用单个空格连接后的字符串)`；空 `Vec` 返回 `(0, "")` |
| 3 | `pub fn consume_and_give_back(s: String) -> (String, usize)` | 返回 `(原字符串, 其字节长度)`，用来体会"转移出去再还回来"有多啰嗦 |

---

## ch04_borrowing — 第 4 章 · 引用与借用

文档：`docs/01-syntax/ch04-borrowing.md`

覆盖：`&T` 与 `&mut T`、同一时刻「多个共享借用」或「一个可变借用」、
借用不能比所有者活得久、切片 `&[T]`/`&str`、NLL（非词法生命周期）直觉。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn longest_word(text: &str) -> &str` | 按空白分词，返回第一个最长单词的切片；空输入或全空白返回 `""`；长度相同时取靠前的 |
| 2 | `pub fn append_exclaim(s: &mut String)` | 在末尾追加一个 `!` |
| 3 | `pub fn halves(nums: &[i32]) -> (&[i32], &[i32])` | 从 `len() / 2` 处切分（整数除法，向下取整），返回前段与后段 |

---

## ch05_structs_enums — 第 5 章 · 结构体、枚举与模式匹配

文档：`docs/01-syntax/ch05-structs-enums.md`

覆盖：结构体与字段、`impl` 与方法/关联函数、`&self` vs `self` vs `&mut self`、
`#[derive(Debug, Clone, PartialEq)]`、枚举携带数据、`Option<T>`、`match` 解构、
`if let`、`matches!` 宏。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub struct Rectangle { pub width: f64, pub height: f64 }` | 派生 `Debug, Clone, PartialEq`；`impl Rectangle { pub fn new(width: f64, height: f64) -> Self; pub fn area(&self) -> f64; pub fn is_square(&self) -> bool; pub fn scale(&mut self, factor: f64) }`。`is_square` 用容差 `1e-9` 比较 |
| 2 | `pub enum Shape { Circle(f64), Rectangle(f64, f64), Triangle(f64, f64) }`（派生 `Debug, Clone, PartialEq`）<br>`pub fn shape_area(shape: &Shape) -> f64` | 圆 `π r²`、矩形 `w*h`、三角形 `0.5*底*高`。用 `std::f64::consts::PI` |
| 3 | `pub fn describe_opt(value: Option<i32>) -> String` | `None` → `"none"`；`Some(0)` → `"zero"`；`Some(n)` 且 `n > 0` → `"positive: {n}"`；`Some(n)` 且 `n < 0` → `"negative: {n}"` |

---

## ch06_error_handling — 第 6 章 · 错误处理

文档：`docs/01-syntax/ch06-error-handling.md`

覆盖：`panic!` vs `Result`、`Result<T, E>` 与 `Option<T>` 的选择、
`?` 运算符、`unwrap`/`expect` 的适用场景、自定义错误枚举、
`From` 驱动的错误传播、与 C 的错误码 / Python 异常的对照。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn parse_age(s: &str) -> Result<u8, String>` | 先 `trim`；空串 → `Err("年龄不能为空".into())`；解析失败 → `Err(format!("不是合法数字: {s}"))`；`> 150` → `Err("年龄超出范围".into())`；否则 `Ok(age)` |
| 2 | `pub fn sum_numbers(input: &str) -> Result<i64, String>` | 按 `,` 切分；空串或全空白 → `Ok(0)`；任一段解析失败 → `Err(format!("不是合法数字: {seg}"))`（用 `?` 传播） |
| 3 | `#[derive(Debug, PartialEq, Eq)] pub enum PortError { Empty, NotANumber(String), TooLarge(u32) }`<br>`pub fn parse_port(s: &str) -> Result<u16, PortError>` | `trim` 后为空 → `Empty`；先按 `u32` 解析，失败 → `NotANumber(原串)`；`> 65535` → `TooLarge(该值)`；否则 `Ok(v as u16)` |

---

## ch07_collections — 第 7 章 · 集合

文档：`docs/01-syntax/ch07-collections.md`

覆盖：`Vec<T>` 的增删改查与扩容、`String` vs `&str` vs `str`、
`HashMap`/`BTreeMap` 的选择、`HashSet`、所有权在集合中的体现、
`entry` API、容量与 `with_capacity`。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn mean_and_median(nums: &[i32]) -> Option<(f64, f64)>` | 空返回 `None`；中位数：排序后奇数个取中间，偶数个取中间两数平均；返回 `(均值, 中位数)` |
| 2 | `pub fn word_count(text: &str) -> std::collections::HashMap<String, usize>` | 转小写、按空白分词、统计词频 |
| 3 | `pub fn dedup_preserve_order(items: &[String]) -> Vec<String>` | 去重但保留首次出现顺序；用 `HashSet` 记录已见 |

---

## ch08_iterators_closures — 第 8 章 · 闭包与迭代器

文档：`docs/01-syntax/ch08-iterators-closures.md`

覆盖：三种闭包 trait `Fn`/`FnMut`/`FnOnce`、闭包捕获、
迭代器的惰性、`map`/`filter`/`fold`/`sum`/`collect`/`take`/`skip`/`zip`/`enumerate`/`chain`、
`&T` 与 `T` 的迭代区别、自定义 `Iterator`（选讲）。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn sum_of_squares_of_evens(nums: &[i32]) -> i64` | 用迭代器链计算所有偶数平方和（返回 `i64`） |
| 2 | `pub fn apply_n_times<F: Fn(i32) -> i32>(f: F, x: i32, n: u32) -> i32` | 把 `f` 在 `x` 上迭代应用 `n` 次 |
| 3 | `pub fn count_by_length(words: &[&str]) -> std::collections::BTreeMap<usize, usize>` | 统计每个"单词长度"出现了多少次（key 按升序） |

---

## ch09_generics_traits — 第 9 章 · 泛型与 trait

文档：`docs/01-syntax/ch09-generics-traits.md`

覆盖：泛型函数/结构体、trait 定义与实现、trait bound 与 `where`、
默认方法、`impl Trait` 参数与返回位置、`dyn Trait` 与动态分发、
泛型单态化（zero-cost）与 C++ 模板的对照、孤儿规则。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T>` | 返回最大值，空返回 `None`（用 `>` 比较） |
| 2 | `pub trait Describable { fn describe(&self) -> String; }`<br>`pub struct Point { pub x: f64, pub y: f64 }`<br>`impl Describable for Point` | `describe` 返回 `format!("Point({x}, {y})")`，浮点用 `{}`（如 `Point(1.5, -2)`）。同时给 `Point` 派生 `Debug, Clone, PartialEq` |
| 3 | `pub fn describe_all(items: &[Box<dyn Describable>]) -> Vec<String>` | 对每个元素调用 `describe` 收集成 `Vec` |

---

## ch10_lifetimes — 第 10 章 · 生命周期

文档：`docs/01-syntax/ch10-lifetimes.md`

覆盖：为什么需要生命周期标注、`'a` 是"承诺"而非"指令"、
函数签名中的省略规则（三条）、结构体持有引用、`&'a self`、
`'static`、生命周期与借用检查器的关系。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str` | 返回较长者；等长返回 `a` |
| 2 | `pub struct Parser<'a> { pub input: &'a str, pub pos: usize }`<br>`impl<'a> Parser<'a> { pub fn new(input: &'a str) -> Self; pub fn rest(&self) -> &'a str; pub fn next_word(&mut self) -> Option<&'a str> }` | `rest` 返回 `&input[pos..]`；`next_word` 跳过前导空白，返回下一个单词并推进 `pos`；没有剩余返回 `None` |
| 3 | `pub fn first_or<'a>(items: &'a [String], fallback: &'a str) -> &'a str` | 空切片返回 `fallback`，否则返回第一个元素的 `&str` |

---

## ch11_smart_pointers — 第 11 章 · 智能指针与内部可变性

文档：`docs/01-syntax/ch11-smart-pointers.md`

覆盖：`Box<T>`（堆分配 + 递归类型）、`Deref`/`DerefMut` 与自动解引用、
`Drop`、`Rc<T>` 共享所有权与引用计数、`RefCell<T>` 与运行时借用检查、
`Rc<RefCell<T>>` 组合、`Cell<T>`、与 C++ `shared_ptr` 的对照、循环引用与 `Weak`。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `#[derive(Debug, PartialEq)] pub enum List { Cons(i32, Box<List>), Nil }`<br>`pub fn from_slice(items: &[i32]) -> List` | 按给定顺序构造链表，`&[1,2,3]` → `Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))))` |
| 2 | `pub fn to_vec(list: &List) -> Vec<i32>` | 把链表还原成 `Vec`（原地顺序） |
| 3 | `pub fn push_all(handles: &[std::rc::Rc<std::cell::RefCell<Vec<i32>>>], value: i32) -> usize` | 对每个 handle 执行 `borrow_mut().push(value)`，返回**最后一个** handle 的长度（空 handles 返回 0） |

---

## ch12_concurrency — 第 12 章 · 并发

文档：`docs/01-syntax/ch12-concurrency.md`

覆盖：`thread::spawn` 与 `move` 闭包、`JoinHandle`、`Send`/`Sync` 是什么、
`Arc<Mutex<T>>` 模式、`mpsc` channel、消息传递 vs 共享内存、
`std::thread::scope`、为什么"数据竞争"在安全 Rust 里编译不过。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn parallel_sum(nums: Vec<i32>) -> i32` | 用 `std::thread::scope` 或 `spawn` 分 4 片并行求和再合并；空输入返回 0 |
| 2 | `pub fn counter_with_mutex(threads: usize, per_thread: usize) -> usize` | `Arc<Mutex<usize>>`，`threads` 个线程各加 `per_thread` 次，返回最终值 |
| 3 | `pub fn producer_consumer(items: Vec<i32>) -> Vec<i32>` | 生产者线程通过 `mpsc::channel` 把每个元素 `* 2` 发出去，主线程收完排序后返回 |

---

## ch13_async — 第 13 章 · async / await 与 Future

文档：`docs/01-syntax/ch13-async.md`

覆盖：`Future` 是什么（可被轮询的状态机）、`async fn` 返回 `impl Future`、
`.await` 的语义、为什么需要执行器、`Pin` 的直觉解释、
与线程/回调的对照、tokio 的角色。

**本 crate 的 `src/lib.rs` 必须原样包含下面这段「教学玩具执行器」**
（用户不需要实现它，它是理解 `.await` 的实验台）：

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// 教学用极简执行器：用一个"什么都不做"的 waker 反复 poll 直到就绪。
/// 它不做任务调度、不跨线程、不省 CPU——只用来说明 `.await` 背后到底发生了什么。
///
/// noop waker 的构造方式取自 `futures-util::task::noop_waker`（同一个模式）。
pub fn block_on<F: Future>(fut: F) -> F::Output {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);
    // SAFETY: 上面四个函数指针都是空操作，data 指针为 null 且永不被解引用。
    let waker = unsafe { Waker::from_raw(RAW) };
    let mut cx = Context::from_waker(&waker);
    let mut fut: Pin<Box<F>> = Box::pin(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 一个"第一次 poll 返回 Pending，第二次就绪"的 Future，用来看清 `Pending` 的存在。
pub struct YieldNow(bool);

impl YieldNow {
    pub fn new() -> Self { Self(false) }
}

impl Default for YieldNow {
    fn default() -> Self { Self::new() }
}

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
```

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub async fn async_add(a: i32, b: i32) -> i32` | 返回 `a + b` |
| 2 | `pub async fn async_sum(values: &[i32]) -> i32` | 用 `for` 循环 + `.await`（内部调用 `async_add`）求和 |
| 3 | `pub async fn async_count_down(n: u32) -> Vec<u32>` | 每轮先 `.await` 一次 `YieldNow`，再 push 当前值，返回 `[n, n-1, ..., 1]`；`n == 0` 返回空 |

测试里用 `block_on(...)` 驱动。

---

## ch14_modules_macros — 第 14 章 · 模块、crate 与宏

文档：`docs/01-syntax/ch14-modules-macros.md`

覆盖：`mod` 与文件树、`pub`/`pub(crate)`/私有、`use` 与重导出 `pub use`、
`crate`/`super`/`self` 路径、包与 crate 的区别、
`macro_rules!` 的 `$x:expr` 片段与重复 `$(...)*`、
`#[derive]` 与声明宏/过程宏的区别、`Cargo.toml` 的 `[features]` 与 workspace。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub mod geometry { pub struct Point { pub x: f64, pub y: f64 } }` + `impl Point { pub fn new(x: f64, y: f64) -> Self; pub fn distance_to(&self, other: &Point) -> f64 }` + `pub use geometry::Point;` | `distance_to` 返回欧氏距离。`Point` 派生 `Debug, Clone, PartialEq`。测试用 `use ch14_modules_macros::Point;`（验证重导出生效） |
| 2 | `#[macro_export] macro_rules! hashmap_of { ... }` | 支持 `hashmap_of!{}`、`hashmap_of!{"a" => 1}`、`hashmap_of!{"a" => 1, "b" => 2,}`（允许尾逗号），生成 `std::collections::HashMap<&str, i32>` |
| 3 | `pub struct Temperature { pub celsius: f64 }` + `impl std::fmt::Display for Temperature` + `pub fn render(t: &Temperature) -> String` | `Display` 输出 `"{celsius}°C"`（用 `{}` 打印 f64）；`render` 返回 `t.to_string()` |

第 2 题是宏，`unimplemented!()` 放在宏展开的表达式位置即可。

---

## ch15_testing_unsafe — 第 15 章 · 测试、unsafe 与 FFI

文档：`docs/01-syntax/ch15-testing-unsafe.md`

覆盖：`#[test]` / `#[cfg(test)]` / 集成测试 / 文档测试 / `#[should_panic]`、
`unsafe` 的五个超能力（解引用裸指针、调 unsafe 函数、访问 `static mut`、
实现 unsafe trait、访问 union 字段）、`unsafe` 不等于"关掉借用检查"、
裸指针 `*const T`/`*mut T`、`extern "C"` 与 ABI、`CString`/`CStr`、
`#[no_mangle] extern "C"` 导出给 C 调用、`build.rs` 简介。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub unsafe fn read_raw(ptr: *const i32) -> i32` | 解引用裸指针并读值 |
| 2 | `pub fn abs_via_ffi(x: i32) -> i32` | 声明 `extern "C" { fn abs(input: i32) -> i32; }` 并调用（libc 的 `abs`） |
| 3 | `pub fn c_strlen(s: &str) -> usize` | 用 `std::ffi::CString::new(s)` 造 C 字符串，声明 `extern "C" { fn strlen(s: *const std::os::raw::c_char) -> usize; }` 并调用；`s` 含内部 `\0` 时返回 0 |

---

## ch16_stdlib_tour — 第 16 章 · 标准库巡礼

文档：`docs/02-stdlib/ch16-stdlib-tour.md`

覆盖：`std` 模块地图（`collections`/`io`/`fs`/`path`/`env`/`process`/`time`/`fmt`/`num`/
`sync`/`thread`/`net`/`error`/`convert`/`iter`/`ops`/`cmp`/`hash`/`borrow`）、
`Read`/`Write`/`BufReader`、`PathBuf` 与跨平台路径、`Duration`/`Instant`、
`FromStr`、`std::error::Error` trait、`?` 在 `Box<dyn Error>` 上的用法、
「标准库有什么 / 没有什么」清单。

| # | 签名 | 语义 |
|---|---|---|
| 1 | `pub fn count_words_in_file(path: &std::path::Path) -> std::io::Result<usize>` | 用 `fs::read_to_string` + `split_whitespace().count()`。文件不存在时把 `io::Error` 原样返回 |
| 2 | `pub fn file_stem_and_ext(path: &str) -> (String, Option<String>)` | 用 `std::path::Path` 取主名与扩展名；无主名返回 `("".to_string(), None)`；有主名无扩展名返回 `(stem, None)` |
| 3 | `pub fn round_trip_number(s: &str) -> Result<String, std::num::ParseFloatError>` | 把 `s` 解析成 `f64`，四舍五入到 2 位小数并格式化为 `{:.2}`；解析失败原样返回 `Err` |

---

## 第 17 章 · 常用第三方库（`labs/`）

文档：`docs/03-libs/ch17-ecosystem.md`，配套 `labs/` 下的独立小 crate。
**不属于自动评测**（`chapters.json` 里 `crate: null`）。

覆盖清单（每个给最小可运行示例 + 一句话选型建议）：
`serde`/`serde_json`、`regex`、`chrono` 或 `time`、`clap`、
`anyhow` + `thiserror`、`rayon`、`itertools`、`rand`、
`tracing` + `tracing-subscriber`、`reqwest`（异步 HTTP）、`tokio`、
`uuid`、`indexmap`、`once_cell`/`std::sync::OnceLock`、`tempfile`、
`thiserror` vs 手写错误类型的取舍、`cargo` 生态工具（`cargo add`/`clippy`/`fmt`/`audit`）。

---

## 第 18–21 章 · 框架入门（`projects/`）

文档：`docs/04-frameworks/*.md`，配套 `projects/` 下可独立运行的 demo。
**不属于自动评测**，需要网络拉取依赖。

| 章 | 框架 | demo 形态 |
|---|---|---|
| 18 | `axum` + `tokio` | 内存版 todo REST API（GET/POST/DELETE + JSON），含路由/提取器/状态共享/错误映射 |
| 19 | `bevy` | 一个 2D 小游戏：键盘移动的方块 + 计分文本 + 碰撞判定 |
| 20 | `sqlx` + SQLite | 建表、增删改查、`query_as!` 编译期校验、连接池、事务 |
| 21 | `candle` 与 `ort` | 用 `ort` 跑一个 ONNX 模型（或 `candle` 跑一个最小张量/线性回归），说明 Rust 在 AI 里的定位 |

每章三题：动手改造 demo（例如加一个路由 / 加一个敌人 / 加一个索引 / 换成真实模型），
不给自动评测，给出验收标准。
