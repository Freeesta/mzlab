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


# ------------------------------------------------------------------------------------------------ difference from the reference
FLOOR = 1000.0            # smallest pseudocount (area units): used when no file gives a noise level, and as a lower bound of it
NOISE_FACTOR = 3.0        # pseudocount = this x the median of the positive areas of the blank files
NOT_TREATED = ("blank", "dark", "qc", "standard")      # roles that are never a treated sample


def default_reference(roles: list[str], times: list) -> list[int]:
    """Columns that are the reference when the user has not chosen: time <= 0 (the start of the series) or a blank / dark control.
    The same rule as the reference of TP_Mine (hr/filters.py split_columns); a file without a time is a reference only by its type."""
    return [i for i, (r, t) in enumerate(zip(roles, times)) if r in ("blank", "dark") or (t is not None and t <= 0)]


def _rank(v: np.ndarray) -> np.ndarray:
    """Ranks 1..n, ties get the mean rank (Spearman without scipy)."""
    o = np.argsort(v, kind="stable")
    r = np.empty(len(v))
    r[o] = np.arange(1, len(v) + 1)
    for u in np.unique(v):
        m = v == u
        if m.sum() > 1:
            r[m] = r[m].mean()
    return r


def _spearman(x: np.ndarray, y: np.ndarray) -> float | None:
    if len(x) < 3 or np.ptp(x) == 0 or np.ptp(y) == 0:
        return None
    return float(np.corrcoef(_rank(x), _rank(y))[0, 1])


def _shape(y: np.ndarray, rho: float | None, floor: float, rho_min: float, drop: float) -> str | None:
    """Descriptive label of a time course (y = mean area at every time, in time order), only to sort and filter, never a verdict:
    'rises_falls' (maximum inside, the last point back under `drop` x maximum), 'rises' / 'falls' (|rho| >= rho_min and the last point
    above / below the first), else 'flat'. None when there are fewer than 3 times, or nothing above `floor`."""
    if len(y) < 3 or rho is None or y.max() <= floor:
        return None
    m = int(np.argmax(y))
    if 0 < m < len(y) - 1 and y[-1] <= drop * y[m]:
        return "rises_falls"
    if rho >= rho_min and y[-1] > y[0]:
        return "rises"
    if rho <= -rho_min and y[-1] < y[0]:
        return "falls"
    return "flat"


