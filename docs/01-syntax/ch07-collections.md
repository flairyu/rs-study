# 第 7 章 · 集合

> **一句话定位**：前六章处理"一个值"；这章起数据变成"一堆值"——`Vec` / `String` / `HashMap` / `HashSet`。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 3 章（所有权）、第 4 章（引用与切片）
> **练习工程**：`exercises/ch07_collections`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

Python 里你随手写 `d = {}`，C 里得手写哈希表，而 Rust 的选择是：常用容器进标准库，
**每个都把所有权规则摆在明面上**。

| 需求 | C / C++ | Python | Rust |
|---|---|---|---|
| 变长数组 | `malloc`+`realloc` / `std::vector` | `list` | `Vec<T>` |
| 哈希表 | 手写 / `std::unordered_map` | `dict` | `std::collections::HashMap<K, V>` |
| 有序表 | 手写红黑树 / `std::map` | 无（`dict` 是插入序） | `std::collections::BTreeMap<K, V>` |
| 集合 | 手写 / `std::unordered_set` | `set` | `std::collections::HashSet<T>` |
| 字符串 | `char*` + 长度 | `str` | `String`（拥有）/ `&str`（借用视图） |
| 越界访问 | UB（读到垃圾或崩溃） | 抛 `IndexError` | `v[i]` **panic**；`v.get(i)` 返回 `Option` |
| 放入元素 | 拷贝 / C++ move | 存引用 | **移动（move）进去**，原变量失效 |
| 边遍历边改 | UB / 迭代器失效 | `RuntimeError` | **编译错误**（借用检查器拦下） |

最短的对照：

```python
nums = [3, 1, 2]
nums.append("hello")      # Python：合法！类型错到运行时才炸
```

```rust
let mut nums = vec![3, 1, 2];
nums.push(4);             // 自动扩容，类型固定
// nums.push("hello");    // ❌ 编译错误：Vec<i32> 只装 i32
let _ = nums[10];         // 编译通过，运行时 panic（不是 UB）
```

（C 的版本是 `malloc` + `realloc` + `nums[3] = 4;` ——越界写，UB，编译器不管你。）

**所有权在集合上的体现**——本章真正的主线：

```rust
let s = String::from("hi");
let mut v = Vec::new();
v.push(s);                            // s 的所有权被移动进 v
// println!("{s}");                   // ❌ s 已被移走

let owned: Vec<String> = v.clone();   // 深克隆：每个 String 都复制一份堆内存
let borrowed: &[String] = &v;         // 零拷贝，只是借来看
```

> ⚠️ **记住这句**：`Vec<String>` 的克隆是**深克隆**（N 次分配），`Vec<i32>` 的克隆
> 只是一次 `memcpy`。C++ 的 `vector<string>` 同理；Python 的 `list` 存的是引用，完全不同。

## 1. `Vec<T>`：Rust 的变长数组

### 1.1 增删改查

```rust
let a = vec![1, 2, 3];                    // 宏，最常见
let b: Vec<i32> = Vec::new();             // 空，靠类型标注推断
let mut c = Vec::with_capacity(100);      // 预留 100 个槽位，不初始化
c.push(1);
c.extend([2, 3]);                         // 追加一批
c.insert(0, 0);                           // 下标 0 处插入，后面整体后移 O(n)

let last = c.pop();                       // Option<i32>，空时 None
let first = c.remove(0);                  // 按下标删并返回值，O(n)
c.retain(|&x| x % 2 == 0);                // 原地保留满足条件的
println!("{last:?} {first} {c:?}");       // 输出: Some(3) 0 [2]

c[0] = 42;                                // 改；越界 panic
println!("{} {}", c.len(), c.is_empty()); // 输出: 1 false
```

| 操作 | `Vec` | C++ `vector` | Python `list` |
|---|---|---|---|
| 尾部追加 | `push` | `push_back` | `append` |
| 尾部删除 | `pop() -> Option<T>` | `pop_back()`（无返回） | `pop()` |
| 按值找下标 | `iter().position()` | `find` | `list.index`（找不到抛异常） |
| 原地过滤 | `retain` | `remove_if` + `erase` | 列表推导式 |

### 1.2 扩容：为什么值得写 `with_capacity`

