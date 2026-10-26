#!/usr/bin/env python3
"""Release gate: refuses a new version while the maps and the guides disagree with the code. Runs only at release time (tools/versione.py --tag), never in the normal CI.

    python3 tools/cancello_rilascio.py            # all the checks for the version of Cargo.toml
    python3 tools/cancello_rilascio.py --root D   # same on another tree (tests)

Each check prints OK or FAIL with file:line and the remedy; exit 1 if any FAIL. Stdlib only. Rules: docs/agenti/rilascio.md."""
from __future__ import annotations

import argparse
import glob
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_COMMITS = 150  # a graphify map older than this many commits on the touched folders is stale
Result = tuple[str, list[str]]  # (check name, problems "file:line: text -> remedy")


def _lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8").splitlines() if path.is_file() else []


def step_ids(root: Path) -> list[str]:
    """Ids of every step/1 descriptor: `mzlab-cli steps --json` when the real tree has the binary, else the exported docs/schemi/steps-*-1.json."""
    exe = shutil.which("mzlab-cli") if root == ROOT else None
    if exe:
        r = subprocess.run([exe, "steps", "--json"], capture_output=True, text=True)
        if r.returncode == 0:
            try:
                return [s["id"] for s in json.loads(r.stdout)]
            except (ValueError, KeyError, TypeError):
                pass
    ids: list[str] = []
    for f in sorted(glob.glob(str(root / "docs/schemi/steps-*-1.json"))):
        ids += [s["id"] for s in json.loads(Path(f).read_text(encoding="utf-8"))]
    return ids


def check_steps(root: Path) -> Result:
    text = "".join(f.read_text(encoding="utf-8") for f in (root / "docs/agenti/cli.md", root / "docs/agenti/cli-passi.md") if f.is_file())
    ids = step_ids(root)
    if not ids:
        return "steps in cli.md", ["docs/schemi/steps-*-1.json:1: no step descriptor found -> export them (docs/agenti/rust-step.md)"]
    return "steps in cli.md", [
        f"docs/agenti/cli.md:1: step `{i}` has no row -> add a line in the area file (docs/agenti/cli.md)" for i in ids if i not in text
    ]


def check_area_files(root: Path) -> Result:
    cited = "".join(f.read_text(encoding="utf-8") for f in (root / "AGENTS.md", root / "docs/agenti/INDICE.md") if f.is_file())
    return "area files in docs/agenti/INDICE.md", [
        f"docs/agenti/INDICE.md:1: docs/agenti/{p.name} is not cited -> add it to the row of its area"
        for p in sorted((root / "docs/agenti").glob("*.md"))
        if p.name not in cited
    ]


def known_commands(root: Path) -> set[str]:
    lib = root / "crates/mzlab-cli/src/lib.rs"
    if not lib.is_file():
        return set()
    src = lib.read_text(encoding="utf-8")
    plain = re.search(r"const PLAIN: \[&str; \d+\] = \[([^\]]*)\]", src)
    return set(re.findall(r'Some\("([a-z][a-z-]*)"\)', src)) | set(re.findall(r'"([a-z][a-z-]*)"', plain.group(1) if plain else ""))


def check_guides(root: Path) -> Result:
    known = known_commands(root)
    bad: list[str] = []
    if not known:
        return "commands in llms.txt / per-agenti.md", []
    for rel in ("llms.txt", "docs/agenti/per-agenti.md"):
        for n, line in enumerate(_lines(root / rel), 1):
            for a, b in re.findall(r"mzlab-cli ([a-z][a-z-]*)(?: ([a-z][a-z-]*))?", line):
                if a not in known:
                    bad.append(f"{rel}:{n}: `mzlab-cli {a}` does not exist -> fix the guide or the command")
                elif a == "flow" and b and b not in known and not b.isupper():
                    bad.append(f"{rel}:{n}: `mzlab-cli flow {b}` does not exist -> fix the guide or the command")
    return "commands in llms.txt / per-agenti.md", bad


def check_readme(root: Path) -> Result:
    bad = []
    for c in sorted((root / "crates").glob("*/")):
        if not c.is_dir():
            continue
        names = [p for p in c.iterdir() if p.name.lower() in ("readme.md", "leggimi.md")]
        if not any(p.read_text(encoding="utf-8").strip() for p in names):
            bad.append(f"{c.relative_to(root)}/README.md:1: missing or empty -> write 3-5 lines: what the crate does and who uses it")
    return "README of every crate", bad


def check_map(root: Path) -> Result:
    bad = []
    if root == ROOT:
        r = subprocess.run([sys.executable, "tools/indice.py", "--controlla"], cwd=root, capture_output=True, text=True)
        if r.returncode != 0:
            bad.append("tools/indice.py:1: the code map misses files -> python3 tools/indice.py")
        gx = root / "graphify-out"
        stamp = max((p.stat().st_mtime for p in gx.rglob("*") if p.is_file()), default=0) if gx.is_dir() else 0
        if stamp:
            g = subprocess.run(["git", "rev-list", "--count", f"--since={int(stamp)}", "HEAD", "--", "crates", "mzlab", "tools"], cwd=root, capture_output=True, text=True)
            if g.returncode == 0 and g.stdout.strip().isdigit() and int(g.stdout) > MAX_COMMITS:
                bad.append(f"graphify-out:1: map older than {MAX_COMMITS} commits on crates/ mzlab/ tools/ -> graphify update .")
    return "code map", bad


def version_of(root: Path) -> str:
    m = re.search(r"\[workspace\.package\][^\[]*?^version\s*=\s*\"([^\"]+)\"", (root / "Cargo.toml").read_text(encoding="utf-8"), re.M | re.S)
    return m.group(1) if m else ""


def check_checklist(root: Path, version: str | None = None) -> Result:
    v = version or version_of(root)
    rel = f"docs/rilascio/CHECKLIST-v{v}.md"
    lines = _lines(root / rel)
    if not lines:
        return "release checklist", [f"{rel}:1: missing -> cp docs/rilascio/CHECKLIST-TEMPLATE.md {rel} and tick every box"]
    bad = [f"{rel}:{n}: open box -> do it, then tick it: {l.strip()[:80]}" for n, l in enumerate(lines, 1) if re.match(r"\s*[-*] \[ \]", l)]
    if not any(re.match(r"\s*[-*] \[[xX]\]", l) for l in lines):
        bad.append(f"{rel}:1: no ticked box -> the file is not a checklist")
    return "release checklist", bad


CHECKS = (check_steps, check_area_files, check_guides, check_readme, check_map, check_checklist)


def run(root: Path = ROOT, version: str | None = None, out=print) -> int:
    failed = 0
    for fn in CHECKS:
        name, problems = fn(root, version) if fn is check_checklist else fn(root)
        out(f"{'FAIL' if problems else 'OK  '} {name}")
        for p in problems:
            out(f"       {p}")
        failed += bool(problems)
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--root", type=Path, default=ROOT)
    ap.add_argument("--version", help="version whose checklist is checked (default: Cargo.toml)")
    a = ap.parse_args(argv)
    return run(a.root.resolve(), a.version)


if __name__ == "__main__":
    sys.exit(main())
