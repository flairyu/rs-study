//! **candle 版本的同一份 demo** —— 需要网络 + 能成功编译 `candle-core` 的机器。
//!
//! ## 为什么它不是默认入口
//!
//! 本教程的运行环境里，cargo 连 candle 的**依赖解析**都跑不完：
//! `cargo fetch` 报 `failed to transfer more than 10 bytes in 30s :: download of ge/mm/gemm failed`
//! （实测 static.crates.io 只有约 19 KB/s，而 candle-core 会拖进 gemm / half /
//! rayon / safetensors 等几十个 crate）。所以默认入口是纯 std 的 `src/main.rs`，
//! 本文件作为**参考实现**保留，代码是按 candle 真实 API 写的。
//!
//! ## 怎么启用它（在有网络的机器上）
//!
//! 1. 在 `Cargo.toml` 的 `[dependencies]`（不是空的那个注释块）下加一行：
//!
//!    ```toml
//!    candle-core = "0.9"      # 或 0.10 / 0.11，本文件用到的 API 一致
//!    ```
//!
//! 2. 在 `src/main.rs` 顶部加 `mod main_candle;`，并把 `main` 换成
//!    `fn main() -> candle_core::Result<()> { main_candle::main() }`
//!    （或者直接把本文件重命名成 `main.rs`、删掉旧的）。
//!
//! 3. `cargo run` —— 第一次会编译几十个 crate，几分钟起步，属正常。
//!
//! ## 与 `src/main.rs` 的对照
//!
//! 四个 demo 的结构**逐行对应**，唯一区别是 `Tensor` 从哪来：
//!
//! | 概念 | 纯 std 版（默认） | candle 版（本文件） |
//! |---|---|---|
//! | 张量类型 | `struct Tensor { shape, data }` | `candle_core::Tensor` |
//! | 构造 | `Tensor::new(&[...], vec![2,3])` | `Tensor::new(&[...], &dev)?.reshape((2,3))?` |
//! | 矩阵乘 | `a.matmul(&b)` / `a.matvec(&w)` | `a.matmul(&b)`（rank-1 自动当行向量） |
//! | 广播加法 | `broadcast_add`（只支持最后一维） | `broadcast_add`（完整 NumPy 规则） |
//! | softmax | `softmax()` | `softmax(D::Minus1)` |
//! | 取类别 | `argmax()` -> f32 | `argmax(D::Minus1)?` -> **i64** |
//! | 设备 | 无（永远 CPU） | `Device::Cpu` / `Device::new_cuda(0)` |
//! | 错误 | 自定义 `TensorError` | `candle_core::Error`（`thiserror` 风格） |
//! | 随机初始化 | 手写固定种子 LCG | `Tensor::rand` / `randn`（无 seed） |

use candle_core::{DType, Device, IndexOp, Result, Tensor, D};

