//! # 第 14 章 · 模块、crate 与宏
//!
//! 本章解决「代码怎么组织」的问题：`mod` 把代码切成树、`pub` 决定谁能看见、
//! `use` 决定路径怎么写，而 `macro_rules!` 让你用**语法匹配**代替文本替换。
//!
//! 做法：把每个 `unimplemented!()` 换成你自己的实现（**不要改签名**；
//! 第 14-2 题连宏的非空分支也要自己补全），然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch14_modules_macros
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`Point`] / [`geometry`] | `mod` 树、`pub` 可见性、`pub use` 重导出、方法实现 |
//! | `hashmap_of!` | `macro_rules!` 的 `$k:expr` 片段与 `$(...),+` 重复、尾逗号 |
//! | [`Temperature`] / [`render`] | 实现 `std::fmt::Display`、`ToString` 由 Display 白送 |
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 14-1 的模块：一个「几何」子模块，[`Point`] 定义在里面。
///
/// 注意 `Point` 本身是 `pub`，两个字段也是 `pub`——缺一个，集成测试就构造不出来。
/// 而文件下方的 `pub use geometry::Point;` 把它**重导出**到 crate 根，
/// 所以测试写 `use ch14_modules_macros::Point;` 就能拿到它，不必写 `geometry::Point`。
pub mod geometry {
    /// 平面上的一个点。已派生 `Debug, Clone, PartialEq`。
    ///
    /// - `Debug`：`assert_eq!` 失败时要打印它；
    /// - `Clone`：想复制一个点时不用自己写；
    /// - `PartialEq`：让 `==` 可用（注意 `f64` 只有 `PartialEq`，没有 `Eq`）。
    #[derive(Debug, Clone, PartialEq)]
    pub struct Point {
        pub x: f64,
        pub y: f64,
    }

    impl Point {
        /// 练习 14-1（a）：用两个坐标构造一个点。
        ///
        /// 这是**关联函数**（没有 `self` 参数），所以调用方式是 `Point::new(...)`，
        /// 而不是 `p.new(...)`。
        ///
        /// 提示：返回 `Self { x, y }`（字段名和变量名相同时可以简写）。
        ///
        /// # 示例
        /// ```
        /// # use ch14_modules_macros::Point;
        /// let p = Point::new(3.0, 4.0);
        /// assert_eq!(p.x, 3.0);
        /// assert_eq!(p.y, 4.0);
        /// ```
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }

        /// 练习 14-1（b）：返回 `self` 到 `other` 的欧氏距离。
        ///
        /// 公式：`sqrt((x1 - x2)^2 + (y1 - y2)^2)`。
        ///
        /// 边界：
        /// - 同一点（包括 `p.distance_to(&p)`）→ `0.0`；
        /// - 坐标为负数 → 先做减法再平方，符号自然消失，结果永远非负；
        /// - 距离是**对称**的：`a.distance_to(&b) == b.distance_to(&a)`。
        ///
        /// 提示：`f64` 的平方用 `.powi(2)`，开平方用 `.sqrt()`，两个方法都在 std 里。
        /// 参数是 `&Point`（借用），所以你不需要、也不应该把 `other` 拿走。
        ///
        /// # 示例
        /// ```
        /// # use ch14_modules_macros::Point;
        /// let a = Point::new(0.0, 0.0);
        /// let b = Point::new(3.0, 4.0);
        /// assert!((a.distance_to(&b) - 5.0).abs() < 1e-9);
        /// ```
        pub fn distance_to(&self, other: &Point) -> f64 {
            ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
        }
    }
}

pub use geometry::Point;

