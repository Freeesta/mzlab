"""Reference answers of two parts of mzFinder that the Rust Dig reimplements (`crates/mzlab-feat/src/legacy/network_pred.rs`, `hidden_fam.rs`), for their
parity tests: the choice of the predecessor of a candidate (`hr/network.py::predecessors`) and the call of a hidden product under an in-source fragment
(`hr/filters.py::isf_hidden`, without sessions).

    python3 tools/finder_funzioni_riferimento.py [--golden]

Writes tests/golden/finder_funzioni.json with --golden, else prints it. Synthetic inputs only. The names of the transformations are not compared: Python
reads its own table and Rust the rules of `TP_Mine/data/regole_tp.json`; the case set uses changes that both have.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))

from mzlab.chem import elements as E  # noqa: E402
from tpmine.hr import filters as FL  # noqa: E402
from tpmine.hr import formula as F  # noqa: E402
from tpmine.hr import network as NW  # noqa: E402

PARENT = "C8H11N4O2"
ELS = F.element_order(E.parse_formula(PARENT))
TABLE = NW.transformation_table(ELS)

# (formulas of the candidates, maxima of their profiles, {(predecessor, candidate): (cosine, matched peaks)}); indices into the candidates
PRED_CASES = [
    (["C8H11N4O3", "C8H11N4O4", "C8H9N4O3", "C8H11N4O4", "C20H11N4O2"], [5.0, 20.0, 10.0, 2.0, 5.0], {}),
    (["C8H11N4O3", "C8H11N4O4", "C8H9N4O3", "C8H11N4O4", "C20H11N4O2"], [5.0, 20.0, 10.0, 2.0, 5.0], {(0, 2): (0.8, 7)}),
    (["C8H9N4O4", "C8H11N4O3"], [5.0, 8.0], {}),
    (["C8H11N4O3", "C12H15N4O2"], [5.0, 10.0], {(0, 1): (0.7, 5)}),
    (["C8H11N4O3", "C12H15N4O2"], [5.0, 10.0], {(0, 1): (0.7, 2)}),
    (["C8H11N4O3", "C12H15N4O2"], [5.0, 10.0], {(0, 1): (0.5, 9)}),
    (["C8H11N4O3", "C8H11N4O4"], [30.0, 5.0], {}),
    (["C8H11N4O3", "C8H11N4O4"], [None, None], {}),
]


def pred_case(forms, tmax, cos):
    cands = [{"id": i, "formula": f, "kinetics": {"tmax": t} if t is not None else {}} for i, (f, t) in enumerate(zip(forms, tmax))]
    res = NW.predecessors(cands, ELS, PARENT, TABLE, cos=cos or None)
    out = []
    for r in res:
        p = r["predecessor"]
        out.append({"to": r["id"] + 1, "from": None if p is None else (0 if p == "progenitore" else p + 1), "cos": r["cos"]})
    return {"forms": forms, "tmax": tmax, "cos": [[j, i, c, n] for (j, i), (c, n) in sorted((cos or {}).items())], "pred": out}


def measure(parent_area, areas, shifts, heights):
    import numpy as np
    return {"parent_area": parent_area, "areas": np.array(areas, float), "apex_shift": np.array(shifts), "heights": np.array(heights, float)}


# (times, parent area, [area, apex shift, height] of the mass in every file)
ISF_CASES = [
    ([-1, 0, 5, 10, 20, 40], [2e8, 2e8, 1.5e8, 1e8, 5e7, 2e7], [(2e6, 0, 3e5), (2e6, 0, 3e5), (1.5e6, 0, 2e5), (1e6, 1, 1e5), (5e5, 0, 6e4), (2e5, 0, 2e4)]),
    ([-1, 0, 5, 10, 20, 40], [6e8, 6e8, 5e8, 4e8, 2e8, 7e7], [(2e5, 0, 6e4), (2e5, 0, 6e4), (2.9e5, 0, 7e4), (3.4e5, 0, 8e4), (4e5, 0, 9e4), (4.4e5, 0, 9e4)]),
    ([-1, 0, 5, 10, 20, 40], [6e8, 6e8, 5e8, 4e8, 2e8, 7e7], [(2e5, 0, 6e4), (2e5, 0, 6e4), (2e5, 20, 8e4), (2e5, 25, 9e4), (2e5, 22, 9e4), (2e5, 20, 7e4)]),
    ([-1, 0, 5, 10, 20, 40], [6e8, 6e8, 5e8, 4e8, 2e8, 7e7], [(2e5, 0, 6e4), (2e5, 0, 6e4), (2e5, 20, 4e4), (2e5, 25, 3e4), (2e5, 22, 9e3), (2e5, 20, 7e3)]),
    ([-1, 0, 5, 10, 20, 40], [6e8, 6e8, 5e8, 4e8, 2e8, 7e7], [(2e5, 0, 6e4), (2e5, 0, 6e4), (2e5, 20, 8e4), (2e5, 2, 9e4), (2e5, 3, 9e4), (2e5, 4, 7e4)]),
    ([-1, 0, 5, 10], [1e8, 1e8, 1e8, 1e8], [(1e6, 0, 2e5), (1e6, 0, 2e5), (1.1e6, 0, 2e5), (1.05e6, 0, 2e5)]),
    ([0, 5, 10, 20], [1e8, 1e8, 1e8, 1e8], [(1e6, 0, 2e5), (4e6, 0, 5e5), (6e6, 0, 7e5), (9e6, 0, 9e5)]),
    ([-1, 0, 5, 10], [5e6, 5e6, 5e6, 5e6], [(1e5, 0, 2e5), (1e5, 0, 2e5), (9e6, 0, 2e5), (9e6, 0, 2e5)]),
]


def isf_case(times, parents, ions):
    ms = [measure(p, [a], [s], [h]) for p, (a, s, h) in zip(parents, ions)]
    res = FL.isf_hidden(ms, times, None, [261.1])
    r = res[0] if res else {"candidate": False, "n_up": 0, "n_shift": 0}
    return {"times": times, "parent": parents, "ion": [list(x) for x in ions], "candidate": bool(r["candidate"]), "n_up": r["n_up"], "n_shift": r["n_shift"]}


def build() -> dict:
    return {"predecessors": [pred_case(*c) for c in PRED_CASES], "isf_hidden": [isf_case(*c) for c in ISF_CASES]}


if __name__ == "__main__":
    out = json.dumps(build(), separators=(",", ":"))
    if "--golden" in sys.argv:
        p = AQUI.parent / "tests" / "golden" / "finder_funzioni.json"
        p.write_text(out)
        print(p, len(out), "bytes")
    else:
        print(out)