`Vec` 内部是「指针 + 长度 + 容量」，元素住在**堆**上：`ptr ──▶ [1|2|3|_|_|_|_|_]`，
`len = 3`（已初始化的元素个数），`cap = 8`（已分配的槽位数）。
`push` 时若 `len == cap`，就得**另分配一块更大的内存、把元素全部搬过去**
（容量近似翻倍，摊还下来仍是 `O(1)`，和 C++ `vector` 一致）。所以预知规模时该预留：

| 写入方式 | 效果 |
|---|---|
| `Vec::new()` 后循环 `push` | 接近容量时反复搬家 + `memcpy` |
| `Vec::with_capacity(n)` 后循环 `push` | 一次分配到位，之后全是 `O(1)` 写入 |

扩容还有**所有权层面的后果**：元素会被搬到新地址，所以不能一边持有 `&v[0]` 一边 `v.push(..)`
（C++ 里这同样让迭代器失效，但没人拦你；Rust 直接编译不过）：

```rust
let mut v = vec![1, 2, 3];
let first = &v[0];
// v.push(4);                 // ❌ error[E0502]: cannot borrow `v` as mutable
println!("{first}");          // 只要 first 还在用，push 就不允许
```

### 1.3 所有权与索引：`v[i]` vs `v.get(i)`

| 写法 | 返回类型 | 越界时 | 什么时候用 |
|---|---|---|---|
| `v[i]` | `T`（拷贝）或 `&T`（`&v[i]`） | **panic** | 你确信下标合法 |
| `v.get(i)` | `Option<&T>` | **`None`** | 下标来自外部或不确定 |
| `v.get_mut(i)` | `Option<&mut T>` | `None` | 同上，且要改 |

```rust
let v = vec![10, 20, 30];
let x = v[1];                    // 20（i32 是 Copy，拷贝出来）
let r = &v[1];                   // &i32，仅借用
println!("{x} {r}");             // 输出: 20 20
assert_eq!(v.get(1), Some(&20));
assert_eq!(v.get(99), None);     // 不 panic
// let boom = v[99];             // 运行时 panic: index out of bounds
if let Some(last) = v.last() { println!("last = {last}"); }   // 输出: last = 30
```

> ⚠️ **坑（C/C++ 迁移必看）**：`v[i]` 越界**不是 UB**，而是 panic——把"读到任意内存"
> 降级成"程序明确崩溃"。惯用写法是：**只要下标不是你刚检查过的，就用 `get`**，
> 再用 `if let` / `unwrap_or` 处理"没有"分支。C 里只能约定"返回 -1 表示失败"，
> `Option<&T>` 则把"可能不存在"放进类型系统。

## 2. `String` vs `&str` vs `str`：三层关系

| 类型 | 是什么 | 是否拥有 | 大小（64 位） | 典型来源 |
|---|---|---|---|---|
| `str` | **不定长**的 UTF-8 字节序列（字符串本体） | 没人能直接拥有 | 编译期未知 | 只能以 `&str` 出现 |
| `&str` | 指向某段 `str` 的**借用**（胖指针：地址 + 长度） | 否 | 16 字节 | `"hi"`、`&s[..]`、`&s` |
| `String` | **拥有**堆内存的可变字符串 | 是 | 24 字节（ptr+len+cap） | `String::from`、`to_string`、`format!` |

一句话：**`str` 是内容本身（不定长、不能独立存在），`&str` 是看它的窗口，
`String` 是把它装进自己堆内存的盒子。** 这和 `[T]` / `&[T]` / `Vec<T>` 是同一套设计：
`[T] ≈ str`（不定长本体，只能借）、`&[T] ≈ &str`（借用视图，胖指针）、
`Vec<T> ≈ String`（拥有所有权的容器）。

### 2.1 为什么函数参数用 `&str` 更好

```rust
// ❌ 只收 String：调用方要么交出所有权，要么先 clone 一份
fn shout_bad(s: String) -> String { s.to_uppercase() }

// ✅ 收 &str：String 能传（自动 Deref 成 &str），&str、字面量也能传
fn shout(s: &str) -> String { s.to_uppercase() }

let owned = String::from("hello");
println!("{}", shout(&owned));   // 零拷贝借用
println!("{}", shout("world"));  // 字面量直接用
println!("{owned}");             // ✅ owned 还能用：shout 只是借去看
// 若签名是 String，这里就得写 shout_bad(owned.clone())
```

三条理由：

