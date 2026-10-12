# 环境与工具链

> 目标：让 `cargo`、`clippy`、`rustfmt`、`rust-analyzer` 四件套可用，并且知道每个命令干啥。

---

## 1. 本机现状（已勘察）

**已安装：rustc / cargo 1.98.1 stable（LLVM 22，x86_64-unknown-linux-gnu）。**
这个版本足够新，`[lints] workspace`、`let-else`、`div_ceil` 等都用得上，
本教程统一使用 **edition 2021**（而不是 2024）以保证可移植性。

但**`cargo` 可能不在 `PATH` 上**：

```
~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/
├── cargo        ✅ 真实的 cargo
├── rustc        ✅ 真实的编译器
├── cargo-clippy / clippy-driver
├── cargo-fmt / rustfmt
├── rustdoc / rust-gdb / rust-lldb
└── ...

~/.cargo/bin/
├── rustup       ← 只有这个和几个第三方工具
├── rustscan / doxx / viu
└── （缺少 cargo / rustc / rustdoc 的 shim！）
```

正常情况下 `rustup` 会在 `~/.cargo/bin/` 里放一堆 **shim**，
由它转发到当前默认 toolchain。你这里 shim 缺失，所以直接敲 `cargo` 会 command not found。

### 修复（二选一）

**方案 A：让 rustup 重建 shim（推荐）**

```bash
rustup default stable        # 或 rustup toolchain install stable
ls ~/.cargo/bin              # 现在应该能看到 cargo / rustc / rustdoc 了
```

如果你的 shell 还没把 `~/.cargo/bin` 加进 `PATH`：

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
cargo --version              # cargo 1.x.y (...)
```

**方案 B：直接用 toolchain 里的二进制**

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
cargo --version
```

> 本教程的 `tools/check.py` **会自动定位 cargo**（先查 `PATH`，再查 `~/.cargo/bin`，
> 最后遍历 `~/.rustup/toolchains/*/bin`），并且会把找到的 bin 目录临时加进 `PATH`
> 好让 cargo 能找到配对的 `rustc`。所以哪怕你没修好 `PATH`，评测脚本也能跑。

---

## 2. 从零安装（换机器 / 重装时用）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version && cargo --version
```

`rustup` 是工具链管理器（类似 `pyenv` + `nvm` 的结合体）：

```bash
rustup show                    # 当前工具链、默认版本、已装组件
rustup update                  # 升级到最新 stable
rustup component add clippy rustfmt rust-src rust-analyzer
rustup toolchain install nightly     # 只在需要 nightly 特性时
rustup target add x86_64-unknown-linux-musl   # 交叉编译到静态链接
```

### 国内网络：配置镜像（能省很多时间）

```bash
# ① rustup 自身下载走镜像
export RUSTUP_DIST_SERVER=https://mirrors.tuna.tsinghua.edu.cn/rustup
export RUSTUP_UPDATE_ROOT=https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup

# ② crates.io 索引与下载走镜像（写进 ~/.cargo/config.toml）
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
EOF
```

> 本教程的 `exercises/` **不依赖任何第三方 crate**，所以在完全离线的环境下
> 也能 `python3 tools/check.py` 跑通所有语法练习。只有第 17 章之后的实验才需要拉依赖。

---

## 3. cargo 命令速查（对比你的已有习惯）

| 任务 | C/C++ | Python | Rust |
|---|---|---|---|
| 新建项目 | 手建目录 + Makefile | `poetry new` | `cargo new demo` / `cargo new --lib demo` |
| 编译 | `make` | —（解释执行） | `cargo build` |
| 发布编译 | `make -O3` | — | `cargo build --release` |
| 运行 | `./a.out` | `python main.py` | `cargo run` |
| 跑测试 | 第三方框架 | `pytest` | **`cargo test`** |
| 加依赖 | 手动下库 | `pip install x` | `cargo add x`（写进 Cargo.toml） |
| 静态检查 | `clang-tidy` | `pylint` | **`cargo clippy`** |
| 格式化 | `clang-format` | `black` | **`cargo fmt`** |
| 文档 | doxygen | sphinx | `cargo doc --open` |
| 基准测试 | 手写 | `timeit` | `cargo bench`（需 criterion） |
| 依赖审计 | — | `pip-audit` | `cargo audit`（需安装） |

最常用的五条：

```bash
cargo check        # 只做类型检查，不生成二进制 —— 比 build 快得多，日常用它
cargo build        # 真编译
cargo run          # 编译并运行
cargo test         # 编译并跑全部测试（单元 + 集成 + 文档测试）
cargo clippy -- -D warnings   # 把 lint 警告当错误，用于给自己上强度
```

> **`cargo check` 是你最该养成的习惯**。改完一行代码先 `check`，
> 它跳过代码生成，通常比 `build` 快 3–10 倍。

---

## 4. 项目结构

```bash
cargo new hello        # 二进制项目
cargo new --lib mylib  # 库项目
```

```
hello/
├── Cargo.toml        # 清单：包名、版本、依赖、特性（≈ pyproject.toml + CMakeLists.txt）
├── Cargo.lock        # 精确版本锁定（库项目通常不提交，应用项目要提交）
├── src/
│   ├── main.rs       # 二进制入口：fn main()
│   └── lib.rs        # 库入口（库项目）
│   └── bin/          # 额外可执行文件放这里
├── tests/            # 集成测试（每个文件是独立 crate）
├── benches/          # 基准测试
├── examples/         # `cargo run --example foo`
└── build.rs          # 构建脚本（可选，≈ configure 阶段）
```

### 工作区（workspace）

多个相关的 crate 放一起共享一份 `target/` 和依赖版本：

```toml
# 根 Cargo.toml
[workspace]
resolver = "2"
members = ["ch01_basics", "ch02_control_flow"]

