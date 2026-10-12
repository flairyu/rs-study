# 第 14 章 · 模块、crate 与宏

> **一句话定位**：`mod` 把代码长成一棵树、`pub` 划出边界、`use` 只负责写路径，
> 而 `macro_rules!` 用**语法匹配**取代了 C 那种文本替换。
> **预计时间**：95 分钟（读 45 + 写 50）
> **前置**：第 9 章（泛型与 trait）、第 13 章（async）
> **练习工程**：`exercises/ch14_modules_macros`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

一句话：**文件不决定结构，`mod` 声明才决定结构**；可见性由 `pub` 强制，而不是靠命名约定。

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 组织单位 | `.h` + `.c`，靠文件名 | `模块.py` + `包/__init__.py` | **`mod` 树 + crate 边界** |
| 引入机制 | `#include` 文本复制 | `import` 运行时执行 | `mod` / `use`，**编译期**解析路径 |
| 命名空间 | 没有，靠前缀命名 | 模块对象 | 模块树 + `crate::`/`super::`/`self::` |
| 可见性 | `static` + 头文件里放什么 | `_name` 只是约定 | `pub`/`pub(crate)`/私有，**编译器强制** |
| 条件编译 | `#ifdef DEBUG` | 无 | `#[cfg(feature = "x")]` |
| 宏 | `#define` 文本替换，括号地狱 | 无（装饰器近似） | `macro_rules!` 语法匹配 + 卫生性 |

后果是：**换文件放代码不影响语义**，可以放心拆；而 `pub` 漏写是编译错误，不是运行时惊喜。

## 1. 包（package）与 crate：两个词，两件事

- **package（包）**：一个 `Cargo.toml` 描述的目录，是 Cargo 的**依赖与发布单位**。
- **crate（箱）**：rustc 一次编译的单位，也是一棵 `mod` 树的根，**只有一个根文件**。

| 你写的路径 | Cargo 把它当成 | crate 名 |
|---|---|---|
| `src/lib.rs` | lib crate 的根 | package 名（`-` 换成 `_`） |
| `src/main.rs` / `src/bin/tool.rs` | bin crate 的根 | package 名 / `tool` |
| `tests/chapter.rs` | 独立的集成测试 crate | 只能 `use` 你的 crate |

所以 `cargo build -p ch14_modules_macros` 的 `-p` 是 **package 名**，`use ch14_modules_macros::Point;`
里是 **crate 名**；package 名带 `-` 时（如 `my-tool`）crate 名会把它换成 `_`。

## 2. `mod`：把代码长成一棵树

`mod a { ... }` 把模块体写在当前文件；`mod a;`（分号）表示模块体在别的文件，Rust 会找 `src/a.rs`
或旧写法 `src/a/mod.rs`——**二者只能存在一个**，否则报 ambiguity。查找位置是「**父模块所在的目录**」。

```
exercises/ch14_modules_macros/
├── src/lib.rs         # crate root：内联 mod geometry + pub use + hashmap_of! + Temperature
├── src/geometry.rs    # 若把 geometry 拆成文件：内容搬来这里，lib.rs 改成 `mod geometry;`
└── tests/chapter.rs   # 独立的集成测试 crate（裁判）
```

模块树同时就是**路径树**：`crate::geometry::Point` 从根一路写下来。

## 3. 可见性：`pub` / `pub(crate)` / 私有

**默认私有。** 但准确含义是「**本模块及其后代模块可见**」，所以父模块的私有工具函数能被子模块调用。

```rust
mod inner {
    pub(crate) fn crate_only() -> i32 { 1 }   // 整个 crate 可见，但不出 crate
    fn private() -> i32 { 2 }                 // 只有 inner 及其后代可见
    pub fn public_api() -> i32 { private() + crate_only() }
}
fn main() {
    println!("{} {}", inner::public_api(), inner::crate_only());   // 输出: 3 1
    // println!("{}", inner::private());   // ❌ 编译不过：error[E0603]: `private` is private
}
```

