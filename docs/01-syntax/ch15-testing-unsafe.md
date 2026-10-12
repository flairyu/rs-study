# 第 15 章 · 测试、unsafe 与 FFI

> **一句话定位**：测试把「我以为对」变成「机器证明它对」；`unsafe` 把 C 的世界围进一圈栅栏，而不是把检查关掉。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 6 章（错误处理）、第 10 章（生命周期）
> **练习工程**：`exercises/ch15_testing_unsafe`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

| 话题 | C | Python | Rust |
|---|---|---|---|
| 测试框架 | 没有官方的，`assert.h` 手写 + 自建 runner | `pytest` / `unittest` | **内建** `#[test]`，cargo 就是 runner |
| 测试放哪 | 随便，通常单独文件 + Makefile | `tests/` 目录、`test_*.py` | 同文件 `#[cfg(test)] mod tests` **或** `tests/` 目录 |
| 文档里的示例 | 注释，没人执行 | doctest（`>>>`） | **doctest**：``` ```rust ``` 代码块默认会被 `cargo test` 执行 |
| 断言失败 | `assert` 宏会被 `NDEBUG` 关掉 | `assert` 抛 `AssertionError` | `assert_eq!` 永远在，还打印左右两边的值 |
| 期望 panic | 无法声明 | `pytest.raises` | `#[should_panic]` 属性 |

第二个锚点，也是本章的核心：

| 话题 | C / C++ | Rust |
|---|---|---|
| 「危险操作」的表达 | 没有标记，靠注释和纪律 | 必须写 `unsafe`，编译器替你标出**所有**危险点 |
| UB 的边界 | 整个程序处处可能 UB | 只有 `unsafe` 块及其调用链里可能 UB |
| `unsafe` 的含义 | —— | 「正确性我来担保」，**不是**「关掉检查」 |
| 借用检查 | 不存在 | **`unsafe` 里依然生效** |

```c
// C：没有任何标记告诉你它危险
int read_it(const int *p) { return *p; }   // p 可能是 NULL / 悬垂 / 未对齐
```

```rust
// Rust：危险被显式写出来，审计时只需看 unsafe 块
pub unsafe fn read_raw(ptr: *const i32) -> i32 {
    unsafe { *ptr }
}
```

`unsafe` 就像 C 里的 `/* 我知道这里危险 */` 注释，区别是：**注释编译器不看，`unsafe` 编译器强制要求**。
于是「哪里需要小心」从口头约定变成了类型系统的一部分。

---

## 1. 单元测试：`#[cfg(test)] mod tests`

```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]                 // 只在 `cargo test` 时编译，发布产物里不存在
mod tests {
    use super::*;            // 拿到父模块（crate 根）里的东西

    #[test]
    fn add_works() {
        assert_eq!(add(1, 2), 3);
        assert_eq!(add(0, 0), 0, "0 是合法输入");
        assert_eq!(add(-1, 1), 0, "正负相消应该得 0");
    }
}
```

1. `#[cfg(test)]` 让整块代码只活在测试构建里——`cargo build --release` 时它消失（对照 C 的 `#ifdef DEBUG`）。
2. 测试模块在**同一个文件**里，所以能访问私有函数和私有字段：这是单元测试最大的优势。
3. **测试失败 = panic**。一个 `#[test]` 里可以放多条断言，失败信息直接指出断在哪条、期望与实际是多少。
   想让断言失败变成 `Err` 以便用 `?`，就让函数返回 `Result<(), Box<dyn std::error::Error>>`（第 16 章展开）。

命名约定：小写下划线，描述「什么条件下、期望什么」。本仓库的自动评测统一用
`ex<章号>_<序号>_<函数名>`，例如 `ex15_2_abs_via_ffi`。

---

## 2. 集成测试：`tests/` 目录

```
exercises/ch15_testing_unsafe/
├── Cargo.toml
├── src/lib.rs          ← 你的实现
└── tests/chapter.rs    ← 公开 API 的裁判
```

`tests/` 下每个顶层 `.rs` 文件都会被编译成一个**独立 crate**，只能看到你库的 `pub` API——
就像任何外部使用者一样。这正是它的价值：**单元测试证明「内部逻辑对」，集成测试证明「对外承诺对」**。

