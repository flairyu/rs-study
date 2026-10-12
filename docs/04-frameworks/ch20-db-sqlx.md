# 第 20 章 · 数据库：sqlx

> **一句话定位**：`sqlx` 是「异步、无 ORM、SQL 优先」的数据库库——SQL 仍然是 SQL，它负责安全地绑定参数、把行映射成 struct、把连接池和事务管好。
> **预计时间**：140 分钟（读 70 + 跑 demo / 改造 70）
> **前置**：第 6 章（错误处理）、第 13 章（async/await）、第 17 章（常用第三方库）
> **配套 demo**：[`projects/sqlx-notes`](../../projects/sqlx-notes/)（可运行，独立 crate，需网络拉依赖）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

同一个需求「查 id=1 的笔记标题」：

```c
/* C + sqlite3：类型要自己记，stmt 忘了 finalize 就泄漏 */
sqlite3_prepare_v2(db, "SELECT title FROM notes WHERE id = ?", -1, &st, NULL);
sqlite3_bind_int(st, 1, 1);
if (sqlite3_step(st) == SQLITE_ROW) printf("%s\n", sqlite3_column_text(st, 0));
```

```python
# Python：DB-API 同步；SQLAlchemy 在 SQL 之上再盖一层 ORM / 表达式 DSL
row = con.execute("SELECT title FROM notes WHERE id = ?", (1,)).fetchone()
print(row[0] if row else "没有")
```

```rust
// Rust + sqlx：异步、绑定是一等公民、结果映射由类型系统检查
let title: Option<String> = sqlx::query_scalar("SELECT title FROM notes WHERE id = ?")
    .bind(1_i64)
    .fetch_optional(&pool)
    .await?;
println!("{}", title.unwrap_or_else(|| "没有".into()));
```

| 维度 | C（libpq / sqlite3） | Python（`sqlite3` / SQLAlchemy） | Rust（`sqlx` / `diesel` / `sea-orm`） |
|---|---|---|---|
| 阻塞模型 | 全同步 | 全同步 | `sqlx` **异步**；`diesel`、`sea-orm` 默认同步 |
| SQL 在哪 | C 字符串 | Python 字符串 | `sqlx` 字符串**或** `query_as!` 编译期校验 |
| 参数绑定 | `sqlite3_bind_*`，下标从 1 起 | `?` / `%s` 随驱动 | 统一 `.bind()`；SQLite 用 `?`，Postgres 用 `$1` |
| 行 → 对象 | 手动 `column_*` | 手动 tuple 或 ORM | `#[derive(FromRow)]` + `query_as` |
| 迁移 | 手写 / Flyway / Alembic | Alembic | `sqlx migrate`（纯 SQL 文件） |

**三个框架的取舍**：`sqlx` 把 SQL 当唯一真相来源，只做四件事——异步 IO、参数绑定、行映射、连接池，**不做 ORM**；`diesel` 是**同步** + 编译期 DSL（`notes::table.filter(notes::id.eq(1))`），类型安全到极致，但 CTE、窗口函数这类复杂 SQL 表达别扭；`sea-orm` 是异步 + 完整 ORM（Entity / Relation / ActiveModel），喜欢 Django ORM 风格的人会觉得亲切。想写 SQL、要异步、讨厌 ORM 隐式行为（N+1、偷偷生成的 JOIN）→ 选 `sqlx`；团队不愿写 SQL 且是同步服务 → `diesel` 的编译期保障更香。本章只讲 `sqlx`。

> ⚠️ **坑**：Rust 标准库**完全没有**数据库 API，连 SQLite 都没有。第 16 章说过：std 只放稳定、无争议、跨平台的东西，SQL 方言显然不属于这一类。

---

## 1. 选 feature：SQLite 还是 Postgres

**feature 不是运行时开关，是编译期开关**——不需要的驱动不会进二进制。

