"""Reference answers of the list of transformations of mzFinder (`tpmine.chem.parse_transformations`, the table of changes of `hr/network.py` and the
`lit:` references of `hr/rules.py`) for the parity test of `crates/mzlab-feat/src/legacy/transf.rs` (crates/mzlab-feat/tests/legacy_transf.rs).

    python3 tools/finder_transf_riferimento.py [--golden]

Writes tests/golden/finder_transf.json with --golden, else prints its size. Synthetic lists only.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))

from tpmine import chem as CH  # noqa: E402
from tpmine.hr import network as NW  # noqa: E402
from tpmine.hr import rules as RU  # noqa: E402

DEFAULT = CH.transformations_text()
CASES = {
    "default": DEFAULT,
    "empty": "",
    "new rows": DEFAULT + "\nmy methylation;+CH3-H;a note\nsulfation;+SO3\n# a comment\n\nnitration;+NO2-H",
    "only new": "ring opening;+O2-C\nacetylation;+C2H2O",
    "duplicate of a known change": "oxygen;+O\nwater loss;-H2O\nO again;+ O",
    "duplicate inside": "a;+N\nb;+N\nc;-N+N\nd;+N+C-C",
    "signs": "x;-C-H4\ny;+C+H4\nz;-Cl+H\nw;C2H4\nv;+O-O",
    "spaces and tabs": "  spaced  ;  + O H2 ; note  \n\ttabbed;\t+CH2\t",
    "literature": "hydroxylation;+O;lit: Smith 2020\nnew lit;+C2H4O;LIT : Jones 1999, doi\nempty lit;+C3;lit:\nnot lit;+C4;literature: x",
    "literature twice": "a;+C5H2;lit: first\nb;+C5H2;lit: second",
    "junk characters": "k;+O?;n\nl;+C!H2\nm;+O.\nn;%+N",
    "empty change": "e;+\nf;-",
    "windows lines": "r1;+C7\r\nr2;+C8\rr3;+C9\n",
    "bad: no change": "just a name",
    "bad: empty change": "name;",
    "bad: second line": "ok;+O\nbad line without semicolon",
    "bad: element": "q;+Xx",
    "a bracket ends the change": "q;+C(H2",
    "bad: starts with a digit": "q;+2O",
    "a sign then a bracket": "q;+(",
    "unicode": "épox;+O\nñ;+C10H3;ünï",
    "semicolons in the comment": "s;+C6;one;two;three",
}


def _dict(vec, els):
    return {e: int(n) for e, n in zip(els, vec) if int(n)}


def one(text: str) -> dict:
    try:
        rows = CH.parse_transformations(text)
    except Exception as e:  # noqa: BLE001  (ValueError and UserError: the page shows str(e))
        return {"error": str(e)}
    els = sorted({e for r in rows for e in r["delta"]} | {e for _, d in NW.HR_TRANSFORMATIONS for e in CH.parse_delta(d)}
                 | {e for t in CH.default_transformations() for e in t["delta"]})
    base = NW.transformation_table(els, None)
    full = NW.transformation_table(els, rows)
    lit = RU.literature(rows, els)
    return {
        "rows": [{"name": r["name"], "delta": r["delta"], "comment": r["comment"]} for r in rows],
        "new": [{"name": n, "delta": _dict(v, els)} for n, v in full[len(base):]],
        "literature": [{"delta": _dict(k, els), "source": s} for k, s in lit.items()],
    }


def build() -> dict:
    return {"cases": [{"name": k, "text": t, **one(t)} for k, t in CASES.items()]}


if __name__ == "__main__":
    out = json.dumps(build(), indent=1, ensure_ascii=False, sort_keys=True)
    if "--golden" in sys.argv:
        (AQUI.parent / "tests" / "golden" / "finder_transf.json").write_text(out + "\n", encoding="utf-8")
    print(len(out), "bytes")
