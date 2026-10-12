//! # 第 20 章 · 数据库：sqlx —— 可运行 demo
//!
//! 一个「笔记」小程序的完整 CRUD + 事务演示，全部跑在本地的 SQLite 文件 `notes.db` 上。
//!
//! **本文件选用了运行时版本的 `query_as::<_, Note>(...)`，而不是编译期的 `query_as!`**，
//! 原因在文件末尾的 `compile_check` 模块和 README 里都有详细解释。
//!
//! 运行：
//!
//! ```bash
//! cargo run
//! ```
//!
//! 说明：这个 demo 操作的是磁盘上的 `notes.db`，且**不做清库**——
//! 所以第 2 次运行你会看到第 1 次留下的行。想重新来过就 `rm notes.db`。

use anyhow::{Context, Result};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// 数据库连接串。想换文件位置就改环境变量 `DATABASE_URL`。
///
/// ⚠️ 注意结尾的 **`?mode=rwc`**（read-write + create）：sqlx 的 `SqliteConnectOptions`
/// 默认 `create_if_missing = false`，不加这个参数时**文件不存在就直接报
/// `(code: 14) unable to open database file`**，第一次运行必然失败。
/// 这里默认带上它，程序才能自己建库（已实测，见 README 的「常见问题」）。
///
/// 其他常用参数：`?mode=ro` 只读、`?cache=shared` 共享缓存、`?busy_timeout=5000` 等锁。
/// `sqlite::memory:` 则是内存库（连接池里每个连接各有一份，除非 `&cache=shared`）。
fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:notes.db?mode=rwc".to_string())
}

/// 一行的映射目标。字段名必须和 SQL 选出来的列名**逐字一致**，
/// `FromRow` 的 derive 才能自动把列塞进字段。
///
/// `body: Option<String>` 对应可空列：SQL 里是 `NULL` 就得到 `None`。
#[derive(Debug, sqlx::FromRow)]
struct Note {
    id: i64,
    title: String,
    body: Option<String>,
    created_at: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = database_url();
    println!("== 0. 连接池 ==");
    println!("DATABASE_URL = {url}");

    // 连接池：sqlx 的核心抽象。`SqlitePool` 内部是 N 个已建好的连接，
    // 需要时借出、用完归还，避免每次查询都重新打开文件 + 握手。
    // SQLite 是**单文件、写操作串行**的，所以 max_connections 开太大反而
    // 容易撞上 "database is locked"；这里 5 是一个务实的默认值。
    let pool: SqlitePool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .with_context(|| format!("连接数据库失败：{url}"))?;

    // 连接级别的初始化：SQLite 默认**不**强制外键（历史包袱），
    // 每个连接都要单独打开一次。换成 Postgres 时这里通常是设置 search_path。
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    init_schema(&pool).await?;

    println!("\n== 1. 插入：query + 绑定参数，拿回自增主键 ==");
    let id1 = insert_note(&pool, "买牛奶", Some("顺便看看有没有酸奶")).await?;
    let id2 = insert_note(&pool, "写周报", None).await?;
    let id3 = insert_note(&pool, "读《Rust 程序设计语言》第 13 章", Some("async 那一章")).await?;
    println!("插入成功，新行 id = {id1}, {id2}, {id3}");

    println!("\n== 2. 查询多行到 struct：query_as ==");
    print_notes(&list_notes(&pool).await?);

    println!("\n== 3. 单行查询：fetch_optional（可能没有） ==");
    match find_note(&pool, id2).await? {
        Some(note) => println!("找到 id={}：{:?}", note.id, note.title),
        None => println!("id={id2} 不存在"),
    }
    let missing = find_note(&pool, 999_999).await?;
    println!("查 id=999999 的结果：{missing:?}（None 而不是报错，这就是 fetch_optional）");

    println!("\n== 4. 更新：WHERE 限定 + rows_affected ==");
    let updated = update_title(&pool, id1, "买牛奶和酸奶").await?;
    println!("受影响行数 = {updated}");

    println!("\n== 5. 删除：DELETE ==");
    let deleted = delete_note(&pool, id3).await?;
    println!("删除 id={id3}，受影响行数 = {deleted}");
    print_notes(&list_notes(&pool).await?);

