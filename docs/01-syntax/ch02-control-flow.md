# 第 2 章 · 函数与控制流

> **一句话定位**：Rust 里 `if` / `loop` / `match` 都是**有值的表达式**——控制流不只是"往哪儿跳"，它还能产出数据。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 1 章 · 变量、类型与可变性
> **练习工程**：`exercises/ch02_control_flow`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 函数返回值 | `return x;` | `return x` | **最后一个表达式即返回值**，`return` 只用于提前退出 |
| 分号 | 每条语句都要 | 不写 | **分号把表达式的值丢掉**，多一个分号就改类型 |
| `if` 的值 | 语句，靠 `?:` 三元 | 有值（`a if c else b`） | **有值**：`let x = if c { a } else { b };`，没有三元 |
| 条件类型 | 非 0 即真 | truthy / falsy | **必须是 `bool`**，`if x`（`x: i32`）编译不过 |
| 无限循环 | `for(;;)` / `while(1)` | `while True` | `loop {}`，**只有它能把 `break value` 带出来** |
| 遍历 | 手写下标 | `for x in v` | `for x in &v`，**迭代器优先**，不写下标 |
| 多路分支 | `switch`：可 fallthrough、可不全 | `match`（3.10+） | `match`：**编译器强制穷尽**、无 fallthrough |

一句话：C 里控制流在"往哪儿跳"，Rust 里控制流在"**算出什么值**"。
所以 `let level = if points >= 1000 { "钻石" } else { "普通" };` 不需要任何中间变量。

## 1. 函数：签名就是契约

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b            // 末尾表达式，没有分号 → 它就是返回值
}

fn abs_early(x: i32) -> i32 {
    if x < 0 {
        return -x;   // 提前返回：带分号，是语句
    }
    x                // 主路径靠"末尾表达式"
}

fn main() {
    println!("{}", add(2, 3));       // 输出: 5
    println!("{}", abs_early(-7));   // 输出: 7
}
```

- 参数必须写类型；返回类型写在 `->` 后面，不写就是 `()`（相当于 C 的 `void`）。
- 函数可以定义在 `main` 之后，Rust 不要求先声明后使用。

### 1.1 表达式 vs 语句（本章最重要的一条分界线）

**语句**做一件事、没有值；**表达式**能算出值。Rust 里 `if`、`match`、`loop`、代码块 `{ ... }`
都有值，分号的作用就是"把表达式的值丢掉，把它变成语句"：

```rust
fn main() {
    // 代码块是表达式：值 = 最后一个不带分号的表达式
    let y = {
        let x = 3;
        x * x          // 没有分号 → 块的值是 9
    };
    println!("{y}");   // 输出: 9

    let z = {
        let x = 3;
        x * x;         // 这一行是语句，值是 ()
    };
    println!("{z:?}"); // 输出: ()
}
```

所以函数末尾多打一个分号，报错看起来像类型不对，其实是值被分号吃掉了：

```rust
// fn double(x: i32) -> i32 { x * 2; }   // ❌ error[E0308]: expected `i32`, found `()`
```

> ⚠️ **坑**：看到 `expected i32, found ()`，先检查"上一行是不是多了个分号"。
> 这是从 C 迁移过来最常犯的第一号错误。

## 2. `if` 是表达式

```rust
fn main() {
    let n = 7;

    // ✅ if 有值，可以直接绑到变量（不需要 C 的 ?: 三元）
    let parity = if n % 2 == 0 { "even" } else { "odd" };
    println!("{n} is {parity}");   // 输出: 7 is odd

    let label = if n < 0 { "负数" } else if n == 0 { "零" } else { "正数" };
    println!("{n} 是{label}");     // 输出: 7 是正数
}
```

两条规则：

1. **当表达式用时 `else` 不能省**：`let x = if c { 1 };` 编译不过——条件为假时编译器变不出 `i32`。
   当语句用时（`if c { do_it(); }`）不需要 `else`，因为不需要值。
2. **所有分支类型必须一致**：Rust 不允许一个绑定今天是 `i32` 明天是 `&str`。

条件本身必须是 `bool`，没有"非 0 即真"：

```rust
let x = 3;
// if x { }        // ❌ error[E0308]: expected `bool`, found integer
if x != 0 { }      // ✅ 显式比较

