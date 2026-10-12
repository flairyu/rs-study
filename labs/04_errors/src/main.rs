//! # 实验 04 · 错误处理：thiserror 定义错误，anyhow 传播错误
//!
//! 四段演示：
//! 1. 用 `thiserror` 定义错误枚举，`#[error("...")]` 写 Display，`#[from]` 自动 `From`；
//! 2. 用 `anyhow::Result` 写一个会传播多种错误的函数，`.context()` 逐层加语境；
//! 3. 打印 `{:?}` 看 anyhow 的完整错误链，以及 `{:#}` 的单行版本；
//! 4. 讲清「库用 thiserror、应用用 anyhow」的取舍。
//!
//! 运行：`cargo run -p l04_errors`

use anyhow::Context; // 把 .context() / .with_context() 带进作用域
use std::num::ParseIntError;

// ===========================================================================
// 第一部分：库侧错误类型（thiserror）
// ===========================================================================

/// 一个「库」会暴露给调用者的错误类型。
///
/// `thiserror` 只做一件事：帮你生成 `Display` 和 `std::error::Error` 的实现，
/// 外加 `From` 转换。它**不**改变控制流，也不打印日志。
///
/// ⚠️ `thiserror` 的典型用法是「结构化错误」：调用者可以用 match 分支分别处理，
/// 所以每个变体都应当携带足够的信息（哪个文件、哪个字段、期望什么）。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `#[error("...")]` 里的字符串就是 `Display` 的实现；
    /// `{0}` / `{field}` 可以插值，`{source}` 特指被包装的底层错误。
    /// 配置项的值非法；这种变体适合携带「哪个 key、什么值」这类结构化信息。
    #[error("配置项 `{key}` 的值非法：{value:?}")]
    InvalidValue { key: String, value: String },

    /// ③ 用 `#[source]`（而不是 `#[from]`）把一个 anyhow::Error 包进来。
    /// 好处：`Error::source()` 仍然能往下钻，错误链不会被打断；
    /// 这让「库暴露稳定错误类型」和「内部用 anyhow 图省事」可以共存。
    #[error("加载配置失败")]
    Load {
        #[source]
        source: anyhow::Error,
    },

    /// 带 `#[from]` 的变体让 `?` 能自动把 io::Error 转成 ConfigError。
    /// 注意：`#[from]` 会生成 `impl From<io::Error> for ConfigError`。
    #[error("读取配置文件失败")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// 同理，字符串解析失败也能被 `?` 自动转换。
    #[error("数字解析失败")]
    ParseInt {
        #[from]
        source: ParseIntError,
    },
}

