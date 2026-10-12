//! 第 9 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch09_generics_traits`（或 `cargo test -p ch09_generics_traits`）

use ch09_generics_traits::*;

#[test]
fn ex9_1_largest() {
    // 边界：空切片没有最大值
    assert_eq!(largest::<i32>(&[]), None, "空切片应返回 None");

    // 边界：单元素，最大就是它自己
    assert_eq!(largest(&[42]), Some(42), "单元素切片应返回该元素");

    // 普通情形：最大值在中间 / 在末尾 / 在开头
    assert_eq!(largest(&[1, 5, 3, 4]), Some(5));
    assert_eq!(largest(&[1, 2, 3, 99]), Some(99));
    assert_eq!(largest(&[100, 2, 3, 4]), Some(100));

    // 边界：全部相等，返回其中之一即可
    assert_eq!(
        largest(&[7, 7, 7, 7]),
        Some(7),
        "元素全部相等时返回任意一个"
    );

    // 边界：负数
    assert_eq!(
        largest(&[-5, -1, -9]),
        Some(-1),
        "负数里最大值是最大的那个负数"
    );
    assert_eq!(largest(&[-3, -3]), Some(-3));

    // 同一份泛型代码必须能对 &str 调用
    assert_eq!(largest(&["apple", "pear", "fig"]), Some("pear"));
    assert_eq!(
        largest(&["same", "same"]),
        Some("same"),
        "字符串相等时返回任意一个"
    );
    assert_eq!(largest::<&str>(&[]), None);
}

#[test]
fn ex9_2_describable_point() {
    let p = Point { x: 1.5, y: -2.0 };

    // 浮点用 {} 打印：1.5 → "1.5"，-2.0 → "-2"（不是 "-2.0"）
    assert_eq!(
        p.describe(),
        "Point(1.5, -2)",
        "describe 必须用 f64 的默认打印规则（-2.0 打印成 -2）"
    );
    assert_eq!(p.describe(), format!("Point({}, {})", p.x, p.y));

    // 整数取值的 f64 也不补小数点和零
    assert_eq!(Point { x: 1.0, y: 0.0 }.describe(), "Point(1, 0)");
    assert_eq!(Point { x: -3.0, y: 4.0 }.describe(), "Point(-3, 4)");

    // 派生出来的 Debug / Clone / PartialEq 也要能用
    let q = p.clone();
    assert_eq!(p, q, "Point 应派生 PartialEq 且支持 clone");
    assert_eq!(
        format!("{:?}", q),
        "Point { x: 1.5, y: -2.0 }",
        "Debug 派生输出"
    );

    // 也可以走 trait 对象调用
    let obj: &dyn Describable = &p;
    assert_eq!(obj.describe(), "Point(1.5, -2)");
}

#[test]
fn ex9_3_describe_all() {
    // 边界：空切片 → 空 Vec（不能 panic）
    let empty: Vec<Box<dyn Describable>> = Vec::new();
    assert_eq!(
        describe_all(&empty),
        Vec::<String>::new(),
        "空输入应返回空 Vec"
    );

    // 混合多种实现类型：Box<dyn Describable> 里装不同的具体类型
    let mut items: Vec<Box<dyn Describable>> = Vec::new();
    items.push(Box::new(Point { x: 1.5, y: -2.0 }));
    items.push(Box::new(Origin));

    assert_eq!(
        describe_all(&items),
        vec!["Point(1.5, -2)".to_string(), "原点".to_string()]
    );

    // 单个元素
    let one: Vec<Box<dyn Describable>> = vec![Box::new(Point { x: 0.0, y: 0.0 })];
    assert_eq!(describe_all(&one), vec!["Point(0, 0)".to_string()]);

    // 顺序必须原样保留，且长度与输入一致
    let all_points: Vec<Box<dyn Describable>> = vec![
        Box::new(Point { x: 1.0, y: 1.0 }),
        Box::new(Point { x: 2.5, y: 3.0 }),
    ];
    let out = describe_all(&all_points);
    assert_eq!(out.len(), 2, "输出长度应与输入长度一致");
    assert_eq!(out[0], "Point(1, 1)");
    assert_eq!(out[1], "Point(2.5, 3)");
}

/// 测试用的第二个 `Describable` 实现：证明 `dyn Describable` 不是 `Point` 专用的。
struct Origin;

impl Describable for Origin {
    fn describe(&self) -> String {
        "原点".to_string()
    }
}