    println!("\n== 6. 事务：要么全成功，要么全回滚 ==");
    // 6a：正常提交 —— 两条 INSERT 是一个原子单元。
    let committed = transfer_demo(&pool, "开会记录", "会后待办", true).await?;
    println!("提交成功：{committed}");

    // 6b：让第二条 INSERT 真的违反 NOT NULL 约束。
    //     第一条（标题合法）已经执行过，但整个事务回滚，所以它**不会**留下痕迹。
    let before: i64 = count_notes(&pool).await?;
    let err = transfer_demo(&pool, "这条不该留下", "", false).await;
    let after: i64 = count_notes(&pool).await?;
    println!("回滚演示：Err = {}", err.unwrap_err());
    println!("事务前 {before} 行，事务后 {after} 行 —— 差值应为 0");

    println!("\n== 7. 最终状态 ==");
    print_notes(&list_notes(&pool).await?);
    println!("总行数 = {}", count_notes(&pool).await?);

    // 只有显式 --features compile-check 时这段才存在（见文件末尾）。
    #[cfg(feature = "compile-check")]
    {
        println!("\n== 8. 编译期校验版本（query_as!） ==");
        print_notes(&compile_check::list_notes_checked(&pool).await?);
    }
    #[cfg(not(feature = "compile-check"))]
    println!("\n（本次没启用 compile-check feature，所以 query_as! 版本没有编译进来）");

    pool.close().await;
    println!("\n连接池已关闭。看看当前目录，多了个 notes.db（可以 rm 掉重来）。");
    Ok(())
}

/// 建表。`CREATE TABLE IF NOT EXISTS` 让程序可以反复启动。
///
/// `created_at` 用 SQLite 的 `datetime('now')` 默认值，存在 **TEXT** 列里
/// （格式 `YYYY-MM-DD HH:MM:SS`，UTC）。见文档「时间类型存 TEXT 还是 INTEGER」。
async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notes (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            title      TEXT NOT NULL,
            body       TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await
    .context("建表失败")?;
    println!("已确保 notes 表存在（CREATE TABLE IF NOT EXISTS）");
    Ok(())
}

/// ① 插入：`sqlx::query` 只关心「执行」，返回的 `SqliteQueryResult` 里有 `last_insert_rowid()`。
///
/// `?` 是**占位符**（SQLite / MySQL 风格）；Postgres 用 `$1, $2`。
/// 值通过 `.bind()` 单独传，绝不要用 `format!` 拼进 SQL —— 那是 SQL 注入的入口。
async fn insert_note(pool: &SqlitePool, title: &str, body: Option<&str>) -> Result<i64> {
    let result = sqlx::query("INSERT INTO notes (title, body) VALUES (?, ?)")
        .bind(title)
        .bind(body)
        .execute(pool)
        .await?;
    Ok(result.last_insert_rowid())
}

/// ② 多行查询映射到 struct。
///
/// 这里用**运行时**版本 `query_as::<_, Note>()`：SQL 只是一个字符串，
/// 列名与字段的匹配在运行时完成（对不上会返回 `Err(ColumnNotFound)`）。
/// 编译期版本 `query_as!(Note, "...")` 的写法见文件末尾 `compile_check` 模块。
async fn list_notes(pool: &SqlitePool) -> Result<Vec<Note>> {
    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, title, body, created_at FROM notes ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(notes)
}

