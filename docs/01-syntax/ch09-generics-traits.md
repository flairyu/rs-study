# 第 9 章 · 泛型与 trait

> **一句话定位**：Rust 用「泛型 + trait bound」同时取代了 C++ 的模板与继承、以及 Python 的隐式鸭子类型——
> 而且这些都在**编译期**结算。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 5 章（结构体与方法）、第 8 章（闭包与迭代器）
> **练习工程**：`exercises/ch09_generics_traits`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

先记住一句话：**Rust 没有继承**。没有 `class Dog : Animal`，没有 `virtual`，没有 `super()`。
代码复用只有两条路：**泛型**（编译期，静态分发）和 **trait 对象**（运行期，动态分发）。
trait 描述的是「这个类型**能做什么**」，而不是「它**是谁的子类**」。

```cpp
// C++：两套机制，语法互不相干
template <typename T> T largest(const std::vector<T>& v);   // 编译期，模板
struct Shape { virtual double area() const = 0; };          // 运行期，虚函数
```
```python
# Python：运行时鸭子类型 —— 只要能 .area() 就行，没有任何声明
def total_area(shapes):
    return sum(s.area() for s in shapes)
```
```rust
// Rust：一套 trait 同时撑起两种分发方式
fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> { /* 编译期单态化 */ }
fn total_area(shapes: &[Box<dyn Shape>]) -> f64 { /* 运行期查虚表 */ }
```

| 话题 | C++ | Python | Rust |
|---|---|---|---|
| 复用方式 | 模板 + 继承/虚函数 | 继承 + 鸭子类型 | **泛型 + trait**（无继承） |
| 编译期多态 | `template<T>` | 无 | 泛型 / `impl Trait`（单态化） |
| 运行期多态 | `virtual` + 虚表 | 天然动态 | `dyn Trait` + 胖指针 |
| 约束检查 | C++20 concepts，之前靠报错猜 | 运行时 `AttributeError` | `T: Trait` **编译期**检查 |
| 接口 / 默认实现 | 抽象基类；基类方法 | ABC；基类方法 | `trait`；trait 默认方法 |
| 给别人的类型加行为 | ADL / 自由函数 | 猴子补丁 | **孤儿规则**（见第 9 节） |

两个直接后果：**① 没有继承 = 没有菱形继承、没有 fragile base class**，共享行为抽 trait、共享数据用组合；
**② trait bound 让「能做什么」写进签名**，读 `fn f<T: Hash + Eq>(...)` 就知道 `T` 必须可哈希——
Python 里这件事要等运行时才知道。

---

## 1. 泛型函数与泛型结构体

泛型函数把「类型」变成参数：

```rust
fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    let mut max = items.first().copied()?;
    for item in items.iter() {
        if *item > max {
            max = *item;
        }
    }
    Some(max)
}

fn main() {
    println!("{:?}", largest(&[1, 5, 3]));          // Some(5)
    println!("{:?}", largest(&["a", "c", "b"]));    // Some("c")
    println!("{:?}", largest::<i32>(&[]));          // None
}
```

- `<T: PartialOrd + Copy>` 是 **trait bound**。C++ 模板是「先编译，错了再说」；Rust 要求**先声明约束，编译器才有资格用 `>`**。
- `items: &[T]` 里的元素是**借来的**，不能 move 走，所以要么 `T: Copy`（复制），要么返回 `Option<&T>`。
- 用 `PartialOrd` 而不是 `Ord`：`f64` 因为 `NaN` 只是**偏序**；用 `Ord` 会拒绝 `largest(&[1.0, 2.0])`。
- `largest::<i32>(&[])` 里的 `::<i32>` 叫 **turbofish**：推导不出 `T` 时显式指定。

泛型结构体／枚举同理，`<T>` 写在类型名后：`struct Pair<T> { first: T, second: T }`，`impl<T: std::fmt::Display> Pair<T> { /* ... */ }`。
`Option<T>`、`Vec<T>`、`Result<T, E>` 你早就在用了。

---

## 2. trait 定义与 impl

trait 是一组「类型承诺提供的行为」：定义用 `trait`，实现用 `impl Trait for Type`。

```rust
trait Area {
    fn area(&self) -> f64;
}

struct Circle { r: f64 }
struct Rect { w: f64, h: f64 }

impl Area for Circle {
    fn area(&self) -> f64 { std::f64::consts::PI * self.r * self.r }
}
impl Area for Rect {
    fn area(&self) -> f64 { self.w * self.h }
}

fn total_area<T: Area>(shapes: &[T]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

fn main() {
    println!("{:.4}", Circle { r: 1.0 }.area());    // 3.1416
    println!("{}", Rect { w: 2.0, h: 3.0 }.area()); // 6
}
```

