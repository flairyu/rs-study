//! # 第 5 章 · 结构体、枚举与模式匹配
//!
//! 本章把「自定义类型」交到你手里：用 `struct` 把相关字段绑在一起，
//! 用 `enum` 表示「几种情况之一」，再用 `match` / `if let` 把它们拆开。
//! 全程只依赖标准库。
//!
//! 运行评测：
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch05_structs_enums
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`Rectangle`] | 结构体与字段、`impl`、`&self` / `&mut self`、关联函数、`#[derive]` |
//! | [`shape_area`] | 枚举携带数据、`match` 解构、`std::f64::consts::PI` |
//! | [`describe_opt`] | `Option<T>`、`match` 的模式与守卫、`format!` |
//!
//! 提示：先读 `docs/01-syntax/ch05-structs-enums.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 5-1：一个矩形。
///
/// 字段是 `pub` 的，所以结构体外面可以直接读写（`r.width = 6.0`）。
/// 派生 `Debug, Clone, PartialEq`，这样 `{:?}` 打印、`.clone()` 复制、`==` 比较
/// 都不用自己写。
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}

impl Rectangle {
    /// 关联函数（第一个参数不是 `self`）：从宽高构造一个矩形。
    ///
    /// 返回类型写 `Self`，它等价于 `Rectangle`，重命名类型时只需改一处。
    /// 字段名和参数名相同时，用**字段初始化简写** `Self { width, height }`。
    ///
    /// # 示例
    /// ```
    /// # use ch05_structs_enums::Rectangle;
    /// let r = Rectangle::new(3.0, 4.0);
    /// assert!((r.width - 3.0).abs() < 1e-9);
    /// assert!((r.height - 4.0).abs() < 1e-9);
    /// ```
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// 方法：借用 `&self`，只读，不消耗调用者。
    ///
    /// 面积为 `width * height`。任一边为 0（面积为 0）或边长为负数都是合法输入，
    /// 直接按代数乘法返回即可，**不要 panic**。
    ///
    /// # 示例
    /// ```
    /// # use ch05_structs_enums::Rectangle;
    /// assert!((Rectangle::new(3.0, 4.0).area() - 12.0).abs() < 1e-9);
    /// assert!((Rectangle::new(0.0, 5.0).area() - 0.0).abs() < 1e-9);
    /// ```
    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    /// 是否正方形：`(width - height).abs() < 1e-9`。
    ///
    /// 浮点不要用 `==` 判等：`1.0` 和 `1.0 + 1e-10` 是不同的浮点数，但工程上应视为相等。
    /// 容差固定用 `1e-9`。
    ///
    /// # 示例
    /// ```
    /// # use ch05_structs_enums::Rectangle;
    /// assert!(Rectangle::new(2.5, 2.5).is_square());
    /// assert!(!Rectangle::new(3.0, 4.0).is_square());
    /// ```
    pub fn is_square(&self) -> bool {
        (self.width - self.height).abs() < 1e-9
    }

    /// 就地按比例缩放：`&mut self` 说明它会**修改**调用者，不返回新值。
    ///
    /// 调用方必须是 `mut` 绑定（`let mut r = ...`），否则编译不过。
    ///
    /// # 示例
    /// ```
    /// # use ch05_structs_enums::Rectangle;
    /// let mut r = Rectangle::new(2.0, 3.0);
    /// r.scale(2.0);
    /// assert!((r.area() - 24.0).abs() < 1e-9);
    /// assert!((r.width - 4.0).abs() < 1e-9);
    /// ```
    pub fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }
}

/// 练习 5-2：三种形状。
///
/// 枚举的每个变体可以携带**不同数量和类型**的数据：圆带 1 个 `f64`（半径），
/// 矩形带 2 个（宽、高），三角形带 2 个（底、高）。
/// 这是 C 的 `enum`（本质是一个整数）做不到的，也是 Rust 里和类型（sum type）的基础。
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// 圆：携带半径。
    Circle(f64),
    /// 矩形：依次携带宽、高。
    Rectangle(f64, f64),
    /// 三角形：依次携带底、高。
    Triangle(f64, f64),
}

/// 练习 5-2：用 `match` 解构枚举并计算面积。
///
/// - `Circle(radius)` → `π r²`，用 `std::f64::consts::PI`
/// - `Rectangle(width, height)` → `width * height`
/// - `Triangle(base, height)` → `0.5 * base * height`
///
/// 参数是 `&Shape`，所以 `match` 出来的绑定是 `&f64`；`f64` 的算术运算符对引用
/// 也有实现，直接 `radius * radius` 即可。三种形状都必须有分支——漏一个编译就报错。
///
/// # 示例
/// ```
/// # use ch05_structs_enums::{shape_area, Shape};
/// let area = shape_area(&Shape::Circle(1.0));
/// assert!((area - std::f64::consts::PI).abs() < 1e-9);
/// assert!((shape_area(&Shape::Rectangle(3.0, 4.0)) - 12.0).abs() < 1e-9);
/// assert!((shape_area(&Shape::Triangle(6.0, 2.0)) - 6.0).abs() < 1e-9);
/// ```
pub fn shape_area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(radius) => std::f64::consts::PI * radius * radius,
        Shape::Rectangle(width, height) => width * height,
        Shape::Triangle(base, height) => 0.5 * base * height,
    }
}

/// 练习 5-3：把 `Option<i32>` 翻译成人类可读的字符串。
///
/// | 输入 | 输出 |
/// |---|---|
/// | `None` | `"none"` |
/// | `Some(0)` | `"zero"` |
/// | `Some(n)`，`n > 0` | `"positive: {n}"` |
/// | `Some(n)`，`n < 0` | `"negative: {n}"` |
///
/// 用 `match` + `if` 守卫（`Some(n) if n > 0`）。注意 `None` 是**一个正常的值**，
/// 不是错误，所以返回 `String` 而不是 `Result`。
///
/// # 示例
/// ```
/// # use ch05_structs_enums::describe_opt;
/// assert_eq!(describe_opt(None), "none");
/// assert_eq!(describe_opt(Some(0)), "zero");
/// assert_eq!(describe_opt(Some(5)), "positive: 5");
/// assert_eq!(describe_opt(Some(-3)), "negative: -3");
/// ```
pub fn describe_opt(value: Option<i32>) -> String {
    match value {
        None => "none".to_string(),
        Some(0) => "zero".to_string(),
        Some(n) if n > 0 => format!("positive: {n}"),
        Some(n) => format!("negative: {n}"),
    }
}
