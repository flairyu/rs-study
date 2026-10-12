//! # 第 12 章 · 并发
//!
//! 本章解决一个问题：**怎么让多个线程同时干活，而编译器保证你踩不到数据竞争**。
//! Rust 的做法不是给运行时加锁，而是把 `Send` / `Sync` 变成编译期检查：
//! 不该跨线程的类型（`Rc`、`RefCell` 的引用）一进 `thread::spawn` 就报错。
//!
//! 做法：把每个 `unimplemented!()` 换成你自己的实现（**不要改函数签名**），然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch12_concurrency
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`parallel_sum`] | `thread::scope` 借用非 `'static` 数据、切片分片不漏元素 |
//! | [`counter_with_mutex`] | `Arc<Mutex<T>>`、锁的作用域、`threads == 0` 的早返回 |
//! | [`producer_consumer`] | `mpsc::channel`、drop sender 关闭通道、收完再排序 |
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 练习 12-1：把切片分成至多 4 片，用多个线程并行求和。
///
/// 语义：返回所有元素的和；**空输入返回 `0`**。
///
/// 约束：
/// - 用 `std::thread::scope` 起线程，闭包直接借用 `nums` 的切片——
///   这正是 `scope` 存在的意义：它保证所有线程在 `scope` 结束前 join，
///   所以借用不必是 `'static`。
/// - 分片大小建议写 `let chunk = (nums.len() + 3) / 4;`（等价于整除向上取整，
///   但**不要**用 `div_ceil`，它在新版本才稳定）。`nums.chunks(chunk)` 会保证
///   每个元素恰好属于一片——元素个数不是 4 的倍数（1/2/3/5/7/10…）时最容易漏元素。
/// - 分片顺序无关紧要：整数加法可交换，最终把各片部分和加起来即可。
///
/// 边界：空 `Vec` 直接返回 `0`（否则 `chunks(0)` 会 panic）；
/// 单元素、元素个数不是 4 的倍数都必须一个不漏。
///
/// # 示例
/// ```
/// # use ch12_concurrency::parallel_sum;
/// assert_eq!(parallel_sum(vec![]), 0);
/// assert_eq!(parallel_sum(vec![1, 2, 3, 4, 5]), 15);
/// ```
pub fn parallel_sum(nums: Vec<i32>) -> i32 {
    // 空输入必须早返回：否则 chunk == 0，chunks(0) 会 panic。
    if nums.is_empty() {
        return 0;
    }
    // 向上取整，保证「至多 4 片」且每个元素都落在某一刀里。
    let chunk = (nums.len() + 3) / 4;
    std::thread::scope(|scope| {
        let handles: Vec<_> = nums
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().copied().sum::<i32>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

/// 练习 12-2：`threads` 个线程共享一个计数器，每个线程把它加 `per_thread` 次。
///
/// 语义：返回最终计数值，预期恰好是 `threads * per_thread`。
///
/// 约束：
/// - 共享状态写成 `std::sync::Arc<std::sync::Mutex<usize>>`；
///   `Rc` 不是 `Send`，跨线程会编译失败，所以共享所有权必须用 `Arc`。
/// - 每个线程拿到 `Arc::clone` 之后，循环 `per_thread` 次：
///   `let mut guard = counter.lock().unwrap(); *guard += 1;`
///   注意锁在 `guard` 离开作用域时自动释放（RAII），不需要手动 unlock。
/// - `threads == 0` 时**必须直接返回 0**，不能去 `join` 一个不存在的线程，也不能死等。
/// - `per_thread == 0` 时线程照常起，但谁都不加，结果仍是 0。
///
/// # 示例
/// ```
/// # use ch12_concurrency::counter_with_mutex;
/// assert_eq!(counter_with_mutex(0, 100), 0);
/// assert_eq!(counter_with_mutex(4, 100), 400);
/// ```
pub fn counter_with_mutex(threads: usize, per_thread: usize) -> usize {
    // threads == 0 时没有线程可 join，直接返回 0。
    if threads == 0 {
        return 0;
    }
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let mut handles = Vec::with_capacity(threads);
    for _ in 0..threads {
        let counter = std::sync::Arc::clone(&counter);
        handles.push(std::thread::spawn(move || {
            for _ in 0..per_thread {
                // guard 每次循环结束（离开作用域）即释放锁，让别的线程有机会拿锁。
                let mut guard = counter.lock().unwrap();
                *guard += 1;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    // 先把值取出来再返回：直接写 `*counter.lock().unwrap()` 作为尾表达式时，
    // 临时 MutexGuard 会比局部变量 `counter` 后 drop，borrow checker 会报 E0597。
    let total = *counter.lock().unwrap();
    total
}

/// 练习 12-3：生产者线程把每个元素乘 2 发出去，主线程收完排序返回。
///
/// 语义：返回升序排列的 `items.iter().map(|x| x * 2)`。
///
/// 约束：
/// - 用 `std::sync::mpsc::channel()` 建通道，`Sender` 移动进生产者线程，
///   生产者在里面 `send(x * 2)`。
/// - 生产者线程一结束，`Sender` 就被 drop；所有 `Sender` 都没了之后，
///   主线程的 `for v in rx`（即 `rx.iter()`）自然结束——**不要用 sleep 判断收完没有**。
/// - 主线程收集完再 `sort_unstable()`：通道是 FIFO，但题目要求返回有序结果。
/// - 别忘了 `join` 生产者线程，否则错过的 panic 会被吞掉。
///
/// 边界：空 `items` 返回空 `Vec`；重复元素要原样保留；负数一样要 `* 2`。
///
/// # 示例
/// ```
/// # use ch12_concurrency::producer_consumer;
/// assert_eq!(producer_consumer(vec![]), Vec::<i32>::new());
/// assert_eq!(producer_consumer(vec![3, 1, 2]), vec![2, 4, 6]);
/// ```
pub fn producer_consumer(items: Vec<i32>) -> Vec<i32> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        for x in items {
            // tx 随闭包结束被 drop；所有 Sender 都没了，主线程的 for v in rx 才会结束。
            tx.send(x * 2).unwrap();
        }
    });
    let mut out: Vec<i32> = Vec::new();
    for v in rx {
        out.push(v);
    }
    handle.join().unwrap();
    out.sort_unstable();
    out
}
