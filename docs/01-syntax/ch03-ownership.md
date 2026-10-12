# 第 3 章 · 所有权

> **一句话定位**：所有权不是"Rust 的特色语法"，而是**用编译期规则替代 GC 与手工 `free`** 的那套规则。
> **预计时间**：100 分钟（读 45 + 写 55）｜**前置**：第 2 章（函数与控制流），本章的坑几乎都在函数边界上
> **练习工程**：`exercises/ch03_ownership`（3 题，只依赖 std）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

### 0.1 一张对照表

| 话题 | C | C++ | Python | Rust |
|---|---|---|---|---|
| 堆内存谁回收 | 你调 `free` | 析构 + `delete`（RAII） | 引用计数 + GC | 编译器在作用域结束处插入 `drop` |
| `a = b` 的语义 | 复制字节 | 默认拷贝（除非 `std::move`） | 两个名字指同一对象 | **默认移动**，`b` 作废 |
| 传参 | 复制，或自己传指针 | 默认按值拷贝 | 传对象引用 | 拥有型值默认**转移所有权** |
| 想保留原值 | 再复制一份 | 再拷贝 / 别 `move` | 什么都不用做 | 显式 `.clone()` 或改用借用（第 4 章） |
| 释放后使用 | UB | UB | 不可能（GC 保活） | **编译错误** |
| 释放两次 | UB | UB（析构写错就炸） | 不可能 | **编译错误** |

**C/C++ 把"谁负责释放"写进注释和约定，Rust 把它写进类型系统。**
代价是得学会和编译器商量所有权；收益是 use-after-free、double-free、内存泄漏
这三类最常见的 C/C++ 事故，在安全 Rust 里根本编译不出来。

### 0.2 同一个任务，四种语言：给字符串加一个 `!`

```c
char *append_bang(const char *s) {      /* C：谁 free 全靠口头约定 */
    size_t n = strlen(s);
    char *out = malloc(n + 2);
    memcpy(out, s, n);
    out[n] = '!'; out[n + 1] = '\0';
    return out;                         /* 调用方忘了 free 就是泄漏 */
}
```

```cpp
std::string append_bang(std::string s) {    // C++：按值传入 = 拷贝一份
    s += '!';
    return s;                               // NRVO 通常省掉这次拷贝
}
std::string a = "hi";
std::string b = append_bang(std::move(a));  // 不写 std::move 就是深拷贝
// move 之后 a 是"有效但未指定"状态：只能赋新值或析构
```

```python
def append_bang(s: str) -> str:
    return s + "!"      # str 不可变，+ 产生新对象
a = "hi"
b = append_bang(a)      # a 完全不受影响，也没有"销毁"一说（GC 管）
```

```rust
fn append_bang(mut s: String) -> String {
    s.push('!');
    s                       // 没有分号：所有权随返回值交回调用方
}
let a = String::from("hi");
let b = append_bang(a);     // a 被移动进函数
// println!("{a}");         // ❌ error[E0382]: borrow of moved value: `a`
println!("{b}");            // 输出: hi!
```

把 C++ 的 `std::move(a)` 和 Rust 的 `append_bang(a)` 并排看：**Rust 省掉了那个 `std::move`，代价是编译器从此盯着 `a` 不让你再用**。本章剩下的内容都在解释这句话。

---

## 1. 栈与堆：所有权要解决的问题从哪来

```rust
let n: i32 = 42;              // 42 整个在栈上，大小编译期已知
let s = String::from("hi");   // String 的"头"在栈上，字节在堆上
```

`String` 在栈上只有三个机器字（64 位机器上 24 字节）：

```
栈（函数返回自动回收）           堆（运行期才知道大小，不自动回收）
┌──────────────────┐
│ ptr ─────────────┼──────►  ┌───┬───┐
│ len = 2          │         │ h │ i │   容量可能大于 len
│ cap = 2          │         └───┴───┘
└──────────────────┘
```

栈帧在函数返回时被弹掉，`ptr/len/cap` 一起消失，**但堆上那两个字节还在**。麻烦就出在这：

- C：让你自己 `free(ptr)`。忘了 → 泄漏；忘两次 → double free；提前 → use-after-free。
- Python/Java：运行时 GC 盯着谁还在引用这块内存。
- Rust：规定每个堆上的值在任意时刻**只有一个所有者（owner）**；所有者离开作用域，值即销毁。
  规则在编译期检查，运行时零成本。

> ⚠️ **坑**：`String` 固定 24 字节，所以"移动 `String`"移动的只是这三个字，堆上字节一个都没动。
> 它既不是深拷贝，也不是两个指针指向同一块内存（那会 double free），
> 而是"把钥匙交出去，并且作废原来那把"。

---

