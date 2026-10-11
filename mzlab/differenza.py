"""Difference from a reference (blank or t0): sample minus reference, for traces and spectra. Pure numpy.

The program shows only the difference: it never says what the difference is (principle 1). The reference is compared with a tolerance
(a retention time that drifts a little, a mass that falls in the same nominal cell) so that what is present in both cancels instead of
leaving a pair of opposite signs, and what is lower in the sample than in the reference stays NEGATIVE unless the student asks to clip it.
"""
from __future__ import annotations

import numpy as np

NORMS = ("none", "tic", "istd")


def local_max(x: np.ndarray, y: np.ndarray, xr: np.ndarray, yr: np.ndarray, tol: float) -> np.ndarray:
    """Reference at the times x: the highest value of the reference within +-tol of each time (and never lower than its interpolation at x).
    NaN where x lies outside the time range of the reference."""
    x, y = np.asarray(x, dtype=np.float64), np.asarray(y, dtype=np.float64)
    xr, yr = np.asarray(xr, dtype=np.float64), np.asarray(yr, dtype=np.float64)
    if len(xr) == 0:
        return np.full(len(x), np.nan)
    ref = np.interp(x, xr, yr)
    if tol > 0 and len(xr) > 1:
        lo = np.searchsorted(xr, x - tol, side="left")
        hi = np.searchsorted(xr, x + tol, side="right")
        for i in np.flatnonzero(hi > lo):
            m = yr[lo[i]:hi[i]].max()
            if m > ref[i]:
                ref[i] = m
    ref[(x < xr[0]) | (x > xr[-1])] = np.nan
    return ref


def subtract_trace(x, y, xr, yr, factor: float = 1.0, rttol: float = 0.1, clip: bool = False) -> np.ndarray:
    """y minus factor times the reference (xr, yr), on the times x of the sample. The reference is its local maximum within +-rttol minutes,
    interpolated on the sample times; outside the time range of the reference the answer is NaN (no data), not 0. clip: negatives become 0."""
    y = np.asarray(y, dtype=np.float64)
    d = y - float(factor) * local_max(x, y, xr, yr, float(rttol))
    return np.where(d < 0, 0.0, d) if clip else d


def area(x, y) -> float:
    """Trapezoid area of a trace (intensity x minutes)."""
    x, y = np.asarray(x, dtype=np.float64), np.asarray(y, dtype=np.float64)
    return float(np.sum((x[1:] - x[:-1]) * (y[1:] + y[:-1]) / 2)) if len(x) > 1 else 0.0


def norm_factor(kind: str, sample: float, ref: float) -> float:
    """Scale to put the reference on the sample: 1 for no normalization, otherwise sample / reference of the total ion current (tic) or of the
    area of the internal standard (istd). 1 too when one of the two is not positive: there is nothing to scale by."""
    if kind not in NORMS:
        raise ValueError(f"norm must be one of {', '.join(NORMS)}, not {kind!r}")
    if kind == "none" or not (sample > 0 and ref > 0):
        return 1.0
    return float(sample) / float(ref)


def _nominal(mz: np.ndarray, below: float) -> np.ndarray:
    """Nominal mass of each m/z at unit resolution: the cell [n - below, n + 1 - below) (the same window as the XIC, XIC_BELOW in the page)."""
    return np.floor(np.asarray(mz, dtype=np.float64) + below).astype(np.int64)


def subtract_spectrum(mz, y, rmz, ry, factor: float = 1.0, unit: bool = True, tol: float = 5.0, below: float = 0.2, clip: bool = False):
    """Difference of two centroid spectra (mean intensity per scan, so windows of different length compare): (m/z, difference).
    unit: low resolution, the lines are compared on the nominal mass (cell [n - below, n + 1 - below)); otherwise high resolution, a reference
    line counts for a sample line within +-tol ppm. Lines only in the reference give negative values at their own m/z. Only the lines
    that are not zero are returned, sorted by m/z. clip: negatives become 0 and are dropped."""
    mz, y = np.asarray(mz, dtype=np.float64), np.asarray(y, dtype=np.float64)
    rmz, ry = np.asarray(rmz, dtype=np.float64), np.asarray(ry, dtype=np.float64) * float(factor)
    if unit:
        ca, cb = _nominal(mz, below), _nominal(rmz, below)
        cells = np.union1d(ca, cb)
        out_mz, out_y = [], []
        for c in cells:
            ia, ib = np.flatnonzero(ca == c), np.flatnonzero(cb == c)
            a = y[ia].max() if len(ia) else 0.0
            b = ry[ib].max() if len(ib) else 0.0
            at = ia[np.argmax(y[ia])] if len(ia) else None
            bt = ib[np.argmax(ry[ib])] if len(ib) else None
            out_mz.append(mz[at] if (at is not None and (bt is None or a >= b)) else rmz[bt])
            out_y.append(a - b)
        out_mz, out_y = np.array(out_mz), np.array(out_y)
    else:
        # high resolution: each sample line minus the strongest reference line within +-tol ppm, then the reference lines no sample line covers
        o = np.argsort(rmz)
        rmz, ry = rmz[o], ry[o]
        w = mz * tol * 1e-6
        lo, hi = np.searchsorted(rmz, mz - w, side="left"), np.searchsorted(rmz, mz + w, side="right")
        ref_at = np.array([ry[l:h].max() if h > l else 0.0 for l, h in zip(lo, hi)])
        covered = np.zeros(len(rmz), dtype=bool)
        for l, h in zip(lo, hi):
            covered[l:h] = True
        only = ~covered
        out_mz = np.concatenate([mz, rmz[only]])
        out_y = np.concatenate([y - ref_at, -ry[only]])
    keep = out_y != 0
    if clip:
        keep &= out_y > 0
    out_mz, out_y = out_mz[keep], out_y[keep]
    o = np.argsort(out_mz)
    return out_mz[o], out_y[o]
