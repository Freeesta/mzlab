# TPMINE-PRIVATE
"""High-resolution feature detection (WP4): ion traces in the MS1 peak table of ONE file, found without any Python loop over peaks, channels or
scans, then aligned across files. numpy only.

Detection: log-ppm channels on two shifted grids; (channel, scan) cells; a segment is a run of scans in one channel with gaps <= `max_gap`.
Segments of neighbouring channels that cover the same scans are stitched (an intense ion wobbles by a few ppm, crosses the channel border and
would be cut into several pieces). The two grids are merged, Fourier-transform side lobes of very intense ions are marked, and the result is
a small table per file: the peak table can be dropped before the next file (memory: one file at a time).

Stitching trade-off (measured on 13 real files): pieces that overlap by half are always joined; pieces that merely touch only when both are
intense (TOUCH_HEIGHT) and alike. Joining touching pieces of weak ions, too, merges unrelated background ions (1.3 M features became 0.45 M
and two products of interest disappeared), so a weak ion that wobbles across a channel border may still be split in a few features."""
from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

CHANNEL_PPM = 4e-6        # 4 ppm, not 10: at 10 ppm a product at 291.1485 merges with a background ion at 291.1446 (13 ppm) and disappears
FLOOR = 2e4               # intensity floor of the Orbitrap noise of the measured files
MIN_HEIGHT = 2e5
MIN_POINTS = 5
LONG_TRACE = 400          # scans: longer traces are continuous background (kept, marked)
MAX_GAP = 2               # missing scans tolerated inside a trace
SLAB_PEAKS = 1_500_000    # peaks per slab of the detection (memory)
TOUCH_HEIGHT = 2e6        # only traces this intense are stitched when their pieces merely touch


@dataclass
class Features:
    """Ion traces of one file. Arrays have one row per feature; areas are counts x seconds."""
    mz: np.ndarray
    rt: np.ndarray                 # minutes, apex
    apex: np.ndarray               # position of the apex scan in the MS1 table
    area: np.ndarray
    height: np.ndarray
    npts: np.ndarray
    left: np.ndarray               # rt of the first and last scan of the trace (minutes)
    right: np.ndarray
    background: np.ndarray         # bool: longer than LONG_TRACE scans
    artefact: np.ndarray           # bool: Fourier side lobe of a much more intense ion
    scan_ids: np.ndarray = field(default=None)       # file index of every MS1 scan of the table (to find the MS2 of a feature)
    scan_rt: np.ndarray = field(default=None)
    dt: float = 0.0                # median scan interval (seconds)
    timing: dict = field(default_factory=dict)

    def __len__(self):
        return len(self.mz)

    def select(self, keep) -> "Features":
        d = {k: getattr(self, k) for k in ("mz", "rt", "apex", "area", "height", "npts", "left", "right", "background", "artefact")}
        return Features(**{k: v[keep] for k, v in d.items()}, scan_ids=self.scan_ids, scan_rt=self.scan_rt, dt=self.dt, timing=self.timing)


