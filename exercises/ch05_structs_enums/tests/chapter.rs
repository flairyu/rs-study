//! 第 5 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch05_structs_enums`（或 `cargo test -p ch05_structs_enums`）

use ch05_structs_enums::*;

/// 浮点比较统一用容差，避免实现顺序不同带来的末位误差。
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn ex5_1_rectangle() {
    // 正常值
    let r = Rectangle::new(3.0, 4.0);
    assert!(close(r.width, 3.0), "new 应把 width 存下来");
    assert!(close(r.height, 4.0), "new 应把 height 存下来");
    assert!(close(r.area(), 12.0), "3x4 的面积应为 12");
    assert!(!r.is_square(), "3x4 不是正方形");

    // 边界：面积为 0（某一边为 0）
    let flat = Rectangle::new(0.0, 5.0);
    assert!(close(flat.area(), 0.0), "一边为 0 时面积应为 0");
    assert!(!flat.is_square(), "0x5 不是正方形");

    // 边界：负数边长，按代数乘法处理，不 panic
    let neg = Rectangle::new(-2.0, 3.0);
    assert!(close(neg.area(), -6.0), "负数边长直接相乘，应为 -6");
    let neg_both = Rectangle::new(-2.0, -3.0);
    assert!(close(neg_both.area(), 6.0), "两边都为负时面积应为正 6");

    // is_square 的容差 1e-9
    assert!(Rectangle::new(2.5, 2.5).is_square(), "2.5x2.5 是正方形");
    assert!(
        Rectangle::new(1.0, 1.0 + 1e-10).is_square(),
        "1e-10 的差在容差内"
    );
    assert!(
        !Rectangle::new(1.0, 1.0 + 1e-6).is_square(),
        "1e-6 的差超出容差"
    );
    assert!(
        Rectangle::new(0.0, 0.0).is_square(),
        "0x0 两边相等，视为正方形"
    );

    // &mut self：scale 就地修改
    let mut scaled = Rectangle::new(2.0, 3.0);
    scaled.scale(2.0);
    assert!(
        close(scaled.width, 4.0) && close(scaled.height, 6.0),
        "scale(2.0) 应把两边都翻倍"
    );
    assert!(close(scaled.area(), 24.0), "翻倍后面积应为原来的 4 倍");
    scaled.scale(0.0);
    assert!(close(scaled.area(), 0.0), "scale(0.0) 后面积应为 0");

    // 字段是 pub 的：外部可以直接改写
    let mut direct = Rectangle::new(1.0, 1.0);
    direct.width = 6.0;
    assert!(close(direct.area(), 6.0), "pub 字段应能从外部改写");

    // #[derive(Debug, Clone, PartialEq)]
    let a = Rectangle::new(1.0, 2.0);
    let b = a.clone();
    assert_eq!(a, b, "Clone + PartialEq 应可用");
    assert_ne!(a, Rectangle::new(2.0, 2.0), "字段不同应不相等");
    assert!(
        format!("{a:?}").contains("Rectangle"),
        "Debug 输出应包含类型名，实际是 {a:?}"
    );
}

#[test]
fn ex5_2_shape_area() {
    use std::f64::consts::PI;

    // 圆：正常值 + 半径为 0 的边界
    assert!(
        close(shape_area(&Shape::Circle(1.0)), PI),
        "单位圆面积应为 π"
    );
    assert!(
        close(shape_area(&Shape::Circle(2.0)), 4.0 * PI),
        "r=2 的圆面积应为 4π"
    );
    assert!(
        close(shape_area(&Shape::Circle(0.0)), 0.0),
        "半径为 0 的圆面积应为 0"
    );

    // 矩形：正常值 + 面积为 0 + 负数边长
    assert!(
        close(shape_area(&Shape::Rectangle(3.0, 4.0)), 12.0),
        "3x4 矩形面积应为 12"
    );
    assert!(
        close(shape_area(&Shape::Rectangle(0.0, 9.0)), 0.0),
        "一边为 0 的矩形面积应为 0"
    );
    assert!(
        close(shape_area(&Shape::Rectangle(-2.0, 5.0)), -10.0),
        "负数边长直接相乘，应为 -10"
    );

    // 三角形：0.5 * 底 * 高
    assert!(
        close(shape_area(&Shape::Triangle(6.0, 2.0)), 6.0),
        "底 6 高 2 的三角形面积应为 6"
    );
    assert!(
        close(shape_area(&Shape::Triangle(2.0, 3.0)), 3.0),
        "三角形应为 0.5*2*3，不是 6"
    );
    assert!(
        close(shape_area(&Shape::Triangle(0.0, 2.0)), 0.0),
        "底为 0 的三角形面积应为 0"
    );

    // #[derive(Debug, Clone, PartialEq)]：比较的是变体 + 携带的数据
    assert_eq!(Shape::Circle(1.0), Shape::Circle(1.0), "同变体同数据应相等");
    assert_ne!(Shape::Circle(1.0), Shape::Circle(2.0), "半径不同应不相等");
    assert_ne!(
        Shape::Rectangle(2.0, 3.0),
        Shape::Triangle(2.0, 3.0),
        "不同变体即使数据相同也不相等"
    );
    assert!(
        format!("{:?}", Shape::Triangle(1.5, 2.0)).contains("Triangle"),
        "Debug 输出应包含变体名"
    );
}

#[test]
fn ex5_3_describe_opt() {
    // 情况 1：None
    assert_eq!(describe_opt(None), "none", "None 应返回 \"none\"");
    // 情况 2：Some(0) —— 不能掉进正数分支
    assert_eq!(describe_opt(Some(0)), "zero", "Some(0) 应返回 \"zero\"");
    // 情况 3：正数
    assert_eq!(
        describe_opt(Some(5)),
        "positive: 5",
        "Some(5) 应返回 \"positive: 5\""
    );
    assert_eq!(
        describe_opt(Some(1)),
        "positive: 1",
        "Some(1) 应返回 \"positive: 1\""
    );
    assert_eq!(
        describe_opt(Some(i32::MAX)),
        format!("positive: {}", i32::MAX),
        "边界值 i32::MAX"
    );
    // 情况 4：负数
    assert_eq!(
        describe_opt(Some(-3)),
        "negative: -3",
        "Some(-3) 应返回 \"negative: -3\""
    );
    assert_eq!(
        describe_opt(Some(-1)),
        "negative: -1",
        "Some(-1) 应返回 \"negative: -1\""
    );
    assert_eq!(
        describe_opt(Some(i32::MIN)),
        format!("negative: {}", i32::MIN),
        "边界值 i32::MIN"
    );
}
