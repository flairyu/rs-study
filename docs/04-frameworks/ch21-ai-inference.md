# 第 21 章 · AI / 推理：candle 与 ort

> **一句话定位**：Rust 不是训练大模型的地方；它擅长的是**把训练好的模型高效、稳定、低依赖地跑在服务端或边缘设备上**。本章讲清 candle / ort / burn / tch 各自的位置，并手写一遍不下载任何权重的推理。
> **预计时间**：120 分钟（读 60 + 跑 demo 20 + 练习 40）
> **前置**：第 1 章（基础）、第 7 章（集合）、第 17 章（常用第三方库）
> **配套 demo**：`projects/ai-infer`（独立 crate，`cargo run` 即可，不需要模型文件、不需要 GPU、**不需要网络**）

> ⚠️ **先说清楚本环境的实测边界**：配套 demo 最终交付的是**纯 `std` 教学张量版**，不是 candle 版。
> 原因是本机带宽太窄（`static.crates.io` 实测约 19 KB/s），`cargo fetch` 连 candle 的依赖
> **解析**都跑不完：`failed to transfer more than 10 bytes in 30s :: download of ge/mm/gemm failed`。
> 全文凡是真的跑过的代码，都会标 ✅；需要网络/模型文件/GPU 才能跑的，都用 ⚠️ 引用块显式标出。

---

## 0. 先建立锚点（给 Python 使用者）

如果你用过 Python 的 AI 栈，先做这张对照。**右边的每一格都比左边小一号**，这是本章最重要的信息：

| 你在 Python 里用的 | Rust 里的对应物 | 成熟度差距 | 典型用途 |
|---|---|---|---|
| PyTorch（训练 + 推理） | `burn` / `candle` | 差 1–2 个数量级（算子覆盖、分布式、调试工具） | 小模型训练、科研原型、可插拔后端 |
| PyTorch（只做推理） | `tch`（libtorch 绑定） | 能力接近，但拖着 C++ 动态库 | 需要完整算子覆盖的推理 |
| ONNX Runtime | `ort`（同源 C API 的 Rust 绑定） | 很接近，API 还在 `2.0.0-rc` | **Python 训练 → Rust 部署**的主流路线 |
| HuggingFace Transformers | `candle-transformers` / `tokenizers` | 模型数量少很多，但覆盖主流 LLM | LLM 推理、CLIP、Whisper… |
| safetensors | `safetensors`（就是 HF 写的 Rust 库） | **基本持平**，甚至是上游 | 权重文件格式 |
| NumPy | `ndarray` / `candle-core` 的张量 | 接近 | 数值计算 |

```python
# Python：三行完成"加载模型 + 推理"
import torch
model = torch.jit.load("mnist.pt").eval()
print(model(torch.randn(1, 1, 28, 28)).argmax(dim=1))
```

```rust
// Rust：模型不是"代码"，而是一份需要你自己喂进去的数据。
// 下面这段是示意，不是本 demo 里能直接跑的东西（见本章「实测 vs 需要额外条件」的标注）。
let mut model = candle_nn::VarBuilder::from_mmaped_safetensors(&["model.safetensors"], DType::F32, &Device::Cpu)?;
let input = Tensor::randn(0f32, 1f32, (1, 1, 28, 28), &Device::Cpu)?;
let logits = model.forward(&input)?;      // 网络结构得你在 Rust 里重新写一遍
println!("{}", logits.argmax(D::Minus1)?);
```

**为什么 Rust 在 AI 里突然有人用了？** 因为训练和推理是两种完全不同的负载。训练要的是"能快速试错的框架"，Python 赢了；推理要的是**吞吐、延迟、内存占用、部署体积、并发安全**，这些恰好是 Rust 的主场：

- **冷启动与体积**：Python + torch 的镜像动辄 2–5 GB，一个 Rust 推理二进制可以做到几十 MB，容器冷启动从秒级降到毫秒级。
- **性能可预测**：没有 GIL、没有 GC；延迟尾部（p99）比"Python 调 C++ 再回调 Python"稳得多。
- **和已有服务同进程**：你的网关、鉴权、限流本来就是 Rust 写的，推理不必再跨一次 HTTP 到 Python 服务。
- **嵌入式/边缘**：`no_std`、交叉编译到 aarch64/wasm 都很成熟，Python 在这类设备上很痛苦。

