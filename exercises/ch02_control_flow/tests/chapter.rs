//! 第 2 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch02_control_flow`（或 `cargo test -p ch02_control_flow`）
//!
//! 每个测试内部都覆盖：正常值 + 边界值（`0` / `1` / 极大值）。

use ch02_control_flow::*;

#[test]
fn ex2_1_fizzbuzz() {
    // 正常值：三种特殊结果各来一遍
    assert_eq!(fizzbuzz(3), "Fizz", "3 只被 3 整除");
    assert_eq!(fizzbuzz(5), "Buzz", "5 只被 5 整除");
    assert_eq!(fizzbuzz(15), "FizzBuzz", "15 同时被 3 和 5 整除");
    assert_eq!(fizzbuzz(30), "FizzBuzz", "30 同时被 3 和 5 整除");
    assert_eq!(fizzbuzz(6), "Fizz", "6 只被 3 整除");
    assert_eq!(fizzbuzz(10), "Buzz", "10 只被 5 整除");

    // 普通值：直接返回十进制字符串，不带空格、不带正号
    assert_eq!(fizzbuzz(1), "1", "1 两个都不整除，返回 \"1\"");
    assert_eq!(fizzbuzz(2), "2", "2 两个都不整除，返回 \"2\"");
    assert_eq!(fizzbuzz(7), "7", "7 两个都不整除，返回 \"7\"");
    assert_eq!(fizzbuzz(98), "98", "98 两个都不整除，返回 \"98\"");

    // 边界值：0 被任何非零整数整除，所以三个条件里第一个就命中
    assert_eq!(fizzbuzz(0), "FizzBuzz", "0 % 3 == 0 且 0 % 5 == 0");

    // 极大值：u32::MAX = 4294967295，数位和为 57（可被 3 整除）且以 5 结尾
    assert_eq!(
        fizzbuzz(u32::MAX),
        "FizzBuzz",
        "u32::MAX 同时被 3 和 5 整除"
    );
    // u32::MAX - 1 = 4294967294，数位和为 56、末位是 4 → 两个都不整除
    assert_eq!(
        fizzbuzz(u32::MAX - 1),
        "4294967294",
        "极大但不整除时，返回它的十进制字符串"
    );
}

#[test]
fn ex2_2_collatz_steps() {
    // 边界值：0 和 1 都不需要任何一步（0 若不提前返回会死循环）
    assert_eq!(collatz_steps(0), 0, "n = 0 约定返回 0");
    assert_eq!(collatz_steps(1), 0, "n = 1 已经在终点，返回 0");

    // 正常值：手工可验算的短序列
    assert_eq!(collatz_steps(2), 1, "2 -> 1，1 步");
    assert_eq!(collatz_steps(4), 2, "4 -> 2 -> 1，2 步");
    assert_eq!(
        collatz_steps(6),
        8,
        "6 -> 3 -> 10 -> 5 -> 16 -> 8 -> 4 -> 2 -> 1，8 步"
    );
    assert_eq!(collatz_steps(7), 16, "7 需要 16 步");
    assert_eq!(collatz_steps(27), 111, "27 是经典的 111 步");

    // 极大值：2 的幂只需要连续折半，步数等于指数（且不会溢出 u64）
    assert_eq!(collatz_steps(1024), 10, "2^10 折半 10 次");
    assert_eq!(collatz_steps(1u64 << 40), 40, "2^40 折半 40 次，仍然安全");
}

#[test]
fn ex2_3_membership_level() {
    // 边界值：两个维度都是 0 → 最底层
    assert_eq!(membership_level(0, 0), "青铜", "没有任何一条命中");

    // 阈值边界：差一点就是下一档
    assert_eq!(membership_level(0, 99), "青铜", "99 分还差 1 分到白银");
    assert_eq!(membership_level(0, 100), "白银", "100 分正好到白银");
    assert_eq!(membership_level(1, 0), "白银", "满 1 年就是白银");
    assert_eq!(
        membership_level(2, 499),
        "白银",
        "满 1 年但不到 3 年、不到 500 分"
    );
    assert_eq!(membership_level(0, 500), "黄金", "500 分正好到黄金");
    assert_eq!(membership_level(3, 0), "黄金", "满 3 年就是黄金");
    assert_eq!(
        membership_level(4, 999),
        "黄金",
        "满 3 年但不到 5 年、不到 1000 分"
    );
    assert_eq!(membership_level(0, 1000), "钻石", "1000 分正好到钻石");
    assert_eq!(membership_level(5, 0), "钻石", "满 5 年就是钻石");

    // 顺序敏感：points 高但 years 低时，也要命中前面那条 points 条件
    assert_eq!(
        membership_level(0, 1500),
        "钻石",
        "年限为 0 但积分够高仍是钻石"
    );
    assert_eq!(membership_level(1, 500), "黄金", "先命中黄金，不能掉到白银");
    assert_eq!(
        membership_level(4, 1000),
        "钻石",
        "先命中钻石，不能掉到黄金"
    );
    assert_eq!(membership_level(5, 1000), "钻石", "两个条件都满足");

    // 极大值：两个维度都拉满
    assert_eq!(
        membership_level(u32::MAX, u32::MAX),
        "钻石",
        "极大值仍然落在最高档"
    );
}
