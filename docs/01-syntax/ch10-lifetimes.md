# 第 10 章 · 生命周期

> **一句话定位**：生命周期不是「让变量活得更久」的咒语，而是你在签名里向借用检查器**承诺的一条约束**——承诺错了，编译就不过。
> **预计时间**：100 分钟（读 45 + 写 55）
> **前置**：第 4 章（引用与借用）、第 9 章（泛型与 trait）
> **练习工程**：`exercises/ch10_lifetimes`（3 题）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

「引用指向的数据被提前释放」这个问题，三门语言给出了三种完全不同的答案：

| 话题 | C / C++ | Python | Rust |
|---|---|---|---|
| 悬垂引用（dangling） | 编译通过，跑到那行就是 UB | 不会发生：引用计数 + GC 保证对象还在 | **编译不过**：借用检查器用生命周期证明「活得够久」 |
| 谁来保证 | 程序员（纪律、注释、code review） | 运行时（GC） | 编译器（类型系统） |
| 出错时机 | 运行时，可能上线几年后才崩 | 基本不会，代价是 GC 停顿与内存开销 | 写完函数签名就报错 |
| 寿命信息写在哪 | 没有，最多一句 `// 返回的指针指向参数` | 没有 | 类型里：`&'a T`、`Parser<'a>` |
| 释放时机 | `free`/`delete` 或 RAII | GC 决定（不确定） | 所有者离开作用域，编译期确定 |
| 需要手写标注吗 | 不适用 | 不适用 | **大多数代码不用**；只有「引用进出函数边界」时才要 |

三条最小对照：

```c
// C：编译通过，运行 UB —— 寿命关系只存在于程序员的脑子里
char *dangle(void) {
    char buf[16] = "hi";
    return buf;              // 返回栈地址
}
```

```python
# Python：对象由引用计数养着，只要还有一个名字指向它就不会被回收
def make():
    return ["hi"]            # 永远不用担心「谁先死」
```

```rust
// Rust：同样的「返回局部变量地址」根本写不出来，编译期就拒绝
fn dangle() -> &str {
    let s = String::from("hi");
    &s                       // ❌ error[E0515]: cannot return reference to local variable `s`
}
```

一句话：Rust 把 C/C++ 里「靠人记住」的寿命关系变成了**类型里可检查的约束**，
又用省略规则让这份约束在大多数时候**不需要你写出来**。

---

## 1. 生命周期标注是「承诺」，不是「指令」

先纠正一个几乎所有人都有的第一印象：

> ❌ 以为：`fn longest<'a>(a: &'a str, b: &'a str) -> &'a str` 里的 `'a`，是在告诉编译器「让 `a`、`b` 多活一会儿」。
>
> ✅ 实际：`'a` 是一个**泛型参数**，代表「调用方保证的一段区域」。这个签名说的是：
> 对**任意**一段区域 `'a`，只要你传进来的两个引用都能活过 `'a`，我返回的引用也保证活过 `'a`。

`'a` 不会改变任何东西的寿命，它只是把输入和输出的寿命**绑定在一起**，
然后让借用检查器去验证每个调用点是否满足这个约束。

所以加标注救不了「数据本身已经没了」的情况：

```rust
// ❌ 写上 'a 也没用：s 在函数返回前就被 drop 了，没有任何 'a 救得了它
fn dangle<'a>() -> &'a str {
    let s = String::from("temp");
    &s
}
```

```text
error[E0515]: cannot return reference to local variable `s`
```

反过来，Rust 里也**没有**「把某个变量的寿命写长一点」的语法。
唯一能改变寿命的手段是改所有权结构——比如不返回 `&str`，改成返回 `String`。

### 全称量化：真正被检查的是调用点

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let s1 = String::from("long string");
    let out;
    {
        let s2 = String::from("xy");
        out = longest(s1.as_str(), s2.as_str());   // 'a 被取成 s2 的存活区域
    }
    println!("{out}");   // ❌ error[E0597]: `s2` does not live long enough
}
```

同一个函数，**换个调用点就换个 `'a`**。这正是「泛型」二字的含义：
`longest` 本身对 `'a` 多长一无所知，它只负责维持输入与输出之间的关系。

