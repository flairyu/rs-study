//! 第 6 章自动评测。**不要修改本文件**——它是你的裁判。
//!
//! 运行：`python3 tools/check.py ch06_error_handling`（或 `cargo test -p ch06_error_handling`）

use ch06_error_handling::*;

#[test]
fn ex6_1_parse_age() {
    // 正常值与边界值
    assert_eq!(parse_age("0"), Ok(0), "\"0\" 是合法年龄");
    assert_eq!(parse_age("42"), Ok(42));
    assert_eq!(parse_age("150"), Ok(150), "150 是上界，仍然合法");
    assert_eq!(parse_age(" 42 "), Ok(42), "首尾空白应该先被 trim 掉");
    assert_eq!(parse_age("\t18\n"), Ok(18), "各种空白都要 trim");

    // 非法值
    assert_eq!(
        parse_age("151"),
        Err("年龄超出范围".to_string()),
        "151 超过 150"
    );
    assert_eq!(
        parse_age(""),
        Err("年龄不能为空".to_string()),
        "空串应该报「年龄不能为空」"
    );
    assert_eq!(
        parse_age("   "),
        Err("年龄不能为空".to_string()),
        "纯空白等价于空串"
    );
    assert_eq!(
        parse_age("abc"),
        Err("不是合法数字: abc".to_string()),
        "错误消息要带上原始输入"
    );
    assert_eq!(
        parse_age("-1"),
        Err("不是合法数字: -1".to_string()),
        "u8 不接受负数"
    );
}

#[test]
fn ex6_2_sum_numbers() {
    // 正常值
    assert_eq!(sum_numbers("1,2,3"), Ok(6));
    assert_eq!(sum_numbers("0"), Ok(0), "单个 0");
    assert_eq!(sum_numbers("0,0,0"), Ok(0), "全是 0");
    assert_eq!(sum_numbers("-5,10"), Ok(5), "负数用 i64 表示");
    assert_eq!(sum_numbers(" 1 , 2 , 3 "), Ok(6), "每段都要 trim");

    // 空输入与非法输入
    assert_eq!(sum_numbers(""), Ok(0), "空串按 0 处理");
    assert_eq!(sum_numbers("   "), Ok(0), "全空白按 0 处理");
    assert_eq!(sum_numbers("\t\n "), Ok(0), "各种空白都按 0 处理");
    assert_eq!(sum_numbers("1,abc,3"), Err("不是合法数字: abc".to_string()));
    assert_eq!(
        sum_numbers("1, 2 ,x"),
        Err("不是合法数字: x".to_string()),
        "报错的必须是 trim 之后的那一段"
    );
    assert_eq!(
        sum_numbers("-1,2.5"),
        Err("不是合法数字: 2.5".to_string()),
        "浮点不算合法整数"
    );
}

#[test]
fn ex6_3_parse_port() {
    // 正常值与边界值
    assert_eq!(parse_port("0"), Ok(0));
    assert_eq!(parse_port("80"), Ok(80));
    assert_eq!(parse_port("8080"), Ok(8080));
    assert_eq!(parse_port("65535"), Ok(65535), "u16 端口上界");
    assert_eq!(parse_port(" 443 "), Ok(443), "首尾空白应该先被 trim 掉");
    assert_eq!(parse_port("\t8080\n"), Ok(8080), "各种空白都要 trim");

    // 非法值
    assert_eq!(
        parse_port("65536"),
        Err(PortError::TooLarge(65536)),
        "65536 应该触发 TooLarge，而不是 NotANumber"
    );
    assert_eq!(
        parse_port("70000"),
        Err(PortError::TooLarge(70000)),
        "大于 u16 上界都是 TooLarge"
    );
    assert_eq!(parse_port(""), Err(PortError::Empty), "空串");
    assert_eq!(parse_port("   "), Err(PortError::Empty), "纯空白");
    assert_eq!(
        parse_port("abc"),
        Err(PortError::NotANumber("abc".to_string()))
    );
    assert_eq!(
        parse_port("-1"),
        Err(PortError::NotANumber("-1".to_string())),
        "负数不是合法端口；u32 解析负数会失败"
    );
}