```toml
[dependencies]
# demo 用的：SQLite
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "migrate"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
anyhow = "1"

# 换 Postgres 只改 sqlx 这一行
# sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "macros", "migrate", "uuid", "chrono"] }
```

| feature | 作用 | 不加会怎样 |
|---|---|---|
| `runtime-tokio` | 用 tokio 做异步运行时（另有 `runtime-async-std`） | 编译报错：没有 runtime |
| `sqlite` / `postgres` / `mysql` | 选驱动，**可以同时开** | 没有 `SqlitePool` 等类型 |
| `macros` | `query!` / `query_as!` 宏 | 宏不可用 |
| `migrate` | `sqlx::migrate!` 与迁移执行器 | 无法在程序内跑迁移 |
| `tls-rustls` / `tls-native-tls` | Postgres/MySQL 的 TLS | 连云数据库可能失败 |
| `uuid` / `chrono` / `time` / `json` | 扩展类型互转 | `Uuid` 不能被 `bind`/`decode` |

> ⚠️ **坑**：默认 feature 里**没有 runtime**。很多人写 `sqlx = "0.8"` 然后报「async runtime not found」，就是漏了 `runtime-tokio`。

**SQLite vs Postgres**：本地文件、单机、嵌入式、测试 → SQLite；多进程/多机并发写、需要 `JSONB`/数组 → Postgres。生产用 SQLite 也别慌：开 WAL、设 `busy_timeout`、写操作串行化，能扛住中小流量。

---

## 2. `SqlitePool` 与连接池

所有查询都接受 `&Pool`（内部自动借还连接）而不是裸连接：开一个 Postgres 连接要 TCP 握手 + TLS + 认证，SQLite 每次打开文件也要重新解析 schema、重建缓存。

```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

let pool: SqlitePool = SqlitePoolOptions::new()
    .max_connections(5)                        // 池子上限；SQLite 写锁串行，别开太大
    .acquire_timeout(Duration::from_secs(5))   // 借不到连接时的等待上限
    .connect("sqlite:notes.db?mode=rwc")
    .await?;
```

> ⚠️ **坑（本 demo 实测踩到的第一个）**：**`?mode=rwc` 不能省**。sqlx 的 `SqliteConnectOptions` 默认 `create_if_missing = false`，所以 `sqlite:notes.db`（不带参数）在**文件不存在时**直接报 `(code: 14) unable to open database file`——而「第一次运行」恰恰就是文件不存在的时候。`mode=rw`（默认）只读写不建库，`mode=rwc` 才建库，`mode=ro` 只读，`mode=memory` 纯内存。等价写法：`SqliteConnectOptions::from_str(url)?.create_if_missing(true)` 配 `.connect_with(...)`。

`Pool` 是 **`Clone` 的**（内部 `Arc`），可以随便 clone 进 `axum` handler，不用再套 `Arc<Mutex<...>>`：

```rust
let pool2 = pool.clone();
tokio::spawn(async move { /* pool2 直接用 */ });
```

### 2.1 连接级初始化：`PRAGMA` / `after_connect`

SQLite 的 `PRAGMA foreign_keys` 默认 **OFF**（历史包袱），而且它是**连接级**设置：

```rust
// 做法 A：启动时对池子发一次（demo 用的，简单）
sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

// 做法 B：每条新连接建立时自动执行（推荐），已实测
use sqlx::{Executor, sqlite::SqlitePoolOptions};

let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .after_connect(|conn, _meta| Box::pin(async move {
        conn.execute("PRAGMA foreign_keys = ON").await?;
        Ok(())
    }))
    .connect("sqlite:notes.db?mode=rwc")
    .await?;
```

> ⚠️ **坑**：做法 A 在 `max_connections > 1` 时**不能保证**每条连接都开了外键（看运气）。demo 里保留它只为演示「连接级 SQL」最直观；真用请换 `after_connect`。回调里用的是 `Executor::execute`（要 `use sqlx::Executor;`），参数是**裸连接**而非 `&Pool`，返回 `Box::pin(async move { ... })` 是 sqlx 0.8 的硬要求，照抄形状即可。

