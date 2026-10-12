//! 第 1 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch01_basics`（或 `cargo test -p ch01_basics`）

use ch01_basics::*;

/// 浮点比较统一用容差，避免实现顺序不同带来的末位误差。
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn ex1_1_fahrenheit_to_celsius() {
    assert!(close(fahrenheit_to_celsius(32.0), 0.0), "32°F 应该是 0°C");
    assert!(
        close(fahrenheit_to_celsius(212.0), 100.0),
        "212°F 应该是 100°C"
    );
    assert!(
        close(fahrenheit_to_celsius(-40.0), -40.0),
        "-40°F 应该等于 -40°C"
    );
    assert!(
        close(fahrenheit_to_celsius(98.6), 37.0),
        "98.6°F 约等于 37°C"
    );
}

#[test]
fn ex1_2_split_duration() {
    assert_eq!(split_duration(0), (0, 0, 0));
    assert_eq!(split_duration(59), (0, 0, 59));
    assert_eq!(split_duration(60), (0, 1, 0));
    assert_eq!(split_duration(3725), (1, 2, 5));
    assert_eq!(split_duration(86_399), (23, 59, 59));
}

#[test]
fn ex1_3_format_score_line() {
    assert_eq!(format_score_line("alice", 7, 8), "alice: 7/8 (87.5%)");
    assert_eq!(format_score_line("bob", 0, 10), "bob: 0/10 (0.0%)");
    assert_eq!(format_score_line("carol", 10, 10), "carol: 10/10 (100.0%)");
    assert_eq!(format_score_line("dave", 1, 3), "dave: 1/3 (33.3%)");
}
