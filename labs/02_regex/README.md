# 实验 02 · regex

> 目录名带序号只是为了排序；**包名**是 `l02_regex`（cargo 不允许包名以数字开头）。

## 它是什么，解决什么问题

`regex` 提供**正则表达式**的编译与匹配。它的实现基于有限自动机，
**保证线性时间**：不会像 PCRE / Python `re` 那样因为嵌套量词出现灾难性回溯
（ReDoS）。代价是**不支持反向引用和环视**（`(?=...)`、`\1`）。

解决的痛点：手写字符串查找 / 切分在「多个分隔符、可选出错、大小写」面前很快失控；
正则把这些规则写成一行模式，比一串 `split` + `starts_with` 更好读、更好改。

## 覆盖的四个点

| # | 演示 | 关键 API |
|---|---|---|
| ① | 基本匹配 | `Regex::new`、`is_match`、`find`、`find_iter`、`Match::start/end` |
| ② | 捕获组解析日志 | `captures`、命名组 `(?P<name>...)`、`caps["name"]`、`caps.name()` 得到 `Option` |
| ③ | 手机号脱敏 | `replace_all`、`${1}` 反向引用、返回 `Cow<str>` |
| ④ | 性能坑 | 循环里 `Regex::new` vs 编译一次复用，用 `std::time::Instant` 实测 |

## 什么时候别用它

- **只是找固定子串**：`str::contains` / `str::find` 更快、更直观，也没有依赖。
- **要解析结构化格式**（JSON、TOML、日志模板）：用正经 parser，正则一改就崩。
- **需要反向引用 / 环视 / 递归匹配**：`regex` 故意不支持，换 `fancy-regex` 或 `pcre2`，
  但要自己承担回溯风险。
- **模式由不可信用户输入**：即使线性时间，也存在「模式太复杂导致编译慢」的问题，
  应当限制模式长度，或用 `RegexBuilder::size_limit`。
- **一次性只匹配一个字符**：`char::is_ascii_digit` 之类的标准库 API 就够了。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 本机 ~/.cargo 只读
cargo run -p l02_regex
```

## 预期输出（节选，耗时数字随机器变化）

```text
实验 02 · regex
（Rust 的 regex crate 保证线性时间匹配，不存在回溯爆炸）

===== ① Regex::new + is_match / find =====
模式：^\d{3}-\d{4}$
is_match("555-1234") = true
is_match("555-12345") = false
...
第一次匹配："rust"，字节区间 7..11
find_iter 找到全部：["rust", "rusty"]

===== ② captures 捕获组：解析日志行 =====
OK   2026-01-02 03:04:05 [INFO] 服务已启动
OK   2026-01-02 03:04:06 [WARN] 磁盘使用率 91%
OK   2026-01-02 03:04:07 [ERROR] 连接数据库失败: timeout
SKIP "这行不是日志，应当被跳过"（不匹配，跳过）
addr=example.com:8080   host=example.com  port=Some("8080")
addr=example.com        host=example.com  port=None

===== ③ replace_all 脱敏手机号 =====
原文：联系人：张三 13812345678，备用 15900001111，座机 0571-88886666。
脱敏：联系人：张三 138****5678，备用 159****1111，座机 0571-88886666。
...
一共匹配到 2 个手机号。

===== ④ 性能坑：在循环里 Regex::new vs 编译一次复用 =====
输入：2000 行日志，同一个模式
循环里反复 Regex::new ：   123.456ms（匹配到 2000 行）
编译一次后复用       ：   789.012µs（匹配到 2000 行）
两种写法结果一致：true
编译一次大约快 156 倍（具体倍数随机器波动，量级稳定）。
```

## 常见坑

- **别在循环里编译正则**。`Regex::new` 要解析模式并构建自动机，
  单次几十微秒，但乘以一万次就是几百毫秒。放到循环外，或全局缓存
  （`std::sync::OnceLock` 是标准库方案，不需要额外依赖）。
- **`\d` 默认只匹配 ASCII 数字**；要匹配 Unicode 数字得显式开 `(?u)` 或用 `\p{Nd}`。
- **`find` 返回的是字节区间**，不是字符下标。切 `&str` 时直接用 `m.as_str()` / `m.range()`，
  自己在 `str` 上按字节切会切碎多字节 UTF-8 而 panic。
- **`captures` 里的可选组取到的是 `Option`**：结构固定用 `&caps["x"]` 方便，
  结构可选必须用 `caps.name("x")` 再判空。
- **`$1` 后面紧跟数字要写成 `${1}`**，否则 `$12` 会被当成第 12 个组。
- **`^`/`$` 默认匹配「整段文本的开头/结尾」**，多行字符串想按行匹配要加 `(?m)`。
