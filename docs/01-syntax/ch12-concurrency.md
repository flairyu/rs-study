# 第 12 章 · 并发

> **一句话定位**：把「多线程难在调试」变成「多线程难在写对类型」——`Send`/`Sync` 让数据竞争在编译期就出不了门。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 11 章（`Arc`、`RefCell`）、第 10 章（`'static`）
> **练习工程**：`exercises/ch12_concurrency`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

C 的 `pthread_create` 只负责起线程，参数怎么传、内存活多久、要不要加锁全靠约定；
Python 的 `threading` 写起来最省心，但 **GIL** 让「并行」名不副实——一旦用 C 扩展或
`multiprocessing` 绕开 GIL，竞态立刻回来。Rust 的做法叫 **fearless concurrency**：
把「能不能跨线程」变成 `Send` / `Sync` 两枚编译器自动推导的标记。

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 起线程 | `pthread_create` / `std::thread` | `threading.Thread` | `thread::spawn(闭包)` → `JoinHandle` |
| 捕获数据 | 手工 `void*` 传递 | 闭包共享，GIL 兜底 | `move` 转移所有权；不 `move` 常编译不过 |
| 共享可变数据 | `pthread_mutex_t` + 自觉 | 有 GIL，很多操作「看起来原子」 | `Arc<Mutex<T>>`，**不加锁就改不了** |
| 编译器检查 | 几乎没有 | 无 | `Send` / `Sync` 编译期拒绝不安全共享 |
| 数据竞争 | UB，事后用 TSan 抓 | 很少崩，逻辑竞态照旧 | 安全代码里**写不出来**，要竞争得 `unsafe` |
| 消息传递 | pipe / 自建队列 | `queue.Queue` | `std::sync::mpsc::channel()` |
| 并行迭代 | OpenMP `#pragma omp parallel for` | `multiprocessing`（进程级） | 标准库手写；`rayon` 一行 `par_iter()` |

根源一句话：**Rust 不允许「同一块内存同时被读又被写」，而数据竞争恰好是这件事的跨线程版**。
把这条规则延伸到线程边界，竞态就被类型系统消掉了——代价是你得先说服编译器「我确实加了锁」。

---

## 1. `thread::spawn` 与 `move` 闭包

```rust
use std::thread;

let v = vec![1, 2, 3];
let handle = thread::spawn(move || v.iter().sum::<i32>());
assert_eq!(handle.join().unwrap(), 6);   // 输出: 6
```

`spawn` 的约束是 `F: FnOnce() -> T + Send + 'static`：新线程可能活得比当前函数久，所以捕获的
东西不能是栈上的临时借用（`'static`），而且必须能安全搬过去（`Send`）。`move` 把捕获值的
所有权搬进闭包，是同时满足这两条的最常见手段。不写 `move` 时闭包默认借用，立刻报错：

```rust
let v = vec![1, 2, 3];
// let h = thread::spawn(|| v.iter().sum::<i32>());
// ❌ error[E0373]: closure may outlive the current function, but it borrows `v`
```

> ⚠️ **坑**：`move` 是「整体搬走」而不是「只搬用到的字段」。闭包里用到 `outer.field` 时整个
> `outer` 会被 move 进去，之后主线程就不能再用它。想只搬一部分，先 `let f = outer.field;`，
> 或者用第 7 节的 `scope` 传借用。

---

## 2. `join` 与 `JoinHandle`

`spawn` 立即返回 `JoinHandle<T>`，线程与主线程**并发**跑；`join()` 阻塞到它结束，返回
`Result<T, Box<dyn Any + Send>>`——子线程 panic 不会炸掉主线程，而是变成 `Err`：

```rust
let h = std::thread::spawn(|| 1 + 1);
assert_eq!(h.join().unwrap(), 2);

let h = std::thread::spawn(|| { panic!("boom"); });
assert!(h.join().is_err());      // 主线程照常活着
```

三条必须记住的语义：

