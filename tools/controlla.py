"""Size limits of the first-party code: files over 500 lines and functions over 80 lines are WARNINGS; the check FAILS only when
the list gets worse than the saved base (tools/soglie_base.json): a file or function over the limit that is not in the base, or one
that grew by more than 10% (at least 10 lines) over its base. The base can only shrink.
The agent documents (AGENTS.md, docs/agenti/*.md except the generated mappa.md and misure-diario.md) have two more limits:
lines over 600 characters (counted per file) and files over 25 KB.

    python3 tools/controlla.py             # report + exit 1 if worse than the base
    python3 tools/controlla.py --lista     # also print every entry over a limit
    python3 tools/controlla.py --aggiorna  # rewrite the base from the current code; refuses if anything got worse
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import indice  # noqa: E402

BASE = indice.ROOT / "tools" / "soglie_base.json"
MAX_FILE, MAX_FUNC = 500, 80
MAX_DOC_LINE, MAX_DOC_BYTES = 600, 25 * 1024
DOC_SKIP = {"mappa.md", "misure-diario.md"}
KINDS = ("file", "funzioni", "doc_righe", "doc_kb")
LABEL = {"file": "file", "funzioni": "funzione", "doc_righe": "documento (righe lunghe)", "doc_kb": "documento (byte)"}
SLACK = 0.10          # growth tolerated on an entry that is already over the limit
SLACK_MIN = 10        # ... but at least this many lines


def misura() -> dict[str, dict[str, int]]:
    files: dict[str, int] = {}
    funcs: dict[str, int] = {}
    for f in indice.tracked():
        text = (indice.ROOT / f).read_text(encoding="utf-8", errors="replace")
        n = text.count("\n") + 1
        if n > MAX_FILE:
            files[f] = n
        for name, _line, length in indice.definizioni(f, text):
            if length > MAX_FUNC:
                k = f"{f}::{name}"
                funcs[k] = max(funcs.get(k, 0), length)
    long_lines: dict[str, int] = {}
    big: dict[str, int] = {}
    docs = [indice.ROOT / "AGENTS.md"] + sorted((indice.ROOT / "docs" / "agenti").glob("*.md"))
    for d in docs:
        if d.name in DOC_SKIP or not d.exists():
            continue
        raw = d.read_bytes()
        rel = d.relative_to(indice.ROOT).as_posix()
        n = sum(1 for line in raw.decode("utf-8", errors="replace").split("\n") if len(line) > MAX_DOC_LINE)
        if n:
            long_lines[rel] = n
        if len(raw) > MAX_DOC_BYTES:
            big[rel] = len(raw)
    return {"file": files, "funzioni": funcs, "doc_righe": long_lines, "doc_kb": big}


def peggiori(now: dict[str, dict[str, int]], base: dict[str, dict[str, int]]) -> list[str]:
    bad = []
    for kind in KINDS:
        for k, v in sorted(now[kind].items()):
            b = base.get(kind, {}).get(k)
            if b is None:
                bad.append(f"NEW {LABEL[kind]} over the limit: {k} ({v})")
            elif v > b + (max(SLACK_MIN, int(b * SLACK)) if kind in ("file", "funzioni") else 0):
                bad.append(f"GREW {k}: {b} -> {v}")
    return bad


def main() -> int:
    now = misura()
    base = json.loads(BASE.read_text(encoding="utf-8")) if BASE.exists() else {}
    # a limit that is not in the base file yet starts from the current state (--aggiorna writes it)
    base = {kind: base[kind] if kind in base else dict(now[kind]) for kind in KINDS}
    bad = peggiori(now, base)
    if "--aggiorna" in sys.argv:
        if bad and BASE.exists():
            print("refusing to update the base, it would get worse:\n  " + "\n  ".join(bad))
            return 1
        # only shrink: keep the smaller of base and now, drop the entries that are no longer over the limit
        new = {kind: {k: min(v, base.get(kind, {}).get(k, v)) for k, v in now[kind].items()} for kind in now}
        BASE.write_text(json.dumps(new, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"base written: {len(new['file'])} files over {MAX_FILE} lines, {len(new['funzioni'])} functions over {MAX_FUNC}, "
              f"{len(new['doc_righe'])} documents with lines over {MAX_DOC_LINE} characters, {len(new['doc_kb'])} documents over 25 KB")
        return 0
    print(f"WARN {len(now['file'])} files over {MAX_FILE} lines, {len(now['funzioni'])} functions over {MAX_FUNC} lines "
          f"(base: {len(base['file'])} / {len(base['funzioni'])}); documents: {len(now['doc_righe'])} with lines over "
          f"{MAX_DOC_LINE} characters, {len(now['doc_kb'])} over 25 KB")
    if "--lista" in sys.argv:
        for kind in KINDS:
            for k, v in sorted(now[kind].items(), key=lambda kv: -kv[1]):
                print(f"  {v:5d}  {k}")
    for b in bad:
        print("FAIL " + b)
    shrunk = [k for kind in KINDS for k in base[kind] if k not in now[kind]]
    if shrunk and not bad:
        print(f"note: {len(shrunk)} entries of the base are now under the limit: run python3 tools/controlla.py --aggiorna to narrow it")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
