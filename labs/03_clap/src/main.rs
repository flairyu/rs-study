//! # 实验 03 · clap（derive 风格）命令行解析
//!
//! 覆盖：位置参数、`--flag` 布尔开关、`-n <count>`、`Option<T>` 可选参数、
//! `#[arg(value_enum)]` 枚举取值、以及 `add` / `list` 两个子命令。
//!
//! 本程序不依赖任何真实数据，只把解析结果打印出来，方便观察 clap 的行为。
//!
//! 运行：
//! ```bash
//! cargo run -p l03_clap -- --help
//! cargo run -p l03_clap                            # 不带子命令，走默认路径
//! cargo run -p l03_clap -- -v -n 3 --tag rust --tag cli --format json item.txt
//! cargo run -p l03_clap -- add --priority high 买牛奶
//! cargo run -p l03_clap -- list --all
//! ```

use clap::{Parser, Subcommand, ValueEnum};

/// 顶层参数。`#[derive(Parser)]` 会生成 `parse()`，并从文档注释
/// （`///`）里提取 `--help` 文本——所以注释不是可选项，它是帮助文档。
#[derive(Debug, Parser)]
#[command(
    name = "l03_clap",
    version,
    about = "clap derive 风格演示：位置参数 / 开关 / 可选参数 / 枚举 / 子命令",
    long_about = None
)]
struct Cli {
    /// 位置参数：本程序只接受一个，可有可无
    #[arg(value_name = "FILE")]
    input: Option<String>,

    /// 布尔开关：出现即为 true，可重复出现
    #[arg(short, long)]
    verbose: bool,

    /// 带值的短选项：`-n 3` 或 `-n3` 都行
    #[arg(short = 'n', long, default_value_t = 1)]
    count: u32,

    /// 可重复的选项，收集成 Vec；一次都不给就是空 Vec
    #[arg(long = "tag", value_name = "TAG")]
    tags: Vec<String>,

    /// 可选参数：给了就是 Some，不给就是 None
    #[arg(long)]
    output: Option<String>,

    /// 枚举取值，clap 会自动校验并列出可选值
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,

    /// 子命令，可选；不写就落到 None，由 main 打印用法提示
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
    Yaml,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 添加一条记录
    Add {
        /// 记录内容（必填位置参数）
        title: String,

        /// 优先级：同样是 value_enum
        #[arg(long, value_enum, default_value_t = Priority::Normal)]
        priority: Priority,

        /// 截止日期，可选
        #[arg(long, value_name = "DATE")]
        due: Option<String>,
    },

    /// 列出记录
    List {
        /// 显示全部（含已完成）
        #[arg(long)]
        all: bool,

        /// 只显示前 N 条
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Priority {
    Low,
    Normal,
    High,
}

fn section(title: &str) {
    println!("\n===== {title} =====");
}

fn main() {
    println!("实验 03 · clap（derive 风格 CLI）");

    // parse() 内部会处理 --help / --version / 参数错误：
    // 该打印帮助就直接打印并 exit(0)，参数非法就打印错误并 exit(2)。
    // 所以走到下面的代码时，参数一定是合法的。
    let cli = Cli::parse();

    section("① 顶层参数解析结果");
    // 用 {:#?} 打印整个结构体，最直观地看到「哪一项是 None、哪一项是空 Vec」。
    println!("{cli:#?}");

    section("② 逐项解读");
    println!(
        "位置参数 input  = {:?}（Option<String>：没给就是 None）",
        cli.input
    );
    println!("开关 --verbose/-v = {}（bool：出现即 true）", cli.verbose);
    println!(
        "选项 -n/--count   = {}（有默认值，所以这里永远有值）",
        cli.count
    );
    println!(
        "可重复 --tag      = {:?}（Vec<String>：给几次就有几个）",
        cli.tags
    );
    println!(
        "可选参数 --output = {:?}（Option<String>：这是「可选的带值参数」）",
        cli.output
    );
    println!("枚举 --format     = {:?}", cli.format);

    // 枚举是普通 Rust 枚举，用 match 分支处理即可。
    match cli.format {
        Format::Text => println!("→ format=text：按纯文本输出"),
        Format::Json => println!("→ format=json：按 JSON 输出"),
        Format::Yaml => println!("→ format=yaml：按 YAML 输出"),
    }

    section("③ 子命令");
    match &cli.command {
        Some(Command::Add {
            title,
            priority,
            due,
        }) => {
            println!("执行子命令 add：");
            println!("  title    = {title:?}");
            println!("  priority = {priority:?}");
            println!("  due      = {due:?}");
            println!("→ 真实程序在这里把记录写进存储，本实验只打印。");
        }
        Some(Command::List { all, limit }) => {
            println!("执行子命令 list：");
            println!("  all   = {all}");
            println!("  limit = {limit}");
            println!("→ 真实程序在这里查存储并输出，本实验只打印。");
        }
        None => {
            // 子命令声明为 Option<Command>，所以「不带子命令」是合法输入，
            // 不会报错。这里走默认路径，分两种情况：
            section("④ 没有给子命令：走默认路径");
            // argc == 1 表示用户只敲了程序名，什么都没给。
            let bare_invocation = std::env::args().count() == 1;
            if bare_invocation {
                // 完全裸跑：打印完整帮助，让用户知道能做什么。
                println!("你只运行了 `l03_clap`，没有给任何参数。");
                println!("可用子命令：add / list。下面自动打印完整帮助：\n");
                // Cli::command() 来自 clap 的 CommandFactory，可手动生成帮助文本。
                use clap::CommandFactory;
                Cli::command().print_help().expect("输出帮助到 stdout");
                println!("\n想跳过这段帮助：`cargo run -p l03_clap -- add 买牛奶`。");
            } else {
                // 给了顶层参数但没给子命令：只给一句提示，不刷整屏帮助。
                println!("你给了顶层参数，但没有指定子命令，所以没有实际动作。");
                println!("加上子命令即可：`add <标题>` 或 `list`。");
                println!("例如：cargo run -p l03_clap -- add --priority high 买牛奶");
            }
        }
    }

    println!("\n提示：`cargo run -p l03_clap -- --help` 是 clap 自动生成的帮助。");
}