def _segments(lmz, mz, it, pos, nscan, shift, max_gap, keep_floor, min_height):
    """Runs of consecutive scans in one log-ppm channel. `lmz` = log(mz)/log(1+4ppm), `mz` sorted ascending (so the channels are contiguous).
    Channels whose strongest peak is below `keep_floor` cannot hold a trace of height `min_height` and are dropped first (most of the noise).
    Returns channel, weighted m/z, apex position, points, height, area (counts, not yet x dt), first and last position of every run; only the
    runs with at least 2 points and a height of `min_height`."""
    b = np.floor(lmz + shift).astype(np.int64)
    rs = np.flatnonzero(np.r_[True, b[1:] != b[:-1]])                   # start of every channel run (mz is sorted)
    cmax = np.maximum.reduceat(it, rs)
    run_ok = cmax >= keep_floor
    sel = np.repeat(run_ok, np.diff(np.r_[rs, len(b)]))
    b, m, y, p = b[sel], mz[sel], it[sel], pos[sel]
    key = b * (nscan + 1) + p
    o = np.argsort(key, kind="stable")
    ks = key[o]
    new = np.r_[True, ks[1:] != ks[:-1]]
    cell = np.flatnonzero(new)                                          # cells = (channel, scan): the peaks of one scan in one channel are summed
    I = np.add.reduceat(y[o], cell)
    MZw = np.add.reduceat((y * m)[o], cell)
    uk = ks[cell]
    B, P = uk // (nscan + 1), uk % (nscan + 1)
    brk = np.r_[True, (B[1:] != B[:-1]) | (P[1:] - P[:-1] > max_gap)]
    st = np.flatnonzero(brk)
    seg = np.cumsum(brk) - 1
    n = np.diff(np.r_[st, len(P)])
    mx = np.maximum.reduceat(I, st)
    good = (n >= 2) & (mx >= min_height)
    hit = np.flatnonzero((I == mx[seg]) & good[seg])                    # first cell of each good run that reaches its maximum = apex
    if len(hit) == 0:                                                   # a file without any trace (a short blank): nothing to return
        e, f = np.zeros(0, np.int64), np.zeros(0)
        return e, f, e, e, f, f, e, e
    sh = seg[hit]
    first = np.r_[True, sh[1:] != sh[:-1]]
    apex = np.zeros(len(st), np.int64)
    apex[sh[first]] = P[hit[first]]
    tot = np.add.reduceat(I, st)
    mzc = np.add.reduceat(MZw, st) / tot
    en = np.r_[st[1:] - 1, len(P) - 1]
    return B[st][good], mzc[good], apex[good], n[good], mx[good], tot[good], P[st][good], P[en][good]


def _stitch(chan, mzc, apex, n, mx, tot, a, z, max_gap=MAX_GAP, touch_ppm=4.0, touch_ratio=10.0, touch_height=TOUCH_HEIGHT):
    """Merge segments of adjacent channels that are pieces of one ion: their scan ranges overlap by at least half of the shorter one; or they
    touch (a gap of at most `max_gap` scans, or at most `max_gap` + 1 shared scans: the ion leaves one channel and enters the next, no scan in
    common) and are alike (m/z within `touch_ppm`, heights within a factor `touch_ratio`) and intense (both above `touch_height`: only the
    intense ions are cut up, and unrelated background ions of low intensity must not join).
    Connected components by label propagation (a handful of vectorised passes). Returns the merged arrays."""
    m = len(chan)
    if m < 2:
        return chan, mzc, apex, n, mx, tot, a, z
    o = np.argsort(chan, kind="stable")                                # segments are already ordered by channel; keep it explicit
    ch = chan[o]
    lo = np.searchsorted(ch, ch + 1, "left")
    hi = np.searchsorted(ch, ch + 1, "right")
    cnt = hi - lo
    i = np.repeat(np.arange(m), cnt)
    j = np.arange(int(cnt.sum())) - np.repeat(np.cumsum(cnt) - cnt, cnt) + np.repeat(lo, cnt)
    ia, ja = o[i], o[j]
    ov = np.minimum(z[ia], z[ja]) - np.maximum(a[ia], a[ja]) + 1
    short = np.minimum(z[ia] - a[ia] + 1, z[ja] - a[ja] + 1)
    like = (np.abs(mzc[ia] - mzc[ja]) <= touch_ppm * 1e-6 * mzc[ia]) & (np.maximum(mx[ia], mx[ja]) <= touch_ratio * np.minimum(mx[ia], mx[ja])) & (np.minimum(mx[ia], mx[ja]) >= touch_height)
    ok = (ov >= 0.5 * short) | ((ov >= 1 - max_gap) & (ov <= max_gap + 1) & like)
    ia, ja = ia[ok], ja[ok]
    lab = np.arange(m)
    if len(ia):
        for _ in range(64):
            low = np.minimum(lab[ia], lab[ja])
            new = lab.copy()
            np.minimum.at(new, ia, low)
            np.minimum.at(new, ja, low)
            new = new[new]
            if np.array_equal(new, lab):
                break
            lab = new
    _, g = np.unique(lab, return_inverse=True)
    k = g.max() + 1
    tots = np.bincount(g, tot, minlength=k)
    mzs = np.bincount(g, mzc * tot, minlength=k) / tots
    ns = np.bincount(g, n, minlength=k)
    mxs = np.zeros(k)
    np.maximum.at(mxs, g, mx)
    a2 = np.full(k, np.iinfo(np.int64).max)
    np.minimum.at(a2, g, a)
    z2 = np.zeros(k, np.int64)
    np.maximum.at(z2, g, z)
    # apex: of the member with the highest height
    oo = np.lexsort((-mx, g))
    firstm = np.r_[True, g[oo][1:] != g[oo][:-1]]
    ap = np.zeros(k, np.int64)
    ap[g[oo][firstm]] = apex[oo][firstm]
    ch2 = np.zeros(k, np.int64)
    ch2[g[oo][firstm]] = chan[oo][firstm]
    return ch2, mzs, ap, ns, mxs, tots, a2, z2


