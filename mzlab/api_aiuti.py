"""Helpers of the endpoints (mzlab/api.py): parsing of query values, the difference options, the JSON answers, the map and its points."""
from __future__ import annotations

import base64
import json

import numpy as np

from .app import rounded
from .differenza import NORMS, area, norm_factor, subtract_spectrum, subtract_trace
from .explore import MAP_POINTS_MAX, check_agg, diff_tolerant, map_cells, rt_groups, subtract_constant
from .i18n import UserError


def _peaks(text):
    """'mz:intensity;mz:intensity' -> [(mz, intensity)] (None when empty)."""
    out = [tuple(float(x) for x in part.split(":")) for part in (text or "").split(";") if part]
    return out[:2000] or None


def _weights(text):
    """'1,1,0.5' -> (mass, isotopes, fragments) weights; missing or bad = 1,2,1."""
    try:
        w = [max(0.0, float(x)) for x in (text or "").split(",")]
    except ValueError:
        w = []
    return tuple((w + [1.0, 2.0, 1.0][len(w):])[:3])


def _diff_opts(q: dict) -> dict | None:
    """The reference of /api/xic, /api/chrom, /api/mrm and /api/spectrum: ref (file number), factor, norm (none | tic | istd), istd (m/z), rttol, clip. None without ref."""
    if q.get("ref") in (None, ""):
        return None
    norm = q.get("norm") or "none"
    if norm not in NORMS:
        raise UserError("err.diff.norm", {"norm": str(norm)[:20]}, f"norm must be one of {', '.join(NORMS)}, not {norm!r}")
    if norm == "istd" and q.get("istd") in (None, ""):
        raise UserError("err.diff.istd", {}, "norm=istd needs istd (the m/z of the internal standard)")
    o = {"ref": int(q["ref"]), "factor": float(q.get("factor", 1.0)), "norm": norm, "istd": float(q["istd"]) if q.get("istd") not in (None, "") else None,
         "rttol": float(q.get("rttol", 0.1)), "clip": q.get("clip") == "1"}
    if not (0 <= o["factor"] <= 100 and 0 <= o["rttol"] <= 10):
        raise UserError("err.diff.range", {}, "factor must be between 0 and 100 and rttol between 0 and 10 minutes")
    return o


def _diff_scale(app, k: int, o: dict, level: int, tol: float) -> float:
    """factor x normalization of the reference towards file k (total ion current or internal standard area, whole file)."""
    if o["norm"] == "none":
        return o["factor"]
    if o["norm"] == "tic":
        a, b = (app._item(i).total("tic", level) for i in (k, o["ref"]))
    else:
        a, b = (app._item(i).xic(o["istd"], tol, level) for i in (k, o["ref"]))
    return o["factor"] * norm_factor(o["norm"], area(*a), area(*b))


def _mrm_area(transitions: list, o: dict) -> float:
    """Area to normalize an MRM file by: all its transitions (tic) or the ones whose precursor is the internal standard (istd)."""
    if o["norm"] == "none":
        return 0.0
    sel = [t for t in transitions if o["norm"] == "tic" or abs((t.get("q1") or 0) - o["istd"]) <= 0.5]
    return sum(area(t["rt"], t["y"]) for t in sel)


def _diff_cut(rt, y, rrt, ry, scale: float, o: dict, nd: int = 1):
    """(rt, y) of the difference rounded for the page; the times outside the range of the reference are left out (no data, not zero)."""
    d = subtract_trace(rt, y, rrt, ry, scale, o["rttol"], o["clip"])
    ok = ~np.isnan(d)
    return rounded(np.asarray(rt)[ok], 4), rounded(d[ok], nd)


def _json(obj, code=200):
    return code, "application/json", json.dumps(obj).encode("utf-8"), {}


