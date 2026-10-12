//! 第 15 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch15_testing_unsafe`（或 `cargo test -p ch15_testing_unsafe`）

use ch15_testing_unsafe::*;

#[test]
fn ex15_1_read_raw() {
    // 先把值绑到栈上，再显式取裸指针——不要写 `read_raw(&x)`：
    // `&x` 是 `&i32`，虽然能隐式 coerce，但显式转换更能看清这一章在干什么。
    let x: i32 = 42;
    let p: *const i32 = &x;
    assert_eq!(unsafe { read_raw(p) }, 42, "应读出 42");

    let neg: i32 = -7;
    let p_neg: *const i32 = &neg;
    assert_eq!(unsafe { read_raw(p_neg) }, -7, "负数也要原样读出");

    let zero: i32 = 0;
    let p_zero: *const i32 = &zero;
    assert_eq!(unsafe { read_raw(p_zero) }, 0, "0 是合法值，不是空指针");

    let big: i32 = i32::MAX;
    let p_big: *const i32 = &big;
    assert_eq!(unsafe { read_raw(p_big) }, i32::MAX, "边界值 i32::MAX");
}

#[test]
fn ex15_2_abs_via_ffi() {
    assert_eq!(abs_via_ffi(5), 5, "正数不变");
    assert_eq!(abs_via_ffi(-5), 5, "负数取绝对值");
    assert_eq!(abs_via_ffi(0), 0, "0 的绝对值是 0");
    assert_eq!(abs_via_ffi(-1), 1, "边界：-1 → 1");
    assert_eq!(abs_via_ffi(1), 1, "边界：1 → 1");
    assert_eq!(
        abs_via_ffi(-2147483647),
        2147483647,
        "i32::MIN + 1 仍然是可移植的"
    );
    // 刻意不测 abs_via_ffi(i32::MIN)：C 标准里 abs(INT_MIN) 是 UB，结果不可移植。
}

#[test]
fn ex15_3_c_strlen() {
    assert_eq!(c_strlen("hello"), 5, "ASCII：5 个字节");
    assert_eq!(
        c_strlen(""),
        0,
        "空字符串：CString::new(\"\") 成功，strlen 返回 0"
    );
    assert_eq!(c_strlen("a"), 1, "单字节");
    assert_eq!(
        c_strlen("中文"),
        6,
        "UTF-8：每个汉字 3 字节，2 个汉字共 6 字节"
    );
    assert_eq!(c_strlen("中"), 3, "UTF-8：单个汉字 3 字节");
    assert_eq!(c_strlen("a中"), 4, "混合：ASCII 1 字节 + 汉字 3 字节");
    assert_eq!(
        c_strlen("a\0b"),
        0,
        "含内部 \\0：CString::new 返回 Err，按约定返回 0"
    );
    assert_eq!(c_strlen("\0"), 0, "整体就是一个 NUL：同样返回 0");
}