1. **不夺走所有权**——调用后原 `String` 依然可用；写 `String` 就是 move 进去。
2. **覆盖面更广**——`String` / `&str` / 字面量都能喂给 `&str` 参数（靠 `Deref` 自动转换）。
3. **不承诺多余的能力**——`&str` 说明"只读、不改、不留"，签名本身就是文档。

对应两条规矩：**参数优先 `&str`**（要原地改才用 `&mut String`）；
**返回新造的字符串用 `String`**（函数内的局部值活不过函数，只能交出所有权）。

### 2.2 常用操作与 UTF-8 陷阱

```rust
let mut s = String::from("hello");
s.push_str(" world");                 // 追加 &str（push 则追加单个 char）
s = s.replace("world", "rust");       // 返回新 String，不原地改
println!("{} | {}", s, s.to_uppercase());
// 输出: hello rust! | HELLO RUST!

let words: Vec<&str> = "  a\tb\n c ".split_whitespace().collect();
println!("{words:?}");                // 输出: ["a", "b", "c"]

let zh = "你好世界";
println!("{}", &zh[0..6]);            // 输出: 你好（每个汉字 3 字节）
println!("{} {}", zh.len(), zh.chars().count());   // 输出: 12 4
// &zh[0..1] 会 panic: byte index 1 is not a char boundary
```

> ⚠️ **坑**：`String::len()` 是**字节数**，不是字符数！`"你好世界"` 是 4 个字符、12 个字节。
> 数字符用 `chars().count()`；按位置切分必须切在字符边界上。
> Python 的 `len("你好世界") == 4`，C 的 `strlen` 是 12，Rust 的 `.len()` 站在 C 这边。

## 3. `HashMap<K, V>` 与 `entry` API

### 3.1 基本用法

```rust
use std::collections::HashMap;

let mut scores = HashMap::new();
scores.insert("alice", 90);                 // key: &str, value: i32
scores.insert("bob", 85);
scores.insert("alice", 95);                 // 已存在：覆盖旧值
println!("{:?}", scores.get("alice"));      // 输出: Some(&95)
println!("{}", scores["alice"]);            // 95（不存在则 panic）
scores.remove("bob");                       // 返回 Option<i32>
for (k, v) in &scores { println!("{k} = {v}"); }
// 输出: alice = 95（多项时顺序不保证）
```

| 行为 | Python `dict` | C++ `unordered_map` | Rust `HashMap` |
|---|---|---|---|
| 迭代顺序 | 插入序（3.7+） | 不确定 | **不确定**（随机种子，防 HashDoS） |
| 取不存在的 key | `d[k]` 抛 `KeyError` | `m[k]` 会**插入默认值** | `m[&k]` **panic**；`m.get(&k)` 给 `Option` |
| 可变地取一项 | 天然可以 | `m[k]` | `m.get_mut(&k)` → `Option<&mut V>` |

`HashMap<K, V>` 要求 `K: Eq + Hash`，于是 `String` / `&str` 可以当 key，
**`f64` 不行**（不实现 `Eq`，因为 `NaN != NaN`，也不实现 `Hash`）。另外 key 是**被 map 拥有**的：

```rust
let mut counts: HashMap<String, usize> = HashMap::new();
counts.insert("hello".to_string(), 1);   // &str 得先"变成" String
// counts.insert("hello", 1);            // ❌ 类型不符：&str ≠ String
println!("{:?}", counts.get("hello"));   // 输出: Some(&1) —— 查询不必造 String
```

最后一行是标准库刻意的设计：`get<Q>(&self, k: &Q) where K: Borrow<Q>`，
`String: Borrow<str>`，所以查询不会产生分配（`HashSet` 同理）。

> ⚠️ **坑**：要稳定顺序就别指望 `HashMap`——换 `BTreeMap`（按 key 排序）、
> 自己把 key 收集起来 `sort()`，或者引入第三方的 `indexmap`。

### 3.2 `entry` API：一次查找完成"有就改、没有就插"

新手第一反应是 `contains_key` + `insert`——那是**两次哈希查找**。惯用写法是 `entry`：

```rust
fn count(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for w in text.split_whitespace() {
        *counts.entry(w.to_string()).or_insert(0) += 1;
    }
    counts
}
```

