# 第 11 章 · 智能指针与内部可变性

> **一句话定位**：`Box` / `Rc` / `RefCell` 是所有权规则的三个「受控例外」——堆分配、共享所有权、运行时借用检查，各占一个合法出口。
> **预计时间**：110 分钟（读 55 + 写 55）｜**前置**：第 3 章（所有权）、第 4 章（借用）、第 5 章（枚举）
> **练习工程**：`exercises/ch11_smart_pointers`（3 题，只依赖 std）

---

## 0. 先建立锚点（给 C/C++/Python 使用者）

### 0.1 一张对照表

| 你想做的事 | C | C++ | Python | Rust 的解法 |
|---|---|---|---|---|
| 在堆上放一个值 | `malloc` / `free` | `new` / `delete` | 对象天然在堆上 | `Box<T>`：唯一所有者，离开作用域自动释放 |
| 自己管释放、防漏防重 | 全靠纪律 | `std::unique_ptr<T>` | 不用管（GC） | 所有权规则 + `Drop`（第 3 章） |
| 多个名字共享同一个对象 | 共享指针 + 约定谁 free | `std::shared_ptr<T>` | 直接赋值、引用计数 | `Rc<T>`：引用计数，计数归零才释放 |
| 观察但不拥有、防循环引用 | — | `std::weak_ptr<T>` | `weakref.ref` | `Weak<T>`：不增强计数，`upgrade()` 可能给 `None` |
| 在共享结构里改内容 | 随你改 | `mutable` 成员 | 随你改 | `RefCell<T>`：借用检查推迟到运行期 |
| 共享 + 可变 | — | `shared_ptr` + 人工约定 | 随你改 | `Rc<RefCell<T>>` 组合（第 6 节） |
| 小 `Copy` 值的内部可变 | 随你改 | `mutable` | 随你改 | `Cell<T>`：按值进出，不交出内部引用 |

一句话：**`Box` 管「放哪」，`Rc` 管「谁拥有」，`RefCell` 管「能不能改」。**
C++ 的 `unique_ptr`/`shared_ptr` 主要在回答「谁来 `delete`」；Rust 的 `Box`/`Rc` 回答完同一个问题后
还要过借用检查器，于是才有了 `RefCell` 这个「把编译期检查换成运行期检查」的逃生舱。

### 0.2 同一个计数器：Python vs Rust

```python
counter = [0]          # 一切皆引用，"改"是改对象
alias = counter
alias[0] += 1
print(counter[0])      # 1 —— 何时销毁由引用计数 + GC 决定
```

```rust
use std::cell::RefCell;
use std::rc::Rc;

let counter = Rc::new(RefCell::new(0));
let alias = Rc::clone(&counter);     // 引用计数 2，共享同一块堆数据
*alias.borrow_mut() += 1;            // 运行时借用检查放行
println!("{}", counter.borrow());    // 输出: 1
```

Python 靠 GC 兜底；Rust 把这件事**写进类型**：`Rc` 说明「这是共享的」，
`RefCell` 说明「这里可能被改，运行时会查」。

---

## 1. `Box<T>`：堆分配与递归类型

```rust
let b = Box::new(5);      // 5 被放到堆上，b 是栈上唯一的所有者
println!("{}", *b);       // 输出: 5 —— 解引用拿回值
println!("{}", b);        // 输出: 5 —— Display 通过 Deref 自动生效
let big = Box::new([0u8; 1024 * 1024]);   // 1 MiB 数组搬去堆，栈上只留一个指针
```

`Box<T>` 是最朴素的智能指针：**独占所有权 + 堆分配 + 和普通值一样的移动/Drop 语义**。
绝大多数代码用不到它，只有三种场合需要：递归类型、值太大不想在栈上搬、
以及第 9 章的 trait 对象 `Box<dyn Trait>`。

### 1.1 递归类型为什么非 `Box` 不可

```rust
enum List {
    Cons(i32, List),   // ❌ error[E0072]: recursive type `List` has infinite size
    Nil,
}
```