def _merge_grids(F: np.ndarray, rt_tol: float = 0.05, ppm: float = 5e-6, slack: float = 0.02) -> np.ndarray:
    """Rows of F (columns: m/z, apex rt, points, first rt, last rt, grid) that are the same ion seen on the two grids; the one with more points
    stays. Same ion: different grids, within 5 ppm, and either the apexes are within 0.05 min or the shorter trace lies inside the longer one
    (a trace that one grid cut into pieces: each piece is inside the whole one). Returns the boolean keep mask."""
    keep = np.ones(len(F), bool)
    o = np.argsort(F[:, 0], kind="stable")
    Fm = F[o]
    for j in range(1, 6):
        if len(Fm) <= j:
            break
        x, y = Fm[j:], Fm[:-j]
        near = (np.abs(x[:, 0] - y[:, 0]) <= x[:, 0] * ppm) & (x[:, 5] != y[:, 5])
        inside = ((x[:, 3] >= y[:, 3] - slack) & (x[:, 4] <= y[:, 4] + slack)) | ((y[:, 3] >= x[:, 3] - slack) & (y[:, 4] <= x[:, 4] + slack))
        same = near & ((np.abs(x[:, 1] - y[:, 1]) <= rt_tol) | inside)
        drop_hi = same & (x[:, 2] <= y[:, 2])
        drop_lo = same & ~drop_hi
        keep[o[j:][drop_hi]] = False
        keep[o[:-j][drop_lo]] = False
    return keep


def fourier_artefacts(mz, apex, height, ratio: float = 50.0, ppm_lo: float = 20.0, ppm_hi: float = 200.0, max_scans: int = 1) -> np.ndarray:
    """Side lobes of Fourier-transform spectra: a feature whose apex is within `max_scans` of that of a feature `ratio` times more intense,
    20-200 ppm away from it. Only the very intense features can be the source, so the search is over few references."""
    out = np.zeros(len(mz), bool)
    if len(mz) == 0:
        return out
    refs = np.flatnonzero(height >= ratio * height.min())
    if len(refs) == 0:
        return out
    o = np.argsort(mz, kind="stable")
    ms = mz[o]
    lo = np.searchsorted(ms, mz[refs] * (1 - ppm_hi * 1e-6), "left")
    hi = np.searchsorted(ms, mz[refs] * (1 + ppm_hi * 1e-6), "right")
    cnt = hi - lo
    r = np.repeat(refs, cnt)
    c = o[np.arange(int(cnt.sum())) - np.repeat(np.cumsum(cnt) - cnt, cnt) + np.repeat(lo, cnt)]
    d = np.abs(mz[c] - mz[r]) / mz[r] * 1e6
    bad = (d >= ppm_lo) & (d <= ppm_hi) & (np.abs(apex[c] - apex[r]) <= max_scans) & (height[r] >= ratio * height[c])
    out[c[bad]] = True
    return out


