#!/usr/bin/env python3
"""Writes the synthetic 80-ion msngrafo/1 used by the layout checks (tests/msn_layout_80.js): one root, then 7 levels of 10 ions and a last one of 9, masses only, no real data.
Built from the shape of tests_e2e/fixtures/msngrafo_esempio.json; `vista` is then added by the engine (`mzlab-cli msn-graph complete`)."""
import json, subprocess, sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
USCITA = RAIZ / "tests_e2e" / "fixtures" / "msngrafo_80.json"
TAGLIE = [1] + [10] * 7 + [9]
LIVELLI = len(TAGLIE)
INIZIO = [sum(TAGLIE[:l]) for l in range(LIVELLI)]


def lcg(seed):
    s = seed
    while True:
        s = (s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        yield s >> 33


def costruisci():
    base = json.loads((RAIZ / "tests_e2e" / "fixtures" / "msngrafo_esempio.json").read_text())
    nodo0 = base["nodi"][1]
    r = lcg(80)
    nodi, archi = [], []
    for l in range(LIVELLI):
        for k in range(TAGLIE[l]):
            i = INIZIO[l] + k
            n = json.loads(json.dumps(nodo0))
            n.update(id=f"n{i}", mz=round(500.0 - l * 40.0 - k * 3.0, 5), livello=l + 1, isolato=False, albero=[], osservazioni=1,
                     rel_max=round(100.0 - k * 7.0, 1), pos={"col": l, "riga": k}, formula=None)
            nodi.append(n)
    for l in range(LIVELLI - 1):
        for k in range(TAGLIE[l + 1]):
            a = INIZIO[l + 1] + k
            padri = {INIZIO[l] + next(r) % TAGLIE[l]}
            if next(r) % 5 == 0:
                padri.add(INIZIO[l] + next(r) % TAGLIE[l])
            for da in sorted(padri):
                perdita = round(nodi[da]["mz"] - nodi[a]["mz"], 5)
                archi.append({"da": f"n{da}", "a": f"n{a}", "perdita": {"da_massa": perdita, "formula": None, "ppm": None, "compatibile": []}, "isolamento": False,
                              "misure": [{"albero": f"x{l + 2}", "livello": l + 2, "att": "cid", "ce": 35, "n_scansioni": 10, "rel": round(5.0 + (next(r) % 900) / 10.0, 1)}]})
    g = {k: base[k] for k in ("formato", "motore", "sorgente", "opzioni", "polarita", "ione", "avvisi")}
    g["opzioni"] = dict(base["opzioni"], formula=None)
    g["ione"] = None
    g["radici"] = ["n0"]
    g["conteggi"] = dict(base["conteggi"], nodi=len(nodi), archi=len(archi))
    g.update(nodi=nodi, archi=archi, spettri=[])
    return g


def main():
    cli = RAIZ / "target" / "release" / "mzlab-cli"
    if len(sys.argv) > 1:
        cli = Path(sys.argv[1])
    tmp = USCITA.with_suffix(".senza-vista.json")
    tmp.write_text(json.dumps(costruisci(), indent=1))
    subprocess.run([str(cli), "msn-graph", "complete", str(tmp), "--out", str(USCITA)], check=True)
    tmp.unlink()
    print(USCITA)


if __name__ == "__main__":
    main()
