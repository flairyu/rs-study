//! 实验 08 · reqwest + tokio：异步 HTTP 客户端
//!
//! 四段演示：
//! 1. `#[tokio::main]` 里 GET 公开 API，并统一设置 `.timeout()`；
//! 2. 把 JSON 反序列化到自定义 struct（只声明需要的字段）；
//! 3. POST JSON：`.json(&payload)` 自动设 Content-Type 并序列化；
//! 4. 错误处理：超时 / 连接失败 / HTTP 非 2xx —— 全部打印错误链，绝不用 unwrap 糊过去。
//!
//! 运行（**需要联网**）：`cargo run -p l08_reqwest`
//! 网络不通时程序不会崩：每个分支都把错误打印出来，然后继续/正常结束。

use std::error::Error as StdError;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// GitHub 仓库信息：**只声明我们关心的字段**，其余字段 serde 默认忽略。
/// 这就是「反序列化到自定义 struct」相对 `serde_json::Value` 的价值：
/// 类型明确、字段名写错立刻编译/运行报错，而不是等到用时才发现是 null。
#[derive(Debug, Deserialize)]
struct Repo {
    full_name: String,
    description: Option<String>,
    stargazers_count: u64,
    language: Option<String>,
    html_url: String,
    archived: bool,
}

/// POST 上去的负载 —— 用同一个 struct 同时 derive Serialize 最省心。
#[derive(Debug, Serialize)]
struct Feedback {
    project: String,
    stars: u64,
    comment: String,
}

const GET_URL: &str = "https://api.github.com/repos/rust-lang/rust";
const POST_URL: &str = "https://httpbin.org/post";
const MISSING_URL: &str = "https://api.github.com/repos/rust-lang/definitely-not-a-repo-xyz";

/// 把 `reqwest::Error` 翻译成「哪一类失败 + 完整错误链」。
///
/// 这一节的重点：`reqwest::Error` 一个类型盖住了超时、DNS、TLS、状态码、解码……
/// 靠 `is_timeout()` / `is_connect()` / `is_status()` / `is_decode()` 分类，
/// 而不是把 `{:?}` 打出来让读日志的人猜。
fn explain(err: &reqwest::Error) -> String {
    let kind = if err.is_timeout() {
        "超时 (timeout)"
    } else if err.is_connect() {
        "连接失败 (connect：DNS/TCP/TLS)"
    } else if err.is_status() {
        "HTTP 状态码非 2xx (status)"
    } else if err.is_decode() {
        "响应解码失败 (decode：JSON 对不上 struct)"
    } else if err.is_body() {
        "响应体读取失败 (body)"
    } else if err.is_request() {
        "请求发送失败 (request)"
    } else {
        "其他错误"
    };

    let mut chain = vec![err.to_string()];
    let mut source = err.source();
    while let Some(cause) = source {
        chain.push(cause.to_string());
        source = cause.source();
    }
    format!("{kind} => {}", chain.join("  <-  "))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    println!("== 实验 08 · reqwest 异步 HTTP ==\n");

    // 一个 Client 复用连接池（每个请求都 Client::new() 会丢掉连接复用）。
    // .timeout() 在这里设一次，就是**所有**请求的默认总超时。
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("rs-study-labs/0.1 (learning project)") // GitHub API 要求带 UA
        .build()?; // 构造失败通常是 TLS 后端问题，直接上报，继续跑也没意义
    println!("(1) Client 建好了：全局超时 10s，带 User-Agent\n");

    // ---------- ① GET 原始响应 ----------
    println!("── ① GET（看状态码与原始文本）──");
    println!("  GET {GET_URL}");
    demo_get_raw(&client).await;

    // ---------- ② GET + 反序列化 ----------
    println!("── ② GET + 反序列化到自定义 struct ──");
    match fetch_repo(&client).await {
        Ok(repo) => {
            println!("  仓库全名 : {}", repo.full_name);
            println!("  star 数  : {}", repo.stargazers_count);
            println!(
                "  主语言   : {}",
                repo.language.as_deref().unwrap_or("(未知)")
            );
            println!(
                "  简介     : {}",
                repo.description.as_deref().unwrap_or("(无简介)")
            );
            println!("  归档     : {}", repo.archived);
            println!("  主页     : {}", repo.html_url);
            println!("  （struct 里没写的字段自动忽略，网络流量照样收，只是不解析）");
        }
        // 注意：即使这里失败，程序也只是打印，然后继续演示后面的部分。
        Err(e) => println!("  请求失败: {}", explain(&e)),
    }
    println!();

    // ---------- ③ POST JSON ----------
    println!("── ③ POST JSON（.json(&payload)）──");
    demo_post_json(&client).await;
    println!();

    // ---------- ④ 三类失败 ----------
    println!("── ④ 错误处理：超时 / 连接失败 / 非 2xx ──");
    demo_errors(&client).await;

    println!("\n小结：reqwest 的错误要**分类处理**（超时该重试、4xx 不该重试、5xx 可退避重试）；");
    println!("      `.error_for_status()` 是最省事的「把 4xx/5xx 变成 Err」，");
    println!("      需要读错误响应体时再手动查 `resp.status().is_success()`。");
    Ok(())
}

