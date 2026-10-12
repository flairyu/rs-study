# 第 1 章 · 变量、类型与可变性

> **一句话定位**：把「编译器替我兜底」的直觉建立起来——Rust 的类型系统比 C 严格，比 Python 早。
> **预计时间**：90 分钟（读 40 + 写 50）
> **前置**：无（先看 [00-overview.md](../00-overview.md) 和 [01-setup.md](../01-setup.md)）
> **练习工程**：`exercises/ch01_basics`（3 题）

---

## 0. 先建立锚点

你已经有 C/C++/Python 基础，所以本章不教「什么是变量」，只讲 **Rust 在哪里和它们不一样**。

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 变量默认 | 可改（`int x`） | 可改（`x = 1`） | **默认不可变**，要改必须写 `mut` |
| 类型 | 静态，声明式 | 动态，运行时 | 静态 + **局部类型推断** |
| 重名变量 | 不允许（同作用域） | 允许覆盖（同一个变量） | 允许 **shadowing**，且可以是不同类型 |
| 转换 | 隐式（`int`→`double`） | 隐式（任意） | **永不隐式**，必须 `as` 或 `From` |
| 整数除零 | UB / SIGFPE | 抛异常 | **panic**（带清晰信息） |
| 溢出 | UB（有符号） | 任意精度 | debug 下 panic，release 下回绕 |
| 未初始化 | 随机值 | 不存在 | **编译错误**，必须先赋值 |
| 空值 | `NULL` / `nullptr` | `None` | 没有 `null`，用 `Option<T>`（第 5 章） |

一句话总结差异的根源：**Rust 把 C/C++ 里"运行时才发现的错误"尽可能搬到了编译期**，
而且宁可让你多打几个字（`mut`、`as`），也不允许悄悄发生隐式行为。

---

## 1. 变量与可变性

```rust
fn main() {
    let x = 5;          // 不可变绑定
    // x = 6;           // ❌ error[E0384]: cannot assign twice to immutable variable

    let mut y = 5;      // 可变绑定
    y = 6;              // ✅

    println!("x = {x}, y = {y}");   // 输出: x = 5, y = 6
}
```

### 为什么默认不可变？

不是因为「不可变更快」（编译器基本能优化掉），而是因为**读代码时不必再追查谁会改它**。
C++ 里 `const` 是加法（你主动加），Rust 里 `mut` 是加法（你主动要）——
默认值反过来之后，函数签名里的 `mut` 就是一种**承诺**：这个值会被改。

### Shadowing（遮蔽）

```rust
let spaces = "   ";          // &str
let spaces = spaces.len();   // usize —— 名字复用，类型都换了
println!("{spaces}");        // 输出: 3

let mut n = 1;
n = n + 1;                   // 这是赋值，不是 shadowing
let n = n * 2;               // 这是 shadowing，新变量，可以换类型
```

和 `mut` 的区别：

| | 能否改类型 | 是否新变量 | 典型用途 |
|---|---|---|---|
| `let mut x` + `x = ...` | ❌ 不能 | ❌ 同一个 | 累加、状态机 |
| `let x = ...` 再来一次 | ✅ 能 | ✅ 新的 | 分步转换数据（解析 → 校验 → 计算） |

> ⚠️ **坑**：shadowing 会让旧变量**被遮蔽但没被销毁**，它的生命周期持续到作用域结束。
> 如果你用 `let` 遮蔽一个持有锁或文件句柄的变量，锁不会提前释放。

### 常量与静态

```rust
const MAX_POINTS: u32 = 100_000;         // 编译期常量，可放在任何作用域
static GREETING: &str = "hello";         // 全局变量，有固定内存地址
static mut COUNTER: u32 = 0;             // ⚠️ 需要 unsafe 才能访问，能不用就不用
```

对照 C：`const` ≈ `#define` + 类型检查（比 `#define` 安全得多），
`static` ≈ 全局变量。Rust 的 `const` 会被内联，没有内存地址。

---

## 2. 标量类型

```rust
// 整数：i8 i16 i32 i64 i128 isize / u8 u16 u32 u64 u128 usize
let a: i32 = -42;
let b = 0xff_u8;        // 255，后缀可以写到字面量上
let c = 1_000_000u64;   // 下划线只是给人看的

// 浮点：f32 f64（默认 f64）
let pi = 3.14159_f64;

// 布尔
let flag: bool = true;

// 字符：4 字节的 Unicode 标量值，不是 C 的 char
let ch: char = '中';     // ✅ 合法
// let bad: char = '中文'; // ❌ 只能是单个标量值

// 字符串字面量是 &str，不是 char 数组
let s: &str = "hello";
```