```rust
// tests/chapter.rs
use ch15_testing_unsafe::*;   // 只能 use 到 pub 的东西

#[test]
fn ex15_2_abs_via_ffi() {
    assert_eq!(abs_via_ffi(-5), 5, "负数取绝对值");
}
```

| | 单元测试 `#[cfg(test)] mod tests` | 集成测试 `tests/*.rs` |
|---|---|---|
| 位置 | `src/` 内部 | 独立目录 |
| 能看到私有项 | ✅ | ❌ 只有 `pub` |
| 编译产物 | 同一个 crate 的测试版 | 单独的 test crate |
| 适合测 | 内部算法、边界分支 | 公开 API 的语义契约 |
| 本仓库用在哪 | 你写实现时自测 | `tools/check.py` 的自动评测 |

> ⚠️ **坑**：`tests/` 下只有**顶层** `.rs` 文件算测试目标；`tests/common/mod.rs` 这类子目录里的文件
> 默认不是独立测试（常用来放共享辅助代码）。另外测试默认**并行**跑，所以测试之间不能依赖执行顺序、
> 不要共享可变全局状态，否则会 flaky。

---

## 3. 文档测试与 `cargo test` 的三类输出

文档注释里的代码块会被 `cargo test` **真的编译并运行**：

````rust
/// 解引用原始指针读出值。
///
/// # 示例
/// ```
/// # use ch15_testing_unsafe::read_raw;   // `#` 开头的行渲染时隐藏，但仍参与编译
/// let x: i32 = 42;
/// let p: *const i32 = &x;
/// assert_eq!(unsafe { read_raw(p) }, 42);
/// ```
pub unsafe fn read_raw(ptr: *const i32) -> i32 { unsafe { *ptr } }
````

- 文档注释（`///`、`//!`）里的代码块默认就是 doctest；`ignore` 只显示不跑，`no_run` 编译但不跑。
- 副作用是**文档永远不会过时**：改了 API 却忘了改文档示例，`cargo test` 立刻变红。

`cargo test` 的输出有**三段**，读懂它们能省很多时间：

```text
     Running unittests src/lib.rs (target/debug/deps/...)   ← ① 单元测试（src/ 里的 mod tests）
test tests::add_works ... ok                    test result: ok. 2 passed; 0 failed; ...
     Running tests/chapter.rs (target/debug/deps/...)       ← ② 集成测试（tests/ 下每个文件）
test ex15_1_read_raw ... ok                     test result: ok. 3 passed; 0 failed; ...
   Doc-tests ch15_testing_unsafe                            ← ③ 文档测试（每个 ``` 代码块一个）
test src/lib.rs - read_raw (line 40) ... ok     test result: ok. 3 passed; 0 failed; ...
```

常用命令：

```bash
cargo test                          # 三样全跑
cargo test -p ch15_testing_unsafe   # 只跑本 crate
cargo test ex15_2                   # 只跑名字含 ex15_2 的测试
cargo test -- --nocapture           # 让 println! 正常打印（默认被吞掉）
cargo test -- --test-threads=1      # 串行跑，调试共享状态时有用
```

---

## 4. `#[should_panic]` 与断言家族

有些行为「应该 panic」才是正确行为，用属性声明：

```rust
#[test]
#[should_panic(expected = "索引越界")]
fn out_of_range_panics() {
    let i = 10;
    let v = vec![1, 2, 3];
    assert!(i < v.len(), "索引越界");
    let _ = v[i];
}
```

`expected = "..."` 要求 panic 消息是它的**子串**。不写的话任何 panic 都算通过，
容易把「因为别的原因崩了」误判成成功——**强烈建议写 `expected`**。

| 宏 | 用途 | 失败时打印 |
|---|---|---|
| `assert!(cond)` / `assert!(cond, "msg {}", x)` | 布尔条件 | 条件表达式 / 你的消息 |
| `assert_eq!(a, b)` / `assert_eq!(a, b, "msg")` | 相等（需 `PartialEq + Debug`） | 左右**两个值** |
| `assert_ne!(a, b)` | 不相等 | 两个值 |

> ⚠️ **坑**：`assert_eq!` 失败要用 `{:?}` 打印，所以类型必须实现 `Debug`（自定义结构体忘了
> `#[derive(Debug)]`，报错是「doesn't implement Debug」而不是断言失败）。
> 另外**浮点不要用 `assert_eq!`**，用容差：`fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }`。