**同时也必须诚实**：

- Rust 的 AI 生态**远不如 Python**。找不到算子就得自己实现或 FFI 回 C++；没有 `pdb`、没有 `torchviz`、没有一行 `model.summary()`。
- **几乎没人用 Rust 从头训练大模型**。社区共识是 **Python 训练 → 导出 → Rust 推理**。
- 版本迭代极快，crate 之间的版本兼容是真实的工程成本（见「常见坑」）。

---

## 1. `candle`：HuggingFace 的纯 Rust 张量库

`candle` 由 HuggingFace 在 2023 年推出，目标是"PyTorch 的极简 Rust 版"。分两层：

| crate | 作用 |
|---|---|
| `candle-core` | `Tensor`、`Device`、autograd、算子（本章 demo 只依赖它） |
| `candle-nn` | `Linear`、`Conv2d`、`LayerNorm`、`VarBuilder`… 搭网络的积木 |
| `candle-transformers` | 现成模型：Llama、Whisper、CLIP、SAM、Stable Diffusion… |

**关键特性**：

- **纯 Rust**，没有 C++/CUDA 运行时依赖（CUDA 后端也走自己的 kernel）；`cargo build` 出来的东西可以直接扔进 `scratch` 镜像。
- 后端可切：CPU / CUDA / Metal，通过 feature 开关，代码里只出现 `Device`。
- **autograd 是真的**：`Tensor::backward()` 能跑反向传播，所以 `candle` 也能训练（小模型）。
- **算子是"够用"而非"齐全"**：从 PyTorch 转过来会发现某些函数不存在，需要自己用基本算子拼。

```rust
// ⚠️ 需要 candle-core（联网拉取 + 编译）。API 名称与语义真实，但**本环境未跑通编译**；
//    可运行的等价版本见 projects/ai-infer/src/main.rs（纯 std 教学 Tensor，命名对齐 candle）。
use candle_core::{DType, Device, Tensor, D};

let dev = Device::Cpu;                                       // 换 Device::new_cuda(0)? 需要 cuda feature
let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], &dev)?.reshape((2, 3))?;
let b = Tensor::new(&[7f32, 8., 9., 10., 11., 12.], &dev)?.reshape((3, 2))?;
let c = a.matmul(&b)?;                                       // (2,3)x(3,2) -> (2,2)
let d = c.broadcast_add(&Tensor::new(&[3f32, -1.], &dev)?)?;      // (2,) 广播到 (2,2)
println!("{}", d.softmax(D::Minus1)?);                        // 沿最后一维归一化
```

> ⚠️ **未实测**：上面这段依赖 `candle-core` 能成功下载并编译。本环境 `cargo fetch` 直接失败
> （`download of ge/mm/gemm failed`），所以只有**算子的名字和调用形状**是可信的，
> 具体版本行为请在能联网的机器上验证。`candle-core` 目前在 crates.io 上的最新版本是 `0.11.0`。

**什么时候选它**：想要 No-C++ 的单文件部署、想读一份可审计的推理代码、想跑 LLM 但不想背 PyTorch 的镜像体积。

**什么时候别选它**：算子覆盖要求高（大量自定义 op）、要分布式训练、团队只会 PyTorch。

---

## 2. `ort`：ONNX Runtime 的 Rust 绑定

`ort` 是微软 ONNX Runtime C API 的 Rust 封装。ONNX 是**框架中立的模型交换格式**：PyTorch 的 `torch.onnx.export`、TensorFlow、PaddlePaddle 都能导出 `.onnx`。

```rust
// ⚠️ 需要额外条件：一个 .onnx 模型文件 + onnxruntime 动态库。本环境未实测，仅示意。
use ort::session::Session;

let mut session = Session::builder()?.commit_from_file("mnist.onnx")?;
let input = ort::value::Tensor::from_array(([1usize, 1, 28, 28], vec![0f32; 784]))?;
let outputs = session.run(ort::inputs!["input" => input])?;
let (shape, probs) = outputs["output"].try_extract_tensor::<f32>()?;
println!("{shape:?} {probs:?}");
```

**优点**：图优化（算子融合、常量折叠）、量化（INT8）、线程池、CUDA/TensorRT/OpenVINO/DirectML 多后端，全都由 ONNX Runtime 负责——**你只写 Rust**。

