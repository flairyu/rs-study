//! # 实验 02 · regex：匹配、捕获组、替换与性能陷阱
//!
//! 四段演示：
//! 1. `Regex::new` + `is_match` / `find`：最基本的匹配与定位；
//! 2. `captures` 捕获组：把一行日志拆成日期 / 级别 / 消息；
//! 3. `replace_all` + `$1` 反向引用：手机号脱敏；
//! 4. **性能坑**：在循环里反复 `Regex::new` 很慢，编译一次复用才快
//!    —— 用 `std::time::Instant` 实测两者耗时并打印对比。
//!
//! 运行：`cargo run -p l02_regex`

use regex::Regex;
use std::time::Instant;

fn section(title: &str) {
    println!("\n===== {title} =====");
}

// ---------------------------------------------------------------------------
// ① 基本匹配：is_match / find
// ---------------------------------------------------------------------------
fn demo_basic_match() {
    section("① Regex::new + is_match / find");

    // Regex::new 编译失败会返回 Err（比如括号不配对），所以必须处理 Result。
    let re = Regex::new(r"^\d{3}-\d{4}$").expect("这个模式是合法的");
    println!("模式：{}", re.as_str());

    for input in ["555-1234", "555-12345", "abc-1234", "555-1234 "] {
        // is_match 只回答「有没有」，不关心位置；^...$ 锚定整串。
        println!("is_match({input:?}) = {}", re.is_match(input));
    }

    // 换个不锚定的模式，看 find 返回的位置。
    let word = Regex::new(r"\brust\w*").expect("模式合法");
    let text = "I like rust, rusty tools and Rustacean.";
    println!("\n在 {text:?} 中查找 /\\brust\\w*/：");
    // find 只返回第一个匹配；要全部匹配用 find_iter。
    match word.find(text) {
        Some(m) => println!(
            "第一次匹配：{:?}，字节区间 {}..{}",
            m.as_str(),
            m.start(),
            m.end()
        ),
        None => println!("没有匹配"),
    }
    let all: Vec<&str> = word.find_iter(text).map(|m| m.as_str()).collect();
    println!("find_iter 找到全部：{all:?}");
    println!("注意：Rustacean 没被匹配——regex 默认区分大小写，它开头是大写 R。");
    println!("想忽略大小写用 RegexBuilder 的 case_insensitive(true)，别手工转小写。");
}

// ---------------------------------------------------------------------------
// ② 捕获组
// ---------------------------------------------------------------------------
fn demo_captures() {
    section("② captures 捕获组：解析日志行");

    // 每个 (...) 是一个捕获组，用名字取比用下标 1/2/3 更不容易错。
    let log_re = Regex::new(
        r"^(?P<date>\d{4}-\d{2}-\d{2})[ T](?P<time>\d{2}:\d{2}:\d{2})\s+(?P<level>[A-Z]+)\s+(?P<msg>.+)$",
    )
    .expect("模式合法");

    let lines = [
        "2026-01-02 03:04:05 INFO  服务已启动",
        "2026-01-02T03:04:06 WARN  磁盘使用率 91%",
        "2026-01-02 03:04:07 ERROR 连接数据库失败: timeout",
        "这行不是日志，应当被跳过",
    ];

    for line in lines {
        // captures 返回 Option<Captures>；不匹配就是 None，不会 panic。
        match log_re.captures(line) {
            Some(caps) => {
                // 命名组用 &caps["name"]，等价于 caps.name("name").unwrap().as_str()。
                let date = &caps["date"];
                let time = &caps["time"];
                let level = &caps["level"];
                let msg = &caps["msg"];
                println!("OK   {date} {time} [{level}] {msg}");
            }
            None => println!("SKIP {line:?}（不匹配，跳过）"),
        }
    }

    // 可选捕获组：用 `(?::(?P<port>\d+))?` 让端口部分整体可选，取的时候得到 Option。
    let with_port = Regex::new(r"^(?P<host>[^:]+)(?::(?P<port>\d+))?$").expect("模式合法");
    for addr in ["example.com:8080", "example.com"] {
        let caps = with_port.captures(addr).expect("总能匹配 host 部分");
        // caps.name("port") 返回 Option<Match>；这一段不存在时就是 None。
        let port = caps.name("port").map(|m| m.as_str());
        println!("addr={addr:<18} host={:<12} port={port:?}", &caps["host"]);
    }
}