---

## 5. `unsafe` 的五个超能力

`unsafe` 能做的事**恰好五件**，多一件都没有：

| # | 超能力 | 例子 | 做错的后果 |
|---|---|---|---|
| 1 | 解引用裸指针 | `unsafe { *ptr }` | 空指针/悬垂 → 段错误或读到垃圾 |
| 2 | 调用 `unsafe fn` / FFI 函数 | `unsafe { abs(x) }` | 参数约定被破坏 → UB |
| 3 | 访问或修改 `static mut` | `unsafe { COUNTER += 1 }` | 多线程下数据竞争 |
| 4 | 实现 `unsafe trait` | `unsafe impl Send for T {}` | 线程不安全类型被跨线程共享 |
| 5 | 访问 `union` 字段 | `unsafe { u.i }` | 读到错误的位模式（值可能未初始化） |

`union` 是给 FFI 和内存布局敏感场景的工具（对照 C 的 `union`），读字段必须 `unsafe`，
因为编译器无法知道你写进去的是哪个变体。

**`unsafe` 不改变的三件事**（最重要）：

```rust
unsafe {
    let s = String::from("x");
    let r = &s;
    let m = &mut s;   // ❌ 依然是 E0502：cannot borrow `s` as mutable...
    println!("{r} {m}");
}
```

- **借用检查照常生效**——`unsafe` 完全不管所有权和生命周期。
- 类型检查、trait 解析、模式匹配穷尽性也照常生效。
- 所以 `unsafe` 是「多了一组你可以做、但必须自己保证正确的事」，粒度是**块级别**的，
  而不是 C++ 那种「一关关一片」的 `-fno-strict-aliasing`。**务必配 `// SAFETY:` 注释写清前置条件。**

> ⚠️ **坑**：`unsafe fn` 在 edition 2021 里函数体**整体**就是 unsafe 上下文，
> 里面的 `unsafe { }` 会触发 `unused_unsafe` 告警；想保留这个显式块就加 `#[allow(unused_unsafe)]`。

---

## 6. 裸指针 `*const T` / `*mut T` vs 引用 `&T` / `&mut T`

| | `&T` / `&mut T` | `*const T` / `*mut T` |
|---|---|---|
| 可为空 | ❌ 永远有效 | ✅ 可以是 `std::ptr::null()` |
| 对齐 | 编译器保证 | 你自己保证 |
| 生命周期 | 有，借用检查器跟踪 | 无，编译器不管死活 |
| 别名规则 | `&mut` 独占，`&` 共享 | 无规则 |
| 跨 FFI | ❌（ABI 不保证） | ✅ 就是 C 的指针 |
| 解引用 | 不用 unsafe | **必须 unsafe** |
| 算术 | ❌ | ✅ `ptr.add(1)` |

```rust
let x: i32 = 42;
let r: &i32 = &x;              // 安全引用
let p: *const i32 = &x;        // &T → *const T：**安全**操作
let q = p as *mut i32;         // *const T → *mut T：也只是转换，不用 unsafe
// unsafe { *q = 1 };          // ❌ x 不可变，改它仍是 UB（写了 unsafe 也没用）
```

注意区分：**「造指针」是安全的，「解引用」才需要 `unsafe`**。
`&x as *const i32` 没问题，只有 `*p` 才需要你担保。

真实场景：FFI 边界、`Vec`/`Box` 这类容器的手写实现、自引用结构、mmap/硬件寄存器，
以及极少数先被 profiler 证明必要的热点。

---

## 7. `extern "C"` 与 ABI：调用 C 函数

**ABI**（Application Binary Interface）规定函数怎么传参、谁清栈、返回值放哪、名字怎么修饰。
Rust 默认 ABI（`extern "Rust"`）**没有稳定保证**，不能跨语言；`extern "C"` 就是通用的 C ABI。

调用 C 函数只需**声明**，不用写函数体：

```rust
extern "C" {
    fn abs(input: i32) -> i32;                              // <stdlib.h>
    fn strlen(s: *const std::os::raw::c_char) -> usize;     // <string.h>
}

pub fn abs_via_ffi(x: i32) -> i32 {
    unsafe { abs(x) }        // 声明是安全的，"调用"才需要 unsafe
}
```

