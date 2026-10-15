"""Compares the strategies that choose the m/z labels of the side spectrum of the map (mzlab/web/mappa-render.js, MAPREND.pick) on synthetic scenes:
a few strong components with isotopes at low and middle m/z, noise peaks everywhere, and small planted peaks at high m/z (the signal one looks for).
  python3 tools/confronta_etichette.py        prints the table; tests/test_mappa_render.py asserts it."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np

JS = Path(__file__).resolve().parents[1] / "mzlab" / "web" / "mappa-render.js"
RUN = """const R=require(process.argv[1]);const sc=JSON.parse(require('fs').readFileSync(0,'utf8'));
console.log(JSON.stringify(sc.map(s=>R.pick(s.mz,s.y,s.o).map(c=>[c.i,c.snr]))));"""
LIMIT = 8


def scene(rng, targets=3):
    mz = list(rng.uniform(120, 480, int(rng.integers(45, 140)))); y = list(rng.lognormal(np.log(4e3), 0.9, len(mz))); kind = ["n"] * len(mz)
    for m in rng.uniform(130, 330, 9):                                    # strong components with M+1 / M+2
        a = 10 ** rng.uniform(5.3, 7.2)
        for d, f in ((0, 1), (1, .16), (2, .05)):
            mz.append(m + d); y.append(a * f); kind.append("c" if d == 0 else "i")
    top = max(y)
    for m in rng.uniform(385, 470, targets):                              # the small signal at high m/z: under 5 % of the highest peak, well over the noise
        mz.append(m); y.append(min(top * 0.04, 10 ** rng.uniform(5.2, 5.9))); kind.append("t")
    return mz, y, kind


def run(strategies=("abs", "snr", "bands"), n=400, seed=7, limit=LIMIT, targets=1):
    if shutil.which("node") is None:
        return None
    rng = np.random.default_rng(seed); sc = [scene(rng, targets) for _ in range(n)]
    res = {}
    for st in strategies:
        o = {"lo": 120, "hi": 480, "strategy": st, "bands": 4, "quota": 1}
        pay = [{"mz": s[0], "y": s[1], "o": o} for s in sc]
        out = json.loads(subprocess.run(["node", "-e", RUN, str(JS)], input=json.dumps(pay), capture_output=True, text=True, check=True).stdout)
        rec = fal = cov = 0
        for (mz, y, kind), picks in zip(sc, out):
            lab = [p[0] for p in picks[:limit]]
            rec += sum(kind[i] == "t" for i in lab) / targets; fal += sum(kind[i] == "n" for i in lab)
            cov += len({min(3, int((mz[i] - 120) // 90)) for i in lab}) / 4
        res[st] = {"recall": rec / n, "noise_labels": fal / n, "coverage": cov / n}
    return res


if __name__ == "__main__":
    for t in (1, 3):
        r = run(targets=t)
        if r is None:
            sys.exit("node is not installed")
        print(f"{t} planted peak(s) at 385-470\n{'strategy':8} {'planted peaks labelled':>24} {'noise peaks labelled':>22} {'m/z quarters covered':>22}")
        for k, v in r.items():
            print(f"{k:8} {v['recall']:24.2f} {v['noise_labels']:22.2f} {v['coverage']:22.2f}")
