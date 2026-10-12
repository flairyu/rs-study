# 第 13 章 · async / await 与 Future

> **一句话定位**：`async fn` 不"启动"任何东西——它只造出一台**惰性的状态机**，必须有人反复 poll 它才会前进。
> **预计时间**：110 分钟（读 50 + 写 60）
> **前置**：第 8 章（闭包与迭代器）、第 9 章（trait）、第 12 章（并发）
> **练习工程**：`exercises/ch13_async`（3 题，`src/lib.rs` 顶部附带一个教学玩具执行器）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

异步这件事，三种语言各走了一条完全不同的路：

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 异步的载体 | 回调 + 事件循环（libuv/epoll/IOCP） | `asyncio` coroutine + 事件循环 | `Future` trait + **外部**执行器 |
| 写完会不会自己跑 | 注册进循环后由循环调用 | `await` 后由 asyncio 循环调度 | **不会**，除非你（或 tokio）去 poll 它 |
| 谁来做"暂停/恢复" | 库 + 你手写的状态变量 | 解释器实现 coroutine | 编译器把 `async fn` **脱糖成状态机** |
| CPU 密集任务 | 自己开线程 | GIL 下无法真正并行 | 配合线程池，与 async 各管一段 |
| 谁来调度 | 事件循环（协作式） | 事件循环（协作式） | 协作式，但调度器由你选（tokio/async-std/自写） |
| 取消任务 | 手动 | `task.cancel()` | 直接 `drop` 掉 Future |

同样是"让出一次再返回 42"。C 里只能写回调（`uv_timer_start(&t, on_timeout, 1000, 0)`），
然后返回事件循环，"下一步该干嘛"的状态得自己存；Python 有 `async def` + `asyncio.run(answer())`，
由解释器内置的循环驱动；而 Rust 是这样：

```rust
async fn answer() -> i32 {
    YieldNow::new().await;   // 让出一次控制权（本章工具类型）
    42
}

fn main() {
    let n = block_on(answer());      // ✅ 被驱动后才真正跑
    assert_eq!(n, 42);
}
```

**关键差异**：Python 的 `asyncio.run` / `await` 内置在语言与解释器里；
Rust 只提供 `Future` + `.await` 两块积木，"事件循环"是**标准库之外**的库（tokio、async-std、smol）。
标准库没有自带执行器不是遗漏，而是设计：嵌入式、游戏引擎、内核各有自己的调度方式。

---

## 1. `async fn` 展开成什么

`async fn` 有两层身份：它**是**一个普通函数，调用它**不执行函数体**，而是立刻返回一个
实现了 `Future` 的匿名类型；返回的那个值是状态机，只有 `.poll()` 被调用时才推进一步。

```rust
// 你写的：
async fn add_async(a: i32, b: i32) -> i32 {
    let s = a + b;
    YieldNow::new().await;   // 一个"暂停点"
    s
}

// 编译器大致生成的结构（伪代码，帮助理解，不能直接编译）：
enum AddAsyncFuture {
    Start { a: i32, b: i32 },    // 参数被捕进状态机，和 move 闭包一样
    WaitingYield { s: i32 },     // 跨过 .await 还活着的局部变量存在这里
    Done,
}
```

三个推论：`.await` 是状态机的**保存点**，跨过它的局部变量被塞进枚举变体，所以 `async` 块的
"栈帧"其实是堆上的一个结构体；状态机类型匿名但 `Future::Output` 确定（`i32`），
没有 Python coroutine 那种动态派发开销；`.await` 只能出现在 `async` 上下文里。

## 2. `async fn` 本身不执行任何代码

本章最重要的一句话，值得单独见证一次：

```rust
static LOG: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

async fn noisy() -> i32 {
    LOG.lock().unwrap().push("body ran");   // 只有被 poll 才会跑
    7
}

fn main() {
    let fut = noisy();                      // 只造 Future
    println!("{:?}", LOG.lock().unwrap());  // 输出: []
    let v = block_on(fut);                  // 从这里开始才真正跑
    println!("{:?} {v}", LOG.lock().unwrap());   // 输出: ["body ran"] 7
}
```

事件循环、tokio、`futures::executor` 做的第一件事，都是把你的 Future 放进"待轮询"队列——
**没有 poll，就没有执行**。所以 `let _ = async_add(1, 2);` 编译能过（只警告 `unused_must_use`），
但函数体一次都没跑。

---

## 3. `.await` 做了什么

三句话说完全部语义：调用内部 Future 的 `.poll(cx)`；若返回 `Poll::Pending`，当前 Future
也返回 `Pending`（同时把 waker 存好，等就绪被唤醒）；若返回 `Poll::Ready(v)`，
整个 `.await` 表达式的值就是 `v`，继续往下走。

