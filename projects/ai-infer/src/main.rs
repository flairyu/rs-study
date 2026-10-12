//! 第 21 章配套 demo：**不下载任何模型权重**的一次推理演示。
//!
//! ```text
//! ① 张量创建与基本运算：matmul / broadcast_add / softmax / argmax
//! ② 手写 2 层 MLP 前向传播：输入向量 -> logits -> softmax -> 预测类别
//! ③ Result 错误处理：形状不匹配 / 类型不符只在运行时暴露，把错误打印出来
//! ④ benchmark：同一个 matmul 跑 N 次，用 std::time::Instant 算平均耗时
//! ```
//!
//! 整个程序**只用标准库**，不读 `.safetensors`、不联网、不需要 GPU、不需要 ONNX 文件，
//! `cargo run` 立刻有输出。之所以能这样，是因为本 demo 关注的是**推理的骨架**
//! （张量 -> 矩阵乘 -> 激活 -> 归一化 -> 取类别），而不是"某个具体模型"。
//!
//! 运行：
//!
//! ```bash
//! cd projects/ai-infer
//! cargo run
//! ```
//!
//! 想看 candle 版本（真正的张量库）怎么办？`src/main_candle.rs` 里有同一份 demo 的 candle 实现，
//! 但**本环境拉不下 candle 依赖**（详见该文件顶部与 README）——默认实现才走纯 std。
//! 本文件里的 `Tensor` 类型是**照着 candle 的 API 形状**写的教学替身：
//! `matmul` / `broadcast_add` / `softmax` / `argmax` 的名字与语义都对齐 `candle_core::Tensor`，
//! 所以这里的代码在你换成 candle 之后**结构完全不变**，只有类型来自哪里不同。

use std::fmt;
use std::time::Instant;

// ===========================================================================
// 0. 一个极简张量：形状 + 一层 f32 数据（为了讲清"形状是运行时值"）
// ===========================================================================

/// 教学用张量。真实项目里等价物是 `candle_core::Tensor`。
#[derive(Clone, Debug, PartialEq)]
struct Tensor {
    shape: Vec<usize>,
    data: Vec<f32>,
}

/// 张量运算的错误类型。
///
/// 关键点：**形状错误全部是运行时的**——`Tensor` 类型本身不携带形状信息，
/// 编译器拦不住 `(2,3) × (2,2)`，这一点和 candle / PyTorch / NumPy 完全一样。
#[derive(Debug, PartialEq)]
enum TensorError {
    /// 两个张量形状不兼容（矩阵乘的内维不等，或无法广播）
    ShapeMismatch {
        op: &'static str,
        lhs: Vec<usize>,
        rhs: Vec<usize>,
    },
    /// 数据个数与声明的形状对不上（candle 里对应 `UnexpectedShape`）
    LengthMismatch { expected: usize, got: usize },
    /// 维度数不满足该算子的要求
    RankError {
        op: &'static str,
        expected: usize,
        got: usize,
    },
    /// 下标越界
    IndexOutOfBounds { index: usize, len: usize },
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensorError::ShapeMismatch { op, lhs, rhs } => {
                write!(f, "{op}: shapes {lhs:?} and {rhs:?} cannot be combined")
            }
            TensorError::LengthMismatch { expected, got } => {
                write!(
                    f,
                    "unexpected number of elements: expected {expected}, got {got}"
                )
            }
            TensorError::RankError { op, expected, got } => {
                write!(f, "{op}: expected a rank-{expected} tensor, got rank {got}")
            }
            TensorError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} out of bounds for tensor of length {len}")
            }
        }
    }
}

impl std::error::Error for TensorError {}

type Result<T> = std::result::Result<T, TensorError>;

fn prod(shape: &[usize]) -> usize {
    shape.iter().product()
}