对照 C++：抽象基类/纯虚函数 ≈ `trait`，继承并 override ≈ `impl Trait for Type`，
而虚函数表只有你用 `dyn Trait` 时才存在。

> ⚠️ **坑**：调用 trait 方法时 **trait 必须在作用域里**（`use` 进来，或同 crate 可见）。
> 否则报 `no method named 'area' found for struct 'Circle'`——方法明明实现了，却因为 trait 没导入而看不见。

---

## 3. trait bound 与 where 子句

约束少时写在尖括号里，一多就换 `where`：

```rust
fn render<T, U>(v: T, label: U) -> String
where
    T: std::fmt::Display + Clone,
    U: Into<String>,
{
    format!("{}: {}", label.into(), v)
}
```

`where` 与尖括号里的 bound **完全等价**，只是排版。Rust 没有 C++ `requires` 那种自由度，
bound 必须写全——换来的是错误信息直接告诉你「`T` 不满足 `Ord`」。
另外记住：`{}` 要 `Display`，`{:?}` 要 `Debug`，**用哪个格式化符就要哪个 bound**，
这是初学者最常见的编译错误。

---

## 4. 默认方法

trait 的方法可以有默认实现，实现类型只写它关心的部分：

```rust
trait Greet {
    fn name(&self) -> String;

    fn greet(&self) -> String {                 // 默认实现，可以白拿，也可以覆盖
        format!("你好，{}！", self.name())
    }
}

struct Cat;
impl Greet for Cat {
    fn name(&self) -> String { "咪咪".to_string() }
    fn greet(&self) -> String { format!("喵～我是{}", self.name()) }
}

fn main() {
    println!("{}", Cat.greet());   // 喵～我是咪咪
}
```

默认方法是「接口 + 缺省实现」二合一，对应 Python 的 mixin、C++ 基类的非虚函数。
区别是：**编译期静态分发**，不产生虚表（除非你用 `dyn`）。

---

## 5. trait 作为参数：`impl Trait`

`impl Trait` 是 trait bound 的语法糖，适合只有一个泛型参数的场景：

```rust
fn show(value: impl std::fmt::Display) -> String { format!("{value}") }
// 等价于 fn show<T: std::fmt::Display>(value: T) -> String
```

| 写法 | 什么时候用 |
|---|---|
| `fn f<T: Trait>(x: T)` | 函数体里要用到 `T` 这个名字，或调用方要 turbofish 指定 |
| `fn f(x: impl Trait)` | 只是「要个实现了 Trait 的东西」，更短 |

> ⚠️ **坑**：`fn f(x: impl Trait, y: impl Trait)` 里两个 `impl Trait` 是**两个不同的匿名类型**，
> 不会因为写法相同就同类。要两个参数同类型，只能写具名泛型 `fn f<T: Trait>(x: T, y: T)`。

---

## 6. 返回 `impl Trait`

返回位置也能写 `impl Trait`：「我返回一个实现了 Trait 的东西，具体类型不告诉你」。

```rust
fn evens(nums: &[i32]) -> impl Iterator<Item = i32> + '_ {
    nums.iter().copied().filter(|n| n % 2 == 0)
}
```

这是零成本的：返回类型在编译期是确定的**具体类型**（`Filter<...>`），只是调用方看不见。
和 C++ 的 `auto` 返回类型思路相同，但 Rust 在签名处就承诺了「它至少是个 `Iterator`」。

限制有两条：**只能返回一种具体类型**（两个 `if` 分支返回不同迭代器 → 编译错误，
要么 `Box<dyn Iterator<Item = i32>>`，要么用枚举包一层）；返回的迭代器常常借用了参数，
需要写成 `impl Iterator<Item = i32> + '_`（生命周期见第 10 章）。

---

## 7. `dyn Trait` 与动态分发

泛型是「编译期把类型填进去」，`dyn Trait` 是「运行期查表找方法」。
想在同一个 `Vec` 里装**不同**的实现类型，就必须用 `dyn`：

```rust
trait Shape { fn area(&self) -> f64; }

struct Circle { r: f64 }
struct Square { side: f64 }

impl Shape for Circle { fn area(&self) -> f64 { 3.0 * self.r * self.r } }
impl Shape for Square { fn area(&self) -> f64 { self.side * self.side } }

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![          // 两种具体类型混装
        Box::new(Circle { r: 1.0 }),
        Box::new(Square { side: 2.0 }),
    ];
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    println!("{total}");                             // 7
}
```

必须 `Box` 的原因：`Circle` 与 `Square` 大小不同，而 `Vec<T>` 要求元素等长。
`Box<dyn Shape>` 是**胖指针**（数据指针 + 虚表指针），占两个机器字。

