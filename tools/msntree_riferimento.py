"""Reference answers of mzlab/chem/msntree.py for crates/mzlab-core/tests/msntree_parity.rs.

    python3 tools/msntree_riferimento.py OUT.json

A synthetic cascade of fragments of a generic formula (no compound names), with a fake run (scans + spectra): [{scans, spectra, formula, ppm, ok | err}].
"""
import json
import sys
from pathlib import Path
from types import SimpleNamespace as NS

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.chem.elements import ELECTRON, MASS, fmt, parse_formula  # noqa: E402
from mzlab.chem.msntree import msn_tree  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402


def ion_mz(f: dict) -> float:
    return sum(MASS[e] * n for e, n in f.items()) - ELECTRON


def lose(f: dict, g: dict) -> dict:
    return {e: f.get(e, 0) - g.get(e, 0) for e in set(f) | set(g) if f.get(e, 0) - g.get(e, 0) > 0}


LOSSES = [{"H": 2, "O": 1}, {"C": 1, "O": 1}, {"C": 1, "H": 2, "O": 1}, {"N": 1, "H": 3}, {"C": 2, "H": 4}, {"C": 1, "H": 4, "O": 1}]


def build(root: str, depth: int, seed: int, pol: int = 1, noise: bool = True, ties: bool = False):
    rng = np.random.default_rng(seed)
    scans, spectra = [], {}
    idx = [0]

    def add(level, path, peaks, n):
        for _ in range(n):
            mz = np.array([m + (rng.normal() * m * 1e-6 if noise else 0) for m, _ in peaks])
            it = np.array([i * (rng.uniform(0.7, 1.3) if noise else 1.0) for _, i in peaks])
            if noise:
                extra = rng.uniform(50, 400, 6)
                mz = np.r_[mz, extra]
                it = np.r_[it, rng.uniform(0.5, 2, 6)]
            o = np.argsort(mz)
            spectra[idx[0]] = (mz[o].tolist(), it[o].tolist())
            scans.append(NS(index=idx[0], level=level, polarity=pol, path=path))
            idx[0] += 1

    f0 = parse_formula(root)
    add(1, [], [(ion_mz(f0), 1000.0), (ion_mz(f0) + 1.00335, 120.0)], 6)

    def grow(f, path, level):
        frags = []
        for j, g in enumerate(LOSSES[: 3 + (level % 3)]):
            sub = lose(f, g)
            if len(sub) and all(sub.get(e, 0) >= 0 for e in sub) and sub != f:
                frags.append((sub, 900.0 / (j + 1)))
        peaks = [(ion_mz(s), i) for s, i in frags] + [(ion_mz(f) + 3.0 if False else ion_mz(f) + 20.0, 15.0)]
        pm = round(ion_mz(f), 2)
        p = path + [(pm, "cid", 35.0)]
        add(level, p, peaks, 8 if level < 4 else 5)
        if level < depth:
            for s, _ in frags[:2]:
                grow(s, p, level + 1)

    grow(f0, [], 2)
    return scans, spectra


def run_case(root, depth, seed, formula, ppm=5.0, pol=1, noise=True, only_ms1=False):
    scans, spectra = build(root, depth, seed, pol, noise)
    if only_ms1:
        scans = [s for s in scans if s.level == 1]
    run = NS(scans=scans, read=lambda i: (np.array(spectra[i][0]), np.array(spectra[i][1])))
    case = {"scans": [{"index": s.index, "level": s.level, "polarity": s.polarity, "path": [list(p) for p in s.path]} for s in scans],
            "spectra": {str(i): v for i, v in spectra.items()}, "formula": formula, "ppm": ppm}
    try:
        case["ok"] = json.loads(json.dumps(msn_tree(run, formula, ppm)))
    except UserError as e:
        case["err"] = e.key
        case["params"] = {k: str(v) for k, v in e.params.items()}
    return case


def main() -> None:
    root = "C10H21N2O4"
    out = [run_case(root, 4, 1, root), run_case(root, 4, 2, None), run_case(root, 3, 3, root, 3.0), run_case(root, 3, 4, None, 5.0, -1), run_case(root, 2, 5, root, 5.0, 1, False),
           run_case(root, 4, 6, "C9H19N2O3"), run_case(root, 2, 7, "xx1"), run_case(root, 2, 8, "C2Xz"), run_case(root, 3, 9, root, 5.0, 1, True, True)]
    # a cascade that stops at MS2: needs at least two stages
    scans, spectra = build(root, 2, 10)
    scans = [s for s in scans if s.level <= 2 and len(s.path) <= 1]
    run = NS(scans=scans, read=lambda i: (np.array(spectra[i][0]), np.array(spectra[i][1])))
    case = {"scans": [{"index": s.index, "level": s.level, "polarity": s.polarity, "path": [list(p) for p in s.path]} for s in scans], "spectra": {str(i): v for i, v in spectra.items()}, "formula": root, "ppm": 5.0}
    try:
        msn_tree(run, root, 5.0)
    except UserError as e:
        case["err"] = e.key
        case["params"] = {}
    out.append(case)
    Path(sys.argv[1]).write_text(json.dumps(out), encoding="utf-8")
    print(len(out))


if __name__ == "__main__":
    main()