1. **不 `join` 就等于 detach**：`JoinHandle` 被 drop 后线程照跑，你无法再等它，也拿不到它的 panic。
2. **主线程一结束，所有子线程立刻被杀**，没有「等所有非 daemon 线程结束」这回事。
3. `join()` 消费 `self`，同一个 `JoinHandle` 只能 join 一次。

---

## 3. 什么时候必须 `Arc`

`Rc<T>` 的引用计数不是原子的，所以它**不是** `Send`：

```rust
let shared = std::rc::Rc::new(vec![1, 2, 3]);
// std::thread::spawn(move || println!("{:?}", shared));
// ❌ error[E0277]: `Rc<Vec<i32>>` cannot be sent between threads safely
```

换成 `Arc`（Atomic Reference Counted）即可跨线程共享所有权：

```rust
use std::sync::Arc;

let data = Arc::new(vec![1, 2, 3]);
let d2 = Arc::clone(&data);              // 克隆的是"指针 + 计数"，不是数据
let h = std::thread::spawn(move || d2.len());
assert_eq!(h.join().unwrap(), 3);
assert_eq!(data.len(), 3);               // 主线程这份仍然可用
```

判断只需问两句：

- **需要多个所有者吗？** 每个线程各自持有一份克隆数据（或在 `scope` 里借用）时根本不需要
  `Arc`——它只解决「同一份数据被多方拥有」。
- **需要改吗？** `Arc<T>` 只给共享的 `&T`，要改就得配 `Mutex`（或 `RwLock`），于是有了
  `Arc<Mutex<T>>`：`Arc` 管「活多久」，`Mutex` 管「谁能改」，缺一不可。

---

## 4. `Mutex` 与锁中毒

```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0usize));
let mut handles = Vec::new();
for _ in 0..4 {
    let counter = Arc::clone(&counter);
    handles.push(std::thread::spawn(move || {
        for _ in 0..1000 {
            let mut guard = counter.lock().unwrap();
            *guard += 1;
        }   // guard 在这里 drop → 自动解锁（RAII）
    }));
}
for h in handles { h.join().unwrap(); }
assert_eq!(*counter.lock().unwrap(), 4000);
```

- `lock()` 返回 `LockResult<MutexGuard<T>>`；`MutexGuard` 实现 `Deref`/`DerefMut`，能直接当 `&mut T` 用。
- **不需要手动 unlock**：`guard` 出作用域即 drop 即解锁；反过来 `guard` 活多久锁就持有多久，
  这是死锁的头号来源。
- **锁中毒（poisoning）**：线程持锁时 panic，`Mutex` 会记住，之后 `lock()` 返回
  `Err(PoisonError)`；`.unwrap()` 会在调用方 panic，想继续用数据就 `into_inner()`：

```rust
match m.lock() {
    Ok(guard) => println!("正常: {}", *guard),
    Err(poisoned) => println!("中毒了: {}", *poisoned.into_inner()),
}
```

设计动机：C 里线程异常退出后锁的状态没人管；Rust 把「数据可能已被写坏」显式化，
逼你决定是恢复还是失败。

---

## 5. 死锁是怎么来的

死锁不是数据竞争，**编译器管不了**，只能靠设计与纪律。三种常见形态：

```rust
// ① 加锁顺序不一致：A 先 a 后 b，B 先 b 后 a → 各持一把互相等
// ② 自锁：std 的 Mutex 不可重入
let m = std::sync::Mutex::new(0);
let _g = m.lock().unwrap();
// let _g2 = m.lock().unwrap();   // ❌ 永久阻塞（不是 panic）
// ③ 持锁做慢操作：持锁去 join / recv / 调用还会取同一把锁的函数
```

防御：**全局约定加锁顺序**、**把临界区压到最短**（先算完再取锁）、需要多把锁时按同一顺序
获取、能在锁外等待就别在锁内等待。