编译器要算出 `size_of::<List>()` 才敢分配栈空间，而 `Cons` 里直接塞一个 `List` 就是
「大小 = 4 + 大小」，永远算不完。换成 `Box<List>`（一个指针，大小固定）就合法了：

```rust
#[derive(Debug, PartialEq)]
enum List {
    Cons(i32, Box<List>),   // ✅ 64 位机器上 8 字节
    Nil,
}
// 堆: [1 | ptr] ──► [2 | ptr] ──► Nil      栈: 一个 (i32 + 指针) 大小的值
```

**递归 = 必须有间接层**：C 里你写 `struct Node *next`，Rust 里就是 `Box<Node>`。

---

## 2. `Deref` / `DerefMut` 与自动解引用

智能指针之所以「像普通值」，全靠 `Deref`：

```rust
use std::ops::Deref;

struct MyBox<T>(T);

impl<T> MyBox<T> {
    fn new(x: T) -> Self { MyBox(x) }
}

impl<T> Deref for MyBox<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }   // 只交出 &T
}

let x = MyBox::new(String::from("hi"));
println!("{}", x.len());   // 输出: 2 —— 自动解引用: &MyBox<String> → &String → &str
```

规则是：**`*x` 展开成 `*(x.deref())`**；方法调用和字段访问处会自动插入一串 `deref()`，
直到找到匹配的方法（deref coercion / 自动解引用链）。`Box<T>`、`Rc<T>`、`String`、`Vec<T>`
都是这么变得好用的。`DerefMut` 是可变版，让 `*x = 值`、`x.push(...)` 这类 `&mut` 操作也能透传。

> ⚠️ **坑**：不要为了「少写几个 `.0`」就给自定义类型实现 `Deref`。它表达的是
> 「我就是一个 `T`」，滥用会让方法解析难以推理（newtype 请写显式方法或实现 `AsRef`）。

---

## 3. `Drop` 与析构顺序

```rust
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let _a = Noisy("a");
    { let _b = Noisy("b"); }     // 内层结束
    println!("main 还在跑");
}                                // 外层结束
// 输出: drop b / main 还在跑 / drop a
```

三条要记住的规则：

1. **同一作用域内，局部变量按声明顺序的逆序析构**；结构体字段按声明顺序析构。
2. `Drop` 不能手动调用（`x.drop()` 编译不过），要提前释放就 `drop(x)`（即 `std::mem::drop`）。
3. 实现了 `Drop` 的类型**不能**再实现 `Copy`（能自动复制又能自动释放必然 double free）。

`Box`/`Rc`/`RefCell` 的释放都建立在「`Drop` 递归触发」上：`Box` drop 时顺手 drop 堆上的 `T`，
于是链表、树能整条链自动清干净——你不用写一行释放代码。

---

## 4. `Rc<T>`：共享所有权与强/弱引用

```rust
use std::rc::Rc;

let a = Rc::new(vec![1, 2, 3]);
println!("{}", Rc::strong_count(&a));   // 输出: 1
let b = Rc::clone(&a);                  // 计数 +1，不是深拷贝
println!("{}", Rc::strong_count(&a));   // 输出: 2
drop(b);
println!("{}", Rc::strong_count(&a));   // 输出: 1
```

- **`Rc::clone(&a)` 是 O(1) 的计数加一**，`(*a).clone()` 才是深拷贝整个 `Vec`。
  代码里看到 `Rc::clone` 就可以放心，它不贵。
- `Rc<T>` 只给**共享读**：它 `Deref` 到 `T`，但拿不到 `&mut T`。想改 → 加 `RefCell`；
  想跨线程 → 换 `Arc`（第 12 章）。
- `Rc` 不是 `Send`/`Sync`：计数只是普通整数，两个线程同时 `clone` 会算错，编译器直接禁止。
- 常用 API：`Rc::strong_count` / `Rc::weak_count` / `Rc::ptr_eq`（比地址而不是比值）/
  `Rc::get_mut`（唯一所有者时能拿到 `&mut T`）/ `Rc::downgrade`。

