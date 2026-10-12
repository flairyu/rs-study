# 第 5 章 · 结构体、枚举与模式匹配

> **一句话定位**：`struct` + `enum` + `match` 是 Rust 的数据建模三件套——把「几个字段」和「几种情况」都变成编译期可检查的类型。
> **预计时间**：90 分钟（读 45 + 写 45）
> **前置**：第 4 章（引用与借用）
> **练习工程**：`exercises/ch05_structs_enums`（3 题）

## 0. 先建立锚点（给 C/C++/Python 使用者）

| 话题 | C | Python | Rust |
|---|---|---|---|
| 打包字段 | `struct Point { int x; int y; };` | `class` / `@dataclass` | `struct Point { x: i32, y: i32 }` |
| 方法写在哪 | 自由函数 `point_shift(&p, dx)` | 类体里的 `def shift(self, dx)` | 独立的 `impl Point { ... }` 块 |
| 默认可见性 | 全部公开 | 全部公开 | **私有**，要公开必须写 `pub` |
| 默认可变性 | 字段可改 | 属性可改 | 跟随绑定：`let mut p` 才能改 |
| 判等 / 打印 | 逐字段手写 | `__eq__` / `__repr__` | `#[derive(PartialEq)]` / `#[derive(Debug)]` |
| "几种情况之一" | `enum` 本质是 `int`，数据另放 | `Union` / 类继承 | `enum`，**每个变体自带数据** |
| 空值 | `NULL`（会段错误） | `None`（任何类型都可能是它） | **没有 null**，只有 `Option<T>` |

同一件事，一句话版：

- **C**：`struct Point { int x; int y; };` + 自由函数 `void point_shift(struct Point *p, int dx);`——数据和方法分开，改数据靠指针。
- **Python**：`@dataclass class Point` 把字段声明和方法体放进同一个 `class`，`self` 永远是可变引用。
- **Rust**：`struct Point { pub x: i32, pub y: i32 }` 只描述数据，行为写在独立的 `impl Point` 块里；`fn shift(&mut self, dx: i32)` 把"能不能改"写进了签名。

枚举的差别更大：C 的 `enum Color { RED, GREEN };` 里 `RED` 就是整数 `0`，它不能"顺便"带一个半径；
Rust 的 `enum Shape { Circle(f64), Rectangle(f64, f64), Triangle(f64, f64) }` 每个变体像一个小 struct，
携带的数据类型可以完全不同。

一句话总结：**Rust 的 `struct`/`enum` 是类型，不是内存布局建议。** `match` 会逼你把每种情况写完，漏一个变体直接编译失败——这是 Python 的 `isinstance` 链和 C 的 `switch` 都给不了的保证。

## 1. 结构体：定义与实例化

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}

let a = Rectangle { width: 3.0, height: 4.0 };   // 实例化就是写一个结构体字面量
println!("{a:?}");                                // 输出: Rectangle { width: 3.0, height: 4.0 }
```

- 字段顺序不影响语义，访问用 `a.width`；没有 `pub` 的字段只在定义它的模块里可见（第 14 章）。
- 结构体**没有构造器**这种特殊函数；"构造"只是返回 `Self` 的普通关联函数。
- 少写一个字段 → `error[E0063]: missing field \`height\` in initializer of \`Rectangle\``。

顺带一提：`struct Meters(f64);` 是**元组结构体**（字段没名字，用 `.0` 访问），`struct Marker;` 是**单元结构体**（不携带数据，用来实现 trait 或做标记）。

> ⚠️ **坑**：`let b = a;` 会**移动**（move）结构体，之后 `a` 不能再用；想留两份就 `a.clone()`。
> 另一个误解：字段全是 `i32` 的结构体**不会**自动变 `Copy`，仍要显式 `#[derive(Copy, Clone)]`。

## 2. 字段初始化简写

参数名和字段名相同时可以只写一遍：

```rust
impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }        // 等价于 Self { width: width, height: height }
    }
}
```

`Self` 是"当前 `impl` 块正在实现的类型"的别名，返回 `Self` 比写 `Rectangle` 更耐改。
对照：Python 必须 `self.width = width` 写两遍，C 的指定初始化 `(struct R){ .width = width }` 更长——
Rust 只是把"名字相同"这个最常见的场景缩短了。

## 3. `impl`：方法、`&self`、`&mut self`、关联函数

方法都写在 `impl` 块里，**第一个参数**决定它是哪一种：

| 写法 | 名字 | 读字段 | 改字段 | 调用后调用者还能用吗 |
|---|---|---|---|---|
| `fn f(&self)` | 方法 | ✅ | ❌ | ✅（共享借用） |
| `fn f(&mut self)` | 方法 | ✅ | ✅ | ✅（可变借用结束后） |
| `fn f(self)` | 方法 | ✅ | ✅（自己的副本） | ❌（所有权被消耗） |
| `fn f()` | 关联函数 | — | — | —（用 `Rectangle::f()` 调用） |