## 2. move 语义：赋值也是转移

```rust
let a = String::from("hello");
let b = a;                   // 所有权从 a 移到 b
// println!("{a}");          // ❌ error[E0382]: borrow of moved value: `a`
println!("{b}");             // 输出: hello

let c = a.clone();           // ✅ 显式深拷贝（多一次堆分配），a 依然可用
println!("{a} {c}");         // 输出: hello hello
```

编译器在这里的判断等价于：`a` 拥有堆内存且**没有实现 `Copy`**，所以 `let b = a;` 不能按位复制
（否则两个所有者各 `drop` 一次，就是 double free）；于是所有权移交给 `b`，`a` 被标记为"已移动"，
此后任何对 `a` 的使用（读、写、再移动、借用）都是 `E0382`。

**"已移动"是编译期状态，不是运行时标志位**：生成的机器码里没有任何检查，只是三个字段被复制到
`b`，之后 `a` 不再被使用而已。`.clone()` 也不是"创可贴"，它是**写在代码里的成本声明**——
看到它就说明这里有分配。

---

## 3. Copy vs Clone：谁可以"复制后两边都能用"

```rust
let x: i32 = 5;
let y = x;                   // i32 实现了 Copy
println!("{x} {y}");         // 输出: 5 5 —— x 依然可用！
```

`i32` 这类值整块数据都在栈上、没有堆资源，按位复制不会造成双重释放，所以实现了 `Copy`，赋值/传参就是静默复制。

| | `Copy` | `Clone` |
|---|---|---|
| 触发方式 | **隐式**（赋值、传参） | **显式**（`.clone()`） |
| 语义 | 逐位复制，原值仍可用 | 你定义的"复制一份" |
| 成本 | 必须很便宜（栈上数据） | 可任意昂贵（深拷贝、重新分配） |
| 关系 | `Copy: Clone`（实现 `Copy` 前必须先 `Clone`） | 可以只 `Clone` 不 `Copy` |
| 典型类型 | `i32`/`f64`/`bool`/`char`/`&T`/`[i32; 3]` | `String`/`Vec<T>`/`Box<T>`/`HashMap<K, V>` |

```rust
#[derive(Clone, Copy, Debug)]
struct Point { x: i32, y: i32 }     // ✅ 字段全是 Copy，才能 Copy
#[derive(Clone, Debug)]
struct Named { name: String }       // ✅ 只能 Clone：String 不是 Copy
```

于是 `let q = p;` 对两个类型含义完全不同：对 `Point` 是静默复制（`p`、`q` 都能用），
对 `Named` 是移动（`p` 当场作废）。

两个容易忽略的点：`&T` 本身是 `Copy`（借来的东西不涉及所有权，所以能随便传）；
实现了 `Drop` 的类型**不能**实现 `Copy`——能自动复制又能自动释放必然 double free，编译器直接拒绝。

---

## 4. 函数传参即转移

```rust
fn shout(s: String) -> String { s.to_uppercase() }   // 返回新 String；旧的 s 在此被 drop
fn double(x: i32) -> i32 { x * 2 }

let text = String::from("hello");
let upper = shout(text);     // text 被移动进 shout
// println!("{text}");       // ❌ E0382：text 已经不归你了
println!("{upper}");         // 输出: HELLO

let n = 21;
let m = double(n);           // i32 是 Copy：传的是副本
println!("{n} {m}");         // 输出: 21 42 —— n 还能用
```

C++ 里"函数签名决定语义"（`std::string s` 拷贝 / `std::string&&` 移动 / `const std::string&` 借用），
Rust 写得更直白：

| 想表达的意思 | Rust 签名 | 调用后原值还能用吗 |
|---|---|---|
| 我要接管这个值 | `fn f(s: String)` | ❌ 不能（除非函数还回来） |
| 我只看看 | `fn f(s: &String)` / `fn f(s: &str)` | ✅ 能（第 4 章） |
| 我要改但不接管 | `fn f(s: &mut String)` | ✅ 能（第 4 章） |
| 我不管，给我一份副本 | 调用处 `f(s.clone())` | ✅ 能，但付拷贝成本 |

**所以"想继续用原值"的答案要么是改用借用，要么是显式 `clone`**，而不是像 C++ 那样指望
编译器隐含一次拷贝。第 4 章的借用，就是为了免掉绝大多数转移。

---

## 5. 返回值即转移

```rust
fn make_greeting() -> String {
    let s = String::from("hello");
    s                            // ⚠️ 没有分号：这是表达式的值，不是语句
}
let g = make_greeting();         // 所有权从函数内部交到 g 手里
println!("{g}");                 // 输出: hello
```

`s` 离开作用域时**不会**被销毁，因为所有权已经通过返回值移出去了：`g` 成为新所有者，
`g` 离开作用域时才 `drop`。对照 C 看这件事有多重要：

