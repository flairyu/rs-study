# Rust 快速入门（面向有 C / C++ / Python 基础的人）

一套**可评测、可批改**的 Rust 自学教程：21 章文档 + 48 道自动评测习题 +
标准库/常用库/四大框架入门 + AI 批改脚本。

难度曲线按「你已经会 C/C++/Python」设计：**不教编程，只讲 Rust 在哪儿不一样**。

---

## 30 秒上手

```bash
cd /home/yxb/code/rs-study

python3 tools/check.py --list        # ① 看全部章节
python3 tools/check.py ch01_basics   # ② 跑第 1 章，先看它怎么失败
$EDITOR exercises/ch01_basics/src/lib.rs   # ③ 把 unimplemented!() 换成你的实现
python3 tools/check.py ch01_basics   # ④ 再跑
```

过不了就让 AI 按层级提示你（**不是直接给答案**）：

```bash
python3 tools/ai_review.py ch01_basics --hint 1   # 只给方向
python3 tools/ai_review.py ch01_basics --hint 3   # 几乎给答案
python3 tools/ai_review.py ch01_basics --solution # 完整参考实现 + 讲解
```

没配 API key 也能用：加 `--dry-run`，它会把整份 prompt 存成文件，你粘到任意聊天窗口即可。

---

## 先读这三篇

| 文档 | 什么时候读 | 内容 |
|---|---|---|
| [00-overview.md](docs/00-overview.md) | **现在** | 总纲：Rust 的心智模型、与 C/C++/Python 的差异地图、怎么用这套教程 |
| [01-setup.md](docs/01-setup.md) | 开始前 | 工具链、cargo 速查、编辑器、怎么读编译器错误 |
| [02-study-plan.md](docs/02-study-plan.md) | 开始前 | 4 周速通 / 8 周扎实计划、每日 90 分钟模板、里程碑项目 |

---

## 章节全图

**第一阶段 · 语法与心智模型**（全部有自动评测，只依赖 std）

| # | 章节 | 文档 | crate | 核心 |
|---|---|---|---|---|
| 1 | 变量、类型与可变性 | [ch01](docs/01-syntax/ch01-basics.md) | `ch01_basics` | `mut`、隐式转换不存在、格式化 |
| 2 | 函数与控制流 | [ch02](docs/01-syntax/ch02-control-flow.md) | `ch02_control_flow` | 表达式 vs 语句、`match` 穷尽 |
| 3 | **所有权** ★ | [ch03](docs/01-syntax/ch03-ownership.md) | `ch03_ownership` | move、`Copy`/`Clone`、`Drop` |
| 4 | **引用与借用** ★ | [ch04](docs/01-syntax/ch04-borrowing.md) | `ch04_borrowing` | `&T` / `&mut T`、切片 |
| 5 | 结构体、枚举与模式匹配 | [ch05](docs/01-syntax/ch05-structs-enums.md) | `ch05_structs_enums` | `impl`、`Option<T>`、`match` |
| 6 | 错误处理 | [ch06](docs/01-syntax/ch06-error-handling.md) | `ch06_error_handling` | `Result`、`?`、自定义错误 |
| 7 | 集合 | [ch07](docs/01-syntax/ch07-collections.md) | `ch07_collections` | `Vec`/`String`/`HashMap` |
| 8 | 闭包与迭代器 | [ch08](docs/01-syntax/ch08-iterators-closures.md) | `ch08_iterators_closures` | `Fn`/`FnMut`/`FnOnce`、惰性 |
| 9 | 泛型与 trait | [ch09](docs/01-syntax/ch09-generics-traits.md) | `ch09_generics_traits` | bound、`impl Trait`、`dyn` |
| 10 | 生命周期 | [ch10](docs/01-syntax/ch10-lifetimes.md) | `ch10_lifetimes` | `'a`、省略规则、`'static` |
| 11 | 智能指针与内部可变性 | [ch11](docs/01-syntax/ch11-smart-pointers.md) | `ch11_smart_pointers` | `Box`/`Rc`/`RefCell` |
| 12 | 并发 | [ch12](docs/01-syntax/ch12-concurrency.md) | `ch12_concurrency` | `Arc<Mutex>`、`mpsc`、`Send` |
| 13 | async / await | [ch13](docs/01-syntax/ch13-async.md) | `ch13_async` | `Future`、`Poll`、手写 `block_on` |
| 14 | 模块、crate 与宏 | [ch14](docs/01-syntax/ch14-modules-macros.md) | `ch14_modules_macros` | `mod`、`pub use`、`macro_rules!` |
| 15 | 测试、unsafe 与 FFI | [ch15](docs/01-syntax/ch15-testing-unsafe.md) | `ch15_testing_unsafe` | `#[test]`、裸指针、`extern "C"` |

**第二阶段 · 标准库与生态**

| # | 章节 | 文档 | 形式 |
|---|---|---|---|
| 16 | 标准库巡礼 | [ch16](docs/02-stdlib/ch16-stdlib-tour.md) | 自动评测 `ch16_stdlib_tour` |
| 17 | 常用第三方库 | [ch17](docs/03-libs/ch17-ecosystem.md) | `labs/` 动手实验 |

**第三阶段 · 框架入门**

