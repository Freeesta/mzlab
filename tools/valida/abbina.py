"""Deterministic one-to-one matching of two lists of (m/z, RT) points: every pair within +-ppm and +-RT is a candidate, the pairs are taken from the
smallest normalised distance up (ties: lower index of the first list, then of the second), each point is used at most once."""
from __future__ import annotations

import numpy as np


def abbina(mz_a, rt_a, mz_b, rt_b, ppm: float = 5.0, rt_tol: float = 0.15) -> tuple[np.ndarray, np.ndarray]:
    """(ia, ib): indices of the matched points, sorted by ia."""
    mz_a, rt_a, mz_b, rt_b = (np.asarray(x, float) for x in (mz_a, rt_a, mz_b, rt_b))
    if len(mz_a) == 0 or len(mz_b) == 0:
        return np.zeros(0, int), np.zeros(0, int)
    o = np.argsort(mz_b, kind="stable")
    sb = mz_b[o]
    lo = np.searchsorted(sb, mz_a * (1 - ppm * 1e-6), "left")
    hi = np.searchsorted(sb, mz_a * (1 + ppm * 1e-6), "right")
    n = hi - lo
    ia = np.repeat(np.arange(len(mz_a)), n)
    ib = o[np.arange(int(n.sum())) - np.repeat(np.cumsum(n) - n, n) + np.repeat(lo, n)]
    ok = np.abs(rt_b[ib] - rt_a[ia]) <= rt_tol
    ia, ib = ia[ok], ib[ok]
    d = np.hypot((mz_b[ib] - mz_a[ia]) / mz_a[ia] * 1e6 / ppm, (rt_b[ib] - rt_a[ia]) / rt_tol)
    order = np.lexsort((ib, ia, d))
    used_a = np.zeros(len(mz_a), bool)
    used_b = np.zeros(len(mz_b), bool)
    out_a, out_b = [], []
    for x, y in zip(ia[order].tolist(), ib[order].tolist()):
        if used_a[x] or used_b[y]:
            continue
        used_a[x] = used_b[y] = True
        out_a.append(x)
        out_b.append(y)
    oa = np.argsort(out_a, kind="stable")
    return np.array(out_a, int)[oa], np.array(out_b, int)[oa]
