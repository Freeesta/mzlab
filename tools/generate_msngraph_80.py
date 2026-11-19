#!/usr/bin/env python3
"""Writes the synthetic 80-ion msn-graph/1 used by the layout checks (tests/msn_layout_80.js): one root, then 7 levels of 10 ions and a last one of 9, masses only, no real data.
Built from the shape of tests_e2e/fixtures/msngrafo_esempio.json; `view` is then added by the engine (`mzlab-cli msn complete`)."""
import json, subprocess, sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
OUTPUT = RAIZ / "tests_e2e" / "fixtures" / "msngrafo_80.json"
TAGLIE = [1] + [10] * 7 + [9]
LEVELS = len(TAGLIE)
START = [sum(TAGLIE[:l]) for l in range(LEVELS)]


def lcg(seed):
    s = seed
    while True:
        s = (s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        yield s >> 33


def build():
    base = json.loads((RAIZ / "tests_e2e" / "fixtures" / "msngrafo_esempio.json").read_text())
    node0 = base["nodes"][1]
    r = lcg(80)
    nodes, edges = [], []
    for l in range(LEVELS):
        for k in range(TAGLIE[l]):
            i = START[l] + k
            n = json.loads(json.dumps(node0))
            n.update(id=f"n{i}", mz=round(500.0 - l * 40.0 - k * 3.0, 5), level=l + 1, isolated=False, tree=[], observations=1,
                     rel_max=round(100.0 - k * 7.0, 1), pos={"col": l, "row": k}, formula=None)
            nodes.append(n)
    for l in range(LEVELS - 1):
        for k in range(TAGLIE[l + 1]):
            a = START[l + 1] + k
            parents = {START[l] + next(r) % TAGLIE[l]}
            if next(r) % 5 == 0:
                parents.add(START[l] + next(r) % TAGLIE[l])
            for src in sorted(parents):
                loss = round(nodes[src]["mz"] - nodes[a]["mz"], 5)
                edges.append({"from": f"n{src}", "to": f"n{a}", "loss": {"da_mass": loss, "formula": None, "ppm": None, "compatible": []}, "isolation": False,
                              "measurements": [{"tree": f"x{l + 2}", "level": l + 2, "activation": "cid", "ce": 35, "n_scans": 10, "rel": round(5.0 + (next(r) % 900) / 10.0, 1)}]})
    g = {k: base[k] for k in ("schema", "engine", "source", "options", "polarity", "ion", "warnings")}
    g["options"] = dict(base["options"], formula=None)
    g["ion"] = None
    g["roots"] = ["n0"]
    g["counts"] = dict(base["counts"], nodes=len(nodes), edges=len(edges))
    g.update(nodes=nodes, edges=edges, spectra=[])
    return g


def main():
    cli = RAIZ / "target" / "release" / "mzlab-cli"
    if len(sys.argv) > 1:
        cli = Path(sys.argv[1])
    tmp = OUTPUT.with_suffix(".no-view.json")
    tmp.write_text(json.dumps(build(), indent=1))
    subprocess.run([str(cli), "msn-graph", "complete", str(tmp), "--out", str(OUTPUT)], check=True)
    tmp.unlink()
    print(OUTPUT)


if __name__ == "__main__":
    main()
