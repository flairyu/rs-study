#!/usr/bin/env python3
"""Rust 学习教程 · AI 批改与指导。

它会：
1. 跑一遍该章的测试，拿到失败详情；
2. 读该章教程文档（**自动去掉参考答案**，避免剧透）；
3. 读你写的 `src/lib.rs`；
4. 把这三样交给一个大模型，让它指出问题、按"提示层级"给你指导。

配置（任选其一）::

    # 方式 A：环境变量（推荐）
    export RUST_TUTOR_API_KEY=sk-xxxx
    export RUST_TUTOR_BASE_URL=https://api.deepseek.com/v1   # 可选，默认就是这个
    export RUST_TUTOR_MODEL=deepseek-chat                    # 可选

    # 方式 B：写配置文件 tools/ai_config.json
    {"api_key": "sk-xxxx", "base_url": "https://api.deepseek.com/v1", "model": "deepseek-chat"}

任何兼容 OpenAI `/chat/completions` 的服务都能用（OpenAI、DeepSeek、Moonshot、
通义、本地 vLLM / Ollama 的 OpenAI 兼容端口……）。

用法::

    python3 tools/ai_review.py ch01_basics              # 默认 hint 层级 1
    python3 tools/ai_review.py ch01_basics --hint 2     # 提示更具体
    python3 tools/ai_review.py ch01_basics --hint 3     # 几乎给答案
    python3 tools/ai_review.py ch01_basics --solution   # 完整参考实现 + 讲解
    python3 tools/ai_review.py ch01_basics --dry-run    # 不调用 API，把 prompt 存成文件

没有 API key 时，`--dry-run` 会把整份 prompt 写到
`reports/ai-prompt-<crate>.md`，你可以直接粘到任意聊天窗口里用。
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check import (  # noqa: E402
    EXERCISES_DIR,
    REPORTS_DIR,
    ROOT,
    build_env,
    find_cargo,
    load_chapters,
    paint,
    C_BAD,
    C_BOLD,
    C_DIM,
    C_OK,
    C_YEL,
    resolve_selection,
    run_chapter,
)

CONFIG_FILE = ROOT / "tools" / "ai_config.json"

HINT_DESC = {
    0: "只说清哪一题错了、涉及哪个概念，不要给任何思路提示。",
    1: "指出问题出在哪一步，给一句方向性提示，但不要给出 API 名或代码。",
    2: "给出该用的关键 API / 语言特性名称，以及一句伪代码级思路，但不要写完整实现。",
    3: "给出接近完整的实现思路和关键代码片段，但**故意留一处**让学习者自己补全。",
}


# --------------------------------------------------------------------------- #
# 配置
# --------------------------------------------------------------------------- #

def load_llm_config(args) -> dict:
    cfg: dict = {}
    if CONFIG_FILE.is_file():
        try:
            cfg = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            print(paint(f"⚠ tools/ai_config.json 不是合法 JSON：{exc}", C_YEL))
    cfg["api_key"] = args.api_key or os.environ.get("RUST_TUTOR_API_KEY") or cfg.get("api_key") or ""
    cfg["base_url"] = (
        args.api_base
        or os.environ.get("RUST_TUTOR_BASE_URL")
        or cfg.get("base_url")
        or "https://api.deepseek.com/v1"
    )
    cfg["model"] = args.model or os.environ.get("RUST_TUTOR_MODEL") or cfg.get("model") or "deepseek-chat"
    return cfg


# --------------------------------------------------------------------------- #
# 素材收集
# --------------------------------------------------------------------------- #

def strip_answers(doc_text: str) -> str:
    """去掉文档里的参考答案，防止模型直接抄给你。"""
    for marker in ("\n## 参考答案", "\n## 参考解答", "\n<details>"):
        idx = doc_text.find(marker)
        if idx != -1:
            doc_text = doc_text[:idx] + "\n\n（参考答案部分已对批改模型隐藏）\n"
    return doc_text


def read_source(crate: str) -> dict[str, str]:
    crate_dir = EXERCISES_DIR / crate
    files: dict[str, str] = {}
    for rel in ("src/lib.rs", "tests/chapter.rs", "README.md"):
        path = crate_dir / rel
        if path.is_file():
            files[rel] = path.read_text(encoding="utf-8")
    src_dir = crate_dir / "src"
    if src_dir.is_dir():
        for extra in sorted(src_dir.glob("*.rs")):
            rel = f"src/{extra.name}"
            if rel not in files:
                files[rel] = extra.read_text(encoding="utf-8")
    return files


def failing_digest(result: dict, max_lines: int = 90) -> str:
    if result["compile_error"]:
        return "【编译失败】\n" + "\n".join(result["raw_output"].splitlines()[:max_lines])
    failed = [t["name"] for t in result["tests"] if t["status"] != "ok"]
    if not failed:
        return "全部通过，没有失败信息。"
    body = "\n".join(result["raw_output"].splitlines()[:max_lines])
    return f"失败用例：{', '.join(failed)}\n\n原始输出（截断）：\n{body}"


def build_prompt(chapter: dict, result: dict, sources: dict[str, str], hint: int, solution: bool) -> tuple[str, str]:
    doc_path = ROOT / chapter["doc"]
    doc_text = doc_path.read_text(encoding="utf-8") if doc_path.is_file() else "（文档缺失）"
    doc_text = strip_answers(doc_text)

    if solution:
        task = (
            "学习者卡住了，请求**完整参考实现**。请：\n"
            "1. 逐个练习给出完整、可直接编译的 Rust 实现；\n"
            "2. 每段实现后解释关键点，以及常见错误写法为什么错；\n"
            "3. 最后给一张「本章必须记住的 3 件事」清单。"
        )
    else:
        task = (
            f"请按提示层级 {hint} 指导学习者：{HINT_DESC[hint]}\n\n"
            "输出结构：\n"
            "## 1. 总体判断\n（一句话：是概念没懂，还是实现细节错了）\n"
            "## 2. 逐题诊断\n（每题：错在哪里 → 为什么错 → 按层级给提示。题目全对的也要说一句「对了，但可以更好」）\n"
            "## 3. 需要补的概念\n（只列真正相关的，给出最小示例）\n"
            "## 4. 下一步\n（一条可执行命令 + 一个自检问题）"
        )

    system = (
        "你是一位资深 Rust 导师，正在批改一位有 C/C++/Python 基础的学习者的练习。\n"
        "风格要求：\n"
        "- 中文回答，术语保留英文原词（ownership / borrow / trait / lifetime 等）。\n"
        "- 直说问题，不要客套，不要复述题目。\n"
        "- **绝对不要**在 hint < 3 时给出完整可编译的答案代码。\n"
        "- 指出错误时引用编译器错误的要点，但不要粘贴整段报错。\n"
        "- 涉及 C/C++/Python 对照时点一句，帮助建立直觉。\n"
    )

    parts = [
        f"# 章节：{chapter['title']}（crate: {chapter['crate']}）",
        "## 该章教程正文（已隐藏参考答案）",
        doc_text,
        "## 学习者的当前实现",
    ]
    for rel, text in sources.items():
        lang = "rust" if rel.endswith(".rs") else "markdown"
        parts.append(f"### 文件 {rel}\n```{lang}\n{text}\n```")
    parts += [
        "## 评测结果",
        f"练习通过：{result['passed']}/{result['total']}",
        failing_digest(result),
        "## 你的任务",
        task,
    ]
    return system, "\n\n".join(parts)


# --------------------------------------------------------------------------- #
# 调用模型
# --------------------------------------------------------------------------- #

def chat(cfg: dict, system: str, user: str, timeout: int = 180) -> str:
    url = cfg["base_url"].rstrip("/") + "/chat/completions"
    payload = {
        "model": cfg["model"],
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": 0.3,
        "stream": False,
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {cfg['api_key']}",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    try:
        return data["choices"][0]["message"]["content"]
    except (KeyError, IndexError) as exc:
        raise RuntimeError(f"模型返回格式不认识：{json.dumps(data, ensure_ascii=False)[:800]}") from exc


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #

def main() -> int:
    parser = argparse.ArgumentParser(description="让 AI 批改你的 Rust 练习", epilog=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("chapter", help="章节 id / crate 名 / 序号")
    parser.add_argument("--hint", type=int, default=1, choices=[0, 1, 2, 3], help="提示层级，默认 1")
    parser.add_argument("--solution", action="store_true", help="直接要完整参考实现")
    parser.add_argument("--dry-run", action="store_true", help="不调用 API，把 prompt 写进 reports/")
    parser.add_argument("--api-key", default=None)
    parser.add_argument("--api-base", default=None)
    parser.add_argument("--model", default=None)
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()

    chapters = load_chapters()
    selected = resolve_selection(chapters, [args.chapter])
    if not selected:
        print(paint(f"✗ 找不到章节 {args.chapter}，用 python3 tools/check.py --list 看全部", C_BAD))
        return 2
    chapter = selected[0]

    cargo, bin_dir = find_cargo()
    if cargo is None:
        print(paint("✗ 找不到 cargo，先装 Rust 工具链。", C_BAD))
        return 2

    print(paint(f"▶ {chapter['title']}", C_BOLD))
    print(paint("  正在跑测试收集失败信息 ...", C_DIM))
    result = run_chapter(chapter, cargo, build_env(bin_dir), offline=False, timeout=600)
    if result["total"] and result["passed"] == result["total"]:
        print(paint(f"  ✓ 这一章已经全绿（练习 {result['passed']}/{result['total']}）。"
                    "把 --solution 去掉也还是有话可说，但建议直接进下一章。", C_OK))

    sources = read_source(chapter["crate"])
    system, user = build_prompt(chapter, result, sources, args.hint, args.solution)

    REPORTS_DIR.mkdir(exist_ok=True)
    stamp = time.strftime("%Y%m%d-%H%M%S")
    prompt_path = REPORTS_DIR / f"ai-prompt-{chapter['crate']}-{stamp}.md"
    prompt_path.write_text(f"# SYSTEM\n\n{system}\n\n# USER\n\n{user}\n", encoding="utf-8")
    rel_prompt = prompt_path.relative_to(ROOT)

    cfg = load_llm_config(args)
    if args.dry_run or not cfg["api_key"]:
        if not cfg["api_key"] and not args.dry_run:
            print(paint("\n⚠ 没有配置 API key（RUST_TUTOR_API_KEY）。已退化为 dry-run。", C_YEL))
            print("  配置方法见 python3 tools/ai_review.py --help")
        print(f"\n  prompt 已保存：{paint(str(rel_prompt), C_BOLD)}")
        print("  把它整份粘进任意聊天窗口，同样能得到批改。")
        return 0

    print(paint(f"  调用模型 {cfg['model']} @ {cfg['base_url']} ...", C_DIM))
    try:
        answer = chat(cfg, system, user, timeout=args.timeout)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", "replace")[:600]
        print(paint(f"✗ HTTP {exc.code}：{detail}", C_BAD))
        print(f"  prompt 仍在 {rel_prompt}")
        return 1
    except Exception as exc:  # noqa: BLE001
        print(paint(f"✗ 调用失败：{exc}", C_BAD))
        print(f"  prompt 仍在 {rel_prompt}")
        return 1

    answer_path = REPORTS_DIR / f"ai-review-{chapter['crate']}-{stamp}.md"
    answer_path.write_text(answer, encoding="utf-8")

    print()
    print(paint("═" * 62, C_BOLD))
    print(paint(f"  AI 批改 · {chapter['title']}", C_BOLD))
    print(paint("═" * 62, C_BOLD))
    print(answer.strip())
    print()
    print(paint(f"  已存档：{answer_path.relative_to(ROOT)}", C_DIM))
    return 0


if __name__ == "__main__":
    sys.exit(main())