---

## 3. 三种查询构造器

| 构造器 | 返回 | 典型用途 |
|---|---|---|
| `sqlx::query(sql)` | `Query`（原始行） | INSERT/UPDATE/DELETE、`PRAGMA` |
| `sqlx::query_as::<_, T>(sql)` | `QueryAs<T>` | `SELECT` → struct / tuple |
| `sqlx::query_scalar::<_, T>(sql)` | `QueryScalar<T>` | `COUNT(*)`、单个 `id`、单个 `String` |

```rust
// execute 返回 QueryResult，含 rows_affected / last_insert_rowid
let r = sqlx::query("INSERT INTO notes (title, body) VALUES (?, ?)")
    .bind("买牛奶").bind(Some("顺便看看有没有酸奶"))
    .execute(&pool).await?;
println!("{} {}", r.rows_affected(), r.last_insert_rowid());

let notes: Vec<Note> = sqlx::query_as::<_, Note>(
    "SELECT id, title, body, created_at FROM notes ORDER BY id DESC")
    .fetch_all(&pool).await?;                       // SELECT → struct

let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes")
    .fetch_one(&pool).await?;                       // 单个值
```

| `.fetch_*` | 0 行时 | >1 行时 | 何时用 |
|---|---|---|---|
| `fetch_one` | `Err(RowNotFound)` | 取第一行，**不报错** | 你确信恰好一行（`COUNT`） |
| `fetch_optional` | `Ok(None)` | 取第一行 | **按主键查、可能不存在**——最常用 |
| `fetch_all` | `Ok(vec![])` | 取全部 | 列表查询 |
| `fetch` | —— | —— | 返回 `Stream`，逐行流式消费 |

`fetch_all` 把整个结果集读进内存；大结果集用 `.fetch(&pool)` 配 `futures_util::TryStreamExt::try_next`（`futures-util` 要自己加进 `Cargo.toml`）：

```rust
use futures_util::TryStreamExt;

let mut stream = sqlx::query_as::<_, Note>("SELECT id, title, body, created_at FROM notes")
    .fetch(&pool);
while let Some(note) = stream.try_next().await? {
    println!("{}", note.title);
}
```

### 3.1 `#[derive(FromRow)]` 的匹配规则

```rust
#[derive(Debug, sqlx::FromRow)]
struct Note {
    id: i64,
    title: String,
    body: Option<String>,   // 可空列 → Option<T>
    created_at: String,
}
```

1. **按列名匹配字段名**，大小写不敏感。改名用 `#[sqlx(rename = "db_col")]`，整体改前缀用 `#[sqlx(rename_all = "camelCase")]`。
2. **NULL 性必须对上**：可空列 → `Option<T>`，非空列 → `T`。反了在 `query_as` 下运行时报错，在 `query_as!` 下编译报错。
3. 列名对不上 → 运行时 `Err(ColumnNotFound)`，错误信息里有列名。统一**显式列出列名**而不是 `SELECT *`——加了一列不会让 `query_as!` 的离线缓存对不上。

---

## 4. `query_as!` 的编译期校验原理

`query_as!(Note, "SELECT ...")` 是一个**过程宏**。`cargo build` 时它：

1. 读 `DATABASE_URL`，**真的建一条连接**（SQLite 直接开文件，Postgres 连服务器）。
2. 把 SQL 交给数据库 `PREPARE`，拿到结果集元数据：每列的名字、类型、是否可空。
3. 与 `Note` 字段逐个比对，**生成**「每列解码成哪个 Rust 类型」的代码（内联展开，运行时零反射）。
4. 任何不匹配 → **编译错误**，且指向 SQL 里那一列。

