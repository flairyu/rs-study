//! 实验 07 · tracing + tracing-subscriber：结构化日志
//!
//! 四段演示：
//! 1. `tracing_subscriber::fmt().with_env_filter(...).init()` 初始化（`EnvFilter` 需要
//!    `tracing-subscriber` 的 `env-filter` feature）；
//! 2. 五个级别 + 结构化字段（`info!(user_id = 7, "登录")`）；
//! 3. `#[instrument]` 自动给函数套 span，看嵌套；
//! 4. `RUST_LOG` 控制级别，以及 `debug!` 默认为什么不打印。
//!
//! 运行：`RUST_LOG=debug cargo run -p l07_tracing`
//! 对照：`cargo run -p l07_tracing`（默认 info，debug 不打印）

use std::time::Duration;

use tracing::{debug, error, info, info_span, instrument, trace, warn, Level};
use tracing_subscriber::EnvFilter;

/// 业务函数：`#[instrument]` 会用函数名建一个 span，
/// 并把每个参数记录成同名字段（`password` 用 skip 排除，别把密码写进日志）。
#[instrument(skip(password))]
fn login(user: &str, password: &str) -> bool {
    info!("开始校验凭证"); // 这行自动落在 login 这个 span 里
    let ok = check_password(user, password);
    if ok {
        info!(roles = ?["user", "reader"], "登录成功");
    } else {
        warn!("登录失败，请检查用户名或密码");
    }
    ok
}

/// 被 login 调用的函数：它的 span 会**嵌套**在 login 的 span 下面。
/// 同样 `skip(password)` —— `#[instrument]` 默认会把**所有**参数记成字段，
/// 忘了 skip 就等于把密码写进日志（这行注释是故意留给你记住这个坑的）。
#[instrument(skip(password))]
fn check_password(user: &str, password: &str) -> bool {
    // 默认级别是 info，所以这行平时根本不会执行到「输出」那一步。
    debug!(
        pwd_len = password.len(),
        "进入密码校验（默认级别下你看不到我）"
    );
    std::thread::sleep(Duration::from_millis(5));
    !password.is_empty() && password != "wrong"
}

fn main() {
    println!("== 实验 07 · tracing 结构化日志 ==\n");

    // ---------- ① 初始化 ----------
    // try_from_default_env() 读 RUST_LOG，例如 RUST_LOG=debug、RUST_LOG=warn,my_crate=debug。
    // 读不到（没设置或格式非法）就退回 "info" —— 生产代码别 unwrap 掉。
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter) // 过滤器：决定哪些事件被记录
        .with_target(false) // 不打印模块路径，输出更短
        .with_level(true)
        .init(); // 设置全局默认 subscriber；**只能调用一次**，第二次会 panic

    let debug_on = tracing::enabled!(Level::DEBUG);
    println!("（提示：RUST_LOG = {:?}）", std::env::var("RUST_LOG").ok());
    println!("（提示：debug! 当前是否启用 = {debug_on}）\n");

    // ---------- ② 级别与结构化字段 ----------
    println!("── ② 五个级别 + 结构化字段 ──");
    info!("程序启动");
    info!(user_id = 7, user = "ada", "登录"); // key = value：结构化字段
    info!(elapsed_ms = 12u64, cache = "miss", "查询完成"); // 数字/字符串字段
    warn!(remaining = 3, limit = 5, "API 配额快用完了");
    error!(code = 500, retryable = false, "数据库连接失败");
    debug!(detail = "只有 RUST_LOG=debug 时才看得到这条"); // 默认级别下被过滤
    trace!("trace 比 debug 还细，几乎只在排查问题时临时打开");

    println!("  ↑ 上面每行日志都带级别、时间、字段；debug!/trace! 默认不出现\n");

    // ---------- ③ #[instrument] 的 span 嵌套 ----------
    println!("── ③ #[instrument]：函数即 span ──");
    let ok = login("ada", "s3cret");
    info!(ok = ok, "login() 返回");

    let fail = login("bob", "wrong");
    info!(ok = fail, "login() 返回");

    // 也可以手动开 span：适合「一次批量操作」这种跨函数的上下文。
    let span = info_span!("batch_import", rows = 3, source = "csv");
    {
        let _guard = span.enter(); // 守卫出作用域时自动退出 span
        for row in 1..=3 {
            info!(row, "导入一行");
        }
    }
    info!("span 已退出，之后的事件不再带 batch_import 字段");
    println!();

    // ---------- ④ 换一个 filter 看 debug ----------
    println!("── ④ debug! 默认不打印，是因为过滤器而不是宏 ──");
    debug!("外层 subscriber 是 info 级，所以这句被丢掉了");

    // with_default：临时在当前作用域换一个允许 debug 的 subscriber，
    // 用来演示「同一行 debug! 在过滤器放宽后就出现了」。
    let verbose = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::with_default(verbose, || {
        info!("（临时 subscriber）现在允许 debug");
        debug!("（临时 subscriber）这句 debug! 打印出来了");
        login("cyd", "hunter2"); // span 与字段照样工作
    });
    println!("  ↑ 同一个 debug! 宏，输出与否完全取决于 filter\n");

    println!("结论：");
    println!("  · 日志**级别**由 subscriber 的 filter 决定，宏只是提交事件（几乎零成本）；");
    println!("  · `RUST_LOG=debug cargo run -p l07_tracing` 就能看到上面所有 debug!；");
    println!("  · 生产里把 `RUST_LOG` 设成 `warn,my_crate=info` 这种「全局 + 单模块」写法最省事；");
    println!("  · 库代码只 `tracing::info!(...)` 提交事件，**不要**自己 init()，把选择权留给用你的 app。");
}
