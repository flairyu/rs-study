# 第 8 章 · 闭包与迭代器

> **一句话定位**：闭包是「能记住环境的函数」，迭代器是「不会多算一步的流水线」——
> 这两样加起来，Rust 里 80% 的 `for` 循环都能删掉，而且不会变慢。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 3 章（所有权）、第 4 章（借用）、第 7 章（集合）
> **练习工程**：`exercises/ch08_iterators_closures`（3 题）

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一件事——**把列表里的偶数平方后求和**——在四门语言里长这样：

```c
// C：逻辑全揉在一个循环里；想抽成"回调"就得自己传函数指针 + void* 上下文
long sum_sq_even(const int *nums, size_t n) {
    long total = 0;
    for (size_t i = 0; i < n; i++)
        if (nums[i] % 2 == 0) { long x = nums[i]; total += x * x; }
    return total;
}
```

```python
# Python：生成器表达式，和 Rust 的链式写法几乎一一对应
def sum_of_squares_of_evens(nums):
    return sum(x * x for x in nums if x % 2 == 0)
```

```rust
// Rust：适配器链；和 Python 一样短，但它是编译期单态化的，没有回调开销
pub fn sum_of_squares_of_evens(nums: &[i32]) -> i64 {
    nums.iter().filter(|&&x| x % 2 == 0)
        .map(|&x| (x as i64) * (x as i64)).sum()
}
```

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 遍历 + 条件 + 变换 | 手写 `for`，逻辑揉在一起 | 生成器表达式 / 列表推导 | 适配器链 `filter().map().sum()` |
| 匿名函数 | C 只有函数指针；C++11 起有 lambda | `lambda x: x + 1` | 闭包 `\|x\| x + 1` |
| 捕获环境 | C 靠 `void*` 手工传上下文 | 自动捕获，`nonlocal` 改写外层 | 自动推断捕获方式，`move` 强制按值 |
| 惰性 | 无此概念 | 生成器惰性，列表推导立即求值 | **适配器全惰性**，直到遇见消费型方法 |
| 性能 | 手写最快 | 推导式有解释器开销 | 适配器链 ≈ 手写循环（内联 + 单态化） |
| 整数 | `int` / `long` 自己选 | 任意精度 | `i32` 平方会溢出，返回 `i64` 前要先转换 |

Python 使用者可以先记这个类比：**Rust 的迭代器就是 Python 的生成器**，
`.collect()` 对应 `list(...)`，`.sum()` 对应 `sum(...)`。再看闭包这一列：
Python 的 `lambda` / 嵌套函数**自动按引用捕获**，改外层变量要 `nonlocal`，
闭包能被调用几次完全靠你自觉；Rust 把这件事变成了**编译期类型**——
编译器自动推断捕获方式，并据此把闭包归到 `Fn` / `FnMut` / `FnOnce` 之一（第 2 节），
调用方在签名里一眼就能看出「这个闭包会被调用几次」。
另外两个关键差别：Rust 的链子**编译期单态化**，没有逐元素函数调用开销；
每个闭包会被归到 `Fn` / `FnMut` / `FnOnce` 之一，这决定了它能否被多次调用。

## 1. 闭包：语法与类型推断

闭包 = **匿名函数 + 捕获环境**：

```rust
let add_one = |x: i32| -> i32 { x + 1 };   // 完整写法
let add_two = |x| x + 2;                   // 类型通常可推断（也常写成 |x: i32| x + 2）
let shout = || println!("hi");             // 无参数
println!("{} {}", add_one(1), add_two(1)); // 输出: 2 3
```

和 `fn` 的三个区别：

| | `fn` 函数项 | 闭包 |
|---|---|---|
| 能捕获外层变量吗 | ❌ 不能 | ✅ 能 |
| 参数/返回类型 | 必须写 | 通常可推断 |
| 类型 | 唯一的函数项类型，可强转成 `fn` 指针 | 每个闭包一个**匿名唯一类型**，不能强转 |

**反直觉之处**：闭包只在**第一次被调用**时确定参数类型，之后不能改：

```rust
let id = |x| x;
let a = id(1);        // 这里确定了参数是 i32
// let b = id("hi");  // ❌ error[E0308]: expected `i32`, found `&str`
```

> ⚠️ **坑**：同一个闭包变量不能既接 `i32` 又接 `&str`；闭包不是泛型的，
> 要泛型就用泛型函数（第 9 章）。

闭包类型没有名字，只能用**泛型参数**或 `impl Trait` 接收：

```rust
fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 { f(x) }
println!("{}", apply(|v| v * 3, 4));   // 输出: 12
```