```rust
impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height          // 只读
    }

    pub fn is_square(&self) -> bool {
        (self.width - self.height).abs() < 1e-9
    }

    pub fn scale(&mut self, factor: f64) {
        self.width *= factor;             // 修改自己，需要 &mut self
        self.height *= factor;
    }
}

let mut r = Rectangle::new(2.0, 3.0);     // 想调用 &mut self，绑定必须是 mut
r.scale(2.0);
println!("{}", r.area());                 // 输出: 24
```

- 对照 Python：`def area(self)` 的 `self` 永远是可变引用，"只读"只能靠约定；Rust 写进签名，编译器替你查。
- 对照 C++：`&self` ≈ `const` 成员函数，`&mut self` ≈ 非 const 成员函数，没有 `self` 的关联函数 ≈ `static` 成员函数。

**为什么方法不写在 `struct` 里面？** 因为 Rust 允许为同一个类型写任意多个 `impl` 块（只要在同一个 crate），
trait 实现也必须写成 `impl Trait for Type`。于是"类型定义"只描述数据，"行为"可以按 trait 或按主题拆开。
上面的 `area` 和 `scale` 就算分成两个 `impl Rectangle` 块写，也完全等价。

> ⚠️ **坑**：`&mut self` 方法一次只能有一个活跃借用。`r.scale(2.0); r.scale(2.0);` 顺序执行没问题，
> 但 `let a = &mut r; let b = &mut r;` 编译不过——第 4 章的借用规则在这里一模一样地生效。

## 4. `#[derive(...)]`：让编译器替你写"无聊的 trait"

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle { pub width: f64, pub height: f64 }

let a = Rectangle { width: 1.0, height: 2.0 };
let b = a.clone();                     // Clone
println!("{a:?}");                     // Debug
assert_eq!(a, b);                      // PartialEq
```

| derive | 给你什么 | 对照 |
|---|---|---|
| `Debug` | `{:?}` / `{:#?}` 打印 | Python `__repr__`、dataclass 自动 |
| `Clone` / `Copy` | `.clone()` 深拷贝 / 赋值即复制不移动 | Python `copy.deepcopy` / C 的按值传参 |
| `PartialEq` / `Eq` | `==` / `!=` | Python `__eq__`、C 的手写比较 |
| `Default` / `Hash` | `Type::default()` / 能当 `HashMap` 的 key（第 7 章） | Python 默认参数 / `__hash__` |

两个容易混的点：

- `Debug` ≠ `Display`：`{}` 要手写 `impl Display`；`{:?}` 由 `Debug` 提供，输出是给开发者看的。
- 浮点字段能 derive `PartialEq`，但**不能** derive `Eq`——因为 `NaN != NaN`，编译器会直接拒绝。

## 5. 枚举：每个变体可以携带不同的数据

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Circle(f64),              // 圆：携带半径
    Rectangle(f64, f64),      // 矩形：依次携带宽、高
    Triangle(f64, f64),       // 三角形：依次携带底、高
}
```

Rust 的枚举是**和类型（sum type）**：`Shape` 的值必然是三者之一，每个变体携带的数据类型可以完全不同，
也不存在 C 里"`kind` 字段和数据对不上"的可能——因为 `kind` 和数据本来就是同一个值。

变体有三种写法，可以混用：

```rust
enum Event {
    Quit,                                  // 单元变体（像 C 的枚举项）
    KeyPress(char),                        // 元组变体
    Resize { width: u32, height: u32 },    // 结构体变体
}
```

标准库里的 `Option<T>` 和 `Result<T, E>` 就是普通的枚举（`Result` 在第 6 章）。

## 6. `Option<T>` 与 `null` 的对比

Python 的 `None` 是**一个值**，任何变量都可能是它，错误只能在运行时冒出来：

```python
def first_char(s):
    return s[0] if s else None      # 返回 str 还是 None？只能看文档/猜

name = first_char("")
print(name.upper())                 # AttributeError: 'NoneType' object has no attribute 'upper'
```

Rust 用一个枚举代替 `null`：

```rust
pub enum Option<T> {                // 标准库定义（简化版）
    None,
    Some(T),
}