def _fail(e, code=400):
    """The answer for an exception: a UserError carries its message key and parameters; the text is for developers (the page translates the key)."""
    if isinstance(e, UserError):
        return _json(e.to_json(), code)
    return _json({"error": str(e) if isinstance(e, ValueError) else f"missing or invalid parameter: {e}"}, code)


MAX_BODY = 8 << 20  # bytes accepted for a JSON request


def _map_factor(app, k: int, level: int, norm: str, agg: str = "mean") -> float:
    """Normalisation of a map before a comparison, from the WHOLE map of the file: «max» = 1 / its most intense bin, «tic» = 1000 / its sum."""
    if norm not in ("max", "tic"):
        return 1.0
    m = app._item(k).ionmap(level, app.session.grid(level), agg)
    t = float(m.max()) if norm == "max" else float(m.sum())
    return (1.0 if norm == "max" else 1000.0) / t if t > 0 else 1.0


def _map(app, q: dict) -> dict:
    """/api/map with a region (rt0, rt1, mz0, mz1, nrt, nmz; at most 1024 x 1024) and/or a reference (ref, norm, rttol = tolerance in RT of the
    difference, default 0.1 min: the reference is replaced by its local maximum over +-rttol).
    agg = max (default: a peak is never diluted by the bin) | mean (default for a difference). mzscale = lin | log (cells of equal ppm; high
    resolution). bg = 1 takes the constant background away (median of every m/z cell along the run). The m/z cells follow the instrument
    (explore.map_cells): the answer says the grid actually used (mz0, dmz or dlog, nmz, mz1)."""
    k, level = int(q["k"]), int(q.get("level", 1))
    region = q.get("rt0") not in (None, "")
    ref = int(q["ref"]) if q.get("ref") not in (None, "") else None
    agg = check_agg(q.get("agg") or ("mean" if ref is not None else "max"))
    scale = q.get("mzscale") or "lin"
    if scale not in ("lin", "log"):
        raise UserError("err.map.mzscale", {"mzscale": str(scale)[:20]}, "mzscale must be lin or log")
    bg = q.get("bg") in ("1", "true", 1, True)
    stretch = (float(q["bgrt0"]), float(q["bgrt1"])) if q.get("bgrt0") not in (None, "") and q.get("bgrt1") not in (None, "") else None     # background of a stretch (mean over this RT interval), instead of the median over the run

    def stretch_bg(it, lv, m0, m1, n, lg, ag):
        return it.map_stretch(lv, stretch[0], stretch[1], m0, m1, n, lg) if stretch else it.map_background(lv, m0, m1, n, lg, ag)
    hr = bool(app._item(k).profile(level)["hr"])
    if region:
        rt0, rt1, mz0, mz1 = float(q["rt0"]), float(q["rt1"]), float(q["mz0"]), float(q["mz1"])
        if not (rt1 > rt0 and mz1 > mz0 and mz0 > 0):
            raise UserError("err.map.region", {}, "empty region: rt1 > rt0 and mz1 > mz0 > 0 are needed")
        nrt, nmz = min(1024, max(1, int(q.get("nrt", 400)))), min(1024, max(1, int(q.get("nmz", 400))))
        mz0, mz1, nmz, log = map_cells(hr, mz0, mz1, nmz, hr and scale == "log")
        grid = {"rt0": rt0, "rt1": rt1, "nrt": nrt, "mz0": mz0, "mz1": mz1, "nmz": nmz}
        if log:
            grid["dlog"] = float(np.log(mz1 / mz0) / nmz)
        else:
            grid["dmz"] = (mz1 - mz0) / nmz

        def get(kk):
            it = app._item(kk)
            m = it.ionmap_region(level, rt0, rt1, nrt, mz0, mz1, nmz, agg, log)
            return subtract_constant(m, stretch_bg(it, level, mz0, mz1, nmz, log, agg)) if bg or stretch else m
    else:
        grid = app.session.grid(level)
        log = False

        def get(kk):
            m = app._item(kk).ionmap(level, grid, agg)
            it = app._item(kk)
            return subtract_constant(m, stretch_bg(it, level, grid["mz0"], grid["mz0"] + grid["nmz"] * grid["dmz"], grid["nmz"], False, agg)) if bg or stretch else m
    m = get(k)
    out = {**grid, "level": level, "region": region, "agg": agg, "log": log, "bg": bg or bool(stretch), "bgstretch": list(stretch) if stretch else None, "hr": hr}
    if not region:
        out["extent"] = app._item(k).data_extent(level)           # where the file has data: the page shows this part as the whole view
    if ref is not None:
        norm = q.get("norm", "abs")
        w = int(round(float(q.get("rttol", 0.1)) / max((grid["rt1"] - grid["rt0"]) / grid["nrt"], 1e-9)))
        wm = 0                                            # high resolution (log axis, cells of equal ppm): the reference is also widened in m/z by +-mztol ppm
        if log:
            wm = min(20, int(round(float(q.get("mztol", 5)) * 1e-6 / max(grid["dlog"], 1e-12))))
        m = diff_tolerant(m * _map_factor(app, k, level, norm, agg), get(ref) * _map_factor(app, ref, level, norm, agg), w, wm)
        out.update(ref=ref, rttol=float(q.get("rttol", 0.1)), w=w, wm=wm)
    out["data"] = base64.b64encode(np.ascontiguousarray(m, dtype="<f4").tobytes()).decode("ascii")
    return out


