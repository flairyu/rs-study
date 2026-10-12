//! # 第 9 章 · 泛型与 trait
//!
//! 本章解决两个问题：**同一份逻辑怎么服务多种类型**（泛型），
//! 以及**「这个类型能做什么」怎么用类型系统表达**（trait）。
//! Rust 用「泛型 + trait bound」取代了 C++ 的继承，也取代了 Python 的隐式鸭子类型。
//!
//! 做法：把每个 `unimplemented!()` 换成你自己的实现（**不要改函数签名**），然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch09_generics_traits
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`largest`] | 泛型函数、trait bound `PartialOrd + Copy`、`Option` 处理空输入 |
//! | [`Describable`] / [`Point`] | trait 定义与 `impl ... for ...`、`{}` 打印 `f64` |
//! | [`describe_all`] | `Box<dyn Describable>`、trait 对象、动态分发 |
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 9-1：返回切片中的最大值；空切片返回 `None`。
///
/// 语义：用 `>` 比较，返回**最大**的那个元素。元素相等时返回其中任意一个即可
/// （测试也只断言"等于最大值"）。
///
/// 边界：
/// - `&[]` → `None`（这是本章最容易漏掉的分支）
/// - 单元素 → `Some(该元素)`
/// - 元素全部相等 → `Some(该元素)`
///
/// 为什么 signature 上有两个 bound：
/// - `PartialOrd` 提供 `>` / `<`（浮点只是偏序，所以不能要求 `Ord`）；
/// - `Copy` 让你可以「复制一个元素出来返回」，而不用把切片里元素的**所有权**搬走
///   —— 切片是借来的，里面的元素你搬不走。
///
/// 提示：用 `items.iter()` 遍历，迭代出来的每一项都是 `&T`。
/// `&&T` 与 `&&T` 之间比较也是合法的，所以 `if item > max { ... }` 能直接写；
/// 若编译不过，就显式解引用成 `*item > *max`。
///
/// # 示例
/// ```
/// # use ch09_generics_traits::largest;
/// if let Some(m) = largest(&[1, 5, 3]) {
///     assert_eq!(m, 5);
/// }
/// assert!(largest::<i32>(&[]).is_none());
/// ```
pub fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    // 参考答案：first().copied() 把 Option<&T> 变成 Option<T>，空切片时 ? 直接返回 None；
    // 循环里 item 是 &T，*item > max 比较的是 T 的值（T: PartialOrd + Copy）。
    let mut max = items.first().copied()?;
    for item in items.iter() {
        if *item > max {
            max = *item;
        }
    }
    Some(max)
}

/// 「可以被描述」的东西。本章的练习 9-2 / 9-3 都围绕它。
///
/// 注意它是 **crate 根导出的公开 trait**：测试会用
/// `use ch09_generics_traits::*;` 把它和 [`Point`] 一起引入。
pub trait Describable {
    /// 返回一句人类可读的描述。
    fn describe(&self) -> String;
}

/// 平面上的一个点。已派生 `Debug, Clone, PartialEq`。
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 练习 9-2：让 [`Point`] 实现 [`Describable`]。
///
/// 输出格式**必须**是 `Point(x, y)`，即 `format!("Point({}, {})", self.x, self.y)`：
///
/// | `self.x` / `self.y` | 输出 |
/// |---|---|
/// | `1.5` / `-2.0` | `Point(1.5, -2)` |
/// | `1.0` / `0.0` | `Point(1, 0)` |
///
/// ⚠️ 坑：`f64` 用 `{}` 打印**不补小数点和零**。`-2.0` 会打印成 `"-2"`、`1.0` 会打印成 `"1"`。
/// 所以不要写成 `Point({:.1}, {:.1})`，也不要期待出现 `"-2.0"`。
///
/// # 示例
/// ```
/// # use ch09_generics_traits::{Describable, Point};
/// let p = Point { x: 1.5, y: -2.0 };
/// assert_eq!(p.describe(), "Point(1.5, -2)");
/// ```
impl Describable for Point {
    fn describe(&self) -> String {
        // 参考答案：{} 打印 f64 取最短表示：1.5 → "1.5"，-2.0 → "-2"，1.0 → "1"。
        format!("Point({}, {})", self.x, self.y)
    }
}

/// 练习 9-3：对每个元素调用 `describe()`，把结果按顺序收集成 `Vec<String>`。
///
/// 参数是 **trait 对象切片** `&[Box<dyn Describable>]`，里面可以混装任意实现了
/// `Describable` 的具体类型 —— 这正是 `dyn` 动态分发的用途。
///
/// 边界：`&[]` → 空 `Vec`（不要 panic，也不要 `unwrap`）。
///
/// 提示：`Box<dyn Describable>` 会自动解引用，直接写 `item.describe()` 即可，
/// 不需要 `(**item).describe()`。收集用 `items.iter().map(...).collect()`。
///
/// # 示例
/// ```
/// # use ch09_generics_traits::{describe_all, Describable, Point};
/// let items: Vec<Box<dyn Describable>> = vec![Box::new(Point { x: 1.5, y: -2.0 })];
/// assert_eq!(describe_all(&items), vec!["Point(1.5, -2)".to_string()]);
/// ```
pub fn describe_all(items: &[Box<dyn Describable>]) -> Vec<String> {
    // 参考答案：Box<dyn Describable> 自动解引用，直接 item.describe() 即可。
    items.iter().map(|item| item.describe()).collect()
}
