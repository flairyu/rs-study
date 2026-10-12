# 内容编写规范（AUTHORING）

本文件是这套教程的**唯一格式契约**：所有章节文档与练习 crate 都必须遵守它。
它同时是写给「以后要继续加章节的人（或 AI）」看的说明书。

---

## 1. 仓库结构

```
rs-study/
├── README.md                 # 总入口：怎么用、现在学到哪、下一步做什么
├── docs/
│   ├── 00-overview.md        # 总纲：Rust 是什么、和 C/C++/Python 的差异地图
│   ├── 01-setup.md           # 环境与工具链（rustup / cargo / clippy / rustfmt）
│   ├── 02-study-plan.md      # 4 周学习计划与时间预算
│   ├── 01-syntax/            # 第 1–15 章：语法与特性
│   ├── 02-stdlib/            # 第 16 章：标准库巡礼
│   ├── 03-libs/              # 第 17 章：常用第三方库
│   └── 04-frameworks/        # 第 18–21 章：web / 游戏 / 数据库 / AI
├── exercises/                # 自动评测的练习工作区（**只依赖 std**）
│   ├── Cargo.toml            # workspace，members 列出全部章节 crate
│   ├── chXX_*/               # 一章一个 crate
│   └── ...
├── labs/                     # 需要第三方 crate 的小实验（serde / regex / ...）
├── projects/                 # 框架 demo（axum / bevy / sqlx / 推理）
└── tools/
    ├── chapters.json         # 章节唯一索引
    ├── check.py              # 跑测试、生成进度报告
    ├── ai_review.py          # 收集代码+失败信息，交给 AI 批改与指导
    └── AUTHORING.md          # 本文件
```

**硬规则**：`exercises/` 下任何 crate **不得**添加第三方依赖。
原因：保证 `cargo test` 在完全离线、无网环境下也能跑通。需要第三方库的内容一律放
`labs/`（实验）或 `projects/`（框架 demo），并在文档里注明「需要网络拉取依赖」。

---

## 2. 一章 = 三份产物

每新增一章 `chXX_name`，必须同时产出：

| 产物 | 路径 | 作用 |
|---|---|---|
| 教程文档 | `docs/<分组>/chXX-<slug>.md` | 讲解 + 示例 + 参考答案 |
| 练习骨架 | `exercises/chXX_name/src/lib.rs` | 用户要改的文件，函数体是 `unimplemented!()` |
| 自动评测 | `exercises/chXX_name/tests/chapter.rs` | 裁判，用户不得修改 |

外加：
- `exercises/chXX_name/Cargo.toml`
- `exercises/chXX_name/README.md`
- 在 `exercises/Cargo.toml` 的 `members` 里登记
- 在 `tools/chapters.json` 里登记

---

## 3. `Cargo.toml` 模板（逐字复制，只改 name）

```toml
[package]
name = "chXX_name"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]

[lints]
workspace = true
```

不要写 `edition = "2024"`：要兼容较旧的 stable 工具链，统一用 `2021`（由 workspace 提供）。

---

## 4. `src/lib.rs` 模板

```rust
//! # 第 N 章 · 章节标题
//!
//! 一到三句话说明本章解决什么问题。
//!
//! 运行评测：
//! ```bash
//! python3 tools/check.py chXX_name
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`ex_a`] | ... |
//! | [`ex_b`] | ... |
//! | [`ex_c`] | ... |
#![allow(unused_variables)]

/// 练习 N-1：一句话说明要做什么。
///
/// 详细说明：输入是什么、输出是什么、边界情况怎么处理。
/// 可以顺便点出这里最容易踩的坑。
///
/// # 示例
/// ```
/// # use chXX_name::ex_a;
/// assert_eq!(ex_a(1), 2);
/// ```
pub fn ex_a(x: i32) -> i32 {
    unimplemented!("练习 N-1：把这里的 unimplemented!() 换成你的实现")
}
```

要点：

1. **每个练习一个 `pub fn`**（也可以是 `pub struct` + 方法，但同一章保持一致）。
2. 函数体只写 `unimplemented!("练习 N-K：...")`。`unimplemented!()` 的类型是 `!`，
   能强制转换成任何返回类型，所以骨架**永远能编译通过**——用户看到的失败一定是断言失败，
   而不是编译错误。这是本教程的核心设计。
3. 文档注释里必须写清**规格**（输入/输出/边界），但不写实现步骤。
   实现思路放到教程文档的「提示」里。
4. 每个函数配一个文档测试（`# 示例` 代码块），它是「规格的可执行版本」。
   注意：骨架期文档测试会失败（因为 `unimplemented!()` panic），这是**正常且有意**的。
