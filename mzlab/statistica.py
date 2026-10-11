"""Statistics of a series of runs, inspired by the mzmine workflow: time profile of one ion, blank subtraction of a feature table,
exploratory PCA. Numbers only, never verdicts: the user chooses the ion, the factors and the thresholds; the program shows what comes out.

Pure numpy on small matrices (features x samples), no state: the page sends the table and gets the result (same shape of work as qaqc.py).
"""
from __future__ import annotations

import numpy as np

from .i18n import UserError


# ------------------------------------------------------------------------------------------------ time profile of one ion
def time_profile(points: list[dict], metric: str = "area", istd: dict | None = None) -> dict:
    """Profile of one ion over the files, in the order of their time.

    points: one dict per file {file, time (minutes or None), rt, area, height} (area / height None where no peak was found).
    metric: 'area' or 'height'. istd: optional {file: {area, height}} of an internal standard chosen by the user (high resolution): the
    value is divided by the standard of the same file ('norm'; None where the standard is missing or zero). Nothing is fitted or smoothed.
    Returns {'metric', 'rows', 'series'}: rows = every file (files with a time first, by time then name; the others after, in the given
    order); series = the indices of rows that have a time and a value, i.e. the points the page joins by a line."""
    if metric not in ("area", "height"):
        raise UserError("err.stat.metric", text="metric must be 'area' or 'height'")
    rows = []
    for i, p in enumerate(points):
        v = p.get(metric)
        v = None if v is None else float(v)
        n = None
        if istd is not None and v is not None:
            s = (istd.get(p["file"]) or {}).get(metric)
            n = (v / float(s)) if s else None
        t = p.get("time")
        rows.append({"file": p["file"], "time": None if t is None else float(t), "rt": p.get("rt"), "area": p.get("area"),
                     "height": p.get("height"), "value": v, "norm": n, "_i": i})
    rows.sort(key=lambda r: (r["time"] is None, r["time"] if r["time"] is not None else 0.0, r["file"] if r["time"] is not None else "", r["_i"]))
    for r in rows:
        del r["_i"]
    key = "norm" if istd is not None else "value"
    series = [i for i, r in enumerate(rows) if r["time"] is not None and r[key] is not None]
    return {"metric": metric, "normalized": istd is not None, "rows": rows, "series": series}


# ------------------------------------------------------------------------------------------------ blank subtraction of a feature table
def _matrix(rows) -> np.ndarray:
    """features x samples list of lists -> float array; a missing value (None / NaN / negative) is 0 (not detected)."""
    a = np.array([[0.0 if (v is None or v != v) else float(v) for v in r] for r in rows], float) if len(rows) else np.zeros((0, 0))
    return np.clip(a, 0.0, None)


def blank_filter(matrix, roles: list[str], factor: float = 3.0, stat: str = "max", min_detect: int = 1) -> dict:
    """Which features rise above the blank. For every feature the blank level is the maximum (or the mean) of its areas in the blank
    files; the threshold is level x factor; a sample file 'detects' the feature when its area is above the threshold (and above 0).
    A feature is kept when at least `min_detect` sample files detect it; the others are discarded (both lists are returned, nothing is
    hidden). Files with role 'blank' form the blank; 'dark' files are neither blank nor sample. factor and min_detect are the user's.
    Returns {'kept': [feature indices], 'discarded': [...], 'rows': per feature {blank, threshold, n_above, n_samples, max_sample}}."""
    if stat not in ("max", "mean"):
        raise UserError("err.stat.blank.stat", text="stat must be 'max' or 'mean'")
    if not factor >= 0 or min_detect < 1:
        raise UserError("err.stat.blank.param", text="factor must be >= 0 and the minimum of detections >= 1")
    m = _matrix(matrix)
    bl = [i for i, r in enumerate(roles) if r == "blank"]
    sm = [i for i, r in enumerate(roles) if r not in ("blank", "dark")]
    if not bl:
        raise UserError("err.stat.blank.none", text="no file is marked as a blank")
    if not sm:
        raise UserError("err.stat.blank.nosample", text="no sample file")
    lvl = m[:, bl].max(axis=1) if stat == "max" else m[:, bl].mean(axis=1)
    thr = lvl * float(factor)
    above = (m[:, sm] > thr[:, None]) & (m[:, sm] > 0)
    n_above = above.sum(axis=1)
    keep = n_above >= int(min_detect)
    rows = [{"blank": float(lvl[i]), "threshold": float(thr[i]), "n_above": int(n_above[i]), "n_samples": len(sm), "max_sample": float(m[i, sm].max())}
            for i in range(m.shape[0])]
    return {"kept": [int(i) for i in np.flatnonzero(keep)], "discarded": [int(i) for i in np.flatnonzero(~keep)], "rows": rows,
            "stat": stat, "factor": float(factor), "min_detect": int(min_detect), "n_blank": len(bl), "n_sample": len(sm)}


