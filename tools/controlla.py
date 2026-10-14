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


BYTE_API = indice.ROOT / "tools" / "api_byte_ammesse.txt"
IDB_OK = {"mzlab/web/qqq-db.js", "mzlab/web/libreria.js", "mzlab/web/libreria-worker.js", "mzlab/web/liste.js", "TP_Mine/js/20_tpmine.js"}     # the schema point and the historical databases


def regole_statiche() -> list[str]:
    """Rules of memory that grep can keep true (docs/agenti/memoria.md): no exported WASM function returns a whole `Vec<u8>` unless it is in the list of
    tools/api_byte_ammesse.txt (`file::function`; new outputs go to a `WriteAt` sink), and IndexedDB is opened only in the files of IDB_OK."""
    import re
    bad = []
    allowed = {l.strip() for l in BYTE_API.read_text(encoding="utf-8").splitlines() if l.strip() and not l.startswith("#")} if BYTE_API.exists() else set()
    sig = re.compile(r"pub fn (\w+)\s*(?:<[^>]*>)?\([^{;]*?\)\s*->\s*(?:std::result::Result<|Result<)?Vec<u8>", re.S)
    for f in indice.tracked():
        if f.startswith("crates/") and "-wasm/src/" in f and f.endswith(".rs"):
            text = (indice.ROOT / f).read_text(encoding="utf-8", errors="replace")
            for m in sig.finditer(text):
                head = text[: m.start()].rstrip().split("\n")
                attrs = []
                for line in reversed(head):
                    if line.strip().startswith(("#[", "///", "//")):
                        attrs.append(line)
                    else:
                        break
                exported = any("wasm_bindgen" in a for a in attrs)
                in_impl = bool(re.search(r"#\[wasm_bindgen\]\s*\nimpl\b", text[: m.start()]))        # a method of an exported impl block (the last block before the function)
                if (exported or in_impl) and f"{Path(f).name}::{m.group(1)}" not in allowed:
                    bad.append(f"NEW exported WASM function that returns a whole Vec<u8>: {f}::{m.group(1)} (write to a WriteAt sink, or list it in tools/api_byte_ammesse.txt with a reason)")
        elif f.endswith(".js") and f.startswith(("mzlab/web/", "TP_Mine/")) and f not in IDB_OK and "/vendor/" not in f:
            if "indexedDB.open(" in (indice.ROOT / f).read_text(encoding="utf-8", errors="replace"):
                bad.append(f"indexedDB.open outside qqq-db.js: {f}")
    return bad


def regole_stato() -> list[str]:
    """E.tab / E.active / E.cur / E.panels and S.view / S.adding are written only by the transitions of mzlab/web/stato.js (docs/agenti/dati.md)."""
    import re
    rx = re.compile(r"\b(?:E\.(?:tab|active|cur|panels)|S\.(?:view|adding)|UIP\.tog)\s*=(?!=)")
    bad = []
    for f in indice.tracked():
        if f.endswith(".js") and f.startswith("mzlab/web/") and "/vendor/" not in f and f != "mzlab/web/stato.js":
            for n, line in enumerate((indice.ROOT / f).read_text(encoding="utf-8", errors="replace").split("\n"), 1):
                if not line.lstrip().startswith("//") and rx.search(line):
                    bad.append(f"direct write of the page state outside stato.js: {f}:{n} (use STATO.file/scheda/attiva/pannelli/vista/aggiunta/insieme)")
    return bad


ESP_BASE = indice.ROOT / "tools" / "esperimenti_base.json"
def confronti_esperimenti() -> dict[str, int]:
    """Per file of mzlab/web: comparisons of a variable with "full", "ms2" or "mrm" (the experiment type is data of esperimenti.js)."""
    import re
    rx = re.compile(r'[=!]==?\s*"(?:full|ms2|mrm)"|"(?:full|ms2|mrm)"\s*[=!]==?')
    out = {}
    for f in indice.tracked():
        if f.endswith(".js") and f.startswith("mzlab/web/") and "/vendor/" not in f and "/teoria/" not in f and f != "mzlab/web/esperimenti.js":
            n = sum(len(rx.findall(l)) for l in (indice.ROOT / f).read_text(encoding="utf-8", errors="replace").split("\n") if not l.lstrip().startswith("//"))
            if n:
                out[f] = n
    return out