一次 `.await` ≈ 一次"可能暂停的函数调用"。对照 C：`.await` 就是编译器替你完成了
"保存局部状态 + 返回事件循环 + 注册唤醒回调"，这就是 async Rust 不需要回调地狱的原因。

| 写法 | 含义 | 会不会暂停 |
|---|---|---|
| `fut.await` | 驱动 `fut` 到就绪 | 可能（Pending 时） |
| `fut.poll(cx)` | 推进**一步** | 直接返回，不阻塞 |
| `async { ... }` | 造一个匿名 Future | 不执行 |
| `block_on(fut)` | 循环 poll 到就绪 | 阻塞当前线程 |

---

## 4. `Poll::Ready` 与 `Poll::Pending`

```rust
pub enum Poll<T> {
    Ready(T),   // 有结果了，Output 就是 T
    Pending,    // 还没好；**我保证**就绪时会调用 waker
}

pub trait Future {
    type Output;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>)
        -> std::task::Poll<Self::Output>;
}
```

两条约定，缺一不可：返回 `Pending` 时**必须**保证将来某刻会调用 `cx.waker().wake()`，
否则执行器不知道该再 poll 你一次；也不能"假装完成"来骗执行器重试——它不会无条件重试。
`YieldNow` 就是这个约定的最小样本：第一次 poll 记下 `true` 并唤醒自己，第二次 poll 返回 `Ready`。
忘了调 waker 的 Future 在 `block_on`（无脑重试型执行器）下看起来能跑，一换到 tokio 就直接卡死——
这不是 tokio 的 bug，是 Future 违反了契约。

---

## 5. 为什么 Future 必须被"驱动"

把 Future 想成**一台需要手摇的发电机**：

| 角色 | 干什么 | 缺失的后果 |
|---|---|---|
| Future | 状态机，产出 `Poll` | 无 |
| `.await` | 把子 Future 的状态机接进父状态机 | 手动写 poll 地狱 |
| Waker | 通知"我准备好了，再来 poll 我" | 调度器不知道该继续谁 |
| 执行器 / 事件循环 | 反复 poll；Pending 时去干别的 | 代码永远不跑 |

标准库只给了前三样，第四样留给你——因为"没事干的时候该干嘛"因场景而异：
Web 服务器去 epoll 等 socket，游戏引擎去跑下一帧，嵌入式去 `wfi` 休眠省电。
`block_on` 是**最朴素**的执行器：Pending 时只 `std::thread::yield_now()` 让出时间片再立刻重试。

---

## 6. 我们自己的 `block_on` 逐行拆解

`exercises/ch13_async/src/lib.rs` 顶部的工具，逐行读一遍：

```rust
pub fn block_on<F: Future>(fut: F) -> F::Output {
    // 1) 造一个"什么都不做"的 waker。RawWakerVTable 的四个函数指针分别是
    //    clone / wake / wake_by_ref / drop，这里全给空实现，data 指针用 null：
    //    waker 只为满足接口，不会被真的调度。
    const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
    const RAW: RawWaker = RawWaker::new(std::ptr::null(), &VTABLE);
    // SAFETY: 上面四个函数指针都是空操作，data 指针为 null 且永不被解引用。
    let waker = unsafe { Waker::from_raw(RAW) };

    // 2) Context 是 poll 的"环境"：执行器通过它把 waker 交给 Future。
    let mut cx = Context::from_waker(&waker);

    // 3) Future 必须被 Pin 住（见第 8 节）；Box::pin 把它放到堆上的固定地址。
    let mut fut: Pin<Box<F>> = Box::pin(fut);

    // 4) Ready 就返回；Pending 就让出时间片再来一遍。
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
```

三个设计细节：`F: Future` + 返回 `F::Output`，所以测试里能直接写
`assert_eq!(block_on(async_add(1, 2)), 3)`（不用 `Box<dyn Future>`）；
用 `Box::pin` 而不是 `Pin::new`，因为后者要求 `F: Unpin`，而 `async` 状态机默认不是；
空转等待在真实项目里是灾难（占满一个核），这里只是教学。

### 为什么 `block_on(async_sum(&[...]))` 这种借用参数也能过？

`async_sum(values: &[i32])` 的 Future 里存着一个 `&[i32]`，而 `.await` 会让状态机暂停，
"跨暂停点的引用"本该是最容易出事的地方。这里没问题：`&[1, 2, 3]` 是临时数组，
生命周期被延长到整条语句结束；`block_on` 在**同一个栈帧**里一路 poll 到 `Ready` 才返回，
Future 和它借用的数据没机会分开，所以借用检查器放行。

