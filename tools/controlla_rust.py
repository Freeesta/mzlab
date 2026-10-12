"""Blocking Rust checks (job `rust` of .github/workflows/rust.yml): clippy and fmt against a base that can only shrink.
Clippy runs on the whole workspace with the default lints plus unwrap_used, indexing_slicing and cast_possible_truncation, which are
switched on only in the parsing modules (`#[warn]` on their `mod` line in lib.rs). The two deny-by-default lints that the old code
still triggers are lowered to warnings so that they are counted instead of stopping the build. The check FAILS when a file gets a lint
that is not in tools/clippy_base.json, or more of one than its base, or when a file that fmt accepted is no longer accepted.

    python3 tools/controlla_rust.py             # report + exit 1 if worse than the base
    python3 tools/controlla_rust.py --aggiorna  # rewrite the base from the current code; refuses if anything got worse (the first run, without a base, writes it)
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / "tools" / "clippy_base.json"
LOWERED = ["clippy::eq_op", "clippy::unused_io_amount"]


def clippy() -> dict[str, int]:
    cmd = ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--keep-going", "--message-format=json", "--"]
    for lint in LOWERED:
        cmd += ["-W", lint]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True).stdout
    seen: set[tuple[str, int, int, str]] = set()
    for line in out.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") != "compiler-message":
            continue
        d = m["message"]
        code = (d.get("code") or {}).get("code") or ""
        if not code.startswith("clippy::") or d["level"] not in ("warning", "error"):
            continue
        span = next((s for s in d["spans"] if s.get("is_primary")), None)
        if span:
            seen.add((span["file_name"], span["line_start"], span["column_start"], code))
    counts: dict[str, int] = {}
    for f, _l, _c, code in seen:
        k = f"{f}::{code}"
        counts[k] = counts.get(k, 0) + 1
    return counts


def fmt() -> list[str]:
    out = subprocess.run(["cargo", "fmt", "--all", "--check"], cwd=ROOT, capture_output=True, text=True).stdout
    files = {Path(p).resolve().relative_to(ROOT).as_posix() for p in re.findall(r"^Diff in (.+?):\d+:?$", out, re.M)}
    return sorted(files)


def peggiori(now: dict, base: dict) -> list[str]:
    bad = []
    for k, v in sorted(now["clippy"].items()):
        b = base["clippy"].get(k)
        if b is None:
            bad.append(f"NEW clippy: {k} ({v})")
        elif v > b:
            bad.append(f"GREW clippy {k}: {b} -> {v}")
    for f in now["fmt"]:
        if f not in base["fmt"]:
            bad.append(f"NEW fmt difference: {f} (run `cargo fmt`)")
    return bad


def main() -> int:
    now = {"clippy": clippy(), "fmt": fmt()}
    base = json.loads(BASE.read_text(encoding="utf-8")) if BASE.exists() else {"clippy": {}, "fmt": []}
    bad = peggiori(now, base)
    print(f"clippy: {sum(now['clippy'].values())} in base {sum(base['clippy'].values())}; fmt: {len(now['fmt'])} files not formatted, base {len(base['fmt'])}")
    if "--aggiorna" in sys.argv:
        if bad and BASE.exists():
            print("refusing to update, worse than the base:\n" + "\n".join(bad))
            return 1
        BASE.write_text(json.dumps(now, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        print(f"base written: {BASE.relative_to(ROOT)}")
        return 0
    if bad:
        print("\n".join(bad))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
