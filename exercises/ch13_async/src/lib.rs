//! # 第 13 章 · async / await 与 Future
//!
//! 本章 3 个练习只用到标准库。和前 12 章不同的是：
//! **`async fn` 调用后不会立刻执行**，你必须用一个执行器去 poll 它。
//! 本文件顶部已经给了你一个教学用的极简执行器 [`block_on`]（外加一个
//! [`YieldNow`]），它们不是习题，而是你的实验台——照着文档读懂它们，
//! 再去做下面三道题。
//!
//! 做法：把每个函数体里的 `unimplemented!()` 换成你自己的实现，然后运行：
//!
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch13_async
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`async_add`] | `async fn` 是函数、返回值是 Future；用 [`block_on`] 取结果 |
//! | [`async_sum`] | 切片借用跨过 `.await`、`async fn` 之间互相 `.await` |
//! | [`async_count_down`] | `YieldNow` 制造 `Poll::Pending`、`.await` 暂停后恢复 |
//!
//! 提示：先读 `docs/01-syntax/ch13-async.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// 教学用极简执行器：用一个"什么都不做"的 waker 反复 poll 直到就绪。
/// 它不做任务调度、不跨线程、不省 CPU——只用来说明 `.await` 背后到底发生了什么。
///
/// noop waker 的构造方式取自 `futures-util::task::noop_waker`（同一个模式）。
pub fn block_on<F: Future>(fut: F) -> F::Output {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);
    // SAFETY: 上面四个函数指针都是空操作，data 指针为 null 且永不被解引用。
    let waker = unsafe { Waker::from_raw(RAW) };
    let mut cx = Context::from_waker(&waker);
    let mut fut: Pin<Box<F>> = Box::pin(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 一个"第一次 poll 返回 Pending，第二次就绪"的 Future，用来看清 `Pending` 的存在。
pub struct YieldNow(bool);

impl YieldNow {
    pub fn new() -> Self {
        Self(false)
    }
}

impl Default for YieldNow {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// 练习 13-1：异步加法。
///
/// 规格：返回 `a + b`。就这么多——本题的目的是让你看清
/// **`async fn` 调用后拿到的不是 `i32`，而是一个还没跑过的 Future**，
/// 必须用 [`block_on`] 驱动它才会真正执行。
///
/// # 示例
/// ```
/// # use ch13_async::{async_add, block_on};
/// assert_eq!(block_on(async_add(1, 2)), 3);
/// ```
pub async fn async_add(a: i32, b: i32) -> i32 {
    unimplemented!("练习 13-1：返回 a + b")
}

/// 练习 13-2：异步求和。
///
/// 规格：把切片里的所有数加起来；空切片返回 `0`。
/// 要求用 `for` 循环遍历 `values`，每一轮调用 `async_add(...)` 后 `.await`，
/// 把结果累加到一个 `i32` 累加器里。
///
/// 注意 `values: &[i32]` 是**借用**参数，而 `.await` 会让函数在中间暂停。
/// 在这里它能工作，是因为 [`block_on`] 在同一个栈帧里把它一次轮询到就绪，
/// 引用没有机会悬垂——但这也正是异步代码里生命周期容易变复杂的原因。
///
/// # 示例
/// ```
/// # use ch13_async::{async_sum, block_on};
/// assert_eq!(block_on(async_sum(&[1, 2, 3, 4])), 10);
/// assert_eq!(block_on(async_sum(&[])), 0);
/// ```
pub async fn async_sum(values: &[i32]) -> i32 {
    unimplemented!("练习 13-2：用 for + .await 求和")
}

/// 练习 13-3：异步倒计时。
///
/// 规格：返回 `[n, n-1, ..., 1]`；`n == 0` 时返回空 `Vec`。
/// 每一轮都必须**先** `YieldNow::new().await`，**再**把当前值 push 进结果，
/// 这样你就能观察到"每次 `.await` 都让出一次 `Poll::Pending`"。
///
/// # 示例
/// ```
/// # use ch13_async::{async_count_down, block_on};
/// assert_eq!(block_on(async_count_down(3)), vec![3, 2, 1]);
/// assert_eq!(block_on(async_count_down(0)), Vec::<u32>::new());
/// ```
pub async fn async_count_down(n: u32) -> Vec<u32> {
    unimplemented!("练习 13-3：从 n 倒数到 1")
}