- `std::os::raw::c_char` 就是 C 的 `char`。它在多数平台是 `i8`，但**别写死 `i8`**——
  有些平台 `char` 无符号，用 `c_char` 才可移植。`i32` 对应 `int`，`usize`/`isize` 对应
  `size_t`/`ptrdiff_t`；C 的 `long` 在 Linux 64 位是 64 位、Windows 是 32 位，跨平台要用 `std::os::raw::c_long`。
- 链接由 std 自动完成：libc 是 Rust 运行时的依赖，不需要 `#[link]`、不需要 `build.rs`。
- 参数/返回值必须是 **FFI-safe** 的：整数、浮点、裸指针、`#[repr(C)]` 结构体可以；
  `String`、`Vec`、`&str`、没有 `#[repr(C)]` 的 `enum` **不行**——它们的布局是 Rust 内部实现，随时会变。
- 跨 FFI 铁律：**panic 不能穿过 FFI 边界**（是 UB，新版本会 abort）。

| C | Rust |
|---|---|
| `#include <stdlib.h>` | 不用 include，直接 `extern "C" { ... }` 声明 |
| `int abs(int);` | `fn abs(input: i32) -> i32;` |
| `size_t strlen(const char *);` | `fn strlen(s: *const c_char) -> usize;` |
| `free(p)` | 通常交给 `Drop` |

---

## 8. `CString` / `CStr` 与 NUL 结尾

C 的字符串是「字节串 + 结尾 `\0`」，Rust 的 `&str` 是「指针 + 长度」，两者**不通用**：

| Rust 类型 | 含义 | NUL 结尾 | 谁拥有内存 |
|---|---|---|---|
| `String` / `&str` | UTF-8，带长度 | ❌ | Rust |
| `std::ffi::CString` | 拥有内存的 C 字符串 | ✅ 自动补 `\0` | Rust（`Drop` 时释放） |
| `std::ffi::CStr` | 借来的 C 字符串（`&CStr`） | ✅ 必须有 `\0` | 别人（常见：C 返回值） |

```rust
use std::ffi::CString;

let c = CString::new("hello").unwrap();          // 分配 "hello\0"
let p: *const std::os::raw::c_char = c.as_ptr();
let n = unsafe { strlen(p) };                    // 5，不含结尾的 \0
// c 离开作用域时自动 free —— 不用手动释放

// 反向：C 给你 const char*，看它长什么样
let back = unsafe { std::ffi::CStr::from_ptr(p) };
assert_eq!(back.to_str().unwrap(), "hello");
```

关键点：

- **`CString::new(s)` 返回 `Result`**，因为 `s` 中间不能有 `\0`：内部 NUL 会让 C 侧提前截断，
  语义必然出错，所以 Rust 直接给 `Err(NulError)`。正确处理是 `match` / `?`，而不是 `unwrap()`：

  ```rust
  pub fn c_strlen(s: &str) -> usize {
      match std::ffi::CString::new(s) {
          Ok(c) => unsafe { strlen(c.as_ptr()) },
          Err(_) => 0,      // 含内部 \0：拒绝，按约定返回 0
      }
  }
  ```

- `CString::new("")` 是**成功**的：构造出只有一个 `\0` 的缓冲区，`strlen` 返回 0。
  于是「空串」和「含 `\0`」在本章约定里都返回 0——刻意的：调用者拿到 0 就知道不能当正常 C 字符串用。
- **`strlen` 数的是字节**。UTF-8 下一个汉字 3 字节，所以 `strlen("中文")` 是 **6**；
  Rust 侧 `"中文".len()` 也是 6，而 `"中文".chars().count()` 才是 2。
- `CStr::from_ptr` 本身是 `unsafe fn`（假设指针非空、以 `\0` 结尾、内存有效）；
  拿到后 `to_str()` **仍可能失败**——C 字符串不保证是合法 UTF-8（那是 `to_string_lossy` 的活）。

---

## 9. 反向：把 Rust 函数导出给 C 调用

让 C 调 Rust 需要三件事：`extern "C"` + `#[no_mangle]`（edition 2021 写法；2024 起是 `#[unsafe(no_mangle)]`）
+ 只暴露 FFI-safe 类型。

