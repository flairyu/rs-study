# 第 16 章 · 标准库巡礼

> **一句话定位**：把 `std` 当成一张地图来读——知道「什么在库里、什么必须去 crates.io 找」，比背 API 更重要。
> **预计时间**：100 分钟（读 50 + 写 50）
> **前置**：第 15 章（测试、unsafe 与 FFI）　**练习工程**：`exercises/ch16_stdlib_tour`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一个需求「读文件并数字数」，三门语言的答案来自完全不同的地方：

```c
/* C：stdio 只给 FILE*；路径、编码、错误码都是平台的事 */
FILE *f = fopen("notes.txt", "r");
if (!f) { perror("fopen"); return 1; }   /* errno 是全局变量 */
```

```python
# Python：内置电池，开箱即用，出错抛异常（但异常容易漏接）
with open("notes.txt", encoding="utf-8") as f:
    words = f.read().split()
```

```rust
// Rust：std 只给"可移植的最小核心"，错误走返回值
fn word_count(path: &str) -> std::io::Result<usize> {
    let text = std::fs::read_to_string(path)?;   // io::Result<String>
    Ok(text.split_whitespace().count())
}
```

| 维度 | C / C++ | Python | Rust |
|---|---|---|---|
| 标准库体量 | 很薄（`libc` 是操作系统的，不是语言的） | 极厚（"内置电池"） | **刻意精简**：只放跨平台、稳定、无争议的部分 |
| 平台相关性 | 高：POSIX 能用的 Windows 常没有 | 低：标准库自己抹平 | 低：同一份代码跨平台 |
| 字符串与编码 | `char*` + 编码约定 | `str` / `bytes` | `String`/`&str` 保证 UTF-8，非 UTF-8 用 `OsString` |
| 报告错误 | `errno` + 返回码（易忽略） | 异常（易漏接） | `Result<T, E>`：**编译器逼你处理** |
| 典型缺口 | 容器、网络、正则都没有 | 几乎没有 | 异步运行时/正则/随机数/日期/序列化/日志都**不在 std** |

C 的标准库薄，是因为它其实是操作系统的 API；Python 的标准库厚，是因为它想把常见需求全包了；**Rust 的 std 刻意保持精简，因为它必须同时满足「永不破坏兼容」和「处处可移植」**。一个 API 进了 std 就几乎不能改语义、不能删，所以凡是**还在演进**（异步运行时、正则引擎）、**有平台依赖**（随机数熵源、TLS）、或**存在多种合理取舍**（日志格式、时区）的东西，社区一律把它们留在 crates.io。这不是缺失，而是设计：**std 负责地基，生态负责楼层。**

---

## 1. std 模块地图