### 1.1 捕获：按需借，不按需搬

编译器为每个闭包挑**刚好够用**的捕获方式：不可变借用 → 可变借用 → 拿走所有权。

```rust
let name = String::from("rust");
let show = || println!("{name}");   // 只读 → 不可变借用，闭包是 Fn
show(); show();                     // ✅ 可以调用任意多次
println!("{name}");                 // ✅ 借用已结束，name 还能用

let mut count = 0;
let mut inc = || count += 1;        // 要改 → 可变借用，闭包是 FnMut（注意 inc 得是 mut）
inc(); inc();
println!("{count}");                // 输出: 2
```

## 2. 三种闭包 trait：`Fn` / `FnMut` / `FnOnce`

三者是**继承**关系，`Fn: FnMut: FnOnce`——读作「`Fn` 也是 `FnMut`，`FnMut` 也是 `FnOnce`」。
所以 **`Fn` 要求最严**（承诺能重复调用），**`FnOnce` 最松**（只承诺能调用一次）。
约束越严，能接收的闭包范围越窄：

- 参数写 `F: FnOnce()` → 什么闭包都能传，但你最多调用 `f` **一次**；
- 参数写 `F: Fn()` → 只有可重复调用的闭包能传，换来想调几次就调几次。

| trait | 调用时怎么用捕获值 | 能调几次 | 典型场景 |
|---|---|---|---|
| `Fn` | 只读（`&self`） | 任意多次 | 判断条件、`map` 的变换 |
| `FnMut` | 可变借用（`&mut self`） | 任意多次（串行） | 累加器、计数器、带状态的过滤 |
| `FnOnce` | 拿走所有权（`self`） | **恰好一次** | `move` 走一个 `String`、把值塞进容器 |

```rust
fn call_twice<F: Fn() -> String>(f: F) -> (String, String) { (f(), f()) }  // 要调两次 → Fn
fn call_once<F: FnOnce() -> String>(f: F) -> String { f() }                // 只调一次 → 最宽松

let a = call_twice(|| String::from("hi"));    // 非捕获闭包：Fn
println!("{a:?}");                            // 输出: ("hi", "hi")

let owned = String::from("bye");
let b = call_once(move || owned);             // move + 返回捕获值 → FnOnce
println!("{b}");                              // 输出: bye
```

### 2.1 `move` 闭包：捕获方式 ≠ 调用能力

```rust
let greeting = String::from("hello");
let by_move = move || println!("{greeting}");  // 捕获方式是"搬走"
by_move(); by_move();                          // ✅ 只读使用，所以仍然是 Fn
// println!("{greeting}");                     // ❌ 所有权已经归闭包了
```

**结论：`move` 决定「怎么捕获」，`Fn`/`FnMut`/`FnOnce` 决定「怎么使用」。**
只有搬进来的值被**消耗**（移出去、或原地改）时，闭包才退化成 `FnOnce`。

> ⚠️ **坑（最容易踩）**：`move` 捕获了非 `Copy` 的值（比如 `String`）并把它返回或塞进容器，
> 闭包就**只能调用一次**。想多次调用：别 `move`、只借用，或者克隆一份进去。

## 3. 迭代器的惰性：不消费，不工作

`Iterator` 只有一个必须实现的方法：

```rust
pub trait Iterator {
    type Item;                                  // 每次产出什么类型
    fn next(&mut self) -> Option<Self::Item>;   // 没有下一个就返回 None
    // 其余几十个方法（map / filter / fold / ...）都有默认实现
}
```

**适配器只是把旧迭代器包起来，什么都不会算；只有消费型方法才驱动 `next`。**

```rust
let v = vec![1, 2, 3];
let lazy = v.iter().map(|x| { println!("计算 {x}"); x * 2 });  // 只构造，不执行
println!("还没开始算");                                        // 先打印这一行
let doubled: Vec<i32> = lazy.collect();                        // 这时才打印"计算 1/2/3"
println!("{doubled:?}");                                       // 输出: [2, 4, 6]
```

惰性带来两个实际好处：

- **不会多算**：`(0..).take(3)` 合法——无限迭代器被消费到 3 个就停；
- **可以短路**：`.any()` / `.find()` / `.position()` 拿到答案就停止 `next`。

顺带看清 `for` 循环的真相（`for x in &v` ≈ `let mut it = v.iter(); while let Some(x) = it.next()`），
它只是迭代器的语法糖。

### 3.1 `&T` 与 `T`：三种迭代方式