fn shape_of(shape: &[usize]) -> String {
    format!(
        "({})",
        shape
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl Tensor {
    /// 从一维数据 + 形状构造。`Tensor::from_slice(&[1., 2., 3., 4.], (2, 2))`
    fn new(data: &[f32], shape: impl Into<Vec<usize>>) -> Result<Self> {
        let shape = shape.into();
        let expected = prod(&shape);
        if expected != data.len() {
            return Err(TensorError::LengthMismatch {
                expected,
                got: data.len(),
            });
        }
        Ok(Tensor {
            shape,
            data: data.to_vec(),
        })
    }

    /// 用固定种子的 LCG 生成 [-scale, scale] 均匀分布张量。
    ///
    /// 刻意不用外部 `rand`：一是少一个依赖，二是固定种子能保证每次运行结果一致
    /// （candle 没有 PyTorch 的 `manual_seed`，随机初始化复现需要自己控制）。
    fn uniform(shape: impl Into<Vec<usize>>, scale: f32, seed: u64) -> Self {
        let shape = shape.into();
        let n = prod(&shape);
        let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let mut data = Vec::with_capacity(n);
        for _ in 0..n {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            // 取高 24 位当作 [0,1) 的均匀数
            let u = ((state >> 40) as f32) / (1u32 << 24) as f32;
            data.push((u * 2.0 - 1.0) * scale);
        }
        Tensor { shape, data }
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn rank(&self) -> usize {
        self.shape.len()
    }

    fn numel(&self) -> usize {
        self.data.len()
    }

    fn scalar(&self) -> Result<f32> {
        match self.numel() {
            1 => Ok(self.data[0]),
            n => Err(TensorError::LengthMismatch {
                expected: 1,
                got: n,
            }),
        }
    }

    /// 二维矩阵乘：`(m, k) × (k, n) -> (m, n)`。对应 `Tensor::matmul`。
    fn matmul(&self, rhs: &Tensor) -> Result<Tensor> {
        if self.rank() != 2 || rhs.rank() != 2 {
            return Err(TensorError::RankError {
                op: "matmul",
                expected: 2,
                got: self.rank().min(rhs.rank()),
            });
        }
        let (m, k) = (self.shape[0], self.shape[1]);
        let (k2, n) = (rhs.shape[0], rhs.shape[1]);
        if k != k2 {
            return Err(TensorError::ShapeMismatch {
                op: "matmul",
                lhs: self.shape.clone(),
                rhs: rhs.shape.clone(),
            });
        }
        // 三重循环 + 按行推进：让内层循环连续访问内存（cache 友好），
        // 这就是"手写 matmul 为什么要调循环顺序"的直观演示。
        let mut out = vec![0.0f32; m * n];
        for i in 0..m {
            for p in 0..k {
                let a = self.data[i * k + p];
                if a == 0.0 {
                    continue;
                }
                let row = &rhs.data[p * n..p * n + n];
                let out_row = &mut out[i * n..i * n + n];
                for j in 0..n {
                    out_row[j] += a * row[j];
                }
            }
        }
        Tensor::new(&out, vec![m, n])
    }

    /// 一维 × 二维：`(k,) × (k, n) -> (n,)`。
    ///
    /// 这是 MLP 每层实际发生的事（一个样本、一个特征向量）。candle / PyTorch 里
    /// 直接用 `x.matmul(w)` 也能走通（它们会自动把 rank-1 当行向量），
    /// 这里显式拆出来，是为了让"长度 k 必须对上"这件事看得见。
    fn matvec(&self, w: &Tensor) -> Result<Tensor> {
        if self.rank() != 1 || w.rank() != 2 {
            return Err(TensorError::RankError {
                op: "matvec",
                expected: 2,
                got: w.rank(),
            });
        }
        let k = self.shape[0];
        let (k2, n) = (w.shape[0], w.shape[1]);
        if k != k2 {
            return Err(TensorError::ShapeMismatch {
                op: "matvec",
                lhs: self.shape.clone(),
                rhs: w.shape.clone(),
            });
        }
        let mut out = vec![0.0f32; n];
        for p in 0..k {
            let a = self.data[p];
            if a == 0.0 {
                continue;
            }
            let row = &w.data[p * n..p * n + n];
            for j in 0..n {
                out[j] += a * row[j];
            }
        }
        Tensor::new(&out, vec![n])
    }

    /// 广播加法——对应 `Tensor::broadcast_add`。
    ///
    /// 只演示最常用的一条规则：**一维张量沿最后一维广播**：
    /// `(m, n) + (n,)` 合法；`(m, n) + (m,)` 在这里会报错（NumPy 里会静默按行广播，容易踩坑）。
    fn broadcast_add(&self, rhs: &Tensor) -> Result<Tensor> {
        // 标量 + 任何张量
        if rhs.numel() == 1 {
            let b = rhs.data[0];
            return Ok(Tensor {
                shape: self.shape.clone(),
                data: self.data.iter().map(|x| x + b).collect(),
            });
        }
        let last = *self.shape.last().unwrap_or(&0);
        let rhs_is_row = rhs.rank() == 1 && rhs.shape[0] == last;
        if !rhs_is_row {
            return Err(TensorError::ShapeMismatch {
                op: "broadcast_add",
                lhs: self.shape.clone(),
                rhs: rhs.shape.clone(),
            });
        }
        let data = self
            .data
            .iter()
            .enumerate()
            .map(|(i, x)| x + rhs.data[i % last])
            .collect();
        Ok(Tensor {
            shape: self.shape.clone(),
            data,
        })
    }

    /// 逐元素 ReLU（`h.relu()?`）
    fn relu(&self) -> Tensor {
        Tensor {
            shape: self.shape.clone(),
            data: self.data.iter().map(|x| x.max(0.0)).collect(),
        }
    }

    /// 沿最后一维做 softmax（`softmax(D::Minus1)`）。数值上先减最大值，避免 exp 溢出。
    fn softmax(&self) -> Result<Tensor> {
        let last = match self.shape.last() {
            Some(&d) if d > 0 => d,
            _ => {
                return Err(TensorError::RankError {
                    op: "softmax",
                    expected: 1,
                    got: self.rank(),
                });
            }
        };
        // 元素总数必须正好是 last 的整数倍（顺手挡住 last 为 0 / 空张量的情况）
        let rows = self.data.len() / last;
        if rows == 0 || rows * last != self.data.len() {
            return Err(TensorError::ShapeMismatch {
                op: "softmax",
                lhs: self.shape.clone(),
                rhs: vec![last],
            });
        }
        let mut out = vec![0.0f32; self.data.len()];
        for chunk in 0..rows {
            let row = &self.data[chunk * last..(chunk + 1) * last];
            let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let exps: Vec<f32> = row.iter().map(|x| (x - max).exp()).collect();
            let sum: f32 = exps.iter().sum();
            for j in 0..last {
                out[chunk * last + j] = exps[j] / sum;
            }
        }
        Ok(Tensor {
            shape: self.shape.clone(),
            data: out,
        })
    }

    /// 沿最后一维取最大值下标（`argmax(D::Minus1)`）——分类任务的输出。
    fn argmax(&self) -> Result<Tensor> {
        let last = match self.shape.last() {
            Some(&d) if d > 0 => d,
            _ => {
                return Err(TensorError::RankError {
                    op: "argmax",
                    expected: 1,
                    got: self.rank(),
                })
            }
        };
        let rows = self.data.len() / last;
        if rows == 0 || rows * last != self.data.len() {
            return Err(TensorError::ShapeMismatch {
                op: "argmax",
                lhs: self.shape.clone(),
                rhs: vec![last],
            });
        }
        let mut out = Vec::with_capacity(rows);
        for chunk in 0..rows {
            let row = &self.data[chunk * last..(chunk + 1) * last];
            let mut best = 0usize;
            for j in 1..last {
                if row[j] > row[best] {
                    best = j;
                }
            }
            out.push(best as f32);
        }
        if self.rank() == 1 {
            Tensor::new(&out, vec![1])
        } else {
            let mut shape = self.shape.clone();
            *shape.last_mut().unwrap() = 1;
            Tensor::new(&out, shape)
        }
    }

    /// 取第 `i` 行（一维张量则取第 `i` 个元素）——对应 `tensor.i(i)?`
    fn i(&self, index: usize) -> Result<Tensor> {
        let last = *self.shape.last().unwrap_or(&0);
        if last == 0 || index >= self.shape[0] {
            return Err(TensorError::IndexOutOfBounds {
                index,
                len: self.shape[0],
            });
        }
        // rank-1：取单个元素，返回 shape [1]（candle 里是 0 维标量）
        if self.rank() == 1 {
            return Tensor::new(&[self.data[index]], vec![1]);
        }
        let start = index * last;
        let mut shape = self.shape.clone();
        shape.remove(0);
        Tensor::new(&self.data[start..start + last], shape)
    }

    /// 求和（全部元素），用来"消费"结果、防止编译器优化掉 benchmark 循环
    fn sum_all(&self) -> f32 {
        self.data.iter().sum()
    }

    /// 全元素均值
    fn mean_all(&self) -> f32 {
        self.sum_all() / self.numel() as f32
    }

    /// 逐元素乘法（`&a * &b`）
    fn mul(&self, rhs: &Tensor) -> Result<Tensor> {
        if self.shape != rhs.shape {
            return Err(TensorError::ShapeMismatch {
                op: "mul",
                lhs: self.shape.clone(),
                rhs: rhs.shape.clone(),
            });
        }
        Ok(Tensor {
            shape: self.shape.clone(),
            data: self
                .data
                .iter()
                .zip(&rhs.data)
                .map(|(a, b)| a * b)
                .collect(),
        })
    }
}

impl fmt::Display for Tensor {
    /// 模仿 candle 的打印：`Tensor[2, 3] [...]` 风格，给出形状与数值。
    ///
    /// 用 `{:.4e}` 而不是 `{:.4}`：softmax 的输出经常跨好几个数量级，
    /// 定点格式会把 `0.0000` 和 `1.0000` 印在一起，看不出概率差异。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tensor[{}]",
            self.shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        write!(f, " [")?;
        for (i, v) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v:.4e}")?;
        }
        write!(f, "]")
    }
}