fn main() -> Result<()> {
    let dev = Device::Cpu;
    println!("device = {dev:?}");
    println!("candle-core 版本见 Cargo.toml / Cargo.lock");

    demo_tensor_basics(&dev)?;
    demo_mlp_forward(&dev)?;
    demo_error_handling(&dev)?;
    demo_bench_matmul(&dev)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// ① 张量创建与基本运算
// ---------------------------------------------------------------------------

fn demo_tensor_basics(dev: &Device) -> Result<()> {
    section("1. 张量创建与基本运算");

    // Tensor::new 吃实现 NdArray 的类型；形状要么用 reshape，要么用 (shape, data) 元组
    let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], dev)?.reshape((2, 3))?;
    let b = Tensor::new(&[7f32, 8., 9., 10., 11., 12.], dev)?.reshape((3, 2))?;
    println!("a.shape = {:?}, dtype = {:?}", a.shape(), a.dtype());
    println!("a = {a}");

    // matmul：(2,3) x (3,2) -> (2,2)
    let prod = a.matmul(&b)?;
    println!("a.matmul(b).shape = {:?}", prod.shape());
    println!("a.matmul(b) = {prod}");

    // broadcast_add：(2,) 广播到 (2,2)
    let bias = Tensor::new(&[3f32, -1.], dev)?;
    let shifted = prod.broadcast_add(&bias)?;
    println!("prod.broadcast_add(bias) = {shifted}");

    // softmax 需要显式给维度；D::Minus1 == 最后一维
    let probs = shifted.softmax(D::Minus1)?;
    println!("softmax(dim=-1) = {probs}");
    println!("每行和 = {}", probs.sum(D::Minus1)?);

    // argmax 返回 I64 张量，取标量要 to_scalar::<i64>()
    let best = probs.argmax(D::Minus1)?;
    println!("argmax(dim=-1) = {best}");

    // 逐元素运算 + 归约
    println!("prod^2 的均值 = {}", (&prod * &prod)?.mean_all()?.to_scalar::<f32>()?);

    // dtype 是可变的：f32 <-> f64 转换是显式的（对应 NumPy 的 astype）
    let as_f64 = a.to_dtype(DType::F64)?;
    println!("a.to_dtype(F64).dtype = {:?}", as_f64.dtype());

    // rank-1 张量可以直接当行向量做 matmul（std 版需要单独的 matvec）
    let x = Tensor::new(&[0.5f32, -1.2, 0.3, 2.0], dev)?;
    let w = Tensor::new(&[1f32, 0., 0., 1., 0.5, -0.5, 2., 0.], dev)?.reshape((4, 2))?;
    println!("rank-1 x (4,) . matmul((4,2)) -> {:?}", x.matmul(&w)?.shape());

    Ok(())
}

// ---------------------------------------------------------------------------
// ② 手写 2 层 MLP 的前向传播（权重用固定种子生成，零下载）
// ---------------------------------------------------------------------------

/// `x(4) -> Linear(4→8) -> ReLU -> Linear(8→3) -> softmax`
fn mlp_forward(x: &Tensor, w1: &Tensor, b1: &Tensor, w2: &Tensor, b2: &Tensor) -> Result<Tensor> {
    let h = x.matmul(w1)?.broadcast_add(b1)?.relu()?;
    let logits = h.matmul(w2)?.broadcast_add(b2)?;
    logits.softmax(D::Minus1)
}

fn demo_mlp_forward(dev: &Device) -> Result<()> {
    section("2. 手写 2 层 MLP 的前向传播");

    let x = Tensor::new(&[0.5f32, -1.2, 0.3, 2.0], dev)?;

    // 注意：candle 的 rand 不吃 seed，也没有 PyTorch 的 manual_seed。
    // 想要可复现就自己控制初始化（见下面 fixed_uniform），或固定随机种子缓存权重。
    let w1 = fixed_uniform(&[], (4, 8), 0.5, 1, dev)?;
    let b1 = Tensor::new(&[0.01f32, -0.02, 0.03, 0.0, 0.02, -0.01, 0.04, -0.03], dev)?;
    let w2 = fixed_uniform(&[], (8, 3), 0.5, 2, dev)?;
    let b2 = Tensor::new(&[0.0f32, 0.05, -0.05], dev)?;

    println!("W1.shape = {:?}, W2.shape = {:?}", w1.shape(), w2.shape());

    let probs = mlp_forward(&x, &w1, &b1, &w2, &b2)?;
    println!("softmax(logits) = {probs}");

    // argmax 是 I64；取第 i 个元素用 .i(i)
    let class = probs.argmax(D::Minus1)?.to_scalar::<i64>()? as usize;
    let confidence = probs.i(class)?.to_scalar::<f32>()?;
    println!("预测类别 = {class}，置信度 = {confidence:.4}");

    // 真实项目里权重这么来（需要 .safetensors 文件 + candle-nn）：
    //
    //   let vb = candle_nn::VarBuilder::from_mmaped_safetensors(
    //       &["model.safetensors"], DType::F32, dev)?;
    //   let w1 = vb.get((4, 8), "layer1.weight")?;   // 输出层的代码完全不变
    //
    // 这就是本章的要点：**前向传播的形状是稳定的，变的是权重从哪来。**

    Ok(())
}