> ⚠️ **坑**：`if let Ok(x) = m.lock() { ... }` 里 `MutexGuard` 临时值会活到整个 `if let` 块结束，
> 块内再取同一把锁就死锁。习惯做法：先把 `lock()` 的结果绑定到变量，再进分支。

---

## 6. `mpsc` channel 与消息传递

`mpsc` = multi-producer, single-consumer：`Sender` 可以 `clone`，`Receiver` 只有一个。

```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel();
let handle = thread::spawn(move || {
    for i in 1..=3 { tx.send(i * 2).unwrap(); }   // tx 被 move 进来
});                                               // 闭包结束 → tx 被 drop
let mut got: Vec<i32> = rx.iter().collect();      // 所有 Sender 没了 → 循环结束
handle.join().unwrap();
got.sort_unstable();
assert_eq!(got, vec![2, 4, 6]);
```

- **通道关闭 = 所有 `Sender` 都被 drop**。这就是「发完就结束」的正确姿势，不需要哨兵值；
  `for v in rx` 等价于 `while let Ok(v) = rx.recv()`。
- `send` 返回 `Err` 说明接收端没了；`recv` 返回 `Err` 说明所有发送端都没了。
- 需要背压用 `mpsc::sync_channel(n)`；`sync_channel(0)` 是「必须有人接才发得出去」的会合语义。
- 这就是那句老话：**不要通过共享内存来通信，而要通过通信来共享内存**。消息传递把
  「谁在何时改了什么」线性化成一条队列，比散落各处的锁好推理。

---

## 7. `thread::scope`：借用非 `'static` 数据

`spawn` 要求 `'static`，但很多时候数据明明活得比线程久，只是编译器不知道。
`std::thread::scope`（Rust 1.63+）保证 scope 结束前所有线程都已 join，于是闭包能直接借用：

```rust
use std::thread;

let nums = vec![1, 2, 3, 4];
let total = thread::scope(|s| {
    let left = s.spawn(|| nums[0] + nums[1]);      // 借用 nums，无需 move
    let right = s.spawn(|| nums[2] + nums[3]);
    left.join().unwrap() + right.join().unwrap()
});
assert_eq!(total, 10);
```

这解决了练习 12-1 的核心矛盾：既要**分片并行**（把 `Vec` 切成多段），又不想为每片 `clone`
一个新 `Vec`——直接借 `nums.chunks(chunk)` 的切片即可。某个线程 panic 时，`scope` 会等其余
线程结束后把 panic 重新抛出来。

---

## 8. `Send` / `Sync` 到底是什么

两个没有方法的 **auto trait**，由编译器看类型结构自动推导（只有写底层同步原语的人才需要
`unsafe impl`）：

| trait | 一句话定义 | 直觉 |
|---|---|---|
| `Send` | 值可以**移动**到另一个线程 | 「这东西能寄快递」 |
| `Sync` | `&T` 可以**共享**给多个线程 | 「它的只读引用能同时借给多人」 |

常见推导结果：

| 类型 | `Send` | `Sync` | 原因 |
|---|---|---|---|
| `i32` / `String` / `Vec<T: Send + Sync>` | ✅ | ✅ | 普通数据 |
| `Rc<T>` | ❌ | ❌ | 引用计数非原子，多线程会数坏 |
| `Arc<T>` | ✅（`T: Send + Sync`） | ✅ | 计数是原子的 |
| `RefCell<T>` | ✅ | ❌ | 运行时借用检查没有同步 |
| `Mutex<T>` | ✅（`T: Send`） | ✅ | 内部保证互斥 |
| `*const T` / `*mut T` | ❌ | ❌ | 裸指针，编译器无法担保 |

规律：**`Send`/`Sync` 会「穿透」容器**——`Vec<T>: Send` 当且仅当 `T: Send`。把 `Rc` 塞进
`Vec` 再送进线程照样报错，而且报错信息会一路指到那个 `Rc`。

---

## 9. 数据竞争在安全 Rust 里为什么编译不过