**整数默认 `i32`，浮点默认 `f64`**。推断不出来时必须标注：

```rust
let guess: u32 = "42".parse().expect("不是数字");
```

### `usize` / `isize`

指针宽度的整数（64 位机器上 64 位），**所有索引、长度都用它**：

```rust
let v = vec![1, 2, 3];
let n: usize = v.len();
let first = v[0];        // 索引是 usize
// let bad = v[0u32];    // ❌ 编译错误
```

> ⚠️ **坑（从 C 迁移最常犯）**：`v[-1]` 在 Rust 编译不过，`v[i]` 越界会 panic（不是 UB，也不会读到垃圾内存）。
> 这是好事：数组越界从「安全漏洞」降级为「崩溃」。

### 溢出行为

```rust
let x: u8 = 255;
// let y = x + 1;                 // debug 下 panic: attempt to add with overflow
let y = x.wrapping_add(1);        // 0，显式回绕
let z = x.saturating_add(1);      // 255，饱和
let (v, overflowed) = x.overflowing_add(1);   // (0, true)
```

- `cargo run`（debug）：溢出 **panic**。
- `cargo run --release`：静默回绕（二进制补码），和 C 一致。
- 想永远 panic：`overflow-checks = true` 写进 release profile。

---

## 3. 复合类型

### 元组：不同类型打包，定长

```rust
let t: (i32, f64, char) = (1, 2.5, 'x');
let (a, b, c) = t;             // 解构
println!("{}", t.0);           // 下标访问，从 0 开始
let unit = ();                 // 空元组 = unit 类型，相当于 C 的 void / Python 的 None
```

函数返回多值就是用元组，**不需要 C 那种指针出参**：

```rust
fn min_max(v: &[i32]) -> (i32, i32) {
    let mut lo = v[0];
    let mut hi = v[0];
    for &x in v {
        if x < lo { lo = x; }
        if x > hi { hi = x; }
    }
    (lo, hi)
}
```

### 数组：同类型，定长，在栈上

```rust
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let zeros = [0u8; 1024];        // 1024 个 0
println!("{}", arr.len());
for x in arr { println!("{x}"); }        // 拷贝遍历（i32 是 Copy）
for x in &arr { println!("{x}"); }       // 借用遍历
```

要变长就用 `Vec<T>`（第 7 章）。C 里 `int arr[5]` 会退化成指针，Rust 的数组是**真正的值类型**：

```rust
fn takes_array(a: [i32; 3]) { }
let arr = [1, 2, 3];
takes_array(arr);      // i32 是 Copy，arr 还能用
takes_array(arr);
```

---

## 4. 类型转换：没有隐式转换

```rust
let a: i32 = 7;
let b: i64 = a as i64;         // ✅ 显式
// let b: i64 = a;             // ❌ mismatched types

let f = 3.99_f64;
let n = f as i32;              // 3 —— 向零截断，和 C 一样
let back = n as u8;            // 注意：负数 as 无符号会回绕/饱和（Rust 1.45+ 对 float as int 是饱和）

let big: u32 = 300;
let small = big as u8;         // 44（300 % 256），as 在整数间是截断，不 panic
let safe = u8::try_from(big);  // Err(...) —— 想安全就用 TryFrom
```

### `as` vs `from` / `into` / `try_from`

| 写法 | 语义 | 会失败吗 |
|---|---|---|
| `x as T` | 原始截断 / 位重解释 | 不会（可能静默丢数据） |
| `T::from(x)` / `x.into()` | 无损转换 | 编译期就不允许有损的 |
| `T::try_from(x)` | 可能失败的转换 | 返回 `Result` |

```rust
let n: u8 = 200;
let wide: u32 = n.into();          // ✅ 无损，自动
let narrowed = u8::try_from(300u32);  // Err(TryFromIntError(()))
```

> ⚠️ **坑**：`as` 是"我知道我在干什么"的开关。数值计算里用 `as` 时，
> 先问自己「这个范围真的装得下吗」。装不下就用 `try_from`。

---

## 5. 格式化输出

