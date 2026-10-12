# 第 4 章 · 引用与借用

> **一句话定位**：`&T` 是「借来读」，`&mut T` 是「借来改」——借用不转移所有权，但编译器会逼你回答「同一时刻有几个人能碰这份数据」。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 3 章 · 所有权
> **练习工程**：`exercises/ch04_borrowing`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

第 3 章的 `shout(s: String)` 把 `String` **吃掉了**，调用完原变量作废。可一个函数只想看一眼字符串，
凭什么把所有权拿走？Rust 的答案是**借用（borrow）**：把「访问权」用引用（reference）借出去，
函数返回时权限自动还回来，所有者全程不变。

| 话题 | C | C++ | Python | Rust |
|---|---|---|---|---|
| 「借用」写出来是什么 | `const T *p` | `const T&` | 变量名本身就是引用 | `&T` |
| 「可改的借用」 | `T *p` | `T&` | 任何名字都能改 | `&mut T` |
| 编译期能否发现悬垂引用 | ❌ 只能靠人 | ❌ 只能靠人 | 不适用（GC） | ✅ 借用检查器直接报错 |
| 同一份数据能有几个「可改路径」 | 随便几个（然后出事） | 随便几个（然后出事） | 随便几个（受 GIL 保护） | **最多一个** `&mut T` |
| 只读别名 | `const` 限定 | `const` 限定 | 到处都是 | **默认就是** `&T`，随便多个 |
| 传参要不要拷贝 | 传指针才不拷贝 | 传 `const T&` 才不拷贝 | 传引用不拷贝，改的是同一个对象 | 默认就是借用，不写 `&` 才是移动/拷贝 |
| 生命周期谁负责 | 程序员 | 程序员（RAII 缓解） | GC | 借用检查器在编译期验证 |

读 vs 改，在 Rust 里是**两个不同的类型**，不是同一个类型加个限定符：

```rust
fn main() {
    let s = String::from("hello");
    let r: &String = &s;            // 共享借用：只能读
    let mut m = String::from("hello");
    let mr: &mut String = &mut m;   // 可变借用：可读可改
    mr.push('!');
    println!("{r} / {mr}");         // 输出: hello / hello!
}
```

对照 C++：类型是 `T`，`const` 和 `&` 只是修饰符；Rust 里 `&T` 与 `&mut T` 是两个**不同类型**
（`shout(const std::string&)` vs `append(std::string&)`）。

**Python 使用者请注意**：Python 里没有「借用」，因为变量名本来就是贴到对象上的标签，
函数改 `list.append` 改的就是同一个 list。Rust 把「共享」与「独占」提升到了类型上，
代价是你要写 `&mut`，收益是编译器替你排除数据竞争和迭代器失效。

**本章只记一条规则就够**（后面所有报错都是它的推论）：

> 同一作用域内：要么任意多个 `&T`，要么恰好一个 `&mut T`——不可兼得。

| | 共享借用 `&T` | 可变借用 `&mut T` |
|---|---|---|
| 同时允许多少个 | 任意多个 | **恰好一个** |
| 能和另一种共存吗 / 能改吗 | ❌ / ❌（除非内部可变性，第 11 章） | ❌ / ✅ |
| 类比 | 多个读者 | 一个写者 |

---

## 1. `&T` 与 `&mut T`

引用是一个带类型和生命周期的地址，它**不拥有数据**，所以也不负责释放数据。

```rust
fn word_count(text: &str) -> usize { text.split_whitespace().count() }  // 只读，不需要所有权

fn main() {
    let owned = String::from("a b c");
    println!("{}", word_count(&owned));  // 输出: 3
    println!("{owned} 还能用");           // 输出: a b c 还能用
}
```

`&owned` 是 `&String`，函数要 `&str`，这里靠 **deref coercion** 自动转换。这也是
`&String → &str`、`&Vec<T> → &[T]`、`&[T; N] → &[T]` 都能直接传的原因。

| 写法 | 名字 | 含义 |
|---|---|---|
| `String` | 所有者 | 拥有堆上的缓冲区，离开作用域就释放 |
| `&String` / `&str` | 共享借用 | 指向这段文本，只读，不能单独活过所有者 |
| `&mut String` | 独占借用 | 指向这段文本，可以改（甚至换掉内部缓冲区） |

`&mut` 不只是「能改」，它同时意味着**排他**：持有 `&mut s` 时，别人连读 `s` 都不行。

```rust
fn main() {
    let mut s = String::from("hi");
    let m = &mut s;
    // println!("{s}");   // ❌ 这段编译不过：借出 &mut 后不能再共享读
    m.push('!');
    println!("{m}");      // 输出: hi!
}
```

