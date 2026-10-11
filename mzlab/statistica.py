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