| 写法 | 谁能用 |
|---|---|
| 什么都不写 | 本模块及其后代模块 |
| `pub(crate)` | 本 crate 内任意位置（最常用） |
| `pub(super)` / `pub(in crate::a::b)` | 父模块（或指定祖先模块）的子树 |
| `pub` | 所有能走到这条路径的人（含跨 crate） |

`pub struct Point { pub x: f64, y: f64 }` 里的 `y` 仍然是私有的，外部连构造都构造不出来——
本章 14-1 两个字段都必须 `pub`；枚举变体则跟着枚举走，不能单独写 `pub`。

## 4. `use` 与路径：`crate` / `super` / `self`

| 起点 | 含义 |
|---|---|
| `crate::` | 当前 crate 根 |
| `super::` | 父模块（可叠加：`super::super::`） |
| `self::` | 当前模块（消歧义时才写） |
| 其他标识符 | 2018 起解释为**外部 crate 名**，或已 `use` 的别名 |

```rust
mod outer {
    pub fn a() -> &'static str { "a" }
    pub mod inner {
        pub fn b() -> &'static str { let _ = super::a(); "b" }   // super:: = 父模块
    }
}

use crate::outer::inner::b as bee;   // use 还能起别名
fn main() { println!("{} {} {}", outer::a(), outer::inner::b(), bee()); }   // 输出: a b b
```

- `use` **不复制、不执行**；删掉它代码照样在，只是你得写全路径。`use` 也能写在函数里，作用域只到
  本函数；`use std::io::{self, Write};` 里的 `self` 表示「`std::io` 本身」。
- 2018 起 `use` 路径必须从 `crate` / `super` / `self` / 外部 crate 名开始。在 crate 根写
  `use geometry::Point;` 会被当成「找外部 crate `geometry`」，要写 `use crate::geometry::Point;`。

> ⚠️ **坑**：`use` 只影响名字解析，**不改变可见性**；外部要用的 `pub` 项，路径上每一级都得公开。

## 5. `pub use`：重导出

```rust
mod geometry {                     // 注意：这个模块是私有的
    pub struct Point { pub x: f64, pub y: f64 }
}
pub use geometry::Point;           // 但 Point 这个名字被挂到了 crate 根

fn main() {
    let p = Point { x: 1.0, y: 2.0 };   // 不必写 geometry::Point
    println!("{} {}", p.x, p.y);        // 输出: 1 2
}
```

上面 `mod geometry` 私有，外部**永远走不到** `geometry::Point`；`pub use` 把 `Point` 再导出到 crate 根，
于是 `use ch14_modules_macros::Point;` 就能用——这就是「**模块结构是私有实现细节，公开 API 由
`pub use` 决定**」的 facade 模式。标准库也这么干：`std::collections::HashMap` 其实是
`std::collections::hash_map::HashMap` 的重导出；重导出**不复制类型**，只是多一个名字。

本章骨架里 `geometry` 本身是 `pub`，两条路径都通。裁判 `tests/chapter.rs` 从 crate 根
`use ch14_modules_macros::{hashmap_of, render, Point, Temperature};`——集成测试是**另一个 crate**，
只能看见 `pub` 的东西，这一行 `use` 本身就是一次可见性考试。

## 6. `#[cfg(test)] mod tests`：测试为什么藏在同一个文件里

```rust
fn private_helper() -> i32 { 7 }

#[cfg(test)]
mod tests {
    use super::*;                                            // 把父模块的东西全拉进来
    #[test]
    fn sees_private() { assert_eq!(private_helper(), 7); }    // 连私有函数都能测
}
```