5. 参数名写全，不要用 `_x` 规避告警——crate 顶部已经有 `#![allow(unused_variables)]`。

---

## 5. `tests/chapter.rs` 模板

```rust
//! 第 N 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py chXX_name`（或 `cargo test -p chXX_name`）

use chXX_name::*;

#[test]
fn ex_n_1_foo() {
    assert_eq!(foo(1), 2);
    // 每个测试至少覆盖：正常值 + 边界值（0 / 空 / 最大值 / 负数）
}
```

要点：

1. 测试函数命名：`ex<章号>_<序号>_<函数名>`，全小写。
2. **3 个练习 → 3 个 `#[test]` 函数**（不要拆成更多函数，便于 `check.py` 统计）。
   单个测试内部可以有多条断言，覆盖正常值、边界值、非法值。
3. 每条断言尽量带中文消息：`assert_eq!(a, b, "空输入应返回 0")`。
4. 浮点比较必须用容差，不要用 `assert_eq!`：
   ```rust
   fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }
   ```
5. 不要 `use std::panic::catch_unwind` 之类的花活；断言失败就是失败。
6. 测试里不要调用 `unimplemented!()` 之外的私有实现，也不要依赖具体算法，
   只依赖 `lib.rs` 里承诺的**公开 API 与语义**。
7. 不要测试「编译失败」的场景（如借用冲突）——那属于文档讨论，不属于自动评测。

---

## 6. 章节文档模板（`docs/.../chXX-slug.md`）

```markdown
# 第 N 章 · 标题

> **一句话定位**：...
> **预计时间**：90 分钟（读 40 + 写 50）
> **前置**：第 N-1 章
> **练习工程**：`exercises/chXX_name`（3 题）

## 0. 先建立锚点（给 C/C++/Python 使用者）

用你最熟悉的三门语言对照，快速说明「Rust 在这里做了什么不同的选择」。
用表格或三段短对比代码。**这一节决定读者会不会读下去，务必具体。**

## 1. 小节标题

讲解 + **可运行示例**。示例代码块必须标明语言是 `rust`，
并且在可能的情况下给出输出（用 `// 输出:` 注释或单独的输出块）。

> ⚠️ **坑**：把最容易犯的错单独拎出来。

### 1.1 子小节

...

## 2. ...

## N. 本章练习

三题，每题给出：题号、要做什么、考点、提示（不给答案）。

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 1-1 | ... | ... | `fahrenheit_to_celsius` |

完成方式：

```bash
python3 tools/check.py chXX_name
```

卡住了就跑：

```bash
python3 tools/ai_review.py chXX_name --hint 2
```

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 1-1
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}
```

... 每题一小段，附一句「为什么这么写」

</details>

## 自测清单

- [ ] 我能不看文档说出 ...
- [ ] 我知道 ... 为什么会导致编译错误
- [ ] 我完成了 3 道练习且 `check.py` 全绿
```

### 写作要求

- **中文**为主，术语保留英文原词（`ownership`、`borrow`、`trait`）。
- 代码示例必须是**真的能编译**的（能跑就更要给出输出）。不确定就写简单一点。
- 每个概念都要回答「**为什么 Rust 要这么设计**」，尤其是从 C/C++ 迁移过来会觉得别扭的地方。
- 避免大段抄官方文档；用自己的话重讲，并给出**对照表**。
- 每章长度控制在大约 200–400 行 Markdown。
- 不要在文档里链接不存在的文件。

---

## 7. `chapters.json` 登记格式

```json
{
  "id": "chXX_name",
  "order": 17,
  "group": "syntax",
  "title": "第 N 章 · 标题",
  "doc": "docs/01-syntax/chXX-slug.md",
  "crate": "chXX_name"
}
```

`group` 取值：`syntax` / `stdlib` / `libs` / `frameworks`。
`crate` 写 `null` 表示该章没有自动评测工程（框架章就是这样）。

---

## 8. 自我检查（提交前逐条过）

- [ ] `exercises/Cargo.toml` 的 `members` 里有我的 crate
- [ ] `tools/chapters.json` 里有我的章节，并且 `doc` 路径真实存在
- [ ] `cargo test -p chXX_name` 能编译（必然失败于断言，但不能失败于编译）
- [ ] `tests/chapter.rs` 里恰好 3 个 `#[test]`
- [ ] 文档里的参考实现与 `tests/chapter.rs` 的期望一致
- [ ] 没有引入任何第三方依赖
- [ ] 没有用 `edition = "2024"`