**代价**：不再是纯 Rust。要么在目标机装 `libonnxruntime.so`，要么用 `download-binaries` feature 让构建脚本下载预编译库（这一步**需要网络**，是 CI 里最常见的失败点）。

**什么时候选它**：`Python 训练 → Rust 部署` 这条路上最省事的一步；模型图已经稳定、不想用 Rust 重写网络结构。`ort` 的 API 目前仍是 `2.0.0-rc`（2.0 尚未正式发布），锁版本时注意 rc 之间偶有 breaking change。

### 2.1 上游那一步：从 PyTorch 导出 ONNX

`ort` 的一半工作量在 Python 侧。**导出时最容易埋雷的是「动态轴」**：

```python
# ⚠️ 需要 PyTorch + 一个训练好的模型。这是 Python 侧的代码，不是 Rust。
import torch

model = MyNet().eval()
dummy = torch.randn(1, 3, 224, 224)              # 形状必须和 Rust 侧一致
torch.onnx.export(
    model, dummy, "model.onnx",
    input_names=["input"], output_names=["logits"],
    dynamic_axes={"input": {0: "batch"}, "logits": {0: "batch"}},   # 关键：把 batch 标成动态
    opset_version=17,
)
```

- **不写 `dynamic_axes`，batch 就被写死成 1**。Rust 侧送 8 条数据进去会直接报 shape 错误——这是最高频的"导出的模型在 Python 里好好的，到 Rust 就崩"。
- **`opset_version` 要够新**：太老的 opset 会让 `LayerNorm`、`Gelu` 这类算子被拆成一堆原始算子，精度和性能都受影响；太新则可能超出你装的 onnxruntime 支持范围。
- **导出后先用 Python 验一遍**：`onnxruntime.InferenceSession` 跑一遍、和 PyTorch 输出对一遍数值（`np.allclose`，容差 1e-4 量级）。这一步通过了，问题就一定在 Rust 侧，能省掉大量来回。

---

## 3. `burn`：纯 Rust 的深度学习框架（训练也能做）

`burn` 的野心比 `candle` 大：**训练 + 推理一整套**，且"后端可插拔"是核心设计——同一份模型代码可以跑在 `NdArray`（纯 CPU）、`wgpu`（跨 GPU，含 WebGPU/浏览器）、`CUDA`、`LibTorch` 上。

```rust
// ⚠️ 需要额外条件：拉取 burn 及其后端。本环境未实测，仅示意 API 形状。
use burn::tensor::{backend::NdArray, Tensor, Distribution};

type B = NdArray;
let w: Tensor<B, 2> = Tensor::random([8, 4], Distribution::Default, &device);
let y = x.matmul(w);                                  // 张量类型带后端参数
```

**它的取舍**：`Tensor<B, D>` 这种带后端/维度参数的泛型让代码更"重"，学习曲线比 candle 陡；换来的是**同一套代码在 CPU/GPU/WASM 之间无痛切换**，以及 `burn Train` 那套训练循环（metrics、checkpoint、超参搜索）。

**什么时候选它**：你想在 Rust 里**从零训练**一个小模型；或者目标平台是 WebGPU / WASM。

---

## 4. `tch`：libtorch 绑定（能力最全，依赖最重）

`tch` 是 PyTorch C++ 前端 libtorch 的绑定。**它几乎就是 PyTorch**：算子覆盖、`torchscript` 加载、GPU，全都到位。

```rust
// ⚠️ 需要额外条件：本机有 libtorch（或用 download-libtorch feature 联网下载 ~2GB）。本环境未实测，仅示意。
use tch::{nn, Device, Tensor};

let vs = nn::VarStore::new(Device::Cpu);
let model = nn::seq().add(nn::linear(vs.root()/"fc1", 784, 128, Default::default()));
let x = Tensor::randn([1, 784], (tch::Kind::Float, Device::Cpu));
println!("{:?}", model.forward(&x).argmax(1, false));
```

**代价**：构建要能找到 libtorch（`LIBTORCH` 环境变量或 `download-libtorch`），第一次编译可能几十分钟、产物几百 MB；GPU 还要版本匹配的 CUDA。**"纯 Rust 部署"的优势基本没了**，换来的是最强的算子覆盖。

**什么时候选它**：你的模型用了大量自定义算子，ONNX 导出会掉精度或失败；且能接受部署镜像里带 libtorch。