`#[cfg(test)]` 让这个模块**只在 `cargo test` 时编译**，平时 `cargo build` 里它根本不存在，
所以不占体积也不影响 API。`mod tests` 是子模块，按第 3 节规则能看见父模块的**私有**项，所以
`private_helper` 可测；而**集成测试（`tests/*.rs`）做不到**——它编译成独立 crate，只能碰公开 API。
本章三道题由 `tests/chapter.rs` 判分，所以四个公开项必须都能从 crate 根拿到。

## 7. `macro_rules!`：语法匹配，而不是文本替换

### 7.1 基本形状

```rust
macro_rules! square { ($x:expr) => { ($x) * ($x) }; }
fn main() {
    let n = 4;
    println!("{} {}", square!(3), square!(n + 1));   // 输出: 9 25
}
```

左边是 **matcher**（匹配的语法形状），右边是 **transcriber**（展开结果）。调用时**从上往下**匹配，
第一条赢——所以特例（如空输入）写前面，通用的写后面。

### 7.2 `$x:expr` 与片段说明符

| 说明符 | 匹配 | 例子 |
|---|---|---|
| `expr` | 一个完整表达式 | `n + 1`、`if a { 1 } else { 2 }` |
| `ident` / `literal` | 标识符 / 字面量 | `count` / `42` |
| `ty` / `pat` | 类型 / 模式 | `Vec<i32>` / `Some(x)` |
| `path` / `block` / `stmt` / `tt` / `item` | 路径 / 块 / 语句 / token 树 / 一项 | `std::mem::swap` / `{ let x = 1; x }` |

跟 C 的 `#define SQUARE(x) ((x)*(x))` 对比：`#define` 是**文本**替换，你得手写括号保证优先级；
`$x:expr` 是**语法**片段，rustc 先把它解析成一个完整表达式、再作为整体替换，所以 `square!(n + 1)`
一定是 `(n+1)*(n+1) = 25`，不会退化成 `n + 1 * n + 1 = 9`。限制是它**只认表达式**——`$p:expr`
不能当模式用，而且 `$x:expr` 后面只能跟 `=>`、`,` 或 `;`，写成 `($a:expr $b:expr)` 会报
`` `$a:expr` is followed by `$b:expr`, which is not allowed ``。

> ⚠️ **坑**：宏是**卫生的（hygienic）**——宏内部引入的名字（如 `let mut m`）不会和调用处的同名变量
> 撞车，但 `$k` / `$v` 是调用处传进来的，按调用处解析。`macro_rules!` 还是**文本序**可见的：
> 同一文件里必须「先定义、后使用」，`#[macro_export]` 是唯一例外。

### 7.3 重复：`$(...),*`、`$(...),+` 与尾逗号

| 写法 | 含义 |
|---|---|
| `$( ... )*` / `$( ... )+` / `$( ... )?` | 重复 0+ 次 / 1+ 次 / 0 或 1 次 |
| `$( ... ),+ $(,)?` | 至少一次、用 `,` 分隔，且**末尾允许一个可选逗号** |

`$( ... )` 里的 `$k` / `$v` 是「重复变量」：matcher 匹配了几组，transcriber 里对应的 `$( ... )`
就展开几次，逐组代入。

> ⚠️ **坑**：`=>` 后面那对花括号是 transcriber 的**分隔符**，不是展开结果的一部分。所以
> `($($k:expr => $v:expr),+) => { let mut m = ...; m };` 展开出来是「`let` 语句 + 表达式」，
> 放在表达式位置会报 `error: expected expression, found 'let' statement`。要让它成为**块表达式**，
> 必须写成 `=> {{ ... }}`——外层是分隔符，内层才是代码。

### 7.4 `#[macro_export]`：把宏放到 crate 根

```rust
#[macro_export]
macro_rules! vec_of {
    () => { Vec::new() };
    ($($x:expr),+ $(,)?) => { vec![$($x),+] };   // 至少一个，末尾容忍一个逗号
}
```

