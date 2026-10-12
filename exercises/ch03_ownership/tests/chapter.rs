//! 第 3 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch03_ownership`（或 `cargo test -p ch03_ownership`）

use ch03_ownership::*;

#[test]
fn ex3_1_shout() {
    // 正常值
    assert_eq!(
        shout(String::from("hello")),
        "HELLO",
        "普通小写应变成全大写"
    );
    assert_eq!(
        shout(String::from("Hello, World!")),
        "HELLO, WORLD!",
        "标点与空格应原样保留"
    );

    // 边界值：空字符串
    assert_eq!(shout(String::new()), "", "空字符串应返回空字符串");

    // 边界值：多字节 UTF-8（汉字没有大小写，应原样保留）
    assert_eq!(
        shout(String::from("中文")),
        "中文",
        "汉字没有大小写，应原样保留"
    );
    assert_eq!(
        shout(String::from("café 中文")),
        "CAFÉ 中文",
        "带重音的拉丁字母变大写，汉字不变"
    );
}

#[test]
fn ex3_2_join_and_count() {
    // 正常值
    assert_eq!(
        join_and_count(vec!["hello".to_string(), "world".to_string()]),
        (2, String::from("hello world")),
        "两个单词应返回 (2, \"hello world\")"
    );
    assert_eq!(
        join_and_count(vec!["中文".to_string(), "测试".to_string()]),
        (2, String::from("中文 测试")),
        "多字节 UTF-8 单词按原样拼接，中间一个空格"
    );

    // 边界值：空 Vec
    assert_eq!(
        join_and_count(Vec::new()),
        (0, String::new()),
        "空 Vec 应返回 (0, \"\")"
    );

    // 边界值：单个元素，不能多出空格
    assert_eq!(
        join_and_count(vec![String::from("solo")]),
        (1, String::from("solo")),
        "只有 1 个单词时不应带空格"
    );
    assert_eq!(
        join_and_count(vec![String::new()]),
        (1, String::new()),
        "单个空字符串仍是 1 个元素，连接结果为空"
    );

    // 边界值：元素内部含空格不改变计数（个数 = 元素个数，不做二次切分）
    assert_eq!(
        join_and_count(vec!["a b".to_string(), "c".to_string()]),
        (2, String::from("a b c")),
        "单词个数等于元素个数，不做二次切分"
    );
}

#[test]
fn ex3_3_consume_and_give_back() {
    // 正常值：还回来的字符串必须还能继续使用（所有权确实回到了调用方）
    let s = String::from("abc");
    let (s, n) = consume_and_give_back(s);
    assert_eq!(s, "abc", "原字符串应原样还回来");
    assert_eq!(n, 3, "\"abc\" 的字节长度是 3");
    assert_eq!(s.len(), n, "还回来的字符串应该还能用，长度与报告值一致");
    assert_eq!(format!("{s}{s}"), "abcabc", "转移回来的值可以继续拼接使用");

    // 边界值：空字符串
    let (s, n) = consume_and_give_back(String::new());
    assert_eq!(s, "", "空字符串应原样返回");
    assert_eq!(n, 0, "空字符串的字节长度是 0");

    // 边界值：多字节 UTF-8 —— len() 数的是字节，不是字符
    let (s, n) = consume_and_give_back(String::from("中文"));
    assert_eq!(s, "中文", "中文字符串应原样返回");
    assert_eq!(n, 6, "\"中文\" 的 UTF-8 字节长度是 6，不是字符数 2");
    assert_eq!(
        String::from("中文").chars().count(),
        2,
        "对照：字符数确实是 2"
    );

    // 边界值：ASCII 与三字节汉字混合（3 + 3 = 6）
    let (s, n) = consume_and_give_back(String::from("ab中"));
    assert_eq!(s, "ab中", "混合内容应原样返回");
    assert_eq!(n, 5, "\"ab中\" 的字节长度是 2 + 3 = 5");
}
