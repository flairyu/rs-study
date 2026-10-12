# 05_rayon · 数据并行（rayon）

> 目录名带序号只为排序。**cargo 不允许包名以数字开头**，所以 `labs/` 里所有成员的包名
> 统一是 `lXX_` 前缀：目录 `05_rayon`，包名 `l05_rayon`。

| 项 | 值 |
|---|---|
| 包名 | `l05_rayon` |
| 覆盖的 crate | `rayon` 1.x、`itertools` 0.14.x |
| 运行 | `cargo run -p l05_rayon`（第一次要联网拉依赖） |
| 前置 | 标准库的迭代器（`iter` / `map` / `fold` / `sum`） |

## 它解决什么问题

「CPU 算不过来」时，最省事的加速方式：把 `iter()` 改成 `par_iter()`，
串行代码几乎原样变成多线程代码，不用手写 `thread::spawn`、更不用自己切分数据。

前提是：每一项的计算**互不依赖**（纯函数、不共享可变状态），
且单项计算足够重、数据量足够大 —— 否则线程调度和结果合并的开销会吃掉全部收益。

## 程序演示了什么

1. **串行 vs 并行计时**：同一份 Collatz 计算，`iter()` 与 `par_iter()` 各跑一遍，
   用 `std::time::Instant` 打印耗时与加速比（先预热线程池，否则对比不公平）。
2. **并行组合子**：`into_par_iter().sum()`、`map().reduce(identity, op)`、
   `fold().reduce()`（每线程局部累加再合并）；并说明浮点并行求和为什么末位会飘。
3. **itertools 三个顺序工具**：`chunk_by`（相邻分组）、`zip_longest`（补长短不一的序列）、
   `join`（拼字符串）。它们和 rayon 无关，但常一起出现。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
cargo run -p l05_rayon
```

想换并行度（默认 = 逻辑核数）可以改环境变量：`RAYON_NUM_THREADS=2 cargo run -p l05_rayon`。

## 预期输出（节选，耗时随机器变化；本机 16 核整程序约 0.45s）

```text
== 实验 05 · rayon 数据并行 ==

逻辑 CPU 数: 16，rayon 全局线程池大小: 16
数据量: 1..=600000，每项都算一次 Collatz 步数

── ① 串行 iter() vs 并行 par_iter() ──
  串行 sum = 75668736，耗时 223.060852ms
  并行 sum = 75668736，耗时 37.452496ms
  加速比 ≈ 6.0x（核越多越明显，但永远达不到 100%）
...
── ② par_iter() 上的 map / sum / reduce / fold ──
  1..=1_000_000 求和 = 500000500000（into_par_iter().sum()）
  Collatz 步数最大值 = 469（reduce(identity, 结合函数)）
  fold+reduce 统计奇偶: 偶数 300000 个，奇数 300000 个
  浮点并行求和 = 13.881901
...
── ③ itertools：chunk_by / zip_longest / join ──
  chunk_by 把**相邻**相同元素分组: [1 × 3] [2 × 2] [3 × 4]
  zip_longest 补齐长度不等的两个序列: ada=90, bob=85, cyd=缺考
  join 把迭代器拼成字符串: rayon + itertools + std
```

> `N = 600_000` 是调过的：太小看不出并行优势，太大就会超过 1 秒。
> 换到核少的机器上耗时会整体变长，但「并行更快」的结论不变。

## 什么时候别用它

- **IO 密集**（网络、磁盘、数据库）：等 IO 的线程照样占着 rayon 的池子，
  该用 `tokio` 之类的异步运行时，而不是加线程。
- **单项计算太轻或数据太少**：几百个元素、每项几纳秒，`par_iter()` 会更慢。
- **需要严格顺序 / 可复现的浮点结果**：并行合并顺序不确定；定点数或串行才能复现。
- **闭包里要改共享状态**：`par_iter` 要求闭包是 `Sync`，真需要共享就得加锁，
  而锁会把并行变回串行 —— 通常应该改成 `map` 出局部结果再用 `reduce` 合并。
- **不想让第三方库霸占线程**：rayon 默认用全局线程池，库代码里调用它会「偷」调用方的核；
  长生命周期的服务用 `ThreadPoolBuilder` 建自己的池更可控。

## 常见坑

- 忘了 `use rayon::prelude::*;`，`par_iter()` 直接找不到。
- 计时没预热：第一次并行调用包含建线程的开销，看起来「并行没变快」。
- `chunk_by()` 返回的是惰性结构体，要 `for (k, group) in &chunks`（迭代**引用**），
  直接 `for ... in chunks` 会报「`ChunkBy` is not an iterator」。
- 以为 `chunk_by` 会做全局分组：它只合并**相邻**相同键，全局分组要先排序。
- `par_iter()` 在 `Vec`、切片、`Range` 上都有，但自定义容器要先实现 `IntoParallelIterator`。