`vec_of!()` 与 `vec_of!(1, 2, 3,)` 都能用。关键在于 `#[macro_export]`：它让宏**忽略模块层级**、
直接出现在 **crate 根**，别人才写得出 `use ch14_modules_macros::hashmap_of;`。另外空分支必须存在，
否则 `vec_of!{}` 在**编译期**就无规则可匹配（`error: no rules expected this token`）——骨架里
预置 `hashmap_of!` 的空分支，正是为了让失败是「断言失败」而不是「编译错误」。

## 8. 声明宏 vs 过程宏

| | 声明宏（`macro_rules!`） | 过程宏（proc macro） |
|---|---|---|
| 定义位置 | 任何 crate、任何模块 | **只能**在 `proc-macro = true` 的 crate |
| 输入 | token 模式匹配 | `TokenStream`，用代码处理 |
| 种类 | 一种 | `#[derive(...)]`、属性宏、函数式宏 |
| 额外依赖 | 零 | 通常要 `syn` + `quote`（第三方） |
| 典型用户 | 你自己写的小工具宏 | `#[derive(Serialize)]`、`tokio::main` |

练习 crate **不得引入第三方依赖**，所以本章只出现声明宏；`#[derive(Debug, Clone, PartialEq)]`
是最常见的**派生宏**，职责是按字段自动生成一份 `impl`。

## 9. `Cargo.toml`：`[features]` 与 workspace

`[features]` 是编译期开关，配合 `#[cfg(feature = "extra")]` 这样的属性使用：

```toml
[features]
default = ["fast"]
fast = []
extra = []            # 只有 cargo build --features extra 时才打开
```

默认打开 `default` 里列的集合，`--no-default-features` 关掉。C 里你用 `#ifdef DEBUG` + `-DDEBUG`
干这件事，Rust 把它规范成了「特性名 + `cfg`」。**feature 应当是可叠加的（additive）**：打开一个只会
「加代码、加能力」，不该关掉别的功能；想让可选依赖本身成为 feature 就写 `dep:serde`。

`[workspace]` 把多个 package 放进同一个虚拟工作区（`members = [...]` + `resolver = "2"`），共享一个
`target/` 与依赖锁，子 crate 用 `edition.workspace = true` 继承设置。整套练习都在 `exercises/Cargo.toml`
这一个 workspace 下，所以你才能用 `cargo test -p ch14_modules_macros` 单独跑。

## 10. 常见坑

1. **`mod x;` 与 `src/x.rs` 对不上**：报 `file not found for module`；`src/x.rs` 与 `src/x/mod.rs` 同存会报 ambiguity。
2. **`pub struct` 不会让字段公开**：`Point` 的 `x`、`y` 都必须 `pub`，否则测试里的 `Point { .. }` 构造不出来。
3. **路径上有一级私有**：`pub` 项也拿不到，报 `E0603 is private`；用 `pub use` 在公开路径上重导出。
4. **`use geometry::Point;` 报找不到 crate**：2018 起 crate 内部一律写 `crate::geometry::Point`。
5. **`no rules expected this token`**：matcher 没覆盖这种写法，或漏了尾逗号规则 `$(,)?`。
6. **`expected expression, found 'let' statement`**：transcriber 少一层花括号，忘了 `{{ ... }}`。
7. **`f64` 用 `{}` 打印不补 `.0`**：所以 14-3 用 `write!(f, "{}°C", self.celsius)`；写 `{:.1}` 会得到 `0.0°C`。

## 11. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 14-1 | 实现 `Point::new` 与 `distance_to`，并靠 `pub use` 把它送到 crate 根 | `mod` 树、`pub` 字段可见性、重导出 | `Point::new` / `Point::distance_to` |
| 14-2 | 补全 `hashmap_of!` 的非空分支（空分支已给） | `macro_rules!`、`$k:expr => $v:expr`、`$(...),+`、`$(,)?`、`#[macro_export]` | `hashmap_of!` |
| 14-3 | 为 `Temperature` 实现 `Display`，让 `render` 复用它 | 实现 std trait、`ToString` blanket impl、`{}` 打印 `f64` | `Temperature` / `render` |

