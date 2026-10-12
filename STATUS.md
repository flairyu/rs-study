# 验证报告 · 这套教程里哪些东西是"实测过的"

> 这不是教程正文，而是一份**可信度说明**。
> 一套教程最容易骗人的地方是：代码片段看起来对，但从来没编译过。
> 这份文件逐项列出「哪些跑过、怎么跑的、哪些只是纸面讲解」。
>
> 验证环境：rustc / cargo **1.98.1**（LLVM 22，x86_64-unknown-linux-gnu），edition 2021。

---

## 1. 一句话结论

| 部分 | 状态 |
|---|---|
| 第 1–16 章练习（`exercises/`，48 道题 / 16 个 crate） | ✅ **全部编译通过**，且**每道题都有已验证的参考答案** |
| 第 1–16 章文档 | ✅ 每章恰好 3 个练习 + 3 个 `#[test]`，题目与测试自洽 |
| 第 17 章 + `labs/`（8 个实验） | ✅ 8 个 crate 全部 `cargo run` 跑通，输出已复核 |
| 第 18 章 axum demo | ✅ 编译 + 5 个真集成测试 + 真起服务 curl 打通 |
| 第 19 章 Bevy demo | ✅ 编译链接成功（本机实测，见 ch19 文档「本机的实测结果」） |
| 第 20 章 sqlx demo | ✅ `cargo run` 跑通（含事务提交与回滚演示） |
| 第 21 章 AI demo | ⚠️ **纯 std 版实测跑通**；candle 版未编译（依赖拉不下来），文件中已明确标注 |
| 第 18–21 章文档里的框架 API 用法 | ✅ 与实测代码一致；未实测的片段都带显式警示 |

---

## 2. 练习系统：怎么证明"题目本身没写错"

只让练习编译通过是不够的——**测试期望值本身可能是错的**，那样学习者永远做不出来。
所以仓库里有一个 `solutions/` 工作区：

```
solutions/chXX_name/
├── Cargo.toml
├── src/lib.rs          ← 参考实现（从练习骨架 cp 起手，只替换 unimplemented!()）
└── tests/chapter.rs    ← 两行 shim：用 #[path] 指回 exercises/ 里的那一份测试
```

关键设计：**测试只有一份真相源**。参考答案与练习共用同一个 `tests/chapter.rs`，
所以答案和题目不可能漂移。

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py --solutions
```

**实测结果：16/16 全绿，48/48 测试通过。**

### 这套自检抓出来的 6 个真实 bug

这些如果只靠人眼审阅，**一定会漏**：

| # | 问题 | 位置 | 修法 |
|---|---|---|---|
| 1 | `longest_word` 规格是"等长取靠前"，但测试与文档期望写成了靠后的词 | ch04 测试 / 骨架 doctest / 文档 | 三处统一为 `"quick"` |
| 2 | 测试期望 `335`，但断言消息自己写着「5 + 300」，正确值是 **305** | ch08 `tests/chapter.rs` | 期望值改为 305 |
| 3 | 骨架的 `# 示例` 本身编译不过：`*m` 解引用 `Option<i32>` → E0614 | ch09 骨架 doctest | 去掉 `*` |
| 4 | `rest()` 的期望值与同一测试里 `pos == 7` 的断言互斥 | ch10 测试 + doctest | 按规格改为 `" beta"` |
| 5 | 文档参考答案 `*counter.lock().unwrap()` 作尾表达式 → **E0597** | ch12 文档 | 改为绑定局部变量，并把坑写进文档 |
| 6 | **ch15 的练习骨架被写成了完整答案**，学习者没有东西可填 | ch15 骨架 | 恢复为 3 个 `unimplemented!()` |

另外还修了 2 处 `unimplemented!("... {x} ...")` 里的 `{}` 被当成 format 占位符导致编译失败。

---

## 3. 练习工程的设计约束（以及为什么）