### 4.1 循环引用与 `Weak<T>`

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: &'static str,
    parent: RefCell<Weak<Node>>,       // 指向父：弱引用，不拥有
    children: RefCell<Vec<Rc<Node>>>,  // 指向子：强引用，拥有
}

let parent = Rc::new(Node {
    name: "parent",
    parent: RefCell::new(Weak::new()),
    children: RefCell::new(vec![]),
});
let child = Rc::new(Node {
    name: "child",
    parent: RefCell::new(Rc::downgrade(&parent)),   // 不增加强计数
    children: RefCell::new(vec![]),
});
parent.children.borrow_mut().push(Rc::clone(&child));

println!("{} {}", Rc::strong_count(&parent), Rc::strong_count(&child));  // 1 2
assert_eq!(child.parent.borrow().upgrade().unwrap().name, "parent");
```

`Rc` 的循环引用是**安全代码也能造成的内存泄漏**：A 强引用 B、B 强引用 A，两边计数永远 ≥ 1。
解决办法是让「回边」用 `Weak<T>`：

- `Rc::downgrade(&rc)` → `Weak<T>`：强计数不变，弱计数 +1；
- `weak.upgrade()` → `Option<Rc<T>>`：目标还活着是 `Some`，已被回收是 `None`；
- 树/图/双向链表/观察者列表的标准配方：**父 → 子用 `Rc`，子 → 父用 `Weak`。**

> ⚠️ **坑**：循环引用的泄漏不会 `panic`，也不会被借用检查器发现，只会让内存慢慢涨。
> 用 `Rc` 建图前，先想清楚哪条边是「拥有」，哪条边只是「回看」。

---

## 5. `RefCell<T>`：运行时借用检查

```rust
use std::cell::RefCell;

let cell = RefCell::new(vec![1, 2, 3]);
cell.borrow_mut().push(4);                // 可变借用：RefMut
println!("{:?}", *cell.borrow());         // 共享借用：Ref → [1, 2, 3, 4]

let r1 = cell.borrow();
let r2 = cell.borrow();                   // ✅ 多个共享借用可以共存
// let m = cell.borrow_mut();             // ❌ panic: already borrowed: BorrowMutError
```

- `borrow()` → `Ref<T>`，`borrow_mut()` → `RefMut<T>`；两者都是 **RAII 守卫**，
  **离开作用域自动归还**——和 `&T`/`&mut T` 一样，只是检查发生在运行时。
- 规则也一样：多个共享借用可以共存，可变借用必须独占，冲突时 `panic!`。
  不想 panic 就用 `try_borrow()` / `try_borrow_mut()`，它们返回 `Result`。
- `RefCell` 不是 `Sync`（计数不是原子的），只能单线程用。

### 5.1 为什么 `RefCell` 能把借用检查推迟到运行时？代价是什么

编译期的借用检查依赖**静态可见的别名关系**：`&T`/`&mut T` 是显式的语法标记，编译器能追踪。
但有些结构的关系只有运行时才知道（图的拓扑、观察者列表、缓存里的父子指针），编译器只能保守
拒绝——`RefCell` 就是给这种情况留的口子。它的实现很土但很有效：内部维护一个「借用标志」，
`borrow()` 时 +1，`borrow_mut()` 要求标志为 0 并标记为独占，`Ref`/`RefMut` 被 drop 时还回去。
**同一套规则，从「编译器证明」变成「运行时记账」。**

代价有三个，必须清楚：

1. **失败从编译错误降级成 panic**——原本编译期就拦下的 bug，可能等到线上才炸；
2. 每次借用都要读写一次计数、多一层守卫对象（开销很小，但不是零）；
3. 类型不再 `Sync`，也不能直接跨线程共享。

**所以：能用编译期借用表达的，就别用 `RefCell`。** 它适合图、观察者、缓存这类
「编译器看不透」的场景，而不是用来「绕过借用检查」的开关。

### 5.2 `Cell<T>`：给 `Copy` 值的内部可变性

```rust
use std::cell::Cell;