# ------------------------------------------------------------------------------------------------ exploratory PCA
MIN_SAMPLES = 5                      # below this the PCA is not offered: with fewer samples the axes describe the noise
SCALINGS = ("pareto", "auto", "none")
TRANSFORMS = ("log", "none")


def impute(x: np.ndarray) -> tuple[np.ndarray, int]:
    """Replace every missing value (0 = not detected) of a feature by 1/5 of the smallest value that feature has (declared, never silent).
    x: features x samples. Returns the filled copy and how many values were replaced."""
    out = x.copy()
    n = 0
    for i in range(out.shape[0]):
        pos = out[i] > 0
        if pos.all():
            continue
        out[i, ~pos] = out[i, pos].min() / 5.0 if pos.any() else 0.0
        n += int((~pos).sum())
    return out, n


def _scale(z: np.ndarray, scaling: str) -> np.ndarray:
    """samples x features: centre every feature, then divide by sd (auto), sqrt(sd) (Pareto) or nothing."""
    z = z - z.mean(axis=0)
    if scaling == "none":
        return z
    sd = z.std(axis=0, ddof=1)
    sd[sd == 0] = 1.0
    return z / (sd if scaling == "auto" else np.sqrt(sd))


def _svd(xs: np.ndarray, k: int):
    u, s, vt = np.linalg.svd(xs, full_matrices=False)
    k = min(k, len(s))
    # sign convention (the sign of a component is arbitrary): the loading of largest size is positive
    for j in range(k):
        if vt[j, np.argmax(np.abs(vt[j]))] < 0:
            vt[j] = -vt[j]; u[:, j] = -u[:, j]
    return u[:, :k] * s[:k], vt[:k].T, (s ** 2) / max((s ** 2).sum(), 1e-300), s


def _align(load_full: np.ndarray, load_sub: np.ndarray) -> np.ndarray:
    """Components of a PCA done without one sample, matched to the components of the full PCA by the largest |cosine| (order and sign of
    the components can change), so that loadings can be compared. Returns the loadings reordered and sign-aligned (features x k)."""
    k = load_full.shape[1]
    c = load_full.T @ load_sub                       # k x k' cosines (unit vectors)
    out = np.zeros_like(load_full)
    used: set[int] = set()
    for j in range(k):
        order = [i for i in np.argsort(-np.abs(c[j])) if i not in used]
        i = order[0] if order else None
        if i is None:
            continue
        used.add(int(i))
        out[:, j] = load_sub[:, i] * (1.0 if c[j, i] >= 0 else -1.0)
    return out


def loo_stability(zt: np.ndarray, scaling: str, load_full: np.ndarray, ncomp: int = 2) -> np.ndarray:
    """Stability of the loadings of every feature: the PCA is repeated leaving out one sample at a time (scaling recomputed each time);
    rel = (sd of the loading over the repetitions) / (typical loading, 1/sqrt(p)); stability = 1 / (1 + rel) in (0, 1] (1 = the loading never
    moved; 0.5 = it moves as much as a typical loading). Returns features x ncomp (one column per component). zt: samples x features."""
    n, p = zt.shape
    k = min(ncomp, load_full.shape[1])
    runs = []
    for i in range(n):
        sub = np.delete(zt, i, axis=0)
        _, ld, _, _ = _svd(_scale(sub, scaling), max(k, 1) + 1)
        runs.append(_align(load_full[:, :k], ld[:, : k + 1] if ld.shape[1] > k else ld))
    sd = np.std(np.array(runs), axis=0)              # features x k
    rel = sd / (1.0 / np.sqrt(p))
    return 1.0 / (1.0 + rel)


def ellipse(points: np.ndarray, chi2: float = 5.991) -> dict | None:
    """95 % covariance ellipse of the points (n x 2) of one group of replicates: centre, semi-axes and angle (radians) of the first axis;
    None below three points. An indication of the spread of the replicates, not a test."""
    if len(points) < 3:
        return None
    c = points.mean(axis=0)
    w, v = np.linalg.eigh(np.cov(points.T))
    w = np.clip(w, 0.0, None)
    return {"cx": float(c[0]), "cy": float(c[1]), "a": float(np.sqrt(chi2 * w[1])), "b": float(np.sqrt(chi2 * w[0])), "angle": float(np.arctan2(v[1, 1], v[0, 1]))}


