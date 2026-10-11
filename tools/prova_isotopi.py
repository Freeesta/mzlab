"""Measures how often the true formula ranks first with the full isotope score (and the fragment constraint) versus the old M+1/M+2 check.

Synthetic spectra with noise, fixed seed, no compound names: random formulas of CHNOS (+Cl/Br in the halogen series), the observed isotope peaks
come from the theoretical fine structure fused at the resolving power, with a mass error, a relative intensity error, dropout of weak peaks and
noise peaks; the MS2 is made of random sub-formulas of the precursor. `python3 tools/prova_isotopi.py [N]` prints the table written in
docs/agenti/alta-risoluzione.md.
"""
from __future__ import annotations

import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.chem.composition import compose, format_formula, ion_mass, isotope_ratios, parse_elements          # noqa: E402
from mzlab.chem.isoscore import fine_pattern, merge_res                                                       # noqa: E402
from mzlab.chem.subformulas import subformulas                                                                # noqa: E402

PLAIN = "C:3-40,H:0-80,N:0-8,O:0-10,S:0-3,P:0-2"
HALO = PLAIN + ",Cl:0-3,Br:0-2"


def draw_formula(rng, halo):
    """A valid [M+H]+ ion formula: the neutral has an integer RDB >= 0 (so the ion passes the Lewis and Senior rules)."""
    while True:
        c = {"C": rng.randint(10, 30), "N": rng.randint(0, 5), "O": rng.randint(0, 6), "S": rng.choice([0, 0, 1, 2])}
        x = 0
        if halo:
            c["Cl"] = rng.choice([0, 1, 1, 2, 3]); c["Br"] = rng.choice([0, 0, 1, 2]); x = c["Cl"] + c["Br"]
        hn = 2 * c["C"] + 2 + c["N"] - x - 2 * rng.randint(1, 9)
        if hn < 1:
            continue
        c["H"] = hn + 1
        m = ion_mass(c, 1)
        if 250 <= m <= 600:
            return {k: v for k, v in c.items() if v}


def observe(c, rng, r200, ppm_err=1.5, int_err=0.08):
    pat = merge_res(fine_pattern(c, 1), r200)
    top = 1e6
    obs = []
    for p in pat:
        if p["rel"] < 0.5:
            continue
        i = p["rel"] / 100 * top * (1 + rng.gauss(0, int_err))
        if p["rel"] < 5 and rng.random() < 0.3:
            continue
        obs.append((p["mz"] * (1 + rng.gauss(0, ppm_err) * 1e-6), max(i, 1.0)))
    mono = pat[0]["mz"]
    for _ in range(rng.randint(0, 3)):
        obs.append((mono + rng.uniform(-0.4, 4.5), top * rng.uniform(0.002, 0.012)))
    return obs


def ms2_of(c, rng):
    _, subs = subformulas(format_formula(c), 1, 60.0)
    pk = [(m, rng.uniform(0.1, 1.0) * 1e5) for _, m in rng.sample(subs, min(len(subs), 6))]
    pk = [(m * (1 + rng.gauss(0, 1.5) * 1e-6), i) for m, i in pk]
    pk += [(rng.uniform(60, 200), rng.uniform(0.02, 0.1) * 1e5) for _ in range(5)]
    return pk


def rank_of(res, truth):
    for i, r in enumerate(res["results"]):
        if r["formula"] == truth:
            return i + 1
    return 99


def run(n=150, seed=7, r200=70000.0, tol=8.0):
    rng = random.Random(seed)
    table = {}
    for halo in (False, True):
        hits = {k: [0, 0] for k in ("old", "iso", "iso+frag")}
        used = 0
        for _ in range(n):
            c = draw_formula(rng, halo)
            truth = format_formula(c)
            mz = ion_mass(c, 1) * (1 + rng.gauss(0, 3.0) * 1e-6)
            obs = observe(c, rng, r200)
            mono = min(obs, key=lambda x: abs(x[0] - mz))
            m1 = sum(i for m, i in obs if 0.9 < m - mono[0] < 1.1) / mono[1]
            m2 = sum(i for m, i in obs if 1.9 < m - mono[0] < 2.1) / mono[1]
            els = parse_elements(HALO if halo else PLAIN)
            kw = dict(elements=els, ion="[M+H]+", tol=tol, nitrogen="none", rules=["LEWIS", "SENIOR"], max_results=200)
            old = compose(mz, **kw, m1=m1, m2=m2 or None)
            old["results"].sort(key=lambda r: (not r["iso"].get("ok", True), abs(r["delta_ppm"])))   # the old check: ok first, then the mass error
            new = compose(mz, **kw, obs=obs, r200=r200)
            fr = compose(mz, **kw, obs=obs, frags=ms2_of(c, rng), r200=r200)
            if not old["results"]:
                continue
            used += 1
            for key, res in (("old", old), ("iso", new), ("iso+frag", fr)):
                k = rank_of(res, truth)
                hits[key][0] += k == 1; hits[key][1] += k <= 3
        table["halogen" if halo else "CHNOS"] = {"n": used, **{k: (v[0] / used, v[1] / used) for k, v in hits.items()}}
    return table


if __name__ == "__main__":
    t = run(int(sys.argv[1]) if len(sys.argv) > 1 else 150)
    for series, row in t.items():
        print(f"{series} (n={row['n']}): " + "; ".join(f"{k} top1 {row[k][0]:.0%} top3 {row[k][1]:.0%}" for k in ("old", "iso", "iso+frag")))
