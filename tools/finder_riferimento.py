"""Reference answers of the parts of mzFinder ported to Rust (`crates/mzlab-feat/src/legacy/`) for their parity test
(crates/mzlab-feat/tests/legacy_parity.rs): `api.formula_info`, the names of `api.classify` (`guess_sample`), the kinetic descriptors, the MS2
consensus and the three components of the MS2 similarity.

    python3 tools/finder_riferimento.py [--golden]

Writes tests/golden/feat/finder.json with --golden, else prints its size. Synthetic inputs only (fixed seed), no compound names.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))

from tpmine import api  # noqa: E402
from tpmine.engine import guess_sample  # noqa: E402
from tpmine.hr import kinetics as KN  # noqa: E402
from tpmine.hr import similarity as SM  # noqa: E402
from tpmine.hr import spectra as SP  # noqa: E402

FORMULAS = [
    ("C10H12N2O3S", "[M+H]+"), ("C6H6O", "[M+H]+"), ("c1ccccc1O", "[M+H]+"), ("CC(=O)Oc1ccccc1C(=O)O", "[M+Na]+"), ("CCN(CC)CC", "[M-H]-"),
    ("O=C(O)C[NH3+]", "[M+H]+"), ("C1CC1", "[M+H]+"), ("c1ccc2ccccc2c1", "[M+H]+"), ("CS(=O)(=O)N", "[M+H]+"), ("ClC(Cl)Cl", "[M+H]+"),
    ("C[C@H](N)C(=O)O", "[M+H]+"), ("[O-][N+](=O)c1ccccc1", "[M+H]+"), ("C#N", "[M+H]+"), ("OC1=CC=CC=C1", "[M+H]+"), ("Brc1ccccc1", "[M+H]+"),
    ("", "[M+H]+"), ("  ", "[M+H]+"), ("C6H6Xx", "[M+H]+"), ("C6(H6", "[M+H]+"), ("C1CC", "[M+H]+"), ("CC)C", "[M+H]+"), ("C[Zz]C", "[M+H]+"),
    ("C6H6O", "[M+K]+"), ("C6H6O", "[M+Q]+"), ("c1ccccc1", "[M+NH4]+"), ("C(C)(C)(C)(C)C", "[M+H]+"), ("CC(C)C(=O)[O-].[Na+]", "[M+H]+"),
]

NAMES = [
    "blank_01.mzML", "BLK-2.mzML", "bianco.mzML", "STD_12ppm.mzML", "cal 3.mzML", "taratura_1.mzML", "dark_t30.mzML", "ctrl-2.mzML", "buio.mzML",
    "S_t0.mzML", "S_t30.mzML", "S_t2,5h.mzML", "S_t1.5ore.mzML", "run_120min.mzML", "run_2h_b.mzML", "x_t30min(2).mzML", "t15.mzML", "plain.mzML",
    "a.b.mzML", "S_T45_rep1.mzML", "S-t 10 min.mzML", "S_30m.mzML", "S_t30m.mzML", "standardize.mzML", "control.mzML", "blank2.mzML", "x_t05min.mzML",
    "S_3,5ore_x.mzML", "dir/S_t60.mzML", "S_t60min",
]


def _list(a):
    return [float(x) for x in np.asarray(a, float)]


def kinetics_cases(rng):
    out = []
    times_sets = [[-1, 0, 5, 10, 20, 30, 60, 120], [-1, -1, 0, 15, 30, 45, 60, 90, 120, 180], [0, 2, 4, 8, 16, 32], [-1, 10, 20, 40]]
    for i in range(60):
        t = list(times_sets[i % len(times_sets)])
        if i % 7 == 3:
            t = [float(x) for x in rng.permutation(t)]
        tt = np.asarray(t, float)
        peak = rng.uniform(5, 150)
        y = rng.uniform(1e4, 1e7) * np.exp(-((tt - peak) / rng.uniform(5, 80)) ** 2) + rng.uniform(0, 1e4, len(t))
        y[tt < 0] = rng.uniform(0, 3e5) if i % 3 else 0.0
        if i % 11 == 5:
            y[:] = 0.0
        if i % 13 == 6:
            y[tt > 0] = y[tt > 0][::-1]
        d = KN.descriptors(t, y)
        ref = float(y[tt <= 0].max()) if (tt <= 0).any() else 0.0
        tr = y[tt > 0]
        out.append({"times": t, "y": _list(y), "out": None if not d.get("ok") else
                    {k: (_list(v) if isinstance(v, np.ndarray) else v) for k, v in d.items()},
                    "ref_max": ref, "absent": bool(KN.not_in_reference(t, tr, ref)) if len(tr) else None})
    return out


def spectrum(rng, prec, n):
    mz = np.sort(rng.uniform(50, prec + 3, n))
    it = rng.uniform(1, 1000, n) ** 2
    return mz, it


def consensus_cases(rng):
    out = []
    for i in range(30):
        base_mz, base_it = spectrum(rng, 300 + 10 * i, 25)
        spectra = []
        for _ in range(int(rng.integers(1, 8))):
            keep = rng.random(len(base_mz)) < 0.7
            m = base_mz[keep] * (1 + rng.normal(0, 2e-6, keep.sum()))
            o = np.argsort(m)
            spectra.append((m[o], (base_it[keep] * rng.uniform(0.5, 1.5, keep.sum()))[o]))
        if i % 9 == 4:
            spectra.append((np.zeros(0), np.zeros(0)))
        c = SP.ms2_consensus([s[0] for s in spectra], [s[1] for s in spectra])
        out.append({"spectra": [[_list(s[0]), _list(s[1])] for s in spectra],
                    "out": {"mz": _list(c["mz"]), "rel": _list(c["rel"]), "n_found": [int(x) for x in c["n_found"]], "n_scans": c["n_scans"]}})
    return out


def similarity_cases(rng):
    out = []
    for i in range(40):
        pa = float(rng.uniform(200, 500))
        delta = float(rng.choice([15.9949, -14.0157, 2.0157, 0.0, 31.9898]))
        pb = pa + delta
        ma, ia = spectrum(rng, pa, int(rng.integers(0, 45)))
        keep = rng.random(len(ma)) < 0.6
        shift = np.where(rng.random(len(ma)) < 0.4, delta, 0.0)
        mb = np.concatenate([ma[keep] + shift[keep] + rng.normal(0, 1e-3, keep.sum()), rng.uniform(50, pb, int(rng.integers(0, 12)))])
        ib = np.concatenate([ia[keep] * rng.uniform(0.3, 2, keep.sum()), rng.uniform(1, 1e5, len(mb) - keep.sum())])
        o = np.argsort(mb)
        mb, ib = mb[o], ib[o]
        sa, sb = SM.prepare(ma, ia, pa), SM.prepare(mb, ib, pb)
        r = SM.pair_components(sa, pa, sb, pb)
        out.append({"a": [_list(ma), _list(ia), pa], "b": [_list(mb), _list(ib), pb], "prep_a": [_list(sa[0]), _list(sa[1])],
                    "ent_a": _list(SM.entropy_weights(np.asarray(sa[1])[None, :])[0]) if len(sa[1]) else [], "out": json.loads(json.dumps(r, default=float))})
    return out


def main(argv):
    rng = np.random.default_rng(20261013)
    ref = {
        "formula_info": [{"text": t, "adduct": a, "out": json.loads(api.formula_info(t, a))} for t, a in FORMULAS],
        "guess": [{"name": n, "out": guess_sample(n)} for n in NAMES],
        "kinetics": kinetics_cases(rng),
        "consensus": consensus_cases(rng),
        "similarity": similarity_cases(rng),
    }
    text = json.dumps(ref, sort_keys=True, allow_nan=True)
    if "--golden" in argv:
        (AQUI.parent / "tests" / "golden" / "feat" / "finder.json").write_text(text + "\n", encoding="utf-8")
    else:
        print(len(text))


if __name__ == "__main__":
    main(sys.argv[1:])