def pca_analysis(matrix, roles: list[str], groups: list[str] | None = None, factor: float = 3.0, stat: str = "max", min_detect: int = 1,
                 min_intensity: float = 0.0, min_samples: int = 2, transform: str = "log", scaling: str = "pareto", ncomp: int = 5) -> dict:
    """Exploratory PCA of a feature table (features x samples, areas). NOT a test: with few or no replicates it is a way to look at the data.

    Order of the work (nothing is hidden: the counts of every step are returned):
      1. samples = files that are not blanks or dark (needs at least MIN_SAMPLES, else UserError);
      2. filter of the features: above the blank (blank_filter; skipped, and said so, when there is no blank file), with a maximum
         area >= min_intensity, present (area >= min_intensity) in at least min_samples samples;
      3. missing values imputed with 1/5 of the minimum of the feature (counted);
      4. transform (log2 or none), centring and scaling (Pareto by default; 'auto' divides by sd and inflates the noise with few samples);
      5. SVD: scores, loadings, variance explained by every component;
      6. stability of the loadings leaving one sample out at a time; 95 % ellipses of the groups of replicates (>= 3 samples).
    Returns indices relative to the input: 'features' (kept), 'samples' (used)."""
    if transform not in TRANSFORMS or scaling not in SCALINGS:
        raise UserError("err.stat.pca.param", text="unknown transform or scaling")
    m = _matrix(matrix)
    sm = [i for i, r in enumerate(roles) if r not in ("blank", "dark")]
    if len(sm) < MIN_SAMPLES:
        raise UserError("err.stat.pca.few", {"n": len(sm), "min": MIN_SAMPLES}, f"PCA needs at least {MIN_SAMPLES} samples, there are {len(sm)}")
    if m.shape[0] == 0:
        raise UserError("err.stat.pca.nofeatures", text="no feature in the table")
    steps = [{"step": "table", "n": int(m.shape[0])}]
    keep = np.ones(m.shape[0], bool)
    has_blank = any(r == "blank" for r in roles)
    if has_blank:
        bf = blank_filter(matrix, roles, factor, stat, min_detect)
        keep &= np.isin(np.arange(m.shape[0]), bf["kept"])
    steps.append({"step": "blank", "n": int(keep.sum()), "applied": has_blank})
    x = m[:, sm]
    keep &= x.max(axis=1) >= min_intensity
    steps.append({"step": "intensity", "n": int(keep.sum()), "min": float(min_intensity)})
    keep &= (x >= max(min_intensity, np.finfo(float).tiny)).sum(axis=1) >= int(min_samples)
    steps.append({"step": "presence", "n": int(keep.sum()), "min_samples": int(min_samples)})
    idx = np.flatnonzero(keep)
    if len(idx) < 2:
        raise UserError("err.stat.pca.nofeatures", text="fewer than two features survive the filters")
    filled, n_imp = impute(x[idx])
    z = np.log2(filled) if transform == "log" else filled
    zt = z.T                                          # samples x features
    sd = zt.std(axis=0, ddof=1)
    live = sd > 0                                     # a constant feature carries nothing
    idx, zt = idx[live], zt[:, live]
    if len(idx) < 2:
        raise UserError("err.stat.pca.nofeatures", text="fewer than two features vary between the samples")
    xs = _scale(zt, scaling)
    scores, loadings, expl, _ = _svd(xs, ncomp)
    stab_pc = loo_stability(zt, scaling, loadings, 2)
    w = loadings[:, : stab_pc.shape[1]] ** 2                       # a feature counts mostly where its loading is large: weight each component's stability by it
    stab = np.where(w.sum(axis=1) > 0, (w * stab_pc).sum(axis=1) / np.maximum(w.sum(axis=1), 1e-300), stab_pc.min(axis=1))
    ell = {}
    if groups is not None:
        gs = [groups[i] for i in sm]
        for g in dict.fromkeys(gs):
            pts = scores[[j for j, q in enumerate(gs) if q == g]][:, :2]
            e = ellipse(pts) if scores.shape[1] >= 2 else None
            if e:
                ell[g] = e
    return {"samples": sm, "features": [int(i) for i in idx], "scores": scores.tolist(), "loadings": loadings.tolist(),
            "explained": expl[: scores.shape[1]].tolist(), "stability": stab.tolist(), "stability_pc": stab_pc.tolist(), "ellipses": ell, "steps": steps,
            "imputed": n_imp, "transform": transform, "scaling": scaling, "n_samples": len(sm), "n_features": int(len(idx))}
