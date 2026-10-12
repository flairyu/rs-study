#!/usr/bin/env python3
"""Rust 学习教程 · 练习评测器。

用法::

    python3 tools/check.py                     # 跑全部章节
    python3 tools/check.py ch01_basics         # 只跑一章（可以写多个）
    python3 tools/check.py --list              # 列出所有章节
    python3 tools/check.py --next              # 只跑到第一个没通过的章节
    python3 tools/check.py ch01_basics -v      # 打印完整测试输出
    python3 tools/check.py --offline           # 强制离线（不访问 crates.io）
    python3 tools/check.py --solutions         # 跑 solutions/ 里的参考答案（仓库自检）

退出码：全部通过 0；有失败 1；环境问题 2。

设计要点：
- 自动定位 cargo（PATH → ~/.cargo/bin → ~/.rustup/toolchains/*/bin），
  所以即使 cargo 不在 PATH 上也能用。
- 只依赖 Python 标准库。
- 每次运行都会把原始输出存到 reports/ 下，方便 ai_review.py 或人回看。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFIG_PATH = ROOT / "tools" / "chapters.json"
EXERCISES_DIR = ROOT / "exercises"
SOLUTIONS_DIR = ROOT / "solutions"
REPORTS_DIR = ROOT / "reports"

TEST_LINE_RE = re.compile(
    # 兼容两种形态：`test chapter::foo ... ok`（默认）与 `foo --- FAILED`（紧凑模式）
    r"^(?:test\s+)?(?P<name>.+?)(?:\s+\.\.\.\s+|\s+---\s+)(?P<status>ok|FAILED|ignored)\s*$",
    re.M,
)
SECTION_RE = re.compile(r"^(?:Running|Doc-tests)\s+(?P<target>.+?)\s*$", re.M)
RESULT_RE = re.compile(r"^test result:\s+(?:ok|FAILED)\.\s+(\d+) passed;\s+(\d+) failed", re.M)

C_OK = "\033[32m"
C_BAD = "\033[31m"
C_DIM = "\033[2m"
C_BOLD = "\033[1m"
C_YEL = "\033[33m"
C_OFF = "\033[0m"


def _color_enabled() -> bool:
    if os.environ.get("NO_COLOR"):
        return False
    return sys.stdout.isatty()


def paint(text: str, color: str) -> str:
    return f"{color}{text}{C_OFF}" if _color_enabled() else text


# --------------------------------------------------------------------------- #
# 环境
# --------------------------------------------------------------------------- #

def find_cargo() -> tuple[str | None, Path | None]:
    """返回 (cargo 可执行文件路径, 需要加到 PATH 的 bin 目录)。"""
    found = shutil.which("cargo")
    if found:
        return found, Path(found).resolve().parent

    candidates: list[Path] = [Path.home() / ".cargo" / "bin" / "cargo"]
    rustup_home = Path(os.environ.get("RUSTUP_HOME") or (Path.home() / ".rustup"))
    if (rustup_home / "toolchains").is_dir():
        candidates.extend(sorted((rustup_home / "toolchains").glob("*/bin/cargo")))
    for cand in candidates:
        if cand.is_file():
            return str(cand), cand.resolve().parent
    return None, None


def build_env(cargo_bin_dir: Path | None) -> dict:
    env = dict(os.environ)
    if cargo_bin_dir is not None:
        env["PATH"] = f"{cargo_bin_dir}{os.pathsep}{env.get('PATH', '')}"
    env["CARGO_TERM_COLOR"] = "never"
    env.setdefault("RUST_BACKTRACE", "0")
    return env


# --------------------------------------------------------------------------- #
# 章节索引
# --------------------------------------------------------------------------- #

def load_chapters() -> list[dict]:
    data = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    chapters = [c for c in data["chapters"] if c.get("crate")]
    chapters.sort(key=lambda c: c["order"])
    return chapters


def resolve_selection(chapters: list[dict], wanted: list[str]) -> list[dict]:
    if not wanted:
        return chapters
    by_key: dict[str, dict] = {}
    for c in chapters:
        for key in (c["id"], c["crate"], str(c["order"])):
            by_key[key] = c
    selected: list[dict] = []
    for w in wanted:
        key = w.strip()
        if key not in by_key:
            print(paint(f"⚠ 不认识的章节：{key}（用 --list 看全部）", C_YEL))
            continue
        if by_key[key] not in selected:
            selected.append(by_key[key])
    return selected


# --------------------------------------------------------------------------- #
# 跑测试
# --------------------------------------------------------------------------- #

def display_name(raw: str) -> str:
    """`chapter::ex1_1_foo` → `ex1_1_foo`，报告里更好读。"""
    prefix = "chapter::"
    return raw[len(prefix):] if raw.startswith(prefix) else raw


def parse_output(output: str) -> dict:
    """把 cargo test 输出切成「练习测试」与「文档测试」两组，并回退到汇总行。

    要兼容 libtest 的两种输出形态：
    - 默认逐行：`test chapter::ex1_1_foo ... ok`
    - 紧凑模式（`--test-threads=1`，或 `--quiet` 被透传时）：
      只打印失败的 `ex1_1_foo --- FAILED`，通过的**不逐行列出**，
      这时只能回退到 `test result: ok. N passed; M failed` 汇总行。

    文档测试的判别：优先看 `Doc-tests` 段落头；段落头不可用时，
    回退到名字形状（libtest 给文档测试起的名字形如
    `src/lib.rs - fahrenheit_to_celsius (line 20)`）。
    """
    sections: list[tuple[str, int]] = []
    for m in SECTION_RE.finditer(output):
        sections.append((m.group("target"), m.start()))

    tests: list[dict] = []
    doctests: list[dict] = []
    for m in TEST_LINE_RE.finditer(output):
        name = m.group("name").strip()
        pos = m.start()

        target = ""
        in_doc_section = False
        for sec_name, start in sections:
            if start < pos:
                target = sec_name
                in_doc_section = sec_name.startswith("Doc-tests")
            else:
                break

        is_doc = in_doc_section or (" - " in name and "(line " in name)
        entry = {"name": name, "status": m.group("status"), "target": target}
        (doctests if is_doc else tests).append(entry)

    # 紧凑模式下拿不到逐用例结果，退化成汇总数字（此时无法列出用例名）。
    fallback = None
    if not tests and not doctests:
        totals = [(int(p), int(f)) for p, f in RESULT_RE.findall(output)]
        if totals:
            fallback = {
                "passed": sum(p for p, _ in totals),
                "failed": sum(f for _, f in totals),
            }
    return {"tests": tests, "doctests": doctests, "fallback": fallback}


def run_chapter(chapter: dict, cargo: str, env: dict, offline: bool, timeout: int,
                workdir: Path | None = None) -> dict:
    crate = chapter["crate"]
    workdir = workdir or EXERCISES_DIR
    # 注意：**不要加 --quiet**。cargo 的 -q 会被透传给 libtest，让它进入
    # 「每个用例只打一个点」的紧凑模式，我们就拿不到逐用例的通过/失败明细了。
    # --no-fail-fast：练习测试挂了也要继续跑文档测试，一次给出全貌。
    cmd = [cargo, "test", "--no-fail-fast", "-p", crate]
    if offline:
        cmd.append("--offline")
    cmd += ["--", "--test-threads=1"]

    started = time.time()
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(workdir),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout,
        )
        output, code = proc.stdout, proc.returncode
    except subprocess.TimeoutExpired as exc:
        output = (exc.stdout or "") + f"\n[超时] 超过 {timeout} 秒未结束\n"
        code = -1
    elapsed = time.time() - started

    parsed = parse_output(output)
    fallback = parsed["fallback"]
    # 编译失败的特征：既没有逐用例结果，也没有汇总结果，而退出码非 0
    compile_error = code != 0 and not parsed["tests"] and not parsed["doctests"] and fallback is None

    REPORTS_DIR.mkdir(exist_ok=True)
    stamp = time.strftime("%Y%m%d-%H%M%S")
    log_path = REPORTS_DIR / f"{crate}-{stamp}.log"
    header = (
        f"# {chapter['title']} ({crate})\n"
        f"# 命令: {' '.join(cmd)}\n"
        f"# 工作目录: {workdir}\n"
        f"# 退出码: {code}  耗时: {elapsed:.1f}s\n"
        f"{'-' * 70}\n"
    )
    log_path.write_text(header + output, encoding="utf-8")

    if fallback is not None:
        # 紧凑模式：只有汇总数字，拿不到用例名
        passed = fallback["passed"]
        total = fallback["passed"] + fallback["failed"]
        doc_passed = doc_total = 0
        compact = True
    else:
        passed = sum(1 for t in parsed["tests"] if t["status"] == "ok")
        total = len(parsed["tests"])
        doc_passed = sum(1 for t in parsed["doctests"] if t["status"] == "ok")
        doc_total = len(parsed["doctests"])
        compact = False
    return {
        "chapter": chapter,
        "crate": crate,
        "exit_code": code,
        "compile_error": compile_error,
        "tests": parsed["tests"],
        "doctests": parsed["doctests"],
        "passed": passed,
        "total": total,
        "doc_passed": doc_passed,
        "doc_total": doc_total,
        "compact": compact,
        "elapsed": elapsed,
        "log_path": str(log_path.relative_to(ROOT)),
        "raw_output": output,
        "cmd": cmd,
    }


# --------------------------------------------------------------------------- #
# 展示
# --------------------------------------------------------------------------- #

def compile_error_digest(output: str, limit: int = 14) -> list[str]:
    lines = [ln for ln in output.splitlines() if ln.startswith("error") or ln.startswith("  -->")]
    return lines[:limit]


def show_result(result: dict, verbose: bool) -> None:
    chapter = result["chapter"]
    print()
    print(paint(f"▶ {chapter['id']}  {chapter['title']}", C_BOLD))

    if result["compile_error"]:
        print(paint("  ✗ 编译失败——测试根本没跑起来。先修编译错误。", C_BAD))
        for line in compile_error_digest(result["raw_output"]):
            print(f"    {line}")
        print(paint(f"    完整日志: {result['log_path']}", C_DIM))
        return

    if result["total"] == 0:
        print(paint("  ⚠ 没有发现测试。是不是 tests/chapter.rs 缺失或没写 #[test]？", C_YEL))
        print(paint(f"    完整日志: {result['log_path']}", C_DIM))
        return

    mark = "✓" if result["passed"] == result["total"] else "✗"
    color = C_OK if result["passed"] == result["total"] else C_BAD
    doc_note = paint("  (文档测试 {}/{})".format(result["doc_passed"], result["doc_total"]), C_DIM)
    print("  {} 练习 {}/{} 通过{}".format(
        paint(mark, color), result["passed"], result["total"], doc_note))
    if result.get("compact"):
        print(paint("  （紧凑模式：只拿到汇总计数，没有逐用例明细）", C_DIM))
    for t in result["tests"]:
        name = display_name(t["name"])
        if t["status"] == "ok":
            print(f"    {paint('✓', C_OK)} {name}")
        else:
            print(f"    {paint('✗', C_BAD)} {name}")
    if verbose:
        print(paint("  ---- 原始输出 ----", C_DIM))
        print(result["raw_output"].rstrip())
    else:
        print(paint(f"    日志: {result['log_path']}", C_DIM))


def show_summary(results: list[dict]) -> None:
    print()
    print(paint("═" * 62, C_BOLD))
    print(paint("  进度总览", C_BOLD))
    print(paint("═" * 62, C_BOLD))
    done = 0
    total_tests = 0
    passed_tests = 0
    for r in results:
        chapter = r["chapter"]
        total_tests += r["total"]
        passed_tests += r["passed"]
        ok = r["total"] > 0 and r["passed"] == r["total"]
        done += 1 if ok else 0
        if r["compile_error"]:
            status = paint("编译失败", C_BAD)
        elif r["total"] == 0:
            status = paint("无测试  ", C_YEL)
        else:
            status = paint(f"{r['passed']}/{r['total']} 通过", C_OK if ok else C_BAD)
        print(f"  {chapter['order']:>2}. {chapter['id']:<26} {status}")
    print(paint("─" * 62, C_DIM))
    print(f"  章节：{done}/{len(results)} 全绿    断言：{passed_tests}/{total_tests} 个测试通过")
    if done == len(results):
        print(paint("\n  🎉 全部通过！可以进入下一章了。", C_OK))
    else:
        nxt = next((r for r in results if r["passed"] != r["total"] or r["total"] == 0), None)
        if nxt is not None:
            print(f"\n  下一步：{paint('python3 tools/check.py ' + nxt['crate'], C_BOLD)}"
                  f"   卡住了就跑 {paint('python3 tools/ai_review.py ' + nxt['crate'] + ' --hint 2', C_BOLD)}")


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #

def main() -> int:
    parser = argparse.ArgumentParser(
        description="跑练习测试并生成进度报告",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("chapters", nargs="*", help="章节 id / crate 名 / 序号，留空表示全部")
    parser.add_argument("--list", action="store_true", help="列出所有章节")
    parser.add_argument("--next", action="store_true", help="只跑到第一个没通过的章节为止")
    parser.add_argument("-v", "--verbose", action="store_true", help="打印完整 cargo 输出")
    parser.add_argument("--offline", action="store_true", help="给 cargo 加 --offline")
    parser.add_argument("--timeout", type=int, default=600, help="单章超时秒数（默认 600）")
    parser.add_argument("--json", action="store_true", help="额外输出机器可读的 JSON")
    parser.add_argument(
        "--solutions", action="store_true",
        help="改跑 solutions/ 里的参考答案（仓库自检：这里应该全绿）",
    )
    args = parser.parse_args()

    chapters = load_chapters()

    if args.list:
        print(f"{'序号':<4}{'crate':<28}{'标题':<34}文档")
        for c in chapters:
            print(f"{c['order']:<4}{c['id']:<28}{c['title']:<34}{c['doc']}")
        return 0

    selected = resolve_selection(chapters, args.chapters)
    workdir = SOLUTIONS_DIR if args.solutions else EXERCISES_DIR
    if args.solutions:
        # 还没写参考实现的章节直接跳过，不然会刷一屏 "no targets specified"。
        selected = [c for c in selected if (workdir / c["crate"] / "src" / "lib.rs").is_file()]
    if not selected:
        print(paint("没有选中任何章节。", C_YEL))
        return 2

    cargo, bin_dir = find_cargo()
    if cargo is None:
        print(paint("✗ 找不到 cargo。", C_BAD))
        print("  请先安装 Rust 工具链：")
        print("    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh")
        print("  或者把已有 toolchain 的 bin 目录加入 PATH。")
        return 2
    print(paint(f"cargo: {cargo}", C_DIM))

    env = build_env(bin_dir)
    results: list[dict] = []
    for chapter in selected:
        print(paint(f"\n编译并测试 {chapter['crate']} ...", C_DIM), end="", flush=True)
        result = run_chapter(chapter, cargo, env, args.offline, args.timeout, workdir=workdir)
        print("\r" + " " * 60 + "\r", end="")
        results.append(result)
        show_result(result, args.verbose)
        if args.next and (result["passed"] != result["total"] or result["total"] == 0):
            print(paint("\n（--next）在第一个未通过的章节停下。", C_DIM))
            break

    show_summary(results)

    if args.json:
        print()
        print(json.dumps(
            [{"crate": r["crate"], "passed": r["passed"], "total": r["total"],
              "compile_error": r["compile_error"], "log": r["log_path"]} for r in results],
            ensure_ascii=False, indent=2,
        ))

    all_ok = all(r["total"] > 0 and r["passed"] == r["total"] for r in results)
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