| 模块 | 里面有什么 | 你什么时候用它 |
|---|---|---|
| `std::collections` | `Vec` `String` `HashMap` `BTreeMap` `HashSet` `VecDeque` `BinaryHeap` | 数据结构（第 7 章） |
| `std::io` | `Read` `Write` `Seek` `BufReader` `BufWriter` `Error` `Result`、`stdin/stdout` | 输入输出、文件流、管道 |
| `std::fs` / `std::path` | `read_to_string` `write` `File` `read_dir`、`Path` `PathBuf` | 操作文件与目录；拼路径**别用字符串切** |
| `std::env` / `std::process` | `args` `var` `temp_dir`、`Command` `exit` | 命令行参数、环境变量、子进程 |
| `std::time` | `Duration` `Instant` `SystemTime` `UNIX_EPOCH` | 超时、计时、时间戳 |
| `std::fmt` | `Display` `Debug` `Formatter` `write!` | 自定义打印格式（第 14 章） |
| `std::num` | `ParseIntError` `ParseFloatError` `NonZeroU32` `Wrapping` | 数字解析、"不会为零"的类型 |
| `std::sync` / `std::thread` | `Arc` `Mutex` `mpsc` `OnceLock`、`spawn` `scope` `sleep` | 线程与共享状态（第 12 章） |
| `std::net` | `TcpListener` `TcpStream` `UdpSocket` `SocketAddr` | 裸 TCP/UDP（HTTP 不在 std） |
| `std::error` / `std::convert` | `Error` trait、`From` `Into` `TryFrom` `AsRef` | 统一错误抽象；类型转换 |
| `std::iter` / `ops` / `cmp` / `hash` / `borrow` | `Iterator` 全部适配器、`Add` `Index` `Deref`、`Ord`、`Hash`、`Cow` | 迭代、运算符重载、排序、可哈希类型 |
| 其他常用 | `mem` `ptr` `slice` `str` `option` `result` `ffi` `os` | 底层操作、FFI、平台特化（第 15 章） |

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn main() {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    counts.insert("rust", 1);
    println!("{counts:?}");                                  // 输出: {"rust": 1}
    let p: PathBuf = Path::new("/tmp/notes.tar.gz").to_path_buf();
    println!("{:?} {:?}", p.file_stem(), p.extension());     // 输出: Some("notes.tar") Some("gz")
}
```

### 1.1 一条贯穿全库的惯例：借用版做参数，拥有版做返回值

`String`/`&str`、`PathBuf`/`&Path`、`Vec<T>`/`&[T]`、`OsString`/`&OsStr` 都是"拥有版 + 借用视图"的成对类型。惯例是 **"borrow in, own out"**：参数收 `&str`/`&Path`/`&[T]`（调用者不必交出所有权），返回值该给拥有版就给 `String`/`PathBuf`（否则活不过函数体）。所以写 API 时参数用 `&Path` 而不是 `&PathBuf`——后者会把调用者锁死在一种类型上。

---

## 2. io：`Read` / `Write`、缓冲与所有权

`std::io` 的核心是两个 trait，而不是具体类型：`Read` 有 `fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>`，`Write` 有 `write` 与 `flush`。注意都是 **`&mut self`**——读/写会推进内部的游标位置。`File`、`TcpStream`、`Stdin` 实现了 `Read`，`File`、`TcpStream`、`Stdout` 实现了 `Write`；参数写 `impl Read` 就能让同一个函数既读文件、又读网络、还读内存里的 `&[u8]`。

一次完整的"写进去再读出来"，顺带看清缓冲与所有权：

```rust
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() -> std::io::Result<()> {
    let path = std::env::temp_dir().join("rs_study_ch16_demo.txt");
    std::fs::write(&path, "alpha\nbeta\n")?;
    let text = std::fs::read_to_string(&path)?;     // 整个文件 -> String
    println!("{}", text.lines().count());           // 输出: 2

    let file = File::open(&path)?;                  // 拿到文件句柄的所有权
    let reader = BufReader::new(file);              // 句柄所有权转移进 BufReader
    for line in reader.lines() {
        println!("[{}]", line?);                    // 输出: [alpha] 然后 [beta]
    }                                               // reader 离开作用域 -> 文件关闭
    let _ = std::fs::remove_file(&path);
    Ok(())
}
```

`File::read` 每次都陷入内核（一次系统调用），逐字节读大文件 = 几十万次系统调用。`BufReader` 在用户态加一层缓冲区，把"很多次小读"合并成"少数几次大读"；`BufWriter` 是它的镜像（把很多次小写攒成一次大写，但别忘显式 `flush()`——drop 时的错误会被丢掉）：

| 写法 | 系统调用 | 适用场景 |
|---|---|---|
| `File::read` 直接循环 | 每次 `read` 一次 | 大块传输（`io::copy`） |
| `BufReader::new(file)` | 约每 8 KiB 一次 | **逐行/逐块处理**，默认就该用 |
| `fs::read_to_string` | 一次性读完 | 小文件、配置文件 |

> ⚠️ **坑**：`read_to_string` 是**追加**到已有的 `String` 上，不是覆盖。循环里复用同一个 buffer 时必须先 `buf.clear()`。

---

## 3. path：`PathBuf` / `Path` / `String` 的分工

`Path` 之于 `PathBuf`，正如 `str` 之于 `String`：一个是**借用视图**（`?Sized`），一个是**拥有所有权的容器**。**为什么不用 `String` 存路径？** 因为"路径"不等于"UTF-8 文本"：Windows 路径内部是 UTF-16，转 UTF-8 可能失败；分隔符也不同（`\` vs `/`）。`PathBuf` 内部用 `OsString` 保存，只在给人看时才通过 `display()` 转换。这也是第 1.1 节惯例的又一例：函数收 `&Path`，返回 `PathBuf`。

| 方法 | 作用 | `"/tmp/notes.tar.gz"` 上 |
|---|---|---|
| `file_name()` | 最后一段（含扩展名） | `Some("notes.tar.gz")` |
| `file_stem()` | 主名（去掉最后一段扩展名） | `Some("notes.tar")` |
| `extension()` | 最后一个点之后的部分 | `Some("gz")` |
| `parent()` / `join("x")` | 上一级目录 / 拼接子路径（自动补分隔符） | `Some("/tmp")` / `/tmp/notes.tar.gz/x` |
| `with_extension("md")` / `display()` | 换扩展名 / 给人看（非 UTF-8 会丢字符） | `/tmp/notes.tar.md` |

```rust
use std::path::{Path, PathBuf};

