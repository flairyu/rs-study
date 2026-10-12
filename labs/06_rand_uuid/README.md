# 06_rand_uuid · 随机数与唯一 ID（rand + uuid）

> 目录名带序号只为排序。**cargo 不允许包名以数字开头**，所以 `labs/` 里所有成员的包名
> 统一是 `lXX_` 前缀：目录 `06_rand_uuid`，包名 `l06_rand_uuid`。

| 项 | 值 |
|---|---|
| 包名 | `l06_rand_uuid` |
| 覆盖的 crate | `rand` 0.9.x、`uuid` 1.x（features: `v4`, `v7`, `serde`） |
| 运行 | `cargo run -p l06_rand_uuid`（第一次要联网拉依赖） |

## 它解决什么问题

两件事经常一起出现：

1. **随机数**：模拟、抽样、抖动（jitter）、加重试、A/B 分流、测试数据；
   关键能力不是「能生成随机数」，而是**可控**：指定范围、指定分布、可复现。
2. **唯一 ID**：数据库主键、请求追踪 ID、幂等键。

`rand` 提供密码学安全的线程局部 RNG（自动用 OS 熵播种）与各种分布；
`uuid` 提供 v4（纯随机）和 v7（时间戳前缀 + 随机）两种常用 UUID。

## 程序演示了什么

1. **基本随机值**：整数 `random_range(1..=6)`、浮点 `random::<f64>()` / `random_range`、
   布尔 `random_bool(0.5)` / `random_ratio(1, 3)`、`rand::random::<u32>()`。
2. **取样**：`IndexedRandom` 的 `choose` / `choose_multiple`（不重复，超出长度静默截断），
   以及「有放回」抽样怎么写。
3. **分布**：`Uniform::new(10.0, 200.0)`（循环里复用比重建划算）、
   `WeightedIndex::new([5, 3, 2])`（加权抽样，抽 1000 次看比例）。
4. **可复现**：`StdRng::seed_from_u64(42)`，两个同种子 RNG 输出完全一致，换种子则不同。
5. **UUID**：`Uuid::new_v4()`、`Uuid::now_v7()`、按字节序排序验证 v7 的时间有序性、
   `Uuid::parse_str` 的成功与失败分支、`Uuid::nil()` / `Uuid::max()`。

> `serde` feature 也开着：这样 `Uuid` 字段能直接放进你自己的 `#[derive(Serialize)]` 结构体，
> 序列化成字符串、反序列化时自动校验格式。本实验没引入 `serde_json`，所以没有当场演示，
> 但 `Cargo.toml` 里已经按「以后要接 serde」的样子开好了。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
cargo run -p l06_rand_uuid
cargo run -p l06_rand_uuid   # 再跑一次：除了「种子 42」那一段，其它都会变
```

## 预期输出（节选，随机部分每次不同）

```text
== 实验 06 · rand 0.9 + uuid ==

── ① 随机整数 / 浮点 / 布尔 ──
  掷骰子 random_range(1..=6)      = 4
  浮点 random::<f64>()            = 0.618892（取值 [0,1)）
...
── ② 从集合里随机取样（IndexedRandom）──
  choose 选一台: Some("osaka-2")（返回 Option，空集合时为 None）
  choose_multiple 选三台（不重复）: ["tokyo-2", "sapporo-1", "osaka-1"]
...
── ③ 范围分布与加权分布 ──
  WeightedIndex([5,3,2]) 抽 1000 次: 红 503 / 绿 298 / 蓝 199（期望 500/300/200）
...
── ④ 固定种子 = 可复现（测试里非常有用）──
  种子 42 的 RNG A 前 5 个: [13, 52, 24, 54, 86]
  种子 42 的 RNG B 前 5 个: [13, 52, 24, 54, 86]
  两个 RNG 输出完全一致 → 断言通过
...
── ⑤ UUID v4 / v7 / 解析 / nil ──
  v4 = 3f2a...（每次不同）
  v7 = 0199...（开头是时间戳，同一毫秒内才随机）
  版本号: v4 -> 4，v7 -> 7
  按字节序排序后: [...] → 先生成的排在前面，适合做数据库主键
  Uuid::nil() = 00000000-0000-0000-0000-000000000000，is_nil() = true
```

> 上面「种子 42」那 5 个数是 0.9.x 当前版本的实际结果；`rand` 升级大版本后可能变，
> 所以**不要**把具体数值写进单元测试，只断言「同种子两次结果相等」。

## 什么时候别用它

- **要密码学强度**（生成 token、会话 ID、密码盐）：用 `rand::rng()` 这类 CSPRNG 没错，
  但别用 `SmallRng`/自造 LCG；密钥材料应该用 `getrandom` 直接取 OS 熵。UUID v4 依赖
  随机数，**不要**拿它当访问令牌（它没有「不可猜测」的承诺，也不同一律用 CSPRNG 播种）。
- **要严格复现的跨版本结果**：`StdRng` 的算法随版本可能变；要稳定就得自带 PRNG 实现。
- **UUID 当顺序主键**：v4 完全随机，做 MySQL/InnoDB 聚簇主键会造成页分裂；
  用 v7（时间有序）或自增 ID。
- **把 `choose_multiple` 当成「一定拿到 n 个」**：集合不够长时是静默截断。
- **在热路径频繁 `WeightedIndex::new`**：构造要建前缀和，应该建一次反复 `sample`。

## 常见坑（rand 0.9 与 0.8 的 API 差异）

| 0.8 写法 | 0.9 写法 |
|---|---|
| `rand::thread_rng()` | `rand::rng()` |
| `rng.gen_range(1..=6)` | `rng.random_range(1..=6)` |
| `rng.gen::<f64>()` / `rng.gen_bool(0.5)` | `rng.random::<f64>()` / `rng.random_bool(0.5)` |
| `use rand::seq::SliceRandom;`（`choose` 在这里） | `use rand::seq::IndexedRandom;` |
| `rand::distributions::*` | `rand::distr::*` |
| `use rand::Rng;` 就有 `gen_*` | `use rand::Rng;` 才有 `random_*`（忘了导入会报「no method」） |

- `seed_from_u64` 来自 `rand::SeedableRng`，忘了导入 trait 会找不到方法。
- `choose_multiple` 返回的是**惰性迭代器**（不是 `Vec`），要 `.collect()`；
  而且对 `[&str; N]` 这种数组，元素类型是 `&&str`，用 `.copied()` 脱一层才好放进 `Vec<&str>`。
- `uuid` 的 `v4`/`v7` 是 **feature**：只写 `uuid = "1"` 时 `new_v4()` 不存在。