---

## 2. `'a` 语法：在哪儿声明，在哪儿使用

生命周期参数和类型参数一样，必须**先声明后使用**，写在尖括号里、类型参数前面：

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { a }   // 函数
struct Parser<'a> { input: &'a str, pos: usize }          // 结构体
impl<'a> Parser<'a> { }                                   // impl 块
enum Token<'a> { Word(&'a str), Eof }                     // 枚举
trait View<'a> { fn get(&self) -> &'a str; }              // trait

fn count_all<'a, T: std::fmt::Display>(items: &'a [T]) -> usize { items.len() }
//            ^^ 生命周期参数写在类型参数 T 前面
```

读法：`&'a str` = 「一个指向 `str` 的引用，它在这段 `'a` 区域内一定有效」。
`'a`、`'b`、`'x` 只是名字（惯例从 `'a` 开始），`'_` 表示「这里有个生命周期，但我不想给它起名」。

```rust
fn first_word(s: &str) -> &str { s.split_whitespace().next().unwrap_or("") }
fn first_word_explicit<'a>(s: &'a str) -> &'a str { s.split_whitespace().next().unwrap_or("") }
fn first_word_anon(s: &'_ str) -> &'_ str { s.split_whitespace().next().unwrap_or("") }
// 上面三种写法完全等价
```

三个必须记住的事实：

1. **生命周期只出现在签名里。** 函数体内部你从来不写 `'a`，局部变量的寿命由编译器推断（NLL）。
2. **没有运行时开销。** 生命周期在编译后就被擦除了，`&'a T` 和 `&T` 生成的机器码完全一样。
3. **`'a` 描述的是「至少多久」的关系，而不是一个具体时长。** 单看 `&'a str` 说不出 `'a` 有多长，
   它的长度永远由调用点决定。

---

## 3. 函数签名里的三条省略规则

你几乎没见过别人手写生命周期，是因为编译器有一套**生命周期省略规则（lifetime elision）**。
它只作用于签名，规则一共三条，按顺序应用：

> **规则 1**：输入位置上每个被省略的生命周期，各自获得一个**独立**的生命周期参数。
> **规则 2**：如果输入位置**恰好只有一个**生命周期参数（不管是你写的还是省略来的），
> 那么所有被省略的输出生命周期都等于它。
> **规则 3**：如果有多个输入生命周期参数，但其中一个是 `&self` / `&mut self`，
> 那么所有被省略的输出生命周期都等于 `self` 的生命周期。

### 规则 1：每个输入引用各自独立

```rust
// 你写的
fn announce(name: &str, city: &str) -> String {
    format!("{name} from {city}")
}

// 编译器补全的
fn announce<'a, 'b>(name: &'a str, city: &'b str) -> String
```

两个引用之间**没有任何关系**：`name` 比 `city` 早失效也没问题。
（这里也没有输出引用，所以规则 1 之后就没规则 2、3 什么事了。）

### 规则 2：只有一个输入生命周期 → 输出跟着它

```rust
// 你写的
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

// 编译器补全的
fn first_word<'a>(s: &'a str) -> &'a str
```

规则 2 数的是「**引用**参数」，不是「所有参数」：

```rust
fn nth_word(s: &str, n: usize) -> &str {
    s.split_whitespace().nth(n).unwrap_or("")
}
// → fn nth_word<'a>(s: &'a str, n: usize) -> &'a str
```

`n: usize` 不引入生命周期，所以规则 2 照样适用。第 4 章的 `longest_word(text: &str) -> &str`
就是被这条规则接住的——那不是「不需要生命周期」，而是「编译器替你写了」。

一旦有**两个**引用输入、又要把引用返回出去，规则 2 就失效，必须自己写：

