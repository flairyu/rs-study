//! # 实验 01 · serde_json：结构体与 JSON 的双向转换
//!
//! 四段演示：
//! 1. 自定义 struct 派生 `Serialize`/`Deserialize`，并做「对象 → JSON → 对象」往返；
//! 2. `serde_json::Value`：结构未知/不稳定时动态取值；
//! 3. `Option<T>` 可选字段 + `#[serde(rename_all = "camelCase")]`；
//! 4. 字段类型不符时反序列化报错，用 `Result` 接住并打印诊断信息。
//!
//! 运行：`cargo run -p l01_serde_json`

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 演示用的用户模型。`serde` 的 derive 会在编译期生成
/// `Serialize`（对象 → JSON）和 `Deserialize`（JSON → 对象）两套代码。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: u64,
    name: String,
    /// `rename_all` 会把 Rust 侧的 `email_addr` 映射成 JSON 里的 `emailAddr`。
    email_addr: String,
    /// 如果 JSON 里没有这个 key，用 `Default::default()`（这里是 `None`）补上。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
    /// `None` 时干脆不出现在序列化结果里，输出更干净。
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
}

/// 带嵌套结构，用来演示「JSON 对象可以嵌套」。
#[derive(Debug, Serialize, Deserialize)]
struct Order {
    order_id: String,
    buyer: User,
    items: Vec<String>,
}

fn section(title: &str) {
    println!("\n===== {title} =====");
}

// ---------------------------------------------------------------------------
// ① 派生 Serialize / Deserialize 并往返
// ---------------------------------------------------------------------------
fn demo_roundtrip() {
    section("① 派生 Serialize/Deserialize 并往返");

    let user = User {
        id: 7,
        name: "Alice".to_string(),
        email_addr: "alice@example.com".to_string(),
        nickname: Some("Ali".to_string()),
        phone: Some("13800001111".to_string()),
    };

    println!("Rust 侧对象：{user:#?}");

    // 序列化：对象 → JSON 字符串。to_string_pretty 只是换了缩进，数据和 to_string 一样。
    let json = serde_json::to_string_pretty(&user).expect("User 可以序列化");
    println!("序列化结果（注意字段名变成了 camelCase）：\n{json}");

    // 反序列化：JSON 字符串 → 对象。
    let back: User = serde_json::from_str(&json).expect("刚才的 JSON 一定能反序列化回去");
    println!("反序列化结果：{back:#?}");
    // PartialEq 比的是 Rust 字段，不是 JSON 文本，所以能直接断言「往返没丢信息」。
    assert_eq!(user, back, "对象 → JSON → 对象 应当无损");
    println!("断言通过：user == back，往返无损。");

    // 嵌套结构同样自动处理，不需要手写解析代码。
    let order = Order {
        order_id: "A-1001".to_string(),
        buyer: user,
        items: vec!["键盘".to_string(), "显示器".to_string()],
    };
    let order_json = serde_json::to_string(&order).expect("Order 可以序列化");
    println!("嵌套对象（单行 JSON）：{order_json}");
}

// ---------------------------------------------------------------------------
// ② serde_json::Value：动态取值
// ---------------------------------------------------------------------------
fn demo_value() {
    section("② serde_json::Value 动态取值");

    // 这个 JSON 结构不固定：不同来源的接口字段可能对不上，
    // 先解析成 Value，再按需取值，比提前定义 struct 更灵活。
    let raw = r#"
        {
            "name": "Alice",
            "age": 30,
            "active": true,
            "tags": ["rust", "serde"],
            "address": { "city": "Hangzhou", "zip": "310000" }
        }
    "#;

    let value: Value = serde_json::from_str(raw).expect("这段 JSON 是合法的");
    println!("解析出的 Value：{value}");

    // as_str / as_u64 / as_bool：取不到对应类型会返回 None，而不是 panic。
    println!("as_str(\"name\")    = {:?}", value["name"].as_str());
    println!("as_u64(\"age\")     = {:?}", value["age"].as_u64());
    println!("as_bool(\"active\") = {:?}", value["active"].as_bool());

    // 数组用 as_array 拿到 &Vec<Value>，再逐个取值。
    let tags = value["tags"].as_array().expect("tags 是数组");
    println!("tags 共 {} 个：{:?}", tags.len(), tags);

    // 嵌套对象：一路用索引往下点。
    println!("address.city = {:?}", value["address"]["city"].as_str());

    // ⚠️ 用 [] 索引一个不存在的 key 不会报错，而是得到 Value::Null。
    let missing = &value["not_there"];
    println!("value[\"not_there\"] = {missing}（是 Null，不是报错）");
    // 想区分「没有这个 key」和「值就是 null」，用 .get()：
    println!("value.get(\"not_there\") = {:?}", value.get("not_there"));
    println!(
        "value[\"name\"].as_u64()（类型不符）= {:?}",
        value["name"].as_u64()
    );
}