1. **每章只依赖 std**。所以 `python3 tools/check.py` 在**断网**环境也能跑通全部 16 章。
   需要第三方 crate 的内容全部隔离在 `labs/` 与 `projects/`。
2. **骨架永远能编译**。每个函数体是 `unimplemented!()`（类型 `!`，能转成任何返回类型），
   所以学习者看到的失败只可能是**断言失败**，不会是编译错误——归因清晰。
3. **每章恰好 3 个 `#[test]`**，与 3 道练习一一对应。
4. 实测：**未实现状态下全量跑一遍 = 16 章全部 `0/3 通过`，用时 35 秒**（编译缓存已热）。

---

## 4. 工具脚本的实测情况

| 脚本 | 实测内容 | 结果 |
|---|---|---|
| `tools/check.py` | 自动定位 cargo（本机 `~/.cargo/bin` 缺 shim，会回退到 `~/.rustup/toolchains/*/bin`）；分章报告；`--solutions` 自检 | ✅ 已跑通全量 + 单章 |
| `tools/check.py` 的输出解析 | Rust 1.98 的 libtest 换了输出格式，且 `cargo test --quiet` 会把 `--quiet` 透传给 libtest 进入"每用例一个点"模式 | ✅ 已适配两种格式 + `test result:` 汇总回退 |
| `tools/ai_review.py` | 跑测试 → 收集代码 → **剥离文档里的参考答案** → 生成 prompt（`--dry-run` 不需 API key） | ✅ 实测：`percent:.1`、`total_seconds / 3600` 等答案特征串均未泄漏 |

> `ai_review.py` 的真实 API 调用路径（HTTP 请求）**未实测**——我没有 API key。
> 它用的是标准库 `urllib` + OpenAI 兼容协议，失败时会把 prompt 落盘并给出明确提示。

---

## 5. `labs/` 与 `projects/` 的实测清单

| 产物 | 实测命令 | 结果 |
|---|---|---|
| `labs/01_serde_json` | `cargo run -p l01_serde_json` | ✅ exit 0，77 行输出 |
| `labs/02_regex` | `cargo run -p l02_regex` | ✅ exit 0；性能对比段实测循环内编译慢 **~400×** |
| `labs/03_clap` | `cargo run -p l03_clap -- --help` | ✅ clap 4 自动帮助，子命令齐全 |
| `labs/04_errors` | `cargo run -p l04_errors` | ✅ 错误链三种打印方式 |
| `labs/05_rayon` | `cargo run -p l05_rayon` | ✅ 串行 181ms vs 并行 29ms |
| `labs/06_rand_uuid` | `cargo run -p l06_rand_uuid` | ✅ 固定种子可复现（断言通过） |
| `labs/07_tracing` | `RUST_LOG=debug cargo run -p l07_tracing` | ✅ 嵌套 span 输出正确 |
| `labs/08_reqwest` | `cargo run -p l08_reqwest` | ✅ **真联网**：GitHub 200 + 反序列化；超时/连接拒绝/404 三类失败都正常收尾 |
| `projects/axum-todo` | `cargo test`（5 个 `#[tokio::test]`）+ 真起服务 curl | ✅ 5/5 通过；200/201/204/400/404 全部符合预期；clippy 干净 |
| `projects/sqlx-notes` | `cargo run`；`cargo run --features compile-check` | ✅ 事务提交与回滚演示正确；`query_as!` 编译期校验版也跑通 |
| `projects/bevy-square` | `cargo build` + `cargo run` | ✅ 编译链接成功（7m39s，exit 0，无 warning）；`cargo run` **成功创建窗口**（`bevy_render` 用 llvmpipe 软件 Vulkan 初始化）。⚠️ 本机只有软件光栅器，**画面内容与碰撞手感未经肉眼确认** |
| `projects/ai-infer` | `cargo run` | ✅ 纯 std 版 exit 0，33 行输出（softmax / argmax / 形状错误的 Result 传播 / benchmark） |