def _map_sticks(app, q: dict) -> dict:
    """/api/mappunti3d: the true points of a region (most intense first, up to max = 200000) for the sticks of the 3D viewer: three base64
    arrays (rt float32, mz float64, inten float32), n = points sent, total = points in the region (n < total: the weakest are left out)."""
    k, level = int(q["k"]), int(q.get("level", 1))
    rt0, rt1, mz0, mz1 = float(q["rt0"]), float(q["rt1"]), float(q["mz0"]), float(q["mz1"])
    if not (rt1 > rt0 and mz1 > mz0):
        raise UserError("err.map.region", {}, "empty region: rt1 > rt0 and mz1 > mz0 are needed")
    r = app._item(k).map_points(level, rt0, rt1, mz0, mz1, int(q.get("max", MAP_POINTS_MAX)))
    b64 = lambda a, dt: base64.b64encode(np.ascontiguousarray(a, dtype=dt).tobytes()).decode("ascii")
    return {"k": k, "level": level, "rt0": rt0, "rt1": rt1, "mz0": mz0, "mz1": mz1, "n": r["n"], "total": r["total"],
            "rt": b64(r["rt"], "<f4"), "mz": b64(r["mz"], "<f8"), "inten": b64(r["inten"], "<f4")}


def _map_points(app, q: dict) -> dict:
    """pts = "rt,mz;rt,mz;..." (the marked points, already on their apex). XIC window: high resolution (ppm > 0) mz +- ppm, otherwise the
    nominal window [n - 0.2; n + 0.8]. grp = tolerance of the RT groups (min)."""
    k, level = int(q["k"]), int(q.get("level", 1))
    ref = app._item(int(q["ref"])) if q.get("ref") not in (None, "") else None
    ppm, item = float(q.get("ppm", 0) or 0), app._item(k)
    pts = []
    for s in (q.get("pts") or "").split(";"):
        if not s.strip():
            continue
        rt, mz = (float(x) for x in s.split(","))
        c, tol = (mz, mz * ppm * 1e-6) if ppm > 0 else (round(mz) + 0.3, 0.5)
        pts.append({"rt": rt, "mz": mz, **item.map_measure(level, rt, c, tol, ref)})
    rows = rt_groups(pts, float(q.get("grp", 0.05)))
    for r in rows:
        r["d"] = r["ia"] - r["ib"] if r["ib"] is not None else None
        r["ratio"] = r["ia"] / r["ib"] if r["ib"] else None
    return {"rows": rows}