---

## 5. 实际工程里的三条路线

这是本章最该记住的一张图。**不要问"Rust 能不能训练模型"，先问"我的模型从哪来"**：

| 路线 | 数据流 | 用到的 crate | 适合 |
|---|---|---|---|
| ① 最常见 | Python 训练 → `torch.onnx.export` → **`.onnx`** → Rust 服务 | `ort` | 模型结构稳定、要 ONNX 的图优化与量化 |
| ② HuggingFace 系 | Python 训练 → **`.safetensors`** → Rust 里重写网络结构 + 加载权重 | `candle-core` + `candle-nn` + `safetensors` | LLM、想要纯 Rust 二进制 |
| ③ 全 Rust | Rust 训练 → Rust 推理（同一份代码） | `burn` / `candle` + autograd | 小模型、边缘设备、WebGPU/WASM |
| （外挂） | Python 服务化，Rust 只做网关 | `reqwest` 调 vLLM/Ollama/Triton | 不想碰模型代码，只要吞吐 |

### 一条 LLM / RAG 服务链路的组件图

```
HTTP 请求 (axum / actix-web)
   │
   ├─▶ 1. 鉴权 / 限流 / 路由        → axum middleware, tower
   │
   ├─▶ 2. Embedding（可选，RAG）    → ort(ONNX 小模型) 或 调远端 API
   │        └─▶ 向量库检索          → qdrant-client / lancedb / pgvector(sqlx)
   │
   ├─▶ 3. Tokenizer                → tokenizers（HF 官方 Rust 实现，纯 Rust、极快）
   │
   ├─▶ 4. 模型前向                  → candle-transformers / ort / tch / 远端 vLLM
   │        └─▶ 权重                 → safetensors / gguf（量化）
   │
   ├─▶ 5. Sampling（top-k/top-p/温度）→ candle 的 logits processor，几十行自己写
   │
   └─▶ 6. 流式返回 (SSE)            → axum + tokio_stream
```

**每一步的 Rust 生态现状**：

| 环节 | 有什么 | 说明 |
|---|---|---|
| HTTP / 并发 | `axum`、`tokio` | **Rust 的强项**，比 Python 侧稳得多 |
| Tokenizer | `tokenizers` | HF 出品，**Rust 是上游实现**，Python 只是绑定 |
| 权重格式 | `safetensors`、`gguf` | safetensors 本来就是 Rust 写的 |
| 模型执行 | `candle`、`ort`、`tch`、`burn` | 差距主要在这里 |
| 向量检索 | `qdrant-client`、`lancedb`、`pgvector` | 都有官方/活跃 Rust 客户端 |
| 采样 / 调度 | 无标准库，自己写 | 逻辑简单，几十行 |
| 可观测性 | `tracing`、`metrics` | 和第 17 章的实验室同一套 |

**一句话**：从"请求进来"到"token 出去"的**管道**，Rust 生态已经很完整；**模型本身**这一格，才是要仔细权衡的地方。

> ⚠️ **本环境的一个现实约束**：本章 demo **刻意不加载任何预训练权重**。真实项目里权重文件通常 100 MB–几十 GB，从 HuggingFace 下载既慢又容易撞上网络/鉴权问题。推理代码的正确性不依赖权重从哪来——**先把张量和前向传播跑通，再换成真权重**。

---

## 6. 与 demo 的对照

`projects/ai-infer` 是一个**独立 crate**（不在 `exercises/` 的 workspace 里——真引入 candle 的话会拉几十个依赖进来）。它有两个入口：

| 文件 | 状态 | 说明 |
|---|---|---|
| `src/main.rs` | ✅ **实测 `cargo run` 跑通** | 纯 `std` 教学 `Tensor`（约 630 行，含注释），零依赖、离线、秒级编译 |
| `src/main_candle.rs` | ⚠️ **未实测**（需要网络拉 candle） | 同一份 demo 的 candle 版，逐行对应；API 按真实 candle 写，启用步骤见 README |

两个入口的四个 demo **逐个对应**：