反过来，把 Future `spawn` 到别的线程（tokio 的常态）时借用活不过那一跳——这就是
`tokio::spawn` 要求 `'static` 的原因，实践中你会写 `Arc<Vec<i32>>` 或 `.to_vec()`。
**借用在异步里能用多久，取决于 Future 被驱动的方式。**

---

## 7. Waker 的作用与 noop waker 的局限

Waker 是"回调的现代写法"：就绪时 Future 调用它，执行器据此把任务重新排进队列，
于是 `.await` 暂停期间**不用占着线程**。

| 执行器 | 怎么知道该重 poll 谁 | CPU 占用 |
|---|---|---|
| `block_on`（本章） | 不知道，直接空转重试 | 一直跑，浪费 |
| tokio（多线程） | waker 唤醒任务 → 工作队列 → 线程池取 | 空闲时接近 0 |
| 单线程 LocalSet | waker 塞进本地队列，主循环消费 | 空闲时接近 0 |

noop waker 的局限正是"空转"：它把唤醒信号丢了，`block_on` 只能靠无条件重试兜底。

---

## 8. `Pin` 的直觉解释

结论先记住：**`Pin<&mut T>` 是"我保证不把这个值挪走"的承诺**，服务于自引用结构。
`async` 状态机天然可能自引用：`let x = ...; something.await; use_ref(&x)` 之后，
状态机里可能同时有 `x` 和指向 `x` 的引用；一旦整体被 `memcpy` 到别处，引用就悬垂了——
而"安全代码不产生悬垂引用"是 Rust 的前提。`Pin` 在类型层面堵住这条路：被 `Pin` 住后
你无法拿到 `&mut T` 整体挪动它（除非 `T: Unpin`，即"移动无害"）。

日常几乎不用碰 `Pin`：`Box::pin(fut)` / `std::pin::pin!(fut)` 是你要记住的两个工具；
`tokio::spawn` 内部已经帮你 Pin 好了；只有手写 `impl Future`（像 `YieldNow`）时才要写
`fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)`。`YieldNow` 里 `self.0 = true` 能编译，
是因为 `bool: Unpin`；换个 `!Unpin` 字段就得用 `unsafe { self.get_unchecked_mut() }`，
并自己承担"不挪动它"的责任。

---

## 9. async 与线程的区别：并发 vs 并行、协作式调度

| | 线程（第 12 章） | async（本章） |
|---|---|---|
| 并行 | 可以，多核同时跑 | 单线程下**不并行**，只并发 |
| 切换时机 | OS 抢占式，你无法预测 | 只在 `.await` 处让出，**协作式** |
| 每个任务成本 | 栈 MB 级 + 系统调用 | 状态机几十~几百字节 + 一次堆分配 |
| 阻塞调用 | 只阻塞自己 | 会卡死整个执行器线程 |
| 数据共享 | `Arc<Mutex<T>>` + `Send` | 单线程内可借用，跨任务一般也是 `Arc` |
| 适合 | CPU 密集、要真并行 | 海量 I/O 等待、连接数上万 |

一句话选择法：**CPU 忙 → 线程（或 rayon）；等 I/O → async。**
"异步"不是"更快"，而是"用一个线程扛住更多等待中的任务"。

> ⚠️ **坑（最常见的线上事故）**：在 async 任务里调用阻塞函数（`std::thread::sleep`、
> 同步文件 IO、长时间持锁），会堵住该执行器线程上的**所有**任务。
> 该等就用 `tokio::time::sleep`，该扔线程池就用 `tokio::task::spawn_blocking`。

---

## 10. tokio 的角色与 `#[tokio::main]`

标准库给了 `Future` 与 `.await`，但**没给执行器**。tokio 就是执行器 + 异步 I/O 运行时
（第 17 章细讲）：

```rust
// 需要依赖：tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
#[tokio::main]                       // 宏把 main 改写成运行时里的 block_on
async fn main() {
    let handle = tokio::spawn(async { 1 + 1 });   // 要求 Future: 'static + Send
    let n = handle.await.unwrap();                // JoinError → unwrap
    println!("{n}");                              // 输出: 2
}
```

`#[tokio::main]` 大致展开成：

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build()
        .unwrap().block_on(async { /* 你的 main 体 */ })
}
```

所以 `#[tokio::main]` 并**不是**语言特性，只是宏 + 运行时 crate；
本章的 `block_on` 和它是同一角色的简化版（少了 I/O、定时器、多线程调度）。

---

## 11. 什么场景该用 async

