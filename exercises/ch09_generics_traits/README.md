# ch09_generics_traits — 第 9 章 · 泛型与 trait

配套文档：[`docs/01-syntax/ch09-generics-traits.md`](../../docs/01-syntax/ch09-generics-traits.md)

本章练的是「一份代码服务多种类型」和「用 trait 描述类型能做什么」：
泛型函数、trait 定义与实现、`dyn Trait` 动态分发。

## 三题

| 题号 | 函数 / 类型 | 考点 |
|---|---|---|
| 9-1 | `largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T>` | 泛型函数、trait bound、`&[T]` 借来的元素不能 move、空切片 → `None` |
| 9-2 | `pub trait Describable` + `Point` 的 `impl Describable for Point` | trait 定义与 `impl ... for ...`、`{}` 打印 `f64` 不补 `.0` |
| 9-3 | `describe_all(items: &[Box<dyn Describable>]) -> Vec<String>` | trait 对象、`Box<dyn Trait>` 自动解引用、动态分发 |

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch09_generics_traits
```

或者直接用 cargo：

```bash
cargo test -p ch09_generics_traits
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch09_generics_traits --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- 第 9-2 题的输出必须是 `Point(1.5, -2)` 这种形式：`f64` 用 `{}` 打印时
  `1.5` → `"1.5"`，但 `-2.0` → `"-2"`、`1.0` → `"1"`，**不会**自动补 `.0`。
- 第 9-1 题的元素在切片里是**借来的**，所以必须靠 `Copy` 复制出一个值再返回，
  不能试图把 `items[0]` 的所有权移出来。