[workspace.package]
edition = "2021"
version = "0.1.0"
```

```bash
cargo test -p ch01_basics     # 只测 workspace 里的某个成员
cargo test --workspace        # 全部
```

**本教程的 `exercises/` 就是一个 workspace**，但它**不用** `--workspace` 一次性跑，
因为 `check.py` 会逐章跑并给出分章进度（一次全跑的话，失败信息会混在一起）。

---

## 5. 编辑器

**VS Code**（最省事）：

```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb        # 调试
code --install-extension tamasfe.even-better-toml   # Cargo.toml 补全
```

推荐 `settings.json`：

```json
{
  "rust-analyzer.check.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": { "editor.defaultFormatter": "rust-lang.rust-analyzer" },
  "rust-analyzer.inlayHints.lifetimeElisionHints.enable": "skip_trivial",
  "rust-analyzer.inlayHints.typeHints.enable": true
}
```

`rust-analyzer` 的价值在于：**它把编译错误实时标红**。
学 Rust 时这相当于开卷考试——你每写一行就知道对不对，反馈循环缩到 1 秒。

**Neovim**：`rustaceanvim`（基于 rust-analyzer）+ `nvim-dap` 调试。

---

## 6. 学会读编译器错误

这是学 Rust 最高杠杆的技能。错误信息结构固定：

```
error[E0382]: borrow of moved value: `s`          ← 错误码，可以 rustc --explain E0382
 --> src/main.rs:4:20
  |
2 |     let t = s;
  |             - value moved here                ← 第一次「出事」的位置
4 |     println!("{s}");
  |                ^ value borrowed here after move  ← 真正报错的位置
  |
  = help: consider cloning the value if the performance cost is acceptable
  = note: move occurs because `s` has type `String`, which does not implement the `Copy` trait
```

读法：

1. **先看错误码**，`rustc --explain E0382` 会给出完整解释和反例。
2. **先看 `-->` 下面被标箭头的行**，那是"真正报错处"。
3. **再看 `- value moved here`**，那才是"病因"——通常不在报错行上。
4. **最后看 `help:` / `note:`**，很多时候编译器直接写了解法。

常见错误码速查（本教程会反复遇到）：

| 错误码 | 含义 | 对应章节 |
|---|---|---|
| E0382 | 使用了已移动的值 | 第 3 章 |
| E0502 | 已有可变借用时又借用 | 第 4 章 |
| E0499 | 同时两个可变借用 | 第 4 章 |
| E0106 | 缺少生命周期标注 | 第 10 章 |
| E0597 | 借用的值活得不够久 | 第 10 章 |
| E0277 | trait bound 不满足 | 第 9 章 |
| E0308 | 类型不匹配 | 第 1 章 |
| E0433 | 找不到模块/路径 | 第 14 章 |

用 `cargo clippy` 而不是 `cargo build` 做日常检查，
它会额外告诉你"这段代码能编译，但写法不地道"——学 Rust 时这是免费的代码评审。

---

## 7. 运行本教程的评测工具

需要 Python 3.8+（只用标准库）：

```bash
cd /home/yxb/code/rs-study
python3 --version               # 确认有 python3

python3 tools/check.py --list   # 列出全部章节
python3 tools/check.py          # 跑全部（第一次会编译 16 个 crate，慢）
python3 tools/check.py ch01_basics -v   # 单章 + 完整输出
```

想用 AI 批改，配置一个 API key（任何兼容 OpenAI 协议的服务都行）：

```bash
export RUST_TUTOR_API_KEY=sk-xxxx
export RUST_TUTOR_MODEL=deepseek-chat
python3 tools/ai_review.py ch01_basics --hint 2
```

不想配 key 也能用：

```bash
python3 tools/ai_review.py ch01_basics --dry-run
# 它会把整份 prompt 存到 reports/ai-prompt-ch01_basics-*.md，你粘到聊天窗口即可
```

---

## 8. 检查清单

- [ ] `cargo --version` 有输出（或知道怎么用 `tools/check.py` 绕过）
- [ ] `cargo clippy --version` 有输出；没有就跑 `rustup component add clippy`
- [ ] `cargo fmt --version` 有输出；没有就跑 `rustup component add rustfmt`
- [ ] 编辑器装了 rust-analyzer，随便打开一个 `.rs` 文件能看到类型提示
- [ ] `python3 tools/check.py ch01_basics` 能跑起来（全红是正常的，你还没做题）
- [ ] 我读懂了上面那个 E0382 的错误示例

下一步 → [02-study-plan.md](02-study-plan.md)
