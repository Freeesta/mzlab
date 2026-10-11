"""Writes docs/agenti/mappa.md: one line per first-party source file (purpose + where its main functions start), grouped by folder.

    python3 tools/indice.py              # rewrite docs/agenti/mappa.md
    python3 tools/indice.py --controlla  # exit 1 if the file is out of date (CI); no files are written

Deterministic: only the tracked files and their text are read. The purpose is the first comment / docstring line of the file,
otherwise the file name. `definizioni()` is also used by tools/controlla.py (size limits)."""
from __future__ import annotations

import ast
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "agenti" / "mappa.md"
SKIP = ("vendor/", "ei-dati.js", "web/elements.js", "glossario-dati.js", "web/esempi/", "/target/")
GROUPS = [  # (title, path prefix, one line per file?)
    ("Teoria e Pratica (`mzlab/web/teoria/`)", "mzlab/web/teoria/", True),
    ("Pagina principale (`mzlab/web/`)", "mzlab/web/", True),
    ("Python (`mzlab/`)", "mzlab/", True),
    ("Rust, nucleo (`crates/mzlab-core`)", "crates/mzlab-core/", True),
    ("Rust, librerie (`crates/mzlab-lib*`)", "crates/mzlab-lib", True),
    ("Rust, WASM e desktop (`crates/mzlab-wasm`, `crates/mzlab-desktop`)", "crates/", True),
    ("Strumenti (`tools/`)", "tools/", True),
    ("mzFinder (`TP_Mine/`)", "TP_Mine/", False),
    ("Test (`tests/`, `tests_e2e/`)", "tests", False),
]
JS_DEF = re.compile(r"^( {0,2})(?:export\s+)?(?:async\s+)?(?:function\*?\s+([A-Za-z_$][\w$]*)|(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*=>)")
RS_DEF = re.compile(r"^( {0,4})(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(fn|struct|enum|trait)\s+([A-Za-z_]\w*)")


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    return sorted(f for f in out if f.endswith((".js", ".py", ".rs")) and not any(k in f for k in SKIP))


def definizioni(path: str, text: str | None = None) -> list[tuple[str, int, int]]:
    """Top-level definitions of a file as (name, first line, length in lines), in file order."""
    if text is None:
        text = (ROOT / path).read_text(encoding="utf-8", errors="replace")
    lines = text.split("\n")
    found: list[tuple[str, int, int]] = []
    if path.endswith(".py"):
        try:
            tree = ast.parse(text)
        except SyntaxError:
            return []
        for n in tree.body:
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                found.append((n.name, n.lineno, (n.end_lineno or n.lineno) - n.lineno + 1))
        return found
    starts: list[tuple[str, int, int]] = []  # name, line, indent
    rx = RS_DEF if path.endswith(".rs") else JS_DEF
    for i, l in enumerate(lines, 1):
        m = rx.match(l)
        if not m:
            continue
        name = m.group(3) if path.endswith(".rs") else (m.group(2) or m.group(3))
        starts.append((name, i, len(m.group(1))))
    for k, (name, i, ind) in enumerate(starts):
        end = len(lines)
        for name2, j, ind2 in starts[k + 1:]:
            if ind2 <= ind:
                end = j - 1
                break
        # drop trailing blank / closing lines shared with the next block
        while end > i and not lines[end - 1].strip():
            end -= 1
        found.append((name, i, end - i + 1))
    return found


def scopo(path: str, text: str) -> str:
    """First comment / docstring line of the file; the file name when there is none."""
    if path.endswith(".py"):
        try:
            d = ast.get_docstring(ast.parse(text))
        except SyntaxError:
            d = None
        if d:
            return d.strip().split("\n")[0][:100]
    else:
        for l in text.split("\n")[:25]:
            s = l.strip()
            if s.startswith(("// @ts-check", "#!", '"use strict"', "'use strict'")) or not s:
                continue
            m = re.match(r"(?://!?|///|/\*\*?|\*)\s*(.*?)\s*(?:\*/)?$", s)
            if m and m.group(1) and not set(m.group(1)) <= set("-=*/ "):
                return m.group(1)[:100]
            break
    return "(" + Path(path).stem + ")"


def render() -> str:
    files = tracked()
    texts = {f: (ROOT / f).read_text(encoding="utf-8", errors="replace") for f in files}
    out = ["# Mappa del codice",
           "Generata da `python3 tools/indice.py` (non modificare a mano; la CI controlla che sia aggiornata). Una riga per file: "
           "righe, scopo (prima riga di commento o docstring), poi `funzione:riga` per le più grandi. Per orientarsi: `grep -n` sul nome, "
           "poi lettura a intervalli. Aree e file d'area: AGENTS.md §5.", ""]
    used: set[str] = set()
    for title, prefix, per_file in GROUPS:
        sel = [f for f in files if f.startswith(prefix) and f not in used]
        if not sel:
            continue
        used.update(sel)
        out.append(f"## {title}")
        if per_file:
            for f in sel:
                t = texts[f]
                n = t.count("\n") + 1
                defs = definizioni(f, t)
                big = sorted(defs, key=lambda d: (-d[2], d[1]))[:5]
                big = sorted((d for d in big if d[2] >= 30), key=lambda d: d[1])
                tail = (" — " + ", ".join(f"{d[0]}:{d[1]}" for d in big)) if big else ""
                out.append(f"- `{f[len(prefix):] if f.startswith(prefix) and prefix != 'crates/mzlab-lib' else f}` ({n}) {scopo(f, t)}{tail}")
        else:
            dirs: dict[str, list[str]] = {}
            for f in sel:
                parts = f.split("/")
                dirs.setdefault("/".join(parts[:-1]) or ".", []).append(f)
            for d, fs in dirs.items():
                tot = sum(texts[f].count("\n") + 1 for f in fs)
                out.append(f"- `{d}/` — {len(fs)} file, {tot} righe: " + ", ".join(Path(f).stem for f in fs[:60]) + (" …" if len(fs) > 60 else ""))
        out.append("")
    return "\n".join(out).rstrip("\n") + "\n"


def main() -> int:
    text = render()
    if "--controlla" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print("docs/agenti/mappa.md is out of date: run python3 tools/indice.py and commit it")
            return 1
        print(f"mappa.md up to date ({text.count(chr(10))} lines)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)} ({text.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