要覆盖的边界：同一点与点到自身（距离 `0`）、负坐标、小数坐标、距离的对称性；`hashmap_of!{}` /
单元素带尾逗号 / 多元素 / 不带尾逗号 / 重复 key（后写覆盖先写）；`0.0`、`100.0`、`-40.0`、`36.6`。

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch14_modules_macros      # 卡住了：python3 tools/ai_review.py ch14_modules_macros --hint 2
```

### 提示（不给答案）

- **14-1**：`new` 返回 `Self { x, y }`；`distance_to` 先算 `(self.x - other.x).powi(2) +
  (self.y - other.y).powi(2)`，最后 `.sqrt()`。`lib.rs` 里的 `pub use geometry::Point;` 是题目的一部分。
- **14-2**：matcher 用 `$($k:expr => $v:expr),+ $(,)?`；transcriber 先 `let mut m = HashMap::new();`，
  再 `$( m.insert($k, $v); )+`，最后把 `m` 交出去，整体用 `{{ }}` 包住。空分支保持原样。
- **14-3**：`fmt` 里只写 `write!(f, "{}°C", self.celsius)` 并把它当返回值交回去；`render` 的函数体
  只有 `t.to_string()`。不要用 `println!`，也不要写 `{:.1}`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 14-1(a)：字段名与参数同名时，Self { x, y } 是 Self { x: x, y: y } 的简写。
pub fn new(x: f64, y: f64) -> Self {
    Self { x, y }
}
// 14-1(b)：先减再平方，符号自然消失；(a² + b²).sqrt() 就是欧氏距离。
pub fn distance_to(&self, other: &Point) -> f64 {
    ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
}

// 14-2：完整宏定义。外层 {} 是 transcriber 的分隔符，内层 {} 才是展开出的块表达式。
#[macro_export]
macro_rules! hashmap_of {
    () => { std::collections::HashMap::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut m = std::collections::HashMap::new();
        $( m.insert($k, $v); )+
        m
    }};
}

// 14-3(a)：{} 打印 f64 取最短表示，0.0 → "0"、36.6 → "36.6"。
impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°C", self.celsius)
    }
}
// 14-3(b)：实现了 Display 就白拿 ToString（std 的 blanket impl）。
pub fn render(t: &Temperature) -> String { t.to_string() }
```

**为什么 `render` 不自己 `format!`？** 那样 14-3 就退化成「把格式化写两遍」，`Display` 成了摆设；
实现了 `Display` 的类型自动获得 `.to_string()`，`render` 只是把这份能力包成普通函数。

</details>

## 自测清单

- [ ] 我能说出 package 与 crate 的区别，以及为什么 `cargo -p` 和 `use` 用的名字可能不一样
- [ ] 我能写 `mod x;` 并正确定位 `src/x.rs`，也知道 `src/x/mod.rs` 是旧写法、二者不能并存
- [ ] 我能解释「私有 = 本模块及其后代可见」，并说清 `pub(crate)` 与 `pub(super)` 的范围
- [ ] 我知道 `pub struct` 不会让字段自动公开，本章 `Point` 为什么两个字段都要 `pub`
- [ ] 我能用 `crate::` / `super::` / `self::` 写出完整路径，并说清 `use` 只是别名
- [ ] 我能解释 `pub use` 为什么能让外部 `use` 到「藏在私有模块里」的类型
- [ ] 我知道 `#[cfg(test)] mod tests` 为什么能测私有函数，而 `tests/chapter.rs` 不能
- [ ] 我能写出带 `$($k:expr => $v:expr),+ $(,)?` 的声明宏，并说清 `{{ }}` 为什么是两层
- [ ] 我知道 `#[macro_export]` 把宏放到 crate 根，也知道声明宏与过程宏的分工
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch14_modules_macros` 全绿
