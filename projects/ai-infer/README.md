# ai-infer —— 第 21 章配套 demo

**一句话**：不下载任何模型权重，把「推理」这件事的骨架跑一遍——张量运算 → 2 层 MLP 前向传播 → `Result` 错误处理 → matmul benchmark。

配套文档：[`../../docs/04-frameworks/ch21-ai-inference.md`](../../docs/04-frameworks/ch21-ai-inference.md)

---

## 快速开始

```bash
cd /home/yxb/code/rs-study/projects/ai-infer
cargo run
```

零依赖、离线可跑、约 0.3 秒编译完。

> **受限环境（只读 `~/.cargo`）**：把 cargo 缓存指到工作区内
> ```bash
> export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home
> ```

---

## 这里到底跑了什么？

四个 demo，都在 `src/main.rs` 里：

| # | 函数 | 演示内容 | 用到的算子 |
|---|---|---|---|
| ① | `demo_tensor_basics` | 张量创建与基本运算 | `matmul`、`broadcast_add`、`softmax`、`argmax`、`mul`、`mean_all` |
| ② | `demo_mlp_forward` | 手写 2 层 MLP 前向传播 | `matvec` + `broadcast_add` + `relu` + `softmax` + `argmax` |
| ③ | `demo_error_handling` | 形状/类型错误只在**运行时**暴露 | 匹配 `TensorError`、`?` 传播 |
| ④ | `demo_bench_matmul` | 64×64 matmul × 200 次平均耗时 | `std::time::Instant` |

网络拓扑（②的主体）：

```
x(4) ──matvec(W1: 4×8)──▶ h(8) ──+b1(8)──▶ relu ──matvec(W2: 8×3)──▶ logits(3) ──+b2(3)──▶ softmax ──▶ argmax
```

---

## `cargo run` 的实际输出（实测，逐字）

```
device = Cpu（纯 std 教学张量；candle 版见 README）

=== 1. 张量创建与基本运算 ===
a.shape = (2, 3)，dtype = f32，numel = 6
a = Tensor[2, 3] [1.0000e0, 2.0000e0, 3.0000e0, 4.0000e0, 5.0000e0, 6.0000e0]
a.matmul(b).shape = (2, 2)
a.matmul(b) = Tensor[2, 2] [5.8000e1, 6.4000e1, 1.3900e2, 1.5400e2]
prod.broadcast_add(bias) = Tensor[2, 2] [6.1000e1, 6.3000e1, 1.4200e2, 1.5300e2]
softmax(dim=-1) = Tensor[2, 2] [1.1920e-1, 8.8080e-1, 1.6701e-5, 9.9998e-1]
每行和 = [1.000000, 1.000000]（softmax 后必然为 1）
argmax(dim=-1) = Tensor[2, 1] [1.0000e0, 1.0000e0]
prod^2 的均值 = 12624.2500

=== 2. 手写 2 层 MLP 的前向传播 ===
x = Tensor[4] [5.0000e-1, -1.2000e0, 3.0000e-1, 2.0000e0]
W1.shape = (4, 8)，W2.shape = (8, 3)
softmax(logits) = Tensor[3] [3.8054e-1, 3.9620e-1, 2.2327e-1]
预测类别 = 1，置信度 = 0.3962
（权重用的是固定种子，所以每次运行结果一致；换 seed 就会变，见练习 21-1）

=== 3. Result 与形状错误（运行时才报） ===
matmul 形状不匹配 -> matmul: shapes [2, 3] and [2, 2] cannot be combined
broadcast_add 形状不匹配 -> broadcast_add: shapes [2, 3] and [5] cannot be combined
Tensor::new 数据/形状不符 -> unexpected number of elements: expected 4, got 3
用 ? 传播错误并取值成功 -> Ok(0.5761169)
a.i(5) -> index 5 out of bounds for tensor of length 2

=== 4. matmul benchmark（Instant 计时） ===
矩阵规模 = 64×64，迭代 = 200 次（单线程）
总耗时 = 434.67ms，平均 = 2.173ms/次，每个 MAC 约 8.29 ns
校验和 = -6503.0（防止编译器优化掉循环）
提示：这只看「相对趋势」，不是严谨 benchmark——正式测量请用 criterion，
      并和 BLAS/mimalloc 等真实后端对比（candle 的 CPU 后端就走 gemm）。
```

第 1 节的 `a.matmul(b)` 可以手算验证：`[1,2,3;4,5,6] × [7,8;9,10;11,12] = [58,64;139,154]`。
softmax 那一行也值得看：`exp(3) : exp(-1)` 悬殊，所以是 `0.119 : 0.881`，而第二行 `exp(-1) : exp(-3)`
相对接近，于是「每行和 = 1」。

> 📌 **只有第 4 节的数字会变**（实测同一台机器在 434–460ms 之间波动，±5% 属正常），
> 第 1–3 节的输出是完全确定的：固定种子 + 确定性算法，逐字节可复现。

---

## 为什么默认是纯 `std`，不是 `candle`？

这是**环境约束下的诚实选择**，不是偷懒：

1. 本教程运行环境的 `~/.cargo` 只读，且**带宽极窄**（实测 `static.crates.io` ≈ 19 KB/s）。
2. candle-core 会拖进 `gemm`、`half`、`rayon`、`safetensors` 等几十个 crate，
   **cargo 的依赖解析在下载之前就要跑完**——光是解析，实测 `cargo fetch` 就报：
   ```
   error: failed to get `gemm` as a dependency of package `candle-core v0.9.2`
   Caused by: download of ge/mm/gemm failed
   Caused by: transfer too slow: failed to transfer more than 10 bytes in 30s (transferred 0 bytes)
   ```
   （也试过把 candle 塞进 `[target.'cfg(target_family = "wasm")'.dependencies]` 想绕开下载，
   但 **cargo 连用不到的 target 下的 optional 依赖也会去解析**，同样卡死。）
