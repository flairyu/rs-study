//! 实验 06 · rand 0.9 + uuid 1.x：随机数与唯一 ID
//!
//! 五段演示：
//! 1. 随机整数 / 浮点 / 布尔；
//! 2. 从集合里随机取样（`choose` / `choose_multiple`）；
//! 3. 范围与加权分布（`random_range` / `Uniform` / `WeightedIndex`）；
//! 4. 固定种子的**可复现**随机（`StdRng::seed_from_u64`）；
//! 5. UUID v4 / v7、解析字符串、`Uuid::nil()`。
//!
//! 运行：`cargo run -p l06_rand_uuid`
//!
//! ⚠ rand 0.9 与 0.8 的 API 差别很大，本文件按 0.9 的真实 API 编写：
//!   `thread_rng()` → `rand::rng()`、`gen_range` → `random_range`、
//!   `rand::distributions` → `rand::distr`、
//!   切片随机方法从 `SliceRandom` 挪到 `IndexedRandom`。

use rand::distr::weighted::WeightedIndex;
use rand::distr::{Distribution, Uniform};
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use uuid::Uuid;

fn main() {
    println!("== 实验 06 · rand 0.9 + uuid ==\n");

    // ---------- ① 基本随机值 ----------
    println!("── ① 随机整数 / 浮点 / 布尔 ──");
    // rng() 拿到线程局部、自动播种的 ThreadRng（0.8 里叫 thread_rng()）。
    let mut rng = rand::rng();

    let dice: u8 = rng.random_range(1..=6); // 闭区间用 ..=，半开区间用 ..
    let index: usize = rng.random_range(0..10);
    let coin: bool = rng.random_bool(0.5);
    let biased: bool = rng.random_bool(0.9); // 90% 概率为 true
    let ratio: bool = rng.random_ratio(1, 3); // 1/3 概率为 true
    let x: f64 = rng.random(); // 默认 [0, 1)
    let y: f32 = rng.random_range(-1.0..1.0);
    let letter: char = rng.sample(rand::distr::Alphanumeric) as char;

    println!("  掷骰子 random_range(1..=6)      = {dice}");
    println!("  下标 random_range(0..10)        = {index}");
    println!("  抛硬币 random_bool(0.5)         = {coin}");
    println!("  加权随机 random_bool(0.9)       = {biased}（大概率 true）");
    println!("  random_ratio(1, 3)              = {ratio}（约 1/3 为 true）");
    println!("  浮点 random::<f64>()            = {x:.6}（取值 [0,1)）");
    println!("  浮点 random_range(-1.0..1.0)    = {y:.6}");
    println!("  随机字母 Alphanumeric           = {letter}");
    println!(
        "  rand::random::<u32>()           = {}",
        rand::random::<u32>()
    );
    println!();

    // ---------- ② 从集合里取样 ----------
    println!("── ② 从集合里随机取样（IndexedRandom）──");
    let servers = ["tokyo-1", "tokyo-2", "osaka-1", "osaka-2", "sapporo-1"];

    let one = servers.choose(&mut rng);
    println!("  choose 选一台: {:?}（返回 Option，空集合时为 None）", one);

    // choose_multiple 是**不重复**抽样：最多拿走集合长度个。
    // 它返回惰性迭代器而不是 Vec，所以要 collect；又因为 `servers` 是 `[&str; 5]`，
    // 元素类型是 `&str`，抽样出来的是 `&&str`，用 `.copied()` 脱一层引用。
    let three: Vec<&str> = servers.choose_multiple(&mut rng, 3).copied().collect();
    println!("  choose_multiple 选三台（不重复）: {:?}", three);

    let all = servers.choose_multiple(&mut rng, 99);
    println!(
        "  要 99 个但只有 {} 个: 实得 {} 个（静默截断，不报错）",
        servers.len(),
        all.len()
    );

    // 需要「可以重复」的抽样（比如抽奖允许同一人中两次）就自己配 random_range。
    let with_replacement: Vec<&str> = (0..4)
        .map(|_| servers[rng.random_range(0..servers.len())])
        .collect();
    println!("  有放回抽样 4 次: {with_replacement:?}");
    println!("  ⚠ 随机取样依赖迭代器长度可数；对 HashMap 直接取样前先 collect 成 Vec。\n");

    // ---------- ③ 范围与加权分布 ----------
    println!("── ③ 范围分布与加权分布 ──");
    // Uniform 把「区间对象」预先算好，循环里复用比重建 random_range 更快。
    let latency = Uniform::new(10.0, 200.0).expect("区间必须非空");
    let samples: Vec<f64> = (0..5).map(|_| latency.sample(&mut rng)).collect();
    let readable: Vec<String> = samples.iter().map(|v| format!("{v:.1}ms")).collect();
    println!("  Uniform(10.0, 200.0) 采 5 个: {}", readable.join(", "));

    // 加权：不是等概率，而是按权重比例。
    let colors = ["红", "绿", "蓝"];
    let weights = [5, 3, 2]; // 红 50%、绿 30%、蓝 20%
    let dist = WeightedIndex::new(weights).expect("权重之和必须大于 0");
    let mut tally = [0usize; 3];
    for _ in 0..1_000 {
        tally[dist.sample(&mut rng)] += 1;
    }
    println!(
        "  WeightedIndex([5,3,2]) 抽 1000 次: 红 {} / 绿 {} / 蓝 {}（期望 500/300/200）",
        tally[0], tally[1], tally[2]
    );
    println!("  可选颜色: {colors:?}");
    println!("  ⚠ 权重全 0 或含负数时 WeightedIndex::new 会返回 Err，别 unwrap 掉。\n");

    // ---------- ④ 可复现随机 ----------
    println!("── ④ 固定种子 = 可复现（测试里非常有用）──");
    let mut rng_a = StdRng::seed_from_u64(42);
    let mut rng_b = StdRng::seed_from_u64(42); // 同一个种子

    let seq_a: Vec<u32> = (0..5).map(|_| rng_a.random_range(0..100)).collect();
    let seq_b: Vec<u32> = (0..5).map(|_| rng_b.random_range(0..100)).collect();
    println!("  种子 42 的 RNG A 前 5 个: {seq_a:?}");
    println!("  种子 42 的 RNG B 前 5 个: {seq_b:?}");
    assert_eq!(seq_a, seq_b, "同种子必须产生同序列");
    println!("  两个 RNG 输出完全一致 → 断言通过");

    // 不同的种子必须给出不同的序列（否则「随机」就没意义了）。
    let mut rng_c = StdRng::seed_from_u64(43);
    let seq_c: Vec<u32> = (0..5).map(|_| rng_c.random_range(0..100)).collect();
    println!("  把种子换成 43: {seq_c:?}");
    assert_ne!(seq_a, seq_c, "不同种子应给出不同序列");

    // 继续往下取，两个同种子 RNG 依然同步 —— 说明「种子决定整条序列」。
    println!(
        "  再从 A/B 各取一个: {} vs {}（依然相同）",
        rng_a.random_range(0..100),
        rng_b.random_range(0..100)
    );
    println!("  ⚠ 可复现 ≠ 跨版本稳定：rand 大版本升级后算法可能变，别把具体数值写进测试。");
    println!("    生产环境的种子要来自 OS 熵（默认的 rand::rng() 就是），不要写死。\n");

    // ---------- ⑤ UUID ----------
    println!("── ⑤ UUID v4 / v7 / 解析 / nil ──");

    let v4 = Uuid::new_v4(); // 纯随机
    let v7 = Uuid::now_v7(); // 前 48 位是毫秒时间戳，同毫秒内随机
    println!("  v4 = {v4}");
    println!("  v7 = {v7}");
    println!(
        "  版本号: v4 -> {}，v7 -> {}",
        v4.get_version_num(),
        v7.get_version_num()
    );

    let mut v7_list: Vec<Uuid> = (0..3).map(|_| Uuid::now_v7()).collect();
    println!("  连出 3 个 v7（生成顺序）: {v7_list:?}");
    v7_list.sort(); // v7 按字节序排 ≈ 按时间排，这是它相对 v4 的最大优势
    println!("  按字节序排序后:          {v7_list:?}");
    println!("  → 先生成的排在前面，适合做数据库主键（v4 完全做不到）");

    // 解析：用户输入/配置文件里的 UUID 必须当心失败。
    let text = "018e4b2c-1a2b-7c3d-8e4f-5a6b7c8d9e0f";
    match Uuid::parse_str(text) {
        Ok(u) => println!(
            "  parse_str(\"{text}\") -> OK: {u}（version {}）",
            u.get_version_num()
        ),
        Err(e) => println!("  parse_str 失败: {e}"),
    }
    match Uuid::parse_str("not-a-uuid") {
        Ok(u) => println!("  居然解析出来了: {u}"),
        Err(e) => println!("  解析 \"not-a-uuid\" -> Err: {e}（真实代码里不要 unwrap）"),
    }

    let nil = Uuid::nil(); // 全 0：表示「没有值」，常作占位符
    let max = Uuid::max(); // 全 1：v7 排序时的上界，也常作「无穷大」
    println!("  Uuid::nil() = {nil}，is_nil() = {}", nil.is_nil());
    println!("  Uuid::max() = {max}");
    println!("  nil 全 0 → 数据库里表示『未分配』比 NULL 更好索引");

    println!("\n完成：随机的难点从来不是「怎么生成」，而是「怎么可复现、怎么不重复」。");
}