| | 泛型 `T: Trait` / `impl Trait` | `dyn Trait` |
|---|---|---|
| 分发时机 | 编译期（单态化） | 运行期（查虚表） |
| 运行时开销 | 零 | 一次间接调用，不能内联 |
| 二进制体积 | 每种类型一份 | 一份 |
| 混装不同类型 | ❌ | ✅ |
| 需要指针吗 | 不需要 | 需要（`&dyn` / `Box<dyn>` / `Arc<dyn>`） |
| 类比 | C++ 模板 | C++ `virtual` |

选型一句话：**类型在编译期固定、追性能 → 泛型；要在集合里混装、或做插件/回调接口 → `dyn`。**

---

## 8. 单态化与零成本抽象

编译时，Rust 会**为每种用到的具体类型各生成一份代码**，这叫**单态化（monomorphization）**：
`largest(&[1, 5, 3])` 生成 `largest_i32`，`largest(&["a", "b"])` 生成 `largest_str`。

后果（和 C++ 模板一样）：运行时**零**泛型开销，`largest(&[1, 5, 3])` 与手写 `i32` 版本一样快；
代价是**二进制膨胀**——用 10 种类型调用就生成 10 份，C++ 编译慢、体积大的根源相同。
另一个副作用：错误发生在**实例化时**，所以报错会同时指向泛型函数定义与调用点。

`dyn Trait` 正好反过来：一份代码，但每次调用多一次指针跳转、无法内联。
这就是「零成本抽象」的准确含义——**你不为没用到的东西付费；用了 `dyn`，就付它的钱。**

---

## 9. 孤儿规则与一致性

孤儿规则（orphan rule）：**`impl Trait for Type` 里，`Trait` 与 `Type` 至少要有一个是当前 crate 定义的。**

```rust
trait MyTrait { fn hello(&self) -> String; }

// ✅ 本地 trait，外部类型
impl MyTrait for Vec<i32> { fn hello(&self) -> String { format!("{} 个", self.len()) } }

// ✅ 外部 trait，本地类型
impl std::fmt::Display for Circle { /* ... */ }

// ❌ 两个都是外部的 → error[E0117]
// impl std::fmt::Display for Vec<i32> { ... }
```

为什么这么严？为了**一致性（coherence）**：同一组 `(Trait, Type)` 全程序只能有一个实现，
编译器才能确定 `x.foo()` 调用谁。若允许给外部类型实现外部 trait，两个 crate 可能各写一份、
在第三个 crate 里撞车——C++ 的 ODR 违规在同一位置，但那边是链接期 UB，这边直接编译不过。

想「给别人的类型加方法」：**定义自己的 trait**，再 `impl MyTrait for ForeignType`。

---

## 10. 常用内置 trait 一览

| trait | 提供什么 | 常用场景 |
|---|---|---|
| `std::fmt::Display` | `{}` 格式化 | 给用户看的输出 |
| `std::fmt::Debug` | `{:?}` / `{:#?}` | 调试、`assert_eq!` 失败信息 |
| `Clone` | `.clone()` 显式深拷贝 | 要「再来一份」 |
| `Copy` | 赋值/传参按位复制，原值仍可用 | `i32`、`f64`、`&T`、`bool` |
| `PartialEq` / `Eq` | `==` / `!=`；`Eq` 是全等价（`f64` 只有 `PartialEq`） | 比较、`HashMap` key 的前置条件 |
| `PartialOrd` / `Ord` | `<` `>` `sort()`；`Ord` 提供 `cmp` 全序 | 排序、求最大最小 |
| `Hash` | 可哈希 | `HashMap` / `HashSet` 的 key |
| `Default` | `Default::default()` | `#[derive(Default)]`、结构体更新语法 |
| `From` / `Into` | 无损转换、`?` 的错误类型转换 | `String::from`、`u8 → u32` |
| `Iterator` | `next()` 与 `map`/`filter`/`collect` 等 | 第 8 章的全部内容 |

两条实用规律：

1. `#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]` 能自动生成大部分实现，字段都满足条件时优先派生。
2. 派生会**给泛型参数自动加 bound**：`#[derive(PartialEq)] struct W<T> { v: T }` 只在 `T: PartialEq` 时能用；`Copy` 与 `Drop` 则互斥。

---

## 11. 常见坑

1. **从借来的切片里「移出」元素**：`fn take_first<T>(items: &[T]) -> T { items[0] }` 编译不过，
   加 `T: Copy` 后写 `items[0]`，或改用 `items[0].clone()`。
