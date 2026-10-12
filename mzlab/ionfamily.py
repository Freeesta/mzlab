"""Ion families: where does this ion come from? Evidence for in-source fragments (ISF) versus real products.

PUBLIC module (also used by the student tool "Da dove viene questo ione?"): it extracts and measures, it never
issues a verdict. The probabilistic verdict lives in the private TP Mine package (tpmine/isf.py), which imports this module.
Everything is plain numpy (runs in Pyodide: no scipy, no sklearn); every function returns floats, lists or dicts that
json.dumps can serialize (use `to_jsonable` on a whole report), and the unit-resolution caveat always applies: an m/z is a
candidate, never an identification.

Why the metrics (the principle): an in-source fragment F is born from its precursor P in the source, so F inherits P's
chromatographic profile (same apex, same shape), the ratio F/P is constant inside the peak and between samples, and F follows
P's kinetics (it decays with the treatment time). A real transformation product is another molecule: another RT (usually
earlier, more polar), another shape, a formation-decay kinetics. No single metric proves it: they are combined with care.

Contents
  1. data       SparseMap-like use of reader.PeakTable, centWave-style ROI tracing (`build_rois`), `extract_trace`, caches
  2. signal     Savitzky-Golay, AsLS / SNIP baseline, CWT peak picking (Ricker, ridge lines), peak limits, FWHM, apex +- sigma
  3. profile    windowed weighted Pearson/Spearman, derivative corr, cosine, apex shift, lag +-2 scans, FWHM ratio,
                shift-surrogate p-value (autocorrelation-aware) and block-bootstrap CI  -> `profile_similarity`
  4. ratio      F = r P in the peak: weighted total least squares (Deming through the origin), Theil-Sen, residuals, R2,
                CV of the per-scan ratio, constancy of r between samples (Cochran Q)        -> `ratio_fit`, `ratio_constancy`
  5. kinetics   areas vs time, Spearman, Kendall tau, first-order decay, consecutive A->B->C, bootstrap CI -> `kinetics`
  6. families   hierarchical clustering with profile + apex + kinetics distance, roles (isotope, adduct, fragment candidate,
                dimer) with formula constraints and isotope envelope check                  -> `ion_families`, `annotate_roles`
  7. MCR        MCR-ALS (non-negativity + unimodality, SIMPLISMA init, SVD/EFA components), NMF-NNDSVD, stability -> `mcr_als`
  8. ramp       source-voltage (DP) ramp: breakdown sigmoid, appearance DP, ISF vs TP slope  -> `source_ramp`
  9. MS2        cosine, modified cosine, "X among the products of P", subset score          -> `ms2_similarity`
 10. report     `origin_report`: everything above for one ion, nothing concluded (used by /api/origin)

Calibration of the thresholds: see `tests/test_ionfamily.py` (synthetic truth, ROC/AUC) and the notes in `calibrate_thresholds`.
"""
from __future__ import annotations

import math
import time
from collections import OrderedDict

import numpy as np

from .i18n import message

_trapz = getattr(np, "trapezoid", None) or np.trapz     # numpy 1.x / 2.x

# ----------------------------------------------------------------------------------------------------------------------
# constants
# ----------------------------------------------------------------------------------------------------------------------
TOL_MZ = 0.35            # Da, ROI half-window (unit resolution; the centroid of a real ion scatters by ~0.04 Da)
MIN_SCANS_RELIABLE = 6   # fewer scans inside the peak -> every shape metric is unreliable
H = 1.00782503223
PROTON = 1.00727646688
NA, K_, NH4, ACN = 22.989769282, 38.9637064, 18.0338262, 41.0265491   # monoisotopic masses (ACN neutral, NH4 as ion mass)
_ISO = {   # element -> [(mass shift (nominal) relative to the lightest isotope, abundance)] for the envelope calculator
    "C": [(0, 0.9893), (1, 0.0107)], "H": [(0, 0.999885), (1, 0.000115)], "N": [(0, 0.99636), (1, 0.00364)],
    "O": [(0, 0.99757), (1, 0.00038), (2, 0.00205)], "S": [(0, 0.9499), (1, 0.0075), (2, 0.0425), (4, 0.0001)],
    "Cl": [(0, 0.7576), (2, 0.2424)], "Br": [(0, 0.5069), (2, 0.4931)], "F": [(0, 1.0)], "P": [(0, 1.0)], "I": [(0, 1.0)],
    "Na": [(0, 1.0)], "K": [(0, 0.932581), (2, 0.067302)], "Si": [(0, 0.9223), (1, 0.0468), (2, 0.0309)],
}
# common neutral losses (same list as the "Perdite neutre" table of the program: web/tables.js), formula -> note
NEUTRAL_LOSSES = ["H2O", "NH3", "CO", "CO2", "CH2O", "CH4O", "C2H2O", "HCOOH", "C2H4", "C3H6", "HCN", "HF", "HCl", "HBr",
                  "SO2", "SO3", "H2S", "CH3", "NO2", "C6H6"]
# adducts of the neutral M, as (name, m/z shift from the neutral mass, multiplier of M)
ADDUCTS = [("[M+H]+", PROTON, 1), ("[M+NH4]+", NH4, 1), ("[M+Na]+", NA - 0.000548579909, 1), ("[M+K]+", K_ - 0.000548579909, 1),
           ("[M+H+ACN]+", PROTON + ACN, 1), ("[2M+H]+", PROTON, 2), ("[2M+Na]+", NA - 0.000548579909, 2)]