```rust
// ❌ error[E0106]: missing lifetime specifier
//    help: this function's return type contains a borrowed value,
//          but the signature does not say whether it is borrowed from `a` or `b`
fn longest_bad(a: &str, b: &str) -> &str { a }

// ✅ 告诉编译器：返回值同时借用 a 和 b，所以两者都有效时它才有效
fn longest(a: &'a str, b: &'a str) -> &'a str { a }
```

### 规则 3：方法里的 `&self` 是默认锚点

```rust
struct Config { name: String }

impl Config {
    // 你写的
    fn name(&self) -> &str { &self.name }

    // 编译器补全的
    fn name<'s>(&'s self) -> &'s str
}
```

规则 3 就是「方法返回内部字段的引用」这么顺手的原因：返回值被绑到 `self` 上，
只要 `Config` 还活着，返回值就有效。

但规则 3 也解释了本章练习 10-2 为什么必须**显式**写 `-> &'a str`：
`Parser<'a>` 的数据来自 `input`，它的寿命是 `'a`，而不是 `&self` 的那一小段。

| 规则 | 什么时候生效 | 例子 |
|---|---|---|
| 1 | 总是（先跑） | `fn f(a: &str, b: &str)` → `<'a, 'b>` |
| 2 | 输入恰好 1 个生命周期 | `fn f(s: &str) -> &str` → `<'a>(s: &'a str) -> &'a str` |
| 3 | 输入 ≥2 个且含 `&self` | `fn f(&self, x: &str) -> &str` → 输出跟 `self` |

---

## 4. 结构体持有引用，必须标注生命周期

```rust
struct Parser<'a> {
    input: &'a str,      // 这个字段借用别人的数据，寿命记作 'a
    pos: usize,
}
```

漏掉 `<'a>` 会怎样？

```rust
struct Bad {
    input: &str,   // ❌ error[E0106]: missing lifetime specifier
}
```

因为结构体值完全可能活得比 `input` 指向的数据更久，编译器需要你把「我保证不会」写成类型约束：
`Parser<'a>` 的每个值都承诺「我只在这个 `'a` 区域内存在」。

标注具有**传染性**：一旦 `Parser` 带上了 `<'a>`，所有提到它的地方都要带参数——
`impl<'a> Parser<'a>`、`fn make<'a>(s: &'a str) -> Parser<'a>`、`Vec<Parser<'a>>`……

```rust
impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser { input, pos: 0 }
    }
}
```

也正因为这种传染性，Rust 的惯例是：**能用拥有的类型，就别用引用**。
`struct Config { name: String }` 永远不用为生命周期操心；
只有 View / Parser / Cursor / Scene 这类「本质上是别人数据的一个窗口」的类型才值得持有引用。
这是「大多数 Rust 代码里几乎看不到 `'a`」的第一个原因。

---

## 5. `&'a self` 与 `&self` 的区别

在 `impl<'a> Parser<'a>` 里，「借用 `self`」和「借用 `self` 里的数据」是两件不同的事：

```rust
impl<'a> Parser<'a> {
    // 写法 A：借用 self 一小会儿；返回值指向 input（'a）
    pub fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    // 写法 B：要求把 self 本身借用整整 'a 那么久
    pub fn rest_v2(&'a self) -> &'a str {
        &self.input[self.pos..]
    }
}
```

| 写法 | 含义 | 后果 |
|---|---|---|
| `&self`，返回 `&'a str` | `self.input` 是 `Copy` 的 `&'a str`，读出来即脱钩 | ✅ 推荐：借用关系与数据的真实寿命一致 |
| `&'a self`，返回 `&'a str` | 要求 `self: &'a Parser<'a>`，即 `Parser` 值本身被借用整整 `'a` | ⚠️ 调用点在 `Parser` 是局部变量时几乎都会报 `does not live long enough`，因为 `'a`（input 的寿命）通常比这个局部变量长 |

写法 A 为什么成立？因为 `self.input: &'a str` 是 `Copy` 的：
把它读出来之后，**返回值的寿命只和 `input` 指向的数据有关，和 `self` 这个结构体无关**。
换句话说，`Parser` 值可以立刻被 drop，只要 `input` 的数据还活着，`rest()` 的结果就依然有效：