// ===========================================================================
// main
// ===========================================================================

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("device = Cpu（纯 std 教学张量；candle 版见 README）");

    demo_tensor_basics()?;
    demo_mlp_forward()?;
    demo_error_handling()?;
    demo_bench_matmul()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// ① 张量创建与基本运算
// ---------------------------------------------------------------------------

fn demo_tensor_basics() -> Result<()> {
    section("1. 张量创建与基本运算");

    let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], vec![2, 3])?;
    let b = Tensor::new(&[7f32, 8., 9., 10., 11., 12.], vec![3, 2])?;
    println!(
        "a.shape = {}，dtype = f32，numel = {}",
        shape_of(a.shape()),
        a.numel()
    );
    println!("a = {a}");

    // matmul：(m, k) × (k, n) -> (m, n)
    let prod = a.matmul(&b)?;
    println!("a.matmul(b).shape = {}", shape_of(prod.shape()));
    println!("a.matmul(b) = {prod}");

    // broadcast_add：偏置 (2,) 广播到 (2, 2)，不需要手动复制
    let bias = Tensor::new(&[3f32, -1.], vec![2])?;
    let shifted = prod.broadcast_add(&bias)?;
    println!("prod.broadcast_add(bias) = {shifted}");

    // softmax：沿最后一维归一化
    let probs = shifted.softmax()?;
    println!("softmax(dim=-1) = {probs}");
    let row_sums: Vec<String> = (0..2)
        .map(|r| format!("{:.6}", probs.i(r).unwrap().sum_all()))
        .collect();
    println!("每行和 = [{}]（softmax 后必然为 1）", row_sums.join(", "));

    // argmax：拿分类结果
    println!("argmax(dim=-1) = {}", probs.argmax()?);

    // 逐元素运算与归约
    println!("prod^2 的均值 = {:.4}", prod.mul(&prod)?.mean_all());

    Ok(())
}