| 写法 | `Item` 类型 | 迭代器本身 | 原集合还能用吗 |
|---|---|---|---|
| `v.iter()` | `&T` | `Iter<'_, T>` | ✅ 借用结束后可用 |
| `v.iter_mut()` | `&mut T` | `IterMut<'_, T>` | ✅ 结束后可用 |
| `v.into_iter()` | `T` | `IntoIter<T>` | ❌ 被消费掉了 |

`for x in v` 用 `into_iter()`（`v` 被 move）；`for x in &v` 用 `iter()`。
`HashMap` / `BTreeMap` 的 `iter()` 产出 `(&K, &V)`。

## 4. 常用适配器

**消费型（consuming）**——驱动迭代器，返回一个值：

| 方法 | 作用 | 返回 |
|---|---|---|
| `collect()` | 收集成集合（`Vec` / `HashMap` / `String` …） | 依赖目标类型 |
| `sum()` / `product()` | 求和 / 求积 | 数值 |
| `count()` | 数元素个数 | `usize` |
| `fold(init, f)` | 从初值开始逐个累积，最通用的「归约」 | 累积值 |
| `for_each(f)` | 对每个元素执行副作用 | `()` |
| `any(p)` / `all(p)` | 有任意一个 / 全部满足条件（短路） | `bool` |
| `position(p)` | 第一个满足条件的下标 | `Option<usize>` |
| `find(p)` | 第一个满足条件的元素 | `Option<Item>` |
| `max()` / `min()` | 最大 / 最小（要求 `Ord`） | `Option<Item>` |
| `last()` / `nth(n)` | 最后一个 / 第 n 个 | `Option<Item>` |

**适配型（adapting）**——返回新迭代器，仍然惰性：

| 方法 | 作用 | 备注 |
|---|---|---|
| `map(f)` | 一对一变换 | `f: FnMut(Item) -> U` |
| `filter(p)` | 保留满足条件的 | `p: FnMut(&Item) -> bool`（注意是**引用**） |
| `filter_map(f)` | 变换 + 过滤合一 | `f` 返回 `Option<U>` |
| `flat_map(f)` | 变换后摊平 | 适合「每个元素变成一段」 |
| `take(n)` / `skip(n)` | 取前 n 个 / 跳过前 n 个 | 无限迭代器的救星 |
| `take_while` / `skip_while` | 条件版的 take / skip | 一旦不满足就停 |
| `zip(other)` | 两个迭代器配对 | 长度不同时取短的 |
| `enumerate()` | 附加下标 | 产出 `(usize, Item)` |
| `chain(other)` | 首尾相接 | 两个迭代器串成一个 |
| `rev()` | 反向 | 要求 `DoubleEndedIterator` |
| `step_by(n)` | 每隔 n 个取一个 | 从第 0 个开始 |
| `inspect(f)` | 调试用旁路观察 | 不改元素，`f: FnMut(&Item)` |

串一遍（每行后面的注释就是输出）：

```rust
let nums = vec![1, 2, 3, 4, 5, 6];
let evens: Vec<i32> = nums.iter().copied().filter(|x| x % 2 == 0).collect(); // [2, 4, 6]
let total: i32 = nums.iter().sum();                                          // 21
let product = nums.iter().fold(1, |acc, x| acc * x);                         // 720
let pairs: Vec<(usize, i32)> = nums.iter().copied().enumerate().collect();   // [(0,1), (1,2), ...]
let zipped: Vec<i32> = nums.iter().copied().zip(nums.iter().copied().skip(2))
    .map(|(a, b)| a + b).collect();                                          // [3, 5, 7, 9, 11]
let chained: Vec<i32> = (1..=2).chain(10..=11).collect();                    // [1, 2, 10, 11]
nums.iter().any(|&x| x > 5);                                                 // true
nums.iter().position(|&x| x == 4);                                           // Some(3)
```

`filter` 的闭包收到的是 `&Item`：`nums.iter()` 的 `Item` 是 `&i32`，
所以 `filter` 实际收到 `&&i32`。三种处理方式里，`.copied()` 最好读（要求 `T: Copy`），
`|&&x|` 零成本但要数清楚几个 `&`，`|x| **x` 最啰嗦但最不容易看错。

> ⚠️ **坑**：`filter` 的闭包参数是**引用**，`map` 的闭包参数是**元素本身**。
> 搞混这两者是本章编译错误的第一大来源。

## 5. 消费型 vs 适配型：链子什么时候真的开始跑

规则只有一句：**链上最后一个方法决定一切。**

