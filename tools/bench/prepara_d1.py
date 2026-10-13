"""Normalised truth of D1 (`verita/D1.json` of the private repository) from the MAF table of MTBLS733 and the RT check (`controlla_rt.py`).

One entry per MAF row: `{id, mz, rt, atteso, formula, ione, visto, d_rt}`. The MAF says `[M-H]+` for every row (inconsistent with positive ESI): the ion is
derived from the formula and the m/z (`[M+H]+`, `[M+Na]+`, or `null` when neither matches within 5 ppm). `visto`/`d_rt` come from the RT report (apex - truth RT).

    python3 tools/bench/prepara_d1.py MAF.tsv RT_REPORT.json OUT.json
"""
from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

try:
    from bench import verita as V
except ImportError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from bench import verita as V
from mzlab.chem.elements import MASS, PROTON, parse_formula

PPM = 5.0
NA_ION = MASS["Na"] - 0.000548579909


def ione(formula: str, mz: float):
    if not formula:
        return None
    M = sum(MASS[e] * n for e, n in parse_formula(formula).items())
    for nome, v in (("[M+H]+", M + PROTON), ("[M+Na]+", M + NA_ION)):
        if abs(mz - v) / v * 1e6 < PPM:
            return nome
    return None


def prepara(maf: str, rt_report: dict) -> dict:
    righe = list(csv.DictReader(open(maf, newline="", encoding="utf-8"), delimiter="\t"))
    vere = V.da_maf(maf, atteso="Compound concentration ratio")
    rt = {c["id"]: c for c in rt_report["compounds"]}
    formula = {f"{(r.get('metabolite_identification') or 'maf' + str(i))}@{i}": r.get("chemical_formula") for i, r in enumerate(righe)}
    out = []
    for c in vere:
        r = rt.get(c["id"], {})
        out.append({"id": c["id"], "mz": c["mz"], "rt": c["rt"], "atteso": c["atteso"], "formula": formula.get(c["id"]) or None,
                    "ione": ione(formula.get(c["id"]) or "", c["mz"]), "visto": bool(r.get("visto")), "d_rt": r.get("d")})
    return {"schema": "bench/verita/1", "dataset": "D1", "accessione": "MTBLS733", "n": len(out), "composti": out}


if __name__ == "__main__":
    t = prepara(sys.argv[1], json.loads(Path(sys.argv[2]).read_text()))
    Path(sys.argv[3]).write_text(json.dumps(t, indent=1) + "\n")
    print(t["n"], "compounds")