def reference_difference(table, roles: list[str], times: list, groups: list | None = None, reference: list[int] | None = None,
                         factor: float = 3.0, ref_stat: str = "max", min_detect: int = 1, pseudo: float | None = None,
                         floor: float = FLOOR, rho_min: float = 0.5, drop: float = 0.5) -> dict:
    """Difference of every feature of a table (features x files, areas) from a reference: numbers to sort and filter, never a verdict.

    reference: column indices chosen by the user; None = `default_reference` (time <= 0, blank, dark). Treated = the other files whose role
    is not blank / dark / qc / standard. A missing area (None / NaN / negative) is 0. times: minutes or None. groups: replicate label per file
    (None = files with the same time are replicates).
    Per feature: ref_level (maximum or mean of the reference areas), treated_max and its file, ratio = treated_max / ref_level (None when
    the level is 0), log2fc = log2((treated_max + pseudo) / (ref_level + pseudo)) with pseudo = 3 x the median of the positive areas of the
    blank files (else of the reference files, else `floor`; never under `floor`) unless given, absent_in_reference (level <= pseudo), shape
    (see `_shape`) with rho = Spearman of the mean area per time (files with role 'sample' and a time, the start included) against time and
    t_max, replicates of the file of the maximum (mean, cv in %, n detected / n files), and the blank_filter verdict on the threshold
    ref_level x factor: kept (at least `min_detect` treated files above it) with a reason ('absent_in_reference' | 'above_threshold') or
    discarded ('no_signal' | 'below_reference' | 'too_few_files'). Both lists are returned; nothing is hidden."""
    if ref_stat not in ("max", "mean"):
        raise UserError("err.stat.blank.stat", text="stat must be 'max' or 'mean'")
    if not factor >= 0 or min_detect < 1 or not floor >= 0 or (pseudo is not None and not pseudo >= 0):
        raise UserError("err.stat.blank.param", text="factor must be >= 0 and the minimum of detections >= 1")
    m = _matrix(table)
    n = len(roles)
    if len(times) != n or (m.shape[0] and m.shape[1] != n) or (groups is not None and len(groups) != n):
        raise UserError("err.stat.ref.shape", text="the table, the roles and the times must have one entry per file")
    tm = [None if (t is None or t != t) else float(t) for t in times]
    explicit = reference is not None
    ref = sorted({int(i) for i in reference}) if explicit else default_reference(roles, tm)
    if any(i < 0 or i >= n for i in ref):
        raise UserError("err.stat.ref.shape", text="the table, the roles and the times must have one entry per file")
    if not ref:
        raise UserError("err.stat.ref.none", text="no reference: mark a blank or give a file at time 0")
    tr = [i for i in range(n) if i not in ref and roles[i] not in NOT_TREATED]
    if not tr:
        raise UserError("err.stat.ref.notreated", text="no treated file besides the reference")
    # pseudocount: the noise of the blanks
    bl = [i for i in ref if roles[i] == "blank"] or [i for i, r in enumerate(roles) if r == "blank"]
    src = "given"
    if pseudo is None:
        src, pseudo = "floor", float(floor)
        for cols, name in ((bl, "blanks"), (ref, "reference")):
            pos = m[:, cols][m[:, cols] > 0] if m.shape[0] and cols else np.zeros(0)
            if len(pos):
                pseudo, src = max(float(floor), NOISE_FACTOR * float(np.median(pos))), name
                break
    pseudo = float(pseudo)
    # kept / discarded: the blank filter with the reference as the blank
    r2 = ["dark"] * n
    for i in ref:
        r2[i] = "blank"
    for i in tr:
        r2[i] = "sample"
    bf = blank_filter(m.tolist(), r2, factor, ref_stat, min_detect) if m.shape[0] else {"kept": [], "discarded": [], "rows": []}
    kept_set = set(bf["kept"])
    # time course: role 'sample' files with a time, the start included
    ser = sorted({tm[i] for i in range(n) if roles[i] == "sample" and tm[i] is not None})
    cols_at = [[i for i in range(n) if roles[i] == "sample" and tm[i] == t] for t in ser]
    tx = np.array(ser, float)
    lab = [str(g) if g not in (None, "") else None for g in groups] if groups is not None else [None] * n

    def replicates(i0: int) -> list[int]:
        same = [i for i in tr if (lab[i0] is not None and lab[i] == lab[i0]) or (lab[i0] is None and lab[i] is None and tm[i0] is not None and tm[i] == tm[i0])]
        return same or [i0]

    rows = []
    for f in range(m.shape[0]):
        a = m[f]
        q = bf["rows"][f]
        lvl, top_i = float(q["blank"]), max(tr, key=lambda i: a[i])
        top = float(a[top_i])
        y = np.array([a[c].mean() for c in cols_at])
        rho = _spearman(tx, y)
        rg = a[replicates(top_i)]
        mean = float(rg.mean())
        absent = lvl <= pseudo
        if f in kept_set:
            reason = "absent_in_reference" if absent else "above_threshold"
        else:
            reason = "no_signal" if top <= 0 else ("below_reference" if q["n_above"] == 0 else "too_few_files")
        rows.append({"ref_level": lvl, "threshold": q["threshold"], "treated_max": top, "max_file": int(top_i),
                     "ratio": None if lvl == 0 else top / lvl, "log2fc": float(np.log2((top + pseudo) / (lvl + pseudo))),
                     "absent_in_reference": bool(absent), "n_above": q["n_above"], "n_treated": len(tr),
                     "shape": _shape(y, rho, pseudo, rho_min, drop), "rho": rho,
                     "t_max": float(ser[int(np.argmax(y))]) if len(y) and y.max() > 0 else None,
                     "rep_mean": mean, "rep_cv": float(rg.std(ddof=1) / mean * 100) if len(rg) > 1 and mean > 0 else None,
                     "rep_n_detected": int((rg > 0).sum()), "rep_n": len(rg), "kept": f in kept_set, "reason": reason})
    return {"reference": ref, "reference_rule": "explicit" if explicit else "metadata", "treated": tr, "times": ser,
            "kept": bf["kept"], "discarded": bf["discarded"], "rows": rows,
            "params": {"factor": float(factor), "ref_stat": ref_stat, "min_detect": int(min_detect), "pseudo": pseudo, "pseudo_source": src,
                       "floor": float(floor), "noise_factor": NOISE_FACTOR, "rho_min": float(rho_min), "drop": float(drop)}}


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
