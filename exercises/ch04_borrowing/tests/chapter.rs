//! 第 4 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch04_borrowing`（或 `cargo test -p ch04_borrowing`）

use ch04_borrowing::*;

#[test]
fn ex4_1_longest_word() {
    // 正常值
    assert_eq!(
        longest_word("hello rust world"),
        "hello",
        "hello 与 world 等长，应取靠前的 hello"
    );
    assert_eq!(
        longest_word("the quick brown fox"),
        "quick",
        "quick 与 brown 都是 5 字节，取靠前的 quick"
    );
    assert_eq!(
        longest_word("i love programming"),
        "programming",
        "最长单词在末尾也要能取到"
    );

    // 边界值：空与全空白
    assert_eq!(longest_word(""), "", "空输入应返回空切片");
    assert_eq!(longest_word("   \t\n  "), "", "全空白应返回空切片");
    assert_eq!(
        longest_word(" \t\n rust \r\n "),
        "rust",
        "前后都是空白时应只返回中间的单词"
    );

    // 边界值：单单词 / 连续空白 / 标点也算单词的一部分
    assert_eq!(longest_word("solo"), "solo", "只有一个单词就是它自己");
    assert_eq!(
        longest_word("a  bbb   cc"),
        "bbb",
        "连续空白不会把单词拼在一起"
    );
    assert_eq!(
        longest_word("hi!! there"),
        "there",
        "标点不切分单词：there(5 字节) 比 hi!!(4 字节) 长"
    );
}

#[test]
fn ex4_2_append_exclaim() {
    // 正常值
    let mut s = String::from("hello");
    append_exclaim(&mut s);
    assert_eq!(s, "hello!", "调用后应追加一个感叹号");

    // 连续调用：每次追加一个
    let mut twice = String::from("wow");
    append_exclaim(&mut twice);
    append_exclaim(&mut twice);
    assert_eq!(twice, "wow!!", "连续调用应每次追加一个 !");

    // 边界值：空字符串、已是感叹号结尾、含多字节 UTF-8 字符
    let mut empty = String::new();
    append_exclaim(&mut empty);
    assert_eq!(empty, "!", "空字符串应变成单个 !");

    let mut bang = String::from("!");
    append_exclaim(&mut bang);
    assert_eq!(bang, "!!", "已有 ! 结尾时继续追加");

    let mut zh = String::from("你好");
    append_exclaim(&mut zh);
    assert_eq!(zh, "你好!", "多字节字符按字符边界追加，不会切坏 UTF-8");
}

#[test]
fn ex4_3_halves() {
    // 正常值：偶数长度与奇数长度
    assert_eq!(
        halves(&[1, 2, 3, 4]),
        (&[1, 2][..], &[3, 4][..]),
        "偶数长度应正中对半切"
    );
    assert_eq!(
        halves(&[1, 2, 3]),
        (&[1][..], &[2, 3][..]),
        "奇数长度向下取整，前段较短"
    );
    assert_eq!(
        halves(&[5, 6, 7, 8, 9]),
        (&[5, 6][..], &[7, 8, 9][..]),
        "切点为 len / 2 = 2"
    );

    // 边界值：空切片与单元素切片
    let (left, right) = halves(&[]);
    assert!(left.is_empty(), "空切片的前段应为空");
    assert!(right.is_empty(), "空切片的后段应为空");

    let (left, right) = halves(&[7]);
    assert!(left.is_empty(), "单元素切片的前段应为空");
    assert_eq!(right, &[7][..], "单元素切片的元素应落在后段");

    // 边界值：负数与非零起点（结果必须是原切片上的视图，而不是新数组）
    assert_eq!(
        halves(&[-3, -2, -1, 0]),
        (&[-3, -2][..], &[-1, 0][..]),
        "负数元素同样按位置切分"
    );
    let data = [10, 20, 30, 40];
    let (left, right) = halves(&data);
    assert_eq!(left, &[10, 20][..], "整段中间切开后前段是 [10, 20]");
    assert_eq!(right, &[30, 40][..], "整段中间切开后后段是 [30, 40]");
    assert_eq!(
        left.as_ptr(),
        data.as_ptr(),
        "前段应指向原切片的起点（零拷贝视图）"
    );
}