/// 固定种子 LCG 生成 `[-scale, scale]` 的均匀矩阵（可复现，替代 `rand`）。
///
/// 真随机版本就一行：`Tensor::rand(-0.5f32, 0.5f32, shape, dev)?`
/// （或在 `candle-core` 上开 `rand` 特性后用 `randn`）。它返回 `Result`，
/// 且每次运行结果不同——想复现就得像这里一样把种子握在自己手里。
fn fixed_uniform(
    head: &[f32],
    shape: (usize, usize),
    scale: f32,
    seed: u64,
    dev: &Device,
) -> Result<Tensor> {
    let (rows, cols) = shape;
    let n = rows * cols;
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        if i < head.len() {
            data.push(head[i]);
        } else {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let u = ((state >> 40) as f32) / (1u32 << 24) as f32;
            data.push((u * 2.0 - 1.0) * scale);
        }
    }
    Tensor::new(data.as_slice(), dev)?.reshape(shape)
}

// ---------------------------------------------------------------------------
// ③ Result 错误处理：形状不匹配只在运行时暴露
// ---------------------------------------------------------------------------

fn demo_error_handling(dev: &Device) -> Result<()> {
    section("3. Result 与形状错误（运行时才报）");

    let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], dev)?.reshape((2, 3))?;
    let wrong = Tensor::new(&[1f32, 2., 3., 4.], dev)?.reshape((2, 2))?;

    // (2,3) x (2,2)：内维不等。编译器完全帮不上忙——形状是运行时值。
    // 这里不 match 具体变体（candle 的 Error 是 #[non_exhaustive] 的），只打印 Display。
    match a.matmul(&wrong) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("matmul 形状不匹配 -> {e}"),
    }

    // 广播失败：给 (2,3) 加 (5,)
    match a.broadcast_add(&Tensor::new(&[0f32; 5], dev)?) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("broadcast_add 形状不匹配 -> {e}"),
    }

    // dtype 也不是编译期的事：f32 张量按 u32 取标量
    match Tensor::new(&[1.5f32], dev)?.to_scalar::<u32>() {
        Ok(v) => println!("不该走到这里: {v}"),
        Err(e) => println!("to_scalar::<u32>() 类型不符 -> {e}"),
    }

    // `?` 把 candle 错误直接往上传（candle_core::Error: std::error::Error + Send + Sync）
    let ok: Result<f32> = (|| {
        let x = Tensor::new(&[1f32, 0., 0.], dev)?;
        Ok(x.softmax(D::Minus1)?.i(0)?.to_scalar::<f32>()?)
    })();
    println!("用 ? 传播错误并取值成功 -> {ok:?}");

    Ok(())
}

// ---------------------------------------------------------------------------
// ④ 简单 benchmark：N 次 matmul 的平均耗时
// ---------------------------------------------------------------------------

fn demo_bench_matmul(dev: &Device) -> Result<()> {
    section("4. matmul benchmark（Instant 计时）");

    let n = 64usize;
    let iters = 200;

    let a = fixed_uniform(&[], (n, n), 1.0, 7, dev)?;
    let b = fixed_uniform(&[], (n, n), 1.0, 11, dev)?;

    let _ = a.matmul(&b)?; // 预热

    let start = std::time::Instant::now();
    let mut acc = 0f32;
    for _ in 0..iters {
        let c = a.matmul(&b)?;
        // 必须"消费"结果，否则整段循环可能被优化掉
        acc += c.sum_all()?.to_scalar::<f32>()?;
    }
    let elapsed = start.elapsed();

    println!("矩阵规模 = {n}×{n}，迭代 = {iters} 次");
    println!(
        "总耗时 = {:.2?}，平均 = {:.3?}/次，每个 MAC 约 {:.2} ns",
        elapsed,
        elapsed / iters as u32,
        elapsed.as_secs_f64() * 1e9 / (iters as f64 * (n * n * n) as f64)
    );
    println!("校验和 = {acc:.1}（防止编译器优化掉循环）");
    println!("提示：candle 的 CPU 后端底层走 gemm，默认是多线程的——");
    println!("      和单线程手写循环比会快很多，但也要注意线程数的公平性。");

    Ok(())
}

// ---------------------------------------------------------------------------

fn section(title: &str) {
    println!("\n=== {title} ===");
}