let name = "".chars().next();       // 类型是 Option<char>
// name.unwrap().to_uppercase();    // 编译过，但空串时会 panic
match name {
    Some(c) => println!("第一个字符: {c}"),
    None => println!("空字符串"),    // ✅ 编译器逼你先处理 None
}
```

区别可以压成三句话：

1. **Rust 的 `None` 不是"任何类型都能是它"。** `None` 的类型是某个具体的 `Option<T>`，不能当 `T` 用；Python 的 `None` 可以赋给任何变量。
2. **`Option<T>` 和 `T` 是两个不同的类型。** 想拿到里面的 `T`，必须显式 `match` / `if let` / `unwrap()` / `unwrap_or(...)`，每个动作都在代码里留了痕。
3. **没有 null，就没有空指针解引用。** C 的 `NULL` 和 Python 的 `None` 都是"运行时才知道的意外"，`Option` 把它变成"编译期的必答题"。`unwrap()` 也会 panic，但那是你**主动**写的，`grep` 一下就能全找出来。

> ⚠️ **坑**：`Option<&T>`、`Option<Box<T>>` 通常和裸指针一样大（`None` 复用空指针这个非法位模式），
> 但 `Option<i32>` **会**比 `i32` 大（需要额外的判别字段）。别以为 `Option` 永远零成本。

## 7. `match`：解构 + 穷尽性检查

```rust
pub fn shape_area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(radius) => std::f64::consts::PI * radius * radius,
        Shape::Rectangle(width, height) => width * height,
        Shape::Triangle(base, height) => 0.5 * base * height,
    }
}
```

- 每个分支是一个**模式**：`Shape::Circle(radius)` 把变体里的 `f64` 绑定到 `radius`。
- 参数是 `&Shape`，所以绑定出来是 `&f64`（match ergonomics）；`f64` 的算术运算符对引用也有实现，直接乘即可。
- **穷尽性**：少写一个变体 → `error[E0004]: non-exhaustive patterns: \`Shape::Triangle(..)\` not covered`。以后给枚举加新变体，所有 `match` 都会替你报警——这是重构时最有价值的一条错误。
- `_ => ...` 能兜底，但也会**关掉**这种保护；自家枚举上尽量别用。

`match` 同样能解构结构体，还能配 `if` 守卫：

```rust
let r = Rectangle { width: 3.0, height: 3.0 };
match r {
    Rectangle { width: w, height: h } if (w - h).abs() < 1e-9 => println!("正方形 {w}x{h}"),
    Rectangle { width, height } => println!("{}x{}", width, height),
}
```

`if` 守卫在前面的模式匹配成功**后**才求值；不满足就继续试下一个分支。

## 8. `if let`、`let else` 与 `matches!`

只关心一个分支时，用 `match` 还要写 `_ => {}`，`if let` 更直接：

```rust
let value = Some(5);

if let Some(n) = value {
    println!("拿到 {n}");        // 输出: 拿到 5
}
// 完全等价于 match value { Some(n) => println!("拿到 {n}"), _ => {} }
```

`if let ... else` 与 `let else`（都从 Rust 1.65 起稳定）处理"不匹配就早退"：

```rust
if let Some(n) = value { println!("有值: {n}"); } else { println!("没有值"); }

fn double_or_zero(value: Option<i32>) -> i32 {
    let Some(n) = value else { return 0; };   // 不匹配就返回，之后 n: i32
    n * 2
}
```

`matches!` 把"这个值匹配某模式吗"变成布尔表达式，适合断言和过滤：

```rust
assert!(matches!(Some(1), Some(_)));
assert!(!matches!(None::<i32>, Some(_)));
assert!(matches!(Shape::Circle(1.0), Shape::Circle(_)));
assert!(matches!(Some(-3), Some(n) if n < 0));   // 也支持 if 守卫

let values = [Some(1), None, Some(3)];
let count = values.iter().filter(|v| matches!(v, Some(_))).count();
assert_eq!(count, 2);   // matches! 也常这么用在 filter 里
```

`if let` / `let else` / `matches!` 都只是 `match` 的语法糖，语义完全一样，选顺眼的那个就行。

## 9. 常见坑

1. **忘写 `pub`**：类型或字段在 crate 外不可见，集成测试直接报"私有"。
2. **忘写 `mut`**：`cannot borrow \`r\` as mutable, as it is not declared as mutable`——`&mut self` 方法要求绑定是 `let mut`。
3. **derive 了 `PartialEq` 却忘了 `Debug`**：`assert_eq!` 编译不过，它要求两边都能 `{:?}`。
4. **浮点用 `==` 判等**：`0.1 + 0.2 != 0.3`；判等用 `(a - b).abs() < 1e-9`。
5. **枚举字段顺序写反**：`Shape::Rectangle(height, width)` 编译不报错，只是结果悄悄错了。严格按规格写。
6. **在 `&self` 方法里改字段**：`cannot assign to \`self.width\`, which is behind a \`&\` reference`；要改就换 `&mut self`。
7. **`Some(0)` 漏掉**：`Some(n) if n > 0` 不匹配 `0`，后面必须还有 `Some(n)` 或 `_` 兜底，否则穷尽性检查会报错——报错是好事。
8. **把 `Option<i32>` 当 `i32` 算**：`x + 1` 编译不过，先解构（或 `unwrap_or(0)`）。