| # | 章节 | 文档 | demo |
|---|---|---|---|
| 18 | Web：axum | [ch18](docs/04-frameworks/ch18-web-axum.md) | `projects/axum-todo` |
| 19 | 游戏：Bevy | [ch19](docs/04-frameworks/ch19-game-bevy.md) | `projects/bevy-square` |
| 20 | 数据库：sqlx | [ch20](docs/04-frameworks/ch20-db-sqlx.md) | `projects/sqlx-notes` |
| 21 | AI 推理：candle / ort | [ch21](docs/04-frameworks/ch21-ai-inference.md) | `projects/ai-infer` |

---

## 目录结构

```
rs-study/
├── README.md              ← 你在这里
├── docs/
│   ├── 00-overview.md     总纲（22 分钟读完，最重要）
│   ├── 01-setup.md        环境与工具链
│   ├── 02-study-plan.md   4 周 / 8 周计划
│   ├── 01-syntax/         第 1–15 章
│   ├── 02-stdlib/         第 16 章
│   ├── 03-libs/           第 17 章
│   └── 04-frameworks/     第 18–21 章
├── exercises/             cargo workspace：16 个练习 crate，零第三方依赖
├── solutions/             ★ 参考答案（16 个 crate）+ 仓库自检，见下
├── labs/                  需要第三方 crate 的实验（serde/regex/clap/...）
├── projects/              框架 demo（axum/bevy/sqlx/ai）
├── tools/
│   ├── check.py           ★ 跑测试 + 进度报告
│   ├── ai_review.py       ★ AI 批改 + 分层提示
│   ├── chapters.json      章节索引
│   ├── AUTHORING.md       编写规范（加新章节看这个）
│   └── CHAPTER-SPECS.md   每章练习的精确 API 规格
└── reports/               运行日志与 AI 批改存档（自动生成，已 gitignore）
```

---

## 设计说明：为什么这么组织

**1. 练习题永远能编译。**
每个函数骨架都是 `unimplemented!()`（类型是 `!`，能强制转成任何返回类型）。
所以你看到的失败永远是**断言失败**（"行为不对"），而不是"编译不过"。
编译错误只可能来自你自己写的代码——归因清晰，也不会让一道题卡住整章。

**2. 评判标准是测试，不是我。**
`tests/chapter.rs` 是唯一裁判，3 题 3 个 `#[test]`。想改题面就改测试——但要先知道自己在干什么。

**3. 练习 crate 零第三方依赖。**
`cargo test` 在断网环境也能跑通全部 16 章。需要依赖的内容全部隔离在 `labs/` 和 `projects/`。

**4. AI 批改按"提示层级"分层。**
`--hint 0/1/2/3` 从"只说哪题错了"到"留一处给你补"。
批改时会**自动把文档里的参考答案从 prompt 里删掉**，防止模型剧透。

**5. 文档的锚点是 C/C++/Python。**
每章开头都有对照表。你是"换语言"，不是"学编程"。

**6. 参考答案不是抄来的，是被测试验证过的。**
`solutions/` 里的每个 crate 都是对应章节的参考实现，而它的 `tests/chapter.rs`
通过 `#[path]` **复用练习工程里的那一份**——测试只有一份真相源，答案和题目不可能漂移。
所以 `python3 tools/check.py --solutions` 全绿就等价于"这 48 道题都是可解的、期望值都是对的"。
自检过程中正是靠它抓出了几处真实的题目 bug（见 [STATUS.md](STATUS.md)）。

---

## 常用命令

```bash
python3 tools/check.py --list                 # 章节列表
python3 tools/check.py                        # 全部章节
python3 tools/check.py ch03_ownership -v      # 单章 + 完整输出
python3 tools/check.py --next                 # 跑到第一个未通过处停下
python3 tools/check.py --json                 # 附机器可读结果
python3 tools/check.py --solutions            # 跑参考答案（仓库自检，应全绿）

python3 tools/ai_review.py ch03_ownership --hint 2
python3 tools/ai_review.py ch03_ownership --solution
python3 tools/ai_review.py ch03_ownership --dry-run   # 不用 API key

cd exercises && cargo check -p ch03_ownership         # 最快的类型检查
cd exercises && cargo clippy -p ch03_ownership -- -D warnings
```

---

## 你需要知道的四个前提

1. **`cargo` 可能不在 `PATH` 上**。本机 `~/.cargo/bin` 缺少 shim。
   先跑 `rustup default stable` 修复，或看 [01-setup.md](docs/01-setup.md) 第 1 节。
   （`tools/check.py` 会自动定位 cargo，所以不修也能用。）
2. **第 1–16 章完全离线可用**；`labs/` 与 `projects/` 需要网络拉取第三方 crate。
3. **AI 批改需要 API key**（任何兼容 OpenAI 协议的服务）。没有就用 `--dry-run` 导出 prompt。
4. **如果你在受限环境（沙箱/只读 `~/.cargo`）里跑 `labs/` 或 `projects/`**，
   把 cargo 的缓存目录指到工作区内再跑：
   ```bash
   export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home
   ```
   普通桌面环境不需要这一步。

---

## 从零开始，第一条命令

```bash
python3 tools/check.py --list && python3 tools/check.py ch01_basics
```

然后读 [docs/00-overview.md](docs/00-overview.md)。

> 前 15 章会很慢，第 3、4 章会让你怀疑人生。挺过去之后，第 9–13 章几乎是免费的
> ——那些设计全是前三章的必然推论。


## 支持作者

如果这个项目对你有帮助，可以考虑请我吃个馒头 ☕

| 微信 | 支付宝 |
| :---: | :---: |
| ![微信收款码](assets/wechat.png) | ![支付宝收款码](assets/alipay.png) |