**适合**：高并发网络服务（axum/tonic，连接数上万）、大量并发下游请求（爬虫、网关）、
需要超时/取消/`select!` 的组合逻辑、GUI 与游戏里的非阻塞任务。

**不适合**：纯 CPU 计算（用线程 / rayon）、简单 CLI 脚本（同步代码更短）、
只有几个并发任务（线程更省心）、硬实时（执行器调度不可控）。
判据是"**任务大部分时间在等，而不是在算**"。

## 12. 常见坑

- `async fn` 不被 `block_on`/`.await` 驱动就永远不跑；`let _ = fut;` 只是静默丢弃。
- 返回 `Pending` 前忘记 `wake`，在真实执行器下永久挂起。
- 在 async 上下文里调用阻塞 API，拖垮整个执行器线程。
- `tokio::spawn` 要求 `'static + Send`：借用局部变量会编译失败，改用 `Arc` 或 move。
- 把 `MutexGuard` 跨 `.await` 持有：`std::sync::MutexGuard` 不是 `Send`，编译器会直接报错
  （这是好事，别硬绕）。
- 以为 `.await` 会自动并行：顺序写 `a.await; b.await;` 是串行，要并发得用
  `tokio::join!` / `futures::join!`。

---

## 13. 本章练习

三题都只依赖 std，用 `src/lib.rs` 顶部现成的 `block_on` 驱动。
**不要修改 `block_on` 与 `YieldNow`**，它们是工具而不是习题。

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 13-1 | 返回 `a + b` | `async fn` 是"造 Future"；用 `block_on` 取 `Output` | `async_add` |
| 13-2 | 切片求和（空切片返回 0） | 借用参数跨 `.await`；循环里 `.await` 子 Future | `async_sum` |
| 13-3 | 返回 `[n, n-1, ..., 1]`（`n == 0` 返回空） | 每轮先 `YieldNow::new().await` 再 push，观察 Pending | `async_count_down` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch13_async      # 做题前先跑一次，看它怎么失败
python3 tools/ai_review.py ch13_async --hint 2
```

### 提示（不给答案）

- **13-1**：函数体就是一个表达式 `a + b`（`async` 块最后一行即返回值）。
- **13-2**：`let mut acc = 0;` + `for &v in values { acc = async_add(acc, v).await; }`，最后 `acc`。
  `values` 是 `&[i32]`，`for &v in values` 拿到的 `v` 是 `i32`（`i32: Copy`）。
- **13-3**：`while n > 0 { YieldNow::new().await; out.push(n); n -= 1; }` 然后返回 `out`。
  `n == 0` 时循环体一次都不进，自然返回空 `Vec`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 13-1：async fn 的函数体就是 Future 的"执行内容"，最后一行是 Output。
// 不用写 return，也不用 unwrap —— block_on 直接把 i32 还给你。
pub async fn async_add(a: i32, b: i32) -> i32 {
    a + b
}

// 13-2：每轮 .await 一次 async_add，累加器跨过 .await 被存进状态机。
// 空切片时 for 体不执行，acc 保持 0 —— 边界自然成立。
pub async fn async_sum(values: &[i32]) -> i32 {
    let mut acc = 0;
    for &v in values {
        acc = async_add(acc, v).await;
    }
    acc
}

// 13-3：先 await 再 push，让"让出一次"发生在记录当前值之前；
// while n > 0 让 n == 0 直接返回空 Vec，不需要特判。
pub async fn async_count_down(n: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut n = n;
    while n > 0 {
        YieldNow::new().await;
        out.push(n);
        n -= 1;
    }
    out
}
```

**为什么 13-3 要在 push 之前 await？** 这样每轮都会亲手触发一次 `Poll::Pending`，
`block_on` 的 Pending 分支才会被执行到；顺序反了结果一样，但你就少了一次观察机会。
另外注意 Rust 的 `.await` 是**后缀运算符**，不是 Python 那种关键字前缀。

</details>

---

## 自测清单

- [ ] 我能解释 `let fut = my_async_fn();` 之后为什么日志里什么都没有
- [ ] 我能说出 `.await` 在 `Poll::Pending` 时对父 Future 做了什么
- [ ] 我知道 `Poll::Pending` 与 waker 之间的契约，以及违反契约的后果
- [ ] 我能逐行讲清 `block_on` 里 `Box::pin`、`Context`、`loop` 各是干什么的
- [ ] 我知道 `Pin` 在保护什么，以及为什么用 `Box::pin` 而不是 `Pin::new`
- [ ] 我能说出 async 与线程在"并发 vs 并行""抢占 vs 协作"上的区别
- [ ] 我知道 `#[tokio::main]` 展开后是什么，以及标准库为什么不带执行器
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch13_async` 全绿