// ---------------------------------------------------------------------------
// ② 手写 2 层 MLP 的前向传播（权重固定，零下载）
// ---------------------------------------------------------------------------

/// 极简 MLP：`x(4) -> Linear(4→8) -> ReLU -> Linear(8→3) -> softmax`。
///
/// 权重在真实项目里来自 safetensors / PyTorch state_dict；这里用固定种子生成，
/// 重点是**前向传播的拓扑**：`probs = softmax(W2 · relu(W1 · x + b1) + b2)`。
fn mlp_forward(x: &Tensor, w1: &Tensor, b1: &Tensor, w2: &Tensor, b2: &Tensor) -> Result<Tensor> {
    // (4,) × (4, 8) -> (8,)，再加 (8,) 的偏置
    let h = x.matvec(w1)?.broadcast_add(b1)?;
    let h = h.relu(); // 非线性：没有它，两层线性等价于一层
    let logits = h.matvec(w2)?.broadcast_add(b2)?;
    logits.softmax() // (3,) 概率分布
}

fn demo_mlp_forward() -> Result<()> {
    section("2. 手写 2 层 MLP 的前向传播");

    // 输入：1 个样本、4 个特征
    let x = Tensor::new(&[0.5f32, -1.2, 0.3, 2.0], vec![4])?;

    // 第一层 4 → 8，第二层 8 → 3（3 个类别）
    let w1 = Tensor::uniform(vec![4, 8], 0.5, 1);
    let b1 = Tensor::new(
        &[0.01f32, -0.02, 0.03, 0.0, 0.02, -0.01, 0.04, -0.03],
        vec![8],
    )?;
    let w2 = Tensor::uniform(vec![8, 3], 0.5, 2);
    let b2 = Tensor::new(&[0.0f32, 0.05, -0.05], vec![3])?;

    println!("x = {x}");
    println!(
        "W1.shape = {}，W2.shape = {}",
        shape_of(w1.shape()),
        shape_of(w2.shape())
    );

    let probs = mlp_forward(&x, &w1, &b1, &w2, &b2)?;
    println!("softmax(logits) = {probs}");

    let class = probs.argmax()?.scalar()? as usize;
    let confidence = probs.i(class)?.scalar()?;
    println!("预测类别 = {class}，置信度 = {confidence:.4}");
    println!("（权重用的是固定种子，所以每次运行结果一致；换 seed 就会变，见练习 21-1）");

    Ok(())
}