| 错误 | `query_as`（运行时） | `query_as!`（编译期） |
|---|---|---|
| 表名拼错 `notez` | `no such table: notez` | 编译失败 |
| 列名拼错 `titel` | `ColumnNotFound("titel")` | 编译失败 |
| `?` 数量与 `.bind()` 不符 | 运行时才炸 | 编译失败 |
| 可空列映射到 `String` | 运行时解码错误 | 编译失败 |

**代价**：构建环境必须能连上数据库，或者用离线缓存：

```bash
cargo install sqlx-cli --no-default-features --features sqlite   # 装一次
cargo sqlx prepare                # 扫源码里的宏，连库校验，结果写进 .sqlx/
git add .sqlx/                    # 提交进版本库
SQLX_OFFLINE=true cargo build     # CI / Docker 里不再需要数据库
```

`.sqlx/` 里是每个宏调用的查询元数据 JSON（按 SQL 内容哈希命名）。改了 SQL 却忘了 `prepare`，CI 会报错——这是**好事**，它逼你保持缓存与 SQL 同步。

### 4.1 本 demo 为什么选 `query_as`

demo 的 CRUD **用的是运行时版本** `query_as::<_, Note>(...)`：

```rust
let notes = sqlx::query_as::<_, Note>(
    "SELECT id, title, body, created_at FROM notes ORDER BY id DESC",
).fetch_all(pool).await?;
```

1. **零配置可跑**。教程读者 clone 下来 `cargo run` 就该看到输出（本教程所有 demo 的硬要求）。
2. **不需要装 `sqlx-cli`**，少了「装 CLI → prepare」这道门槛。
3. `query_as!` 要求构建时 `notes.db` 里**已经有 `notes` 表**——而表恰恰是这个程序自己建的，形成**鸡生蛋**：编译需要表，建表需要编译。

但编译期校验是这库的卖点，不能不演示，所以用 **feature 门控**两全其美：

```toml
[features]
compile-check = []     # 默认不启用
```

```bash
cargo run                                              # 默认路径：零配置
cargo run                                              # 先生成 notes.db + notes 表
DATABASE_URL="sqlite:notes.db?mode=rwc" cargo run --features compile-check   # 编译期校验版
```

门后的代码就是 `src/main.rs` 末尾这一段（**已实测编译并运行通过**）：

```rust
#[cfg(feature = "compile-check")]
pub async fn list_notes_checked(pool: &SqlitePool) -> anyhow::Result<Vec<Note>> {
    let notes = sqlx::query_as!(
        Note,
        r#"SELECT id, title, body, created_at FROM notes ORDER BY id DESC"#
    )
    .fetch_all(pool)
    .await?;
    Ok(notes)
}
```

**生产项目怎么选？** 只要能连库（或肯维护 `.sqlx/`），就**用宏**：SQL 还是你写的 SQL，但编译器帮你核对它和 Rust 类型是否一致。运行时 `query_as` 的合理场景是动态拼 SQL（列名来自配置）、或构建环境实在连不上库又不想维护缓存。

---

## 5. 绑定参数与 SQL 注入

```rust
// ✅ 值走 bind，SQL 结构固定
sqlx::query("SELECT id, title FROM notes WHERE title = ?")
    .bind(user_input).fetch_all(&pool).await?;

// ❌ 用户输入进了 SQL 文本，`' OR 1=1 --` 立刻击穿
let sql = format!("SELECT id, title FROM notes WHERE title = '{user_input}'");
sqlx::query(&sql).fetch_all(&pool).await?;
```

绑定的本质是**协议层的参数**：SQL 文本先被 `PREPARE` 编译成执行计划，值后到，**永远不参与语法解析**。所以 `'; DROP TABLE notes; --` 只是普通字符串。

| 数据库 | 占位符 |
|---|---|
| SQLite / MySQL | `?`（也可 `?1` 编号），按出现顺序对应 `.bind()` |
| Postgres | `$1, $2`（**从 1 开始**），同一个 `$n` 可重复引用 |

