# exercises —— 自动评测的练习工作区

一个 **cargo workspace**，每章一个 crate，**只依赖标准库**（离线可编译）。

```bash
cd /home/yxb/code/rs-study

python3 tools/check.py --list            # 列出全部章节
python3 tools/check.py ch03_ownership    # 跑一章
python3 tools/check.py                   # 跑全部（首次较慢）
python3 tools/check.py --next            # 跑到第一个未通过处停下
```

也可以直接用 cargo（需要 `cargo` 在 `PATH` 上）：

```bash
cd exercises
cargo test -p ch03_ownership
cargo test -p ch03_ownership ex3_1_longest_word    # 只跑一个用例
cargo check -p ch03_ownership                      # 只做类型检查（快）
cargo clippy -p ch03_ownership -- -D warnings      # 上强度
```

## 目录约定

```
chXX_name/
├── Cargo.toml
├── README.md
├── src/lib.rs          ← 你唯一要改的文件（把 unimplemented!() 换成实现）
└── tests/chapter.rs    ← 裁判，不要改
```

## 设计原则（为什么练习题"永远能编译"）

每个习题函数的骨架都是：

```rust
pub fn foo(x: i32) -> i32 {
    unimplemented!("练习 1-1：把这里的 unimplemented!() 换成你的实现")
}
```

`unimplemented!()` 的类型是 `!`（never type），能强制转换成任何返回类型，
所以**骨架一定编译通过**。这意味着：

- 你看到的失败永远是**测试断言失败**（"行为不对"），而不是"编译不过"。
- 编译错误只可能来自**你自己写的代码**，归因清晰。
- LLM 批改时也能准确区分"没做"和"做错了"。

## 加新章节

见 [`../tools/AUTHORING.md`](../tools/AUTHORING.md) 与
[`../tools/CHAPTER-SPECS.md`](../tools/CHAPTER-SPECS.md)。要点：

1. 建 `chXX_name/{Cargo.toml,src/lib.rs,tests/chapter.rs,README.md}`
2. 在 `Cargo.toml`（本文件所在目录）的 `[workspace] members` 里登记
3. 在 `../tools/chapters.json` 里登记
4. **不要加第三方依赖**——需要依赖的实验放 `../labs/`
