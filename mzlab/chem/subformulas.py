"""All the sub-formulas of a formula, with the exact m/z of their ions (high resolution only).

A search tool, not an answer: the module lists masses and the program measures what the file shows at each of them; it never says which
sub-formula is a fragment or what the compound is.
"""
from __future__ import annotations

import numpy as np

from ..i18n import UserError
from .elements import MASS, fmt, parse_formula

ELECTRON = 0.00054858
MAX_COMBINATIONS = 300_000


def submasses(formula: str, z: int = 1, min_mz: float = 0.0) -> np.ndarray:
    """Sorted m/z of every sub-formula of the ion `formula` (as `subformulas`, without building the names: for the fragment constraint)."""
    f = parse_formula(formula)
    z = int(z) or 1
    els = sorted(f)
    total = 1
    for e in els:
        total *= f[e] + 1
    if total > MAX_COMBINATIONS:
        raise UserError("err.subxic.many", {"n": total}, f"{total} sub-formulas: write a smaller formula")
    mass = np.zeros(1)
    for e in els:
        mass = (mass[:, None] + MASS[e] * np.arange(f[e] + 1)[None, :]).ravel()
    mz = (mass[1:] - z * ELECTRON) / abs(z)                      # index 0 is the empty formula
    return np.sort(mz[mz >= min_mz])


def subformulas(formula: str, z: int = 1, min_mz: float = 0.0) -> tuple[dict, list[tuple[str, float]]]:
    """(parsed formula, [(sub-formula, ion m/z)]) for every sub-formula (each element from 0 to its count, at least one atom) whose ion m/z is
    at least `min_mz`. The formula is read as the formula of the ION: the mass loses (z > 0) or gains (z < 0) |z| electrons. Sorted by m/z."""
    f = parse_formula(formula)
    z = int(z) or 1
    els = sorted(f, key=lambda e: (e not in ("C", "H"), e))
    total = 1
    for e in els:
        total *= f[e] + 1
    if total > MAX_COMBINATIONS:
        raise UserError("err.subxic.many", {"n": total}, f"{total} sub-formulas: write a smaller formula")
    grids = np.indices([f[e] + 1 for e in els]).reshape(len(els), -1)
    mass = sum(grids[i] * MASS[e] for i, e in enumerate(els))
    mz = (mass - z * ELECTRON) / abs(z)
    keep = (grids.sum(axis=0) > 0) & (mz >= min_mz)
    order = np.argsort(mz[keep], kind="stable")
    cols, mzs = grids[:, keep][:, order], mz[keep][order]
    return f, [(fmt({e: int(cols[i, j]) for i, e in enumerate(els) if cols[i, j]}), float(mzs[j])) for j in range(cols.shape[1])]


def pearson(a: np.ndarray, b: np.ndarray) -> float | None:
    """Correlation of two traces; None when one of them is flat."""
    if len(a) < 3 or a.std() == 0 or b.std() == 0:
        return None
    return float(np.corrcoef(a, b)[0, 1])


def measure(table, f: dict, subs: list[tuple[str, float]], ppm: float, max_rows: int = 300) -> dict:
    """Measures of every sub-formula on a peak table (`PeakTable`: peaks sorted by m/z). Returns the XIC that sums the peaks of all the windows
    (a peak inside two windows counts once) and one row per sub-formula that shows signal: m/z, mean error in ppm (weighted by intensity),
    apex intensity, RT of the apex, correlation r with the trace of the whole formula. No labels."""
    n = len(table.rt)
    mzs = np.array([m for _, m in subs], dtype=float)
    tol = mzs * ppm * 1e-6
    lo, hi = mzs - tol, mzs + tol
    if table.mz.dtype == np.float32:
        lo, hi = lo.astype(np.float32), hi.astype(np.float32)
    a = np.searchsorted(table.mz, lo, side="left")
    b = np.searchsorted(table.mz, hi, side="right")
    hit = np.flatnonzero(b > a)
    whole = fmt(f)
    full_i = next((i for i, (s, _) in enumerate(subs) if s == whole), None)
    full = np.zeros(n)
    if full_i is not None and b[full_i] > a[full_i]:
        full = np.bincount(table.pos[a[full_i]:b[full_i]], weights=table.inten[a[full_i]:b[full_i]], minlength=n)
    used = np.zeros(len(table.mz), dtype=bool)
    rows = []
    for i in hit:
        sl = slice(a[i], b[i])
        used[sl] = True
        y = np.bincount(table.pos[sl], weights=table.inten[sl], minlength=n)
        apex = int(np.argmax(y))
        w = table.inten[sl].astype(float)
        err = float((np.average(table.mz[sl].astype(float), weights=w) - mzs[i]) / mzs[i] * 1e6) if w.sum() > 0 else None
        rows.append({"formula": subs[i][0], "mz": round(float(mzs[i]), 5), "ppm": None if err is None else round(err, 2),
                     "int": round(float(y[apex]), 1), "rt": round(float(table.rt[apex]), 4),
                     "r": None if i == full_i else (None if (r := pearson(y, full)) is None else round(r, 3))})
    idx = np.flatnonzero(used)
    comb = np.bincount(table.pos[idx], weights=table.inten[idx], minlength=n) if len(idx) else np.zeros(n)
    rows.sort(key=lambda r: -r["int"])
    return {"rt": [round(float(v), 4) for v in table.rt], "y": [round(float(v), 1) for v in comb],
            "full": [round(float(v), 1) for v in full], "n_sub": len(subs), "n_hit": len(rows), "rows": rows[:max_rows]}
