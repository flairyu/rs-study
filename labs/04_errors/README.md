# 实验 04 · anyhow + thiserror

> 目录名带序号只是为了排序；**包名**是 `l04_errors`（cargo 不允许包名以数字开头）。

## 它们是什么，解决什么问题

Rust 的错误处理只有 `Result<T, E>` 一个机制，但 **E 该是什么类型**没有统一答案：

| 场景 | 问题 | 解法 |
|---|---|---|
| 库 / 领域层 | 调用者需要 `match` 出失败原因，E 必须是**具体、可穷举**的类型 | `thiserror` 派生 `Display` + `Error` + `From` |
| 应用 / `main` / 业务流程 | 一个函数里可能出十几种错，为每种组合定义枚举类型会爆炸；调用者只想打印 | `anyhow::Error`（类型擦除）+ `.context()` |

- `thiserror`：**宏**。你写枚举，它生成样板代码。没有运行时开销，错误类型仍是你自己的。
- `anyhow`：**运行时错误类型**。任何实现了 `std::error::Error` 的类型都能 `?` 进去，
  并且可以一层层挂上「在做什么时失败」的语境，打印时形成错误链。

## 覆盖的四个点

| # | 演示 | 关键 API |
|---|---|---|
| ① | `thiserror` 错误枚举 | `#[error("...")]`、`#[from]`、`#[error(transparent)]`、带字段变体 |
| ② | `anyhow::Result` 传播多种错误 | `?`、`.context()` / `.with_context()`、`anyhow!` / `bail!` |
| ③ | 打印错误链 | `{:#}` 单行链、`{:?}` 详细、手写 `Error::source()` 遍历 |
| ④ | 取舍 | 「库用 thiserror、应用用 anyhow」，两者可以叠加 |

## 核心规矩

```rust
// 库：错误是公开 API，必须能被 match
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置项 `{key}` 的值非法：{value:?}")]
    InvalidValue { key: String, value: String },
    #[error("读取配置文件失败")]
    Io { #[from] source: std::io::Error },   // ? 自动 From
    // 想让库内部继续用 anyhow、对外仍暴露稳定类型，就包一层 #[source]：
    #[error("加载配置失败")]
    Load { #[source] source: anyhow::Error },
}

// 应用：只关心「哪一步、做什么时失败」
fn load_config(path: &str) -> anyhow::Result<(u16, f64, String)> {
    let text = read_config_text(path).with_context(|| format!("打开配置 {path} 失败"))?;
    ...
}
```

**一句话：错误要被别人 match，就用 thiserror；错误只会被你打印，就用 anyhow。**

## 什么时候别用它

- **`anyhow` 不要出现在库的公开 API 里**。`anyhow::Error` 类型擦除后调用者
  无法 `match`（只能 `downcast_ref`，很脆），而且 anyhow 承诺不稳定的小版本语义，
  依赖它的库会把约束传给所有下游。
- **`thiserror` 不要用在 `main` 里**。给每种失败组合定义一个枚举，代码量会爆炸，
  而且这些类型没有复用价值。
- **错误里不要放大对象 / 敏感信息**：`#[error("...")]` 生成的是 `Display`，
  `Display` 会被打印、写日志、返回给用户；`Debug` 才用于诊断。两者要分清。
- **不要用 `.unwrap()` / `.expect()` 代替错误传播**在库代码里；panic 是给「bug」用的，
  不是给「可预期的失败」用的（应用里对「不可能发生」的不变量可以用 `expect`）。
- **需要结构化错误码给外部 API 时**：用 `thiserror` 定义枚举，再手工实现
  `IntoResponse`（web）之类的转换，不要直接 `anyhow` 一把梭。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 本机 ~/.cargo 只读
cargo run -p l04_errors
```

程序不会故意失败退出（所有失败都被演示代码接住了），所以退出码是 0。

## 预期输出（节选）

```text
实验 04 · 错误处理（thiserror + anyhow）
（thiserror 生成 Display/Error/From；anyhow 负责跨层传播和上语境）

===== ① thiserror 错误枚举 =====
parse_port("8080") = Ok(8080)
parse_port("80") = Err -> 配置项 `port` 的值非法："80"
   → 业务规则不允许：port=80（低于 1024 的端口需要 root）
    │ 配置项 `port` 的值非法："80"
parse_port("not_a_number") = Err -> 数字解析失败
   → 不是合法数字：invalid digit found in string（这是 #[from] 自动转换来的）
    │ 数字解析失败
      │ invalid digit found in string

#[from] 自动转换：读取配置文件失败
transparent 变体：权限不足

===== ③ 打印 {:?} 看错误链 =====
--- load_config("bad.conf") ---
解析 bad.conf 第 1 行的 port 时出错: 数字解析失败: invalid digit found in string  ← 用户该看到的一行
--- 详细（{:?}）：
解析 bad.conf 第 1 行的 port 时出错

Caused by:
    0: 数字解析失败
    1: invalid digit found in string
--- 手动遍历 source() 得到的错误链：
  │ 解析 bad.conf 第 1 行的 port 时出错
    │ 数字解析失败
      │ invalid digit found in string
```

顶层 `AppError` 的链（`#[source]` 包裹 anyhow 错误，链路不会断）：

```text
顶层 AppError 的 Display：配置错误：加载配置失败
错误链（source() 展开）：
  │ 配置错误：加载配置失败
    │ 加载配置失败
      │ 解析 bad.conf 第 1 行的 port 时出错
        │ 数字解析失败
          │ invalid digit found in string
```

## 常见坑

- `#[from]` **每个错误类型只能有一个**同样 source 类型的变体，否则 `From` 实现冲突。
- 只想**保留错误链**而不想要 `From` 转换，用 `#[source]`：它只标记「这是下层原因」，
  于是可以把 `anyhow::Error` 这种「不该出现在公开 API 里」的类型包一层，
  同时保住 `source()` 链（本实验的 `ConfigError::Load` 就是这么写的）。
- `#[error("...")]` 的格式化串里引用字段用 `{field}`；引用元组位置用 `{0}`；
  引用 source 用 `{source}`。写错字段名是编译错误，不是运行时错误。
- `.context()` 加的是「**在做什么**」，不是「为什么失败」。
  写成 `.context("解析 port 出错")`，而不是 `.context("不是合法数字")`——后者是 source 的职责。
- `anyhow::Error` 的 `{:#}` 只打印一层语境 + 链尾，想看全部用 `{:?}` 或自己遍历 `source()`。
- `main() -> anyhow::Result<()>` 返回 `Err` 时会用 `Debug` 打印并**退出码 1**；
  想完全自定义输出，就在 `main` 里 `match` 后 `std::process::exit`。
- `thiserror` 在你的 crate 里生成 `impl From<...>`；如果手写同类型 `From` 会编译冲突。
