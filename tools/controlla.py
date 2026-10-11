"""Size limits of the first-party code: files over 500 lines and functions over 80 lines are WARNINGS; the check FAILS only when
the list gets worse than the saved base (tools/soglie_base.json): a file or function over the limit that is not in the base, or one
that grew by more than 10% (at least 10 lines) over its base. The base can only shrink.

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
    return {"file": files, "funzioni": funcs}


def peggiori(now: dict[str, dict[str, int]], base: dict[str, dict[str, int]]) -> list[str]:
    bad = []
    for kind in ("file", "funzioni"):
        for k, v in sorted(now[kind].items()):
            b = base.get(kind, {}).get(k)
            if b is None:
                bad.append(f"NEW {kind[:-1] if kind == 'file' else 'funzione'} over the limit: {k} ({v})")
            elif v > b + max(SLACK_MIN, int(b * SLACK)):
                bad.append(f"GREW {k}: {b} -> {v}")
    return bad


def main() -> int:
    now = misura()
    base = json.loads(BASE.read_text(encoding="utf-8")) if BASE.exists() else {"file": {}, "funzioni": {}}
    bad = peggiori(now, base)
    if "--aggiorna" in sys.argv:
        if bad and BASE.exists():
            print("refusing to update the base, it would get worse:\n  " + "\n  ".join(bad))
            return 1
        # only shrink: keep the smaller of base and now, drop the entries that are no longer over the limit
        new = {kind: {k: min(v, base.get(kind, {}).get(k, v)) for k, v in now[kind].items()} for kind in now}
        BASE.write_text(json.dumps(new, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"base written: {len(new['file'])} files over {MAX_FILE} lines, {len(new['funzioni'])} functions over {MAX_FUNC}")
        return 0
    print(f"WARN {len(now['file'])} files over {MAX_FILE} lines, {len(now['funzioni'])} functions over {MAX_FUNC} lines "
          f"(base: {len(base['file'])} / {len(base['funzioni'])})")
    if "--lista" in sys.argv:
        for kind in ("file", "funzioni"):
            for k, v in sorted(now[kind].items(), key=lambda kv: -kv[1]):
                print(f"  {v:5d}  {k}")
    for b in bad:
        print("FAIL " + b)
    shrunk = [k for kind in ("file", "funzioni") for k in base.get(kind, {}) if k not in now[kind]]
    if shrunk and not bad:
        print(f"note: {len(shrunk)} entries of the base are now under the limit: run python3 tools/controlla.py --aggiorna to narrow it")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