/// ③ 单行查询：`fetch_optional` 把「零行」变成 `Ok(None)`。
///
/// 对比：`fetch_one` 零行会报 `RowNotFound` 错；`fetch_all` 永远返回 `Vec`（可能为空）。
async fn find_note(pool: &SqlitePool, id: i64) -> Result<Option<Note>> {
    let note = sqlx::query_as::<_, Note>(
        "SELECT id, title, body, created_at FROM notes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(note)
}

/// ④ 更新：返回「受影响行数」，调用者据此判断目标是否存在。
async fn update_title(pool: &SqlitePool, id: i64, new_title: &str) -> Result<u64> {
    let result = sqlx::query("UPDATE notes SET title = ? WHERE id = ?")
        .bind(new_title)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// ⑤ 删除。同样用 `rows_affected()` 报告是否真的删到了东西。
async fn delete_note(pool: &SqlitePool, id: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// ⑥ 事务：`pool.begin()` 拿到一个 `Transaction`，它 **Deref 到连接**，
/// 所以期间所有 `execute(&mut *tx)` 都走同一条连接、同一个事务。
///
/// `ok = false` 时第二条 INSERT 绑定 `NULL` 给 `title`，撞上 `title TEXT NOT NULL`
/// 约束；`?` 直接把错误抛出去 —— 此时 `tx` 还没 `commit()` 就被 drop，
/// sqlx 的 `Transaction::drop` 会回滚，所以第一条 INSERT **也不会**留痕。
///
/// 想显式回滚（比如业务规则不允许继续，但又不想造一个 SQL 错误）就写
/// `tx.rollback().await?`，语义一样清楚。
async fn transfer_demo(
    pool: &SqlitePool,
    first: &str,
    second_title: &str,
    ok: bool,
) -> Result<String> {
    let mut tx = pool.begin().await?;

    // 第一条：标题合法，一定会真的写进去（但还没提交）。
    sqlx::query("INSERT INTO notes (title, body) VALUES (?, '事务里的第一条')")
        .bind(first)
        .execute(&mut *tx) // 注意是 &mut *tx：借用事务背后的连接
        .await?;

    // 第二条：ok=false 时把 NULL 绑给 title（NOT NULL 列）来触发真实失败。
    // `.bind(None::<&str>)` 就是把 SQL NULL 送进去，而不是空字符串 ''。
    let maybe_null_title: Option<&str> = if ok { Some(second_title) } else { None };
    sqlx::query("INSERT INTO notes (title, body) VALUES (?, '事务里的第二条')")
        .bind(maybe_null_title) // Some -> 正常；None -> SQL NULL -> 违反 NOT NULL
        .execute(&mut *tx)
        .await
        .context("第二条 INSERT 失败（这里是故意让它失败的）")?;

    tx.commit().await?; // 不 commit 就 drop = 回滚
    Ok(format!("已提交两条：{first} / {second_title}"))
}

/// 单值查询：`query_scalar` 适合 `SELECT COUNT(*)` 这类「只有一列一行」的场景。
async fn count_notes(pool: &SqlitePool) -> Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// 把查询结果打成表。`body` 是 `Option<String>`：`None` 就说明数据库里是 NULL。
fn print_notes(notes: &[Note]) {
    if notes.is_empty() {
        println!("（没有笔记）");
        return;
    }
    for note in notes {
        // body 是 Option<String>：None 说明数据库里存的是 NULL。
        let body = note.body.as_deref().unwrap_or("—");
        println!(
            "  #{:<3} {:<28} body={:<20} created_at={}",
            note.id, note.title, body, note.created_at
        );
    }
}

// ---------------------------------------------------------------------------
// 编译期校验版本（默认不编译；见 Cargo.toml 的 compile-check feature）
// ---------------------------------------------------------------------------
//
// 启用方式：
//
// ```bash
// cd projects/sqlx-notes
// rm -f notes.db
// DATABASE_URL="sqlite:notes.db" cargo run --features compile-check
// ```
//
// `query_as!` 是**过程宏**：它在 `cargo build` 时真的去连 DATABASE_URL 指向的库，
// 把 `SELECT id, title, body, created_at FROM notes ...` 交给 SQLite 解析，
// 从而在**编译期**知道：
//   * 列名拼错 -> 编译报错（而不是运行时 ColumnNotFound）
//   * 表/列不存在 -> 编译报错
//   * 每个 `?` 的类型与占位符数量是否对得上
//   * 每一列的 NULL 性：可空列必须映射到 `Option<T>`，否则编译报错
// 代价：**构建环境必须能连上数据库**，或者提前把校验结果存进 `.sqlx/` 离线缓存
// （`cargo sqlx prepare`，CI 里配 `SQLX_OFFLINE=true`）。
#[cfg(feature = "compile-check")]
mod compile_check {
    use super::Note;
    use sqlx::SqlitePool;

    /// 与 `list_notes` 等价的编译期校验版本，由 `main` 的 `== 8 ==` 段调用。
    /// 注意 SQL 里的列必须和 `Note` 字段一一对应，宏会逐列检查 NULL 性。
    pub async fn list_notes_checked(pool: &SqlitePool) -> anyhow::Result<Vec<Note>> {
        let notes = sqlx::query_as!(
            Note,
            r#"SELECT id, title, body, created_at FROM notes ORDER BY id DESC"#
        )
        .fetch_all(pool)
        .await?;
        Ok(notes)
    }
}
