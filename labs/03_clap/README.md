# 实验 03 · clap（derive 风格命令行）

> 目录名带序号只是为了排序；**包名**是 `l03_clap`（cargo 不允许包名以数字开头）。

## 它是什么，解决什么问题

`clap` 是 Rust 最常用的命令行解析库。**derive 风格**指你不用手工构建解析器，
而是定义一个普通 struct，用属性标注每个字段对应哪种参数：

```rust
#[derive(Parser)]
struct Cli {
    input: Option<String>,                  // 位置参数
    #[arg(short, long)] verbose: bool,      // -v / --verbose
    #[arg(short = 'n', long, default_value_t = 1)] count: u32,
    #[arg(long, value_enum, default_value_t = Format::Text)] format: Format,
    #[command(subcommand)] command: Option<Command>,
}
```

它解决的痛点：手写 `std::env::args()` 要自己处理 `-n3` / `-n 3` / `--count=3`、
`--help`、`--version`、非法值的报错文案、以及 `--` 之后的位置参数；
clap 全部代劳，并且**参数定义和帮助文本是同一处**，不会写文档写漏。

## 覆盖的六个点

| # | 演示 | 写法 |
|---|---|---|
| ① | 位置参数 | `input: Option<String>` + `value_name` |
| ② | 布尔开关 | `#[arg(short, long)] verbose: bool` |
| ③ | 带值短选项 | `#[arg(short = 'n', long, default_value_t = 1)] count: u32` |
| ④ | 可选参数 | `output: Option<String>` |
| ⑤ | 枚举取值 | `#[arg(long, value_enum)] format: Format`（自动校验并列出候选值） |
| ⑥ | 子命令 | `#[command(subcommand)] command: Option<Command>`，含 `add` / `list` |

另外 `///` 文档注释会被 clap 提取成 `--help` 文本——**注释就是帮助文档**。

## 不带子命令时走哪条路？

`command` 声明为 `Option<Command>`，所以 `cargo run -p l03_clap`（不带任何参数）
**不会报错**，而是走 `None` 分支：打印一段说明，再用
`Cli::command().print_help()` 自动输出完整帮助。

这是有意设计的，因为常见的「工具不带参数」有两种合理语义：

- **报错**：把 `command: Command`（不是 `Option`）——不带子命令 clap 直接退出码 2 并提示用法。
  适合「必须指定动作」的工具（如 `git` 裸跑只打印帮助）。
- **默认动作**：用 `Option<Command>` + `default_value` 或 `None` 分支，
  适合「不带参数就做最常用的事」（如 `ls`）。

本实验选第二种，把两种路径的差异留在代码注释里，方便对照。

## 什么时候别用它

- **只有一两个固定参数**：`std::env::args()` 或 `argh` 这类更轻的库就够，
  clap 会带来可观的编译时间（derive + 大量泛型）。
- **需要和子命令共享大量状态**：clap 只负责解析；配置合并、层级覆盖要自己写。
- **要解析的不是命令行**（配置文件、环境变量）：用 `figment` / `config` 之类的库。
- **追求极小二进制**：clap 体积不小，空间受限场景考虑手写解析。
- **运行时动态生成参数**：derive 是静态的，动态场景要用 clap 的 builder API（`Command::new`）。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 本机 ~/.cargo 只读

cargo run -p l03_clap -- --help
cargo run -p l03_clap                                  # 不带子命令：默认路径
cargo run -p l03_clap -- -v -n 3 --tag rust --tag cli --format json item.txt
cargo run -p l03_clap -- add --priority high 买牛奶
cargo run -p l03_clap -- list --all -n 5
```

> ⚠️ `cargo run` 会吃掉第一个 `--` 之前的参数，所以给程序传参**必须**写
> `cargo run -p l03_clap -- --help`，那个 `--` 是 cargo 的分隔符。

## 预期输出

`cargo run -p l03_clap -- --help`（clap 自动生成，节选）：

```text
clap derive 风格演示：位置参数 / 开关 / 可选参数 / 枚举 / 子命令

Usage: l03_clap [OPTIONS] [FILE] [COMMAND]

Commands:
  add   添加一条记录
  list  列出记录
  help  Print this message or the help of the given subcommand(s)

Arguments:
  [FILE]  位置参数：本程序只接受一个，可有可无

Options:
  -v, --verbose             布尔开关：出现即为 true，可重复出现
  -n, --count <COUNT>       带值的短选项：`-n 3` 或 `-n3` 都行 [default: 1]
      --tag <TAG>           可重复的选项，收集成 Vec；一次都不给就是空 Vec
      --output <OUTPUT>     可选参数：给了就是 Some，不给就是 None
      --format <FORMAT>     枚举取值，clap 会自动校验并列出可选值 [default: text]
                             [possible values: text, json, yaml]
  -h, --help                Print help
  -V, --version             Print version
```

`cargo run -p l03_clap -- add --priority high 买牛奶`（节选）：

```text
实验 03 · clap（derive 风格 CLI）

===== ① 顶层参数解析结果 =====
Cli {
    input: None,
    verbose: false,
    count: 1,
    tags: [],
    output: None,
    format: Text,
    command: Some(
        Add {
            title: "买牛奶",
            priority: High,
            due: None,
        },
    ),
}

===== ③ 子命令 =====
执行子命令 add：
  title    = "买牛奶"
  priority = High
  due      = None
```

完全不带参数时（`argc == 1`，节选）：

```text
===== ④ 没有给子命令：走默认路径 =====
你只运行了 `l03_clap`，没有给任何参数。
可用子命令：add / list。下面自动打印完整帮助：
...
想跳过这段帮助：`cargo run -p l03_clap -- add 买牛奶`。
```

只给了顶层参数、没给子命令时（节选）——只给一句提示，不刷整屏帮助：

```text
===== ④ 没有给子命令：走默认路径 =====
你给了顶层参数，但没有指定子命令，所以没有实际动作。
加上子命令即可：`add <标题>` 或 `list`。
例如：cargo run -p l03_clap -- add --priority high 买牛奶
```

## 常见坑

- 忘写 `--`：`cargo run -p l03_clap --help` 会把 `--help` 交给 **cargo**，
  打印的是 cargo 的帮助。
- `--version` 需要 `#[command(version)]`；不写的话 `-V` 不存在。
- `#[arg(short)]` 取字段名首字母；字段名以非字母开头或想换字母，要写 `short = 'x'`。
- `bool` 字段默认是**开关**（出现即 true），不要给它 `default_value_t`，
  否则会变成「必须带值」的选项。
- 子命令的 `-n` 和顶层的 `-n` 是两个独立参数，互不影响；
  想让顶层选项在子命令后也能用，要标 `#[arg(global = true)]`。