```rust
fn main() {
    let text = String::from("hello world");
    let word;
    {
        let mut p = Parser::new(&text);
        word = p.rest();          // p 在块结束时销毁
    }
    println!("{word}");           // ✅ 仍然有效：word 借的是 text
}
```

> ⚠️ **坑**：如果写成省略形式 `fn rest(&self) -> &str`，规则 3 会把它补成 `-> &'s str`，
> 即「结果借用 `self`」。它能编译，但比实际需要**更严格**：
> 上面那段 `word = p.rest()` 就活不出块了。数据本来来自 `'a`，就应该显式写 `-> &'a str`。

---

## 6. `'static`：能活到程序结束，而不是「必须永远活着」

`'static` 是所有生命周期里最长的一个：**整个程序运行期间都有效**。

最常见的 `'static` 就是字符串字面量——它被编译进二进制，天然活到程序结束：

```rust
let s: &'static str = "hello";        // ✅ 字面量的类型就是 &'static str
static GREETING: &str = "hi";         // ✅ static 项里的引用默认就是 'static

let owned = String::from("hi");
// let bad: &'static str = &owned;    // ❌ `owned` does not live long enough
// let bad2: &'static str = &String::from("hi");  // ❌ temporary value dropped while borrowed
```

所以看到 `fn name() -> &'static str`，它只能返回字面量（或 `Box::leak` 之类主动泄漏的东西），
**不可能**返回局部变量的引用——这正是第 2 章里那些返回 `"钻石"`/`"黄金"` 的函数必须写
`&'static str` 的原因。

### 误解一：`'static` 不是「这个值必须永远活着」

`'static` 是**下界**（「至少活这么久」），不是「禁止提前销毁」：

```rust
fn main() {
    let s: String = String::from("temporary");   // String: 'static
    println!("{s}");
}   // s 在这里正常 drop —— 它没活到程序结束，但它的类型依然满足 'static
```

`String: 'static` 的意思是「这个类型内部**不含**短命的借用」，
所以它可以被存到任何地方、传给任何要求 `'static` 的 API。值本身当然可以提前销毁。

### 误解二：`T: 'static` 不等于「`T` 是 `&'static` 某个东西」

泛型约束里的 `T: 'static` 表示「`T` 里没有借来的短命数据」：

```rust
fn spawn_style<T: 'static>(value: T) { /* 假装要把它送去另一个线程 */ }

spawn_style(String::from("owned"));   // ✅ String: 'static
spawn_style(42);                      // ✅ i32: 'static
spawn_style("literal");               // ✅ &'static str: 'static

let local = 7;
// spawn_style(&local);               // ❌ `local` does not live long enough
```

`std::thread::spawn` 要求闭包是 `'static` 就是这个道理：新线程可能比创建它的函数活得久，
所以闭包不能借用栈上的局部变量（第 12 章会展开）。

---

## 7. 为什么大多数 Rust 代码里根本看不到生命周期

看完前面几节你可能觉得「这东西好麻烦」。但翻一翻真实项目会发现，显式 `'a` 很少出现。
原因有四个：

1. **省略规则覆盖了绝大多数函数**：单引用进出（规则 2）、`&self` 方法（规则 3）。
   只有「多个输入引用竞争谁是返回值的来源」时，才需要你表态。
2. **函数体里不写生命周期**：借用检查器在函数体内自行推断（NLL）。
   你只可能在**函数签名**、**结构体/枚举字段**、**impl 头**、**trait bound** 这四个地方见到 `'a`。
3. **结构体通常拥有数据**：用 `String` 而不是 `&str`、用 `Vec<T>` 而不是 `&[T]`。
   代价是可能多一次拷贝或分配，换来的是类型里没有生命周期参数——不传染。
4. **`'static` 的出现次数远多于 `'a`**，而且大多写在泛型约束里（`T: 'static`）。

