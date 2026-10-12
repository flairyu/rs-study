//! 第 16 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch16_stdlib_tour`（或 `cargo test -p ch16_stdlib_tour`）

use ch16_stdlib_tour::*;
use std::path::PathBuf;

/// 在系统临时目录里造一个**唯一**的临时文件名。
///
/// 用 `std::process::id()` + 测试名（tag）拼后缀：并行跑测试、或重复跑测试时都不会撞名。
/// 这里刻意不用第三方 `tempfile`——`exercises/` 下只允许依赖 std。
fn temp_file(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rs_study_ch16_{}_{}.txt", std::process::id(), tag))
}

/// 写文件、读结果、删文件，把「清理」放在断言之前，断言失败也不会留下垃圾文件。
fn count_in_new_file(tag: &str, content: &str) -> std::io::Result<usize> {
    let path = temp_file(tag);
    std::fs::write(&path, content).expect("写临时文件失败");
    let result = count_words_in_file(&path);
    let _ = std::fs::remove_file(&path);
    result
}

#[test]
fn ex16_1_count_words_in_file() {
    // 正常值：换行、制表符、连续空格都算分隔符
    assert_eq!(
        count_in_new_file("basic", "hello world\nfoo\tbar  baz").unwrap(),
        5,
        "任意空白分词，连续空白不算空词"
    );

    // 边界：空文件
    assert_eq!(count_in_new_file("empty", "").unwrap(), 0, "空文件应返回 0");

    // 边界：只有空白
    assert_eq!(
        count_in_new_file("blank", "  \n\t\r\n   ").unwrap(),
        0,
        "只有空白应返回 0"
    );

    // 多行 + 非 ASCII：按 Unicode 空白切分，中文词也算一个 word
    assert_eq!(
        count_in_new_file("mixed", "你好 世界\nRust is fast\n").unwrap(),
        5,
        "中文词各算一个"
    );

    // 文件不存在：必须返回 Err（而不是 panic，也不是 Ok(0)）
    let missing = temp_file("missing");
    let _ = std::fs::remove_file(&missing);
    assert!(
        count_words_in_file(&missing).is_err(),
        "文件不存在时应把 io::Error 原样返回"
    );
}

#[test]
fn ex16_2_file_stem_and_ext() {
    assert_eq!(
        file_stem_and_ext("notes.txt"),
        ("notes".to_string(), Some("txt".to_string())),
        "普通文件"
    );
    assert_eq!(
        file_stem_and_ext("/tmp/archive.tar.gz"),
        ("archive.tar".to_string(), Some("gz".to_string())),
        "扩展名只取最后一个点之后的部分"
    );
    assert_eq!(
        file_stem_and_ext("dir/sub/README"),
        ("README".to_string(), None),
        "有主名无扩展名"
    );
    assert_eq!(
        file_stem_and_ext("a/b/c.txt"),
        ("c".to_string(), Some("txt".to_string())),
        "多级目录只看最后一段"
    );
    assert_eq!(
        file_stem_and_ext("./a/b/"),
        ("b".to_string(), None),
        "结尾斜杠不影响最后一段"
    );

    // 无主名：空串 / 根目录 / 当前目录 / 上级目录
    assert_eq!(file_stem_and_ext(""), ("".to_string(), None), "空串无主名");
    assert_eq!(
        file_stem_and_ext("/"),
        ("".to_string(), None),
        "根目录无主名"
    );
    assert_eq!(
        file_stem_and_ext("."),
        ("".to_string(), None),
        "当前目录无主名"
    );
    assert_eq!(
        file_stem_and_ext(".."),
        ("".to_string(), None),
        "上级目录无主名"
    );

    // 以点开头的隐藏文件：整体是主名，没有扩展名
    assert_eq!(
        file_stem_and_ext(".gitignore"),
        (".gitignore".to_string(), None),
        "以点开头的文件名没有扩展名"
    );
}

#[test]
fn ex16_3_round_trip_number() {
    // 用对二进制浮点不敏感的输入，避开 2.005 这类「看起来能进位」的坑
    assert_eq!(round_trip_number("3.14159").unwrap(), "3.14");
    assert_eq!(round_trip_number("2.5").unwrap(), "2.50", "两位小数会补零");
    assert_eq!(
        round_trip_number("42").unwrap(),
        "42.00",
        "整数输入也要两位小数"
    );
    assert_eq!(round_trip_number("0").unwrap(), "0.00");
    assert_eq!(round_trip_number("-3.14159").unwrap(), "-3.14", "负数");
    assert_eq!(
        round_trip_number("12.999").unwrap(),
        "13.00",
        "四舍五入到 2 位"
    );
    assert_eq!(
        round_trip_number("1e3").unwrap(),
        "1000.00",
        "科学计数法也是合法 f64"
    );
    assert_eq!(
        round_trip_number(&round_trip_number("3.14159").unwrap()).unwrap(),
        "3.14",
        "结果再解析一次应当稳定"
    );

    // 解析失败：原样返回 Err，不能 panic
    assert!(round_trip_number("abc").is_err(), "非数字应返回 Err");
    assert!(round_trip_number("").is_err(), "空串应返回 Err");
    assert!(round_trip_number("1.2.3").is_err(), "多个小数点应返回 Err");
    assert!(round_trip_number(" 42 ").is_err(), "parse 不做 trim");
}
