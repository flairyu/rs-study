# ch11_smart_pointers — 第 11 章 · 智能指针与内部可变性

配套文档：[`docs/01-syntax/ch11-smart-pointers.md`](../../docs/01-syntax/ch11-smart-pointers.md)

本章练的是所有权规则的三个「受控例外」：`Box<T>` 把值放到堆上（递归类型必需），
`Rc<T>` 让多个名字共享同一份数据，`RefCell<T>` 把借用检查从编译期挪到运行期。

## 三题

| 题号 | 函数 | 考点 |
|---|---|---|
| 11-1 | `from_slice(items: &[i32]) -> List` | 递归枚举必须 `Box`、`iter().rev().fold()` |
| 11-2 | `to_vec(list: &List) -> Vec<i32>` | 沿 `&List` 迭代、`&Box<List>` 的解引用强制转换 |
| 11-3 | `push_all(handles: &[Rc<RefCell<Vec<i32>>>], value: i32) -> usize` | 共享可变、`borrow_mut()` 守卫何时归还 |

`List` 已经定义好（派生 `Debug, PartialEq`），你只需要实现三个函数。

## 怎么做

1. 打开 `src/lib.rs`，把每个 `unimplemented!()` 换成你的实现。**不要改函数签名**。
2. 跑评测：

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch11_smart_pointers
```

或者直接用 cargo：

```bash
cargo test -p ch11_smart_pointers
```

3. 卡住了就让 AI 给提示（不会直接给答案）：

```bash
python3 tools/ai_review.py ch11_smart_pointers --hint 2
```

## 注意

- `tests/chapter.rs` 是裁判，**不要修改**。它每个测试都覆盖正常值 + 边界值
  （空切片、单元素、负数、空 `handles`、多个 handle 共享同一个 `Rc`）。
- 本章 crate 只依赖 `std`，离线可跑。
- 骨架期测试会失败在 `unimplemented!()` panic 上，这是正常的；只要不是**编译错误**就行。
- 11-3 里不要在两条语句之间持有 `borrow_mut()` 的临时值：守卫活到语句结束才归还，
  期间对同一个 `RefCell` 再借一次就会 panic。