def _slab_edges(mz: np.ndarray, max_peaks: int, min_gap_ppm: float = 50.0) -> list[int]:
    """Cut points (indices into the m/z-sorted peaks) that split the table in slabs of about `max_peaks` peaks, each at the widest gap of the
    neighbourhood and only where the gap is at least `min_gap_ppm` (a trace never spans such a gap: the channels are 4 ppm, the stitching joins
    neighbours). Slabs bound the transient memory of the detection; the result does not change."""
    n = len(mz)
    cuts = [0]
    gap = np.diff(mz) / mz[1:] * 1e6 if n > 1 else np.zeros(0)
    while n - cuts[-1] > 1.4 * max_peaks:
        lo, hi = cuts[-1] + int(0.6 * max_peaks), min(n - 1, cuts[-1] + int(1.4 * max_peaks))
        j = lo + int(np.argmax(gap[lo:hi]))
        if gap[j] < min_gap_ppm:
            hi2 = min(n - 1, hi + max_peaks)                              # no wide gap here: look further
            if hi2 <= hi:
                break
            j = hi + int(np.argmax(gap[hi:hi2]))
            if gap[j] < min_gap_ppm:
                break
        cuts.append(j + 1)
    cuts.append(n)
    return cuts


def detect(table, floor: float = FLOOR, min_points: int = MIN_POINTS, min_height: float = MIN_HEIGHT, max_gap: int = MAX_GAP,
           long_trace: int = LONG_TRACE, max_peaks: int = SLAB_PEAKS) -> Features:
    """Ion traces of a PeakTable (MS1 of one file and polarity). See the module docstring."""
    import time
    t0 = time.perf_counter()
    nscan = len(table.rt)
    k = table.inten >= floor
    mz, it, pos = table.mz[k].astype(np.float64), table.inten[k].astype(np.float64), table.pos[k].astype(np.int64)
    dt = float(np.median(np.diff(table.rt))) * 60 if nscan > 1 else 1.0
    rows = []
    lmz = np.log(mz) / np.log1p(CHANNEL_PPM) if len(mz) else mz
    cuts = _slab_edges(mz, max_peaks) if len(mz) else [0, 0]
    for lo, hi in zip(cuts[:-1], cuts[1:]):
        for shift in (0.0, 0.5):
            if hi == lo:
                break
            # fragments of a wobbling ion are short: only 2 points are asked before stitching, the length test (min_points) comes after it
            chan, mzc, apex, n, mx, tot, a, z = _segments(lmz[lo:hi], mz[lo:hi], it[lo:hi], pos[lo:hi], nscan, shift, max_gap, min_height / 4, min_height)
            chan, mzc, apex, n, mx, tot, a, z = _stitch(chan, mzc, apex, n, mx, tot, a, z, max_gap)
            good = (n >= min_points) & (mx >= min_height)
            rows.append(np.c_[mzc[good], table.rt[apex[good]], n[good], mx[good], tot[good] * dt, apex[good], table.rt[a[good]], table.rt[z[good]], (z - a + 1)[good] > long_trace,
                              np.full(good.sum(), shift)])
    F = np.concatenate(rows) if rows else np.zeros((0, 10))
    F = F[_merge_grids(F[:, [0, 1, 2, 6, 7, 9]])] if len(F) else F
    art = fourier_artefacts(F[:, 0], F[:, 5], F[:, 3]) if len(F) else np.zeros(0, bool)
    o = np.argsort(F[:, 0], kind="stable")
    F, art = F[o], art[o]
    return Features(mz=F[:, 0], rt=F[:, 1], apex=F[:, 5].astype(np.int64), area=F[:, 4], height=F[:, 3], npts=F[:, 2].astype(np.int64), left=F[:, 6], right=F[:, 7],
                    background=F[:, 8] > 0, artefact=art, scan_ids=np.asarray(table.scan_ids), scan_rt=np.asarray(table.rt), dt=dt,
                    timing={"detect": round(time.perf_counter() - t0, 3)})


def detect_file(run, polarity: int = 1, **kw) -> Features:
    """Features of one Run; the MS1 peak table is released afterwards (one file in memory at a time)."""
    t = run.table(1, polarity)
    try:
        return detect(t, **kw)
    finally:
        run._tables.clear()


