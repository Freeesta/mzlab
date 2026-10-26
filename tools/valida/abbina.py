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


FRAZIONE_FWHM = 0.35           # a broad band accepts a row whose RT is within this share of its FWHM from the apex ...
IOU_MIN = 0.40                 # ... or whose [rt_min, rt_max] overlaps the true interval at least this much (intersection over union)


def iou(a0, a1, b0, b1):
    """Intersection over union of the intervals [a0, a1] and [b0, b1] (arrays, NaN where an interval is unknown -> 0)."""
    inter = np.maximum(0.0, np.minimum(a1, b1) - np.maximum(a0, b0))
    union = np.maximum(a1, b1) - np.minimum(a0, b0)
    with np.errstate(invalid="ignore", divide="ignore"):
        r = np.where(union > 0, inter / union, 0.0)
    return np.nan_to_num(r)


def finestra_larghi(mz_a, rt_a, fwhm_a, larga_a, rt_min_a, rt_max_a, mz_b, rt_b, rt_min_b, rt_max_b, ppm: float = 5.0, rt_tol: float = 0.15):
    """(accepted, tol): matrices [a x b] of the pairs within +-ppm and in RT, and the RT tolerance used for each pair. A narrow truth (`larga_a` False) accepts
    |dRT| <= rt_tol, exactly as `abbina`. A broad one accepts |dRT| <= max(rt_tol, 0.35 * FWHM) or an interval overlap (IoU) >= 0.40; `fwhm_a` is in minutes."""
    mz_a, rt_a, fwhm_a, rt_min_a, rt_max_a, mz_b, rt_b, rt_min_b, rt_max_b = (np.asarray(x, float) for x in (mz_a, rt_a, fwhm_a, rt_min_a, rt_max_a, mz_b, rt_b, rt_min_b, rt_max_b))
    larga_a = np.asarray(larga_a, bool)
    tol = np.where(larga_a, np.maximum(rt_tol, FRAZIONE_FWHM * fwhm_a), rt_tol)[:, None] * np.ones((1, len(mz_b)))
    ok = np.abs(mz_b[None, :] - mz_a[:, None]) <= mz_a[:, None] * ppm * 1e-6
    ok &= np.abs(rt_b[None, :] - rt_a[:, None]) <= tol
    if larga_a.any():
        sov = iou(rt_min_a[:, None], rt_max_a[:, None], rt_min_b[None, :], rt_max_b[None, :]) >= IOU_MIN
        ok_mz = np.abs(mz_b[None, :] - mz_a[:, None]) <= mz_a[:, None] * ppm * 1e-6
        ok = ok | (ok_mz & sov & larga_a[:, None])
    return ok, tol


def abbina_larghi(mz_a, rt_a, fwhm_a, larga_a, rt_min_a, rt_max_a, mz_b, rt_b, rt_min_b, rt_max_b, ppm: float = 5.0, rt_tol: float = 0.15) -> tuple[np.ndarray, np.ndarray]:
    """One-to-one matching of the true compounds a (some of them broad bands) to the rows b, greedy from the smallest normalised distance as `abbina` (the
    RT part is divided by the tolerance of the pair; ties: lower index of a, then of b). With no broad truth it returns what `abbina` returns."""
    mz_a, rt_a, mz_b, rt_b = (np.asarray(x, float) for x in (mz_a, rt_a, mz_b, rt_b))
    if len(mz_a) == 0 or len(mz_b) == 0:
        return np.zeros(0, int), np.zeros(0, int)
    ok, tol = finestra_larghi(mz_a, rt_a, fwhm_a, larga_a, rt_min_a, rt_max_a, mz_b, rt_b, rt_min_b, rt_max_b, ppm, rt_tol)
    ia, ib = np.nonzero(ok)
    d = np.hypot((mz_b[ib] - mz_a[ia]) / mz_a[ia] * 1e6 / ppm, (rt_b[ib] - rt_a[ia]) / tol[ia, ib])
    used_a = np.zeros(len(mz_a), bool)
    used_b = np.zeros(len(mz_b), bool)
    out_a, out_b = [], []
    for x, y in zip(ia[np.lexsort((ib, ia, d))].tolist(), ib[np.lexsort((ib, ia, d))].tolist()):
        if used_a[x] or used_b[y]:
            continue
        used_a[x] = used_b[y] = True
        out_a.append(x)
        out_b.append(y)
    oa = np.argsort(out_a, kind="stable")
    return np.array(out_a, int)[oa], np.array(out_b, int)[oa]