- 结尾是 `collect` / `sum` / `count` / `fold` / `for_each` / `any` …：整条链开始工作；
- 结尾是 `map` / `filter` / `take` …：什么都没发生，你只是造了个迭代器对象。

```rust
v.iter().map(|x| x * 2);        // ⚠️ 能编译，但触发 unused_must_use 警告：真的什么都没做
let _ = v.iter().map(|x| x * 2); // 加 let _ 只是让警告消失，依然没做
let done: Vec<i32> = v.iter().map(|x| x * 2).collect();   // ✅ 这里才真的算了
```

`Iterator` 被标注了 `#[must_use]`，所以「造了迭代器却没用」只是警告不是错误——
**不要用 `let _ =` 糊弄过去**，那通常意味着你忘了 `.collect()`。

`fold` 是所有消费型方法的「母方法」：`nums.iter().fold(0, |acc, x| acc + x)` 等价于
`nums.iter().sum::<i32>()`，`fold(0, |acc, _| acc + 1)` 等价于 `count()`。

什么时候用 `for`、什么时候用链子：有**提前退出**（复杂条件的 `break`）就用 `for` 或
`any` / `find`；每轮做多件不相关的事（打印 + 累加 + push）时 `for` 更清楚；
纯粹「过滤 → 变换 → 汇总」用链子，通常更短且不慢。

## 6. 返回迭代器：`impl Iterator`

闭包类型没有名字，所以函数**不能把具体迭代器类型写进返回类型**，要用 `impl Trait`：

```rust
// 返回一条「会产出偶数平方」的链子；调用方看不见它具体是什么类型
fn even_squares(nums: &[i32]) -> impl Iterator<Item = i64> + '_ {
    nums.iter().filter(|&&x| x % 2 == 0)
        .map(|&x| (x as i64) * (x as i64))
}
let v: Vec<i64> = even_squares(&[1, 2, 3, 4]).collect();   // [4, 16]
```

两个要点：

1. **`+ '_` 是必需的**：迭代器借用了 `nums`，返回类型必须声明这个生命周期（第 10 章）。
   如果迭代器拥有自己的数据（比如产出 `1..=10`），就不需要。
2. **每个返回位置只能有一个具体类型**，所以「按条件返回不同链子」编译不过：

```rust
// ❌ error[E0308]: `if` and `else` have incompatible types
// fn pick(nums: &[i32], only_even: bool) -> impl Iterator<Item = i32> {
//     if only_even { nums.iter().copied().filter(|x| x % 2 == 0) }
//     else { nums.iter().copied() }
// }
```

绕过去的三种办法：`Box<dyn Iterator<Item = i32>>`（一次动态分发开销）、
把两条分支统一成同一种链子（比如让 `filter` 的闭包一直存在）、
或者自定义一个 `Either` 风格的枚举。

## 7. 常见坑

| 坑 | 现象 | 正确做法 |
|---|---|---|
| 忘了消费 | `unused_must_use` 警告，代码「没反应」 | 结尾加 `.collect()` / `.count()` / `for_each()` |
| `filter` 参数是引用 | `expected &i32, found i32` | 写 `\|&&x\|` 或先 `.copied()` |
| 平方溢出 | debug 下 `attempt to multiply with overflow` | 先 `as i64` **再**相乘，别最后才转 |
| `filter` 后拿不到所有权 | `cannot move out of borrowed content` | `filter` 只给引用；要拿值就在 `map` 里 `cloned()` / `copied()` |
| `move` 闭包只能调一次 | `use of moved value` | 闭包只借用，或克隆，或确实只需调一次 |
| `collect()` 类型不明确 | `type annotations needed` | 写 `let v: Vec<_> = ....collect()` 或 `.collect::<Vec<_>>()` |
| 边遍历边改集合 | 借用冲突 | 先 `collect()` 出要改的内容，再分阶段处理 |
| `iter()` vs `into_iter()` | 元素类型差一个 `&` | 记住：`iter()` → `&T`，`into_iter()` → `T` |

> ⚠️ **坑（性能直觉）**：`.map().filter().map()` 这种长链**不会**产生中间集合，
> 也没有虚函数调用，它被编译成一趟循环，和手写 `for` 差不多快。
> 「为了性能不写迭代器」在 Rust 里通常是错误的理由——先写清楚，再测量。

