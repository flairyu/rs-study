//! 第 8 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch08_iterators_closures`（或 `cargo test -p ch08_iterators_closures`）

use ch08_iterators_closures::*;
use std::collections::BTreeMap;

#[test]
fn ex8_1_sum_of_squares_of_evens() {
    assert_eq!(sum_of_squares_of_evens(&[]), 0, "空切片应返回 0");
    assert_eq!(
        sum_of_squares_of_evens(&[1, 3, 5, 7]),
        0,
        "全是奇数应返回 0"
    );
    assert_eq!(sum_of_squares_of_evens(&[2]), 4, "只有一个偶数");
    assert_eq!(
        sum_of_squares_of_evens(&[1, 2, 3, 4]),
        20,
        "2² + 4² = 20，奇数要被过滤掉"
    );
    assert_eq!(
        sum_of_squares_of_evens(&[-1, -2, -3, -4]),
        20,
        "负偶数的平方也是正数"
    );
    assert_eq!(
        sum_of_squares_of_evens(&[0, 0, 1]),
        0,
        "0 是偶数，平方还是 0"
    );
    assert_eq!(
        sum_of_squares_of_evens(&[10, 11, 12]),
        244,
        "100 + 144 = 244"
    );
    assert_eq!(
        sum_of_squares_of_evens(&[46_341, 46_340]),
        2_147_395_600,
        "返回类型是 i64：46340² 已经超出 i32 的范围，平方前就要转换"
    );
}

#[test]
fn ex8_2_apply_n_times() {
    assert_eq!(
        apply_n_times(|v| v + 1, 0, 0),
        0,
        "n = 0 时应原样返回 x（f 一次也不调用）"
    );
    assert_eq!(
        apply_n_times(|v| v + 1, 7, 0),
        7,
        "n = 0 时不能改变 x，即使 f 有副作用"
    );
    assert_eq!(apply_n_times(|v| v + 1, 0, 5), 5, "加 1 连续应用 5 次");
    assert_eq!(
        apply_n_times(|v| v * 2, 3, 4),
        48,
        "乘 2 连续应用 4 次：3 → 6 → 12 → 24 → 48"
    );
    assert_eq!(
        apply_n_times(|v| v - 3, 10, 6),
        -8,
        "闭包可以做减法和负数运算"
    );
    assert_eq!(
        apply_n_times(|v| v.abs() + 1, -5, 3),
        8,
        "|-5| + 1 = 6，|6| + 1 = 7，|7| + 1 = 8"
    );
    assert_eq!(
        apply_n_times(|v| v + 1, 3, 2) + apply_n_times(|v| v * 10, 3, 2),
        305,
        "同一个闭包传给两次调用，Fn 可以被重复调用：5 + 300"
    );
}

#[test]
fn ex8_3_count_by_length() {
    let expected: BTreeMap<usize, usize> = [(3, 2), (4, 1)].into_iter().collect();
    assert_eq!(
        count_by_length(&["abc", "def", "ghij"]),
        expected,
        "长度 3 出现 2 次、长度 4 出现 1 次"
    );
    assert_eq!(count_by_length(&[]), BTreeMap::new(), "空输入应返回空 map");

    let with_empty = count_by_length(&["", "", "a"]);
    assert_eq!(
        with_empty.get(&0),
        Some(&2),
        "空字符串的长度是 0，也要被统计"
    );
    assert_eq!(with_empty.get(&1), Some(&1), "长度为 1 的字符串出现 1 次");

    let repeated = count_by_length(&["ab", "cd", "ef", "gh"]);
    assert_eq!(repeated.len(), 1, "重复长度只产生一个 key");
    assert_eq!(repeated.get(&2), Some(&4), "重复长度要累加而不是覆盖");

    let mixed = count_by_length(&["x", "yy", "zz", "www", "vvvv"]);
    let keys: Vec<usize> = mixed.keys().copied().collect();
    assert_eq!(keys, vec![1, 2, 3, 4], "BTreeMap 的 key 必须按升序排列");
    assert_eq!(mixed.get(&2), Some(&2), "长度为 2 的出现 2 次");
    assert_eq!(mixed.get(&1), Some(&1), "长度为 1 的出现 1 次");
    assert_eq!(mixed.get(&3), Some(&1), "长度为 3 的出现 1 次");
    assert_eq!(mixed.get(&4), Some(&1), "长度为 4 的出现 1 次");
}
