# TPMINE-PRIVATE
"""mzFinder: the RT x m/z map shared by the low-resolution engine (`discover`). Only numpy.

- build_grid    the map grid of a PeakTable (rows = RT bins, columns = m/z bins);
- find_points   local maxima of A - B above thresholds (S/N in A, ratio A/B, minimum width)."""
from __future__ import annotations

import math

import numpy as np

SOGLIE = {"min_sn": 5.0, "min_ratio": 3.0, "min_width": 0.03, "max_points": 60}


def soglie(user: dict | None = None) -> dict:
    out = dict(SOGLIE)
    for k, v in (user or {}).items():
        if k in out and v is not None and math.isfinite(float(v)):
            out[k] = float(v)
    return out


def build_grid(t, rt0: float, drt: float, nrt: int, mz0: float, dmz: float, nmz: int, win: int = 0, block: int = 128) -> np.ndarray:
    """The map grid of a PeakTable (same layout as the page's: rows = RT bins, columns = m/z bins) as float32: the intensity summed in each cell, averaged
    over the scans that fall in the RT bin; win > 0 smooths the columns with a triangular kernel of radius ~win bins (two boxes of win // 2) (an XIC-like window whose maximum sits
    on the centroid of an ion, not on a plateau). A map of more than 6 million cells is filled `block` rows at a time."""
    out = np.zeros((nrt, nmz), np.float32)
    if not len(t.rt) or not len(t.mz):
        return out
    sbin = np.floor((np.asarray(t.rt, float) - rt0) / drt).astype(np.int64)         # RT bin of every scan (-1: outside the map)
    sbin[(sbin < 0) | (sbin >= nrt)] = -1
    scans = np.bincount(sbin[sbin >= 0], minlength=nrt).astype(np.float64)
    rb = sbin[t.pos]
    jb = np.clip(((np.asarray(t.mz) - mz0) * (1.0 / dmz)).astype(np.int64), 0, nmz - 1)
    w = np.asarray(t.inten)
    if (rb < 0).any():
        keep = rb >= 0
        rb, jb, w = rb[keep], jb[keep], w[keep]
    small = nrt * nmz <= 6_000_000
    if not small:
        o = np.argsort(rb, kind="stable")
        rb, jb, w = rb[o], jb[o], w[o]
    for r0 in range(0, nrt, nrt if small else block):
        r1 = min(r0 + (nrt if small else block), nrt)
        if small:
            idx, ww = rb * nmz + jb, w
        else:
            a, b = np.searchsorted(rb, r0), np.searchsorted(rb, r1)
            if b <= a:
                continue
            idx, ww = (rb[a:b] - r0) * nmz + jb[a:b], w[a:b]
        g = np.bincount(idx, weights=ww, minlength=(r1 - r0) * nmz).reshape(r1 - r0, nmz)
        g = (g / np.maximum(scans[r0:r1], 1.0)[:, None]).astype(np.float32)
        for _ in range(2 if win > 0 else 0):          # two boxes of radius win // 2 = one triangle of radius ~win
            h, acc = max(win // 2, 1), g.copy()
            for d in range(1, h + 1):
                acc[:, d:] += g[:, :-d]
                acc[:, :-d] += g[:, d:]
            g = acc
        out[r0:r1] = g
    return out


# ---------------------------------------------------------------------------------------------------------------- 1. points
def find_points(diff: np.ndarray, a: np.ndarray, rt0: float, rt1: float, mz0: float, dmz: float, th: dict | None = None) -> list[dict]:
    """Local maxima of the difference (3 x 3 neighbourhood, strictly above its neighbours on the right/below to give one point per plateau).
    diff, a: arrays (nrt, nmz) = A - B and A. S/N of A: (A - median) / (1.4826 x MAD) over the cells that hold signal; ratio = A / (A - diff);
    width = minutes where the difference stays above half of the maximum along RT."""
    th = soglie(th)
    nrt, nmz = diff.shape
    if nrt < 3 or nmz < 3:
        return []
    drt = (rt1 - rt0) / nrt
    pos = a[a > 0]
    med = float(np.median(pos)) if pos.size else 0.0
    mad = float(np.median(np.abs(pos - med))) * 1.4826 if pos.size else 0.0
    p = np.pad(diff, 1, constant_values=-np.inf)
    cen = p[1:-1, 1:-1]
    ok = cen > 0
    for di in (-1, 0, 1):
        for dj in (-1, 0, 1):
            if di == dj == 0:
                continue
            nb = p[1 + di:1 + di + nrt, 1 + dj:1 + dj + nmz]
            ok &= (cen > nb) if (di, dj) > (0, 0) else (cen >= nb)
    ii, jj = np.nonzero(ok)
    ia_v, d_v = a[ii, jj], diff[ii, jj]
    b_v = ia_v - d_v
    with np.errstate(divide="ignore", invalid="ignore"):
        keep = np.where(b_v > 0, ia_v / b_v, np.inf) >= th["min_ratio"]
        if mad > 0:
            keep &= (ia_v - med) / mad >= th["min_sn"]
    out = []
    for i, j in zip(ii[keep], jj[keep]):
        ia, d = float(a[i, j]), float(diff[i, j])
        b = ia - d
        ratio = ia / b if b > 0 else math.inf
        sn = (ia - med) / mad if mad > 0 else None
        lo = hi = i
        while lo > 0 and diff[lo - 1, j] >= d / 2:
            lo -= 1
        while hi < nrt - 1 and diff[hi + 1, j] >= d / 2:
            hi += 1
        width = (hi - lo + 1) * drt
        if width < th["min_width"]:
            continue
        out.append({"rt": rt0 + (i + 0.5) * drt, "mz": mz0 + (j + 0.5) * dmz, "diff": d, "ia": ia, "ratio": None if math.isinf(ratio) else ratio, "sn": sn, "width": width})
    out.sort(key=lambda r: -r["diff"])
    return out[: int(th["max_points"])]