```rust
/// 加法器：给 C 调用的入口。无指针参数，任何输入都安全。
#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

/// 统计 C 字符串长度。调用者必须保证 `s` 非空、以 NUL 结尾、指向有效内存。
#[no_mangle]
pub unsafe extern "C" fn rust_strlen(s: *const std::os::raw::c_char) -> usize {
    if s.is_null() { return 0; }
    let c = unsafe { std::ffi::CStr::from_ptr(s) };
    c.to_bytes().len()
}
```

- `#[no_mangle]` 阻止 Rust 给符号加哈希后缀（`_ZN...`），C 侧才能按 `rust_add` 这个名字找到它；
  忘了它，链接时就是 undefined symbol。
- 编译成库：`cargo build --release` → `target/release/libch15_testing_unsafe.so`
  （macOS `.dylib`、Windows `.dll`）。想更贴近 C 习惯可加 `[lib] crate-type = ["cdylib"]`。
- C 侧：`int rust_add(int a, int b);`，然后
  `gcc main.c -L target/release -lch15_testing_unsafe -o app`。
- **`build.rs` 与 `cc` crate 一句话**：当你要**编译并链接一段 C/C++ 源码**（而不只是声明已有符号）时，
  在 `build.rs` 里用 `cc` crate 调系统编译器，如 `cc::Build::new().file("src/foo.c").compile("foo");`
  ——那需要第三方 crate，本仓库练习一律只用 std。

---

## 10. 什么时候真的需要 `unsafe`

1. **FFI**（本章）：和 C/C++/系统调用/硬件打交道，不可回避的主战场。
2. **性能热点**：省边界检查、免初始化，**前提是先有 profiler 数据**；`get_unchecked` 省下的 1%
   通常不值得一个潜在 UB。
3. **实现安全抽象**：`Vec`、`Rc`、`Mutex`、`RefCell` 内部都是 `unsafe`，但生态里的 unsafe 应该
   **被封装起来**，对外只给安全 API（本章的 `abs_via_ffi` 就是这个模式）。
4. **平台/底层**：`mmap`、裸系统调用、内核模块、嵌入式寄存器。

反过来这些**不需要** `unsafe`（新手常误判）：改可变变量、`as` 数值转换、索引越界 panic、
`unwrap()`、并发（`Arc<Mutex<_>>`）、调用任何安全函数。

> ⚠️ **坑**：不要为了让编译器闭嘴而加 `unsafe`。看到 `unsafe` 的第一反应应该是
> 「SAFETY 注释写了什么前置条件？调用方满足了吗？」

---

## 11. 常见坑

| 坑 | 现象 | 正确做法 |
|---|---|---|
| 在 `unsafe fn` 里写 `unsafe {}` | `unused_unsafe` 告警 | 加 `#[allow(unused_unsafe)]`，或开 `unsafe_op_in_unsafe_fn` 并保留块 |
| 把 `unsafe` 当万能钥匙 | 借用错误依旧报 | 它只解锁那五件事 |
| `CString::new(s).unwrap()` | 输入含 `\0` 时 panic | `match` / `?`，返回错误或约定值 |
| 用 `strlen` 数「字符数」 | 中文长度算成 3 倍 | `strlen` 是字节数；字符数用 `chars().count()` |
| `abs(i32::MIN)` | UB（C 里 `-INT_MIN` 溢出） | 别传 `MIN`，或改用 `wrapping_abs()` / `i64` |
| FFI 传 `String`/`Vec` | 编译错误或 UB | 传 `*const c_char` / `*const u8` + 长度 |
| `&x` 直接当 `*const T` 用 | 能编译，但看不出「裸指针」的意图 | 显式 `let p: *const i32 = &x;` |
| 忘了 `#[no_mangle]` | C 链接报 undefined symbol | 导出函数加 `#[no_mangle] pub extern "C"` |
| 测试共享全局状态 | 随机失败（flaky） | 测试保持独立；全局用 `Mutex`/`OnceLock` |
| 文档示例没跟着代码改 | Doc-tests 段失败 | 把 doctest 当契约维护 |

---