一个实用判断法：**当你要返回一个引用，而它可能来自两个以上输入时**，
或者**当你要把引用存进结构体时**，才需要主动想生命周期。其余时间交给省略规则。

什么时候不该硬凹生命周期？需要长期保存的数据，直接拥有更省心：

```rust
// ❌ 费劲让结构体借用字符串，结果函数一返回就没人持有数据了
struct Report<'a> { title: &'a str }

// ✅ 拷贝一次（或移动一次）比到处传染 'a 更划算
struct Report { title: String }
```

---

## 8. 常见坑

### 坑 1：两个引用输入 + 返回引用 = 必须手写 `'a`

```rust
fn pick_bad(a: &str, b: &str, take_a: bool) -> &str {
    if take_a { a } else { b }
}
// error[E0106]: missing lifetime specifier
//   help: this function's return type contains a borrowed value,
//         but the signature does not say whether it is borrowed from `a` or `b`
```

编译器不知道返回值到底借用谁，只能拒绝。加上 `<'a>`，把两者绑到同一个生命周期即可。

### 坑 2：想返回函数内部新建的 `String` 的切片

```rust
fn make_greeting(name: &str) -> &str {
    let s = format!("hello {name}");
    &s                       // ❌ error[E0515]: cannot return reference to local variable `s`
}
```

`format!` 出来的 `String` 归函数所有，返回它的切片必然悬垂。
正确做法是把所有权一起交出去：`fn make_greeting(name: &str) -> String`。
**生命周期标注在这类错误面前无能为力**，这是新手最主要的误解。

### 坑 3：把「借用 `self`」当成「借用 `self` 的数据」

`fn rest(&self) -> &str` 会被规则 3 绑到 `&self` 上。如果结果其实来自 `self` 里的
`&'a str` 字段，调用方就会莫名其妙地「活不够久」。显式写 `-> &'a str` 才对。

### 坑 4：`&'static str` 不能从普通 `&str` 变出来

```rust
fn config_path(section: &str) -> &'static str {
    section          // ❌ lifetime may not live long enough
}
```

`'static` 比任何调用点传进来的生命周期都长，**不能把短的当长的用**。
反过来永远没问题：`&'static str` 可以当普通 `&str` 用（生命周期可以变短，不能变长）。

### 坑 5：结构体带上 `<'a>` 后到处写 `Parser<'a>` 太累

常见解法：让结构体拥有数据（`String`）；用索引、`Rc`/`Arc` 代替引用；
或者只在最外层解析入口用一次 `<'a>`，内部尽快把需要的数据拷贝出来。

### 坑 6：以为生命周期是运行时机制

`fn longest<'a>(a: &'a str, b: &'a str) -> &'a str` 编译后 `'a` 就消失了，
运行时没有任何检查、没有任何开销。生命周期是**纯编译期的证明**，
和 Python 的 GC、C++ `shared_ptr` 的引用计数不是一回事。

---

## 9. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 10-1 | 返回两个字符串里较长的一个 | 显式 `'a`、两个输入绑到同一生命周期、等长取第一个 | `longest` |
| 10-2 | 借用一个输入的极简分词器 | 结构体字段带 `'a`、`&self` 返回 `&'a str`、`char_indices` 定位边界 | `Parser` |
| 10-3 | 取第一个元素，空切片用 fallback | 返回值生命周期来自切片元素、`Option<&String>` → `&str` | `first_or` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch10_lifetimes
```

卡住了跑：

```bash
python3 tools/ai_review.py ch10_lifetimes --hint 2
```

### 提示（不给答案）

- **10-1**：签名已经写好了。函数体只有一个比较：`if a.len() >= b.len() { a } else { b }`。
  用 `>=` 而不是 `>`，才能满足「等长返回 `a`」。
- **10-2**：`next_word` 分两步——先在 `self.input[self.pos..]` 上用 `char_indices()`
  找到第一个非空白字符，算出它的**字节**下标 `start`；再从 `start` 开始找第一个空白字符，
  得到 `end`；更新 `self.pos = end`，返回 `Some(&self.input[start..end])`。
  全是空白就返回 `None`（`pos` 不动）；找不到结尾空白就取到 `input.len()`。
  ⚠️ 用 `char_indices()` 给出的字节偏移做切片，不要自己按固定字节数推进。
- **10-3**：`items.first().map(|s| s.as_str()).unwrap_or(fallback)` 一行搞定。
  `items` 是 `&'a [String]`，`first()` 给的是 `Option<&String>`，`as_str()` 之后
  引用的生命周期仍然是 `'a`。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
