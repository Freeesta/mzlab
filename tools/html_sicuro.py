"""Guard against HTML injection through template literals (docs/agenti/dati.md, «HTML sicuro»).

A line of mzlab/web/*.js that assigns innerHTML / outerHTML or calls insertAdjacentHTML with a `${...}` interpolation must use the tagged template
`h` (explore.js: every `${}` is escaped, `raw()` lets trusted markup through). The lines that still do not are counted per file in
tools/html_sicuro_base.json; the count can only go down.

    python3 tools/html_sicuro.py            # exit 1 if a file has more such lines than its base
    python3 tools/html_sicuro.py --aggiorna # lower the base to the current counts (refuses to raise it)
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import indice  # noqa: E402

BASE = indice.ROOT / "tools" / "html_sicuro_base.json"
SINK = re.compile(r"innerHTML|outerHTML|insertAdjacentHTML")
TAGGED = re.compile(r"\bh`")


def misura() -> dict[str, int]:
    """Per file, the number of lines with an HTML sink and a `${` that do not use the tag `h`."""
    out: dict[str, int] = {}
    for f in indice.tracked():
        if not (f.startswith("mzlab/web/") and f.endswith(".js")) or "/vendor/" in f or "/teoria/" in f or "/esempi/" in f:
            continue
        n = sum(1 for line in (indice.ROOT / f).read_text(encoding="utf-8", errors="replace").split("\n")
                if SINK.search(line) and "${" in line and not TAGGED.search(line))
        if n:
            out[f] = n
    return out


def peggiori(now: dict[str, int], base: dict[str, int]) -> list[str]:
    return [f"HTML sink with an unescaped template (use h`...` with raw() for trusted markup): {f} has {n}, base {base.get(f, 0)}"
            for f, n in sorted(now.items()) if n > base.get(f, 0)]


def main() -> int:
    now = misura()
    base = json.loads(BASE.read_text(encoding="utf-8")) if BASE.exists() else {}
    bad = peggiori(now, base) if BASE.exists() else []
    if "--aggiorna" in sys.argv:
        if bad:
            print("refusing to update the base, it would get worse:\n  " + "\n  ".join(bad))
            return 1
        BASE.write_text(json.dumps(now, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        print(f"base written: {sum(now.values())} lines in {len(now)} files")
        return 0
    print(f"html_sicuro: {sum(now.values())} lines in {len(now)} files (base {sum(base.values())})")
    for b in bad:
        print("FAIL " + b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