/// 另一种常见形态：不需要携带数据的「扁平」错误，用 `#[error(transparent)]`
/// 直接透传底层错误的 Display，或者干脆用元组变体。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("配置错误：{0}")]
    Config(#[from] ConfigError),

    #[error("用户 `{user}` 没有权限执行 `{action}`")]
    Forbidden { user: String, action: String },

    #[error(transparent)]
    Other(#[from] std::io::Error),
}

/// 一个「库风格」的函数：返回具体的错误类型，而不是 anyhow::Error。
///
/// 这样调用者能 match 出每一种失败原因；代价是签名里多了一个类型参数式的信息。
pub fn parse_port(raw: &str) -> Result<u16, ConfigError> {
    // 解析失败时 ? 会用 #[from] 生成的 From 把 ParseIntError 包成 ConfigError::ParseInt。
    let port: u16 = raw.parse()?;
    if port < 1024 {
        // 业务规则错误：带上 key 和原始值，调用者能据此提示用户改哪一项。
        return Err(ConfigError::InvalidValue {
            key: "port".to_string(),
            value: raw.to_string(),
        });
    }
    Ok(port)
}

// ===========================================================================
// 第二部分：应用侧错误传播（anyhow）
// ===========================================================================

/// 模拟读取一个配置文件。这里直接用内嵌字符串代替真实文件，
/// 保证输出稳定、可复现。
fn read_config_text(path: &str) -> anyhow::Result<String> {
    // `.context()` 给 Result 加一层语境：出错时 anyhow 会把它插进错误链。
    // 注意 context 接收的是「描述这次操作」的话，而不是错误原因本身。
    let text = match path {
        "app.conf" => "port = 8080\nratio = 3.14\nname = demo\n".to_string(),
        "bad.conf" => "port = not_a_number\nratio = 3.14\n".to_string(),
        _ => {
            // 真实代码里这里是 std::fs::read_to_string(path)；
            // 用 ? 传播 io::Error，anyhow 会自动把它装箱。
            anyhow::bail!("找不到配置文件 {path}");
        }
    };
    Ok(text)
}

/// 一个会传播**多种**错误的函数：io 错误、解析错误、缺字段错误……
/// 返回 `anyhow::Result<T>` 后，函数签名不必枚举所有错误类型。
fn load_config(path: &str) -> anyhow::Result<(u16, f64, String)> {
    let text = read_config_text(path)
        .with_context(|| format!("打开配置 {path} 失败（当前工作目录可能不对）"))?;

    let mut port = None;
    let mut ratio = None;
    let mut name = None;

    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // split_once 找不到 '=' 时返回 None，这里显式报错并带上行号。
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("第 {} 行缺少 `=`：{line:?}", lineno + 1))?;
        let key = key.trim();
        let value = value.trim();

        match key {
            "port" => {
                // port 要过两道校验，错误类型是库侧的 ConfigError，
                // 在这里被 ? 自动转成 anyhow::Error，并附上语境。
                port =
                    Some(parse_port(value).with_context(|| {
                        format!("解析 {path} 第 {} 行的 port 时出错", lineno + 1)
                    })?);
            }
            "ratio" => {
                ratio = Some(value.parse::<f64>().with_context(|| {
                    format!(
                        "解析 {path} 第 {} 行的 ratio 时出错（值 = {value:?}）",
                        lineno + 1
                    )
                })?);
            }
            "name" => name = Some(value.to_string()),
            // 未知字段不算致命错误，只警告并忽略。
            other => println!("  [警告] 忽略未知配置项 {other:?}"),
        }
    }

    // 缺必填项时用 anyhow! 现场造一个错误；.context() 补上「在做什么」。
    let port = port.context("配置里缺少必填项 `port`")?;
    let ratio = ratio.context("配置里缺少必填项 `ratio`")?;
    let name = name.unwrap_or_else(|| "<未命名>".to_string());

    Ok((port, ratio, name))
}

// ===========================================================================
// 第三部分：打印错误链
// ===========================================================================

/// 手写遍历 `std::error::Error::source()`，看清「错误链」到底是什么结构。
fn print_source_chain(mut err: &dyn std::error::Error) {
    let mut depth = 0;
    loop {
        println!("  {}│ {err}", "  ".repeat(depth));
        match err.source() {
            Some(next) => {
                err = next;
                depth += 1;
            }
            None => break,
        }
    }
}

fn try_load(path: &str) {
    println!("\n--- load_config({path:?}) ---");
    match load_config(path) {
        Ok((port, ratio, name)) => {
            println!("成功：port={port}, ratio={ratio}, name={name}");
        }
        Err(err) => {
            // {:#} 是 anyhow 的「单行链」格式：用 `: ` 把每层语境串起来。
            println!("{:#}  ← 用户该看到的一行", err);
            // {:?} 是 anyhow 的「多行详细」格式：每层一行，还带 backtrace 提示。
            println!("--- 详细（{{:?}}）：");
            println!("{err:?}");
            println!("--- 手动遍历 source() 得到的错误链：");
            print_source_chain(err.as_ref());
        }
    }
}

// ===========================================================================
// 第四部分：库用 thiserror、应用用 anyhow
// ===========================================================================

