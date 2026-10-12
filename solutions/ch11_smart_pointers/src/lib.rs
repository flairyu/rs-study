//! # 第 11 章 · 智能指针与内部可变性
//!
//! 本章用三个小练习摸清三件事：`Box<T>` 的堆分配与递归类型、
//! `Rc<T>` 的共享所有权、`RefCell<T>` 的运行时借用检查。
//!
//! 运行评测：
//! ```bash
//! cd /home/yxb/code/rs-study
//! python3 tools/check.py ch11_smart_pointers
//! ```
//!
//! | 练习 | 考点 |
//! |---|---|
//! | [`from_slice`] | 递归枚举必须 `Box`、按顺序构造链表 |
//! | [`to_vec`] | 沿 `&List` 遍历、`&Box<List>` 的解引用强制转换 |
//! | [`push_all`] | `Rc<RefCell<Vec<i32>>>`、`borrow_mut()` 守卫何时归还 |
//!
//! 提示：先读 `docs/01-syntax/ch11-smart-pointers.md`，再动手。
#![allow(unused_variables)] // 骨架期减少噪音；写完实现后可以删掉这一行

/// 单向链表：`Cons(值, 尾巴)` 或 `Nil`。
///
/// 尾巴必须是 `Box<List>`：枚举的大小要在编译期已知，
/// 直接写 `Cons(i32, List)` 会得到 `error[E0072]: recursive type has infinite size`。
#[derive(Debug, PartialEq)]
pub enum List {
    Cons(i32, Box<List>),
    Nil,
}

/// 练习 11-1：按切片里的顺序构造链表。
///
/// `&[1, 2, 3]` 必须得到
/// `Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))))`——
/// 第一个元素在表头。空切片返回 [`List::Nil`]。
///
/// # 示例
/// ```
/// # use ch11_smart_pointers::{from_slice, List};
/// assert_eq!(from_slice(&[]), List::Nil);
/// assert_eq!(from_slice(&[1]), List::Cons(1, Box::new(List::Nil)));
/// assert_eq!(
///     from_slice(&[1, 2]),
///     List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))))
/// );
/// ```
pub fn from_slice(items: &[i32]) -> List {
    items
        .iter()
        .rev()
        .fold(List::Nil, |acc, &x| List::Cons(x, Box::new(acc)))
}

/// 练习 11-2：把链表还原成 `Vec<i32>`（保持原有顺序）。
///
/// 空链表返回空 `Vec`。元素正负都要原样保留。
///
/// # 示例
/// ```
/// # use ch11_smart_pointers::{from_slice, to_vec};
/// assert_eq!(to_vec(&from_slice(&[1, 2, 3])), vec![1, 2, 3]);
/// ```
pub fn to_vec(list: &List) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur: &List = list;
    while let List::Cons(v, next) = cur {
        out.push(*v);
        cur = next; // &Box<List> → &List（解引用强制转换）
    }
    out
}

/// 练习 11-3：对每个 `handle` 执行一次 `borrow_mut().push(value)`，
/// 返回**最后一个** `handle` 的长度；`handles` 为空时返回 `0`。
///
/// 多个 handle 可以指向同一个 `Rc`（即同一个 `RefCell<Vec<i32>>`），
/// 此时它们共享同一个 `Vec`，push 会累加、`len()` 一起变化。
/// 注意不要跨语句持有 `borrow_mut()` 返回的守卫。
///
/// # 示例
/// ```
/// # use ch11_smart_pointers::push_all;
/// # use std::cell::RefCell;
/// # use std::rc::Rc;
/// let a = Rc::new(RefCell::new(Vec::new()));
/// let b = Rc::clone(&a);              // 与 a 共享同一个 Vec
/// assert_eq!(push_all(&[a, b], 7), 2);   // 被 push 了两次
/// ```
pub fn push_all(handles: &[std::rc::Rc<std::cell::RefCell<Vec<i32>>>], value: i32) -> usize {
    for handle in handles {
        // 守卫是临时值，在本语句结尾（分号处）就归还，
        // 因此同一个 RefCell 被循环借多次也不会 panic。
        handle.borrow_mut().push(value);
    }
    match handles.last() {
        Some(handle) => handle.borrow().len(),
        None => 0,
    }
}