# ---------------------------------------------------------------------------------------------------------------------- gap filling
def trace_area(table, mz: float, rt_lo: float, rt_hi: float, floor: float = FLOOR, ppm: float = 5.0, min_points: int = MIN_POINTS, max_gap: int = MAX_GAP,
               long_trace: int = LONG_TRACE, edge: float | None = None) -> tuple[float, float]:
    """Area (counts x seconds, as `detect`) and apex RT of the strongest trace of `mz` +- `ppm` whose apex lies in [rt_lo, rt_hi]: the same rule as the
    detection (gaps <= `max_gap`, at least `min_points` points) but with the intensity floor as the only height threshold. (0.0, nan) when there is none."""
    nscan = len(table.rt)
    if nscan < 3:
        return 0.0, float("nan")
    x = table.xic(mz, mz * ppm * 1e-6)
    x = np.where(x >= floor, x, 0.0)
    i0, i1 = int(np.searchsorted(table.rt, rt_lo)), int(np.searchsorted(table.rt, rt_hi, "right"))
    if i1 <= i0 or not (x[i0:i1] > 0).any():
        return 0.0, float("nan")
    apex = i0 + int(np.argmax(x[i0:i1]))
    a = z = apex
    miss = 0
    for j in range(apex - 1, -1, -1):                      # left edge: stop after more than max_gap empty scans
        miss = 0 if x[j] > 0 else miss + 1
        if miss > max_gap:
            break
        if x[j] > 0:
            a = j
    miss = 0
    for j in range(apex + 1, nscan):
        miss = 0 if x[j] > 0 else miss + 1
        if miss > max_gap:
            break
        if x[j] > 0:
            z = j
    seg = x[a:z + 1]
    if (seg > 0).sum() < min_points or z - a + 1 > long_trace:
        return 0.0, float("nan")
    dt = float(np.median(np.diff(table.rt))) * 60
    return float(seg.sum() * dt), float(table.rt[apex])


def fill_gaps(al: "Alignment", rows, tables, columns, floor: float = FLOOR, ppm: float = 5.0, rt_tol: float = 0.1, rtmap=None) -> np.ndarray:
    """Replace the zero areas of the groups `rows` in the files `columns` by the area of the same ion re-extracted at the intensity floor (m/z: the
    group's, +- `ppm`). Where the peak is looked for: with the RT map of the series (`rtmap`, see `rtalign`) at its reference RT shifted by the
    file's own shift, +- `rt_tol` + twice the residual of that file's curve (the apex must lie there); without a map, at the median apex of the
    files where it was found, +- its half width + `rt_tol`. `tables`: a callable file index -> PeakTable (one file is touched at a time by the
    caller). Only m/z, RT and area are looked at. Changes `al.area` and returns the boolean matrix (len(al), n_files) of the filled cells."""
    filled = np.zeros(al.area.shape, bool)
    rows = np.asarray(rows, int)
    use_map = rtmap is not None and rtmap.ok
    for k in columns:
        todo = rows[al.area[rows, k] <= 0]
        if len(todo) == 0:
            continue
        table = tables(k)
        for g in todo.tolist():
            have = ~np.isnan(al.apex_rt[g])
            if not have.any():
                continue
            if use_map:
                ks = np.flatnonzero(have)
                ref = float(np.median([rtmap.reference(float(al.apex_rt[g, j]), j) for j in ks]))
                c = float(rtmap.expected(ref, k))
                w = rt_tol + 2 * float(rtmap.rms[k])
            else:
                c = float(np.median(al.apex_rt[g][have]))
                w = float(np.median((al.right[g] - al.left[g])[have])) / 2 + rt_tol
            area, _ = trace_area(table, float(al.mz[g]), c - w, c + w, floor, ppm)
            if area > 0:
                al.area[g, k] = area
                filled[g, k] = True
    return filled


# ---------------------------------------------------------------------------------------------------------------------- alignment
@dataclass
class Alignment:
    """Groups of features across files: one row per (m/z, RT) group, one column per file."""
    mz: np.ndarray
    rt: np.ndarray
    area: np.ndarray                # (groups, files); 0 where the file has no feature
    height: np.ndarray
    apex_rt: np.ndarray             # float32 (groups, files), NaN where absent
    left: np.ndarray
    right: np.ndarray
    n_files: int

    def __len__(self):
        return len(self.mz)


