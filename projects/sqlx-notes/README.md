# sqlx-notes —— 第 20 章 · 数据库：sqlx

一个**能直接跑起来**的 SQLite 笔记 demo：建表 → 插入 → 查询 → 单行 → 更新 → 删除 → 事务（含回滚），
全部用 [`sqlx`](https://crates.io/crates/sqlx) 0.8 的异步 API 完成。

配套教程：[`docs/04-frameworks/ch20-db-sqlx.md`](../../docs/04-frameworks/ch20-db-sqlx.md)

---

## 1. 怎么跑

> ⚠️ 本 crate **不属于任何 workspace**（`projects/` 下每个 demo 都是独立 crate）。
> 第一次运行要下载 sqlx + tokio 的全部依赖，**需要网络**，可能要几十秒到几分钟。

```bash
cd /home/yxb/code/rs-study/projects/sqlx-notes

# 本沙箱的 ~/.cargo 只读，必须把 CARGO_HOME 指到仓库里（普通机器上跳过这两行）
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/yxb/code/rs-study/.cargo-home

cargo run
```

**运行时会生成 `notes.db`**（SQLite 单文件数据库，就在当前工作目录）。
它**没有被提交进仓库**，也不该提交；想回到干净状态：

```bash
rm -f notes.db          # 下次 cargo run 会重新建表
```

### 换一个数据库文件

程序读环境变量 `DATABASE_URL`，默认是 `sqlite:notes.db?mode=rwc`：

```bash
DATABASE_URL="sqlite:/tmp/scratch.db?mode=rwc" cargo run
```

> ⚠️ **`?mode=rwc` 不能省**。sqlx 的 `SqliteConnectOptions` 默认 `create_if_missing = false`，
> 所以 `sqlite:notes.db`（不带参数）在**文件不存在时会直接报错**：
>
> ```text
> Error: 连接数据库失败：sqlite:notes.db
> Caused by:
>     0: error returned from database: (code: 14) unable to open database file
> ```
>
> 这个行为是**实测出来的**（sqlx 0.8.6）：`mode=rw` 已经是默认值，只有 `mode=rwc`（read-write + create）
> 才会让 sqlx 建库。第一次 `cargo run` 必然走「建库」这条路，所以默认串里必须带它。
> 其他有用的参数：`?mode=ro` 只读、`?cache=shared` 共享缓存、`?busy_timeout=5000` 等锁。
> （`sqlite:` 后面是**相对当前工作目录**的路径。）

---

## 2. 预期输出

下面是**从零开始（`rm -f notes.db` 之后）实际跑出来的完整输出**：

```text
== 0. 连接池 ==
DATABASE_URL = sqlite:notes.db?mode=rwc
已确保 notes 表存在（CREATE TABLE IF NOT EXISTS）

== 1. 插入：query + 绑定参数，拿回自增主键 ==
插入成功，新行 id = 1, 2, 3

== 2. 查询多行到 struct：query_as ==
  #3   读《Rust 程序设计语言》第 13 章         body=async 那一章            created_at=2026-10-12 07:31:39
  #2   写周报                          body=—                    created_at=2026-10-12 07:31:39
  #1   买牛奶                          body=顺便看看有没有酸奶            created_at=2026-10-12 07:31:39

== 3. 单行查询：fetch_optional（可能没有） ==
找到 id=2："写周报"
查 id=999999 的结果：None（None 而不是报错，这就是 fetch_optional）

== 4. 更新：WHERE 限定 + rows_affected ==
受影响行数 = 1

== 5. 删除：DELETE ==
删除 id=3，受影响行数 = 1
  #2   写周报                          body=—                    created_at=2026-10-12 07:31:39
  #1   买牛奶和酸奶                       body=顺便看看有没有酸奶            created_at=2026-10-12 07:31:39

== 6. 事务：要么全成功，要么全回滚 ==
提交成功：已提交两条：开会记录 / 会后待办
回滚演示：Err = 第二条 INSERT 失败（这里是故意让它失败的）
事务前 4 行，事务后 4 行 —— 差值应为 0

== 7. 最终状态 ==
  #5   会后待办                         body=事务里的第二条              created_at=2026-10-12 07:31:39
  #4   开会记录                         body=事务里的第一条              created_at=2026-10-12 07:31:39
  #2   写周报                          body=—                    created_at=2026-10-12 07:31:39
  #1   买牛奶和酸奶                       body=顺便看看有没有酸奶            created_at=2026-10-12 07:31:39
总行数 = 4

（本次没启用 compile-check feature，所以 query_as! 版本没有编译进来）

连接池已关闭。看看当前目录，多了个 notes.db（可以 rm 掉重来）。
```

几点说明（不是 bug）：

- `created_at` 是 **UTC**（本地时间是 15:31，输出是 07:31），因为 `datetime('now')` 返回 UTC。想存本地时间用 `datetime('now','localtime')`，但**生产环境应该存 UTC**（见教程「常见坑」第 6 条）。
- 中文标题的列宽会有轻微错位：`{:<28}` 按 **`char` 数**补齐，而终端里中文字符占 2 格。这是格式化细节，不影响逻辑。
- 第二次以后运行行数会累加（4 → 8 → 12…），因为数据留在 `notes.db` 里。

**要盯住的三行**：

1. `受影响行数 = 1` —— `rows_affected()` 是判断「UPDATE/DELETE 是否真的命中」的唯一可靠方式。
2. `事务前 4 行，事务后 4 行 —— 差值应为 0` —— 事务回滚生效，第一条已执行的 INSERT 没留下痕迹。
3. `查 id=999999 的结果：None` —— `fetch_optional` 把「查不到」变成值，而不是错误。

### 代码里没有出现 `query_as!`？

**对，这是有意的**。`src/main.rs` 的 CRUD 用**运行时**版本 `query_as::<_, Note>("SELECT ...")`，
因为它**不需要构建期连数据库**，任何机器上 clone 完就能 `cargo run`。

编译期校验的 `query_as!` 版本放在文件末尾的 `compile_check` 模块里，用 feature 门控。
**已实测跑通**：

```bash
# 第一次必须先 cargo run，让程序建出 notes 表和 notes.db
cargo run
# 然后带 feature 编译并运行：query_as! 在构建时连上 sqlite:notes.db?mode=rwc 校验 SQL
DATABASE_URL="sqlite:notes.db?mode=rwc" cargo run --features compile-check
```

启用后会多出 `== 8. 编译期校验版本（query_as!） ==` 一段，内容和 `== 7 ==` 一致——
但这一版的 SQL 是**在编译期被 SQLite 解析并核对过列名/NULL 性**的。

为什么必须先 `cargo run` 一次？因为 `query_as!` 构建时要 `PREPARE` 那句 `SELECT`，
而 `notes` 表是程序自己建的——**编译需要表、建表需要编译**，这就是「鸡生蛋」。
生产项目的标准解法是离线缓存：

```bash
cargo install sqlx-cli --no-default-features --features sqlite   # 装一次
cargo sqlx prepare                       # 生成 .sqlx/ 目录，提交进版本库
SQLX_OFFLINE=true cargo build            # 之后编译不再需要数据库
```

两种写法的取舍详见教程文档「`query_as!` 的编译期校验原理」一节。

---

## 3. 文件结构

```text
projects/sqlx-notes/
├── Cargo.toml      # sqlx 0.8 / tokio 1 / anyhow 1；compile-check 是可选 feature
├── README.md       # 本文件
├── notes.db        # 运行时生成，不在版本库里（根 .gitignore 已忽略）
└── src/main.rs     # 全部代码，约 300 行，每段都有中文注释
```

---

## 4. 代码地图（`src/main.rs`）

| 段 | 函数 | 演示了什么 |
|---|---|---|
| 0 | `main` 开头 | `SqlitePoolOptions::new().max_connections(5).connect()`、`PRAGMA foreign_keys = ON` |
| 建表 | `init_schema` | `CREATE TABLE IF NOT EXISTS`，`created_at TEXT DEFAULT (datetime('now'))` |
| ① | `insert_note` | `sqlx::query(...).bind(...).execute()` → `last_insert_rowid()` |
| ② | `list_notes` | `query_as::<_, Note>().fetch_all()`，`#[derive(FromRow)]` |
| ③ | `find_note` | `fetch_optional` → `Option<Note>` |
| ④ | `update_title` | `rows_affected()` |
| ⑤ | `delete_note` | 同上，用于 DELETE |
| ⑥ | `transfer_demo` | `pool.begin()` / `&mut *tx` / `commit()` / 出错 drop 自动回滚 |
| ⑦ | `count_notes` | `query_scalar` 取单值 |
| ⑧ | `compile_check::list_notes_checked` | `query_as!` 编译期校验（feature 门控） |

---

## 5. 三道改造题（验收标准）

改完请**真的 `cargo run`**，按下面对应的验收标准逐条对。

### 改造题 1：加 `tags` 表，做关联查询

- 新建 `tags(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)` 和
  `note_tags(note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE, tag_id INTEGER NOT NULL REFERENCES tags(id), PRIMARY KEY (note_id, tag_id))`。
- 写 `fn set_tags(pool, note_id, tags: &[&str]) -> Result<()>`，用**一个事务**完成
  「删旧关联 → upsert 标签 → 插新关联」。
- 写 `fn notes_with_tag(pool, tag: &str) -> Result<Vec<Note>>`，用 `JOIN` 实现。

**验收标准**：

1. 建表后 `PRAGMA foreign_keys = ON` 生效——插入不存在的 `tag_id` 会报 `FOREIGN KEY constraint failed`（如果你把连接池初始化那段删掉，就**不会**报错，请亲眼对比一次）。
2. `DELETE FROM notes WHERE id = ?` 之后，`note_tags` 里对应的行**自动消失**（`ON DELETE CASCADE`）。
3. `notes_with_tag("rust")` 返回的 `Vec<Note>` 里没有重复行（一个 note 打两个相同 tag 时不允许——靠主键保证），且能正确打印。

### 改造题 2：加全文检索

- 用 SQLite 的 `LIKE '%' || ? || '%'` **先做一版**：`fn search_notes(pool, kw: &str) -> Result<Vec<Note>>`。
- 再用 FTS5 做一版：`CREATE VIRTUAL TABLE notes_fts USING fts5(title, body, content='notes', content_rowid='id')`，
  配 `CREATE TRIGGER` 保持同步，查询用 `SELECT n.* FROM notes n JOIN notes_fts f ON f.rowid = n.id WHERE notes_fts MATCH ?`。
- 在代码注释里写清你选了哪版、为什么。

**验收标准**：

1. `search_notes("Rust")` 能命中标题或正文包含 `Rust` 的行；搜 `"rust"`（小写）也必须命中（说明你处理了 `LIKE` 对 ASCII 大小写不敏感、对非 ASCII 敏感这件事）。
2. 关键词里含 `%` 或 `_` 时不会变成通配符（提示：`ESCAPE` 子句）。
3. FTS5 版在 `INSERT`/`UPDATE`/`DELETE` 后都能搜到最新结果（触发器或手动同步二选一，但要一致）。
4. **关键**：`search_notes` 用 `.bind()` 传参，代码里**没有任何 `format!` 拼 SQL**。

### 改造题 3：换成 Postgres，列出要改哪几处

不要求你真的装一个 Postgres，但要求你**在代码里改完并让 `cargo check` 通过**
（Postgres 类型检查不需要真连库，只要不启用 `query_as!`）。

**验收标准**：

1. `Cargo.toml` 依赖改成 `features = ["runtime-tokio", "postgres", "macros", "migrate"]`，同时把 `sqlite` feature 删掉或保留（保留则要解释为什么）。
2. 类型替换完整：`SqlitePool` → `PgPool`（`sqlx::postgres::PgPoolOptions`）；`last_insert_rowid()` → `INSERT ... RETURNING id` + `query_scalar`；
   `?` → `$1, $2`；`i64` 主键 → `INTEGER PRIMARY KEY AUTOINCREMENT` 改为 `BIGSERIAL PRIMARY KEY` 或 `GENERATED ALWAYS AS IDENTITY`。
3. `DATABASE_URL` 形如 `postgres://user:pass@localhost/dbname`。
4. **在 README 或代码注释里写出一份「改这几处」的清单**，并说明哪一处最容易漏（提示：`last_insert_rowid()` 是 SQLite 独有的，Postgres 没有对应函数；`datetime('now')` 也不是 Postgres 的函数）。
5. 说明：`current_setting`/`search_path` 之类的连接初始化 PRAGMA 段该怎么改。

---

## 6. 常见问题

| 现象 | 原因 / 处理 |
|---|---|
| `(code: 14) unable to open database file`（第一次运行） | `DATABASE_URL` 少了 `?mode=rwc`。sqlx 默认**不建库**，文件不存在就报这个错。**这是本 demo 实测踩到的第一个坑** |
| `error: no such table: notes`（编译期，来自 `query_as!`） | 编译时连的库里没表。先 `cargo run` 建表，或改用 `.sqlx` 离线缓存 |
| `error: error communicating with database: ...` 构建时报错 | `DATABASE_URL` 指向的文件/服务不可用；宏在构建期真的会连库 |
| `database is locked` | SQLite 写锁串行。把 `max_connections` 调小（SQLite 场景常用 1），或连接串加 `?busy_timeout=5000` |
| `no column found for name: xxx` | 运行时 `query_as::<_, T>` 的列名与结构体字段名不一致；SQLite 返回的列名就是 `AS` 后面的名字 |
| `the trait bound Option<String>: Decode<...>` / NULL 报错 | 可空列必须映射到 `Option<T>`；`query_as!` 会在编译期拦住你 |
| 数据在第二次运行后变多 | 符合预期：记录留在 `notes.db`。`rm notes.db` 重来 |