数据竞争需要三条同时成立：**(1) 两个以上线程访问同一块内存；(2) 至少一个是写；(3) 没有同步**。
而安全 Rust 的借用规则是：同一时刻要么有任意多个 `&T`，要么只有一个 `&mut T`。跨线程时再加
两条：值要 `Send`，共享的 `&T` 要求 `T: Sync`。合起来看，「多线程写同一块内存」在类型层面
就要求「同时存在 `&mut` 和别的引用」——正是借用检查器禁止的形状，于是编译失败：

```rust
let mut data = vec![1, 2, 3];
// let h = std::thread::spawn(|| data.push(4));   // ❌ 闭包可变借用 data
// data.push(5);                                  //    主线程同时还要用 data
```

想真的写出竞争只有两条路：`unsafe`（自己担保），或者用标准库的同步原语（`Mutex` / `RwLock` /
`Atomic*` / channel）把「同步」写进类型里。**不是"运行时检查更快"，而是"不可能发生的错误
不需要检查"**。

---

## 10. `rayon`：一行换掉手写分片

标准库没有并行迭代器；第三方 crate `rayon` 能让顺序代码几乎原样变并行：

```rust
// Cargo.toml 里加 rayon 之后（第 17 章讲；本章练习工程禁止加依赖）：
// let total: i32 = nums.par_iter().sum();
```

它内部就是 work-stealing 线程池 + `Send`/`Sync` 约束——**本章的类型规则同样是它的地基**。

---

## 11. 常见坑

- **忘了 `move`**：`error[E0373]`，闭包借用活得不够久。加 `move`，或改用 `thread::scope`。
- **`Rc` 跨线程**：`error[E0277] ... cannot be sent between threads safely`，换 `Arc`。
- **锁守卫活太久**：拿着 guard 去 `join`/`recv`/再取锁 → 死锁；临界区里只做必须做的事。
- **`chunks(0)` panic**：空输入时 `(0 + 3) / 4 == 0`，所以 `parallel_sum` 必须先早返回。
- **分片漏元素**：`chunk = len / 4` 会丢尾巴，要用向上取整 `(len + 3) / 4`，且遍历**所有** chunk。
- **用 `sleep` 当同步**：机器一忙就翻车；用 `join`（等线程）和通道关闭（等消息）对齐。
- **忘了 `join` 吞掉 panic**：子线程炸了你却看到测试全绿；要么 `join().unwrap()`，要么处理 `Err`。
- **主线程提前退出**：`main` 返回时子线程全被杀；想让后台线程跑完就得 `join`。

---