/// ① 最朴素的一步：拿到响应，打印状态码和前 200 个字符。
async fn demo_get_raw(client: &reqwest::Client) {
    match client.get(GET_URL).send().await {
        Ok(resp) => {
            let status = resp.status();
            println!("  HTTP {status}（is_success = {}）", status.is_success());
            match resp.text().await {
                Ok(body) => {
                    let snippet: String = body.chars().take(200).collect();
                    println!("  前 200 字符: {snippet} …");
                }
                Err(e) => println!("  读取响应体失败: {}", explain(&e)),
            }
        }
        Err(e) => println!("  请求失败: {}", explain(&e)),
    }
}

/// ② 反序列化：`error_for_status()` 先把 4xx/5xx 变成 Err，再 `.json::<Repo>()`。
async fn fetch_repo(client: &reqwest::Client) -> Result<Repo, reqwest::Error> {
    client
        .get(GET_URL)
        .send()
        .await?
        .error_for_status()? // 非 2xx 就别硬解析了
        .json::<Repo>() // 只认需要的字段；类型对不上 -> is_decode()
        .await
}

/// ③ POST JSON：`.json(&payload)` 会设置 `Content-Type: application/json` 并序列化。
async fn demo_post_json(client: &reqwest::Client) {
    let payload = Feedback {
        project: "rs-study labs".to_string(),
        stars: 108_000,
        comment: "hello from reqwest".to_string(),
    };
    println!("  POST {POST_URL}");
    println!(
        "  发送: {}",
        serde_json::to_string(&payload).unwrap_or_default()
    );

    let result: Result<serde_json::Value, reqwest::Error> = async {
        let resp = client
            .post(POST_URL)
            .json(&payload) // 自动序列化 + 设置 Content-Type
            .send()
            .await?
            .error_for_status()?;
        resp.json::<serde_json::Value>().await
    }
    .await;

    match result {
        Ok(value) => {
            // httpbin 会把收到的 JSON 原样放在 "json" 字段里。
            let echoed = value.get("json").cloned().unwrap_or(value);
            match serde_json::to_string_pretty(&echoed) {
                Ok(pretty) => {
                    for line in pretty.lines() {
                        println!("  {line}");
                    }
                }
                Err(e) => println!("  回显没法格式化: {e}"),
            }
        }
        Err(e) => println!("  请求失败: {}", explain(&e)),
    }
}

/// ④ 三种典型失败，全部走 `Result`，一次都不 unwrap。
async fn demo_errors(client: &reqwest::Client) {
    // (a) 超时：给保留网段 192.0.2.0/24（TEST-NET-1，永远不可路由）一个 500ms 预算。
    //     用这么小的超时是为了「确定性」：最多等 0.5 秒就返回，不会把演示卡住。
    println!("  (a) 超时: GET http://192.0.2.1:81/ ，超时设 500ms");
    match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(impatient) => match impatient.get("http://192.0.2.1:81/").send().await {
            Ok(resp) => println!("      居然连上了？HTTP {}", resp.status()),
            Err(e) => println!("      {}", explain(&e)),
        },
        Err(e) => println!("      Client 构造失败: {e}"),
    }
    println!("      → 超时通常是「网络不可达/服务卡住」，值得重试，但要设上限（如指数退避）");

    // (b) 连接失败：本机 9 端口（历史上是 discard 服务）几乎肯定没有进程在监听。
    println!("  (b) 连接失败: GET http://127.0.0.1:9/");
    match client.get("http://127.0.0.1:9/").send().await {
        Ok(resp) => println!("      本机 9 端口居然有服务？HTTP {}", resp.status()),
        Err(e) => println!("      {}", explain(&e)),
    }
    println!("      → 连接被拒绝/DNS 解析不了，重试通常没用，应该报警或降级");

    // (c) 非 2xx：GitHub 对不存在的仓库返回 404。
    println!("  (c) HTTP 非 2xx: GET {MISSING_URL}");
    match client.get(MISSING_URL).send().await {
        Ok(resp) => {
            let status = resp.status();
            println!(
                "      收到响应 HTTP {status}（is_success = {}）",
                status.is_success()
            );
            if status.is_success() {
                println!("      （意外：这个仓库居然存在）");
                return;
            }
            // 方式一：自己检查状态码 —— 好处是还能读到服务端给的错误详情。
            match resp.text().await {
                Ok(body) => {
                    let snippet: String = body.chars().take(160).collect();
                    println!("      手动读错误体: {snippet} …");
                }
                Err(e) => println!("      读错误体失败: {}", explain(&e)),
            }
            // 方式二：error_for_status() 把 4xx/5xx 统一变成 Err，和别的错误走同一条路。
            match client
                .get(MISSING_URL)
                .send()
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(resp) => println!("      第二次请求: HTTP {}", resp.status()),
                Err(e) => println!("      error_for_status() 之后: {}", explain(&e)),
            }
            println!("      → 4xx 是调用方的问题，重试没用；5xx 才是服务端问题，可以退避重试");
        }
        Err(e) => println!("      请求失败: {}", explain(&e)),
    }
}
