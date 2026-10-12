# ch05_structs_enums · 结构体、枚举与模式匹配

第 5 章的练习工程。配套讲解：`docs/01-syntax/ch05-structs-enums.md`。

## 怎么用

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch05_structs_enums       # 跑自动评测
cargo test -p ch05_structs_enums                # 等价的手工方式
```

卡住了要提示：

```bash
python3 tools/ai_review.py ch05_structs_enums --hint 2
```

## 规则

- 只改 `src/lib.rs`：把每个 `unimplemented!()` 换成你的实现。
- **不要修改 `tests/chapter.rs`**——它是裁判。
- 不允许添加第三方依赖；本 crate 只依赖标准库（`std`）。

## 三题

| 题号 | 任务 | 对应 API |
|---|---|---|
| 5-1 | 实现 `Rectangle` 的构造、面积、正方形判断与缩放 | `Rectangle::{new, area, is_square, scale}` |
| 5-2 | 用 `match` 解构 `Shape` 并计算面积 | `shape_area` |
| 5-3 | 把 `Option<i32>` 描述成四种字符串 | `describe_opt` |

## 边界值提醒（评测里真的会考）

- 矩形：面积为 0（某边为 0）、负数边长、`scale(0.0)`、`is_square` 的 1e-9 容差。
- 圆：半径为 0。
- `Option`：`None`、`Some(0)`、正数、负数，四种都要处理，`Some(0)` 不能掉进正数分支。