`entry` 返回 `Entry::Occupied` / `Entry::Vacant`，常用方法：`or_insert(v)`（空槽插入 `v`，
返回 `&mut V`）、`or_insert_with(f)`（惰性算值）、`or_default()`（插 `V::default()`）、
`and_modify(f).or_insert(v)`（先改后插）。**`or_insert` 返回 `&mut V` 是关键**——
`*... += 1` 直接改写槽里的值，不必第二次查找。

> ⚠️ **坑**：`entry()` 即使 key 已存在也要你提供 key（`w.to_string()` 先分配了），
> 覆盖那次分配就白费。性能敏感时可以先用 `get_mut` 探一下：
> ```rust
> if let Some(c) = counts.get_mut(word) {
>     *c += 1;                              // 命中：零分配
> } else {
>     counts.insert(word.to_string(), 1);   // 未命中：才分配
> }
> ```

---

## 4. 什么时候用 `BTreeMap` 而不是 `HashMap`

两者 API 几乎一样（`insert` / `get` / `remove` / `entry` 都有），区别在底层：

| | `HashMap` | `BTreeMap` |
|---|---|---|
| 底层 | 哈希表 | B 树（有序） |
| 迭代顺序 | **不确定** | **按 key 升序** |
| 单次操作 | 平均 `O(1)`，最坏 `O(n)` | 稳定 `O(log n)` |
| 范围查询 | 没有 `range` | `range(a..b)` 原生支持 |
| 取最小/最大 | 手动扫 `O(n)` | `first_key_value` / `last_key_value` |
| key 约束 | `Eq + Hash` | `Ord` |

选型口诀：**只求快、不管顺序 → `HashMap`**；**要有序遍历 / 范围查询 / 最小 key → `BTreeMap`**；
**key 是浮点（有 `Ord` 没 `Eq + Hash`）→ 只能 `BTreeMap`**。

```rust
use std::collections::BTreeMap;

let mut m = BTreeMap::new();
m.insert("banana", 2);
m.insert("apple", 1);
m.insert("cherry", 3);
for (k, _) in m.range("b".."d") { print!("{k} "); }
println!();   // 输出: banana cherry（按 key 升序；HashMap 做不到 range）
```

> ⚠️ **坑（写测试会遇到）**：`HashMap` 迭代顺序不确定，断言整个 map 相等请用
> `assert_eq!(map1, map2)`（`HashMap` 实现了 `PartialEq`，比的是内容不是顺序）；
> 若要把内容格式化成字符串再比，务必先收集成 `Vec` 再 `sort`，否则测试会随机失败。

## 5. `HashSet<T>`

`HashSet<T>` 就是"只有 key、没有 value"的 `HashMap<T, ()>`，用于**去重**和**成员判断**。
`insert` 返回 `bool`（`true` = 这次真插进去了），这一点在去重时特别好用：

```rust
use std::collections::HashSet;

let mut seen = HashSet::new();
seen.insert("a");
println!("{} {}", seen.insert("a"), seen.contains("a"));   // 输出: false true

let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
let b: HashSet<i32> = [3, 4].into_iter().collect();
let mut inter: Vec<i32> = a.intersection(&b).copied().collect();
inter.sort_unstable();
println!("{inter:?}");                 // 输出: [3]

/// 用 insert 的返回值当过滤器，顺序天然保留。
fn dedup_via_insert(items: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    items.iter().filter(|it| seen.insert(it.as_str())).cloned().collect()
}
```

对照 Python：`set.add()` 返回 `None`，判断"是否新元素"要 `in` + `add` 两次哈希；
Rust 的 `insert` 一次调用同时完成判断、插入和返回结果。

---

## 6. 常见坑汇总

1. **`v[i]` panic，`v.get(i)` 返回 `Option`**——下标来自外部输入时永远用 `get`。
2. **`String::len()` 是字节数**，切片也必须切在字符边界上，切在汉字中间会 panic。
3. **`HashMap` 迭代顺序不确定**，测试不要依赖；要保序就换 `BTreeMap` 或自己排序。
4. **key 必须 `Eq + Hash`**，所以 `f64` 不能当 `HashMap` 的 key。
5. **放进去 = 移进去**：`v.push(s)` 之后 `s` 不能再用；要保留就提前 `&s` 或 `clone()`。
6. **扩容会让已有借用失效**，`&v[0]` 与 `v.push(..)` 不能共存（编译错误，不是 UB）。
7. **`contains_key` + `insert` 是两次查找**，惯用 `entry` 一次搞定。
8. **`for x in v` 会移动整个 `Vec`**：只读写 `for x in &v`，要改写 `for x in &mut v`。

