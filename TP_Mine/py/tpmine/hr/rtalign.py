# TPMINE-PRIVATE
"""Minimal RT alignment of a series of HR files (WP4c). The anchors are the features that every file shows and that nothing else crowds: strong,
alone in their m/z window and RT neighbourhood, with the apex of the same peak in all files. The apex of an anchor in a file minus its median apex
over the files is the RT shift of that file at that RT; the shift curve of a file is piecewise linear between the medians of consecutive groups of
anchors (MassCube, Nat. Commun. 2025: isolated anchors and interpolation). The curve only serves to find where a feature is expected in a file
(gap filling) and to show the drift: no signal is moved. numpy only."""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .features import MIN_HEIGHT, Alignment

MIN_ANCHORS = 8           # fewer anchors in all files: the ones that miss a file or two are accepted (never fewer than 3 files)
MAX_SPREAD = 0.5          # minutes: an anchor whose apexes differ more than this between files is not one peak
CROWD_PPM, CROWD_RT = 5.0, 0.5


@dataclass
class RtMap:
    """Shift curves: `shift[k, j]` = apex RT in file k minus the reference RT, at the reference RT `grid[j]` (minutes)."""
    grid: np.ndarray
    shift: np.ndarray            # (files, nodes)
    n_anchors: int
    rms: np.ndarray              # (files,) residual of the anchors against the curve, minutes

    @property
    def ok(self) -> bool:
        return self.n_anchors >= 3 and len(self.grid) > 0

    def at(self, k: int, rt) -> np.ndarray:
        """Shift of file k at the reference RT `rt` (0 without a curve)."""
        if not self.ok:
            return np.zeros(np.shape(rt))
        return np.interp(rt, self.grid, self.shift[k])

    def expected(self, ref_rt: float | np.ndarray, k: int):
        """Where a peak whose reference RT is `ref_rt` is expected in file k."""
        return ref_rt + self.at(k, ref_rt)

    def reference(self, rt: np.ndarray, k: int) -> np.ndarray:
        """Reference RT of a peak seen at `rt` in file k (the inverse of `expected`, by one fixed-point step: the shifts change slowly with the RT)."""
        return rt - self.at(k, rt - self.at(k, rt))

    def as_dict(self, labels: list[str]) -> dict:
        r3 = lambda a: [round(float(v), 4) for v in a]            # noqa: E731
        return {"files": list(labels), "rt": r3(self.grid), "shift": [r3(s) for s in self.shift], "anchors": int(self.n_anchors), "rms": r3(self.rms)}


def isolated(al: Alignment, ppm: float = CROWD_PPM, rt: float = CROWD_RT) -> np.ndarray:
    """True for the groups that have no other group within `ppm` and `rt` minutes (an isomer or a neighbour would make the apex ambiguous)."""
    o = np.argsort(al.mz, kind="stable")
    mz, r = al.mz[o], al.rt[o]
    lo = np.searchsorted(mz, mz * (1 - ppm * 1e-6), "left")
    hi = np.searchsorted(mz, mz * (1 + ppm * 1e-6), "right")
    n = hi - lo
    g = np.repeat(np.arange(len(mz)), n)
    j = np.arange(int(n.sum())) - np.repeat(np.cumsum(n) - n, n) + np.repeat(lo, n)
    near = (j != g) & (np.abs(r[j] - r[g]) <= rt)
    crowded = np.zeros(len(mz), bool)
    crowded[g[near]] = True
    out = np.zeros(len(mz), bool)
    out[o] = ~crowded
    return out


def build(al: Alignment, min_height: float = MIN_HEIGHT, min_anchors: int = MIN_ANCHORS) -> RtMap:
    """The shift curves of the files of `al` (see the module docstring). Without enough anchors the map is empty (`ok` is False, all shifts 0)."""
    nf = al.n_files
    empty = RtMap(np.zeros(0), np.zeros((nf, 0)), 0, np.zeros(nf))
    if len(al) == 0 or nf < 2:
        return empty
    A = al.apex_rt.astype(np.float64)
    seen = ~np.isnan(A)
    with np.errstate(all="ignore"):
        strong = np.where(seen, al.height, np.nan)
        ok_h = np.nanmedian(strong, axis=1) >= 10 * min_height
        spread = np.nanmax(A, axis=1) - np.nanmin(A, axis=1)
    base = ok_h & (spread <= MAX_SPREAD) & isolated(al)
    npres = seen.sum(1)
    pick = base & (npres == nf)
    if pick.sum() < min_anchors:                                       # a feature that misses a file or two still anchors the others
        pick = base & (npres >= max(3, int(np.ceil(0.75 * nf))))
    if pick.sum() < 3:
        return empty
    A = A[pick]
    ref = np.nanmedian(A, axis=1)
    d = A - ref[:, None]
    o = np.argsort(ref)
    ref, d = ref[o], d[o]
    m = len(ref)
    size = max(5, m // 12)                                              # anchors per node of the curve
    edges = np.unique(np.r_[np.arange(0, m, size), m])
    if len(edges) > 2 and edges[-1] - edges[-2] < size // 2:            # a short last group joins the one before it
        edges = np.delete(edges, -2)
    grid, shift = [], []
    with np.errstate(all="ignore"):
        for a, b in zip(edges[:-1], edges[1:]):
            grid.append(float(np.median(ref[a:b])))
            shift.append(np.nanmedian(d[a:b], axis=0))
    grid, shift = np.array(grid), np.array(shift).T                     # (files, nodes)
    shift = np.where(np.isnan(shift), 0.0, shift)
    if ref[0] < grid[0]:                                                # the curve spans the whole range of the anchors (flat beyond the first and last group)
        grid, shift = np.r_[ref[0], grid], np.c_[shift[:, 0], shift]
    if ref[-1] > grid[-1]:
        grid, shift = np.r_[grid, ref[-1]], np.c_[shift, shift[:, -1]]
    res = np.zeros(nf)
    for k in range(nf):
        v = ~np.isnan(d[:, k])
        if v.any():
            res[k] = float(np.sqrt(np.mean((d[v, k] - np.interp(ref[v], grid, shift[k])) ** 2)))
    return RtMap(grid, shift, int(m), res)
