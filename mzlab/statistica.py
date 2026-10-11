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