let maybe: Option<i32> = Some(3);
// if maybe { }    // ❌ 没有 truthy：Option 不能当真假用
if maybe.is_some() { }   // ✅
```

> ⚠️ **坑**：Rust 没有 `?:`，`if` 表达式就是它的替代品；`if (x = 1)` 这种手滑连写都写不出来。

## 3. `loop` / `while` / `for`

### 3.1 `loop` 可以 `break value`

`loop` 既是无限循环，也是**唯一能带值跳出**的循环：

```rust
fn main() {
    let mut n = 1u32;
    let mut steps = 0u32;

    let result = loop {
        if n >= 100 {
            break (n, steps);   // break 带一个值 → 整个 loop 表达式的值
        }
        n *= 2;
        steps += 1;
    };

    println!("{result:?}");     // 输出: (128, 7)
}
```

C 想从循环里带出两个结果，得用出参指针或标志变量；Rust 直接让 `break` 带出元组。
`while` / `for` 只能写 `break;`：

```rust
// let x = while true { break 1; };  // ❌ error[E0571]: `break` with a value can only be used in `loop`
```

### 3.2 `while` 与 `for`：遍历别用下标

```rust
fn main() {
    // while：条件与循环体相关、次数事先不知道
    let mut fuel = 3;
    while fuel > 0 {
        println!("fuel = {fuel}");
        fuel -= 1;
    }
    // 输出: fuel = 3 / fuel = 2 / fuel = 1（各占一行）

    // 遍历集合时，❌ while + 手写下标：忘写 i += 1 就死循环，还可能越界
    let v = vec![10, 20, 30];
    let mut i = 0;
    while i < v.len() {
        println!("{}", v[i]);
        i += 1;
    }

    // ✅ for：没有下标，边界由迭代器保证
    for x in &v {
        println!("{x}");
    }

    // 需要下标时才用 enumerate，它给出 (下标, 元素)
    for (idx, x) in v.iter().enumerate() {
        println!("v[{idx}] = {x}");
    }
}
// 输出: 10 20 30 / 10 20 30 / v[0] = 10、v[1] = 20、v[2] = 30（各占一行）
```

为什么 `for` 更好？

- **不可能越界**：`v[i]` 每次都要边界检查；迭代器自己推进下标，检查能被优化掉。
- **不用维护状态**：`i += 1` 忘写是经典 bug，`for` 里根本没这个变量。
- **意图清楚**：`for x in &v` 一眼看出"我只是读一遍所有元素"；clippy 也会用
  `clippy::needless_range_loop` 抓出 `for i in 0..v.len() { v[i] }`。

顺带记住三种迭代方式的区别（第 4 章细讲借用）：`for x in &v` 只读，
`for x in &mut v` 可以在循环里改元素（`*x += 1`），`for x in v` 会 move 掉整个集合。

### 3.3 循环标签：一次跳出多层

```rust
fn main() {
    let grid = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];

    'rows: for row in &grid {
        for &cell in row {
            if cell == 5 {
                println!("在 {row:?} 里找到 5");
                break 'rows;      // 连同内层一起跳出
            }
        }
    }
    // 输出: 在 [4, 5, 6] 里找到 5
}
```

标签写成 `'名字:` 加在循环前；`continue 'outer` 会**顺带结束当前内层循环**，
比 C 的 goto / 布尔标志干净得多；标签对 `loop` / `while` / `for` 都有效。

## 4. `match`：必须穷尽

`match` 和 C 的 `switch` 有四个关键区别：

| | C `switch` | Rust `match` |
|---|---|---|
| 覆盖不全 | 合法（悄悄跳过） | **编译错误**，必须穷尽 |
| fallthrough | 默认掉下去，要写 `break` | 不存在，分支彼此独立 |
| 有没有值 | 语句 | **表达式**，各分支类型必须统一 |
| 能匹配什么 | 整数 / 枚举常量 | 字面量、范围、元组、枚举、绑定、`\|` 组合 |

```rust
enum Coin { Penny, Nickel, Dime, Quarter }