> ⚠️ **坑**：`&mut T` 的「唯一」是**编译期**规则，不是说内存里物理上只存在一个指针。
> 重借用（`&mut *m`、把 `m` 传进函数）会产生多个互不重叠的可变引用，借用检查器保证的是
> 「同一时刻只有一条活跃路径能改它」。别把它理解成 C 的 `restrict` 提示。

---

## 2. 借用规则：为什么必须是「多个读」或「一个写」

这条规则同时解决两类 C/C++ 经典事故：**迭代器失效**（边遍历 `Vec` 边 `push`，扩容后旧指针悬垂）
和**数据竞争**（多线程同时读写同一块内存）；把规则提前到编译期，它们就从「偶尔崩在客户机器上」
变成「编译不过」。共享借用可以并存，因为只读不会改变任何已有结论：

```rust
fn main() {
    let v = vec![1, 2, 3];
    let a = &v; let b = &v; let c = &v;       // 多个共享借用，随便多少个
    assert_eq!((a[0], b[1], c.len()), (1, 2, 3));
}
```

想改的时候必须独占——下面是**两个会编译失败的例子**：

```rust,compile_fail
// ❌ 这段编译不过：cannot borrow `v` as mutable because it is also borrowed as immutable
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];          // 共享借用在后面还要用
    v.push(4);                  // 这里要 &mut v —— 冲突
    println!("{first}");
}
```

改 `Vec` 可能触发扩容，`first` 指向的旧缓冲区就报废了，编译器拒绝它是**对的**。

```rust,compile_fail
// ❌ 这段编译不过：cannot borrow `s` as mutable more than once at a time
fn main() {
    let mut s = String::from("hello");
    let a = &mut s;      // 第一条 &mut
    let b = &mut s;      // 第二条 &mut —— 不允许
    a.push('a');
    b.push('b');
    println!("{a}{b}");
}
```

需要把修改串起来写时，Rust 也允许——下面这段是编译能过的：

```rust
fn main() {
    let mut v = vec![1, 2, 3];
    v.push(4);                 // 先改（这条 &mut 用完就结束）
    let first = &v[0];         // 再借来读（借用从这一行才开始）
    assert_eq!(*first, 1);
}
```

> ⚠️ **坑**：规则管的是**借用存活的区间**，不是「写过 `&mut` 这几个字符」。
> 不理解这点会觉得编译器前后矛盾；理解了就知道它只是在算时间轴（第 5 节）。

---

## 3. 借用不能比所有者活得久

所有权在第 3 章已经确定：数据只有一个主人，主人走了数据就没了。引用只是「临时的看管权」，
所以借用活不过所有者——下面的函数想返回局部变量的引用，**编译不过**：

```rust,compile_fail
// ❌ 这段编译不过：missing lifetime specifier —— 返回值是引用，却没有任何输入借用可绑定
fn bad() -> &String {
    let s = String::from("temp");   // s 在函数结束时被 drop
    &s                              // 若放行，返回的就是悬垂引用
}
```

要修只有两条路：把所有权交出去，或者让调用方提供数据。

```rust
// 路 1：返回所有权（第 3 章的做法）
fn good_move() -> String { String::from("temp") }

// 路 2：借用调用方给的数据（第 10 章会显式写 'a，这里靠省略规则）
// 含义：返回的引用活得和入参一样久。
fn good_borrow(s: &str) -> &str { s }
```

省略规则（第 10 章展开）：唯一输入引用决定输出；有 `&self` / `&mut self` 时输出绑定 `self`。
练习 4-1 的 `longest_word(text: &str) -> &str` 正是第一种：**只有一个输入引用**，
所以返回值必然来自 `text`，绝不可能来自函数内部临时造出来的 `String`。

> ⚠️ **坑**：`fn first_word(s: &str) -> &str { &s[0..3] }` 合法；但函数里若先
> `let t = s.to_string();` 再返回 `&t[0..3]`，一定编译不过——签名要改成 `-> String`（一次拷贝）。

---

## 4. 切片 `&[T]` 与 `&str`：借来的一段

切片是引用的一种常见形态：**指向连续一段元素的胖指针**（地址 + 长度），零拷贝。

| 类型 | 指向什么 | 大小（`usize` 个数） | 能改吗 |
|---|---|---|---|
| `&[T]` / `&mut [T]` | 数组或 `Vec` 的一段 | 2（指针 + 长度） | 前者只读，后者可改元素 |
| `&str` | 一段 UTF-8 字节 | 2（指针 + 长度） | 不能改 |
| `String` / `Vec<T>` | 自己拥有一段 | 3（多一个容量） | 随便改 |

切片可以被「再切」无数次，每次都是新视图：