---

## 7. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 7-1 | 同时求均值与中位数 | `&[i32]` 切片借用、`Option` 表示空输入、偶数中位数、排序副本 | `mean_and_median` |
| 7-2 | 统计词频（忽略大小写） | `to_lowercase`、`split_whitespace`、`entry().or_insert(0) += 1` | `word_count` |
| 7-3 | 保序去重 | `HashSet` 记录已见、`String` 克隆、保序收集 | `dedup_preserve_order` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch07_collections      # 做题前先跑一次，看它怎么失败
python3 tools/ai_review.py ch07_collections --hint 2   # 卡住了再跑
```

### 提示（不给答案）

- **7-1**：先 `if nums.is_empty() { return None; }`。均值别忘 `as f64`。
  中位数先克隆再排：`let mut sorted = nums.to_vec(); sorted.sort_unstable();`，
  用 `sorted.len() / 2` 取中间下标；偶数个元素要取 `[mid - 1]` 和 `[mid]` 求平均。
- **7-2**：先 `let lowered = text.to_lowercase();` **绑定成变量**，再
  `lowered.split_whitespace()` 遍历——不绑定的临时 `String` 会让借出的 `&str` 一起失效。
  计数用 `*counts.entry(word.to_string()).or_insert(0) += 1;`。
- **7-3**：准备一个 `HashSet` 和结果 `Vec`，遍历 `items`；若 `seen.insert(..)` 返回 `true`
  （第一次见），就把元素 `clone()` 进结果。用 `HashSet<&str>`（`insert(item.as_str())`）
  可以少克隆一次：key 只是借用，被借的 `String` 活在整个循环里。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 顶部统一写：use std::collections::{HashMap, HashSet};

// 7-1：入参是共享借用，所以排序必须排副本；空输入返回 None。
pub fn mean_and_median(nums: &[i32]) -> Option<(f64, f64)> {
    if nums.is_empty() {
        return None;
    }
    // 用 i64 累加，避免元素多时 i32 溢出；最后再转 f64。
    let sum: i64 = nums.iter().map(|&x| x as i64).sum();
    let mean = sum as f64 / nums.len() as f64;
    // nums 是 &[i32]，不能直接 sort（那是 &mut 方法）——先 to_vec() 拷一份。
    let mut sorted = nums.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    let median = if sorted.len() % 2 == 1 {
        sorted[mid] as f64
    } else {
        (sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0
    };
    Some((mean, median))
}

// 7-2：先转小写得到一个新 String，再按空白分词。
// key 用 String：返回的 map 要比输入的 &str 活得久，必须自己拥有 key。
pub fn word_count(text: &str) -> HashMap<String, usize> {
    let lowered = text.to_lowercase();
    let mut counts = HashMap::new();
    for word in lowered.split_whitespace() {
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
    counts
}

// 7-3：HashSet 只管"见过没有"，顺序由结果 Vec 保证。
// 让 HashSet 借用元素（&str），只有结果才需要真正克隆。
pub fn dedup_preserve_order(items: &[String]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        if seen.insert(item.as_str()) {
            result.push(item.clone());
        }
    }
    result
}
```

**为什么 7-1 用 `sort_unstable`？** 它原地快排、不分配临时缓冲区（`sort` 稳定但要多一块
临时内存）；一串 `i32` 里相等元素没有可观察顺序，不必为稳定性付费。
**为什么 7-3 用 `HashSet<&str>`？** 借用比克隆一整份 `String` 便宜，而被借的 `String`
活在 `items` 里，生命周期完全够用。

</details>

---

## 自测清单

- [ ] 我能说出 `v[i]` 和 `v.get(i)` 在越界时分别发生什么，以及该用哪个
- [ ] 我能解释 `str` / `&str` / `String` 的关系，以及为什么参数优先写 `&str`
- [ ] 我知道 `String::len()` 数的是字节；`v[i]` 越界是 panic 而不是 UB
- [ ] 我能说出 `with_capacity` 在什么场景值得写，以及扩容为什么会破坏借用
- [ ] 我能用 `entry` API 写词频统计，并说出它比 `contains_key` + `insert` 好在哪
- [ ] 我知道 `HashMap` 迭代顺序不确定（要序用 `BTreeMap`），`f64` 不能当 key
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch07_collections` 全绿