let c = Cell::new(1);
c.set(2);                    // 不需要 &mut，也不需要 borrow()
println!("{}", c.get());     // 输出: 2

// get 要求 T: Copy；非 Copy 用 replace / take（按值进出）
let s = Cell::new(String::from("hi"));
let old = s.replace(String::from("bye"));   // 返回旧值
let taken = s.take();                       // 换回 Default，返回旧值
assert_eq!((old, taken), ("hi".to_string(), "bye".to_string()));
```

| | `Cell<T>` | `RefCell<T>` |
|---|---|---|
| 取值方式 | 按值进出（`get` 要求 `T: Copy`，或用 `replace`/`take`） | `borrow()` / `borrow_mut()` 守卫 |
| 会失败吗 | 不会 | 借用冲突时 panic |
| 开销 | 几乎为零 | 一次借用计数 |
| 能拿到内部引用吗 | ❌ 绝不 | ✅ `&T` / `&mut T` |

`Cell` 的哲学是「没有引用就没有别名问题」：它只让你把值换进换出，所以可以随便改。
计数器、标志位这类小 `Copy` 状态用 `Cell`，需要读内部引用（比如 `Vec`）才用 `RefCell`。

---

## 6. `Rc<RefCell<T>>`：共享 + 可变的组合拳

```rust
use std::cell::RefCell;
use std::rc::Rc;

let shared = Rc::new(RefCell::new(vec![1]));
let a = Rc::clone(&shared);
let b = Rc::clone(&shared);
a.borrow_mut().push(2);
b.borrow_mut().push(3);
println!("{:?}", *shared.borrow());   // 输出: [1, 2, 3]
```

这就是 Python/Java 里「多个对象持有同一个可变列表」的 Rust 版本：`Rc` 负责「谁拥有」
（共享、计数），`RefCell` 负责「能不能改」（运行时借用）。练习 11-3 就是这个模式的切片版。

> ⚠️ **坑**：用 `Rc<RefCell<T>>` 的地方往往意味着数据结构变复杂了。先问一句
> 「能不能改成一个人拥有、其他人传 `&mut`？」——能用编译期检查就别交给运行时。

---

## 7. 常见坑

1. **`Rc::clone(&x)` 写成 `(*x).clone()`**：前者计数 +1（O(1)），后者深拷贝整个值。
2. **跨语句持有 `borrow_mut()` 的守卫**：
   ```rust
   let mut g = cell.borrow_mut();     // 守卫活到作用域结束
   g.push(1);
   // let n = cell.borrow().len();    // ❌ panic: already mutably borrowed
   drop(g);                           // 还回去之后才行
   let n = cell.borrow().len();       // ✅
   ```
   借用区间由**守卫变量的作用域**决定，而不是 `borrow_mut()` 写在哪儿。写成一条语句
   （`cell.borrow_mut().push(1);`）时，守卫在分号处就归还了。
3. **`Rc` 循环引用**：A→B→A 全是强引用 = 内存泄漏，回边改用 `Weak` + `upgrade()`。
4. **把 `Rc`/`RefCell` 送进线程**：`Rc` 不是 `Send`/`Sync`、`RefCell` 不是 `Sync`，编译不过；
   该场景用 `Arc<Mutex<T>>`（第 12 章）。
5. **以为 `Box<T>` 移动时会拷贝堆数据**：移动的只是那个指针，堆上的 `T` 一动没动。
6. **拿 `RefCell` 当「万能钥匙」**：编译过了，但 panic 只是从编译期挪到了运行期。

---

## 8. 本章练习

| 题号 | 任务 | 考点 | 对应函数 |
|---|---|---|---|
| 11-1 | 按切片顺序搭一条链表 | 递归枚举必须 `Box`、`iter().rev().fold()` | `from_slice` |
| 11-2 | 把链表还原成 `Vec<i32>` | `&List` 遍历、`&Box<List>` 的解引用强制转换 | `to_vec` |
| 11-3 | 给每个 handle push，返回最后一个的长度 | `Rc<RefCell<Vec<i32>>>`、守卫何时归还 | `push_all` |

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch11_smart_pointers      # 卡住：python3 tools/ai_review.py ch11_smart_pointers --hint 2
```