| 函数 | 演示什么 | std 版（✅ 实测） | candle 版（⚠️ 需网络） |
|---|---|---|---|
| `demo_tensor_basics` | 张量创建 + 基本运算 | `Tensor::new(&[..], vec![2,3])` | `Tensor::new(&[..], &dev)?.reshape((2,3))?` |
| `demo_mlp_forward` | 手写 2 层 MLP 前向 | `matvec` + `broadcast_add` + `relu` + `softmax` + `argmax` | 同左（candle 里 `x.matmul(&w)` 直接支持 rank-1） |
| `demo_error_handling` | 形状/类型错误是**运行时**的 | 匹配 `TensorError`、`?` 传播 | 匹配 `candle_core::Error`、`?` 传播 |
| `demo_bench_matmul` | N 次 matmul 平均耗时 | `std::time::Instant`、结果必须被"消费" | 同左（但 candle 底层走多线程 `gemm`） |

**为什么 std 版能替代教学职责**：std 版的 API 命名是**刻意对齐** `candle_core::Tensor` 的
（`new` / `matmul` / `broadcast_add` / `relu` / `softmax` / `argmax` / `i` / `sum_all` / `mean_all`），
所以这里学到的调用顺序，换成 candle 之后**结构一行都不用改**。

### 6.1 前向传播的拓扑

```
x(4)  ──matmul(W1: 4x8)──▶ h(8) ──+b1(8)──▶ relu ──matmul(W2: 8x3)──▶ logits(3) ──+b2(3)──▶ softmax ──▶ argmax
```

代码就三行（✅ 实测，输出 `softmax(logits) = [0.3805, 0.3962, 0.2233]` → 类别 `1`）：

```rust
// 注意 std 版把「rank-1 向量 × 矩阵」单独写成 matvec；candle 里 x.matmul(w) 就够了
let h = x.matvec(w1)?.broadcast_add(b1)?;    // 广播：b1 是 (8,)，自动加到每个输出上
let h = h.relu();                            // 非线性，缺了就退化成单层线性模型
let probs = h.matvec(w2)?.broadcast_add(b2)?.softmax()?;
```

**`broadcast_add` 是这里最像 NumPy 的地方**：`(8,)` 的偏置加到 `(8,)` 的激活上毫无悬念，但真实代码里是 `(batch, 8)` + `(8,)`，靠广播规则自动对齐。顺带一个容易忽略的细节：**广播的语义在不同库里并不一样**——std 版刻意只实现了「沿最后一维」这一条规则，所以 `(2,3) + (2,)` 直接报错；而 NumPy 会静默按行广播。**这就是"看起来对"的代码算错数的经典来源。**

**权重为什么是写死的**：demo 用 `fixed_uniform()`（固定种子的 LCG）生成矩阵，保证每次运行结果一致，也不引入 `rand` 依赖。原因是 **candle 没有 PyTorch 的 `manual_seed`**，`Tensor::rand` 每次运行都不同——想要可复现，就得像 demo 这样自己握种子。真实项目里这一步是 `VarBuilder::from_mmaped_safetensors([...])`，之后的所有算子代码**一模一样**——这正是本章想让你看到的：**推理代码的形状是稳定的，变的是权重从哪来。**

### 6.2 为什么错误处理值得单独一节

```rust
let a = Tensor::new(&[1f32, 2., 3., 4., 5., 6.], vec![2, 3])?;   // 2x3
let wrong = Tensor::new(&[1f32, 2., 3., 4.], vec![2, 2])?;       // 2x2
let _ = a.matmul(&wrong);
// -> matmul: shapes [2, 3] and [2, 2] cannot be combined
```

类型系统能保证"这是 `Tensor`"，却**保证不了"它的形状是 `(2,3)`"**。在 Rust 里这有点反直觉（我们习惯了编译器兜底），但张量形状是运行时值，和 Python 一样得靠测试和断言兜住。

值得留意的是这些错误**都是 `Err` 而不是 panic**：candle 和本 demo 一样走 `Result`，所以 `?` 能把"某个权重文件里第 3 层形状不对"一路带回 `main`，而不是在第 30 层算子处炸掉。这比 NumPy 的广播静默出错、或者 PyTorch 的 `RuntimeError` 打断整个进程要好排查。

> 💡 **工程建议**：给模型输入加一道 `ensure!` 式的形状检查（`input.dims() != [1, 3, 224, 224]` 就早失败），比让错误传播到第 30 层算子才炸容易定位。

---

## 7. 本章练习

