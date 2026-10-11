"""QA/QC measures of a run series (internal standards, blanks, mass accuracy, replicate repeatability, sample/blank ratio).

Numbers only, never verdicts: the program shows how the series behaves; the reader decides (NORMAN guide, Hollender et al.,
Environ. Sci. Eur. 35:75, 2023; BP4NTA Study Reporting Tool). Pure numpy: the app feeds it traces and the page draws the result.
"""
from __future__ import annotations

import numpy as np

ROLES = ("sample", "blank", "qc", "standard", "dark")


def ppm_error(observed: float, theoretical: float) -> float:
    """Signed mass error in ppm: (observed - theoretical) / theoretical * 1e6."""
    return (observed - theoretical) / theoretical * 1e6


def peak_in_window(rt, y, rt_expected: float, half_window: float) -> dict | None:
    """Apex RT, area (trapezoid above the straight line between the window edges) and height of the highest point within
    rt_expected +- half_window (minutes); None when the window holds fewer than 3 scans or no signal."""
    rt, y = np.asarray(rt, float), np.asarray(y, float)
    m = (rt >= rt_expected - half_window) & (rt <= rt_expected + half_window)
    if m.sum() < 3 or not np.any(y[m] > 0):
        return None
    t, v = rt[m], y[m]
    base = np.linspace(v[0], v[-1], len(v))
    net = np.clip(v - base, 0, None)
    i = int(np.argmax(net))
    if net[i] <= 0:
        return None
    area = float(np.sum((net[1:] + net[:-1]) * np.diff(t)) / 2)
    return {"apex": float(t[i]), "area": area, "height": float(net[i]), "i0": int(np.flatnonzero(m)[0]), "i1": int(np.flatnonzero(m)[-1])}


def cv_percent(values) -> float | None:
    """Relative standard deviation (n - 1) in percent; None for fewer than two values or a zero mean."""
    v = np.asarray([x for x in values if x is not None], float)
    if len(v) < 2 or v.mean() == 0:
        return None
    return float(v.std(ddof=1) / abs(v.mean()) * 100)


def slope(values) -> float | None:
    """Least-squares slope per position of the series (drift per file); None below three defined values."""
    pts = [(i, x) for i, x in enumerate(values) if x is not None]
    if len(pts) < 3:
        return None
    x, y = np.array([p[0] for p in pts], float), np.array([p[1] for p in pts], float)
    return float(np.polyfit(x, y, 1)[0])


def mass_accuracy(errors) -> dict | None:
    """Mean and maximum (absolute) error in ppm of the observed ions; None when there are none."""
    e = [x for x in errors if x is not None]
    if not e:
        return None
    return {"mean": float(np.mean(e)), "max_abs": float(np.max(np.abs(e))), "n": len(e)}


def summarize(rows: list[dict], roles: list[str], rt_expected: float) -> dict:
    """Add the measures that need the whole series to the per-file rows of one standard and return the summary.

    rows: one dict per file in the order of the series ({'rt', 'area', 'dppm'}, None where the ion was not found).
    Adds 'drt' (RT - expected) and 'rel' (area / median area of the files that are not blanks or dark)."""
    ref = [r["area"] for r, ro in zip(rows, roles) if r and ro != "blank" and ro != "dark" and r.get("area")]
    med = float(np.median(ref)) if ref else None
    out = []
    for r in rows:
        if not r:
            out.append(None)
            continue
        o = dict(r)
        o["drt"] = o["rt"] - rt_expected
        o["rel"] = (o["area"] / med) if med else None
        out.append(o)
    found = [o for o in out if o]
    live = [o["area"] for o, ro in zip(out, roles) if o and ro not in ("blank", "dark")]
    reps = [o["area"] for o, ro in zip(out, roles) if o and ro == "qc"]
    return {
        "rows": out,
        "median_area": med,
        "rt_drift_per_file": slope([o["rt"] if o else None for o in out]),
        "area_drift_per_file": slope([o["area"] if o else None for o in out]),
        "cv_area": cv_percent(live),
        "cv_area_qc": cv_percent(reps),
        "mass_accuracy": mass_accuracy([o.get("dppm") for o in found]),
    }


def sample_blank_ratios(areas: list[float | None], roles: list[str], threshold: float = 3.0) -> dict:
    """Sample / blank area ratio of one ion: for every non-blank file area / mean area of the blanks (None when no blank is
    measured; infinite when the ion is absent from the blanks). 'at_least' = the ratio reaches `threshold` (3 or 10): a number
    against the line the user chose, not an identification."""
    bl = [a for a, ro in zip(areas, roles) if ro == "blank" and a is not None]
    ref = float(np.mean(bl)) if bl else None
    rows = []
    for a, ro in zip(areas, roles):
        if ro == "blank" or ref is None or a is None:
            rows.append(None)
            continue
        ratio = float("inf") if ref == 0 else a / ref
        rows.append({"ratio": None if ratio == float("inf") else ratio, "absent_in_blank": ratio == float("inf"), "at_least": ratio >= threshold})
    return {"blank_area": ref, "threshold": threshold, "rows": rows}
