"""Peaks of a mass spectrum with unit resolution: the same rule for profile files and for centroid files.

One nominal mass = one peak. The unit cell of nominal mass n is [n - XIC_BELOW, n + 1 - XIC_BELOW]: the cells tile the axis, so it is
wider than the default XIC window [n - 0.2, n + 0.3] of web/pannelli.js `xicWin` (tests/test_peaks.py checks the shared lower edge).
"""
from __future__ import annotations

import numpy as np

XIC_BELOW = 0.2         # unit cell of nominal mass n = [n - 0.2, n + 0.8]
_SG5 = np.array([-3.0, 12.0, 17.0, 12.0, -3.0]) / 35.0      # Savitzky-Golay, 5 points, quadratic: a short smoothing that keeps the width of the peak


def nominal(mz):
    """Nominal mass of the unit window that holds m/z."""
    return np.floor(np.asarray(mz, dtype=float) + XIC_BELOW).astype(np.int64)


def profile_peaks(mz, y, min_rel: float = 0.002):
    """Peaks ('cime') of a profile spectrum: (m/z, height), one per nominal mass.

    The profile is smoothed a little; each unit window gives at most one peak: its highest real top (a point that is only the tail of the
    neighbouring peak is not a top). m/z = centroid (weighted by intensity) of the points above half height around the maximum,
    height = height of the smoothed maximum. Peaks lower than min_rel x the highest one are noise and are dropped.
    """
    mz = np.asarray(mz, dtype=float)
    y = np.asarray(y, dtype=float)
    n = len(mz)
    if n == 0:
        return np.zeros(0), np.zeros(0)
    if n < 5:
        keep = y > 0
        return mz[keep], y[keep]
    s = np.maximum(np.convolve(y, _SG5, mode="same"), 0.0)
    nom = nominal(mz)
    starts = np.flatnonzero(np.r_[True, nom[1:] != nom[:-1]])
    g = np.cumsum(np.r_[False, nom[1:] != nom[:-1]])
    left = np.r_[-np.inf, s[:-1]]
    right = np.r_[s[1:], -np.inf]
    top = np.where((s >= left) & (s > right) & (s > 0), s, 0.0)      # only real tops count: the tail of a neighbouring peak is not a peak
    gmax = np.maximum.reduceat(top, starts)
    cand = np.flatnonzero((top == gmax[g]) & (top > 0))
    _, first = np.unique(g[cand], return_index=True)
    j = cand[first]                                  # the highest top of each window
    gj = g[j]
    ok = s[j] >= min_rel * max(float(s.max()), 1e-12)
    j, gj = j[ok], gj[ok]
    if len(j) == 0:
        return np.zeros(0), np.zeros(0)
    half = np.full(len(starts), np.inf)
    half[gj] = 0.5 * s[j]
    m = s >= half[g]
    rid = np.cumsum(np.r_[True, m[1:] != m[:-1]])
    rj = np.full(len(starts), -1)
    rj[gj] = rid[j]
    keep = m & (rid == rj[g])                         # the points above half height that are joined to the maximum
    w = np.where(keep, s, 0.0)
    sw = np.bincount(g, weights=w, minlength=len(starts))[gj]
    sm = np.bincount(g, weights=w * mz, minlength=len(starts))[gj]
    return sm / np.maximum(sw, 1e-12), s[j]


def merge_unit(mz, y):
    """Centroids of the same unit window become one peak (intensity = sum, m/z = weighted mean), like a unit-resolution instrument sees them."""
    mz = np.asarray(mz, dtype=float)
    y = np.asarray(y, dtype=float)
    if len(mz) < 2:
        return mz, y
    order = np.argsort(mz, kind="stable")
    mz, y = mz[order], y[order]
    nom = nominal(mz)
    starts = np.flatnonzero(np.r_[True, nom[1:] != nom[:-1]])
    sy = np.add.reduceat(y, starts)
    smz = np.add.reduceat(mz * y, starts) / np.maximum(sy, 1e-12)
    return smz, sy


def pad_zeros(mz, y, step: float):
    """Profile points with the zeros that the converter left out put back next to each gap, so that a line drawn through them goes down to zero;
    a single point gets one on each side."""
    mz = np.asarray(mz, dtype=float)
    y = np.asarray(y, dtype=float)
    if len(mz) == 1 and step and step > 0:
        return np.array([mz[0] - step, mz[0], mz[0] + step]), np.array([0.0, y[0], 0.0])
    if len(mz) < 2 or not step:
        return mz, y
    gap = np.flatnonzero(np.diff(mz) > 1.5 * step)
    if len(gap) == 0:
        return mz, y
    zm = np.r_[mz[gap] + step, mz[gap + 1] - step]
    out_m = np.r_[mz, zm]
    out_y = np.r_[y, np.zeros(len(zm))]
    o = np.argsort(out_m, kind="stable")
    return out_m[o], out_y[o]