| 题号 | 任务 | 考点 | 验收标准 |
|---|---|---|---|
| 21-1 | 把 MLP 的权重从固定值改成随机初始化，观察预测类别变化 | `Tensor::rand` / `randn`、`Device`、可复现性 | 连续跑两次结果不同；再引入固定种子后两次一致；能解释为什么 |
| 21-2 | 用 `ort` 跑一个从 PyTorch 导出的 MNIST 模型 | ONNX 导出、`Session`、输入张量构造 | `cargo run` 能对一张手写数字图输出 0–9 的预测与置信度 |
| 21-3 | 把 demo 包成 `GET /predict` 的 axum 服务 | 第 18 章 + 本章的整合 | `curl 'localhost:3000/predict?x=0.5,-1.2,0.3,2.0'` 返回 JSON `{"class":2,"probs":[...]}` |

### 提示（不给答案）

- **21-1**：`Tensor::rand(0f32, 1f32, shape, &dev)?` 返回 `Result`（要 `rand` 特性，candle-core 默认开启）。想要可复现就用 `Tensor::rand` 前先设自己的 LCG，或干脆学 demo 里的 `fixed_uniform` 换 seed——注意 `rand` 本身不吃 seed，candle 没有 PyTorch 的 `manual_seed`。
- **21-2**：Python 侧 `torch.onnx.export(model, dummy, "mnist.onnx", input_names=["input"], output_names=["output"])`；Rust 侧 shape 必须是 `[N, C, H, W]`，别忘了 `try_extract_tensor` 拿到的是一维切片 + shape，得自己 `argmax`。**这一题需要网络（下载 onnxruntime）+ 模型文件**，跑不通不是你的错，记录下卡在哪一步同样算完成。
- **21-3**：`axum::extract::Query<HashMap<String, String>>`，把逗号分隔的数字 `parse::<f32>()` 成 `Vec<f32>`，再 `Tensor::new(&v[..], &Device::Cpu)?`。错误路径用 `IntoResponse` 映射成 400，别 `unwrap()`。

### 三题的难度和依赖，差别很大

| 题号 | 需要什么 | 预计耗时 | 卡住的常见原因 |
|---|---|---|---|
| 21-1 | ✅ 无额外条件（就用本 demo） | 15 分钟 | 忘了 `rand` 拿不到可复现结果 |
| 21-3 | ✅ 只需要 `axum` + `tokio`（能联网拉 crate） | 40 分钟 | 把 `Query` 解析错误当成了 500 |
| 21-2 | ⚠️ **需要网络 + PyTorch + onnxruntime + 模型文件** | 1–2 小时 | `dynamic_axes` 没写、opset 不匹配、找不到动态库 |

**21-2 做不出来很正常。** 它的价值在于让你亲手撞一次"Python 训练 → Rust 部署"的每一道墙：导出、动态轴、动态库、形状对齐。如果卡住了，请记录**卡在第几步、报什么错**——这份记录本身就是本章最有价值的产出，比一个跑通的 demo 更能说明 Rust AI 部署的真实成本。

---

## 8. 常见坑（至少这 5 条都真实存在）

1. **candle 与 ort 的版本/feature 地狱**。`candle-*` 系列 crate **必须版本对齐**（`candle-core = "0.9"` 配 `candle-nn = "0.9"`），混用会在编译期报一堆 trait 不匹配。`ort` 目前只有 `2.0.0-rc.*`（无 stable），rc 之间改 API 是常事——**生产项目请把 `Cargo.lock` 提交进版本库**，并给 `ort` 写精确版本 `= "2.0.0-rc.13"`。
2. **CUDA feature 让构建时间和体积爆炸**。加 `--features cuda` 会编译成千上万个 kernel（几十分钟，`target/` 上 GB），而且**必须目标机有匹配的 CUDA toolkit**；`ort` 的 `download-binaries` 默认下 CPU 版，切 GPU 要换 feature 加 `CUDA_PATH`。建议**默认纯 CPU 开发，只在最终产物的构建里开 GPU**。
3. **张量形状错误只在运行时暴露**（见 6.2）。类型系统拦不住 `(2,3) x (2,2)`；加上广播规则，"看起来对"的代码可能静默算出错误结果。**给每个网络层入口写形状断言**，比事后 debug 便宜。
4. **tokenizer 与模型必须成套**。换模型时只换了权重、没换 tokenizer（或 vocab 大小不一致），表现为输出乱码或直接 index out of range。`tokenizers` 一般从 HuggingFace 仓库拿 `tokenizer.json`，它和权重是同一个 commit 的一部分——**把两者当一份产物管理**。
5. **精度与量化**。`f16`/`bf16` 在 CPU 上可能比 `f32` **更慢**（缺硬件加速，运行时反复转换）；INT8/GGUF 量化能省几倍内存，但不同量化方案（Q4_0 / Q4_K_M…）精度差异很大，且**量化后的模型往往不再和原 tokenizer 完全对齐**。上线前一定要有一套自己的评测集对比，而不是只测"能不能跑"。
6. （附带）**不要用 `unwrap()` 读模型文件**。权重路径、形状、dtype 全来自外部文件，任何一处不对都会 panic；用 `?` + `anyhow::Context` 把"哪个文件、期望什么形状"带进错误信息。