## 12. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 15-1 | 解引用裸指针读出 `i32` | 裸指针 vs 引用、`unsafe fn`、函数体为什么还要写 `unsafe` 块 | `read_raw` |
| 15-2 | 包装 C 运行库的 `abs` | `extern "C"` 声明、调用点必须 `unsafe`、`abs(INT_MIN)` 的 UB | `abs_via_ffi` |
| 15-3 | 用 C 的 `strlen` 数字节 | `CString` / NUL 结尾 / 内部 `\0` 报错 / UTF-8 字节数 | `c_strlen` |

骨架在 `exercises/ch15_testing_unsafe/src/lib.rs`，函数体是 `unimplemented!()`；
`extern "C"` 声明已经写在文件顶部，直接用。

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch15_testing_unsafe
```

### 提示（不给答案）

- **15-1**：函数体一行。`unsafe fn` 的签名是对外承诺，但块内解引用仍需 `unsafe { }`。
- **15-2**：`unsafe { abs(x) }`。想一想：为什么这个包装能让调用者**不用**写 `unsafe`？
- **15-3**：`CString::new(s)` 返回 `Result`；`strlen` 只收 `*const c_char`，先转指针；
  含内部 `\0` 时按约定返回 `0`。

卡住了就跑 `python3 tools/ai_review.py ch15_testing_unsafe --hint 2`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 先声明 C 符号（不是第三方 crate，libc 由 std 自动链接）
extern "C" {
    fn abs(input: i32) -> i32;
    fn strlen(s: *const std::os::raw::c_char) -> usize;
}

// 15-1：显式 unsafe 块在 edition 2021 下算冗余（告警），但把危险点标得更清楚，
// 所以用 #[allow(unused_unsafe)] 保留它。
#[allow(unused_unsafe)]
pub unsafe fn read_raw(ptr: *const i32) -> i32 {
    unsafe { *ptr }
}

// 15-2：只有调用 C 函数那一行需要 unsafe。函数本身保持安全签名。
pub fn abs_via_ffi(x: i32) -> i32 {
    unsafe { abs(x) }
}

// 15-3：CString::new 负责补结尾 \0（并拒绝内部 \0）；
// strlen 数的是字节：UTF-8 下 "中文" 是 6 字节。
pub fn c_strlen(s: &str) -> usize {
    match std::ffi::CString::new(s) {
        Ok(c) => unsafe { strlen(c.as_ptr()) },
        Err(_) => 0,
    }
}
```

**为什么 `abs_via_ffi` 不写成 `unsafe fn`？** 它对所有 `i32` 都满足 C 侧的前置条件，
没有「调用者必须额外保证」的契约。把 `unsafe` 关在内部、对外给安全 API，
就是「unsafe 要小、要包住」的实践。（`i32::MIN` 的 UB 来自 C 标准对 `abs` 的定义，不是指针问题，
所以测试刻意避开它。）

**为什么 `c_strlen` 不直接 `.unwrap()`？** `unwrap` 会在含内部 `\0` 时 panic；
跨 FFI 的输入常来自外部数据，用 panic 处理「不可表示的输入」会拖垮进程。返回 0 更温和。

</details>

---

## 自测清单

- [ ] 我能说出单元测试（`#[cfg(test)] mod tests`）与集成测试（`tests/`）的三个区别
- [ ] 我知道 `cargo test` 三段输出分别是什么（unittests / integration / Doc-tests）
- [ ] 我能写出一个 `#[should_panic(expected = "...")]` 测试，并解释为什么要写 `expected`
- [ ] 我能不查文档列出 `unsafe` 的五个超能力
- [ ] 我能解释「`unsafe` 不等于关掉借用检查」，并说出它**不**改变哪些检查
- [ ] 我能说清 `&T` 与 `*const T` 在空值/对齐/生命周期/别名上的区别
- [ ] 我知道「造裸指针」不用 unsafe、「解引用」才要
- [ ] 我能写出 `extern "C"` 块声明 `abs` 与 `strlen`，并在调用点正确包 `unsafe`
- [ ] 我知道 `CString::new` 为什么返回 `Result`，以及内部 `\0` 该怎么处理
- [ ] 我能解释 `strlen("中文") == 6`，以及它和 `chars().count()` 的区别
- [ ] 我知道导出给 C 要加 `#[no_mangle] pub extern "C"`，以及为什么 panic 不能穿过 FFI
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch15_testing_unsafe` 全绿