### 版本一致性

- `labs/` 与文档里的第三方 crate 版本**实测对齐**（例如 `reqwest` 统一在 0.13.5，
  文档里「0.13 的 TLS feature 叫 `rustls` 而不是 `rustls-tls`」的说法与实际依赖一致）。
- 第 18–21 章文档中的 API 用法均**以实际编译结果为准**，包括几个"凭记忆必错"的点：
  axum 0.8 的 `:id` 能编译但**启动即 panic**（要写 `{id}`）；
  裸 `Json` 的失败是 400/415/422 三种码；`rand` 0.9 改了 `rng()`/`random_range`；
  `itertools` 0.14 的 `chunk_by` 返回惰性结构体。

---

## 6. 已知的"纸面内容"（未实测，已在原处标注）

1. **`projects/ai-infer/src/main_candle.rs`**：按真实 candle API 写的完整实现，
   但本机 `static.crates.io` 只有 ~19 KB/s，`candle-core` 的 `gemm` 依赖拉不下来，
   **从未编译过**。文件顶部、`README.md`、ch21 文档都标注了这一点并给了恢复步骤。
2. **ch21 文档里 `ort` / `burn` / `tch` 的片段**：一律带 ⚠️「需要额外条件」，
   因为需要 `.onnx` 模型文件 / GPU / libtorch。
3. **ch17 文档里 `cargo audit` / `deny` / `expand` / `watch`**：只给了命令，本机未安装。
4. **ch17 文档里的版本号**来自实际 `Cargo.lock` / `cargo tree`；"镜像当前解析到的最新版"
   这一限制已在文档里注明。
5. `ai_review.py` 的真实 API 调用路径（见 §4）。

---

## 7. 复现全部验证

```bash
cd /home/yxb/code/rs-study
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"

# 练习工程（断网可用）：期望 16 章全部 0/3（你还没做题）
python3 tools/check.py

# 参考答案自检：期望 16/16 全绿、48/48 通过
python3 tools/check.py --solutions

# 第三方库实验（需要网络）
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 仅在 ~/.cargo 只读的环境需要
cd labs && cargo run -p l01_serde_json

# 框架 demo
cd ../projects/axum-todo   && cargo test && cargo run
cd ../projects/sqlx-notes  && cargo run
cd ../projects/ai-infer    && cargo run
cd ../projects/bevy-square && cargo build
```

---

## 8. 环境备注（构建过程中踩到的坑）

- **`~/.cargo` 只读**：受限环境里 cargo 无法解压下载的 crate（`Read-only file system`）。
  解法是把 `CARGO_HOME` 指到工作区内：`export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home`
  （已 gitignore）。普通桌面环境不需要。
- **cargo 包名不能以数字开头**：所以 `labs/` 的目录名是 `01_serde_json`，
  **包名**是 `l01_serde_json`（`cargo run -p l01_serde_json`）。各 README 已注明。
- **crates.io 直连不稳**：本机 `static.crates.io` 实测 ~19 KB/s。
  需要拉重依赖时按 [docs/01-setup.md](docs/01-setup.md) 第 2 节配国内镜像。
- 构建产物（`target/`）与依赖缓存（`.cargo-home/`）都在 `.gitignore` 里。
- **构建产物已经清理**：仓库里没有留任何 `target/`（那是 10 GB 级的临时文件）。
  第一次 `cargo build` / `cargo run` 会重新生成，Bevy 那个 demo 需要约 8 分钟。
- **依赖缓存保留着**，所以**不需要重新下载** crate：
  仓库根的 `.cargo-home/`（labs 与多数 demo 用）和
  `projects/bevy-square/.cargo-home/`（Bevy 单独一份，因为共用时会卡 package-cache 文件锁）。
  这两个目录都可以随时删掉；删掉后重新构建会重新下载依赖。
