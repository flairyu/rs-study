//! 第 12 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch12_concurrency`（或 `cargo test -p ch12_concurrency`）
//!
//! 注意：这里**不用 `sleep` 做同步**。线程之间只靠 `join` 与通道关闭语义对齐——
//! 用 `sleep` 的测试只是"碰巧能过"，不是真的同步。

use ch12_concurrency::*;

#[test]
fn ex12_1_parallel_sum() {
    // 边界：空输入，分片前必须先返回，否则 chunks(0) 会 panic
    assert_eq!(parallel_sum(vec![]), 0, "空 Vec 应返回 0");

    // 边界：单元素 / 恰好 4 个元素
    assert_eq!(parallel_sum(vec![42]), 42, "单元素应原样返回");
    assert_eq!(
        parallel_sum(vec![i32::MAX]),
        i32::MAX,
        "单元素不应有额外运算"
    );
    assert_eq!(parallel_sum(vec![-7]), -7, "单个负数");
    assert_eq!(parallel_sum(vec![1, 2, 3, 4]), 10, "正好 4 个元素");

    // 关键边界：元素个数不是 4 的倍数，一个都不能漏
    assert_eq!(parallel_sum(vec![1, 2]), 3, "2 个元素（chunk = 1）");
    assert_eq!(parallel_sum(vec![1, 2, 3]), 6, "3 个元素（chunk = 1）");
    assert_eq!(
        parallel_sum(vec![1, 2, 3, 4, 5]),
        15,
        "5 个元素（chunk = 2）"
    );
    assert_eq!(
        parallel_sum(vec![1, 2, 3, 4, 5, 6, 7]),
        28,
        "7 个元素（chunk = 2）"
    );
    assert_eq!(
        parallel_sum((1..=10).collect()),
        55,
        "10 个元素（chunk = 3）"
    );
    assert_eq!(
        parallel_sum((1..=100).collect()),
        5050,
        "100 个元素（chunk = 25）"
    );

    // 边界：负数、零、极值混在一起（i32::MAX + i32::MIN == -1，不溢出）
    assert_eq!(parallel_sum(vec![-1, -2, -3, -4, -5]), -15, "全负数");
    assert_eq!(parallel_sum(vec![0, 0, 0, 0, 0, 0]), 0, "全零");
    assert_eq!(
        parallel_sum(vec![i32::MAX, i32::MIN, 0]),
        -1,
        "极值相加不溢出"
    );
    assert_eq!(
        parallel_sum(vec![7, 7, 7, 7, 7, 7, 7, 7, 7]),
        63,
        "全部相等"
    );
}

#[test]
fn ex12_2_counter_with_mutex() {
    // 边界：threads == 0 必须直接返回 0，不能死等
    assert_eq!(counter_with_mutex(0, 1), 0, "threads=0 应立即返回 0");
    assert_eq!(
        counter_with_mutex(0, 5),
        0,
        "threads=0 时 per_thread 无关紧要"
    );
    assert_eq!(counter_with_mutex(0, 0), 0, "threads=0 且 per_thread=0");

    // 边界：per_thread == 0，线程照起但不加
    assert_eq!(counter_with_mutex(1, 0), 0, "per_thread=0 结果为 0");
    assert_eq!(
        counter_with_mutex(4, 0),
        0,
        "多线程 per_thread=0 结果仍为 0"
    );

    // 正常值：单线程、多线程、互质数量
    assert_eq!(counter_with_mutex(1, 1), 1, "单线程加一次");
    assert_eq!(counter_with_mutex(1, 1000), 1000);
    assert_eq!(counter_with_mutex(3, 7), 21, "3 * 7");
    assert_eq!(counter_with_mutex(4, 1000), 4000, "4 个线程各加 1000 次");
    assert_eq!(
        counter_with_mutex(8, 12_345),
        98_760,
        "8 * 12345，锁不能丢更新"
    );
}

#[test]
fn ex12_3_producer_consumer() {
    // 边界：空 items → 空 Vec（生产者不发任何消息，通道立刻关闭）
    assert_eq!(
        producer_consumer(vec![]),
        Vec::<i32>::new(),
        "空 items 应返回空 Vec"
    );

    // 边界：单元素
    assert_eq!(producer_consumer(vec![1]), vec![2], "单元素乘 2");
    assert_eq!(producer_consumer(vec![0]), vec![0], "0 * 2 == 0");

    // 正常值：乱序输入必须返回升序
    assert_eq!(
        producer_consumer(vec![3, 1, 2]),
        vec![2, 4, 6],
        "输出必须排序"
    );
    assert_eq!(producer_consumer(vec![5, 4, 3, 2, 1]), vec![2, 4, 6, 8, 10]);
    assert_eq!(
        producer_consumer(vec![10, 8, 6, 4, 2]),
        vec![4, 8, 12, 16, 20]
    );

    // 边界：负数、零、重复元素
    assert_eq!(
        producer_consumer(vec![0, -1, -2]),
        vec![-4, -2, 0],
        "负数排序（-4 < -2 < 0）"
    );
    assert_eq!(
        producer_consumer(vec![5, 5, 5]),
        vec![10, 10, 10],
        "重复元素要保留"
    );
    assert_eq!(
        producer_consumer(vec![i32::MIN / 2]),
        vec![i32::MIN],
        "i32::MIN/2 乘 2 恰好等于 i32::MIN，不溢出"
    );

    // 元素个数不是 4 的倍数也一样
    assert_eq!(
        producer_consumer((1..=10).collect()),
        (1..=10).map(|x| x * 2).collect::<Vec<i32>>(),
        "10 个元素全部收到且有序"
    );
}