// ---------------------------------------------------------------------------
// ③ Result 错误处理：形状不匹配只在运行时暴露
// ---------------------------------------------------------------------------

fn demo_error_handling() -> Result<()> {
    section("3. Result 与形状错误（运行时才报）");

    let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], vec![2, 3])?;
    let wrong = Tensor::new(&[1f32, 2., 3., 4.], vec![2, 2])?;

    // (2,3) × (2,2)：内维 3 ≠ 2。编译器完全帮不上忙——形状是运行时值。
    match a.matmul(&wrong) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("matmul 形状不匹配 -> {e}"),
    }

    // 广播也会失败：给 (2,3) 加一个 (5,) 的偏置
    match a.broadcast_add(&Tensor::new(&[0f32; 5], vec![5])?) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("broadcast_add 形状不匹配 -> {e}"),
    }

    // 构造期就能发现的错误：数据个数和形状对不上
    match Tensor::new(&[1f32, 2., 3.], vec![2, 2]) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("Tensor::new 数据/形状不符 -> {e}"),
    }

    // `?` 可以把错误直接往上传（TensorError 实现了 std::error::Error）
    let ok: Result<f32> = (|| {
        let x = Tensor::new(&[1f32, 0., 0.], vec![3])?;
        x.softmax()?.i(0)?.scalar()
    })();
    println!("用 ? 传播错误并取值成功 -> {ok:?}");

    // 下标越界
    match a.i(5) {
        Ok(_) => println!("不该走到这里"),
        Err(e) => println!("a.i(5) -> {e}"),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// ④ 简单 benchmark：N 次 matmul 的平均耗时
// ---------------------------------------------------------------------------

fn demo_bench_matmul() -> Result<()> {
    section("4. matmul benchmark（Instant 计时）");

    let n = 64usize;
    let iters = 200;

    let a = Tensor::uniform(vec![n, n], 1.0, 7);
    let b = Tensor::uniform(vec![n, n], 1.0, 11);

    // 预热一次：不把首次分配的时间算进去
    let _ = a.matmul(&b)?;

    let start = Instant::now();
    let mut acc = 0f32;
    for _ in 0..iters {
        let c = a.matmul(&b)?;
        // 必须"消费"结果，否则优化器可能把整个循环删掉
        acc += c.sum_all();
    }
    let elapsed = start.elapsed();

    let total_macs = iters as f64 * (n * n * n) as f64;
    println!("矩阵规模 = {n}×{n}，迭代 = {iters} 次（单线程）");
    println!(
        "总耗时 = {:.2?}，平均 = {:.3?}/次，每个 MAC 约 {:.2} ns",
        elapsed,
        elapsed / iters as u32,
        elapsed.as_secs_f64() * 1e9 / total_macs
    );
    println!("校验和 = {acc:.1}（防止编译器优化掉循环）");
    println!("提示：这只看「相对趋势」，不是严谨 benchmark——正式测量请用 criterion，");
    println!("      并和 BLAS/mimalloc 等真实后端对比（candle 的 CPU 后端就走 gemm）。");

    Ok(())
}

// ---------------------------------------------------------------------------

fn section(title: &str) {
    println!("\n=== {title} ===");
}