```c
char *bad(void) {
    char buf[8] = "hi";
    return buf;          /* ⚠️ 返回栈地址：悬垂指针，UB，往往只给个警告 */
}
```

Rust 里等价的写法是编译错误，而不是 UB：

```rust
fn bad() -> &'static String {
    let s = String::from("hi");
    &s                   // ❌ error[E0515]: cannot return reference to local variable `s`
}
```

区别在于：C 返回的是"指向已销毁栈帧的地址"；Rust 返回的是**堆内存的所有权**——
被销毁的只是栈上的头（ptr/len/cap），头里指针指向的堆内存还活着，把所有权一起交出去就安全了。
多返回值就是同一思路用元组表达（练习 3-3 正是如此）：

```rust
fn consume_and_give_back(s: String) -> (String, usize) {
    let len = s.len();           // 先算：usize 是 Copy，与 s 无关
    (s, len)                     // 再把 s 移动进元组还回去
}
```

> ⚠️ **坑**：写成 `(s, s.len())` 编译不过——元组从左到右求值，`s` 先被移动进元组，
> 紧接着对已移动的 `s` 求 `.len()` 就是 `E0382`。这类"顺序问题"是学习期最常见的失分点。

---

## 6. Drop 与作用域：释放是确定的

```rust
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let _a = Noisy("a");
    {
        let _b = Noisy("b");
        println!("内层作用域结束");
    }                            // ← _b 在这里被 drop
    println!("main 结束");
}                                // ← _a 在这里被 drop
// 输出（同一作用域内，局部变量按声明顺序的逆序释放）:
// 内层作用域结束 / drop b / main 结束 / drop a
```

要提前释放，就把它移动给 `std::mem::drop`——它是个普通函数，参数按值接收，函数体结束即析构：

```rust
let s = String::from("hi");
drop(s);                     // 等价于 std::mem::drop(s)：立刻释放
// println!("{s}");          // ❌ E0382：已经被移动进 drop
```

字符串、`Vec`、`Box`、文件句柄、锁守卫……所有"拥有资源"的类型都遵循这套规则。
（顺带一提：结构体的单个字段也能被单独移走，叫 partial move，编译器对每个字段分别记账。）

---

## 7. 为什么不需要 `free` / `delete`

**核心答案**：Rust 选择"没有 GC、也不靠手工释放"的第三条路——**把内存管理变成类型系统的一部分**。

1. 每个值有且只有一个所有者（`Box<T>`/`Rc<T>` 这类智能指针是受控例外，第 11 章讲）。
2. 所有者离开作用域，编译器就在**每一条退出路径**（正常返回、`?`、panic 展开、`break`）上插入 `drop`。
3. 于是释放时机是**静态确定的**：不用运行时扫描（没有 GC 停顿），不用程序员记住（没有泄漏），也不可能释放两次。

对照 C++ 的 RAII：思想一致，但**默认值反了过来**。

| | C++ | Rust |
|---|---|---|
| 释放时机 | 析构函数（RAII）——要自己写对 rule of five | 编译器插入 `drop`，你连写都不用写 |
| 赋值/传参 | 默认**拷贝**，要移动得写 `std::move` | 默认**移动**，要拷贝得写 `.clone()` |
| 移动后的对象 | "有效但未指定状态"，读它是逻辑错误，编译器不管 | 编译期禁止使用（`E0382`） |
| 悬垂引用 | UB，靠 `unique_ptr` + 纪律规避 | 借用检查器直接拒绝编译 |
| 释放两次 | 拷贝语义写错就 double free（UB） | 不可能：move 让旧绑定失效 |

一句话：**Rust 的 move 是默认行为，C++ 的 move 要你亲手写 `std::move`；
Rust 的释放不需要你写析构函数，C++ 的释放要靠你写对析构与拷贝语义。**

### 7.1 UB 与 use-after-free 为什么编译不过

```rust
// ① use-after-free：释放后使用
let s = String::from("hi");
drop(s);
// println!("{s}");          // ❌ E0382: use of moved value: `s`
// ② double free：两个所有者会各释放一次
let a = String::from("hi");
let b = a;                   // 所有权移动，a 失效
// drop(a);                  // ❌ E0382
drop(b);                     // ✅ 只有 b 真正释放
// ③ 悬垂引用：引用比所有者活得久
fn dangling() -> &'static String {
    let s = String::from("hi");
    &s                       // ❌ E0515: cannot return reference to local variable `s`
}
```

同样三段代码在 C 里都是**能编译、能跑、可能今天不炸明天炸**的 UB。Rust 把它们从
"运行时的不确定行为"搬成了"编译期的确定错误"——报错难看一点，换来线上不会出现这类安全问题。

