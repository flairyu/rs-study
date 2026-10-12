# ch16_stdlib_tour · 第 16 章 标准库巡礼

教程文档：[`docs/02-stdlib/ch16-stdlib-tour.md`](../../docs/02-stdlib/ch16-stdlib-tour.md)

```bash
cd /home/yxb/code/rs-study
python3 tools/check.py ch16_stdlib_tour
```

| 练习 | 函数 | 考点 |
|---|---|---|
| 16-1 | `count_words_in_file` | `fs::read_to_string`、`io::Result`、`?` 与错误原样传播 |
| 16-2 | `file_stem_and_ext` | `Path` / `PathBuf`、主名与扩展名、`Option` 边界 |
| 16-3 | `round_trip_number` | `FromStr`/`parse`、`ParseFloatError`、`{:.2}` 格式化 |

改 [`src/lib.rs`](src/lib.rs) 里的三个 `unimplemented!()`。
第 1 题的测试会真实读写 `std::env::temp_dir()` 下的临时文件（用完即删）。
卡住就跑 `python3 tools/ai_review.py ch16_stdlib_tour --hint 1`。