fn explain_tradeoffs() {
    println!("\n===== ④ 取舍：库用 thiserror、应用用 anyhow =====");
    println!(
        r#"
为什么这样分工？

thiserror（库侧 / 领域层）
  · 错误类型是**公开 API 的一部分**，要能被 match：调用者可能想「端口非法就提示用户，文件不存在就重试」。
  · 每个变体语义明确、可穷举；库不应替调用者决定怎么打印日志。
  · 代价：新增一种失败就要改枚举（对下游是破坏性变更），且要手写 From 转换。

anyhow（应用侧 / main 和业务编排）
  · main、CLI、一次性的业务流程只关心「失败了，把上下文打出来」，
    不想为每种组合定义新枚举类型（N 种错误相乘会爆炸）。
  · .context() 让错误链自然记录「在哪一步、做什么时失败」，比单层错误信息有用得多。
  · 代价：anyhow::Error 是类型擦除的，不能 match 出具体原因，也不适合做库的公开错误类型。

一句话记忆：**错误要被别人 match，就用 thiserror；错误只会被你打印，就用 anyhow。**
两者不冲突——库返回 thiserror 枚举，应用用 anyhow 把它们和语境一起包起来，正是下面的代码。
"#
    );

    // 演示两件事：
    // (a) 库错误可以直接当 anyhow::Error 传上去；
    // (b) 应用侧还能把 anyhow 错误再包一层，保留整条链。
    let app = run_app("bad.conf");
    match app {
        Ok(()) => println!("意外成功"),
        Err(err) => {
            println!("顶层 AppError 的 Display：{err}");
            println!("顶层 AppError 的 Debug：\n{err:?}");
            println!("错误链（source() 展开）：");
            print_source_chain(&err);
        }
    }
}

/// 应用层函数：对外返回稳定的 `AppError`，内部却用 `anyhow::Result` 图方便。
/// 边界处用 `#[source]` 把 anyhow 错误**包一层**（而不是拍平成字符串），
/// 这样 `Error::source()` 仍能一路钻到底层原因。
fn run_app(path: &str) -> Result<(), AppError> {
    let cfg = load_config(path).map_err(|source| ConfigError::Load { source })?;
    println!("配置加载成功：{cfg:?}");
    Ok(())
}

fn main() {
    println!("实验 04 · 错误处理（thiserror + anyhow）");
    println!("（thiserror 生成 Display/Error/From；anyhow 负责跨层传播和上语境）");

    println!("\n===== ① thiserror 错误枚举 =====");
    // 每个变体的 Display 都是 #[error("...")] 生成的那句话。
    for raw in ["8080", "80", "not_a_number"] {
        match parse_port(raw) {
            Ok(port) => println!("parse_port({raw:?}) = Ok({port})"),
            Err(err) => {
                println!("parse_port({raw:?}) = Err -> {err}");
                // 用 match 分别处理，这正是「库用 thiserror」的意义所在。
                match &err {
                    ConfigError::InvalidValue { key, value } => {
                        println!("   → 业务规则不允许：{key}={value}（低于 1024 的端口需要 root）");
                    }
                    ConfigError::ParseInt { source } => {
                        println!("   → 不是合法数字：{source}（这是 #[from] 自动转换来的）");
                    }
                    ConfigError::Io { source } => {
                        println!("   → io 错误：{source}");
                    }
                    ConfigError::Load { source } => {
                        // #[source] 让这里还能继续往下钻。
                        println!("   → 包装的 anyhow 错误：{source:#}");
                    }
                }
                // 用 &dyn Error 也能拿到 source 链。
                print_source_chain(&err);
            }
        }
    }

    // 演示 #[from]：io::Error 自动变成 ConfigError::Io，无需手写 From。
    let io_err = ConfigError::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "文件不存在",
    ));
    println!("\n#[from] 自动转换：{io_err}");
    // transparent 变体直接透传底层 Display。
    let other = AppError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "权限不足",
    ));
    println!("transparent 变体：{other}");

    println!("\n===== ② anyhow::Result 传播多种错误 + .context() =====");
    println!("同一个函数 load_config 会成功、也会因不同类型的原因失败：");

    println!("\n===== ③ 打印 {{:?}} 看错误链 =====");
    try_load("app.conf"); // 成功
    try_load("bad.conf"); // 解析失败
    try_load("missing.conf"); // 文件不存在

    explain_tradeoffs();

    // main 返回 anyhow::Result 时，Err 会以 {:?} 形式打印并返回非零退出码。
    // 本实验故意返回 ()，让上面所有失败都只是演示，程序正常退出。
}