## 12. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 12-1 | 把 `Vec<i32>` 最多分 4 片，用 `thread::scope` 并行求和 | 借用非 `'static` 数据、分片不漏、空输入早返回 | `parallel_sum` |
| 12-2 | `threads` 个线程各把共享计数器加 `per_thread` 次 | `Arc<Mutex<T>>`、守卫作用域、`threads == 0` 不死等 | `counter_with_mutex` |
| 12-3 | 生产者把每个元素 `* 2` 发出去，主线程收完排序 | `mpsc::channel`、drop sender 关闭通道、收完排序 | `producer_consumer` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch12_concurrency      # 做题前先跑一次，看它怎么失败
python3 tools/ai_review.py ch12_concurrency --hint 2   # 卡住了看提示
```

### 提示（不给答案）

- **12-1**：先 `if nums.is_empty() { return 0; }`；`let chunk = (nums.len() + 3) / 4;`；在
  `thread::scope(|s| { ... })` 里对 `nums.chunks(chunk)` 逐个 `s.spawn(move || ...)`，收集
  `ScopedJoinHandle`，最后把所有 `join().unwrap()` 的部分和相加。
- **12-2**：`if threads == 0 { return 0; }`；`Arc::new(Mutex::new(0usize))`；循环里先
  `let counter = Arc::clone(&counter);` 再 `move` 进 `spawn`；内层写成
  `{ let mut guard = counter.lock().unwrap(); *guard += 1; }`——`guard` 必须在循环体内，
  否则整段循环只加一次。
- **12-3**：`let (tx, rx) = std::sync::mpsc::channel();`；生产者 `move` 走 `items` 和 `tx`，
  `for x in items { tx.send(x * 2).unwrap(); }` 后闭包结束即 drop `tx`；主线程
  `let mut out: Vec<i32> = rx.iter().collect();`，`join` 之后 `out.sort_unstable()`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 12-1：chunk 向上取整，保证"最多 4 片"且一个元素都不漏；scope 让闭包直接借用切片。
pub fn parallel_sum(nums: Vec<i32>) -> i32 {
    if nums.is_empty() {
        return 0;
    }
    let chunk = (nums.len() + 3) / 4;
    std::thread::scope(|scope| {
        let handles: Vec<_> = nums
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().copied().sum::<i32>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

// 12-2：threads == 0 先返回；循环内重新 Arc::clone 让每个线程各持一份所有权；
// guard 在循环体内 drop 即解锁。
pub fn counter_with_mutex(threads: usize, per_thread: usize) -> usize {
    if threads == 0 {
        return 0;
    }
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let mut handles = Vec::with_capacity(threads);
    for _ in 0..threads {
        let counter = std::sync::Arc::clone(&counter);
        handles.push(std::thread::spawn(move || {
            for _ in 0..per_thread {
                let mut guard = counter.lock().unwrap();
                *guard += 1;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    // ⚠️ 这里**不能**直接写 `*counter.lock().unwrap()` 当尾表达式：
    // 那会产生一个临时的 MutexGuard，它的 drop 发生在局部变量 `counter` 之后，
    // 借用检查器会报 E0597（"counter does not live long enough"）。
    // 先绑到一个局部变量，让 guard 在语句结束时就还回去。详见下面「常见坑」。
    let total = *counter.lock().unwrap();
    total
}

// 12-3：tx 随闭包结束被 drop，rx.iter() 才会结束；通道是 FIFO，但题目要求有序，故统一排序。
pub fn producer_consumer(items: Vec<i32>) -> Vec<i32> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        for x in items {
            tx.send(x * 2).unwrap();
        }
    });
    let mut out: Vec<i32> = rx.iter().collect();
    handle.join().unwrap();
    out.sort_unstable();
    out
}
```

**为什么 12-1 不写 `len.div_ceil(4)`？** 它更新更清楚，但需要较新的工具链；
`(len + 3) / 4` 在任何 stable 上都是同一结果。

**为什么 12-2 不把 `guard` 提到循环外？** 那样只有第一次 `+= 1` 生效，结果永远是 `threads`，
而且绕过了「反复取锁/放锁」这个考点本身。

</details>

---

## 自测清单

- [ ] 我能说出 `thread::spawn` 对闭包的两个要求（`Send + 'static`），以及 `move` 为什么常是必需
- [ ] 我知道不 `join` 会怎样（detach）、主线程结束时子线程会怎样（被杀）
- [ ] 我能解释什么时候必须用 `Arc`，什么时候根本不需要它
- [ ] 我知道 `Mutex` 的 guard 出作用域才解锁，以及「锁中毒」是什么、怎么恢复
- [ ] 我能说出至少两种死锁成因，并知道编译器为什么管不了死锁
- [ ] 我能解释「所有 `Sender` 被 drop，`for v in rx` 就结束」，且不用 `sleep` 做同步
- [ ] 我能用 `thread::scope` 借用非 `'static` 数据，并说清它和 `spawn` 的区别
- [ ] 我能背出 `Send` 与 `Sync` 的定义，并判断 `Rc`/`Arc`/`RefCell`/`Mutex` 各自是否具备
- [ ] 我能解释「数据竞争为什么在安全 Rust 里编译不过」
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch12_concurrency` 全绿