def regole_esperimenti(aggiorna: bool = False) -> list[str]:
    """The comparisons with the experiment names outside esperimenti.js can only go down, file by file (base: tools/esperimenti_base.json)."""
    now = confronti_esperimenti()
    base = json.loads(ESP_BASE.read_text(encoding="utf-8")) if ESP_BASE.exists() else dict(now)
    bad = [f"comparisons with an experiment name grew in {f}: {n} (base {base.get(f, 0)}); read it from ESP in esperimenti.js" for f, n in sorted(now.items()) if n > base.get(f, 0)]
    if aggiorna and not bad:
        ESP_BASE.write_text(json.dumps({f: min(n, base.get(f, n)) for f, n in now.items()}, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    return bad


FRAME_BASE = indice.ROOT / "tools" / "frame_base.json"
def chiamate_frame() -> dict[str, int]:
    """Per file of mzlab/web: direct uses of requestAnimationFrame / cancelAnimationFrame / new ResizeObserver (frame.js owns them)."""
    import re
    rx = re.compile(r"\b(?:requestAnimationFrame|cancelAnimationFrame)\(|new ResizeObserver\(")
    out = {}
    for f in indice.tracked():
        if f.endswith(".js") and f.startswith("mzlab/web/") and "/vendor/" not in f and "/teoria/" not in f and f != "mzlab/web/frame.js":
            n = sum(len(rx.findall(l)) for l in (indice.ROOT / f).read_text(encoding="utf-8", errors="replace").split("\n") if not l.lstrip().startswith("//"))
            if n:
                out[f] = n
    return out


def regole_frame(aggiorna: bool = False) -> list[str]:
    """Direct frame-clock calls outside frame.js can only go down, file by file (base: tools/frame_base.json)."""
    now = chiamate_frame()
    base = json.loads(FRAME_BASE.read_text(encoding="utf-8")) if FRAME_BASE.exists() else dict(now)
    bad = [f"direct requestAnimationFrame/ResizeObserver calls grew in {f}: {n} (base {base.get(f, 0)}); use FRAME in frame.js" for f, n in sorted(now.items()) if n > base.get(f, 0)]
    if aggiorna and not bad:
        FRAME_BASE.write_text(json.dumps({f: min(n, base.get(f, n)) for f, n in now.items()}, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    return bad


ATTESE_BASE = indice.ROOT / "tools" / "attese_base.json"
def attese_fisse() -> dict[str, int]:
    """Per e2e script of tests_e2e/: fixed pauses (wait_for_timeout / time.sleep); wait on the STATE instead (attendi() in lib.py)."""
    import re
    rx = re.compile(r"\bwait_for_timeout\(|\btime\.sleep\(")
    out = {}
    for f in indice.tracked():
        if f.startswith("tests_e2e/") and f.endswith(".py"):
            n = sum(len(rx.findall(l)) for l in (indice.ROOT / f).read_text(encoding="utf-8", errors="replace").split("\n") if not l.lstrip().startswith("#"))
            if n:
                out[f] = n
    return out


def regole_attese(aggiorna: bool = False) -> list[str]:
    """Fixed pauses in the e2e scripts can only go down, file by file; a new script cannot have any (base: tools/attese_base.json)."""
    now = attese_fisse()
    base = json.loads(ATTESE_BASE.read_text(encoding="utf-8")) if ATTESE_BASE.exists() else dict(now)
    bad = [f"fixed pauses grew in {f}: {n} (base {base.get(f, 0)}); wait on the state with attendi() in tests_e2e/lib.py" for f, n in sorted(now.items()) if n > base.get(f, 0)]
    if aggiorna and not bad:
        ATTESE_BASE.write_text(json.dumps({f: min(n, base.get(f, n)) for f, n in now.items()}, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    return bad


def main() -> int:
    now = misura()
    base = json.loads(BASE.read_text(encoding="utf-8")) if BASE.exists() else {}
    # a limit that is not in the base file yet starts from the current state (--aggiorna writes it)
    base = {kind: base[kind] if kind in base else dict(now[kind]) for kind in KINDS}
    bad = peggiori(now, base) + regole_statiche() + regole_stato() + regole_esperimenti("--aggiorna" in sys.argv) + regole_frame("--aggiorna" in sys.argv) + regole_attese("--aggiorna" in sys.argv)
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
