//! 第 14 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch14_modules_macros`（或 `cargo test -p ch14_modules_macros`）

// 只从 **crate 根** 导入：Point 能拿到，说明 lib.rs 里的 `pub use geometry::Point;` 生效了；
// hashmap_of 能拿到，说明 `#[macro_export]` 把宏放到了 crate 根。
use ch14_modules_macros::{hashmap_of, render, Point, Temperature};

/// 浮点比较统一用容差，避免实现顺序不同带来的末位误差。
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn ex14_1_point_distance() {
    let origin = Point::new(0.0, 0.0);

    // 正常值：3-4-5 直角三角形
    let p = Point::new(3.0, 4.0);
    assert!(close(p.distance_to(&origin), 5.0), "(3,4) 到原点距离应为 5");
    assert!(close(origin.distance_to(&p), 5.0), "距离应当对称");

    // 边界：距离为 0（同一点，包括点到自身）
    assert!(close(origin.distance_to(&origin), 0.0), "同一点距离应为 0");
    assert!(close(p.distance_to(&p), 0.0), "点到自身的距离应为 0");

    // 边界：负数坐标（先减后平方，符号消失）
    let neg = Point::new(-3.0, -4.0);
    assert!(
        close(neg.distance_to(&origin), 5.0),
        "(-3,-4) 到原点距离应为 5"
    );
    assert!(
        close(p.distance_to(&neg), 10.0),
        "(3,4) 到 (-3,-4) 距离应为 10"
    );
    assert!(close(neg.distance_to(&p), 10.0), "带负数坐标时距离也应对称");

    // 小数坐标
    let a = Point::new(1.5, 2.0);
    let b = Point::new(4.5, 6.0);
    assert!(
        close(a.distance_to(&b), 5.0),
        "(1.5,2) 到 (4.5,6) 距离应为 5"
    );
    assert!(close(a.distance_to(&a), 0.0), "小数坐标点到自身距离应为 0");

    // 字段可读 + 派生的 PartialEq / Debug（assert_eq! 需要 Debug）
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 4.0);
    assert_eq!(Point::new(1.0, -2.0), Point { x: 1.0, y: -2.0 });
}

#[test]
fn ex14_2_hashmap_of() {
    // 边界：空 map。类型必须显式标注——空分支只有 HashMap::new()，推不出 K / V。
    let empty: std::collections::HashMap<&str, i32> = hashmap_of! {};
    assert!(empty.is_empty(), "hashmap_of! 的空调用应返回空 HashMap");
    assert_eq!(empty.len(), 0, "空 map 的长度应为 0");

    // 边界：单元素 + 尾逗号
    let one: std::collections::HashMap<&str, i32> = hashmap_of! {"a" => 1,};
    assert_eq!(one.len(), 1, "单元素 map 的长度应为 1");
    assert_eq!(one.get("a"), Some(&1), "key \"a\" 应映射到 1");
    assert_eq!(one.get("b"), None, "不存在的 key 应返回 None");

    // 多元素：正数、零、负数都覆盖，且带尾逗号
    let m: std::collections::HashMap<&str, i32> = hashmap_of! {
        "x" => 10,
        "y" => 0,
        "z" => -7,
    };
    assert_eq!(m.len(), 3, "三个键值对应得到长度 3");
    assert_eq!(m.get("x"), Some(&10), "\"x\" 应映射到 10");
    assert_eq!(m.get("y"), Some(&0), "\"y\" 应映射到 0");
    assert_eq!(m.get("z"), Some(&-7), "\"z\" 应映射到 -7");
    assert_eq!(m.get("nope"), None, "不存在的 key 应返回 None");

    // 边界：没有尾逗号的写法同样必须合法
    let no_tail: std::collections::HashMap<&str, i32> = hashmap_of! {"k" => 1, "v" => 2};
    assert_eq!(no_tail.len(), 2, "无尾逗号的写法应同样合法");
    assert_eq!(no_tail.get("k"), Some(&1));
    assert_eq!(no_tail.get("v"), Some(&2));

    // 同名 key：按书写顺序 insert，后写的覆盖先写的
    let dup: std::collections::HashMap<&str, i32> = hashmap_of! {"a" => 1, "a" => 2};
    assert_eq!(dup.len(), 1, "重复 key 只应留下一个条目");
    assert_eq!(dup.get("a"), Some(&2), "重复 key 应保留后写入的值");
}

#[test]
fn ex14_3_temperature_display() {
    // 边界：0°C（注意 f64 用 {} 打印是 "0"，不是 "0.0"）
    assert_eq!(render(&Temperature { celsius: 0.0 }), "0°C");
    // 正常值
    assert_eq!(render(&Temperature { celsius: 100.0 }), "100°C");
    assert_eq!(render(&Temperature { celsius: 25.0 }), "25°C");
    // 边界：负温度
    assert_eq!(render(&Temperature { celsius: -40.0 }), "-40°C");
    // 边界：小数温度
    assert_eq!(render(&Temperature { celsius: 36.6 }), "36.6°C");
    assert_eq!(render(&Temperature { celsius: -0.5 }), "-0.5°C");

    // Display 自动带来 ToString（blanket impl），并且 {} 也能打印
    let t = Temperature { celsius: 21.5 };
    assert_eq!(t.to_string(), "21.5°C");
    assert_eq!(format!("{}", t), "21.5°C");
}