fn main() {
    let full: PathBuf = Path::new("/tmp/data").join("report.txt");   // 跨平台安全
    println!("{}", full.with_extension("md").display());             // 输出: /tmp/data/report.md
    println!("{:?}", Path::new("README").extension());               // 输出: None
}
```

几处必须知道的特例（练习 16-2 正是考它）：`".gitignore"` 以点开头且后面没有别的点 → **整个名字都是主名，扩展名是 `None`**；`"archive.tar.gz"` 只取**最后一个**点之后 → `Some("gz")`；`"/"`、`"."`、`".."`、`""` 都没有"最后一段文件名" → 主名与扩展名都是 `None`；结尾斜杠（`"a/b/"`）不影响判定。

> ⚠️ **坑**：不要用 `rsplit('.')` 手写这个逻辑，你会立刻在 `.gitignore`、`archive.tar.gz` 和"无扩展名"上翻车。

---

## 4. time：`Duration` / `Instant` / `SystemTime`

| 类型 | 是什么 | 用途 | 会倒退吗 |
|---|---|---|---|
| `Duration` | 一段**时间长度**（内部是秒 + 纳秒的整数对，不是 `f64`） | 超时、间隔、差值 | 不适用 |
| `Instant` | **单调时钟**上的一个点 | 测耗时、超时判定 | 不会 |
| `SystemTime` | **墙上时钟**上的一个点 | 时间戳、日志、文件时间 | **会**（NTP 校时） |

```rust
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(3));
    println!("{}", start.elapsed() >= Duration::from_millis(3));   // 输出: true
}
```

> ⚠️ **坑**：**永远不要用 `SystemTime` 测耗时**。时钟被往回拨时，你的"耗时"会变成负数或离谱的值。

---

## 5. num 与 `FromStr`：`parse` 是统一入口

`str::parse` 不是特殊函数，它是 `FromStr` 的语法糖（`fn from_str(s: &str) -> Result<Self, Self::Err>`）；所有数字类型、`bool`、`char`、`IpAddr`、`PathBuf` 都实现了它。调用时靠**返回类型推断**决定解析成什么，推不出来就用 turbofish：

```rust
fn main() {
    let n: i32 = "42".parse().unwrap();              // 靠左边类型标注
    let f = "3.5".parse::<f64>().unwrap();           // turbofish
    println!("{n} {f}");                             // 输出: 42 3.5
    println!("{:?}", " 42 ".parse::<i32>());         // 输出: Err(ParseIntError { kind: InvalidDigit })
    println!("{:?}", "1e3".parse::<f64>());          // 输出: Ok(1000.0)（i32 则会失败）
}
```

为什么是 `Result` 而不是异常？因为在 Rust 里 **`Result` 是所有可能失败的统一货币**：`?` 能把它一路往上传，最终变成退出码或被 `match` 成分支。失败类型是具体的（`ParseIntError`/`ParseFloatError`），意味着你可以在签名里承诺"失败只有这一种原因"。

> ⚠️ **坑**：`parse` **不 trim、不认千分位、不认 `0x`**——`" 42 "`、`"1_000"`、`"0xff"` 全是 `Err`。反过来 `f64` 接受 `"NaN"`/`"inf"`（大小写不敏感），要拒绝就自己查 `is_finite()`。

---

## 6. error：`std::error::Error` 与 `Box<dyn Error>`

`std::error::Error` 的定义只有一件事要记：它要求 `Debug + Display`，外加一个 `fn source(&self) -> Option<&(dyn Error + 'static)>`（默认返回 `None`）。`Display` 给用户看，`Debug` 给程序员看，`source()` 把多个错误串成链——这是 `?` 能"保留根因"的基础。自定义错误只要实现 `Display + Error` 就能融入生态：

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]                       // Debug 由 derive 提供
struct BadConfig(String);              // 存下出问题的那一行
impl fmt::Display for BadConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "配置行不是数字: {:?}", self.0)
    }
}
impl Error for BadConfig {}            // 有内层错误时再覆盖 source() 返回它