批量插入用 `QueryBuilder`（已实测）：

```rust
use sqlx::QueryBuilder;

let mut qb = QueryBuilder::new("INSERT INTO notes (title) ");
qb.push_values(["a", "b", "c"], |mut b, title| { b.push_bind(title); });
qb.build().execute(&pool).await?;
```

> ⚠️ **坑**：`bind` 保护的是**值**，不是**标识符**。表名、列名、`ORDER BY` 的字段没法绑定，只能从白名单里挑，绝不要直接 `format!` 用户输入。

---

## 6. `Option<T>` ↔ NULL

| SQL | Rust |
|---|---|
| `NOT NULL` 列 | `T`（例如 `title: String`） |
| 可空列 | `Option<T>`（SQL `NULL` ↔ `None`） |
| `bind(None::<&str>)` | 写入 `NULL`，而不是 `''` |

**`''` ≠ `NULL`**：`WHERE body = ''` 查不到 `body IS NULL` 的行；`query_as!` 也会把「`String` 接可空列」当编译错误。demo 的回滚演示就利用了这个区别：绑 `None` 才触发 `NOT NULL`，绑 `Some("")` 不会。另外 `COALESCE(body, '')` 能「消灭」NULL，代价是丢掉「不知道」与「空」的区分（业务决策）；`COUNT(body)` 只数非 NULL，`COUNT(*)` 数所有行，写报表别混。

---

## 7. 事务与回滚

```rust
let mut tx = pool.begin().await?;          // BEGIN：从池里借一条连接并固定住

sqlx::query("INSERT INTO notes (title, body) VALUES (?, '事务里的第一条')")
    .bind(first)
    .execute(&mut *tx)                     // 注意 &mut *tx
    .await?;

sqlx::query("INSERT INTO notes (title, body) VALUES (?, '事务里的第二条')")
    .bind(maybe_null_title)                // None → NULL → 违反 NOT NULL
    .execute(&mut *tx)
    .await?;                               // ← ? 抛出，tx 被 drop → 自动回滚

tx.commit().await?;                        // 到这行才真正落盘
```

1. **`&mut *tx` 不是丑语法**：`Transaction` 通过 `DerefMut` 解引用成背后的连接；`execute` 要 `&mut SqliteConnection`。
2. **同一个事务必须用同一条连接**。`tx` 期间不能再 `pool.execute(...)`——那会借走另一条连接、跑在事务之外。
3. **没 `commit()` 就 drop = 回滚**。`Transaction::drop` 会排队一个 `ROLLBACK`，这条规则救了无数「忘记处理错误」的代码。
4. **`rollback()` 可以显式调用**。业务规则不允许继续时比构造假 SQL 错误更清晰；效果相同。

demo 的验证方式很直接：事务前数一次行数、故意让第二条 INSERT 失败、再数一次，**差值必须是 0**。另外别在持有 `tx` 时 `.await` 一个很慢的外部 IO——SQLite 的写锁会一直握着，别人全在等。

---

## 8. 迁移（`sqlx migrate`）

```bash
cargo install sqlx-cli --no-default-features --features sqlite
sqlx migrate add create_notes        # 生成 migrations/20261012120000_create_notes.sql
DATABASE_URL="sqlite:notes.db" sqlx migrate run      # 执行未应用的迁移
sqlx migrate revert                  # 回滚最后一个（需要 -r 生成的 down 文件）
```