// 10-1：两个输入引用共享同一个 'a，返回值也被约束在 'a 内。
// 用 >= 而不是 >，是为了满足「等长返回 a」的约定。
pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

// 10-2：先跳过前导空白，再找单词末尾；全程用 char_indices() 的字节偏移，
// 保证切片落在 char 边界上（换成非 ASCII 输入也不会把字符切坏）。
pub struct Parser<'a> {
    pub input: &'a str,
    pub pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser { input, pos: 0 }
    }

    // 只借用 self 一小会儿，返回的却是 input 的数据（'a）。
    // 写成 -> &str 会被省略规则绑到 &self 上，比需要的更严格。
    pub fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    pub fn next_word(&mut self) -> Option<&'a str> {
        let input = self.input;              // 把 &'a str 拷出来，与 &mut self 脱钩

        // 1) 跳过前导空白；全是空白就到此为止
        let start = match input[self.pos..].char_indices().find(|(_, c)| !c.is_whitespace()) {
            Some((offset, _)) => self.pos + offset,
            None => return None,
        };

        // 2) 找下一个空白；找不到就一直到输入末尾
        let end = match input[start..].char_indices().find(|(_, c)| c.is_whitespace()) {
            Some((offset, _)) => start + offset,
            None => input.len(),
        };

        self.pos = end;
        Some(&input[start..end])
    }
}

// 10-3：first() 借的是 items，unwrap_or 借的是 fallback，
// 两者被签名统一到同一个 'a；只有空切片才会用到 fallback。
pub fn first_or<'a>(items: &'a [String], fallback: &'a str) -> &'a str {
    items.first().map(|s| s.as_str()).unwrap_or(fallback)
}
```

**为什么 10-1 不写 `<'a, 'b>` 再返回 `&'a str`？**
因为返回值只能有一个生命周期。如果分别给 `'a`、`'b`，返回 `a` 时它只保证活 `'a`、
返回 `b` 时只保证活 `'b`，编译器要的是一个**统一**的保证，所以两个输入必须共享同一个 `'a`
（调用点取两者的交集）。

**为什么 10-2 的 `rest` 用 `&self` 而不是 `&'a self`？**
`self.input` 是 `Copy` 的 `&'a str`，读出来之后返回值就和 `self` 无关了。
写成 `&'a self` 会要求 `Parser` 值本身被借用整整 `'a`，调用方寸步难行。

**10-2 为什么返回 `None` 时不能推进 `pos`？**
因为「没有下一个单词」和「已经消费到末尾」是两件事。若把 `pos` 推到输入末尾，
`rest()` 就会把剩下的空白也吃掉，调用方再也看不到原始剩余内容了。

</details>

---

## 自测清单

- [ ] 我能说出生命周期标注**不能**做什么（不能让变量活得更久）
- [ ] 我能背出三条省略规则，并各举一个例子
- [ ] 我知道 `fn f(a: &str, b: &str) -> &str` 为什么编译不过，该怎么修
- [ ] 我知道结构体字段是引用时为什么必须写 `<'a>`，以及它会「传染」到哪些地方
- [ ] 我能解释 `fn rest(&self) -> &'a str` 和 `fn rest(&'a self) -> &'a str` 的区别
- [ ] 我能说清 `'static` 表示「能活到程序结束」，以及 `String: 'static` 为什么成立
- [ ] 我能说出为什么 `fn f() -> &str { let s = String::new(); &s }` 加任何标注都没用
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch10_lifetimes` 全绿