### 提示（不给答案）

- **11-1**：第一个元素必须在表头，而 `fold` 是从左往右走的——要么先 `rev()` 再 fold，
  要么换个方向把新元素顶到前面。起点是 `List::Nil`，每步用 `Box::new(acc)` 把搭好的尾巴装箱。
- **11-2**：用 `&List` 游标循环 `match`：`List::Cons(v, next)` 里 `v` 是 `&i32`（`push(*v)`），
  `next` 是 `&Box<List>`；下一轮直接把它交给游标——`&Box<List>` 到 `&List` 的解引用强制转换
  会自动发生。`List::Nil` 就是循环结束条件。
- **11-3**：一次借用一条语句：`for h in handles { h.borrow_mut().push(value); }`。
  返回值用 `handles.last().map(|h| h.borrow().len()).unwrap_or(0)`——
  `borrow()` 也在同一个表达式里用完，不会跨语句持有。

---

## 参考答案（先自己写，再看）

<details>
<summary>点击展开 3 题参考答案</summary>

```rust
#[derive(Debug, PartialEq)]
pub enum List {
    Cons(i32, Box<List>),
    Nil,
}

// 11-1：先 rev()，再从 Nil 开始从左往右折叠，每步把元素顶到表头，折完正好是原顺序。
// 也可以不 rev，改成从右往左折叠 + 头插，但那要额外处理方向。
pub fn from_slice(items: &[i32]) -> List {
    items.iter().rev().fold(List::Nil, |acc, &x| List::Cons(x, Box::new(acc)))
}

// 11-2：用 &List 游标边走边收集。next 是 &Box<List>，赋给 &List 游标时
// 解引用强制转换会自动生效（等价于 cur = &**next）。
pub fn to_vec(list: &List) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur: &List = list;
    while let List::Cons(v, next) = cur {
        out.push(*v);
        cur = next;               // &Box<List> → &List
    }
    out
}

// 11-3：每个 handle 借用一次、push 一次，守卫在分号处就归还，
// 所以同一个 RefCell 被循环借多次也不会 panic；多个 handle 指向同一个 Rc 时 push 会叠加。
pub fn push_all(handles: &[std::rc::Rc<std::cell::RefCell<Vec<i32>>>], value: i32) -> usize {
    for h in handles {
        h.borrow_mut().push(value);
    }
    handles.last().map(|h| h.borrow().len()).unwrap_or(0)
}
```

**为什么 `from_slice` 要 `rev()`？** `fold` 的累积值是「已经搭好的部分」；从左往右折叠时新元素
只能接到尾部，而链表的尾部埋在箱子深处（要 O(n) 走到）。先 `rev()` 让新元素永远接在表头，
整体就是 O(n)。链表题的通用直觉：**头插便宜，尾插贵。**

**为什么 `to_vec` 用循环而不是递归？** 递归版在长链表上会爆栈，而且每层都要自己维护 `out`；
循环版是尾递归的等价写法，更稳。

</details>

---

## 自测清单

- [ ] 我能说出 `Box<T>` 的三个使用场景，以及递归类型为什么必须 `Box`
- [ ] 我能解释 `*x` 在 `Box<T>`/自定义指针上是怎么展开成 `deref()` 的
- [ ] 我知道 `Rc::clone` 和 `(*rc).clone()` 的区别，以及 `Rc` 为什么不是 `Send`
- [ ] 我能画出循环引用导致泄漏的过程，并用 `Weak` + `upgrade()` 修好它
- [ ] 我能说出 `RefCell` 的借用规则、冲突时的行为，以及 `try_borrow` 的区别
- [ ] 我能举出 `RefCell` 把借用检查推迟到运行时的三个代价
- [ ] 我完成了 3 道练习，`python3 tools/check.py ch11_smart_pointers` 全绿