## 8. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 8-1 | 计算切片里所有偶数的平方和（返回 `i64`） | `filter` + `map` + `sum`、`&i32` 解构、溢出 | `sum_of_squares_of_evens` |
| 8-2 | 把闭包在同一个值上迭代应用 `n` 次 | 泛型 `F: Fn(i32) -> i32`、闭包可多次调用 | `apply_n_times` |
| 8-3 | 统计每种「单词长度」出现了多少次 | 迭代器 + `entry`、`BTreeMap` 升序 key | `count_by_length` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch08_iterators_closures   # 卡住了：python3 tools/ai_review.py ch08_iterators_closures --hint 2
```

### 提示（不给答案）

- **8-1**：`nums.iter()` 产出 `&i32`。三选一：闭包写 `|&&x|`、先 `.copied()`、或写 `|x| **x`。
  乘之前一定先 `x as i64`——测试里有一个值会让 `i32` 平方溢出。
- **8-2**：`let mut acc = x; for _ in 0..n { acc = f(acc); } acc`。
  `n` 是 `u32`，`0..n` 在 `n == 0` 时是空区间，天然不用特判；`acc` 要 `mut`。
- **8-3**：两种写法都对：`.map(|w| (w.len(), 1)).collect()`；
  或者 `let mut m = BTreeMap::new(); for w in words { *m.entry(w.len()).or_insert(0) += 1; }`。
  别漏了空字符串——它的长度 `0` 也要进 map。

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
use std::collections::BTreeMap;

// 8-1：整条链都是惰性的，.sum() 才驱动它跑起来。
// filter 的闭包收到 &&i32，用 |&&x| 一次解掉两层引用；map 的闭包收到 &i32，用 |&x| 解一层。
// 关键：(x as i64) * (x as i64) —— 先转 i64 再平方。
// 写成 (x * x) as i64 就晚了：46340 的平方会先在 i32 里溢出。
pub fn sum_of_squares_of_evens(nums: &[i32]) -> i64 {
    nums.iter().filter(|&&x| x % 2 == 0)
        .map(|&x| (x as i64) * (x as i64)).sum()
}

// 8-2：F: Fn(i32) -> i32 表示「f 可以被调用任意多次」，正好是循环需要的。
// 写 FnOnce 就编译不过——它只承诺能调用一次。
// n 是 u32，0..n 在 n == 0 时为空区间，直接返回 x，无需特判。
pub fn apply_n_times<F: Fn(i32) -> i32>(f: F, x: i32, n: u32) -> i32 {
    let mut acc = x;
    for _ in 0..n { acc = f(acc); }
    acc
}

// 8-3：words.iter() 产出 &&str，w.len() 会自动解引用到 str::len。
// map 成 (长度, 1) 后 collect 成 BTreeMap，相同 key 的 value 会自动累加。
// 用 BTreeMap 而不是 HashMap：key 天然升序，测试可以直接比 keys()。
pub fn count_by_length(words: &[&str]) -> BTreeMap<usize, usize> {
    words.iter().map(|w| (w.len(), 1)).collect()
}

// 8-3 的循环版（完全等价）：
// let mut counts = BTreeMap::new();
// for w in words { *counts.entry(w.len()).or_insert(0) += 1; }
// counts
```

**为什么 8-1 返回 `i64` 而不是 `i32`？** 平方会让数量级翻倍：`46341² ≈ 2.147×10⁹`
已经超过 `i32::MAX`（2147483647）。Rust 的整数**不会自动升级**，
所以「先转换、再运算」要刻进肌肉记忆（第 1 章讲过 `as` 的截断语义）。

**为什么 8-2 用 `Fn` 而不是 `FnMut`？** `Fn` 更严，换来「能重复调用」的保证；
`Fn` 的闭包也自动满足 `FnMut`，所以调用方不受影响。我们不改捕获状态，`Fn` 最贴切。

</details>

## 自测清单

- [ ] 我能写出闭包语法，并说出它和 `fn` 的两点区别
- [ ] 我能解释为什么 `let f = |x| x; f(1); f("a");` 编译不过
- [ ] 我能说出 `Fn` / `FnMut` / `FnOnce` 的区别，以及参数该选哪个
- [ ] 我知道 `move` 闭包在什么情况下会退化成只能调用一次
- [ ] 我能解释「迭代器是惰性的」，并说出哪一类方法会真正驱动它
- [ ] 我能默写 `map` / `filter` / `fold` / `sum` / `collect` / `take` / `skip` / `zip` / `enumerate` / `chain` 的用途
- [ ] 我知道 `filter` 的闭包收到引用、`map` 的闭包收到元素，并能处理 `&&T`
- [ ] 我知道要用 `impl Iterator<Item = T> + '_` 返回迭代器，以及它的两个限制
- [ ] 我完成了 3 道练习且 `python3 tools/check.py ch08_iterators_closures` 全绿