def _limit_span(x: np.ndarray, g: np.ndarray, max_span: float) -> np.ndarray:
    """Groups g (non-decreasing; x is sorted inside each group) wider than `max_span` are cut at their largest internal gap, repeatedly (a few
    vectorised passes). A chain of small gaps would otherwise join an ion to the background ions that surround it."""
    for _ in range(32):
        brk = np.r_[True, g[1:] != g[:-1]]
        st = np.flatnonzero(brk)
        cid = np.cumsum(brk) - 1
        span = np.maximum.reduceat(x, st) - np.minimum.reduceat(x, st)
        big = (span > max_span)[cid]
        if not big.any():
            break
        gap = np.r_[0.0, np.diff(x)]
        gap[st] = -1.0
        gap = np.where(big, gap, -1.0)
        mxg = np.maximum.reduceat(gap, st)[cid]
        cut = big & (gap == mxg) & (gap > 0)
        g = np.cumsum(brk | cut) - 1
    return g


def align(feats: list[Features], ppm: float = 5.0, rt_gap: float = 0.1, max_ppm_span: float = 10.0, max_rt_span: float | None = None,
          drop_artefacts: bool = True) -> Alignment:
    """Align the features of all files: sort by m/z and break the groups where the gap is above `ppm`, then sort by RT inside each group and
    break where the gap is above `rt_gap` minutes; groups wider than `max_ppm_span` or `max_rt_span` are cut at their largest gap. Area,
    height, apex and limits of the peak are kept per file (the largest feature of the group in a file wins)."""
    nf = len(feats)
    cols = []
    for k, f in enumerate(feats):
        keep = ~f.artefact if drop_artefacts else np.ones(len(f), bool)
        cols.append(np.c_[f.mz[keep], f.rt[keep], f.area[keep], f.height[keep], f.left[keep], f.right[keep], np.full(keep.sum(), k)])
    X = np.concatenate(cols) if cols else np.zeros((0, 7))
    if len(X) == 0:
        z = np.zeros((0, nf))
        return Alignment(np.zeros(0), np.zeros(0), z, z, z.astype(np.float32), z.astype(np.float32), z.astype(np.float32), nf)
    o = np.argsort(X[:, 0], kind="stable")
    X = X[o]
    cl = np.r_[0, np.cumsum(np.diff(X[:, 0]) > X[1:, 0] * ppm * 1e-6)]
    cl = _limit_span(np.log(X[:, 0]) * 1e6, cl, max_ppm_span)              # ln(m/z) x 1e6 = ppm
    o = np.lexsort((X[:, 1], cl))
    X, cl = X[o], cl[o]
    g = np.r_[0, np.cumsum((np.diff(cl) != 0) | (np.diff(X[:, 1]) > rt_gap))]
    if max_rt_span is not None:
        g = _limit_span(X[:, 1], g, max_rt_span)
    ng = int(g[-1]) + 1
    fi = X[:, 6].astype(np.int64)
    # the largest feature of each (group, file) gives apex and limits
    o = np.lexsort((-X[:, 2], fi, g))
    first = np.r_[True, (g[o][1:] != g[o][:-1]) | (fi[o][1:] != fi[o][:-1])]
    top = o[first]
    A = np.zeros((ng, nf))
    np.maximum.at(A, (g, fi), X[:, 2])
    H = np.zeros((ng, nf))
    np.maximum.at(H, (g, fi), X[:, 3])
    R = np.full((ng, nf), np.nan, np.float32)
    L = np.full((ng, nf), np.nan, np.float32)
    Rr = np.full((ng, nf), np.nan, np.float32)
    R[g[top], fi[top]] = X[top, 1]
    L[g[top], fi[top]] = X[top, 4]
    Rr[g[top], fi[top]] = X[top, 5]
    # m/z and RT of a group: area-weighted over the largest feature of each file (the tails of an intense ion, which drift in m/z by a few ppm
    # with the space charge, would otherwise pull the group away from the peak)
    w = np.bincount(g[top], X[top, 2])
    mz = np.bincount(g[top], X[top, 0] * X[top, 2]) / w
    rt = np.bincount(g[top], X[top, 1] * X[top, 2]) / w
    return Alignment(mz=mz, rt=rt, area=A, height=H, apex_rt=R, left=L, right=Rr, n_files=nf)
