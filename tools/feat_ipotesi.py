"""Reference of the hypotheses of `crates/mzlab-feat/src/hypo`: the parent times the rules of mzFinder, written in plain Python loops.

    python3 tools/feat_riferimento.py OUT --solo ipotesi [--golden]

writes `ipotesi_regole.json`: for an invented parent formula, every net change of formula that `tpmine.hr.network.transformation_table` gives in one or two
steps (the table, not the data file: the Rust test also checks the file against it), with the fewest steps, the formula, the neutral mass and the [M+H]+ m/z.
Rules of a class (`rules.CLASS_RULES`) apply only if the parent has the group. No compound names: the parent is a formula of invented atoms."""
from __future__ import annotations

import itertools
import json

from mzlab.chem import elements as E
from tpmine import chem
from tpmine.hr import network as NW
from tpmine.hr import rules as RU

ELS = ["C", "H", "N", "O", "S", "F", "Cl", "Br"]
PADRI = ["C12H17ClN2O3S", "C9H13N3O2", "C15H22BrNO4"]
MIN_MZ = 50.0


def ipotesi(formula: str, adduct: str = "[M+H]+", max_steps: int = 2) -> list[dict]:
    parent = E.parse_formula(formula)
    pvec = [int(parent.get(e, 0)) for e in ELS]
    table = [(n, v) for n, v in NW.transformation_table(ELS) if n not in RU.CLASS_RULES or RU.has_group(n, pvec, ELS, None)]
    best: dict[tuple, int] = {}
    for k in range(1, max_steps + 1):
        for combo in itertools.combinations_with_replacement(range(len(table)), k):
            net = tuple(sum(table[i][1][j] for i in combo) for j in range(len(ELS)))
            if any(net) and net not in best:
                best[net] = k
    out = []
    for net, k in best.items():
        f = {e: int(parent.get(e, 0)) + n for e, n in zip(ELS, net)}
        if any(n < 0 for n in f.values()):
            continue
        f = {e: n for e, n in f.items() if n}
        m = E.mass(f)
        mz = E.ion_mz(m, adduct)
        if mz >= MIN_MZ:
            out.append({"formula": E.fmt(f), "delta": chem.fmt_delta({e: n for e, n in zip(ELS, net) if n}), "steps": k, "mass": m, "mz": mz})
    out.sort(key=lambda r: (r["mz"], r["formula"]))
    return out


def un_file_ipotesi(out, golden) -> str:
    doc = {"adduct": "[M+H]+", "max_steps": 2, "parents": [{"formula": f, "hypotheses": ipotesi(f)} for f in PADRI]}
    text = json.dumps(doc, indent=1) + "\n"
    (out / "ipotesi_regole.json").write_text(text)
    if golden is not None:
        (golden / "ipotesi_regole.json").write_text(text)
    return f"{sum(len(p['hypotheses']) for p in doc['parents'])} ipotesi"