## 10. 本章练习

| 题号 | 任务 | 考点 | 对应 API |
|---|---|---|---|
| 5-1 | 给 `Rectangle` 写构造、面积、正方形判断、缩放 | `impl`、`&self` / `&mut self`、关联函数、`#[derive]` | `Rectangle::{new, area, is_square, scale}` |
| 5-2 | 用 `match` 给三种形状算面积 | 枚举携带数据、模式解构、`std::f64::consts::PI` | `shape_area` |
| 5-3 | 把 `Option<i32>` 描述成四种字符串 | `Option`、`match` 守卫、`format!` | `describe_opt` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch05_structs_enums                          # 跑评测
python3 tools/ai_review.py ch05_structs_enums --hint 2             # 卡住了要提示
```

提示（不给答案）：

- **5-1**：`new` 用 `Self { width, height }`；`area` 就是一次乘法；`is_square` 用 `(self.width - self.height).abs() < 1e-9`；`scale` 接收 `&mut self`，两行 `*=`。负数边长、面积为 0 都直接按公式算，不要 `panic!`。
- **5-2**：`match shape { ... }` 三个分支；圆用 `std::f64::consts::PI * radius * radius`，三角形别忘了 `0.5 *` 这个系数。
- **5-3**：分支顺序是 `None` → `Some(0)` → `Some(n) if n > 0` → `Some(n)`。最后那个分支只能拿到负数，所以不用再写守卫；字符串用 `format!("positive: {n}")`。

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 5-1
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }                       // 字段初始化简写
    }

    pub fn area(&self) -> f64 {
        self.width * self.height                     // 只读，不需要 mut
    }

    pub fn is_square(&self) -> bool {
        (self.width - self.height).abs() < 1e-9      // 浮点绝不用 ==
    }

    pub fn scale(&mut self, factor: f64) {
        self.width *= factor;                        // &mut self：就地修改调用者
        self.height *= factor;
    }
}

// 5-2
pub fn shape_area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(radius) => std::f64::consts::PI * radius * radius,
        Shape::Rectangle(width, height) => width * height,
        Shape::Triangle(base, height) => 0.5 * base * height,
    }
}

// 5-3
pub fn describe_opt(value: Option<i32>) -> String {
    match value {
        None => "none".to_string(),
        Some(0) => "zero".to_string(),
        Some(n) if n > 0 => format!("positive: {n}"),
        Some(n) => format!("negative: {n}"),
    }
}
```

为什么这么写：

- `new` 用 `Self` 而不是 `Rectangle`，以后重命名类型只改一处；字段初始化简写省掉重复的名字。
- `is_square` 用容差而不是 `==`：`1.0 + 1e-10` 与 `1.0` 是不同位模式，却在工程意义上相等。
- `shape_area` 里 `match shape` 的绑定是 `&f64`，标准库为引用实现了算术运算符，所以照常写 `*`；三种形状都写了分支、没有 `_`，以后给 `Shape` 加变体会立刻编译报错。
- `describe_opt` 的分支顺序不能换：`Some(0)` 必须排在 `Some(n) if n > 0` 前面，否则 `0` 会被当成正数。

与自动评测的期望值一致：`Rectangle::new(3.0, 4.0).area() == 12.0`、`is_square` 在 `1e-9` 容差内为真、
`shape_area(&Shape::Circle(1.0)) == π`、`shape_area(&Shape::Triangle(6.0, 2.0)) == 6.0`、
`describe_opt(Some(0)) == "zero"`、`describe_opt(Some(-3)) == "negative: -3"`。

</details>

## 自测清单

- [ ] 我能说出 `&self`、`&mut self`、`self` 三种接收者的区别，以及各自要求调用方写什么
- [ ] 我知道为什么方法写在 `impl` 块而不是 `struct` 里，也见过同一个类型的多个 `impl` 块
- [ ] 我能解释 `#[derive(Debug, Clone, PartialEq)]` 分别给了什么，以及为什么 `f64` 不能 derive `Eq`
- [ ] 我能说清 Rust 的 `Option<T>` 和 Python 的 `None` 的区别，以及"Rust 为什么没有 null"
- [ ] 我能不查资料写出 `shape_area` 的 `match`，并说出穷尽性检查帮了我什么
- [ ] 我知道 `if let` / `let else` / `matches!` 各自适合什么场合
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch05_structs_enums` 全绿