// ---------------------------------------------------------------------------
// ③ replace_all 脱敏
// ---------------------------------------------------------------------------
fn demo_replace() {
    section("③ replace_all 脱敏手机号");

    // 中国大陆手机号：1 开头，第二位 3-9，共 11 位；\b 保证不会切到别的数字中间。
    let phone = Regex::new(r"\b(1[3-9]\d)(\d{4})(\d{4})\b").expect("模式合法");

    let text = "联系人：张三 13812345678，备用 15900001111，座机 0571-88886666。";
    println!("原文：{text}");

    // $1/$2/$3 是反向引用；用 ${1} 明确边界，避免 $12 被当成第 12 个组。
    let masked = phone.replace_all(text, "${1}****${3}");
    println!("脱敏：{masked}");

    // replace_all 返回 Cow<str>：没有匹配时直接借用原串，不做额外分配。
    let no_hit = phone.replace_all("这里没有手机号", "${1}****${3}");
    println!("无匹配时：{no_hit}（此时它借用原字符串，未发生拷贝）");

    // 统计匹配个数：find_iter 比手工数 replace 的返回值省事。
    println!("一共匹配到 {} 个手机号。", phone.find_iter(text).count());
    println!("注意座机 0571-88886666 没被动：它不符合手机号的 1[3-9] 开头规则。");
}

// ---------------------------------------------------------------------------
// ④ 性能坑：别在循环里编译正则
// ---------------------------------------------------------------------------
fn demo_perf() {
    section("④ 性能坑：在循环里 Regex::new vs 编译一次复用");

    // 输入规模刻意取小一点：错误写法在 2000 行时会让程序跑好几秒，
    // 500 行已经足够看出数量级差异。
    let lines: Vec<String> = (0..500)
        .map(|i| format!("2026-01-02 03:04:05 INFO  第 {i} 条日志，手机号 13812345678"))
        .collect();
    let pattern = r"^(?P<date>\d{4}-\d{2}-\d{2})[ T](?P<time>\d{2}:\d{2}:\d{2})\s+(?P<level>[A-Z]+)\s+(?P<msg>.+)$";

    // —— 错误示范：每一轮都 Regex::new ——
    // 正则要先解析模式、再编译成自动机，这个开销远大于匹配本身。
    let start = Instant::now();
    let mut hits_bad = 0usize;
    for line in &lines {
        let re = Regex::new(pattern).expect("模式合法"); // ❌ 每次循环都编译
        if re.is_match(line) {
            hits_bad += 1;
        }
    }
    let bad = start.elapsed();

    // —— 正确做法：编译一次，循环里只复用 ——
    // 匹配本身几乎不耗时，耗时几乎全在编译上。
    let start = Instant::now();
    let re = Regex::new(pattern).expect("模式合法"); // ✅ 只编译一次
    let mut hits_good = 0usize;
    for line in &lines {
        if re.is_match(line) {
            hits_good += 1;
        }
    }
    let good = start.elapsed();

    println!("输入：{} 行日志，同一个模式", lines.len());
    println!("循环里反复 Regex::new ：{bad:>10.3?}（匹配到 {hits_bad} 行）");
    println!("编译一次后复用       ：{good:>10.3?}（匹配到 {hits_good} 行）");
    println!("两种写法结果一致：{}", hits_bad == hits_good);
    // 只打印倍数，不做「必须大于 N 倍」的断言，免得在不同机器上偶发失败。
    let ratio = bad.as_secs_f64() / good.as_secs_f64().max(f64::EPSILON);
    println!("编译一次大约快 {ratio:.0} 倍（具体倍数随机器波动，量级稳定）。");

    println!("\n结论：");
    println!("· 正则对象只应编译一次：放到循环外，或用 OnceLock / LazyLock 做全局缓存。");
    println!("· 单次编译大约几十微秒，但在成千上万次循环里它会支配总耗时。");
    println!("· 真正的匹配（is_match / find）本身非常快。");
}

fn main() {
    println!("实验 02 · regex");
    println!("（Rust 的 regex crate 保证线性时间匹配，不存在回溯爆炸）");

    demo_basic_match();
    demo_captures();
    demo_replace();
    demo_perf();
}
