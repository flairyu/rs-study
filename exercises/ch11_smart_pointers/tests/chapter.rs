//! 第 11 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch11_smart_pointers`（或 `cargo test -p ch11_smart_pointers`）

use ch11_smart_pointers::*;
use std::cell::RefCell;
use std::rc::Rc;

/// 测试侧的小工具：手工搭一条链表，给 `to_vec` 准备输入。
/// 它只用来造数据，不依赖你写的 `from_slice`。
fn list(items: &[i32]) -> List {
    items
        .iter()
        .rev()
        .fold(List::Nil, |acc, &x| List::Cons(x, Box::new(acc)))
}

#[test]
fn ex11_1_from_slice() {
    assert_eq!(from_slice(&[]), List::Nil, "空切片应该构造出 Nil");

    assert_eq!(
        from_slice(&[42]),
        List::Cons(42, Box::new(List::Nil)),
        "单元素切片应该是一个 Cons + Nil"
    );

    assert_eq!(
        from_slice(&[1, 2, 3]),
        List::Cons(
            1,
            Box::new(List::Cons(2, Box::new(List::Cons(3, Box::new(List::Nil)))))
        ),
        "顺序必须与切片一致：第一个元素在表头"
    );

    assert_eq!(
        from_slice(&[-1, 0, -5]),
        List::Cons(
            -1,
            Box::new(List::Cons(0, Box::new(List::Cons(-5, Box::new(List::Nil)))))
        ),
        "负数和 0 都要原样保留"
    );

    assert_eq!(from_slice(&[7, 7]), list(&[7, 7]), "重复元素不能被悄悄去重");
}

#[test]
fn ex11_2_to_vec() {
    assert_eq!(
        to_vec(&List::Nil),
        Vec::<i32>::new(),
        "空链表应该还原成空 Vec"
    );

    assert_eq!(to_vec(&list(&[5])), vec![5], "单元素链表");

    assert_eq!(to_vec(&list(&[1, 2, 3])), vec![1, 2, 3], "顺序不能被反过来");

    assert_eq!(to_vec(&list(&[-1, 0, -5])), vec![-1, 0, -5], "负数原样保留");

    assert_eq!(to_vec(&list(&[3, 3, 1])), vec![3, 3, 1], "重复元素保留");
}

#[test]
fn ex11_3_push_all() {
    // 边界：空 handles 切片，不 panic 且返回 0
    assert_eq!(push_all(&[], 1), 0, "空 handles 应返回 0");

    // 单个 handle：push 后返回它的长度
    let single = Rc::new(RefCell::new(vec![1, 2]));
    assert_eq!(
        push_all(&[Rc::clone(&single)], 3),
        3,
        "应返回最后一个 handle 的长度"
    );
    assert_eq!(*single.borrow(), vec![1, 2, 3], "value 应该被 push 进去");

    // 多个 handle，各自指向不同的 Vec：结果只看最后一个
    let a: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
    let b: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![9]));
    assert_eq!(
        push_all(&[Rc::clone(&a), Rc::clone(&b)], 5),
        2,
        "只统计最后一个 handle"
    );
    assert_eq!(*a.borrow(), vec![5], "前面的 handle 也要被 push");
    assert_eq!(*b.borrow(), vec![9, 5], "最后一个 handle 收到 value");

    // 同一块数据、两个 handle（对同一个 Rc 做两次 Rc::clone）
    let shared: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![0]));
    let h1 = Rc::clone(&shared);
    let h2 = Rc::clone(&shared);
    assert_eq!(
        Rc::strong_count(&shared),
        3,
        "shared + h1 + h2，共 3 个所有者"
    );
    assert_eq!(
        push_all(&[h1, h2], 7),
        3,
        "同一个 Vec 被 push 了两次：1 + 2 = 3"
    );
    assert_eq!(
        *shared.borrow(),
        vec![0, 7, 7],
        "两个 handle 改的是同一块数据"
    );
    assert_eq!(
        Rc::strong_count(&shared),
        1,
        "h1 与 h2 已随临时数组一起释放"
    );
}