/// 练习 14-2：用 `macro_rules!` 造一个 `hashmap_of!` 宏。
///
/// 要求支持下面这些调用形态（宏不区分 `{}` / `()` / `[]`，用哪种括号都行）：
///
/// | 调用 | 结果 |
/// |---|---|
/// | `hashmap_of!{}` | 空 `HashMap` |
/// | `hashmap_of!{"a" => 1}` | 一个键值对 |
/// | `hashmap_of!{"a" => 1, "b" => 2}` | 两个键值对 |
/// | `hashmap_of!{"a" => 1, "b" => 2,}` | 同上，**允许尾逗号** |
///
/// 语义：按书写顺序逐个 `insert`，所以同名 key **后写的覆盖先写的**。
///
/// ⚠️ 骨架里**空分支已经是实现好的**（`() => { std::collections::HashMap::new() }`），
/// 这是故意的：如果连空分支都没有，`hashmap_of!{}` 会在**编译期**就匹配失败，
/// 整章测试会变成"编译错误"而不是"断言失败"。你只需要补全非空分支。
///
/// 提示：非空分支匹配的是「一对」`$k:expr => $v:expr`，多对之间用逗号分隔，
/// 因此要写成 `$($k:expr => $v:expr),+`（`+` 表示至少一次），末尾再挂一个
/// 可选的逗号 `$(,)?`。展开时先把 `m` 建出来，再 `$( m.insert($k, $v); )+`，
/// 最后把 `m` 交出去——注意展开结果要用 `{{ ... }}` 包成一个块表达式。
///
/// `#[macro_export]` 的作用：把宏导出到 **crate 根**，别的 crate 才能
/// `use ch14_modules_macros::hashmap_of;`。
///
/// # 示例
/// ```
/// # use ch14_modules_macros::hashmap_of;
/// let m = hashmap_of!{"a" => 1, "b" => 2,};
/// assert_eq!(m.len(), 2);
/// assert_eq!(m.get("a"), Some(&1));
/// ```
#[macro_export]
macro_rules! hashmap_of {
    () => { std::collections::HashMap::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut m = std::collections::HashMap::new();
        $( m.insert($k, $v); )+
        m
    }};
}

/// 一个摄氏温度。字段是 `pub` 的，所以测试可以直接写 `Temperature { celsius: 0.0 }`。
pub struct Temperature {
    pub celsius: f64,
}

/// 练习 14-3（a）：为 [`Temperature`] 实现 `std::fmt::Display`。
///
/// 输出格式：直接用 `{}` 打印 `self.celsius`，后面紧跟一个 `°C` 后缀。
///
/// | `celsius` | 输出 |
/// |---|---|
/// | `0.0` | `0°C` |
/// | `-40.0` | `-40°C` |
/// | `36.6` | `36.6°C` |
///
/// ⚠️ 坑：`f64` 用 `{}` 打印是**最短表示**，不补小数点：`0.0` → `"0"`、`100.0` → `"100"`。
/// 所以别写 `{:.1}`，否则 `0.0` 会变成 `"0.0°C"`。
///
/// 提示：在 `fmt` 里用 `write!(f, "...", self.celsius)`，把它的返回值（`fmt::Result`）
/// 直接当函数结果交回去；不要用 `println!`。
/// 如果你想少打几个字，可以在文件顶部 `use std::fmt;`，然后把签名里的
/// `std::fmt::Formatter` / `std::fmt::Result` 简写成 `fmt::Formatter` / `fmt::Result`。
///
/// 好处：一旦实现了 `Display`，std 的 blanket impl
/// `impl<T: Display + ?Sized> ToString for T` 会**白送** `.to_string()`，
/// 下面的 [`render`] 就靠它。
///
/// # 示例
/// ```
/// # use ch14_modules_macros::Temperature;
/// let t = Temperature { celsius: 36.6 };
/// assert_eq!(t.to_string(), "36.6°C");
/// ```
impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°C", self.celsius)
    }
}

/// 练习 14-3（b）：把温度渲染成字符串。
///
/// 用 `Display` 提供的 `to_string()` 即可，**不要**在这里手写 `format!("{}°C", ...)`——
/// 那样就绕过了 `Display` 这个抽象，等于两道题只做了一道。
///
/// # 示例
/// ```
/// # use ch14_modules_macros::{render, Temperature};
/// assert_eq!(render(&Temperature { celsius: 0.0 }), "0°C");
/// ```
pub fn render(t: &Temperature) -> String {
    t.to_string()
}
