//! # 第 15 章 · 测试、unsafe 与 FFI
//!
//! 本章 3 个练习只用到标准库。`abs` 与 `strlen` 是宿主 C 运行库（libc）里
//! 本来就有的函数，通过 `extern "C"` **直接声明**即可调用，不需要任何第三方 crate。
//!
//! 运行评测：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch15_testing_unsafe
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`read_raw`] | 解引用裸指针；`unsafe fn` 里为什么还要写 `unsafe` 块 |
//! | [`abs_via_ffi`] | `extern "C"` 声明、ABI、`unsafe` 调用点、跨 FFI 的 UB 风险 |
//! | [`c_strlen`] | `CString` / NUL 结尾 / `strlen` 数的是**字节**不是字符 |
//!
//! 提示：先读 `docs/01-syntax/ch15-testing-unsafe.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

// FFI 声明并不是"函数定义"，而是告诉 Rust：链接时会有这几个 C 符号。
// 注意它们都在 `extern` 块里，**没有函数体**。
extern "C" {
    /// C 标准库的 `abs`：返回 `int` 的绝对值。
    fn abs(input: i32) -> i32;

    /// C 标准库的 `strlen`：返回 NUL 之前的字节数（不含 NUL 本身）。
    fn strlen(s: *const std::os::raw::c_char) -> usize;
}

/// 练习 15-1：解引用原始指针并读出它指向的 `i32`。
///
/// 参数 `ptr` 是 **裸指针**（`*const i32`）。裸指针和 `&i32` 的区别：
///
/// - `&i32` 是**引用**：一定非空、一定对齐、一定指向存活的有效数据，编译器替你保证。
/// - `*const i32` 只是一串地址：可能是空、可能悬垂、可能不对齐，编译器一概不管。
///
/// 所以解引用必须写在 `unsafe { }` 里——你是在向编译器**担保**前置条件已经成立。
///
/// # 安全性（Safety）
///
/// 调用者必须保证 `ptr` 非空、按 4 字节对齐，并且指向一个仍存活且可读的 `i32`。
/// 本练习的测试只传入 `&x` 转出来的指针，因此满足上述条件。
///
/// # 示例
/// ```
/// # use ch15_testing_unsafe::read_raw;
/// let x: i32 = 42;
/// let p: *const i32 = &x;
/// assert_eq!(unsafe { read_raw(p) }, 42);
/// ```
#[allow(unused_unsafe)] // edition 2021 里 unsafe fn 的函数体算作 unsafe 上下文，显式 unsafe 块会多一条告警
pub unsafe fn read_raw(ptr: *const i32) -> i32 {
    unimplemented!("练习 15-1：用 unsafe 块解引用裸指针，返回它指向的值")
}

/// 练习 15-2：把 C 运行库的 `abs` 包装成安全的 Rust 函数。
///
/// 这里演示 FFI 的典型分层：**不安全的边界只开一个口子，对外的函数是安全的**。
///
/// - `#[allow(...)]` 之外没什么魔法：`abs_via_ffi` 本身是 `pub fn`（不是 `unsafe fn`），
///   因为对**任何** `i32` 它都不会捅出内存安全问题。
/// - 真正调用 C 函数的那一行必须包在 `unsafe { }` 里。
///
/// # 跨 FFI 的坑
///
/// C 函数完全不受 Rust 检查：传错类型、传空指针、忘了 NUL 结尾，结果都是 UB。
/// 特别地，**不要**传入 `i32::MIN`：C 标准规定 `abs(INT_MIN)` 是未定义行为
/// （补码里 `-INT_MIN` 无法表示），本练习的测试刻意避开了它。
///
/// # 示例
/// ```
/// # use ch15_testing_unsafe::abs_via_ffi;
/// assert_eq!(abs_via_ffi(-7), 7);
/// assert_eq!(abs_via_ffi(0), 0);
/// ```
pub fn abs_via_ffi(x: i32) -> i32 {
    unimplemented!("练习 15-2：在 unsafe 块里调用上面声明的 C 函数 abs")
}

/// 练习 15-3：用 C 的 `strlen` 数一个 Rust 字符串的**字节数**。
///
/// Rust 的 `&str` 不是 NUL 结尾的（它自带长度），C 的 `strlen` 却依赖结尾的 `\0`，
/// 所以中间要有一步转换：`std::ffi::CString::new(s)` 会分配一块
/// 「内容 + 结尾 `\0`」的内存，`as_ptr()` 给出指向它的裸指针。
///
/// 边界规则：
///
/// - `s` 为空串 → `CString::new("")` 成功，`strlen` 返回 `0`。
/// - `s` 含内部 `\0`（如 `"a\0b"`）→ `CString::new` 返回 `Err`，本函数**返回 0**。
///   这是必须的：如果不检查，C 侧会在第一个 `\0` 就停下，静默算错长度。
/// - `strlen` 数的是 **UTF-8 字节**，不是字符：`"中文"` 是 2 个字符、**6** 个字节，返回 `6`。
///
/// # 示例
/// ```
/// # use ch15_testing_unsafe::c_strlen;
/// assert_eq!(c_strlen("hello"), 5);
/// assert_eq!(c_strlen("中文"), 6);      // UTF-8 每个汉字 3 字节
/// assert_eq!(c_strlen("a\0b"), 0);      // 内部 NUL：CString::new 报错
/// ```
pub fn c_strlen(s: &str) -> usize {
    unimplemented!("练习 15-3：CString::new(s) 成功后用 unsafe 调用 strlen，失败返回 0")
}
