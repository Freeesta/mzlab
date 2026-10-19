#!/usr/bin/env python3
"""Golden data for the hydrogen-rearrangement attribution of the MS2 engine (SOLO test reference; the engine is Rust).

Built on Attributio's own sub-formula table (`review/explain.py`): a peak is assigned the closest sub-formula of the ion; when none fits, the peak is tried
again shifted by k hydrogen atoms (k = +1, -1, +2, -2, the rearrangements MS-FINDER allows) and the fragment formula takes k more hydrogens.
Writes tests/data/attributio/ms2_engine/rearrange.json.
Use: ATTRIBUTIO_SRC=../attributio/src python3 tools/rearrange_riferimento.py   (needs numpy)
"""
import json
import os
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, os.environ.get("ATTRIBUTIO_SRC", str(Path.home() / "HRMS_Lab/_sistema/attributio/src")))
from attributio.review import explain as X  # noqa: E402
from attributio.review.settings import ReviewSettings  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "tests/data/attributio/ms2_engine/rearrange.json"
SHIFTS = [0, 1, -1, 2, -2]
PENALTY = 0.25


def attribute(fd, mz, pol, em, ei, S):
    am = X.match_adduct(fd, mz, pol)
    if not am or abs(am[2]) > S.ppm_ion:
        return None
    table = X.subformula_table(am[1], pol, am[0], S.max_subformulas)
    els, counts, masses = table
    H = X.ATOM["H"]
    hits = []
    for m in em:
        hit = None
        for k in SHIFTS:
            h = X.assign([m - k * H], table, S.ppm_frag)[0]
            if not h:
                continue
            cnt = dict(zip(els, counts[h[0]].tolist()))
            cnt["H"] = cnt.get("H", 0) + k
            if cnt["H"] < 0:
                continue
            mz_th = float(masses[h[0]]) + k * H
            ppm = (m - mz_th) / mz_th * 1e6
            if abs(ppm) > S.ppm_frag:
                continue
            loss = {e: am[1].get(e, 0) - cnt.get(e, 0) for e in set(am[1]) | set(cnt)}
            hit = dict(k=k, formula=X.fmt_formula({e: v for e, v in cnt.items() if v}), mz_th=mz_th, ppm=ppm, loss=X.fmt_delta(loss) or "-")
            break
        hits.append(hit)
    it = np.asarray(ei, float)
    tot = it.sum()
    score = sum(it[i] * (1 - PENALTY * abs(h["k"])) for i, h in enumerate(hits) if h) / tot
    return dict(adduct=am[0], ion_formula=X.fmt_formula(am[1]), hits=hits,
                n_explained=sum(1 for h in hits if h), n_rearranged=sum(1 for h in hits if h and h["k"]),
                frac_explained=float(sum(it[i] for i, h in enumerate(hits) if h) / tot), score=float(score))


def main():
    S = ReviewSettings()
    H = X.ATOM["H"]
    losses = [18.010565, 27.994915, 17.026549, 46.005479, 44.026215, 28.0313, 56.0626, 31.98983, 79.9568, 14.01565]
    cases = [("C10H12N2O", "POS"), ("C17H19NO3", "POS"), ("C8H10N4O2", "POS"), ("C15H15ClN2O", "POS"), ("C9H8O4", "NEG"),
             ("C12H10N2O3S", "POS"), ("C6H5NO3", "NEG"), ("C21H23ClN2O3", "POS"), ("C16H16Br2O2", "NEG"), ("C9H13N5O3", "POS")]
    out = []
    for k, (f, pol) in enumerate(cases):
        fd = X.parse_formula(f)
        ion = X.ion_mass(fd, pol)
        mz = ion * (1 + (k % 3 - 1) * 1.5e-6)
        raw_mz, raw_it = [], []
        for j, loss in enumerate(losses):
            m = ion - loss + (SHIFTS[(j + k) % 5] * H if j % 2 else 0.0)
            if m > 40:
                raw_mz.append(m * (1 + (j % 4 - 1.5) * 2e-6))
                raw_it.append(1000.0 / (j + 1) + 7 * k)
        raw_mz += [ion, 61.0123 + k * 0.01, 90.0002 + k * 3.1]
        raw_it += [800.0, 40.0, 30.0]
        order = np.argsort(raw_mz)
        raw_mz = [round(float(raw_mz[i]), 6) for i in order]
        raw_it = [float(raw_it[i]) for i in order]
        em, ei, _ = X.prep_ms2(raw_mz, raw_it, mz)
        r = attribute(fd, mz, pol, [float(v) for v in em], [float(v) for v in ei], S)
        out.append(dict(formula=f, pol=pol, mz=mz, mz_peaks=[float(v) for v in em], it=[float(v) for v in ei], result=r))
    OUT.write_text(json.dumps({"penalty": PENALTY, "shifts": SHIFTS, "cases": out}))
    print(sum(1 for c in out if c["result"]), "cases;", sum(c["result"]["n_rearranged"] for c in out if c["result"]), "rearranged peaks")


if __name__ == "__main__":
    main()
