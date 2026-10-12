//! 第 13 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch13_async`（或 `cargo test -p ch13_async`）
//!
//! 注意：这里所有断言都被 [`block_on`] 包住。`async fn` 本身不执行任何代码，
//! 只有被 poll 才开始跑——`block_on` 就是那个驱动器。

use ch13_async::*;

#[test]
fn ex13_1_async_add() {
    assert_eq!(block_on(async_add(1, 2)), 3, "1 + 2 应该等于 3");
    assert_eq!(block_on(async_add(0, 0)), 0, "0 + 0 应该等于 0");
    assert_eq!(block_on(async_add(-5, 3)), -2, "负数相加：-5 + 3 = -2");
    assert_eq!(
        block_on(async_add(-7, -8)),
        -15,
        "两个负数相加：-7 + (-8) = -15"
    );
    assert_eq!(block_on(async_add(100, -100)), 0, "正负抵消应该等于 0");
    assert_eq!(block_on(async_add(-1, 1)), 0, "边界：-1 + 1 = 0");
}

#[test]
fn ex13_2_async_sum() {
    assert_eq!(block_on(async_sum(&[])), 0, "空切片应该返回 0");
    assert_eq!(block_on(async_sum(&[42])), 42, "单元素应该返回它自己");
    assert_eq!(block_on(async_sum(&[0, 0, 0])), 0, "全零应该返回 0");
    assert_eq!(block_on(async_sum(&[1, 2, 3, 4])), 10, "1+2+3+4 = 10");
    assert_eq!(block_on(async_sum(&[-1, -2, -3])), -6, "全负数：-6");
    assert_eq!(
        block_on(async_sum(&[5, -5, 10, -10])),
        0,
        "正负抵消应该返回 0"
    );
}

#[test]
fn ex13_3_async_count_down() {
    assert_eq!(
        block_on(async_count_down(0)),
        Vec::<u32>::new(),
        "n = 0 应该返回空 Vec"
    );
    assert_eq!(block_on(async_count_down(1)), vec![1], "n = 1 应该返回 [1]");
    assert_eq!(
        block_on(async_count_down(2)),
        vec![2, 1],
        "n = 2 应该返回 [2, 1]"
    );
    assert_eq!(
        block_on(async_count_down(5)),
        vec![5, 4, 3, 2, 1],
        "n = 5 应该返回 [5, 4, 3, 2, 1]"
    );
}