fn value_in_cents(coin: &Coin) -> u32 {
    match coin {
        Coin::Penny => 1,
        Coin::Nickel => 5,
        Coin::Dime => 10,
        Coin::Quarter => 25,
    }   // 4 个变体都有分支 → 穷尽；少写一个就编译不过
}

fn main() {
    println!("{}", value_in_cents(&Coin::Quarter));   // 输出: 25
}
```

匹配整数时编译器无法证明你覆盖了全部取值，所以要自己补 `_`：

```rust
let score: u32 = 85;
let grade = match score {
    90..=100 => "A",
    80..=89 => "B",
    70..=79 => "C",
    _ => "F",            // 必须兜底，否则 non-exhaustive patterns
};
println!("{grade}");     // 输出: B
```

> ⚠️ **坑**：`_` 是"我确认剩余情况都归这里"。给枚举加新变体时，
> 编译器会替你找出所有漏掉的 `match`——这正是穷尽性检查的价值。

### 4.1 `if` 守卫（guard）：分支之上再加条件

守卫**不参与穷尽性判断**，而且**分支从上到下依次尝试，顺序即优先级**：

```rust
fn membership_level(years: u32, points: u32) -> &'static str {
    match (years, points) {
        (y, p) if y >= 5 || p >= 1000 => "钻石",
        (y, p) if y >= 3 || p >= 500 => "黄金",
        (y, p) if y >= 1 || p >= 100 => "白银",
        _ => "青铜",
    }
}

fn main() {
    println!("{}", membership_level(4, 0));     // 输出: 黄金
    println!("{}", membership_level(0, 1000));  // 输出: 钻石
    println!("{}", membership_level(1, 0));     // 输出: 白银
    println!("{}", membership_level(0, 99));    // 输出: 青铜
}
```

`(y, p)` 是**不可反驳模式**，它总能匹配，所以真正做判断的是守卫里的布尔表达式；
四个分支全带守卫时编译器不认为穷尽，`_ => "青铜"` 必须有。

> ⚠️ **坑**：返回 `&'static str` 时每个分支都得是字面量。
> `String::from("钻石")` 类型对不上；从 `String` 借出来的 `&str` 也不是 `'static`。

## 5. 常见坑（逐条自查）

1. **函数末尾多一个分号** → `expected i32, found ()`，删掉分号。
2. **`if` 当表达式用时漏 `else`** → `if may be missing an else clause`。
3. **分支类型不一致** → 每个分支必须算出同一类型，必要时统一成 `String`。
4. **在 `while` / `for` 里写 `break value`** → 带值的 `break` 只属于 `loop`。
5. **`while` 索引遍历忘写 `i += 1`** → 死循环；改用 `for x in &v` 从根上避免。
6. **`for` 遍历时改集合** → 借用冲突，编译期就拦下：

   ```rust
   // let mut v = vec![1, 2, 3];
   // for x in &v {                  // 持有 v 的不可变借用
   //     if *x == 2 { v.push(4); }  // ❌ cannot borrow `v` as mutable
   // }
   ```

   改成"先记录、循环结束后再改"，或改用下标（第 4 章展开）。
7. **`match` 少分支 / 所有分支都有守卫** → 补 `_ => ...`，别指望守卫帮你穷尽。
8. **整数溢出**：`3 * n + 1` 或累加在 debug 下溢出会 panic（release 下静默回绕）；
   本章 Collatz 练习注意 `n` 接近 `u64::MAX` 时装不下。