// ---------------------------------------------------------------------------
// ③ Option<T> 与 rename_all
// ---------------------------------------------------------------------------
fn demo_optional_fields() {
    section("③ Option<T> 可选字段 + rename_all = camelCase");

    // 只有必填字段：nickname 有 #[serde(default)]，缺失也能反序列化成功。
    let raw = r#"{
        "id": 8,
        "name": "Bob",
        "emailAddr": "bob@example.com"
    }"#;

    let bob: User = serde_json::from_str(raw).expect("缺 nickname 时由 default 补 None");
    println!("缺可选字段也能解析：{bob:#?}");
    assert_eq!(bob.nickname, None);
    assert_eq!(bob.phone, None);
    // 注意 JSON 里写的是 emailAddr（camelCase），Rust 侧字段叫 email_addr。
    assert_eq!(bob.email_addr, "bob@example.com");

    // 两个 Option 都是 None 时，skip_serializing_if 让它们从输出里消失。
    let json = serde_json::to_string(&bob).expect("可以序列化");
    println!("两个可选字段都为空时序列化：{json}");
    println!("→ 输出里没有 nickname / phone，也依然是合法 JSON。");

    // 反过来：给 JSON 补上可选字段，也能读进来。
    let with_phone: User = serde_json::from_str(
        r#"{"id":9,"name":"Carol","emailAddr":"c@example.com","phone":"13900002222"}"#,
    )
    .expect("可选字段存在时正常读入");
    println!("带 phone 时：{with_phone:#?}");
}

// ---------------------------------------------------------------------------
// ④ 序列化/反序列化失败：类型不符的错误处理
// ---------------------------------------------------------------------------
fn demo_errors() {
    section("④ 类型不符时的错误处理");

    // id 应该是数字，这里给了字符串 "abc"：反序列化会失败。
    let bad = r#"{
        "id": "abc",
        "name": "Dave",
        "emailAddr": "dave@example.com"
    }"#;

    match serde_json::from_str::<User>(bad) {
        Ok(user) => println!("意外成功：{user:?}"),
        Err(err) => {
            // serde_json::Error 的 Display 会带上「第几行第几列 + 期望什么类型」。
            println!("反序列化失败，错误信息：{err}");
            println!("错误所在行：{}，列：{}", err.line(), err.column());
            println!("→ 生产代码应当把这样的 Err 往上传播，而不是 unwrap。");
        }
    }

    // 连 JSON 语法都不对：错误同样从 Result 里出来，不会 panic。
    let not_json = r#"{ "id": 1, }"#;
    match serde_json::from_str::<User>(not_json) {
        Ok(user) => println!("意外成功：{user:?}"),
        Err(err) => println!("语法不合法时的错误：{err}"),
    }

    // 顺带一提：序列化返回的也是 Result 而不是 String。
    // ⚠️ 一个反直觉的点：JSON 没有 NaN/Infinity 字面量，serde_json 的处理是
    // **静默写成 null**，而不是报错——所以「序列化成功」不等于「数据保真」。
    let half = serde_json::to_string(&1.5_f64);
    println!("序列化普通浮点数：{half:?}");
    let nan = serde_json::to_string(&f64::NAN).expect("这里并不会失败");
    println!("序列化 NaN 得到：{nan}（被写成 null，不是报错！）");
    let inf = serde_json::to_string(&f64::INFINITY).expect("这里并不会失败");
    println!("序列化 INFINITY 得到：{inf}");
    println!("→ 若这类值不能丢，要在自己的类型上拦截，别指望序列化帮你报错。");
}

fn main() {
    println!("实验 01 · serde / serde_json");
    println!("（`serde` 提供序列化框架，`serde_json` 提供 JSON 这个具体格式）");

    demo_roundtrip();
    demo_value();
    demo_optional_fields();
    demo_errors();
}