```rust
fn main() {
    let text = "hello rust";
    let word = &text[0..5];          // 从 &str 切出 &str
    let nums = [1, 2, 3, 4, 5];
    let (left, right) = nums.split_at(nums.len() / 2);   // 切成两个 &[i32]
    println!("{word} {left:?} {right:?}");   // 输出: hello [1, 2] [3, 4, 5]
}
```

由此得到一条 API 设计结论：**只读就收 `&[T]` / `&str`，不要收 `&Vec<T>` / `&String`**。
前者能接收 `&Vec`、`&[T; N]`、`Box<[T]>`；后者只能接收 `&Vec`。

> ⚠️ **坑**：`text[i..j]` 的下标是**字节**偏移，不是字符。对 `"中文"` 写 `&s[0..1]` 会 panic
> （byte index 1 is not a char boundary）；按字符切要用 `chars()` / `char_indices()`。
> `split_at` 对 `&[T]` 没有这个问题，只要 `mid <= len` 就合法。

---

## 5. NLL：借用结束在「最后一次使用」，不是「作用域末尾」

NLL = **Non-Lexical Lifetimes**（非词法生命周期），2018 edition 起默认启用：借用区间从
「花括号」缩短为「**从借用到最后一次使用**」。第 2 节那个「先 `push` 再 `&v[0]`」的例子就是它。

**但 NLL 不是魔法**：下面这段依然编译不过，因为迭代期间借用一直活着。修法是
**把「决定」和「修改」分开**：先收集结论，迭代结束再改。

```rust,compile_fail
// ❌ 这段编译不过：cannot borrow `v` as mutable because it is also borrowed as immutable
fn main() {
    let mut v = vec![1, 2, 3];
    for x in &v {            // 迭代期间 &v 一直有效
        if *x == 1 { v.push(99); }   // 需要 &mut v —— 冲突
    }
}
```

```rust
fn main() {
    let mut v = vec![1, 2, 3];
    let should_push = v.iter().any(|&x| x == 1);   // 借用在这里结束
    if should_push { v.push(99); }
    println!("{v:?}");       // 输出: [1, 2, 3, 99]
}
```

> ⚠️ **坑**：`for x in &v` 借用整个 `Vec`；`for x in v` 会把它**移动**走（之后不能再 `v.push`）。
> 想边遍历边改，通常得用 `Vec::retain`、索引循环，或者先收集再改。

---

## 6. 悬垂引用为什么编译不过

一句话：**编译期能看出「引用比所有者活得久」就直接拒绝**。Rust 没有运行时机制检测悬垂引用，
它靠签名里的生命周期信息做静态证明。

```rust,compile_fail
// ❌ 这段编译不过：`s` does not live long enough / borrowed value does not live long enough
fn main() {
    let r;                      // 声明在外层，作用域覆盖整个 main
    {                           // 内层作用域：s 只活到这里
        let s = String::from("short");
        r = &s;                 // r 借用了 s，但 s 马上要被 drop
    }
    println!("{r}");            // 使用 r —— 它已经悬垂了
}
```

让所有者的作用域包住借用的作用域就合法了（`let s` 在外层，`let r = &s` 寿命更短）：

```rust
fn main() {
    let s = String::from("long enough");
    let r = &s;
    println!("{r}");            // 输出: long enough
}
```

对照 C++：`std::string& bad() { std::string s = "temp"; return s; }` 照样编译通过，
运行到 `printf` 时才是 UB。这就是本章最该记住的一句：**Rust 把「悬垂引用」从运行时 UB
变成编译期错误**；代价是你要学会看借用检查器的报错。

---

## 7. 常见坑

- **一边遍历一边改**：`for x in &v { v.push(...) }` 编译不过；先收集再改（第 5 节）。
- **`&mut` 与 `&` 混用**：拿到 `&mut v` 后，连 `v.len()` 都不行，直到这条借用结束。
- **返回借用临时值**：`fn f() -> &str { &String::from("x") }` 必然失败；函数内先 `.to_string()`
  再返回切片同理，签名要改成 `-> String`（代价是一次拷贝）。
- **以为 `&String` 就是 `&str`**：类型不同，靠 deref coercion 才能传；签名一律写 `&str`。
- **下标当字符下标用**：`&s[0..1]` 对多字节字符会 panic（第 4 节）。
- **`split_whitespace` 的产物永远非空**：它自动跳过连续空白，所以 `""` / `"   "` 得到空迭代器
  （练习 4-1 的边界值天然成立）；`.len()` 是**字节**长度（`"你好".len() == 6`）。

---