// io::Error 和 ParseFloatError 都能自动装进 Box<dyn Error>
fn load(path: &str) -> Result<f64, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;    // io::Error -> Box<dyn Error>
    let value: f64 = text.trim().parse()?;        // ParseFloatError -> Box<dyn Error>
    Ok(value)
}
```

`?` 能把各种错误塞进 `Box<dyn Error>`，靠的是标准库的 `impl<E: Error + 'static> From<E> for Box<dyn Error>`：**任何标准错误类型都能自动装箱**。`main` 也可以写成 `fn main() -> Result<(), Box<dyn Error>>`，这样顶层也能直接用 `?`。

> ⚠️ **坑**：`Box<dyn Error>` **不是 `Send + Sync`**，不能跨线程传递（要写 `Box<dyn Error + Send + Sync>`）；而且装箱**抹掉具体类型**，调用者只能读消息、不能 `match`。库的公开 API 应该返回自定义 `enum`，`Box<dyn Error>` 留给 `main` 和内部胶水代码。

---

## 7. std 有什么 / 没有什么

| 能力 | 在 std 里？ | 该用什么 |
|---|---|---|
| 容器、字符串、迭代器 | ✅ 有 | `std::collections` 等 |
| 文件、目录、路径、环境变量 | ✅ 有 | `std::fs` / `std::path` / `std::env` |
| 线程、锁、channel、原子类型 | ✅ 有 | `std::thread` / `std::sync` |
| 裸 TCP / UDP | ✅ 有 | `std::net` |
| 单调时钟与时长 | ✅ 有 | `Instant` / `Duration` |
| 全局懒初始化 | ✅ 有（1.80+） | `OnceLock` / `LazyLock`（`once_cell` 多数场景已不需要） |
| HTTP 客户端 / 服务端 | ❌ 无 | `reqwest` / `ureq` / `axum` |
| async 运行时 | ❌ 无（只有 `Future`/`Waker` 接口，见第 13 章） | `tokio` / `async-std` / `smol` |
| 正则表达式 | ❌ 无 | `regex` |
| 随机数 | ❌ 无 | `rand`（std 只在 `HashMap` 内部做哈希随机化） |
| 日期、时区、日期格式化 | ❌ 无（只有时间戳与 `Duration`） | `chrono` / `time` |
| 序列化（JSON/TOML/…） | ❌ 无 | `serde` + `serde_json` / `toml` |
| 日志、命令行参数解析 | ❌ 无（只有裸的 `env::args`） | `log` / `tracing`、`clap` |
| 临时文件/目录 | ❌ 无（只能手拼 `env::temp_dir()`） | `tempfile` |
| 压缩、TLS/HTTPS | ❌ 无 | `flate2` / `zstd`、`rustls` / `native-tls` |
| 人机友好的错误报告 | ⚠️ 有 trait，无脚手架 | `anyhow`（应用） / `thiserror`（库） |

### 7.1 为什么偏偏是这几个不在 std

- **async 运行时**：std 只定义了 `Future`/`Poll`/`Waker` 这套**接口**，但"怎么调度、几个线程、要不要 epoll/io_uring"是重大取舍；写进 std 等于钦定一种运行时，生态会失去演进空间——所以第 13 章才要自己写玩具执行器。
- **正则**：编译期显著增大二进制体积，引擎算法仍在演进（回溯 vs 自动机、Unicode 属性支持程度），放 crates.io 按版本迭代更好。
- **随机数**：熵源是平台相关的（`/dev/urandom`、`BCryptGenRandom`、`getrandom`），"用哪个 PRNG"也没有唯一答案（安全 vs 速度）。
- **日期与时区**：时区数据库每年更新好几次，而 std 跟着工具链发布节奏走，放进去会立刻过期。

共同规律：**会持续演进、有平台依赖、或存在多种合理取舍 → 出 std；普适、稳定、跨平台 → 进 std。**

---

## 8. 常见坑（汇总）

1. **`read_to_string` 是追加**：复用 buffer 前先 `clear()`；**`read` 不保证读满**，要读够 N 字节用 `read_exact`。
2. **`lines()` 会剥掉行尾**（`\n` 与 `\r\n` 都剥）；要原样保留用 `read_line`。
3. **路径不要用字符串切**：`file_stem`/`extension` 已处理 `.gitignore`、`a.tar.gz`、`..` 等特例；`to_str()` 对非 UTF-8 返回 `None`（给人看用 `display()`）。计时则只用 `Instant`——`SystemTime` 会被校时调整。
4. **`parse` 很严格**：不 trim、不认 `_` 和 `0x`；但 `f64` 认 `"NaN"`/`"inf"`。
5. **浮点四舍五入有两条路，结果可能不同**（已实测）：

   ```rust
   fn main() {
       println!("{:.2}", 2.005_f64);                            // 输出: 2.00
       println!("{:.2}", (2.005_f64 * 100.0).round() / 100.0);  // 输出: 2.01
       println!("{:.2}", (0.125_f64 * 100.0).round() / 100.0);  // 输出: 0.13（四舍五入）
       println!("{:.2}", 0.125_f64);                            // 输出: 0.12（就近取偶）
   }
   ```

   `2.005` 在 f64 里是 `2.00499999999999989…`，`{:.2}` 对原值正确舍入得 `2.00`；但它乘 100 后恰好被舍入到 `200.5`，`round()` 就进位了。**涉及钱和精度时用整数分或 `rust_decimal`。**