```sql
-- migrations/20261012120000_create_notes.sql
CREATE TABLE IF NOT EXISTS notes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL,
    body       TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

程序内跑迁移（需要 `migrate` feature）：`sqlx::migrate!("./migrations").run(&pool).await?;`——宏把迁移文件**编译进二进制**，发布时不用额外拷文件。

| 规则 | 原因 |
|---|---|
| 迁移文件一旦提交就**不要改** | 生产库已记录它的版本/哈希，改了会校验失败 |
| 一个迁移只做一件事，配套 `-r` 回滚脚本 | 出问题能快速退 |
| 迁移里**不要**依赖应用代码 | 它是历史上某个时点的快照 |
| 时间戳前缀别手写 | `sqlx migrate add` 自动生成，保证顺序 |

> ⚠️ **坑**：本 demo **没用迁移**，而是 `init_schema` 里 `CREATE TABLE IF NOT EXISTS`。好处是「一个 `cargo run` 从零到可用」，代价是 schema 变更无法追踪。真项目请直接用迁移并删掉 `init_schema`。

---

## 9. 类型映射表

**SQLite → Rust**（demo 用到的加粗）：

| SQLite 声明类型 | Rust | 备注 |
|---|---|---|
| `INTEGER` | **`i64`** / `i32` / `u8` … | SQLite 内部只有 8 字节整数，`i64` 最保险 |
| `INTEGER`（0/1） | `bool` | sqlx 支持 |
| `REAL` | **`f64`** / `f32` | |
| `TEXT` | **`String`** / `&str` | 编码约定是 UTF-8 |
| `BLOB` | `Vec<u8>` / `&[u8]` | |
| 任意 | **`Option<T>`** | 可空 |
| `TEXT`（ISO8601） | `String`，或 `chrono::NaiveDateTime`（需 `chrono` feature） | 见「常见坑」第 6 条 |

**Postgres → Rust**：

| Postgres | Rust | 需要的 feature |
|---|---|---|
| `BIGINT` / `INT4` | `i64` / `i32` | —— |
| `TEXT` / `VARCHAR` | `String` | —— |
| `BOOL` | `bool` | —— |
| `UUID` | `uuid::Uuid` | `uuid` |
| `TIMESTAMPTZ` / `TIMESTAMP` | `chrono::DateTime<Utc>` / `NaiveDateTime` | `chrono` |
| `NUMERIC` | `rust_decimal::Decimal` | `rust_decimal` |
| `JSONB` / `JSON` | `serde_json::Value` | `json` |
| `TEXT[]` | `Vec<String>` | —— |

---

## 与 demo 的对照

打开 [`projects/sqlx-notes/src/main.rs`](../../projects/sqlx-notes/src/main.rs)，逐段对上：

| demo 位置 | 对应本章 | 关键行 |
|---|---|---|
| `main` 开头 | §2 | `SqlitePoolOptions::new().max_connections(5).connect(&url)` |
| `PRAGMA foreign_keys = ON` | §2.1 | 连接级初始化 |
| `init_schema` | §8 | `CREATE TABLE IF NOT EXISTS notes (...)` |
| `insert_note` | §3、§5 | `.bind(title).bind(body).execute()` → `last_insert_rowid()` |
| `list_notes` | §3.1、§4.1 | `query_as::<_, Note>(...).fetch_all()` |
| `find_note` | §3 | `fetch_optional` → `Option<Note>` |
| `update_title` / `delete_note` | §3 | `rows_affected()` |
| `transfer_demo` | §7 | `pool.begin()` / `&mut *tx` / `commit()` / 出错自动回滚 |
| `count_notes` | §3 | `query_scalar` + `fetch_one` |
| `compile_check` 模块 | §4.1 | `query_as!`，由 `--features compile-check` 打开 |

```bash
cd /home/yxb/code/rs-study/projects/sqlx-notes
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home   # 本沙箱 ~/.cargo 只读
cargo run
rm -f notes.db     # demo 运行时生成，别留在仓库里
```

---

## 本章练习

三题都是**改造 demo**，完整验收标准写在 [`projects/sqlx-notes/README.md`](../../projects/sqlx-notes/README.md)。

| 题号 | 任务 | 考点 |
|---|---|---|
| 20-1 | 加 `tags` + `note_tags` 两张表，用事务做「打标签」，用 `JOIN` 做关联查询 | 外键、`PRAGMA foreign_keys` 的连接级语义、`ON DELETE CASCADE` |
| 20-2 | 给笔记加搜索：先 `LIKE` 版，再 FTS5 版 | 绑定参数（含 `ESCAPE`）、`MATCH`、触发器同步 |
| 20-3 | 把 SQLite 换成 Postgres，列出要改的每一处并让 `cargo check` 通过 | feature 切换、`?` → `$n`、`last_insert_rowid()` → `RETURNING` |

**20-1 提示**：`note_tags` 主键设成 `PRIMARY KEY (note_id, tag_id)`，重复打标签自动被拒；「删旧 → 插新」必须在同一事务里。别忘了外键要求**每条连接**都开 `PRAGMA foreign_keys`（用 §2.1 的 `after_connect`），否则你会以为代码写对了，其实约束根本没生效。

**20-2 提示**：`LIKE` 的 `%`/`_` 是通配符，用户搜 `100%` 要写 `WHERE title LIKE ? ESCAPE '\'` 并绑 `"%100\%"`。FTS5 的 `MATCH` 绑的是查询表达式（如 `"rust OR async"`），同步用 `AFTER INSERT/UPDATE/DELETE` 三个触发器（external content 表的标准套路）。