> ⚠️ **坑**：`unsafe` 不是"关掉借用检查"的开关，它只打开五个额外超能力（解引用裸指针等，第 15 章）。
> 所有权规则在 `unsafe` 里**依然生效**，需要人工担保的是 `unsafe` 块内你自己承诺的不变量。

---

## 8. 常见坑

1. **移动后再用**（`E0382`），最常出现在循环里：`for w in words {}` 会消费 `words`，
   之后连 `words.len()` 都不能调；之后还想用就 `for w in &words`（借用），或者一开始就别移动。
2. **拿 `.clone()` 当万能药**：能过编译，但可能白付一次堆分配；先用借用（第 4 章）试。
3. **以为 `let b = a;` 是"两边共享"**：Rust 里它是移动，`a` 立即作废；
   Python 那种共享语义要靠 `Rc<T>`（第 11 章）。
4. **忘了 `len()` 是字节数**：`"中文".len() == 6`，`"中文".chars().count() == 2`；
   按字节下标切字符串还可能 panic（第 7 章）。
5. **返回局部变量的引用**（`E0515`）：要么返回值本身让所有权一起走，
   要么让调用方把数据传进来再借用（第 10 章生命周期）。
6. **`let _ = f(x);` 丢掉返回值**就是当场 `drop`；把值交给函数又不要回返回值，等于主动放弃它。

---

## 9. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 3-1 | 把字符串变成全大写 | 传参即转移；`to_uppercase()` 返回新 `String` | `shout` |
| 3-2 | 消费 `Vec<String>`，数个数并空格拼接 | `Vec` 的所有权、`into_iter()`、个数按元素算 | `join_and_count` |
| 3-3 | 原样还回字符串并报告字节长度 | 返回值即转移；先取 `len()` 的顺序问题 | `consume_and_give_back` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch03_ownership      # 卡住了跑 python3 tools/ai_review.py ch03_ownership --hint 2
```

### 提示（不给答案）

- **3-1**：`s.to_uppercase()` 返回一个**新** `String`（不需要 `mut s`），直接把它作为函数的值返回，
  注意行尾别加分号。
- **3-2**：先 `let count = words.len();`（`usize` 是 `Copy`，随手就能存下来），再 `words.join(" ")`。
  也可以 `for w in words.into_iter()` 自己 `push_str`，此时要记得"第一个元素前面不加空格"。
  `vec![String::new()]` 的答案是 `(1, "")`——个数按元素算，不按字符算。
- **3-3**：先 `let len = s.len();` 再返回 `(s, len)`；顺序反过来写成 `(s, s.len())` 会被编译器拒绝。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 3-1：参数按值接收意味着所有权已归函数；to_uppercase() 分配一个新 String 返回给调用方，
// 所以这里的 s 不会被"泄漏"，它只是正常走到生命终点。
pub fn shout(s: String) -> String {
    s.to_uppercase()
}

// 3-2：先把元素个数存成 usize（Copy，与 words 无关），再让 join 借用 words。
// words.join(" ") 需要 &[String]，编译器会自动把 &Vec<String> 转成 &[String]。
pub fn join_and_count(words: Vec<String>) -> (usize, String) {
    let count = words.len();
    let joined = words.join(" ");
    (count, joined)
}

// 3-3：关键是顺序——先取长度（usize 是 Copy），再把 s 移动进元组还回去。
// 写成 (s, s.len()) 就是先移动再求长度，E0382。
pub fn consume_and_give_back(s: String) -> (String, usize) {
    let len = s.len();
    (s, len)
}
```

**为什么 3-2 不写成 `(words.len(), words.join(" "))`？** 语义上等价（`words.len()` 只是借用，
求值结束借用就结束），但把长度先落到一个 `usize` 局部变量上，顺序明确、也避免以后表达式变长时
和借用检查器"斗智"。

</details>

---

## 自测清单

- [ ] 我能画出 `String` 在栈/堆上的三字段布局，并解释 move 时到底移动了什么
- [ ] 我能说出为什么 Rust 需要所有权：没有 GC，也不能靠手工 `free`/`delete`
- [ ] 我知道 `let b = a;`（`a: String`）和 `let y = x;`（`x: i32`）为什么表现不同
- [ ] 我能不查表说出 `Copy` 与 `Clone` 的三个区别（隐式/显式、成本、约束）
- [ ] 我知道 `fn f(s: String)` 和 `fn f(s: &str)` 对调用方的区别
- [ ] 我能解释"返回值即转移"，以及为什么返回局部变量的引用会 `E0515`
- [ ] 我知道 `"中文".len()` 是 6 而不是 2，以及该用什么数字符
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch03_ownership` 全绿