6. **`Box<dyn Error>` 不跨线程**：需要时写 `Box<dyn Error + Send + Sync>`。

---

## 9. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 16-1 | 统计文本文件的单词数 | `fs::read_to_string`、`io::Result`、错误原样传播 | `count_words_in_file` |
| 16-2 | 把路径拆成 `(主名, 扩展名)` | `Path` 的点号特例、`Option` 边界 | `file_stem_and_ext` |
| 16-3 | 解析数字并保留两位小数 | `FromStr`/`parse`、`ParseFloatError`、`{:.2}` | `round_trip_number` |

题目规格写在 [`src/lib.rs`](../../exercises/ch16_stdlib_tour/src/lib.rs) 的文档注释里（含可执行示例）。
```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch16_stdlib_tour                       # 做题前先跑一次，看它怎么失败
python3 tools/ai_review.py ch16_stdlib_tour --hint 2          # 卡住时
```

### 提示（不给答案）

- **16-1**：`fs::read_to_string(path)?` 拿到 `String`，再 `split_whitespace().count()`。测试会真的在 `std::env::temp_dir()` 下建文件，别忘了"文件不存在要返回 `Err`"。
- **16-2**：别手写切分。`Path::file_stem()`/`extension()` 返回 `Option<&OsStr>`，用 `.map(|s| s.to_string_lossy().into_owned())` 转换，主名再 `.unwrap_or_default()` 折叠成空串。
- **16-3**：`let v: f64 = s.parse()?;`——`?` 自动把 `ParseFloatError` 交出去；格式化用 `format!("{:.2}", ...)`。测试用的都是不敏感输入，不必纠结 `2.005`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 16-1：? 把 io::Error 原样抛出、不做包装，调用者因此还能读到 error.kind()。
pub fn count_words_in_file(path: &std::path::Path) -> std::io::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    Ok(text.split_whitespace().count())
}

// 16-2：把「点号规则」全部交给 Path；file_stem 为 None（""、"/"、"."、".."）时主名折叠成空串。
pub fn file_stem_and_ext(path: &str) -> (String, Option<String>) {
    let p = std::path::Path::new(path);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| e.to_string_lossy().into_owned());
    (stem, ext)
}

// 16-3：先乘 100、round、再除 100 实现四舍五入，最后 {:.2} 补齐两位小数（"42" -> "42.00"）。
pub fn round_trip_number(s: &str) -> Result<String, std::num::ParseFloatError> {
    let value: f64 = s.parse()?;
    let rounded = (value * 100.0).round() / 100.0;
    Ok(format!("{rounded:.2}"))
}
```

**为什么 16-2 不用 `split('.')`？** `Path` 的规则是"最后一个点，且以点开头的文件名不算扩展名"：`".gitignore"` 没有扩展名，`"a.tar.gz"` 的扩展名是 `gz`，`".."` 根本没有主名。手写要把这些特例一条条补上，而 `Path` 已经帮你写好了——这就是"用标准库表达意图"。

</details>

---

## 自测清单

- [ ] 我能说出 `std` 里**没有**的四样东西（async 运行时、正则、随机数、日期）以及各自该用的 crate
- [ ] 我知道 `PathBuf`/`Path` 为什么要独立于 `String`/`&str` 存在，以及 `.gitignore`/`a.tar.gz` 的 stem/ext
- [ ] 我知道 `Duration`/`Instant`/`SystemTime` 的分工，以及为什么不能用 `SystemTime` 计时
- [ ] 我能解释 `BufReader` 解决什么性能问题，以及 `read_to_string` 的"追加"语义
- [ ] 我能给自定义错误实现 `Display + Error` 并塞进 `Box<dyn Error>`，也知道它为什么不能跨线程、为什么浮点四舍五入有两条路
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch16_stdlib_tour` 全绿