**20-3 提示**：`cargo check` 不需要真连库（只要不启用 `query_as!`）。最容易漏的三处：① `last_insert_rowid()` 在 Postgres 不存在，改成 `INSERT ... RETURNING id` + `query_scalar`；② 建表 SQL 里 `datetime('now')` 与 `AUTOINCREMENT` 都不是 Postgres 的（用 `now()` / `GENERATED ALWAYS AS IDENTITY`）；③ `?` 全部换成 `$1, $2`。

---

## 常见坑

1. **`query_as!` 在编译期真的会连数据库**。CI 上没配 `DATABASE_URL` 会直接编译失败（`error communicating with database`）。解决：`cargo sqlx prepare` 生成 `.sqlx/` 并提交，然后把 `SQLX_OFFLINE=true` 设进 CI。新人第一次把 sqlx 项目推上 CI 最常踩这个。
2. **`?mode=rwc` 不能省**（实测）：sqlx 默认 `create_if_missing = false`，第一次运行（文件还不存在）会报 `(code: 14) unable to open database file`。
3. **SQLite 的写锁是全局的**。同一时刻只有一个写事务，第二个会等（默认 `busy_timeout` 为 0，直接报 `database is locked`）。缓解：写多的场景 `max_connections(1)`、连接串加 `?busy_timeout=5000`、或开 WAL（`PRAGMA journal_mode=WAL`，实测返回 `"wal"`）。**别把 SQLite 当 Postgres 用**。
4. **`?` 与 `From<sqlx::Error>`**。`?` 能用，前提是返回类型（如 `anyhow::Result<T>`）实现了 `From<sqlx::Error>`；`anyhow` 对任何 `std::error::Error` 都有 blanket impl，所以无脑可用。但**别在库的公开 API 里用 `anyhow::Error`**——调用方没法按错误种类匹配；要细粒度就用 `thiserror` 定义错误枚举并 `#[from] sqlx::Error`。
5. **`async` 里不能跨 `.await` 持有非 `Send` 的东西**。典型是 `std::sync::MutexGuard`、`rusqlite::Rows`、以及**借用了连接的 `Row` / `Transaction`**。把 `Arc<Mutex<Connection>>` 换成 `tokio::sync::Mutex`，或者干脆用 `sqlx` 的 `Pool`（自己就是并发安全的）。症状是 `future cannot be sent between threads safely`。
6. **NULL 与 `Option` 必须对号入座**。可空列 → `Option<T>`，非空列 → `T`。反过来，**别因为怕出错把所有字段都写成 `Option`**——那等于丢掉「数据库保证了非空」这一信息，之后每个使用点都要多一次 `unwrap`。
7. **时间类型：存 TEXT 还是 INTEGER？** SQLite 没有真正的日期类型。TEXT（demo 用的）可读性好、能直接字符串排序，配 `chrono` feature 可映射 `NaiveDateTime`；INTEGER（Unix 时间戳）比较/计算便宜但不可读。无论哪种，**跨时区业务一律存 UTC**（`datetime('now')` 返回的就是 UTC——demo 跑在本地 15:31，输出是 `07:31`，正是这个原因），只在展示层转本地时区。隐藏坑：`datetime('now')` 的格式与 `chrono` 默认解析格式并不 100% 对齐，解析失败先 `SELECT created_at` 看一眼真实字符串。
8. **忘了 `.await` 等于什么都没发生**。`sqlx::query(...).execute(&pool)` 不 `.await` 不会执行任何 SQL（有 `#[must_use]` 警告，别忽略）。另外程序结束前 `pool.close().await` 能等到干净关闭。
9. **`sqlite::memory:` 不是一个共享的内存库**（已实测）。池里每条连接各持一份**独立**内存库：

   ```rust
   // 实测输出：Ok(0) —— 建表在连接 A，COUNT 跑在连接 B，B 里根本没有 t 表
   let p = SqlitePool::connect("sqlite::memory:").await?;
   sqlx::query("CREATE TABLE t (a INTEGER)").execute(&p).await?;
   let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t").fetch_one(&p).await?;
   ```

   加 `cache=shared` **还不够**（实测仍是 `Ok(0)`）：持有内存库的那条连接一旦被回收，库就随之销毁。要可靠共享得**让至少一条连接一直活着**——`SqlitePoolOptions::new().min_connections(1).idle_timeout(None).max_lifetime(None).connect("sqlite:file:m?mode=memory&cache=shared")`。即便这样，`CREATE TABLE` 与紧接着的 `SELECT` 仍可能落在不同连接上，**第一次查不到是常态**（实测：`Ok(0)` 之后 `Ok(1)`）。测试里想干净隔离，用临时文件比 `:memory:` 省心得多。