2. **`dyn Trait` 必须有间接层**：`fn f(x: dyn Trait)` 非法（unsized），要 `&dyn Trait` / `Box<dyn Trait>`。
3. **`Box<dyn Trait>` 默认隐含 `'static`**：装借用数据要写 `Box<dyn Trait + '_>`。
4. **trait 没 `use` 进来，方法就找不到**：报 `no method named ... found`，先查 `use`。
5. **派生 `Debug` 会传染 bound**：`#[derive(Debug)] struct W<T> { v: T }` 只在 `T: Debug` 时可打印。
6. **`impl Trait` 返回值只能一种类型**：两个分支返回不同迭代器 → 换 `Box<dyn Iterator<Item = _>>`。
7. **`f64` 用 `{}` 打印不补 `.0`**：`format!("{}", -2.0_f64)` 是 `"-2"`，`1.0` 是 `"1"`。
8. **想给外部类型加方法**：`impl Display for Vec<i32>` 永远写不了，定义自己的 trait。

---

## 12. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 9-1 | 求切片最大值，空切片返回 `None` | 泛型函数、`PartialOrd + Copy`、`Option` | `largest` |
| 9-2 | 让 `Point` 实现 `Describable`，输出 `Point(1.5, -2)` | trait 定义与实现、`{}` 打印 `f64` | `Describable` / `Point` |
| 9-3 | 把一组 trait 对象描述成 `Vec<String>` | `Box<dyn Trait>`、动态分发、自动解引用 | `describe_all` |

要覆盖的边界：空切片 → `None`、单元素、元素全部相等、`i32` 与 `&str` 都能调用 `largest`、
空的 `&[Box<dyn Describable>]` → 空 `Vec`。

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch09_generics_traits      # 卡住了：python3 tools/ai_review.py ch09_generics_traits --hint 2
```

### 提示（不给答案）

- **9-1**：`items.iter()` 迭代出的是 `&T`；`items.first().copied()?` 能在空切片时直接把 `None` 抛出去。
- **9-2**：`format!("Point({}, {})", self.x, self.y)`，不要加 `:.1`，也不要手动补 `.0`。
- **9-3**：`items.iter().map(|item| item.describe()).collect()`，`&[]` 自然得到空 `Vec`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 9-1：first().copied() 把 Option<&T> 变成 Option<T>，空切片时 ? 直接返回 None。
// 循环里 item 是 &T，*item > max 比较的是 T 的值（T: PartialOrd + Copy）。
pub fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    let mut max = items.first().copied()?;
    for item in items.iter() {
        if *item > max {
            max = *item;
        }
    }
    Some(max)
}

// 9-2：{} 打印 f64 取最短表示：1.5 → "1.5"，-2.0 → "-2"，1.0 → "1"。
pub trait Describable {
    fn describe(&self) -> String;
}

impl Describable for Point {
    fn describe(&self) -> String {
        format!("Point({}, {})", self.x, self.y)
    }
}

// 9-3：Box<dyn Describable> 自动解引用，直接 item.describe() 即可。
pub fn describe_all(items: &[Box<dyn Describable>]) -> Vec<String> {
    items.iter().map(|item| item.describe()).collect()
}
```

也可以用 `fold` 写 9-1，省掉 `mut`：
`items.iter().copied().fold(None, |max, x| match max { Some(m) if m >= x => Some(m), _ => Some(x) })`。

**为什么 9-1 不返回 `Option<&T>`？** 那样省一次拷贝（`T: PartialOrd` 就够），
但调用方拿到的是借用，生命周期被绑死在输入切片上。返回 `Option<T>` 更像「值语义」的 API，
也正好逼你理解 `Copy` 在这里的作用。

</details>

---

## 自测清单

- [ ] 我能解释「Rust 没有继承」之后，行为复用靠哪两种机制，各自什么时候用
- [ ] 我能写出带 `where` 子句的泛型函数，并说清它和 `<T: Trait>` 的关系
- [ ] 我知道 `largest<T: PartialOrd + Copy>` 里的 `Copy` 为什么必需，去掉会报什么错
- [ ] 我能说出 `impl Trait` 参数与 `dyn Trait` 在分发时机、性能、混装能力上的区别
- [ ] 我知道 `impl Trait` 返回位置为什么只能返回一种具体类型
- [ ] 我能说出孤儿规则的内容，以及它为什么保证了一致性
- [ ] 我能说出 Display / Debug / Clone / Copy / PartialEq / Eq / PartialOrd / Ord / Hash / Default / From / Into / Iterator 各自解决什么问题
- [ ] 我知道 `format!("{}", -2.0_f64)` 输出 `-2` 而不是 `-2.0`
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch09_generics_traits` 全绿
