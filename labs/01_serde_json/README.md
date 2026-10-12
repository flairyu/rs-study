# 实验 01 · serde / serde_json

> 目录名带序号只是为了排序；**包名**是 `l01_serde_json`（cargo 不允许包名以数字开头）。

## 它是什么，解决什么问题

`serde` 是 Rust 生态**统一的序列化 / 反序列化框架**：你在自己的类型上派生
`Serialize` / `Deserialize`，它就生成「类型 ↔ 数据格式」的转换代码；具体格式由
后端 crate 决定，`serde_json` 就是其中最常用的 JSON 后端。

解决的痛点：

- 手写 `json.dumps` 风格的代码要一字段一字段拼字符串，改字段就要改两处，还容易漏转义；
- 从 JSON 取值要自己判断类型、处理缺失字段，一处忘了就 panic；
- 换格式（JSON → YAML → MessagePack）不用重写结构体，只换后端 crate。

`serde` 的 derive 是**编译期**展开的，没有反射、没有运行时类型检查表，所以性能接近手写解析。

## 覆盖的四个点

| # | 演示 | 关键 API / 属性 |
|---|---|---|
| ① | struct 派生并往返 | `#[derive(Serialize, Deserialize)]`、`to_string_pretty`、`from_str` |
| ② | 动态取值 | `serde_json::Value`、`as_str` / `as_u64` / `as_array`、`.get()` |
| ③ | 可选字段与改名 | `Option<T>`、`#[serde(default)]`、`#[serde(rename_all = "camelCase")]`、`skip_serializing_if` |
| ④ | 失败处理 | `serde_json::Error`、`err.line()` / `err.column()`、`to_string` 也会返回 `Err` |

## 什么时候别用它

- **结构固定又只在程序内部传递**：直接定义 struct 就好，别先解析成 `Value` 再手工取值，
  那等于把编译期检查换成了运行时 `Option` 判断。
- **性能敏感的极简场景**：只要读一个数字时，`serde_json` 的依赖树和编译时间不划算；
  但**不要**为了省依赖去手写 JSON 解析器，格式的边界情况（转义、Unicode、指数）非常多。
- **超大文件 + 只读一小部分**：`from_str` / `to_string` 是全量的；
  流式场景要用 `StreamDeserializer` 之类的增量 API。
- **想要「字段名保持原样、不做任何映射」**：默认就是原样，别再加 `rename_all` 反而绕晕自己。
- **需要 JSON5 / 注释 / 尾逗号**：`serde_json` 是严格 JSON，不接受这些；换 `json5` 之类的 crate。

## 怎么跑

```bash
cd /home/yxb/code/rs-study/labs
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 本机 ~/.cargo 只读
cargo run -p l01_serde_json
```

## 预期输出（节选）

```text
实验 01 · serde / serde_json
（`serde` 提供序列化框架，`serde_json` 提供 JSON 这个具体格式）

===== ① 派生 Serialize/Deserialize 并往返 =====
Rust 侧对象：User {
    id: 7,
    name: "Alice",
    email_addr: "alice@example.com",
    nickname: Some("Ali"),
    phone: Some("13800001111"),
}
序列化结果（注意字段名变成了 camelCase）：
{
  "id": 7,
  "name": "Alice",
  "emailAddr": "alice@example.com",
  "nickname": "Ali",
  "phone": "13800001111"
}
...
断言通过：user == back，往返无损。

===== ② serde_json::Value 动态取值 =====
as_str("name")    = Some("Alice")
as_u64("age")     = Some(30)
value["not_there"] = null（是 Null，不是报错）

===== ③ Option<T> 可选字段 + rename_all = camelCase =====
缺可选字段也能解析：User { ... nickname: None, phone: None }
两个可选字段都为空时序列化：{"id":8,"name":"Bob","emailAddr":"bob@example.com"}

===== ④ 类型不符时的错误处理 =====
反序列化失败，错误信息：invalid type: string "abc", expected u64 at line 2 column 19
错误所在行：2，列：19
语法不合法时的错误：trailing comma at line 1 column 12
序列化普通浮点数：Ok("1.5")
序列化 NaN 得到：null（被写成 null，不是报错！）
```

## 常见坑

- `#[serde(rename_all = "camelCase")]` 同时作用于**序列化和反序列化**，所以手工写的
  JSON 也必须用 camelCase 的 key，写 `email_addr` 会报「missing field」。
- 用 `value["key"]` 索引不存在的字段**不会报错**，得到的是 `Value::Null`；
  想区分「key 不存在」和「值就是 null」，要用 `value.get("key")`。
- 反序列化失败给的是 `Result`，不是 panic——`unwrap()` 是你在主动把它变成 panic。
- **`Option<T>` 反序列化时允许 key 缺失，但序列化时默认写成 `null`**；
  想让 `null` 从输出里消失，加 `#[serde(skip_serializing_if = "Option::is_none")]`。
- **JSON 没有 NaN / Infinity 字面量**，而 `serde_json` 会把它们**静默写成 `null`**
  而不是报错——「序列化成功」不等于「数据保真」。
