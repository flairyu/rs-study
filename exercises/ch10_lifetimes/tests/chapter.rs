//! 第 10 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch10_lifetimes`（或 `cargo test -p ch10_lifetimes`）

use ch10_lifetimes::*;

#[test]
fn ex10_1_longest() {
    // 正常值：长度不同
    assert_eq!(longest("hello", "hi"), "hello", "hello 更长，应返回 a");
    assert_eq!(longest("ab", "cde"), "cde", "cde 更长，应返回 b");
    assert_eq!(
        longest("rust", "rustlang"),
        "rustlang",
        "较长者在第二个参数上也要取到"
    );

    // 边界值：等长必须返回第一个参数 a
    assert_eq!(longest("abc", "xyz"), "abc", "等长时按约定返回 a");
    assert_eq!(longest("", ""), "", "两个空串等长，返回 a（还是空串）");

    // 边界值：空串参与比较
    assert_eq!(longest("", "rust"), "rust", "a 为空时返回 b");
    assert_eq!(longest("rust", ""), "rust", "b 为空时返回 a");
    assert_eq!(longest("x", ""), "x", "另一个空串分支");

    // 返回值是原字符串上的切片，不是新分配的 String
    let s1 = String::from("alpha");
    let s2 = String::from("beta");
    let out = longest(&s1, &s2);
    assert_eq!(out, "alpha");
    assert_eq!(
        out.as_ptr(),
        s1.as_ptr(),
        "返回的切片应直接借用第一个输入（零拷贝）"
    );
}

#[test]
fn ex10_2_parser() {
    // 边界值：空输入
    let mut empty = Parser::new("");
    assert_eq!(empty.rest(), "", "空输入的 rest 是空串");
    assert_eq!(empty.next_word(), None, "空输入没有单词");
    assert_eq!(empty.pos, 0, "没有单词时 pos 不应推进");

    // 边界值：全空白输入 —— next_word 返回 None，pos 不动
    let mut blank = Parser::new("   \t\n ");
    assert_eq!(blank.next_word(), None, "全空白输入没有单词");
    assert_eq!(blank.pos, 0, "全空白时 pos 保持 0");
    assert_eq!(blank.rest(), "   \t\n ", "全空白时 rest 原样返回整段输入");

    // 正常值 + 连续多个空格 + 首尾空白
    let mut spaced = Parser::new("  hello   world  ");
    assert_eq!(spaced.pos, 0, "new 之后 pos 从 0 开始");
    assert_eq!(
        spaced.rest(),
        "  hello   world  ",
        "未消费时 rest 等于整个输入"
    );
    assert_eq!(spaced.next_word(), Some("hello"), "先跳过前导空白再取单词");
    assert_eq!(spaced.pos, 7, "pos 应推进到 hello 之后的第一个空白处");
    assert_eq!(
        spaced.rest(),
        "   world  ",
        "rest 从当前 pos 开始，保留剩余空白"
    );
    assert_eq!(
        spaced.next_word(),
        Some("world"),
        "连续多个空格不产生空单词"
    );
    assert_eq!(spaced.pos, 15, "pos 应推进到 world 之后的第一个空白处");
    assert_eq!(spaced.next_word(), None, "只剩尾部空白时应返回 None");
    assert_eq!(spaced.pos, 15, "返回 None 时 pos 不再推进");
    assert_eq!(spaced.rest(), "  ", "尾部空白仍然留在 rest 里");

    // 边界值：末尾没有空白，最后一个单词一直取到输入末尾
    let mut tail = Parser::new("alpha beta");
    assert_eq!(tail.next_word(), Some("alpha"));
    assert_eq!(
        tail.rest(),
        " beta",
        "消费一个单词后 rest 从 pos 开始，pos 停在单词后的第一个空白处"
    );
    assert_eq!(
        tail.next_word(),
        Some("beta"),
        "末尾无空白也要能取到最后一个单词"
    );
    assert_eq!(tail.pos, 10, "末尾无空白时 pos 推进到输入长度");
    assert_eq!(tail.rest(), "", "全部消费完 rest 为空");
    assert_eq!(tail.next_word(), None, "消费完后再取是 None");

    // 边界值：单单词、无任何空白
    let mut solo = Parser::new("solo");
    assert_eq!(
        solo.next_word(),
        Some("solo"),
        "只有前导空白需要跳过时也要正确"
    );
    assert_eq!(solo.pos, 4, "pos 推进到输入末尾");
    assert_eq!(solo.rest(), "");
    assert_eq!(solo.next_word(), None);
}

#[test]
fn ex10_3_first_or() {
    // 边界值：空切片 → fallback（长短都要覆盖）
    assert_eq!(
        first_or(&[], "fallback"),
        "fallback",
        "空切片应返回 fallback"
    );
    assert_eq!(first_or(&[], ""), "", "空切片 + 空 fallback 返回空串");
    assert_eq!(first_or(&[], "x"), "x", "fallback 比元素短时也照用");
    assert_eq!(
        first_or(&[], "a-very-long-fallback-string"),
        "a-very-long-fallback-string",
        "fallback 比元素长时也照用"
    );

    // 正常值：非空切片返回第一个元素，fallback 完全不参与
    let items = vec!["alpha".to_string(), "beta".to_string()];
    assert_eq!(
        first_or(&items, "x"),
        "alpha",
        "fallback 比元素短：仍返回第一个元素"
    );
    assert_eq!(
        first_or(&items, "a-very-long-fallback-string"),
        "alpha",
        "fallback 比元素长：仍返回第一个元素"
    );
    assert_eq!(
        first_or(&items, ""),
        "alpha",
        "空 fallback 也抢不走第一个元素"
    );

    // 边界值：第一个元素是空串 ≠ 没有元素
    let empty_first = vec![String::new(), "second".to_string()];
    assert_eq!(
        first_or(&empty_first, "fallback"),
        "",
        "Some(空串) 必须返回空串，不能当成空切片去用 fallback"
    );

    // 单元素切片
    let single = vec!["only".to_string()];
    assert_eq!(first_or(&single, "fallback"), "only");

    // 返回值是 items 上的切片，而不是新分配的
    assert_eq!(
        first_or(&items, "x").as_ptr(),
        items[0].as_ptr(),
        "返回值应借自 items[0]（生命周期来自 items）"
    );
}