## 8. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 4-1 | 返回第一个最长的单词（切片，不拷贝） | `&str` 进 `&str` 出、省略规则、`split_whitespace` | `longest_word` |
| 4-2 | 在字符串末尾追加一个 `!` | `&mut String` 可变借用、`push` | `append_exclaim` |
| 4-3 | 从 `len() / 2` 处把切片切成两半 | `&[i32]`、`split_at`、多个共享借用 | `halves` |

三题的签名（照抄，别改）：

```rust
pub fn longest_word(text: &str) -> &str
pub fn append_exclaim(s: &mut String)
pub fn halves(nums: &[i32]) -> (&[i32], &[i32])
```

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch04_borrowing              # 跑评测
python3 tools/ai_review.py ch04_borrowing --hint 2 # 卡住了就让 AI 给提示
```

### 提示（不给答案）

- **4-1**：`text.split_whitespace()` 产出 `&str` 迭代器。要返回切片，可以用一个 `&str` 变量
  记住当前最长者（初值 `""`），长度比较用 `>`，这样等长时自然保留靠前的；循环结束后直接返回它。
  也可以记住「起始偏移 + 长度」再用 `&text[start..start + len]` 取。
- **4-2**：只有一行 `s.push('!')`。参数是 `&mut String`，`push` 直接改调用方那个 `String`。
- **4-3**：一行 `nums.split_at(nums.len() / 2)`。空切片时切点是 0，两段都为空；
  单元素时切点也是 0，元素全在后段——这两条都要测到。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 4-1：返回 text 内部的切片，零拷贝。
// `best` 初值是 ""（'static），赋成 `word` 后被借用检查器收窄为与 `text` 同寿命。
// 用 `>` 而不是 `>=`：等长时保留靠前的那个。
// 空输入 / 全空白时循环体一次都不执行，best 仍是 ""，天然满足边界要求。
pub fn longest_word(text: &str) -> &str {
    let mut best = "";
    for word in text.split_whitespace() {
        if word.len() > best.len() {
            best = word;
        }
    }
    best
}

// 4-2：借用而不是拿走所有权，所以调用方之后还能继续用这个 String。
pub fn append_exclaim(s: &mut String) {
    s.push('!');
}

// 4-3：split_at 返回的两个 &[i32] 都是对 nums 的共享借用，可以同时存在——
// 这正是「任意多个 &T」那条规则在标准库里的样子。
pub fn halves(nums: &[i32]) -> (&[i32], &[i32]) {
    nums.split_at(nums.len() / 2)
}
```

**4-1 的关键点**：`longest_word(text: &str) -> &str` 只有一个输入引用，按省略规则输出必须绑定
`text`——下表里返回的全是 `text` 的视图，不是新字符串。

与 `tests/chapter.rs` 一致的期望值：

| 调用 | 期望结果 |
|---|---|
| `longest_word("hello rust world")` | `"hello"`（与 `world` 等长，取靠前） |
| `longest_word("the quick brown fox")` | `"quick"`（`quick` 与 `brown` 都是 5 字节，取靠前的 `quick`） |
| `longest_word("solo")` | `"solo"` |
| `longest_word("a  bbb   cc")` | `"bbb"`（连续空白不算进单词） |
| `longest_word(" \t\n rust \r\n ")` | `"rust"` |
| `longest_word("hi!! there")` | `"there"`（5 字节 > 4 字节，标点不断词） |
| `longest_word("")` / `longest_word("   \t\n  ")` | `""` |
| `append_exclaim(&mut String::from("hello"))` | `"hello!"` |
| `append_exclaim(&mut String::new())` | `"!"` |
| `append_exclaim(&mut String::from("你好"))` | `"你好!"` |
| `halves(&[1, 2, 3, 4])` | `(&[1, 2], &[3, 4])` |
| `halves(&[1, 2, 3])` | `(&[1], &[2, 3])` |
| `halves(&[5, 6, 7, 8, 9])` | `(&[5, 6], &[7, 8, 9])` |
| `halves(&[])` | `(&[], &[])` |
| `halves(&[7])` | `(&[], &[7])` |

</details>

---

## 自测清单

- [ ] 我能说出「同一作用域内：要么任意多个 `&T`，要么恰好一个 `&mut T`」，并写出两个编译不过的例子
- [ ] 我能解释为什么拿到 `&mut s` 之后连 `println!("{s}")` 都不行
- [ ] 我知道 `&T` 不拥有数据，所以借用必须比所有者短命，以及违反时编译器报什么错
- [ ] 我能说出 `&String` / `&str` / `String` 的区别，以及签名为什么要收 `&str`
- [ ] 我能解释 NLL 把借用区间缩短成「最后一次使用」，以及它救不了「边遍历边 push」
- [ ] 我知道 `split_at` 为什么能安全地返回两个 `&[i32]`
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch04_borrowing` 全绿