## 6. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 2-1 | FizzBuzz：被 3 和 5 整除 → `"FizzBuzz"`；只被 3 整除 → `"Fizz"`；只被 5 整除 → `"Buzz"`；否则返回 `n` 的十进制字符串 | `if` 表达式、判断顺序、`String` 与 `&str` | `fizzbuzz` |
| 2-2 | Collatz 步数：偶数除 2、奇数乘 3 加 1，返回到达 1 的步数；`0` 和 `1` 都返回 `0` | `loop` + `break`、提前 `return`、边界值防死循环 | `collatz_steps` |
| 2-3 | 会员等级：按 `years` / `points` 阈值顺序判定 `"钻石"` / `"黄金"` / `"白银"` / `"青铜"` | `match` 元组 + `if` 守卫、顺序敏感 | `membership_level` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch02_control_flow   # 卡住时用 python3 tools/ai_review.py ch02_control_flow --hint 2
```

### 提示（不给答案）

- **2-1**：先判断"同时被 3 和 5 整除"（等价 `n % 15 == 0`），再判断单独条件。
  顺序反了，`15` 会先命中 `% 3` 输出 `"Fizz"`。注意 `0 % 3 == 0` 也成立，
  所以 `fizzbuzz(0)` 应该返回 `"FizzBuzz"`。返回值是 `String`：
  普通数用 `n.to_string()`，字面量用 `"Fizz".to_string()`。
- **2-2**：`while current != 1 { ... }` 或 `loop { ... break; }` 都行，步数用 `u32` 累加。
  **关键**：进循环前先 `if n <= 1 { return 0; }`，否则 `n = 0` 时 `0 / 2` 还是 `0`，永远到不了 1。
  计数放在算出下一个 `current` 之后，别漏也别重复。
- **2-3**：`match (years, points)` + `(y, p) if ...` 守卫，最严的条件写最上面，
  第一个命中的分支直接生效。每个分支返回 `&'static str` 字面量（不要 `String`）。
  所有分支都带守卫时编译器不认为穷尽，必须补 `_ => "青铜"`。

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 2-1：先处理"同时整除"这个最强条件。每个分支都返回 String。
pub fn fizzbuzz(n: u32) -> String {
    if n % 15 == 0 {
        "FizzBuzz".to_string()
    } else if n % 3 == 0 {
        "Fizz".to_string()
    } else if n % 5 == 0 {
        "Buzz".to_string()
    } else {
        n.to_string()
    }
}

// 2-2：n = 0 和 n = 1 都在进循环前返回 0。
pub fn collatz_steps(n: u64) -> u32 {
    if n <= 1 {
        return 0; // 少了这一句，n = 0 会 0 -> 0 -> 0 永远死循环
    }

    let mut current = n;
    let mut steps = 0u32;

    while current != 1 {
        current = if current % 2 == 0 {
            current / 2
        } else {
            3 * current + 1
        };
        steps += 1;
    }

    steps
}

// 2-3：match 元组 + if 守卫，自上而下第一个命中的分支生效。
pub fn membership_level(years: u32, points: u32) -> &'static str {
    match (years, points) {
        (y, p) if y >= 5 || p >= 1000 => "钻石",
        (y, p) if y >= 3 || p >= 500 => "黄金",
        (y, p) if y >= 1 || p >= 100 => "白银",
        _ => "青铜",
    }
}
```

**2-1**：`n % 15 == 0` 与 `n % 3 == 0 && n % 5 == 0` 等价，少一次取模，但必须排在 `% 3` 前面。
**2-2**：这里"循环条件"和"循环体"是对齐的，`while` 比 `loop` + `break value` 更直白。
**已知限制**：`3 * current + 1` 在 `current` 接近 `u64::MAX` 时会溢出（debug 下 panic），
真实 Collatz 需要大整数；评测里用的是 `2^40` 这种安全值。

</details>

## 自测清单

- [ ] 我能说出"表达式"和"语句"的区别，以及分号在其中的作用
- [ ] 我知道 `fn f() -> i32 { x * 2; }` 为什么编译不过，怎么改
- [ ] 我能写出 `let x = if c { a } else { b };`，并解释 `else` 为什么不能省
- [ ] 我知道 `break value` 只能用在哪种循环里，以及它替掉了 C 的什么写法
- [ ] 我能说出三条"`for` 比 `while` 索引遍历更好"的理由
- [ ] 我会用循环标签一次跳出两层循环
- [ ] 我知道 `match` 分支从上到下匹配、`_` 为什么常常必须
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch02_control_flow` 全绿
