//! 第 7 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch07_collections`（或 `cargo test -p ch07_collections`）

use ch07_collections::*;

/// 浮点比较统一用容差，避免实现顺序不同带来的末位误差。
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn ex7_1_mean_and_median() {
    // 边界值：空切片
    assert_eq!(
        mean_and_median(&[]),
        None,
        "空切片没有均值与中位数，应返回 None"
    );

    // 边界值：单元素（奇数个的退化情形）
    let (mean, median) = mean_and_median(&[5]).unwrap();
    assert!(close(mean, 5.0), "单元素均值就是它自己");
    assert!(close(median, 5.0), "单元素中位数就是它自己");

    // 正常值：奇数个取中间
    let (mean, median) = mean_and_median(&[3, 1, 2]).unwrap();
    assert!(close(mean, 2.0), "[3,1,2] 的均值应为 2.0");
    assert!(close(median, 2.0), "奇数个元素的中位数应取排序后的中间值");

    // 边界值：偶数个取中间两数的平均值
    let (mean, median) = mean_and_median(&[4, 2, 1, 3]).unwrap();
    assert!(close(mean, 2.5), "[4,2,1,3] 的均值应为 2.5");
    assert!(close(median, 2.5), "偶数个元素应取中间两数 (2+3)/2 = 2.5");

    // 边界值：未排序输入也必须先排序；两数时中位数即两者平均
    let (_, median) = mean_and_median(&[10, 2]).unwrap();
    assert!(close(median, 6.0), "[10,2] 的中位数应为 (2+10)/2 = 6.0");

    // 边界值：负数
    let (mean, median) = mean_and_median(&[-5, -1, -3]).unwrap();
    assert!(close(mean, -3.0), "[-5,-1,-3] 的均值应为 -3.0");
    assert!(close(median, -3.0), "排序后 [-5,-3,-1] 的中间值是 -3");

    // 边界值：元素较多时不做整数除法
    let (mean, median) = mean_and_median(&[1, 2, 3, 4, 5, 6, 7]).unwrap();
    assert!(close(mean, 4.0), "1..=7 的均值应为 4.0");
    assert!(close(median, 4.0), "1..=7 的中位数应为 4.0");

    let (mean, _) = mean_and_median(&[1, 2]).unwrap();
    assert!(close(mean, 1.5), "均值必须是浮点，不能是整数除法得到的 1");

    // 入参不能被修改：中位数必须排的是副本
    let data = [9, 1, 5, 3, 7];
    let _ = mean_and_median(&data);
    assert_eq!(data, [9, 1, 5, 3, 7], "mean_and_median 不应修改入参切片");
}

#[test]
fn ex7_2_word_count() {
    // 边界值：空字符串与全空白
    assert!(word_count("").is_empty(), "空字符串应返回空 map");
    assert!(word_count("   \t\n ").is_empty(), "全空白应返回空 map");

    // 正常值：按空白分词
    let counts = word_count("the quick brown fox");
    assert_eq!(counts.len(), 4, "四个不同的词应产生四个 key");
    assert_eq!(counts["the"], 1);
    assert_eq!(counts["quick"], 1);
    assert_eq!(counts["fox"], 1);

    // 边界值：重复词与大小写混合都必须归一到小写
    let counts = word_count("Hello hello HELLO world World");
    assert_eq!(
        counts.len(),
        2,
        "大小写不同但拼写相同的词应合并成同一个 key"
    );
    assert_eq!(counts["hello"], 3, "Hello / hello / HELLO 应合并计数为 3");
    assert_eq!(counts["world"], 2, "world / World 应合并计数为 2");
    assert!(!counts.contains_key("Hello"), "key 必须是小写形式");

    // 边界值：连续空白、换行、制表符都不产生空字符串 key
    let counts = word_count("  a\t\tb\n\nc  ");
    assert_eq!(counts.len(), 3, "连续空白只能产生 3 个词，不能有空词");
    assert!(!counts.contains_key(""), "空串不是词，不能进 map");
    assert_eq!(counts["a"], 1);
    assert_eq!(counts["c"], 1);

    // 整段文本与逐词比较：HashMap 实现了 PartialEq，可以直接 assert_eq!
    assert_eq!(
        word_count("one two two"),
        word_count("TWO one TWO"),
        "计数结果应只与词频有关，与出现顺序、大小写无关"
    );

    // 边界值：含中文的字符串
    let counts = word_count("你好 世界 你好");
    assert_eq!(counts.len(), 2, "中文词同样按空白分词");
    assert_eq!(counts["你好"], 2, "中文重复词应正确累加");
    assert_eq!(counts["世界"], 1);

    // 中英混排 + 多字节字符
    let counts = word_count("Rust 你好 rust 你好");
    assert_eq!(counts["rust"], 2);
    assert_eq!(counts["你好"], 2);
    assert_eq!(counts.len(), 2);
}

#[test]
fn ex7_3_dedup_preserve_order() {
    // 边界值：空切片
    let empty: Vec<String> = Vec::new();
    assert!(
        dedup_preserve_order(&empty).is_empty(),
        "空切片应返回空 Vec"
    );

    // 边界值：单元素
    let one = vec!["solo".to_string()];
    assert_eq!(
        dedup_preserve_order(&one),
        vec!["solo".to_string()],
        "单元素原样返回"
    );

    // 正常值：保序去重
    let items: Vec<String> = ["b", "a", "b", "c", "a"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let expected: Vec<String> = ["b", "a", "c"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        dedup_preserve_order(&items),
        expected,
        "应保留首次出现顺序并去掉不相邻的重复项"
    );

    // 边界值：全部相同 → 只留一个
    let same: Vec<String> = ["x", "x", "x", "x"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        dedup_preserve_order(&same),
        vec!["x".to_string()],
        "全部重复时应只剩一个"
    );

    // 边界值：本来就没有重复 → 原样返回（顺序也不能变）
    let unique: Vec<String> = ["z", "y", "x"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        dedup_preserve_order(&unique),
        unique,
        "无重复时顺序与内容都不变"
    );

    // 边界值：空字符串是合法元素，也有自己的身份
    let with_empty: Vec<String> = ["", "a", "", "b"].iter().map(|s| s.to_string()).collect();
    let expected: Vec<String> = ["", "a", "b"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        dedup_preserve_order(&with_empty),
        expected,
        "空字符串也应参与去重"
    );

    // 边界值：含中文的字符串
    let zh: Vec<String> = ["你好", "世界", "你好", "Rust"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let expected: Vec<String> = ["你好", "世界", "Rust"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        dedup_preserve_order(&zh),
        expected,
        "多字节字符同样按内容去重"
    );
}
