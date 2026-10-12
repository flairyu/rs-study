//! 实验 05 · rayon：把 `iter()` 换成 `par_iter()` 就并行
//!
//! 三段演示：
//! 1. 同一份「重计算」跑串行与并行，用 `Instant` 对比耗时；
//! 2. `par_iter()` 上的 `map` / `reduce` / `sum` / `fold`；
//! 3. itertools 的顺序迭代器工具（`chunk_by` / `zip_longest` / `join`）。
//!
//! 运行：`cargo run -p l05_rayon`

use std::time::Instant;

use itertools::{EitherOrBoth, Itertools};
use rayon::prelude::*;

/// 参与计算的数字个数。挑得够大，才能看出并行的收益（整体仍在一秒内）。
const N: u64 = 600_000;

/// 故意的「重计算」：Collatz（3n+1）序列要多少步才到 1。
///
/// 选它是因为：每一步只依赖上一步（纯函数、无共享状态），
/// 正是 rayon 最擅长的场景；而且它足够慢，能把计算时间放到毫秒级。
fn collatz_steps(mut n: u64) -> u64 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

fn main() {
    println!("== 实验 05 · rayon 数据并行 ==\n");
    println!(
        "逻辑 CPU 数: {}，rayon 全局线程池大小: {}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        rayon::current_num_threads()
    );
    println!("数据量: 1..={N}，每项都算一次 Collatz 步数\n");

    let numbers: Vec<u64> = (1..=N).collect();

    // 预热：线程池第一次启动（创建线程）本身要花时间，
    // 不预热的话并行那一轮的计时会把这笔开销算进去，对比不公平。
    let _ = numbers.par_iter().map(|&n| collatz_steps(n)).sum::<u64>();

    // ---------- ① 串行 vs 并行 ----------
    println!("── ① 串行 iter() vs 并行 par_iter() ──");

    let start = Instant::now();
    let serial_sum: u64 = numbers.iter().map(|&n| collatz_steps(n)).sum();
    let serial_time = start.elapsed();

    let start = Instant::now();
    let parallel_sum: u64 = numbers.par_iter().map(|&n| collatz_steps(n)).sum();
    let parallel_time = start.elapsed();

    // 整数求和可结合，并行结果必须和串行一致 —— 这正是「敢并行」的前提。
    assert_eq!(serial_sum, parallel_sum, "并行与串行必须得到同一个整数结果");

    println!("  串行 sum = {serial_sum}，耗时 {serial_time:?}");
    println!("  并行 sum = {parallel_sum}，耗时 {parallel_time:?}");
    if parallel_time.as_secs_f64() > 0.0 {
        println!(
            "  加速比 ≈ {:.1}x（核越多越明显，但永远达不到 100%）",
            serial_time.as_secs_f64() / parallel_time.as_secs_f64()
        );
    }
    println!("  ⚠ 并行不是免费的：切分任务、合并结果都有开销，");
    println!("    数据量小或每项太轻时，par_iter() 可能比 iter() 还慢。\n");

    // ---------- ② par_iter 上的常见组合子 ----------
    println!("── ② par_iter() 上的 map / sum / reduce / fold ──");

    let sum: u64 = (1..=1_000_000u64).into_par_iter().sum();
    println!("  1..=1_000_000 求和 = {sum}（into_par_iter().sum()）");

    let max_steps = numbers
        .par_iter()
        .map(|&n| collatz_steps(n))
        .reduce(|| 0, u64::max);
    println!("  Collatz 步数最大值 = {max_steps}（reduce(identity, 结合函数)）");

    // fold + reduce 是「两段式」：fold 在每个线程内部攒局部结果，
    // reduce 再把各线程的局部结果合起来 —— 避免为每一项都做一次全局合并。
    let (evens, odds) = numbers
        .par_iter()
        .fold(
            || (0u64, 0u64),
            |mut acc, &n| {
                if n % 2 == 0 {
                    acc.0 += 1;
                } else {
                    acc.1 += 1;
                }
                acc
            },
        )
        .reduce(|| (0u64, 0u64), |a, b| (a.0 + b.0, a.1 + b.1));
    println!("  fold+reduce 统计奇偶: 偶数 {evens} 个，奇数 {odds} 个");

    let sum_f64: f64 = (1..=N).into_par_iter().map(|n| 1.0 / (n as f64)).sum();
    println!("  浮点并行求和 = {sum_f64:.6}");
    println!("  ⚠ 浮点加法不满足结合律，而并行的合并顺序由线程调度决定：");
    println!("    同一程序两次运行的末位可能不同；要可复现就得串行或用定点数。\n");

    // ---------- ③ itertools：顺序迭代器的补充 ----------
    // 注意：这些是**顺序**工具，和 rayon 无关；rayon 负责「并行」，
    // itertools 负责「少写 for 循环」，两者经常一起出现在同一个 crate 里。
    println!("── ③ itertools：chunk_by / zip_longest / join ──");

    let votes = [1, 1, 1, 2, 2, 3, 3, 3, 3];
    print!("  chunk_by 把**相邻**相同元素分组: ");
    // 注意：chunk_by() 返回的是惰性结构体，要迭代它的**引用**（`&chunks`），
    // 直接 `for .. in chunks` 会因为 ChunkBy 本身没实现 Iterator 而编译失败。
    let chunks = votes.iter().chunk_by(|&&v| v);
    for (key, group) in &chunks {
        let count = group.count();
        print!("[{key} × {count}] ");
    }
    println!();
    println!("    （只合并相邻项；想全局分组请先排序，或改用 counts()/into_group_map()）");

    let names = ["ada", "bob", "cyd"];
    let scores = [90, 85];
    print!("  zip_longest 补齐长度不等的两个序列: ");
    let mut parts = Vec::new();
    for pair in names.iter().zip_longest(scores.iter()) {
        match pair {
            EitherOrBoth::Both(n, s) => parts.push(format!("{n}={s}")),
            EitherOrBoth::Left(n) => parts.push(format!("{n}=缺考")),
            EitherOrBoth::Right(s) => parts.push(format!("(?={s})")),
        }
    }
    println!("{}", parts.iter().join(", "));

    let crates_used = ["rayon", "itertools", "std"];
    println!(
        "  join 把迭代器拼成字符串: {}",
        crates_used.iter().join(" + ")
    );

    println!("\n完成：并行的关键不是「更快的 for」，而是「把数据切成互不依赖的块」。");
}