Rust 的 `format!` 家族（`println!` / `format!` / `write!`）语法上像 Python 的 f-string，
但在**编译期**检查格式串，写错了直接编译失败。

```rust
let (name, score, total) = ("alice", 7, 8);
println!("{name}: {score}/{total}");                  // alice: 7/8
println!("{} {}", name, score);                       // 位置参数，和 C 类似
println!("{0}-{1}-{0}", "a", "b");                    // a-b-a，按索引重复用
println!("{:.1}%", score as f64 / total as f64 * 100.0);  // 87.5%
println!("{:>8}|{:<8}|{:^8}|", "right", "left", "mid");
println!("{:08.2}", 3.14159);                         // 00003.14
println!("{:?}", (1, "x"));                           // (1, "x")  Debug
println!("{:#?}", vec![1, 2]);                        // 美化版 Debug，多行
println!("{:b} {:o} {:x} {:X}", 255, 255, 255, 255);  // 11111111 377 ff FF
println!("{:e}", 1234.5);                             // 1.2345e3
```

对照表：

| Rust | Python | C |
|---|---|---|
| `{}` | `{}` | `%s` / `%d` |
| `{:.2}` | `{:.2f}` | `%.2f` |
| `{:>8}` | `{:>8}` | `%8d` |
| `{:<8}` | `{:<8}` | `%-8d` |
| `{:08}` | `{:08d}` | `%08d` |
| `{:x}` | `{:x}` | `%x` |
| `{:?}` | `{!r}`（有点像） | 无 |

**浮点默认不补零、整数默认不显示小数**：`format!("{}", 87.5_f64)` → `"87.5"`，
`format!("{}", 100.0_f64)` → `"100"`（注意！不是 `"100.0"`）。
要固定小数位就用 `{:.1}`。

---

## 6. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 1-1 | 华氏温度转摄氏温度 | 浮点算术、整数除法陷阱 | `fahrenheit_to_celsius` |
| 1-2 | 把总秒数拆成时/分/秒 | 整数除与取模、元组返回 | `split_duration` |
| 1-3 | 生成一行成绩文本 | `as` 转换、`{:.1}` 格式化 | `format_score_line` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch01_basics      # 做题前先跑一次，看它怎么失败
```

卡住了：

```bash
python3 tools/ai_review.py ch01_basics --hint 2
```

### 提示（不给答案）

- **1-1**：`5 / 9` 是整数除法，结果是 `0`。把其中一个操作数写成 `5.0` 或 `9.0`。
- **1-2**：`h = s / 3600`，`m = (s % 3600) / 60`，`sec = s % 60`。返回 `(h, m, sec)`。
- **1-3**：`score / total` 是整数除法，先 `score as f64`；再 `* 100.0`；格式化用 `{:.1}`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 1-1：把 5 / 9 写成 5.0 / 9.0。顺序上先减后乘除，避免中间结果太小丢精度。
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}

// 1-2：整数除法向零截断，配合 % 就能逐级剥离高位单位。
pub fn split_duration(total_seconds: u32) -> (u32, u32, u32) {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    (hours, minutes, seconds)
}

// 1-3：关键是 score / total 必须先转 f64，否则 7/8 会是 0。
// 用 {:.1} 固定一位小数，和题目要求的 "87.5%" 对齐。
pub fn format_score_line(name: &str, score: u32, total: u32) -> String {
    let percent = score as f64 / total as f64 * 100.0;
    format!("{name}: {score}/{total} ({percent:.1}%)")
}
```

**为什么 `1-1` 不写 `(f - 32.0) / 1.8`？**
数学上等价，但 `1.8` 在二进制浮点里不能精确表示，会引入多余的舍入误差。
`* 5.0 / 9.0` 只做一次乘一次除，误差更小、也更贴近公式。

</details>

---

## 自测清单

- [ ] 我能解释 `let x = 5; x = 6;` 为什么编译不过，以及 `mut` 加在哪
- [ ] 我能说出 shadowing 和 `mut` 赋值的两个区别
- [ ] 我知道 `let x: i32 = 1.5;` 为什么错，正确写法是什么
- [ ] 我能写出 `u8` 加法溢出的三种处理方式
- [ ] 我知道 `as` 什么时候会静默丢数据，以及该用什么替代
- [ ] 我能不查表写出「保留两位小数的百分比」的格式串
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch01_basics` 全绿