3. 而本章要讲的东西——张量形状、矩阵乘、广播、softmax 数值稳定性、形状错误只在运行时暴露、
   计时——**一个都不需要真实的张量库**。

所以 `src/main.rs` 手写了一个约 630 行的 `Tensor`（含大量中文注释），**API 命名刻意对齐 `candle_core::Tensor`**：
`new` / `matmul` / `broadcast_add` / `relu` / `softmax` / `argmax` / `i` / `sum_all` / `mean_all`。
你在本 demo 里学到的调用顺序，换成 candle 之后**一行都不用改结构**。

---

## candle 路线（需要网络，本环境未跑通）

完整实现放在 [`src/main_candle.rs`](src/main_candle.rs)：它就是同一份 demo 的 candle 版，
逐行对应 std 版，并且包含只有在真实张量库上才有的东西——
`Device`、`DType` 转换、`I64` 的 `argmax`、`candle_core::Error`。

**启用步骤**（在有网络的机器上）：

1. `Cargo.toml` 的 `[dependencies]`（当前是空的，注释说明了原因）加：
   ```toml
   candle-core = "0.9"      # 或 0.10 / 0.11，本 demo 用到的 API 一致
   ```
2. 在 `src/main.rs` 顶部加 `mod main_candle;`，把 `main` 改成
   `fn main() -> candle_core::Result<()> { main_candle::main() }`
   （或者直接把 `main_candle.rs` 改名成 `main.rs`）。
3. `cargo run`。第一次编译几十个 crate，几分钟起步，属正常。

**std 版 vs candle 版的关键差异**（也是文档里反复强调的点）：

| 概念 | 本 demo（std） | candle |
|---|---|---|
| 设备 | 永远 CPU | `Device::Cpu` / `Device::new_cuda(0)?`（需 `cuda` feature） |
| 形状 | `vec![2, 3]` | `.reshape((2, 3))?`，且形状参与类型 `Shape` |
| rank-1 × 矩阵 | 需要单独的 `matvec` | `x.matmul(&w)` 自动把 rank-1 当行向量 |
| `argmax` 返回 | `f32` | **`I64` 张量**，要 `to_scalar::<i64>()` |
| 随机数 | 自写固定种子 LCG | `Tensor::rand(lo, hi, shape, dev)?`，**不吃 seed** |
| 错误类型 | 自定义 `TensorError` | `candle_core::Error`（`#[non_exhaustive]`） |

---

## 想改成真模型？三条路的依赖坐标

本 demo 刻意不碰模型文件。真要加载权重时（**都需要网络**）：

| 路线 | Cargo.toml | 还需要什么 |
|---|---|---|
| ONNX（Python 训练 → Rust 部署） | `ort = "=2.0.0-rc.13"` | 一个 `.onnx` 文件；onnxruntime 动态库（`download-binaries` feature 会自动下，需网络） |
| safetensors + candle | `candle-core = "0.9"`, `candle-nn = "0.9"` | `.safetensors` 权重 + 在 Rust 里重写网络结构 |
| 纯 Rust 训练 | `burn = "0.22"`（配一个后端如 `burn-ndarray`） | 训练数据；后端可换 wgpu/CUDA |

关于 `ort` / `burn` / `tch` 的取舍、三条路线的选择、以及一条 LLM/RAG 服务链路的组件图，
见 [`../../docs/04-frameworks/ch21-ai-inference.md`](../../docs/04-frameworks/ch21-ai-inference.md)。

---

## 可以怎么改（练习入口）

- **改权重种子**：`Tensor::uniform(vec![4, 8], 0.5, 1)` 里的最后一个参数是 seed。
  改成 `42`、`7`，看第 2 节「预测类别」怎么变——这就是练习 21-1。
- **改网络结构**：把 `vec![4, 8]` 改成 `vec![4, 16]`，同时把 `W2` 改成 `vec![16, 3]`，
  再自己加一层。注意 `b1` 的长度必须跟着变（这是最容易漏的一处）。
- **加一个类别**：`W2` 改成 `vec![8, 5]`、`b2` 换成 5 个值。
- **看形状错误长什么样**：把第 3 节里 `wrong` 的形状改成 `(3, 2)`，错误信息会变。
- **换 benchmark 规模**：`demo_bench_matmul` 里的 `n` 改成 `128`、`256`，观察
  "每个 MAC 的纳秒数"是否变化（小矩阵摊不开 cache，单位成本会更高）。

---

## 期望行为 vs 已知限制

**可预期**：

- `cargo run` 必定有输出，且第 1、3、4 节的结果完全可复现（固定种子 + 确定性算法）。
- 第 4 节的耗时**会随机器变化**（上面 434.67ms / 2.173ms 是这台机器上的一次实测，多次运行会波动 ±5%）。

**限制（不要误解）**：

- 这个 `Tensor` 是**教学替身**：只支持 1D/2D、只有 f32、广播只实现"最后一维"这一条规则、
  没有 autograd、没有 GPU。它不足以支撑真实模型，只够把概念跑清楚。
- 所以：**要跑真模型，请走上面的 candle / ort 路线**，本 crate 不是可用的推理框架。