def to_jsonable(o):
    """numpy -> plain Python recursively (so that json.dumps works on a whole report); NaN/inf -> None."""
    if isinstance(o, dict):
        return {str(k): to_jsonable(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [to_jsonable(v) for v in o]
    if isinstance(o, np.ndarray):
        return to_jsonable(o.tolist())
    if isinstance(o, (np.bool_, bool)):
        return bool(o)
    if isinstance(o, (np.integer, int)):
        return int(o)
    if isinstance(o, (np.floating, float)):
        f = float(o)
        return f if math.isfinite(f) else None
    return o


# ----------------------------------------------------------------------------------------------------------------------
# 1. data: ROIs and traces
# ----------------------------------------------------------------------------------------------------------------------
class Window:
    """Dense extraction of a RT window: `rt` (n scans), `mz` (m ROI centres), `X` (m x n intensities), plus the scan range."""

    def __init__(self, rt, mz, X, scan_lo, mz_lo=None, mz_hi=None, n_peaks=0):
        self.rt, self.mz, self.X, self.scan_lo = rt, mz, X, scan_lo
        self.mz_lo, self.mz_hi, self.n_peaks = mz_lo, mz_hi, n_peaks

    @property
    def dt(self) -> float:
        """Median scan interval in minutes."""
        return float(np.median(np.diff(self.rt))) if len(self.rt) > 2 else 0.0


_CACHE: "OrderedDict" = OrderedDict()
_CACHE_MAX = 6


def _cache_get(key):
    v = _CACHE.get(key)
    if v is not None:
        _CACHE.move_to_end(key)
    return v


def _cache_put(key, v):
    _CACHE[key] = v
    while len(_CACHE) > _CACHE_MAX:
        _CACHE.popitem(last=False)


def clear_cache():
    _CACHE.clear()


def _scan_sorted(table, lo: int, hi: int):
    """Peaks of scans lo..hi-1 sorted by scan (then m/z): (mz, inten, pos, offsets) with offsets[i] = first peak of scan lo+i."""
    sel = (table.pos >= lo) & (table.pos < hi)
    mz, y, p = table.mz[sel], table.inten[sel], table.pos[sel]
    order = np.argsort(p, kind="stable")
    mz, y, p = mz[order], y[order], p[order]
    off = np.searchsorted(p, np.arange(lo, hi + 1))
    return mz, y, p, off


def build_rois(table, rt0: float, rt1: float, tol: float = TOL_MZ, min_scans: int = 4, max_gap: int = 2,
               mz_range=None, min_int: float = 0.0, key=None) -> Window:
    """Regions of interest (ROI) in a RT window, centWave style: scan after scan every centroid is attached to the ROI whose
    running mean m/z is within +-tol (continuity in RT: a ROI survives `max_gap` empty scans); two centroids of the same scan never
    share a ROI (the stronger wins). ROIs seen in fewer than `min_scans` scans are dropped. Result: dense matrix ROI x scan.
    Unit resolution: tol 0.35 keeps neighbouring nominal masses apart. Cached per (file key, window, tol)."""
    ck = (key, round(rt0, 4), round(rt1, 4), tol, min_scans, max_gap, mz_range, min_int)
    if key is not None:
        hit = _cache_get(ck)
        if hit is not None:
            return hit
    rt = table.rt
    lo = int(np.searchsorted(rt, rt0, side="left"))
    hi = int(np.searchsorted(rt, rt1, side="right"))
    n = hi - lo
    if n <= 0 or len(table.mz) == 0:
        return Window(rt[lo:hi], np.zeros(0), np.zeros((0, max(n, 0))), lo)
    mz, y, p, off = _scan_sorted(table, lo, hi)
    keep = y > min_int
    if mz_range is not None:
        keep &= (mz >= mz_range[0]) & (mz <= mz_range[1])
    # ROI state (python lists of arrays would be slow: use growing numpy buffers)
    cap = max(1024, int(len(mz) / max(n, 1)) * 4)
    r_mz = np.zeros(cap)
    r_cnt = np.zeros(cap, dtype=np.int64)
    r_last = np.full(cap, -10 ** 9, dtype=np.int64)
    n_roi = 0
    a_roi, a_scan, a_y, a_mz = [], [], [], []
    for s in range(n):
        a, b = off[s], off[s + 1]
        if b <= a:
            continue
        k = keep[a:b]
        pm, py = mz[a:b][k], y[a:b][k]
        if len(pm) == 0:
            continue
        assigned = np.full(len(pm), -1, dtype=np.int64)
        extra = np.zeros(len(pm), dtype=bool)          # centroid inside the tolerance of a ROI that already has a stronger one this scan
        act = np.flatnonzero(r_last[:n_roi] >= s - max_gap - 1)
        near = np.zeros(len(pm), dtype=bool)
        if len(act):
            am = r_mz[act]
            o = np.argsort(am)
            act, am = act[o], am[o]
            j = np.searchsorted(am, pm)
            jl, jr = np.clip(j - 1, 0, len(am) - 1), np.clip(j, 0, len(am) - 1)
            dl, dr = np.abs(pm - am[jl]), np.abs(pm - am[jr])
            jb = np.where(dl <= dr, jl, jr)
            near = np.minimum(dl, dr) <= tol
            cand = np.flatnonzero(near)
            if len(cand):
                order = cand[np.argsort(-py[cand], kind="stable")]
                tgt = act[jb[order]]
                _, first = np.unique(tgt, return_index=True)
                strong = order[first]
                assigned[strong] = act[jb[strong]]
                rest = np.setdiff1d(order, strong)
                assigned[rest] = act[jb[rest]]
                extra[rest] = True
        # a centroid within tol of an active ROI never starts a new ROI (it would split the ion into competing ROIs)
        new = np.flatnonzero(~near)
        if len(new):
            if n_roi + len(new) > len(r_mz):
                grow = max(len(r_mz), len(new))
                r_mz = np.concatenate([r_mz, np.zeros(grow)])
                r_cnt = np.concatenate([r_cnt, np.zeros(grow, dtype=np.int64)])
                r_last = np.concatenate([r_last, np.full(grow, -10 ** 9, dtype=np.int64)])
            ids = np.arange(n_roi, n_roi + len(new))
            r_mz[ids], r_cnt[ids] = pm[new], 0
            assigned[new] = ids
            n_roi += len(new)
        # update running means with the strongest centroid of each ROI (robust to chemical-noise neighbours)
        m_ = ~extra
        ids = assigned[m_]
        r_mz[ids] = (r_mz[ids] * r_cnt[ids] + pm[m_]) / (r_cnt[ids] + 1)
        r_cnt[ids] += 1
        r_last[assigned] = s
        a_roi.append(assigned)
        a_scan.append(np.full(len(assigned), s, dtype=np.int64))
        a_y.append(py)
        a_mz.append(pm)
    if n_roi == 0 or not a_roi:
        return Window(rt[lo:hi], np.zeros(0), np.zeros((0, n)), lo)
    a_roi, a_scan, a_y, a_mz = map(np.concatenate, (a_roi, a_scan, a_y, a_mz))
    good = r_cnt[:n_roi] >= min_scans
    remap = np.full(n_roi, -1, dtype=np.int64)
    remap[good] = np.arange(int(good.sum()))
    sel = remap[a_roi] >= 0
    X = np.bincount(remap[a_roi[sel]] * n + a_scan[sel], weights=a_y[sel], minlength=int(good.sum()) * n).reshape(int(good.sum()), n)
    centre = np.bincount(remap[a_roi[sel]], weights=a_mz[sel] * a_y[sel], minlength=X.shape[0]) / np.maximum(X.sum(1), 1e-12)
    order = np.argsort(centre)
    w = Window(rt[lo:hi], centre[order], X[order], lo, float(mz.min()), float(mz.max()), int(keep.sum()))
    if key is not None:
        _cache_put(ck, w)
    return w


def extract_trace(table, mz: float, tol: float, rt0: float | None = None, rt1: float | None = None):
    """(rt, intensity) of one ion: sum of the centroids within mz +- tol per scan, in the window (all the run if None)."""
    y = table.xic(mz, tol)
    rt = table.rt
    if rt0 is None:
        return rt, y
    a, b = np.searchsorted(rt, rt0, side="left"), np.searchsorted(rt, rt1, side="right")
    return rt[a:b], y[a:b]


# ----------------------------------------------------------------------------------------------------------------------
# 2. signal processing
# ----------------------------------------------------------------------------------------------------------------------
def savgol_coeffs(window: int, poly: int, deriv: int = 0) -> np.ndarray:
    """Savitzky-Golay convolution coefficients (odd window), derivative order `deriv`, unit spacing."""
    window = int(window) | 1
    half = window // 2
    x = np.arange(-half, half + 1, dtype=float)
    A = np.vander(x, poly + 1, increasing=True)
    pinv = np.linalg.pinv(A)
    c = pinv[deriv] * math.factorial(deriv)
    return c[::-1]


def savgol(y, window: int = 7, poly: int = 2, deriv: int = 0) -> np.ndarray:
    """Savitzky-Golay smoothing/derivative with edge padding by reflection (so the edges are not zeroed)."""
    y = np.asarray(y, float)
    n = len(y)
    window = int(window) | 1
    if n < 3:
        return y.copy()
    window = min(window, n if n % 2 else n - 1)
    poly = min(poly, window - 1)
    half = window // 2
    c = savgol_coeffs(window, poly, deriv)
    pad = np.pad(y, half, mode="reflect") if n > half else np.pad(y, half, mode="edge")
    return np.convolve(pad, c, mode="valid")


def snip_baseline(y, iters: int = 20) -> np.ndarray:
    """SNIP baseline (statistics-sensitive non-linear iterative peak clipping), clipping window growing to `iters` scans."""
    y = np.asarray(y, float)
    v = np.log(np.log(np.sqrt(np.maximum(y, 0) + 1) + 1) + 1)
    n = len(v)
    for w in range(1, int(min(iters, max(1, n // 2 - 1))) + 1):
        a = (np.r_[v[w:], [v[-1]] * w] + np.r_[[v[0]] * w, v[:-w]]) / 2.0
        v = np.minimum(v, a)
    return (np.exp(np.exp(v) - 1) - 1) ** 2 - 1


_ASLS_P: dict = {}


def asls_baseline(y, lam: float = 1e5, p: float = 0.01, iters: int = 10, max_n: int = 160) -> np.ndarray:
    """Asymmetric least squares baseline (Eilers & Boelens 2005): minimise sum w (y-z)^2 + lam sum (D2 z)^2 with weights p for
    points above the baseline and 1-p below. The baseline is smooth by construction, so a long trace is first averaged in blocks of k
    points (k = ceil(n / max_n), lam scaled by k^-4 to keep the same smoothness in time) and the result interpolated back: the dense
    solve is cubic in n, which matters in the browser (Pyodide, about 10x slower). Very long traces fall back to SNIP."""
    y = np.asarray(y, float)
    n = len(y)
    if n < 5:
        return np.full(n, float(y.min()) if n else 0.0)
    if n > 20000:
        return snip_baseline(y)
    k = max(1, -(-n // max_n))
    if k > 1:
        m = n // k
        yd = y[: m * k].reshape(m, k).mean(axis=1)
        xd = (np.arange(m) + 0.5) * k - 0.5
        zd = asls_baseline(yd, lam / k ** 4, p, iters, max_n=10 ** 9)
        return np.minimum(np.interp(np.arange(n), xd, zd), y)
    key = (n, lam)
    P = _ASLS_P.get(key)
    if P is None:
        D = np.diff(np.eye(n), 2, axis=0)
        P = lam * (D.T @ D)
        if len(_ASLS_P) > 64:
            _ASLS_P.clear()
        _ASLS_P[key] = P
    w = np.ones(n)
    z = y.copy()
    for _ in range(iters):
        A = P.copy()
        A[np.diag_indices(n)] += w
        z = np.linalg.solve(A, w * y)
        w_new = np.where(y > z, p, 1 - p)
        if np.array_equal(w_new, w):
            break
        w = w_new
    return np.minimum(z, y)


def noise_level(y) -> float:
    """Robust noise sigma of a trace from the first differences (MAD): insensitive to peaks and slow drift."""
    y = np.asarray(y, float)
    if len(y) < 4:
        return 0.0
    d = np.diff(y)
    mad = np.median(np.abs(d - np.median(d)))
    return float(1.4826 * mad / math.sqrt(2.0))


def ricker(width: float, n: int) -> np.ndarray:
    h = n // 2
    x = np.arange(-h, h + 1, dtype=float)
    a = width
    return (1 - (x / a) ** 2) * np.exp(-0.5 * (x / a) ** 2) / math.sqrt(a)


def cwt_ricker(y, widths) -> np.ndarray:
    """Continuous wavelet transform with Ricker wavelets, one row per width (scans), same length as y."""
    y = np.asarray(y, float)
    out = np.zeros((len(widths), len(y)))
    for i, w in enumerate(widths):
        k = int(min(10 * w, len(y) - 1)) | 1
        wv = ricker(w, k)
        pad = np.pad(y, k // 2, mode="edge")
        out[i] = np.convolve(pad, wv, mode="valid")[: len(y)]
    return out


def _parabola_apex(rt, y, i, half: int = 2):
    """Apex time with uncertainty from a quadratic fit around index i: (t_apex, sigma_t, height). sigma by the delta method with
    the residual variance of the fit (or the noise level, if larger)."""
    n = len(y)
    a, b = max(0, i - half), min(n, i + half + 1)
    if b - a < 3:
        return float(rt[i]), float(np.diff(rt).mean()) if n > 1 else 0.0, float(y[i])
    x = rt[a:b] - rt[i]
    A = np.vander(x, 3, increasing=True)
    coef, res, rank, _ = np.linalg.lstsq(A, y[a:b], rcond=None)
    c0, c1, c2 = coef
    dt = float(np.median(np.diff(rt))) if n > 1 else 1.0
    if c2 >= 0 or rank < 3:
        return float(rt[i]), dt / 2, float(y[i])
    xa = -c1 / (2 * c2)
    xa = float(np.clip(xa, x[0], x[-1]))
    h = c0 + c1 * xa + c2 * xa * xa
    r = y[a:b] - A @ coef
    s2 = max(float(r @ r) / max(len(x) - 3, 1), noise_level(y) ** 2, 1e-12)
    cov = s2 * np.linalg.inv(A.T @ A)
    # xa = -c1/(2 c2): d/dc1 = -1/(2 c2), d/dc2 = c1/(2 c2^2)
    g = np.array([0.0, -1 / (2 * c2), c1 / (2 * c2 * c2)])
    var = float(g @ cov @ g)
    return float(rt[i] + xa), float(math.sqrt(max(var, 0.0))), float(h)


def detect_peaks(rt, y, snr: float = 5.0, min_scans: int = 3, baseline: str = "asls", frac: float = 0.05,
                 widths=None, smooth: int = 5) -> dict:
    """Peak picking centWave-style on one trace.

    1 baseline (AsLS or SNIP) -> 2 Savitzky-Golay smoothing -> 3 CWT with Ricker wavelets at several scales, ridge lines
    across scales, SNR = CWT ridge maximum / noise of the finest scale -> 4 limits: where the signal falls below `frac` of the
    apex height or where the derivative changes sign (valley) -> 5 FWHM by linear interpolation, apex +- sigma by parabola.
    Returns {"baseline", "smooth", "noise", "peaks": [ {apex_i, apex_rt, apex_sigma, height, area, lo, hi, fwhm_min, fwhm_scans,
    n_scans, snr, reliable} ]} sorted by height; reliable = False when fewer than MIN_SCANS_RELIABLE scans are inside the peak."""
    rt = np.asarray(rt, float)
    y = np.asarray(y, float)
    n = len(y)
    out = {"baseline": np.zeros(n), "smooth": np.zeros(n), "noise": 0.0, "peaks": []}
    if n < 7 or not np.any(y > 0):
        return out
    base = asls_baseline(y) if baseline == "asls" else snip_baseline(y)
    yc = y - base
    ys = np.maximum(savgol(yc, smooth, 2), 0.0)
    noise = max(noise_level(yc), 1e-9)
    out.update(baseline=base, smooth=ys, noise=noise)
    if widths is None:
        widths = np.unique(np.round(np.geomspace(1.0, max(3.0, min(n / 6.0, 12.0)), 7), 2))
    C = cwt_ricker(ys, widths)
    # ridge lines: local maxima per scale linked from the smallest to the largest scale
    peaks_per_scale = []
    for i in range(len(widths)):
        c = C[i]
        mx = np.flatnonzero((c[1:-1] > c[:-2]) & (c[1:-1] >= c[2:]) & (c[1:-1] > 0)) + 1
        peaks_per_scale.append(mx)
    scale_noise = np.array([max(float(np.median(np.abs(C[i] - np.median(C[i]))) * 1.4826), 1e-12) for i in range(len(widths))])
    ridges = []
    for i, mx in enumerate(peaks_per_scale):
        for pos in mx:
            for r in ridges:
                if r["last_scale"] == i - 1 or r["last_scale"] == i - 2:
                    if abs(r["pos"][-1] - pos) <= max(1.0, widths[i] / 2.0):
                        if r["last_scale"] == i:
                            continue
                        r["pos"].append(pos)
                        r["val"].append(C[i, pos] / scale_noise[i])
                        r["scale"].append(i)
                        r["last_scale"] = i
                        break
            else:
                ridges.append({"pos": [pos], "val": [C[i, pos] / scale_noise[i]], "scale": [i], "last_scale": i})
    found, taken = [], []
    for r in ridges:
        if len(r["pos"]) < 3:
            continue
        k = int(np.argmax(r["val"]))
        if r["val"][k] < snr:
            continue
        w = widths[r["scale"][k]]
        c0 = int(r["pos"][k])
        a, b = max(0, int(c0 - w)), min(n, int(c0 + w) + 1)
        apex = a + int(np.argmax(ys[a:b]))
        if ys[apex] < snr * noise:
            continue
        if any(abs(apex - t) <= max(1, w / 2) for t in taken):
            continue
        taken.append(apex)
        found.append((apex, float(r["val"][k]), float(w)))
    peaks = []
    for apex, rsn, w in sorted(found):
        h = float(ys[apex])
        lo = apex
        while lo > 0 and ys[lo - 1] > frac * h and ys[lo - 1] <= ys[lo] * 1.0 + 1e-12:
            lo -= 1
        hi = apex
        while hi < n - 1 and ys[hi + 1] > frac * h and ys[hi + 1] <= ys[hi] * 1.0 + 1e-12:
            hi += 1
        # one more point on each side if the signal is already below the fraction (the foot)
        lo2, hi2 = max(lo - 1, 0), min(hi + 1, n - 1)
        half = 0.5 * h
        # FWHM by interpolation
        li = apex
        while li > 0 and ys[li] > half:
            li -= 1
        ri = apex
        while ri < n - 1 and ys[ri] > half:
            ri += 1
        tl = rt[li] + (half - ys[li]) / max(ys[li + 1] - ys[li], 1e-12) * (rt[li + 1] - rt[li]) if (li < apex and ys[li] <= half) else rt[li]
        tr = rt[ri] - (half - ys[ri]) / max(ys[ri - 1] - ys[ri], 1e-12) * (rt[ri] - rt[ri - 1]) if (ri > apex and ys[ri] <= half) else rt[ri]
        fwhm = float(max(tr - tl, 0.0))
        dt = float(np.median(np.diff(rt)))
        t_ap, s_ap, hh = _parabola_apex(rt, ys, apex)
        area = float(_trapz(yc[lo2:hi2 + 1].clip(0), rt[lo2:hi2 + 1]))
        ns = hi2 - lo2 + 1
        if ns < min_scans:
            continue
        peaks.append({"apex_i": int(apex), "apex_rt": t_ap, "apex_sigma": s_ap, "height": hh, "area": area, "lo": int(lo2),
                      "hi": int(hi2), "fwhm_min": fwhm, "fwhm_scans": fwhm / dt if dt > 0 else 0.0, "n_scans": int(ns),
                      "snr": float(rsn), "reliable": bool(ns >= MIN_SCANS_RELIABLE and (fwhm / dt if dt > 0 else 0) >= 3.0)})
    peaks.sort(key=lambda d: -d["height"])
    out["peaks"] = peaks
    return out


# ----------------------------------------------------------------------------------------------------------------------
# small statistics (numpy only)
# ----------------------------------------------------------------------------------------------------------------------
def rankdata(a) -> np.ndarray:
    """Average ranks (ties share the mean rank), 1-based."""
    a = np.asarray(a, float)
    order = np.argsort(a, kind="mergesort")
    r = np.empty(len(a))
    sa = a[order]
    i = 0
    while i < len(a):
        j = i
        while j + 1 < len(a) and sa[j + 1] == sa[i]:
            j += 1
        r[order[i:j + 1]] = (i + j) / 2.0 + 1
        i = j + 1
    return r


def wpearson(x, y, w=None) -> float:
    """Weighted Pearson correlation (NaN if a trace is constant, also up to rounding noise)."""
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) < 3:
        return float("nan")
    w = np.ones(len(x)) if w is None else np.asarray(w, float)
    sw = w.sum()
    if sw <= 0:
        return float("nan")
    mx, my = (w * x).sum() / sw, (w * y).sum() / sw
    cx, cy = x - mx, y - my
    vx, vy = (w * cx * cx).sum(), (w * cy * cy).sum()
    if vx <= 1e-300 or vy <= 1e-300 or vx <= 1e-24 * sw * mx * mx or vy <= 1e-24 * sw * my * my:
        return float("nan")                              # constant up to rounding noise (the sums differ by an ulp between numpy builds)
    return float((w * cx * cy).sum() / math.sqrt(vx * vy))


def wspearman(x, y, w=None) -> float:
    return wpearson(rankdata(x), rankdata(y), w)


def cosine(x, y) -> float:
    x, y = np.asarray(x, float), np.asarray(y, float)
    d = float(np.linalg.norm(x) * np.linalg.norm(y))
    return float(x @ y / d) if d > 0 else float("nan")


def kendall_tau(x, y) -> float:
    """Kendall tau-b (ties corrected)."""
    x, y = np.asarray(x, float), np.asarray(y, float)
    n = len(x)
    if n < 2:
        return float("nan")
    sx, sy = np.sign(x[:, None] - x[None, :]), np.sign(y[:, None] - y[None, :])
    iu = np.triu_indices(n, 1)
    c = float((sx * sy)[iu].sum())
    tx, ty = float((sx[iu] != 0).sum()), float((sy[iu] != 0).sum())
    d = math.sqrt(tx * ty)
    return c / d if d > 0 else float("nan")


def theil_sen(x, y, max_pts: int = 80):
    """Theil-Sen line (median of pairwise slopes, median intercept): (slope, intercept). Thinned deterministically above max_pts."""
    x, y = np.asarray(x, float), np.asarray(y, float)
    if len(x) > max_pts:
        idx = np.linspace(0, len(x) - 1, max_pts).astype(int)
        x, y = x[idx], y[idx]
    iu = np.triu_indices(len(x), 1)
    dx = (x[:, None] - x[None, :])[iu]
    dy = (y[:, None] - y[None, :])[iu]
    ok = np.abs(dx) > 1e-12 * max(1.0, float(np.abs(x).max()) if len(x) else 1.0)
    if not ok.any():
        return float("nan"), float("nan")
    s = float(np.median(dy[ok] / dx[ok]))
    return s, float(np.median(y - s * x))


def _gammainc_q(a: float, x: float) -> float:
    """Regularised upper incomplete gamma Q(a, x) (series / continued fraction): chi-square survival = Q(df/2, chi2/2)."""
    if x <= 0:
        return 1.0
    if x < a + 1:
        ap, s, d = a, 1.0 / a, 1.0 / a
        for _ in range(500):
            ap += 1
            d *= x / ap
            s += d
            if abs(d) < abs(s) * 1e-14:
                break
        return float(max(0.0, 1.0 - s * math.exp(-x + a * math.log(x) - math.lgamma(a))))
    b = x + 1 - a
    c = 1e300
    d = 1.0 / b
    h = d
    for i in range(1, 500):
        an = -i * (i - a)
        b += 2
        d = an * d + b
        d = 1e-300 if abs(d) < 1e-300 else d
        c = b + an / c
        c = 1e-300 if abs(c) < 1e-300 else c
        d = 1.0 / d
        de = d * c
        h *= de
        if abs(de - 1) < 1e-14:
            break
    return float(min(1.0, math.exp(-x + a * math.log(x) - math.lgamma(a)) * h))


def chi2_sf(x: float, df: int) -> float:
    return _gammainc_q(df / 2.0, x / 2.0)


def perm_pvalue(stat, x, y, n_perm: int = 4000, seed: int = 0) -> float:
    """Two-sided permutation p-value of an association statistic(x, y); exact for n <= 7, Monte Carlo above."""
    import itertools
    x, y = np.asarray(x, float), np.asarray(y, float)
    n = len(x)
    obs = abs(stat(x, y))
    if not math.isfinite(obs):
        return float("nan")
    if n <= 7:
        perms = list(itertools.permutations(range(n)))
    else:
        rng = np.random.default_rng(seed)
        perms = [rng.permutation(n) for _ in range(n_perm)]
    hit = sum(1 for p in perms if abs(stat(x, y[list(p)])) >= obs - 1e-12)
    return hit / len(perms) if n <= 7 else (hit + 1) / (len(perms) + 1)


# ----------------------------------------------------------------------------------------------------------------------
# 3. profile similarity
# ----------------------------------------------------------------------------------------------------------------------
def _prep(rt, y, smooth: int = 5):
    """baseline-corrected, smoothed copy of a trace + noise level."""
    y = np.asarray(y, float)
    base = asls_baseline(y) if len(y) >= 7 else np.zeros(len(y))
    yc = np.maximum(y - base, 0.0)
    return yc, np.maximum(savgol(yc, smooth, 2), 0.0), noise_level(yc)


def _pick_peak(det, near_i=None, tol_scans=None):
    """The peak of a detection result closest to `near_i` (within tol_scans) or the highest one."""
    ps = det["peaks"]
    if not ps:
        return None
    if near_i is None:
        return ps[0]
    best = min(ps, key=lambda p: abs(p["apex_i"] - near_i))
    if tol_scans is not None and abs(best["apex_i"] - near_i) > tol_scans:
        return None
    return best


def profile_similarity(rt, x, p, p_apex_rt: float | None = None, n_boot: int = 200, n_shift: int = 400, seed: int = 0) -> dict:
    """All the profile-similarity evidence between the trace of an ion X and the trace of a candidate precursor P (same RT axis).

    The correlations are computed inside the peak window (union of the limits of P's peak and of X's own peak, if it has one),
    never on the whole chromatogram: zeros outside a peak inflate Pearson. Returned (floats; NaN -> None after to_jsonable):
      pearson, pearson_w (weights = mean normalised intensity), spearman, spearman_w, deriv_corr (first derivative), cosine,
      apex_diff_s (+- apex_diff_sigma_s), lag_best (scans) and xcorr_best (cross-correlation at lags -2..2), xcorr_lags,
      fwhm_ratio (X/P), p_shift (how special is the alignment: fraction of time-shifted copies of X that correlate at least as well
      with P, so autocorrelation of the smooth peaks is accounted for), corr_ci (block bootstrap 95% interval of Pearson),
      n_scans, reliable, x_has_peak, warnings (list of messages {key, params} for the student tool)."""
    rt, x, p = np.asarray(rt, float), np.asarray(x, float), np.asarray(p, float)
    n = len(rt)
    res = {"n_scans": 0, "reliable": False, "x_has_peak": False, "warnings": []}
    if n < 7:
        res["warnings"].append(message("of.warn.shortWindow"))
        return res
    pc, ps, pnoise = _prep(rt, p)
    xc, xs, xnoise = _prep(rt, x)
    detp, detx = detect_peaks(rt, p), detect_peaks(rt, x)
    near = int(np.argmin(np.abs(rt - p_apex_rt))) if p_apex_rt is not None else None
    pk = _pick_peak(detp, near)
    if pk is None:
        res["warnings"].append(message("of.warn.noParentPeak"))
        return res
    xk = _pick_peak(detx, pk["apex_i"], max(3, int(pk["fwhm_scans"] * 2)))
    res["x_has_peak"] = xk is not None
    lo, hi = pk["lo"], pk["hi"]
    if xk is not None:
        lo, hi = min(lo, xk["lo"]), max(hi, xk["hi"])
    sl = slice(lo, hi + 1)
    nw = hi - lo + 1
    res["n_scans"], res["window"] = nw, [float(rt[lo]), float(rt[hi])]
    if nw < MIN_SCANS_RELIABLE:
        res["warnings"].append(message("of.warn.fewScans", n=nw))
    if xk is None:
        res["warnings"].append(message("of.warn.ionNoPeak"))
    xw, pw = xc[sl], pc[sl]
    xsw, psw = xs[sl], ps[sl]
    wts = 0.5 * (xsw / max(xsw.max(), 1e-12) + psw / max(psw.max(), 1e-12)) + 1e-3
    res["pearson"] = wpearson(xw, pw)
    res["pearson_w"] = wpearson(xw, pw, wts)
    res["spearman"] = wspearman(xw, pw)
    res["spearman_w"] = wspearman(xw, pw, wts)
    dxs, dps = savgol(xc, 7, 2, 1)[sl], savgol(pc, 7, 2, 1)[sl]
    res["deriv_corr"] = wpearson(dxs, dps)
    res["cosine"] = cosine(xw, pw)
    # apex shift in seconds
    if xk is not None:
        d = (xk["apex_rt"] - pk["apex_rt"]) * 60.0
        s = math.hypot(xk["apex_sigma"], pk["apex_sigma"]) * 60.0
        res["apex_diff_s"], res["apex_diff_sigma_s"] = float(d), float(s)
        res["fwhm_ratio"] = float(xk["fwhm_min"] / pk["fwhm_min"]) if pk["fwhm_min"] > 0 else float("nan")
        res["x_snr"] = xk["snr"]
    else:
        res["apex_diff_s"] = res["apex_diff_sigma_s"] = res["fwhm_ratio"] = float("nan")
        res["x_snr"] = 0.0
    res["p_apex_rt"] = pk["apex_rt"]
    # cross-correlation at lags -2..2 (positive lag: X later than P)
    lags = list(range(-2, 3))
    xcs = []
    for L in lags:
        xs_l = np.roll(xc, -L)[sl]
        xcs.append(wpearson(xs_l, pw))
    xa = np.array([c if math.isfinite(c) else -2 for c in xcs])
    res["xcorr_lags"] = lags
    res["xcorr"] = xcs
    res["lag_best"] = int(lags[int(np.argmax(xa))])
    res["xcorr_best"] = float(xa.max()) if xa.max() > -2 else float("nan")
    # shift-surrogate p-value
    rng = np.random.default_rng(seed)
    obs = res["pearson"]
    if math.isfinite(obs):
        L = n // 2
        sh = [s_ for s_ in range(-L, L + 1) if abs(s_) >= 2]
        if len(sh) > n_shift:
            sh = list(rng.choice(sh, n_shift, replace=False))
        cnt = 0
        used = 0
        for s_ in sh:
            c = wpearson(np.roll(xc, s_)[sl], pw)
            if math.isfinite(c):
                used += 1
                cnt += c >= obs - 1e-12
        res["p_shift"] = float((1 + cnt) / (1 + used)) if used else float("nan")
    else:
        res["p_shift"] = float("nan")
    # block bootstrap CI of Pearson
    if nw >= 8 and math.isfinite(obs):
        bl = int(max(3, round(pk["fwhm_scans"] / 2))) if pk["fwhm_scans"] > 0 else 3
        nb = int(math.ceil(nw / bl))
        cs = []
        for _ in range(n_boot):
            st = rng.integers(0, max(nw - bl, 1), nb)
            idx = np.concatenate([np.arange(a, min(a + bl, nw)) for a in st])[:nw]
            c = wpearson(xw[idx], pw[idx])
            if math.isfinite(c):
                cs.append(c)
        res["corr_ci"] = [float(np.percentile(cs, 2.5)), float(np.percentile(cs, 97.5))] if len(cs) > 10 else [float("nan")] * 2
    else:
        res["corr_ci"] = [float("nan")] * 2
    res["reliable"] = bool(nw >= MIN_SCANS_RELIABLE and xk is not None and pk["reliable"])
    res["p_peak"] = {k: pk[k] for k in ("apex_rt", "apex_sigma", "height", "area", "lo", "hi", "fwhm_min", "fwhm_scans", "n_scans")}
    if xk is not None:
        res["x_peak"] = {k: xk[k] for k in ("apex_rt", "apex_sigma", "height", "area", "lo", "hi", "fwhm_min", "fwhm_scans", "n_scans", "snr")}
    return res


# ----------------------------------------------------------------------------------------------------------------------
# 4. ratio F/P
# ----------------------------------------------------------------------------------------------------------------------
def _deming_origin(P, F, sP, sF):
    """Slope r of F = r P through the origin minimising sum (F - rP)^2 / (sF^2 + r^2 sP^2) (weighted total least squares)."""
    r0 = float((F * P).sum() / max((P * P).sum(), 1e-300))
    if r0 <= 0:
        return 0.0
    grid = r0 * np.geomspace(1 / 60.0, 60.0, 240)
    best = None
    for _ in range(2):
        r = grid[:, None]
        S = (((F[None, :] - r * P[None, :]) ** 2) / (sF[None, :] ** 2 + r ** 2 * sP[None, :] ** 2)).sum(1)
        k = int(np.argmin(S))
        best = float(grid[k])
        lo, hi = grid[max(k - 1, 0)], grid[min(k + 1, len(grid) - 1)]
        grid = np.linspace(lo, hi, 120)
    return best


def ratio_fit(rt, x, p, window, cv: float = 0.1, min_rel: float = 0.2, n_boot: int = 200, seed: int = 0) -> dict:
    """Fit F = r P inside the peak window [i_lo, i_hi] (indices of rt) with the fragment trace x and the precursor trace p.

    Points with p below `min_rel` of its apex are ignored (the foot has no information on the ratio). Estimators:
      r_tls   weighted total least squares through the origin (errors in both; sigma_i^2 = noise^2 + (cv*y_i)^2),
      r_ts / intercept_ts   Theil-Sen line with intercept (robust; ISF has intercept ~ 0),
      r_median   median of the per-scan ratios, cv_ratio their coefficient of variation (MAD-based and classical),
      r2   R^2 of F against r_tls * P, resid_rel (RMS residual / apex of F), resid_acf1 (lag-1 autocorrelation of the residuals:
      a systematic pattern = the shapes differ), r_se (block bootstrap)."""
    i0, i1 = int(window[0]), int(window[1])
    xc, _, nx = _prep(rt, x)
    pc, _, npn = _prep(rt, p)
    P, F = pc[i0:i1 + 1], xc[i0:i1 + 1]
    out = {"ok": False}
    if len(P) < 4 or P.max() <= 0:
        return out
    m = P >= min_rel * P.max()
    if m.sum() < 4:
        return out
    P, F = P[m], F[m]
    sP = np.sqrt(npn ** 2 + (cv * P) ** 2)
    sF = np.sqrt(nx ** 2 + (cv * F) ** 2)
    r = _deming_origin(P, F, sP, sF)
    ts, ic = theil_sen(P, F)
    with np.errstate(divide="ignore", invalid="ignore"):
        ratios = F / P
    med = float(np.median(ratios))
    mad = float(1.4826 * np.median(np.abs(ratios - med)))
    fit = r * P
    ss_res = float(((F - fit) ** 2).sum())
    ss_tot = float(((F - F.mean()) ** 2).sum())
    resid = F - fit
    acf1 = wpearson(resid[:-1], resid[1:]) if len(resid) > 4 else float("nan")
    rng = np.random.default_rng(seed)
    bl = max(2, len(P) // 5)
    rs = []
    for _ in range(n_boot):
        nb = int(math.ceil(len(P) / bl))
        st = rng.integers(0, max(len(P) - bl, 1), nb)
        idx = np.concatenate([np.arange(a, min(a + bl, len(P))) for a in st])[:len(P)]
        rs.append(_deming_origin(P[idx], F[idx], sP[idx], sF[idx]))
    out.update(ok=True, n=int(len(P)), r_tls=float(r), r_se=float(np.std(rs)), r_ts=ts, intercept_ts=ic,
               intercept_rel=float(ic / (r * P.max())) if r * P.max() > 0 else float("nan"), r_median=med,
               cv_ratio=float(ratios.std() / abs(ratios.mean())) if ratios.mean() != 0 else float("nan"),
               cv_ratio_mad=float(mad / abs(med)) if med != 0 else float("nan"),
               r2=float(1 - ss_res / ss_tot) if ss_tot > 0 else float("nan"),
               resid_rel=float(math.sqrt(ss_res / len(P)) / max(F.max(), 1e-12)), resid_acf1=acf1,
               points={"p": P.tolist(), "f": F.tolist(), "fit": fit.tolist()})
    return out


def ratio_constancy(ratios, ses=None, labels=None) -> dict:
    """Is the ratio F/P the same in every sample? ratios = r per sample (from `ratio_fit`), ses = their standard errors.
    Cochran Q = sum ((r_k - r_w) / se_k)^2 ~ chi2(K-1) when the ratio is constant; also CV and relative range between samples.
    ISF -> constant r; a product that co-elutes with the parent -> r changes with the treatment time."""
    r = np.asarray([v for v in ratios], float)
    ok = np.isfinite(r)
    out = {"k": int(ok.sum())}
    if ok.sum() < 2:
        return {**out, "q": float("nan"), "p": float("nan"), "cv": float("nan"), "range_rel": float("nan")}
    r = r[ok]
    if ses is None:
        se = np.full(len(r), max(float(r.mean()) * 0.05, 1e-12))
    else:
        se = np.asarray(ses, float)[ok]
        se = np.maximum(se, 1e-3 * np.maximum(np.abs(r), 1e-12))
    w = 1.0 / se ** 2
    rw = float((w * r).sum() / w.sum())
    q = float((w * (r - rw) ** 2).sum())
    out.update(r_weighted=rw, q=q, df=int(len(r) - 1), p=chi2_sf(q, len(r) - 1), cv=float(r.std(ddof=1) / abs(r.mean())) if r.mean() else float("nan"),
               range_rel=float((r.max() - r.min()) / abs(r.mean())) if r.mean() else float("nan"), ratios=r.tolist())
    if labels is not None:
        out["labels"] = [l for l, o in zip(labels, ok) if o]
    return out


# ----------------------------------------------------------------------------------------------------------------------
# 5. kinetics between samples
# ----------------------------------------------------------------------------------------------------------------------
def peak_area(rt, y, i0: int, i1: int) -> float:
    """Baseline-corrected trapezoid area (intensity x minute) of y over the index range [i0, i1]."""
    yc, _, _ = _prep(rt, y)
    return float(_trapz(yc[i0:i1 + 1], rt[i0:i1 + 1]))


def fit_decay(t, y, n_boot: int = 120, seed: int = 0) -> dict:
    """First-order decay y = A exp(-k t) (k >= 0) with residual-bootstrap 95% intervals (reported, but with 3-6 points they are
    wide and optimistic: `reliable` is False below 5 points)."""
    t, y = np.asarray(t, float), np.asarray(y, float)
    def solve(yy):
        span = max(float(t.max() - t.min()), 1e-9)
        ks = np.concatenate([[0.0], np.geomspace(1e-3 / span, 50.0 / span, 160)])
        for _ in range(2):
            E = np.exp(-np.outer(ks, t))
            A = (E * yy).sum(1) / np.maximum((E * E).sum(1), 1e-300)
            S = ((yy[None, :] - A[:, None] * E) ** 2).sum(1)
            i = int(np.argmin(S))
            ks = np.linspace(ks[max(i - 1, 0)], ks[min(i + 1, len(ks) - 1)], 60)
            ks = ks[ks >= 0]
            if len(ks) == 0:
                ks = np.array([0.0])
        E = np.exp(-np.outer(ks, t))
        A = (E * yy).sum(1) / np.maximum((E * E).sum(1), 1e-300)
        S = ((yy[None, :] - A[:, None] * E) ** 2).sum(1)
        i = int(np.argmin(S))
        return float(A[i]), float(ks[i]), float(S[i])
    A, k, sse = solve(y)
    out = {"A": A, "k": k, "sse": sse, "half_life": float(math.log(2) / k) if k > 0 else float("inf"), "n": int(len(t)),
           "reliable": bool(len(t) >= 5)}
    res = y - A * np.exp(-k * t)
    rng = np.random.default_rng(seed)
    ks_ = []
    for _ in range(n_boot):
        yb = A * np.exp(-k * t) + rng.choice(res, len(res), replace=True)
        ks_.append(solve(yb)[1])
    out["k_ci"] = [float(np.percentile(ks_, 2.5)), float(np.percentile(ks_, 97.5))]
    return out


def _consecutive_shape(t, k1, k2):
    t = np.asarray(t, float)
    k2 = np.asarray(k2, float)
    if k2.ndim == 0:
        k2 = k2[None]
    d = k2[:, None] - k1
    with np.errstate(divide="ignore", invalid="ignore"):
        g = k1 / d * (np.exp(-k1 * t)[None, :] - np.exp(-k2[:, None] * t[None, :]))
    near = np.abs(d[:, 0]) < 1e-6 * max(k1, 1e-9)
    g[near] = (k1 * t * np.exp(-k1 * t))[None, :]
    return g


def fit_consecutive(t, y, k1: float, n_boot: int = 300, seed: int = 0) -> dict:
    """Formation-decay of an intermediate B in A -> B -> C: y = s * k1/(k2-k1) (e^{-k1 t} - e^{-k2 t}), k1 known (parent decay),
    s (scale) and k2 free; grid on k2, closed-form s. Residual bootstrap for the intervals."""
    t, y = np.asarray(t, float), np.asarray(y, float)
    span = max(float(t.max() - t.min()), 1e-9)
    k2s = np.geomspace(1e-3 / span, 50.0 / span, 500)
    def solve(yy):
        G = _consecutive_shape(t, k1, k2s)
        s = (G * yy).sum(1) / np.maximum((G * G).sum(1), 1e-300)
        s = np.maximum(s, 0)
        S = ((yy[None, :] - s[:, None] * G) ** 2).sum(1)
        i = int(np.argmin(S))
        return float(s[i]), float(k2s[i]), float(S[i])
    s, k2, sse = solve(y)
    res = y - s * _consecutive_shape(t, k1, k2)[0]
    rng = np.random.default_rng(seed)
    kk = []
    for _ in range(n_boot):
        yb = s * _consecutive_shape(t, k1, k2)[0] + rng.choice(res, len(res), replace=True)
        kk.append(solve(yb)[1])
    return {"s": s, "k2": k2, "sse": sse, "k2_ci": [float(np.percentile(kk, 2.5)), float(np.percentile(kk, 97.5))], "n": int(len(t)),
            "t_max": float(math.log(k2 / k1) / (k2 - k1)) if k2 > 0 and abs(k2 - k1) > 1e-9 else float(1 / k1)}


def _aicc(sse, n, k):
    if n - k - 1 <= 0 or sse <= 0:
        return float("nan")
    return float(n * math.log(sse / n) + 2 * k + 2.0 * k * (k + 1) / (n - k - 1))


def kinetics(times, parent_area, ion_area, labels=None) -> dict:
    """Behaviour of an ion across samples at different treatment times, compared with the parent. Few points (3-6): everything
    comes with its uncertainty and a warning. Returned:
      spearman / kendall vs the parent (and exact-permutation p), spearman_time (monotone trend with time), slope_sign,
      ratio per sample (ion/parent), parent first-order decay fit, models for the ion: 'tracks_parent' (c * parent), 'decay'
      (A exp(-k t)), 'formation_decay' (A -> B -> C with the k of the parent) with SSE and AICc."""
    t = np.asarray(times, float)
    P = np.asarray(parent_area, float)
    X = np.asarray(ion_area, float)
    o = np.argsort(t)
    t, P, X = t[o], P[o], X[o]
    n = len(t)
    out = {"n": int(n), "warnings": []}
    if n < 3:
        out["warnings"].append(message("of.warn.fewSamples3"))
        return out
    if n < 5:
        out["warnings"].append(message("of.warn.fewSamplesCi", n=n))
    pn, xn = P / max(P.max(), 1e-12), X / max(X.max(), 1e-12)
    out["times"], out["parent_norm"], out["ion_norm"] = t.tolist(), pn.tolist(), xn.tolist()
    with np.errstate(divide="ignore", invalid="ignore"):
        out["ratio"] = np.where(P > 0, X / P, np.nan).tolist()
    out["spearman_parent"] = wspearman(X, P)
    out["kendall_parent"] = kendall_tau(X, P)
    out["p_spearman_parent"] = perm_pvalue(wspearman, X, P)
    out["spearman_time"] = wspearman(t, X)
    out["kendall_time"] = kendall_tau(t, X)
    sl, _ = theil_sen(t, xn)
    out["slope_norm_per_min"] = sl
    out["slope_sign"] = int(np.sign(sl)) if math.isfinite(sl) else 0
    dp = fit_decay(t, pn)
    out["parent_decay"] = dp
    k1 = dp["k"]
    models = {}
    c = float((X * P).sum() / max((P * P).sum(), 1e-300))
    sse = float(((X - c * P) ** 2).sum())
    models["tracks_parent"] = {"c": c, "sse": sse, "n_par": 1, "aicc": _aicc(sse, n, 1)}
    de = fit_decay(t, X)
    models["decay"] = {"A": de["A"], "k": de["k"], "k_ci": de["k_ci"], "sse": de["sse"], "n_par": 2, "aicc": _aicc(de["sse"], n, 2)}
    if k1 > 0:
        fc = fit_consecutive(t, X, k1)
        models["formation_decay"] = {**fc, "n_par": 2, "aicc": _aicc(fc["sse"], n, 2)}
    out["models"] = models
    return out


# ----------------------------------------------------------------------------------------------------------------------
# 6. ion families, roles, formula constraints
# ----------------------------------------------------------------------------------------------------------------------
def _formula(f):
    from .chem.elements import parse_formula
    return parse_formula(f) if isinstance(f, str) else dict(f)


_MASS_CACHE: dict = {}


def _mass(f) -> float:
    if isinstance(f, str) and f in _MASS_CACHE:        # the neutral-loss table is small and queried hundreds of times per report
        return _MASS_CACHE[f]
    from .chem.elements import mass
    m = mass(_formula(f))
    if isinstance(f, str):
        _MASS_CACHE[f] = m
    return m


def isotope_pattern(formula, charge_h: int = 1, n: int = 5) -> np.ndarray:
    """Relative isotope pattern (M, M+1, M+2, ...) at unit resolution of a neutral formula + `charge_h` protons (normalised to M=1
    would hide a monoisotopic peak that is not the lightest: the pattern is normalised to its maximum)."""
    cnt = _formula(formula)
    cnt["H"] = cnt.get("H", 0) + charge_h
    pat = np.array([1.0])
    for el, k in cnt.items():
        iso = _ISO.get(el)
        if iso is None or k <= 0:
            continue
        single = np.zeros(max(s for s, _ in iso) + 1)
        for s, a in iso:
            single[s] += a
        single /= single.sum()
        acc = np.array([1.0])
        base = single
        e = int(k)
        while e:
            if e & 1:
                acc = np.convolve(acc, base)[: n + 3]
            base = np.convolve(base, base)[: n + 3]
            e >>= 1
        pat = np.convolve(pat, acc)[: n + 3]
    pat = pat[:n]
    return pat / pat.max()


def loss_candidates(delta: float, tol: float = 0.5, max_combo: int = 2):
    """Neutral losses (single or a combination of up to `max_combo` of the table) whose mass is within tol of delta (Da)."""
    from itertools import combinations_with_replacement
    out = []
    for k in range(1, max_combo + 1):
        for combo in combinations_with_replacement(NEUTRAL_LOSSES, k):
            m = sum(_mass(c) for c in combo)
            if abs(m - delta) <= tol:
                out.append({"loss": "+".join(combo), "mass": float(m), "err": float(delta - m)})
    out.sort(key=lambda d: (abs(d["err"]), d["loss"].count("+")))
    return out


def _subformula(parent, loss_parts, h_slack: int = 1):
    """Fragment formula = parent - losses (as a dict) if it is a chemical subset of the parent (H may be exchanged by +-h_slack);
    returns (ok, fragment formula dict)."""
    fp = _formula(parent)
    frag = dict(fp)
    for part in loss_parts:
        for el, k in _formula(part).items():
            frag[el] = frag.get(el, 0) - k
    ok = all(v >= 0 for el, v in frag.items() if el != "H") and frag.get("H", 0) >= -h_slack
    frag = {k: v for k, v in frag.items() if v > 0}
    return bool(ok), frag


def annotate_roles(mz_x: float, mz_p: float, formula_p: str | None = None, tol: float = 0.5) -> list[dict]:
    """Hypotheses on the relation between an ion X and a candidate precursor P (assumed [M+H]+), by m/z difference only; each is a
    hint, never a conclusion. Roles: isotope (X = P+1, +2, +3 or P = X+1..), adduct ([M+NH4]+, [M+Na]+, [M+K]+, [M+H+ACN]+, [2M+H]+,
    [2M+Na]+), fragment_candidate (P - X = known neutral loss, up to two), dimer/cluster (via [2M+H]+). With the formula of P the
    fragment candidates also get `formula_ok` (the fragment formula must be a subset of the precursor formula, H transfers allowed)
    and `fragment_formula`. The difference is used, not the absolute m/z: the instrument offset (~+0.3 Da) cancels."""
    out = []
    d = mz_x - mz_p
    for k in (1, 2, 3):
        if abs(d - k * 1.003355) <= tol:
            out.append({"role": "isotope", "label": "M+%d of P" % k, "label_key": "of.role.isoP", "params": {"k": k}, "delta_obs": d, "delta_exp": k * 1.003355, "err": d - k * 1.003355})
        if abs(-d - k * 1.003355) <= tol:
            out.append({"role": "isotope", "label": "X is M+%d (P is the isotope of X)" % k, "label_key": "of.role.isoX", "params": {"k": k}, "delta_obs": d, "delta_exp": -k * 1.003355, "err": d + k * 1.003355})
    M = mz_p - PROTON
    for name, shift, mult in ADDUCTS:
        if name == "[M+H]+":
            continue
        exp = mult * M + shift - (M + PROTON)
        if abs(d - exp) <= tol:
            role = "dimer" if mult == 2 else "adduct"
            out.append({"role": role, "label": name, "delta_obs": d, "delta_exp": exp, "err": d - exp})
    if d < -0.5:
        for c in loss_candidates(-d, tol):
            row = {"role": "fragment_candidate", "label": "P - " + c["loss"], "delta_obs": d, "delta_exp": -c["mass"], "err": d + c["mass"]}
            if formula_p:
                ok, frag = _subformula(formula_p, c["loss"].split("+"))
                from .chem.elements import fmt
                row["formula_ok"], row["fragment_formula"] = ok, fmt(frag)
            out.append(row)
    return out


def isotope_check(w: Window, mz: float, i0: int, i1: int, formula: str | None = None, tol: float = 0.5) -> dict:
    """Observed (M+1)/M and (M+2)/M of the ion at m/z `mz` from the areas of the ROIs at +1 and +2 in [i0, i1], with the pattern
    expected for `formula` (+H) when given: a fragment that kept a Cl must show M+2/M near 0.32 (one Cl), a fragment without Cl ~0.05."""
    def area(m):
        if len(w.mz) == 0:
            return 0.0
        j = int(np.argmin(np.abs(w.mz - m)))
        return float(w.X[j, i0:i1 + 1].sum()) if abs(w.mz[j] - m) <= tol else 0.0
    a0, a1, a2 = area(mz), area(mz + 1.003355), area(mz + 2.00671)
    out = {"obs_m1": a1 / a0 if a0 > 0 else float("nan"), "obs_m2": a2 / a0 if a0 > 0 else float("nan")}
    if formula:
        pat = isotope_pattern(formula)
        out.update(exp_m1=float(pat[1] / pat[0]), exp_m2=float(pat[2] / pat[0]))
    return out


def _pairwise_windowed_corr(Y: np.ndarray, thr: float = 0.05, min_overlap: int = 4, chunk: int = 16) -> np.ndarray:
    """Pearson correlation of every pair of traces (rows of Y, each normalised to its maximum) over the scans where at least one of
    the two is above `thr` (the shared peak region), never over the zeros. NaN where the region has fewer than min_overlap scans."""
    m, n = Y.shape
    Rm = np.full((m, m), np.nan)
    A = Y / np.maximum(Y.max(1, keepdims=True), 1e-300)
    for a in range(0, m, chunk):
        b = min(a + chunk, m)
        Ma = (A[a:b, None, :] >= thr) | (A[None, :, :] >= thr)           # (c, m, n)
        cnt = Ma.sum(2).astype(float)
        x = A[a:b, None, :]
        y = A[None, :, :]
        sx = (Ma * x).sum(2)
        sy = (Ma * y).sum(2)
        sxx = (Ma * x * x).sum(2)
        syy = (Ma * y * y).sum(2)
        sxy = (Ma * x * y).sum(2)
        with np.errstate(divide="ignore", invalid="ignore"):
            cov = sxy - sx * sy / cnt
            vx = sxx - sx * sx / cnt
            vy = syy - sy * sy / cnt
            r = cov / np.sqrt(vx * vy)
        r[(cnt < min_overlap) | ~np.isfinite(r)] = np.nan
        Rm[a:b] = r
    return Rm


def _average_linkage(D: np.ndarray, cut: float):
    """Agglomerative clustering with average linkage on a distance matrix until the closest pair is above `cut`. Returns the
    labels (0..k-1) and the merge heights."""
    m = len(D)
    D = D.astype(float).copy()
    np.fill_diagonal(D, np.inf)
    members = {i: [i] for i in range(m)}
    active = list(range(m))
    heights = []
    while len(active) > 1:
        sub = D[np.ix_(active, active)]
        k = int(np.argmin(sub))
        i, j = divmod(k, len(active))
        d = sub[i, j]
        if not np.isfinite(d) or d > cut:
            break
        a, b = active[i], active[j]
        na, nb = len(members[a]), len(members[b])
        newrow = (D[a] * na + D[b] * nb) / (na + nb)
        D[a, :] = newrow
        D[:, a] = newrow
        D[a, a] = np.inf
        D[b, :] = np.inf
        D[:, b] = np.inf
        members[a] += members.pop(b)
        active.remove(b)
        heights.append(float(d))
    labels = np.empty(m, dtype=int)
    for c, a in enumerate(active):
        labels[members[a]] = c
    return labels, heights


def ion_families(w: Window, min_snr: float = 10.0, kin: np.ndarray | None = None, weights=(1.0, 0.6, 0.6), cut: float = 0.3,
                 max_ions: int = 400) -> dict:
    """Group the ROIs of a window into families (ions that look like one species: same profile, same apex, same kinetics).

    Distance between ions i, j = weighted mean of  d_profile = (1 - r_window)/2  (Pearson over the shared peak region),
    d_apex = min(|apex_i - apex_j| / 4 scans, 1)  and, if `kin` (ROI x sample matrix of areas, same row order as w.mz) is given,
    d_kin = (1 - Spearman)/2 between the kinetic profiles. Average linkage, cut at `cut`. Only ROIs with signal >= min_snr times
    the noise are used. Returns {"ions": [...], "clusters": [{"members": [...], "core": index, "apex_rt": ...}], "dist": matrix}.
    A family is a hypothesis of common origin, not a proof: coeluting different compounds can look alike at unit resolution."""
    m0, n = w.X.shape
    out = {"ions": [], "clusters": [], "dist": None}
    if m0 == 0:
        return out
    ys, apex, snr = np.zeros((m0, n)), np.zeros(m0, dtype=int), np.zeros(m0)
    for j in range(m0):
        yc, sm, nz = _prep(w.rt, w.X[j])
        ys[j] = sm
        apex[j] = int(np.argmax(sm))
        snr[j] = sm.max() / max(nz, 1e-9)
        seg = w.X[j, max(apex[j] - 3, 0):apex[j] + 4]
        if (seg > 0).sum() < min(5, len(seg)):          # isolated spikes (chemical noise), not an elution profile
            snr[j] = 0.0
    keep = np.flatnonzero(snr >= min_snr)
    if len(keep) > max_ions:
        keep = keep[np.argsort(-ys[keep].max(1))[:max_ions]]
        keep.sort()
    if len(keep) == 0:
        return out
    Y = ys[keep]
    R = _pairwise_windowed_corr(Y)
    dprof = np.where(np.isfinite(R), (1 - R) / 2.0, 1.0)
    ap = apex[keep].astype(float)
    dapex = np.minimum(np.abs(ap[:, None] - ap[None, :]) / 4.0, 1.0)
    wc, wa, wk = weights
    D = wc * dprof + wa * dapex
    wsum = wc + wa
    if kin is not None:
        kk = np.asarray(kin, float)[keep]
        rk = np.array([[wspearman(kk[i], kk[j]) if i != j else 1.0 for j in range(len(kk))] for i in range(len(kk))])
        D = D + wk * np.where(np.isfinite(rk), (1 - rk) / 2.0, 0.5)
        wsum += wk
    D = D / wsum
    np.fill_diagonal(D, 0.0)
    labels, heights = _average_linkage(D, cut)
    for c in range(labels.max() + 1):
        idx = np.flatnonzero(labels == c)
        core = int(idx[np.argmax(Y[idx].max(1))])
        out["clusters"].append({"members": [int(keep[i]) for i in idx], "core": int(keep[core]), "apex_rt": float(w.rt[int(ap[core])]),
                                 "mz": [float(w.mz[keep[i]]) for i in idx], "height": float(Y[core].max())})
    out["clusters"].sort(key=lambda d: -d["height"])
    out["ions"] = [{"roi": int(keep[i]), "mz": float(w.mz[keep[i]]), "apex_rt": float(w.rt[int(ap[i])]), "snr": float(snr[keep[i]]),
                    "cluster": int(labels[i])} for i in range(len(keep))]
    out["dist"] = D
    out["cut"] = cut
    return out


# ----------------------------------------------------------------------------------------------------------------------
# 7. deconvolution: MCR-ALS and NMF
# ----------------------------------------------------------------------------------------------------------------------
def estimate_components(D: np.ndarray, max_k: int = 4, min_var: float = 0.002) -> dict:
    """Number of chemical components of the matrix D (scans x channels) from the singular values against the noise: the noise sigma
    comes from the first differences of the channels along time (MAD); a component counts when its singular value exceeds the
    Marchenko-Pastur edge sigma (sqrt(n) + sqrt(m)) by 30%. Also the evolving factor analysis (EFA): forward singular values of
    expanding windows, so that the scan where each component starts to appear is reported (`efa_start`)."""
    n, m = D.shape
    out = {"k": 1, "sv": [], "efa_start": []}
    if n < 4 or m < 2:
        return out
    sv = np.linalg.svd(D, compute_uv=False)
    sig = float(1.4826 * np.median(np.abs(np.diff(D, axis=0) - np.median(np.diff(D, axis=0)))) / math.sqrt(2))
    edge = sig * (math.sqrt(n) + math.sqrt(m))
    frac = sv ** 2 / max(float((sv ** 2).sum()), 1e-300)
    k = int(np.sum((sv > 1.3 * edge) & (frac >= min_var))) if sig > 0 else int(np.sum(sv > 0.02 * sv[0]))
    k = int(np.clip(k, 1, min(max_k, len(sv))))
    out.update(k=k, sv=sv[:max_k + 1].tolist(), sigma=sig, edge=float(edge))
    starts = []
    prev = 0
    for t in range(max(3, n // 20), n + 1, max(1, n // 60)):
        s = np.linalg.svd(D[:t], compute_uv=False)
        c = int(np.sum(s > 1.3 * edge * math.sqrt(t / n)))
        while prev < min(c, k):
            starts.append(t - 1)
            prev += 1
    out["efa_start"] = starts
    return out


def _isotonic_up_down(c: np.ndarray) -> np.ndarray:
    """Unimodality constraint: closest (least squares) non-decreasing then non-increasing curve around the maximum
    (pool-adjacent-violators on each side of the peak)."""
    def pav(v):
        vals, wts = [], []
        for x in v:
            vals.append(float(x))
            wts.append(1)
            while len(vals) > 1 and vals[-2] > vals[-1]:
                tot = vals[-2] * wts[-2] + vals[-1] * wts[-1]
                wt = wts[-2] + wts[-1]
                vals[-2:], wts[-2:] = [tot / wt], [wt]
        return np.repeat(vals, wts)
    k = int(np.argmax(c))
    left = pav(c[:k + 1])
    right = pav(c[k:][::-1])[::-1]
    out = np.concatenate([left[:-1], [max(left[-1], right[0])], right[1:]])
    return out


def simplisma(D: np.ndarray, k: int, offset: float = 0.05) -> list[int]:
    """SIMPLISMA: indices of k 'pure variables' (channels) of D (scans x channels); a pure variable is a channel to which only one
    component contributes. Purity = std/(mean + offset*max mean), then sequential weights by orthogonalisation."""
    n, m = D.shape
    mu = D.mean(0)
    sd = D.std(0)
    alpha = offset * mu.max()
    purity = sd / (mu + alpha)
    Dn = D / np.sqrt(mu ** 2 + (sd + 0) ** 2 + 1e-300) / math.sqrt(n)
    sel = []
    for _ in range(k):
        best, best_j = -1.0, 0
        for j in range(m):
            if j in sel:
                continue
            M = np.column_stack([Dn[:, j]] + [Dn[:, s] for s in sel])
            wgt = np.linalg.det(M.T @ M)
            sc = purity[j] * wgt
            if sc > best:
                best, best_j = sc, j
        sel.append(best_j)
    return sel


def nndsvd(D: np.ndarray, k: int):
    """NNDSVD initialisation of non-negative factors D ~ W H (Boutsidis & Gallopoulos): W (scans x k), H (k x channels)."""
    U, s, Vt = np.linalg.svd(D, full_matrices=False)
    W = np.zeros((D.shape[0], k))
    H = np.zeros((k, D.shape[1]))
    W[:, 0] = math.sqrt(s[0]) * np.abs(U[:, 0])
    H[0] = math.sqrt(s[0]) * np.abs(Vt[0])
    for j in range(1, k):
        u, v = U[:, j], Vt[j]
        up, un, vp, vn = np.maximum(u, 0), np.maximum(-u, 0), np.maximum(v, 0), np.maximum(-v, 0)
        nup, nun, nvp, nvn = map(np.linalg.norm, (up, un, vp, vn))
        mp, mn = nup * nvp, nun * nvn
        if mp >= mn:
            uu, vv, sg = up / max(nup, 1e-300), vp / max(nvp, 1e-300), mp
        else:
            uu, vv, sg = un / max(nun, 1e-300), vn / max(nvn, 1e-300), mn
        W[:, j] = math.sqrt(s[j] * sg) * uu
        H[j] = math.sqrt(s[j] * sg) * vv
    return W, H


def nmf(D: np.ndarray, k: int, n_iter: int = 300, seed: int = 0) -> dict:
    """Non-negative matrix factorisation D ~ C S^T (multiplicative updates) from an NNDSVD start. Fast alternative to MCR-ALS."""
    D = np.maximum(D, 0)
    W, H = nndsvd(D, k)
    eps = 1e-12
    W, H = W + eps, H + eps
    for _ in range(n_iter):
        H *= (W.T @ D) / (W.T @ W @ H + eps)
        W *= (D @ H.T) / (W @ H @ H.T + eps)
    return {"C": W, "S": H.T, "lof": _lof(D, W, H.T)}


def _lof(D, C, S) -> float:
    r = D - C @ S.T
    return float(100.0 * math.sqrt((r * r).sum() / max((D * D).sum(), 1e-300)))


def mcr_als(D: np.ndarray, k: int | None = None, init: str = "simplisma", max_iter: int = 300, unimodal: bool = True,
            seed: int = 0, tol: float = 1e-6, S0: np.ndarray | None = None) -> dict:
    """Multivariate curve resolution - alternating least squares of D (scans x channels) = C S^T + E with C, S >= 0.

    C: elution profiles (one column per component); S: pure spectra (channels x k, each normalised to its maximum; the scale is
    carried by C); unimodal=True forces every elution profile to have one maximum (isotonic regression on both sides of the apex).
    Initial spectra: SIMPLISMA pure variables ('simplisma'), NNDSVD ('nndsvd'), random ('random') or given (`S0`).
    k=None: from `estimate_components`. Returns C, S, lof (lack of fit, % of the norm of D), r2, n_iter, k."""
    D = np.asarray(D, float)
    n, m = D.shape
    if k is None:
        k = estimate_components(D)["k"]
    k = int(min(k, n, m))
    rng = np.random.default_rng(seed)
    if S0 is not None:
        S = np.maximum(np.asarray(S0, float), 0)
    elif init == "simplisma" and k <= m:
        idx = simplisma(D, k)
        # spectra of the pure variables: the rows of D at the scan where each pure variable peaks
        S = np.column_stack([D[int(np.argmax(D[:, j]))] for j in idx])
    elif init == "nndsvd" and k <= min(n, m):
        S = nndsvd(D, k)[1].T
    else:
        S = rng.random((m, k)) * D.max()
    S = np.maximum(S, 1e-12)
    prev = None
    C = np.zeros((n, k))
    it = 0

    def c_step(S_):
        C_ = np.maximum(np.linalg.lstsq(S_ + 1e-12, D.T, rcond=None)[0].T, 0)
        if unimodal:
            for j in range(C_.shape[1]):
                if C_[:, j].max() > 0:
                    C_[:, j] = _isotonic_up_down(C_[:, j])
        return C_

    for it in range(1, max_iter + 1):
        C = c_step(S)
        S = np.maximum(np.linalg.lstsq(C + 1e-12, D, rcond=None)[0].T, 0)
        mx = S.max(0)
        mx[mx <= 0] = 1.0
        S = S / mx
        cur = _lof(D, c_step(S), S)
        if prev is not None and abs(prev - cur) < tol * max(prev, 1e-12):
            break
        prev = cur
    C = c_step(S)            # the elution profiles carry the scale of the unit-maximum spectra
    cur = _lof(D, C, S)
    return {"C": C, "S": S, "lof": cur, "r2": float(1 - (cur / 100) ** 2), "n_iter": it, "k": k}


def mcr_stability(D: np.ndarray, k: int, n_starts: int = 8, seed: int = 0) -> dict:
    """Rotational ambiguity / stability: repeat MCR-ALS from `n_starts` random starts and compare each solution with the SIMPLISMA
    reference component by component (best cosine of the pure spectra, greedy matching). Returns the cosine per component
    (min, mean) and the spread of lack of fit. Low cosine = the decomposition is not unique: do not trust the clean spectra."""
    ref = mcr_als(D, k, "simplisma")
    cos_all, lofs = [], []
    for s in range(n_starts):
        r = mcr_als(D, k, "random", seed=seed + s)
        lofs.append(r["lof"])
        used, cs = set(), []
        for j in range(ref["k"]):
            c = [cosine(ref["S"][:, j], r["S"][:, q]) if q not in used else -1 for q in range(r["k"])]
            q = int(np.argmax(c))
            used.add(q)
            cs.append(c[q])
        cos_all.append(cs)
    cos_all = np.array(cos_all)
    return {"cos_min": cos_all.min(0).tolist(), "cos_mean": cos_all.mean(0).tolist(), "lof_ref": ref["lof"],
            "lof_sd": float(np.std(lofs)), "lof_range": [float(min(lofs)), float(max(lofs))], "n_starts": n_starts}


def deconvolve_window(w: Window, i0: int, i1: int, k: int | None = None, mz_range=None, min_snr: float = 5.0, stability: bool = True,
                      min_rel: float = 0.02) -> dict:
    """MCR-ALS on the ROI x scan window [i0, i1]: each component = a chromatogram + a clean spectrum (the spectrum of the parent with
    its in-source fragments separates from that of a coeluting product when their elution profiles differ even a little)."""
    X = w.X[:, i0:i1 + 1]
    ok = np.flatnonzero(X.max(1) > 0)
    if mz_range is not None:
        ok = ok[(w.mz[ok] >= mz_range[0]) & (w.mz[ok] <= mz_range[1])]
    snr = np.array([X[j].max() / max(noise_level(w.X[j]), 1e-9) for j in ok])
    ok = ok[snr >= min_snr]
    if len(ok):                                       # only the channels that matter in this window (chemical background is left out)
        mx = X[ok].max(1)
        ok = ok[mx >= min_rel * mx.max()]
    if len(ok) < 2 or X.shape[1] < 6:
        return {"ok": False}
    D = X[ok].T
    res = mcr_als(D, k)
    out = {"ok": True, "k": res["k"], "lof": res["lof"], "r2": res["r2"], "rt": w.rt[i0:i1 + 1].tolist(), "mz": w.mz[ok].tolist(),
           "C": res["C"].T.tolist(), "S": res["S"].T.tolist(), "n_iter": res["n_iter"], "components": estimate_components(D)}
    if stability and res["k"] >= 1:
        out["stability"] = mcr_stability(D, res["k"])
    return out


# ----------------------------------------------------------------------------------------------------------------------
# 8. source-voltage (DP) ramp
# ----------------------------------------------------------------------------------------------------------------------
def source_ramp(dp, f_int, p_int) -> dict:
    """Breakdown curves from standards acquired at different declustering potentials (DP, V): the ISF ratio F/(F+P) rises with the DP
    along a sigmoid  y = ymax / (1 + exp(-(DP - DP50)/s));  a real product that merely co-elutes has a ratio that does not depend on the
    DP. Returns the relative intensity, the sigmoid fit (ymax, dp50, slope s, appearance DP = DP at 10% of ymax, R2 of the fit),
    Spearman and Theil-Sen slope of the ratio against DP, and `dp_dependence` = (ratio at the highest DP - ratio at the lowest) / mean.
    Needs >= 4 DP values; below that only the correlation is reported."""
    dp, F, P = (np.asarray(v, float) for v in (dp, f_int, p_int))
    o = np.argsort(dp)
    dp, F, P = dp[o], F[o], P[o]
    with np.errstate(divide="ignore", invalid="ignore"):
        y = F / (F + P)
    ok = np.isfinite(y)
    out = {"n": int(ok.sum()), "dp": dp.tolist(), "ratio": y.tolist(), "warnings": []}
    if ok.sum() < 3:
        out["warnings"].append(message("of.warn.dp3"))
        return out
    d, v = dp[ok], y[ok]
    out["spearman"] = wspearman(d, v)
    out["slope_per_V"], _ = theil_sen(d, v)
    out["dp_dependence"] = float((v[-1] - v[0]) / max(float(v.mean()), 1e-12))
    if ok.sum() >= 4:
        best = None
        for ymax in np.linspace(max(v.max(), 1e-6), min(max(v.max() * 1.5, 1e-6), 1.0), 30):
            for dp50 in np.linspace(d.min(), d.max(), 60):
                for s in np.geomspace(max((d.max() - d.min()) / 60, 1e-3), (d.max() - d.min()) * 2, 40):
                    fit = ymax / (1 + np.exp(-(d - dp50) / s))
                    sse = float(((v - fit) ** 2).sum())
                    if best is None or sse < best[0]:
                        best = (sse, ymax, dp50, s)
        sse, ymax, dp50, s = best
        sst = float(((v - v.mean()) ** 2).sum())
        out["sigmoid"] = {"ymax": float(ymax), "dp50": float(dp50), "s": float(s), "dp_appearance": float(dp50 - s * math.log(9.0)),
                          "r2": float(1 - sse / sst) if sst > 0 else float("nan")}
    else:
        out["warnings"].append(message("of.warn.dp4"))
    return out


# ----------------------------------------------------------------------------------------------------------------------
# 9. MS2 similarity
# ----------------------------------------------------------------------------------------------------------------------
def greedy_match(S):
    """Greedy one-to-one matching by descending weight, dense and batched.

    S: (..., Ka, Kb) non-negative candidate weights (0 = not a candidate). Returns (total weight, number of pairs) per leading index.
    Each round takes the pairs that are the maximum of both their row and their column ("locally dominant"); with distinct weights the pairs
    taken over the rounds are exactly those of the global greedy (heaviest pair first, then the heaviest compatible one...)."""
    W = np.asarray(S, float)
    S = W + (W > 0) * 1e-12 * np.arange(W.shape[-2] * W.shape[-1]).reshape(W.shape[-2:])       # distinct weights, only to decide the order
    tot = np.zeros(S.shape[:-2])
    n = np.zeros(S.shape[:-2], int)
    for _ in range(min(S.shape[-2:])):
        if not (S > 0).any():
            break
        dom = (S > 0) & (S == S.max(-1, keepdims=True)) & (S == S.max(-2, keepdims=True))
        tot += (W * dom).sum((-1, -2))                 # the sums use the true weights, not the tie-breaking ones
        n += dom.sum((-1, -2))
        S = np.where(dom.any(-1, keepdims=True) | dom.any(-2, keepdims=True), 0.0, S)
    return tot, n


def _candidates(a, b, tol, shifts):
    """(ia, ib) of every pair with |b - a - d| <= tol for some d in shifts, without building the Ka x Kb matrix."""
    order = np.argsort(b, kind="stable")
    bs = b[order]
    keys = []
    for d in shifts:
        lo = np.searchsorted(bs, a + d - tol - 1e-9, "left")
        n = np.searchsorted(bs, a + d + tol + 1e-9, "right") - lo
        ia = np.repeat(np.arange(len(a)), n)
        ib = order[np.arange(int(n.sum())) - np.repeat(np.cumsum(n) - n, n) + np.repeat(lo, n)]
        ok = np.abs(b[ib] - a[ia] - d) <= tol
        keys.append(ia[ok] * len(b) + ib[ok])
    keys = np.unique(np.concatenate(keys)) if keys else np.zeros(0, np.int64)
    return keys // max(len(b), 1), keys % max(len(b), 1)


def _greedy_pairs(ia, ib, w, na, nb):
    """Global greedy one-to-one matching of weighted candidate pairs (heaviest first; ties by position), by locally dominant rounds.
    Returns the chosen indices into ia/ib/w, heaviest first."""
    if len(w) == 0:
        return np.zeros(0, int)
    rank = np.empty(len(w), np.int64)
    rank[np.lexsort((np.arange(len(w)), -w))] = np.arange(len(w))
    alive = np.arange(len(w))
    out = []
    while len(alive):
        ra = np.full(na, len(w), np.int64)
        rb = np.full(nb, len(w), np.int64)
        np.minimum.at(ra, ia[alive], rank[alive])
        np.minimum.at(rb, ib[alive], rank[alive])
        dom = alive[(rank[alive] == ra[ia[alive]]) & (rank[alive] == rb[ib[alive]])]
        out.append(dom)
        ua, ub = np.zeros(na, bool), np.zeros(nb, bool)
        ua[ia[dom]] = True
        ub[ib[dom]] = True
        alive = alive[~(ua[ia[alive]] | ub[ib[alive]])]
    out = np.concatenate(out)
    return out[np.argsort(rank[out])]


def _match(a_mz, a_i, b_mz, b_i, tol, shift=0.0, also_shift=None):
    """Greedy one-to-one matching of the peaks of a to those of b: list of (ia, ib) by descending product of the intensities.

    A pair is a candidate when the peak of a, moved by `shift`, is within tol of the peak of b; with `also_shift` it is also a candidate when it
    is within tol after that second shift (the modified cosine: fragments that kept their mass and fragments that moved with the precursor
    compete for the same peaks in one matching, instead of the unshifted ones taking them first)."""
    a, b = np.asarray(a_mz, float), np.asarray(b_mz, float)
    ya, yb = np.asarray(a_i, float), np.asarray(b_i, float)
    ia, ib = _candidates(a, b, tol, (shift,) if also_shift is None else (shift, also_shift))
    pick = _greedy_pairs(ia, ib, ya[ia] * yb[ib], len(a), len(b))
    return list(zip(ia[pick].tolist(), ib[pick].tolist()))


def ms2_similarity(mz_x, spec_x, mz_p, spec_p, tol: float = 0.5, mz_cut: float = 1.5) -> dict:
    """Similarity of the product-ion spectra of an ion X and of a candidate precursor P (spec = (mz array, intensity array)).

    cosine: peaks matched at the same m/z (+-tol); modified_cosine: also matches shifted by the precursor mass difference
    (mz_p - mz_x), which finds the fragments that both share with a constant mass loss; x_in_p: is X among the products of P
    (a peak of P's spectrum within tol of mz_x, excluding P's own precursor); subset_score: fraction of the intensity of X's
    products (with sqrt weighting) found among P's products (same m/z, or shifted by the mass difference). Square-root weighting
    reduces the dominance of the base peak. A high value says the spectra are compatible, not that X comes from P.
    The modified cosine is one matching in which a peak of X can pair with a peak of P at shift 0 or at the precursor shift, the heaviest pairs first."""
    ax, ix = (np.asarray(v, float) for v in spec_x)
    ap, ip = (np.asarray(v, float) for v in spec_p)
    out = {"n_x": int(len(ax)), "n_p": int(len(ap))}
    if len(ax) == 0 or len(ap) == 0:
        out.update(cosine=float("nan"), modified_cosine=float("nan"), x_in_p=False, subset_score=float("nan"))
        return out
    sx, sp = np.sqrt(ix), np.sqrt(ip)
    nx, npp = np.linalg.norm(sx), np.linalg.norm(sp)
    nan = float("nan")
    pairs0 = _match(ax, sx, ap, sp, tol, 0.0)
    pairs = _match(ax, sx, ap, sp, tol, 0.0, also_shift=mz_p - mz_x)
    c0 = float(sum(sx[i] * sp[j] for i, j in pairs0) / (nx * npp)) if nx * npp > 0 else nan
    mc = float(sum(sx[i] * sp[j] for i, j in pairs) / (nx * npp)) if nx * npp > 0 else nan
    out.update(cosine=c0, modified_cosine=mc)
    prod_p = ap[np.abs(ap - mz_p) > mz_cut]
    out["x_in_p"] = bool(np.any(np.abs(prod_p - mz_x) <= tol))
    found = [i for i, _ in pairs]
    out["subset_score"] = float(sum(sx[i] ** 2 for i in found) / max((sx ** 2).sum(), 1e-300))
    return out


def ms2_membership(mz_x: float, mz_p: float, spec_p, tol: float = 0.5, mz_cut: float = 1.5) -> dict:
    """When only the product-ion spectrum of the candidate precursor P was acquired: is there a product of P at the m/z of X
    (excluding P's own precursor peak)? Returns {x_in_p, nearest_product (m/z), delta, rel_intensity (of that product, % of P's base peak)}.
    A fragment seen in the MS2 of P is *compatible* with in-source fragmentation; it does not prove it."""
    ap, ip = (np.asarray(v, float) for v in spec_p)
    keep = np.abs(ap - mz_p) > mz_cut
    ap, ip = ap[keep], ip[keep]
    if len(ap) == 0:
        return {"x_in_p": False, "n_p": 0}
    j = int(np.argmin(np.abs(ap - mz_x)))
    return {"x_in_p": bool(abs(ap[j] - mz_x) <= tol), "nearest_product": float(ap[j]), "delta": float(ap[j] - mz_x),
            "rel_intensity": float(100 * ip[j] / max(ip.max(), 1e-300)), "n_p": int(len(ap))}


# ----------------------------------------------------------------------------------------------------------------------
# 11. calibration helpers (ROC, thresholds by cross-validation)
# ----------------------------------------------------------------------------------------------------------------------
def roc_curve(scores, labels):
    """(fpr, tpr, thresholds) of a score where a HIGHER value means label 1; ties handled by threshold steps."""
    s, y = np.asarray(scores, float), np.asarray(labels, int)
    ok = np.isfinite(s)
    s, y = s[ok], y[ok]
    order = np.argsort(-s, kind="mergesort")
    s, y = s[order], y[order]
    tps, fps = np.cumsum(y), np.cumsum(1 - y)
    last = np.r_[np.diff(s) != 0, True]
    tpr = np.r_[0.0, tps[last] / max(y.sum(), 1)]
    fpr = np.r_[0.0, fps[last] / max((1 - y).sum(), 1)]
    return fpr, tpr, np.r_[np.inf, s[last]]


def auc(scores, labels) -> float:
    """Area under the ROC curve (Mann-Whitney form with average ranks): 0.5 = no information, 1 = perfect."""
    s, y = np.asarray(scores, float), np.asarray(labels, int)
    ok = np.isfinite(s)
    s, y = s[ok], y[ok]
    n1, n0 = int(y.sum()), int((1 - y).sum())
    if n1 == 0 or n0 == 0:
        return float("nan")
    r = rankdata(s)
    return float((r[y == 1].sum() - n1 * (n1 + 1) / 2.0) / (n1 * n0))


def youden_threshold(scores, labels) -> dict:
    """Threshold maximising TPR - FPR (Youden J); {thr, tpr, fpr, j}."""
    fpr, tpr, thr = roc_curve(scores, labels)
    j = tpr - fpr
    k = int(np.argmax(j))
    return {"thr": float(thr[k]), "tpr": float(tpr[k]), "fpr": float(fpr[k]), "j": float(j[k])}


def cv_threshold(scores, labels, k: int = 5, seed: int = 0) -> dict:
    """k-fold cross-validation of the Youden threshold: threshold chosen on k-1 folds, accuracy measured on the held-out fold.
    Returns the thresholds per fold (their spread tells how stable it is) and the held-out balanced accuracy."""
    s, y = np.asarray(scores, float), np.asarray(labels, int)
    ok = np.isfinite(s)
    s, y = s[ok], y[ok]
    rng = np.random.default_rng(seed)
    folds = np.array_split(rng.permutation(len(s)), k)
    thr, acc = [], []
    for f in folds:
        tr = np.setdiff1d(np.arange(len(s)), f)
        t = youden_threshold(s[tr], y[tr])["thr"]
        pred = s[f] >= t
        tp = ((pred == 1) & (y[f] == 1)).sum() / max((y[f] == 1).sum(), 1)
        tn = ((pred == 0) & (y[f] == 0)).sum() / max((y[f] == 0).sum(), 1)
        thr.append(t)
        acc.append(0.5 * (tp + tn))
    return {"thresholds": [float(t) for t in thr], "thr_mean": float(np.mean(thr)), "thr_sd": float(np.std(thr)),
            "balanced_accuracy": float(np.mean(acc))}


def calibrate_thresholds(scores: dict, labels, k: int = 5) -> dict:
    """For each metric (name -> scores, higher = more 'inherits the profile of P'): AUC, Youden threshold on all data and its
    cross-validated spread/balanced accuracy. The thresholds are indications, never rules: the student tool shows the numbers and
    only the private TP Mine module combines them (with these calibrations as likelihoods)."""
    y = np.asarray(labels, int)
    return {name: {"auc": auc(sc, y), **youden_threshold(sc, y), "cv": cv_threshold(sc, y, k)} for name, sc in scores.items()}


# ----------------------------------------------------------------------------------------------------------------------
# 10. origin report (what /api/origin returns): evidence only, no verdict
# ----------------------------------------------------------------------------------------------------------------------
def _area_in(rt, y, t0: float, t1: float) -> float:
    yc, _, _ = _prep(rt, y)
    m = (rt >= t0) & (rt <= t1)
    return float(_trapz(yc[m], rt[m])) if m.sum() >= 2 else 0.0


def origin_report(samples, mz: float, parent_mz: float, rt0: float | None = None, rt1: float | None = None, ref: int | None = None,
                  tol: float = TOL_MZ, formula_p: str | None = None, top: int = 40, ms2=None, min_rel_area: float = 0.005) -> dict:
    """Evidence on where the ion at `mz` comes from, relative to the candidate precursor `parent_mz`. `samples` is a list of
    dicts {"label", "time" (min or None), "table" (reader.PeakTable of the full-scan level), "key" (cache key)}. `ms2` (optional):
    {"x": (mz, int), "p": (mz, int)} product-ion spectra of X and of P. Nothing is concluded: every number is a measurement with its
    own limits, `warnings` lists them as messages {key, params} for the student tool. Heavy parts are cached per sample and window."""
    t_start = time.time()
    rep = {"mz": float(mz), "parent_mz": float(parent_mz), "warnings": [], "samples": [], "candidates": [], "timing": {}}
    if not samples:
        rep["warnings"].append(message("of.warn.noFull"))
        return rep
    # reference sample = where the parent is strongest (unless chosen)
    if ref is None:
        tops = [float(s["table"].xic(parent_mz, 0.5).max()) if len(s["table"].rt) else 0.0 for s in samples]
        ref = int(np.argmax(tops))
    S = samples[ref]
    rt, P = extract_trace(S["table"], parent_mz, 0.5)
    det = detect_peaks(rt, P)
    if not det["peaks"]:
        rep["warnings"].append(message("of.warn.noParentRef"))
        return rep
    pk = det["peaks"][0]
    if rt0 is None or rt1 is None:
        half = max(0.4, 4.0 * pk["fwhm_min"])
        rt0, rt1 = pk["apex_rt"] - half, pk["apex_rt"] + half
    rep.update(ref=ref, ref_label=S.get("label"), window=[float(rt0), float(rt1)], parent_apex_rt=pk["apex_rt"],
               parent_fwhm_s=pk["fwhm_min"] * 60)
    peak_t0, peak_t1 = float(rt[pk["lo"]]), float(rt[pk["hi"]])
    w = build_rois(S["table"], rt0, rt1, tol, key=S.get("key", id(S["table"])))
    rep["timing"]["rois_s"] = time.time() - t_start
    rep["n_rois"] = int(len(w.mz))
    if pk["n_scans"] < MIN_SCANS_RELIABLE:
        rep["warnings"].append(message("of.warn.parentFewScans", n=pk["n_scans"]))
    # P and X in the reference window
    rtw, Pw = extract_trace(S["table"], parent_mz, tol, rt0, rt1)
    flat = Pw >= 0.97 * Pw.max()
    rep["parent_flat_top_scans"] = int(flat.sum())
    if flat.sum() >= 3 and Pw.max() > 0:
        rep["warnings"].append(message("of.warn.flatTop", n=int(flat.sum())))
    _, Xw = extract_trace(S["table"], mz, tol, rt0, rt1)
    sim = profile_similarity(rtw, Xw, Pw, p_apex_rt=pk["apex_rt"])
    rep["profile"] = sim
    rep["warnings"] += sim.get("warnings", [])
    # ratio F/P in the window of P's peak
    i0, i1 = int(np.searchsorted(rtw, peak_t0)), int(np.searchsorted(rtw, peak_t1, side="right")) - 1
    rf = ratio_fit(rtw, Xw, Pw, (i0, i1))
    rep["ratio"] = rf
    # co-eluting candidates ranked by profile correlation (the student chooses; nothing is picked for them)
    t1 = time.time()
    area = w.X[:, max(i0, 0):i1 + 1].sum(1) if len(w.mz) else np.zeros(0)
    order = np.argsort(-area)[:top] if len(area) else []
    cands = []
    pmax = float(area.max()) if len(area) else 0.0
    for j in order:
        if area[j] < min_rel_area * pmax:
            continue
        s2 = profile_similarity(w.rt, w.X[j], Pw, p_apex_rt=pk["apex_rt"], n_boot=0, n_shift=100)
        if not s2.get("n_scans"):
            continue
        cands.append({"mz": float(w.mz[j]), "area_rel": float(area[j] / pmax) if pmax > 0 else 0.0,
                      **{k: s2.get(k) for k in ("pearson", "spearman", "deriv_corr", "cosine", "apex_diff_s", "apex_diff_sigma_s",
                                                "fwhm_ratio", "p_shift", "lag_best", "reliable", "x_has_peak")},
                      "roles": annotate_roles(float(w.mz[j]), parent_mz, formula_p)})
    cands.sort(key=lambda d: -(d["pearson"] if d["pearson"] is not None and math.isfinite(d["pearson"]) else -2))
    rep["candidates"] = cands
    rep["timing"]["candidates_s"] = time.time() - t1
    rep["roles"] = annotate_roles(mz, parent_mz, formula_p)
    rep["isotopes"] = isotope_check(w, mz, max(i0, 0), i1, None)
    # lighter neighbours 1 and 2 Da below X (is X the M+1 / M+2 of another ion?): area ratio X/neighbour and profile correlation
    rep["neighbors"] = []
    for kk in (1, 2):
        if len(w.mz):
            jn = int(np.argmin(np.abs(w.mz - (mz - kk * 1.003355))))
            if abs(w.mz[jn] - (mz - kk * 1.003355)) <= 0.5:
                an = float(w.X[jn, max(i0, 0):i1 + 1].sum())
                ax = float(Xw[max(i0, 0):i1 + 1].sum())
                rn = wpearson(Xw[max(i0, 0):i1 + 1], w.X[jn, max(i0, 0):i1 + 1])
                rep["neighbors"].append({"delta": kk, "mz": float(w.mz[jn]), "area_ratio": ax / an if an > 0 else float("nan"), "pearson": rn})
    # mass coincidence at unit resolution: another strong co-eluting ROI within 1 Da of X
    near = [c for c in cands if 0.4 < abs(c["mz"] - mz) <= 1.0 and c["area_rel"] > 0.05]
    if near:
        rep["warnings"].append(message("of.warn.massCoincidence", mz=", ".join("%.1f" % c["mz"] for c in near[:3])))
    if rf.get("ok") and rf.get("n", 0) < 6:
        rep["warnings"].append(message("of.warn.fewRatio", n=rf["n"]))
    if sim.get("x_snr", 1) < 5:
        rep["warnings"].append(message("of.warn.weakIon"))
    # across samples
    rows, ratios, ses = [], [], []
    for k, s in enumerate(samples):
        tb = s["table"]
        r_, p_ = extract_trace(tb, parent_mz, tol, rt0, rt1)
        _, x_ = extract_trace(tb, mz, tol, rt0, rt1)
        if len(r_) < 7:
            continue
        ap, ax = _area_in(r_, p_, peak_t0, peak_t1), _area_in(r_, x_, peak_t0, peak_t1)
        row = {"k": k, "label": s.get("label"), "time": s.get("time"), "area_parent": ap, "area_ion": ax}
        a0, a1 = int(np.searchsorted(r_, peak_t0)), int(np.searchsorted(r_, peak_t1, side="right")) - 1
        if ap > 0 and a1 - a0 >= 4:
            rf_k = ratio_fit(r_, x_, p_, (a0, a1), n_boot=60)
            if rf_k.get("ok"):
                row["ratio"], row["ratio_se"] = rf_k["r_tls"], rf_k["r_se"]
                ratios.append(rf_k["r_tls"])
                ses.append(rf_k["r_se"])
        # apex of the ion in this sample, to see if it moves with the parent
        pks = detect_peaks(r_, p_)["peaks"]
        pk_k = min(pks, key=lambda d: abs(d["apex_rt"] - pk["apex_rt"])) if pks else None
        row["parent_apex_rt"] = pk_k["apex_rt"] if pk_k and abs(pk_k["apex_rt"] - pk["apex_rt"]) < 0.3 else None
        rows.append(row)
    rep["samples"] = rows
    rep["timing"]["samples_s"] = time.time() - t1
    if len(ratios) >= 2:
        rep["ratio_constancy"] = ratio_constancy(ratios, ses, [r["label"] for r in rows if "ratio" in r])
    tt = [(r["time"], r["area_parent"], r["area_ion"]) for r in rows if r["time"] is not None]
    if len(tt) >= 3:
        rep["kinetics"] = kinetics([a for a, _, _ in tt], [b for _, b, _ in tt], [c for _, _, c in tt])
        rep["warnings"] += rep["kinetics"].get("warnings", [])
    else:
        rep["warnings"].append(message("of.warn.fewTimed"))
    if ms2 is not None and ms2.get("p") is not None:
        rep["ms2"] = ms2_similarity(mz, ms2["x"], parent_mz, ms2["p"]) if ms2.get("x") is not None else ms2_membership(mz, parent_mz, ms2["p"])
    rep["timing"]["total_s"] = time.time() - t_start
    rep["note"] = message("of.note")
    return to_jsonable(rep)