---

## 自测清单

- [ ] 我能说出 `sqlx` / `diesel` / `sea-orm` 的定位差异，以及「sqlx 不做 ORM」具体指什么
- [ ] 我知道 `?mode=rwc` 为什么不能省（`create_if_missing` 默认 false）
- [ ] 我知道 `Cargo.toml` 里选 `sqlite` 还是 `postgres` 会带来哪些代码改动
- [ ] 我能解释 `max_connections` 的意义，以及为什么 SQLite 下不宜开大
- [ ] 我知道 `PRAGMA foreign_keys = ON` 是**连接级**的，并会用 `after_connect` 给池里每条连接都设上
- [ ] 我能分清 `query` / `query_as` / `query_scalar`，以及 `fetch_one` / `fetch_optional` / `fetch_all` 的区别
- [ ] 我能讲清 `query_as!` 编译期校验的原理（连库 → PREPARE → 比对列元数据）与 `.sqlx/` + `SQLX_OFFLINE=true` 的用法
- [ ] 我知道本 demo 为什么默认用 `query_as::<_, Note>`，以及怎么用 `--features compile-check` 跑宏版本
- [ ] 我能写出一个正确的事务（`begin` / `&mut *tx` / `commit`），并解释「不 commit 就 drop 会回滚」
- [ ] 我知道为什么绝不能把用户输入 `format!` 进 SQL，以及 `bind` 为什么能挡住注入
- [ ] 我能说清 `Option<T>` 与 NULL 的对应关系，以及 `''` 与 `NULL` 的区别
- [ ] 我知道 SQLite 里时间存 TEXT 和 INTEGER 的取舍，以及为什么一律存 UTC
- [ ] 我跑通了 `cargo run`，看到 `受影响行数 = 1` 和「事务前 N 行，事务后 N 行 —— 差值应为 0」