---

## 9. 自测清单

- [ ] 我能说出"Rust 在 AI 里不是训练主力"的三个具体理由，以及 Rust 赢在哪三个场景
- [ ] 我能默画三条路线：ONNX→`ort`、safetensors→`candle`、全 Rust→`burn`
- [ ] 我知道 `candle` / `ort` / `burn` / `tch` 各自最大的**取舍**（纯 Rust vs 算子覆盖 vs 可插拔后端 vs 拖着 C++）
- [ ] 我理解 `broadcast_add` 的广播规则，并能解释为什么 `matmul` 的形状错误编译期发现不了
- [ ] 我能说出 LLM 服务链路六个环节里，哪些 Rust 生态已经很强、哪些还得自己写
- [ ] 我跑通了 `projects/ai-infer` 的 `cargo run`，并知道 demo 里哪些 API 是真的、哪些片段需要额外条件
- [ ] 我做完了 3 道练习（21-2 若因网络受限未完成，我能说清卡在哪一步）

---

## 附：本环境实测与未实测的边界

这张表是本章的"可信度地图"。**凡是标 ✅ 的，都是我在本机真的跑过的；标 ⚠️ 的一律当作"伪代码 + API 形状参考"来读。**

| 代码 | 状态 | 缺什么条件 |
|---|---|---|
| `projects/ai-infer/src/main.rs`（纯 std 教学张量，**默认入口**） | ✅ **本环境实测 `cargo run` 跑通**，逐字输出见 `README.md` | 无 |
| `projects/ai-infer/src/main_candle.rs`（candle 版，**未接线**） | ⚠️ **未实测**：API 按真实 candle 写，但没编译过 | 能下完 candle 依赖的网络（本机 `cargo fetch` 在 `gemm` 处超时） |
| 第 1 节 candle 片段 | ⚠️ 同上（算子名与调用形状可信，版本行为未验证） | 同上 |
| 第 2 节 `ort` 片段 + 2.1 的 PyTorch 导出 | ⚠️ **未实测** | `.onnx` 文件 + onnxruntime 动态库（`download-binaries` 需联网）+ Python/PyTorch |
| 第 3 节 `burn` 片段 | ⚠️ **未实测**：仅示意张量类型形状 | 拉取 `burn` + 一个后端（如 `burn-ndarray`） |
| 第 4 节 `tch` 片段 | ⚠️ **未实测** | libtorch（`LIBTORCH` 环境变量，或联网下载约 2 GB） |

### 怎么自己复现 ✅ 那一行

```bash
cd /home/yxb/code/rs-study/projects/ai-infer
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 只读 ~/.cargo 的受限环境才需要
cargo run 2>&1 | tail -40
```

预期：0 依赖、约 0.3 秒编译完，四个 demo 依次打印；第 2 节给出 `预测类别 = 1，置信度 = 0.3962`，
第 4 节给出 64×64 matmul × 200 次的平均耗时（本机实测 `434.67ms` 总计 / `2.173ms` 每次，
跑多次会在 434–460ms 之间波动——**只有这一节的数字会变，其余完全确定**）。

> ⚠️ **最后重申**：文档里带 ⚠️ 的代码块都属于"需要额外条件"，请在有网络/模型/GPU 的机器上**另建项目**验证，
> 不要直接改本 demo 去试——先把本 demo 的 std 版跑通，你对"推理的骨架"就有了不会被版本问题干扰的理解。
