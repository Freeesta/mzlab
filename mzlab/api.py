"""The endpoints of the program, independent of the transport.

`dispatch` is used by the test server (tools/dev_server.py) and by the browser version (Pyodide, web/browser-worker.js):
same routes, same answers, one implementation.
"""
from __future__ import annotations

import base64
import json

import numpy as np

from .app import rounded
from .bigfiles import too_big
from .i18n import UserError
from .chem.elements import formula_mz
from .differenza import NORMS, area, norm_factor, subtract_spectrum, subtract_trace
from .explore import MAP_POINTS_MAX, check_agg, diff_tolerant, map_cells, rt_groups, subtract_constant


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


# ---- routes: GET handlers take (app, q), POST handlers (app, body); `fail` lists the exceptions that are the user's (a JSON error
# with a key, status 400/500 as before) and not a server fault.
GET: dict = {}
POST: dict = {}


def _get(path, fail=()):
    def deco(fn):
        GET[path] = (fn, fail)
        return fn
    return deco


def _post(path):
    def deco(fn):
        POST[path] = fn
        return fn
    return deco


def _ks(q) -> list:
    return [int(x) for x in q["k"].split(",") if x]


def _has_ref(q) -> bool:
    return q.get("ref") not in (None, "")


@_get("/api/analog")
def _r_analog(app, q):    # one analog trace (pressure, flow...): minutes, values reduced to min/max per interval, unit to show, unit note
    return _json(app.analog(int(q["k"]), int(q["i"]), int(q.get("n", 2000))))


@_get("/api/gradient")
def _r_gradient(app, q):
    return _json({"gradient": app.gradient(int(q["k"]))})


@_get("/api/delay")
def _r_delay(app, q):     # delay of the mass spectrometer after the PDA (cross-correlation of the two traces): a measure, never applied by itself
    return _json({"delay": app.delay(int(q["k"]))})


@_get("/api/state")
def _r_state(app, q):
    return _json(app.state())


@_get("/api/notebook")
def _r_notebook(app, q):
    return _json(app.notebook())


@_get("/api/perf")
def _r_perf(app, q):
    return _json(app.perf())


@_get("/api/session")
def _r_session(app, q):
    return _json(app.session_state())


@_get("/api/chrom")
def _r_chrom(app, q):
    if _has_ref(q):
        o = _diff_opts(q)
        k, kind, level = int(q["k"]), q.get("kind", "tic"), int(q.get("level", 1))
        args = (level, float(q["mz0"]) if q.get("mz0") else None, float(q["mz1"]) if q.get("mz1") else None, float(q["prec"]) if q.get("prec") else None, q.get("filt") or None)
        rt, y = app._item(k).total(kind, *args)
        rr, ry = app._item(o["ref"]).total(kind, *args)
        sc = _diff_scale(app, k, o, level, float(q.get("tol", 0.35)))
        rt, y = _diff_cut(rt, y, rr, ry, sc, o)
        return _json({"rt": rt, "y": y, "ref": o["ref"], "scale": sc})
    return _json(app.chrom(int(q["k"]), q.get("kind", "tic"), int(q.get("level", 1)),
                           float(q["mz0"]) if q.get("mz0") else None, float(q["mz1"]) if q.get("mz1") else None,
                           float(q["prec"]) if q.get("prec") else None, q.get("filt") or None))


@_get("/api/qaqc", fail=(ValueError, KeyError))
def _r_qaqc(app, q):      # QA/QC measures of one ion over the series (standards, blanks, mass accuracy, sample / blank): numbers only
    return _json(app.qaqc_ion([int(x) for x in q["k"].split(",") if x], [x for x in q.get("roles", "").split(",")],
                              float(q["mz"]), float(q["rt"]), float(q.get("win", 0.5)), float(q.get("tol", 0.5)), q.get("unit", "da"),
                              int(q.get("level", 1)), float(q.get("thr", 3)), q.get("ratio") == "1"))


@_get("/api/filters")
def _r_filters(app, q):   # the scan types of a file (scan filters) with their counts: what the filter menu of a cell lists
    return _json(app.filters(int(q["k"])))


@_get("/api/scaninfo", fail=(ValueError, KeyError))
def _r_scaninfo(app, q):  # every name / value pair of the header of a scan, as the file has it
    return _json(app.scaninfo(int(q["k"]), int(q["sid"])))


@_get("/api/scanlist", fail=(ValueError, KeyError))
def _r_scanlist(app, q):  # the scans of a level / scan type in columns (the list of scans of the bench)
    return _json(app.scanlist(int(q["k"]), int(q.get("level", 1)), q.get("filt") or None))


@_get("/api/fileinfo", fail=(ValueError, KeyError))
def _r_fileinfo(app, q):  # what the mzML says about the file and the instrument
    return _json(app.fileinfo(int(q["k"])))


@_get("/api/xic")
def _r_xic(app, q):
    if _has_ref(q):
        o = _diff_opts(q)
        mz, tol, level = float(q["mz"]), float(q.get("tol", 0.35)), int(q.get("level", 1))
        rr, ry = app._item(o["ref"]).xic(mz, tol, level)
        out = []
        for k in _ks(q):
            rt, y = app._item(k).xic(mz, tol, level)
            sc = _diff_scale(app, k, o, level, tol)
            out.append({"k": k, **dict(zip(("rt", "y"), _diff_cut(rt, y, rr, ry, sc, o))), "scale": sc})
        return _json({"mz": mz, "tol": tol, "ref": o["ref"], "traces": out})
    return _json(app.xic(_ks(q), float(q["mz"]), float(q.get("tol", 0.35)), int(q.get("level", 1))))


@_get("/api/subxic", fail=(ValueError, KeyError))
def _r_subxic(app, q):    # combined XIC of all the sub-formulas of a formula (high resolution only): measures, no labels
    return _json(app.subxic(int(q["k"]), q["f"], float(q.get("ppm", 5)), float(q.get("min", 100)), int(q.get("z", 1))))


@_get("/api/origin")
def _r_origin(app, q):    # evidence on where an ion comes from (ionfamily); never a verdict
    return _json(app.origin(int(q.get("k", -1)), float(q["mz"]), float(q["parent"]),
                            float(q["rt0"]) if q.get("rt0") else None, float(q["rt1"]) if q.get("rt1") else None, q.get("formula")))


@_get("/api/mrm")
def _r_mrm(app, q):
    if not _has_ref(q):
        return _json(app.mrm(_ks(q)))
    o = _diff_opts(q)
    ref_all = app._item(o["ref"]).mrm()
    ref = {f"{t['q1']}>{t['q3']}": t for t in ref_all}
    trs_all = {k: app._item(k).mrm() for k in _ks(q)}
    files = []
    for k in _ks(q):
        trs = []
        for t in trs_all[k]:
            r = ref.get(f"{t['q1']}>{t['q3']}")
            if r is None:         # the reference lacks this transition: nothing to take away
                trs.append({**t, "nodiff": True})
                continue
            sc = o["factor"] * norm_factor(o["norm"], _mrm_area(trs_all[k], o), _mrm_area(ref_all, o))
            rt, y = _diff_cut(t["rt"], t["y"], r["rt"], r["y"], sc, o)
            trs.append({**t, "rt": rt, "y": y, "scale": sc})
        files.append({"k": k, "transitions": trs})
    return _json({"files": files, "ref": o["ref"]})


@_get("/api/method_warnings")
def _r_method_warnings(app, q):
    return _json(app.method_warnings())


@_get("/api/method")
def _r_method(app, q):
    return _json(app.method(int(q["k"])))


@_get("/api/nearest_scan")
def _r_nearest_scan(app, q):
    return _json(app.nearest_scan(
        int(q["k"]), float(q["rt"]),
        filter=q.get("filter"), level=int(q.get("level", 1)),
        precursor=float(q["precursor"]) if q.get("precursor") else None
    ))


@_get("/api/scan_axis", fail=(ValueError, KeyError))
def _r_scan_axis(app, q):  # RT and index of every scan of a level (the sight of the map picks the nearest scan in the page)
    return _json(app.scan_axis(int(q["k"]), int(q.get("level", 1))))


@_get("/api/spectrum")
def _r_spectrum(app, q):
    pr = q.get("precursor")
    bg = None
    if q.get("bgk") not in (None, ""):
        bg = {"k": int(q["bgk"]), "rt0": float(q["bgrt0"]), "rt1": float(q["bgrt1"]), "factor": float(q.get("bgf", 1.0)),
              "tol": float(q.get("bgtol", 5.0)), "clip": q.get("bgclip", "1") == "1"}
    return _json(app.spectrum(int(q["k"]), float(q["rt0"]), float(q["rt1"]), int(q.get("level", 1)),
                              float(pr) if pr not in (None, "") else None, float(q.get("bin", 0.1)), bg, q.get("merge") == "1", q.get("hr") != "0", q.get("filt") or None))


@_get("/api/spectra", fail=(ValueError, KeyError))
def _r_spectra(app, q):    # single scans i0..i1 (at most 60), for scan-by-scan navigation
    pr = q.get("prec", q.get("precursor"))
    return _json(app.spectra(int(q["k"]), int(q["i0"]), int(q["i1"]), int(q.get("level", 1)),
                             float(pr) if pr not in (None, "") else None, float(q.get("bin", 0.1)), q.get("merge") == "1", q.get("hr") != "0", q.get("filt") or None))


@_get("/api/scanbin", fail=(ValueError, KeyError))
def _r_scanbin(app, q):    # the same scans as /api/spectra, as a binary block (no JSON, no decimals): see App.scanbin
    pr = q.get("prec", q.get("precursor"))
    body = app.scanbin(int(q["k"]), int(q["i0"]), int(q["i1"]), int(q.get("level", 1)), float(pr) if pr not in (None, "") else None,
                       q.get("filt") or q.get("filter") or None, q.get("hr") != "0", q.get("merge") == "1")
    return 200, "application/octet-stream", body, {}


@_get("/api/dda", fail=(ValueError, KeyError))
def _r_dda(app, q):        # DDA and single scans as stored in the file (high resolution, B2)
    return _json(app.dda(int(q["k"]), q.get("hr") != "0"))


@_get("/api/scan", fail=(ValueError, KeyError))
def _r_scan(app, q):
    return _json(app.scan(int(q["k"]), int(q["sid"]), q.get("hr") != "0"))


@_get("/api/scanavg", fail=(ValueError, KeyError))
def _r_scanavg(app, q):
    return _json(app.scanavg(int(q["k"]), [int(x) for x in q["sids"].split(",") if x], q.get("hr") != "0"))


@_get("/api/formula", fail=(ValueError,))      # a typo in the formula is the user's, not a server fault
def _r_formula(app, q):
    return _json(formula_mz(q.get("f", ""), q.get("adduct") or None))


@_get("/api/composition", fail=(ValueError, KeyError, IndexError))
def _r_composition(app, q):   # formulas that fit an exact m/z (mzlab.chem.composition): candidates, never a verdict
    from .chem.composition import compose, parse_elements
    rdb = [float(x) for x in (q.get("rdb") or "-1,100").split(",")]
    el = parse_elements(q["elements"]) if q.get("elements") else None
    return _json(compose(float(q["mz"]), elements=el, ion=q.get("ion") or "[M+H]+", z=int(q["z"]) if q.get("z") else None,
                         tol=float(q.get("tol", 5)), unit=q.get("unit", "ppm"), rdb=(rdb[0], rdb[1]), nitrogen=q.get("n", "none"),
                         rules=[x for x in (q.get("rules") or "").split(",") if x], max_results=min(int(q.get("max", 10)), 200),
                         parent=q.get("parent") or None, m1=float(q["m1"]) if q.get("m1") else None, m2=float(q["m2"]) if q.get("m2") else None,
                         iso_tol=float(q.get("isotol", 0.2)), obs=_peaks(q.get("obs")), frags=_peaks(q.get("frags")),
                         r200=float(q.get("r200", 70000)), sigma_ppm=float(q.get("sppm", 3)), sigma_int=float(q.get("sint", 0.10)),
                         weights=_weights(q.get("w")), iso_min_rel=float(q.get("imin", 1.0)), iso_noise_rel=float(q.get("inoise", 1.0)),
                         frag_ppm=float(q["fppm"]) if q.get("fppm") else None, norm=q.get("norm") if q.get("norm") in ("mono", "max", "sum") else "fit"))


@_get("/api/contaminants")
def _r_contaminants(app, q):   # built-in list of known contaminants, expanded into ions (mzlab.chem.contaminants); matching is done in the browser (web/liste.js)
    from .chem.contaminants import builtin
    return _json({"lists": [builtin()]})


@_get("/api/msntree", fail=(ValueError, KeyError))
def _r_msntree(app, q):   # tree of the fragmentation paths of an MSn file (consensus spectra and formulas): candidates, not identifications
    return _json(app.msn_tree(int(q["k"]), q.get("formula") or None, float(q.get("ppm", 5))))


@_get("/api/map", fail=(ValueError, KeyError))
def _r_map(app, q):
    return _json(_map(app, q))


@_get("/api/mappunti3d", fail=(ValueError, KeyError))
def _r_mappunti3d(app, q):    # true points of a region (RT, m/z, intensity) for the sticks of the 3D viewer
    return _json(_map_sticks(app, q))


@_get("/api/mapsnap", fail=(ValueError, KeyError))
def _r_mapsnap(app, q):       # marked point of the map: the local maximum near the click (+-0.2 min, +-1 bin)
    return _json(app._item(int(q["k"])).map_snap(int(q.get("level", 1)), float(q["rt"]), float(q["mz"]), float(q.get("dmz", 1.0))) or {})


@_get("/api/mappunti", fail=(ValueError, KeyError))
def _r_mappunti(app, q):      # measures of the marked points (A, B, A - B, A/B, S/N, RT groups, delta m): numbers only
    return _json(_map_points(app, q))


# ---- POST
@_post("/api/explore")
def _p_explore(app, body):
    app.open_session(body)
    return _json(app.session_state())


@_post("/api/gradient")
def _p_gradient(app, body):      # the gradient the student typed, pasted or imported for a file (or null to remove it)
    return _json({"gradient": app.set_gradient(body)})


@_post("/api/gradient_parse")
def _p_gradient_parse(app, body):  # the text of a Chromeleon script or a CSV -> the gradient model (nothing is stored)
    from .analog import course_gradient, parse_chromeleon, parse_gradient_csv
    fmt = body.get("fmt")
    return _json({"gradient": course_gradient() if fmt == "course" else (parse_chromeleon if fmt == "chromeleon" else parse_gradient_csv)(body.get("text", ""))})


@_post("/api/meta")
def _p_meta(app, body):          # metadata of the open files (sample table): type, time, concentration, replicate group
    return _json(app.set_meta(body))


@_post("/api/stat/profile")
def _p_stat_profile(app, body):    # time profile of one ion chosen by the user (values already integrated by the page): order, normalization
    from .statistica import time_profile
    return _json(time_profile(body.get("points", []), body.get("metric", "area"), body.get("istd")))


@_post("/api/stat/blank")
def _p_stat_blank(app, body):      # blank subtraction of a feature table: threshold = blank x factor; two lists (kept, discarded)
    from .statistica import blank_filter
    return _json(blank_filter(body.get("matrix", []), body.get("roles", []), float(body.get("factor", 3)), body.get("stat", "max"), int(body.get("min_detect", 1))))


@_post("/api/stat/reference")
def _p_stat_reference(app, body):  # difference of every feature from the reference (time <= 0, blank, dark or chosen): numbers and two lists
    from .statistica import FLOOR, identity_links, reference_difference
    out = reference_difference(body.get("matrix", []), body.get("roles", []), body.get("times", []), body.get("groups"), body.get("reference"),
                               float(body.get("factor", 3)), body.get("stat", "max"), int(body.get("min_detect", 1)),
                               None if body.get("pseudo") is None else float(body["pseudo"]), float(body.get("floor", FLOOR)))
    if body.get("mz") and len(body["mz"]) == len(body.get("rt", [])):        # shown beside the numbers, never merged: adducts and in-source fragments of other rows
        out["identity"] = identity_links(body["mz"], body["rt"])
    return _json(out)


@_post("/api/stat/pca")
def _p_stat_pca(app, body):        # exploratory PCA of a feature table (filters, imputation, scaling, loadings stability): a view, not a test
    from .statistica import pca_analysis
    return _json(pca_analysis(body.get("matrix", []), body.get("roles", []), body.get("groups"), float(body.get("factor", 3)), body.get("stat", "max"),
                              int(body.get("min_detect", 1)), float(body.get("min_intensity", 0)), int(body.get("min_samples", 2)),
                              body.get("transform", "log"), body.get("scaling", "pareto"), int(body.get("ncomp", 5))))


@_post("/api/remove")
def _p_remove(app, body):
    app.remove_file(body.get("name", ""))
    return _json({"files": app.files(), "methods": app.methods()})


@_post("/api/notebook")
def _p_notebook(app, body):
    app.save_notebook(body)
    return _json({"ok": True})


@_post("/api/new")
def _p_new(app, body):
    app.reset(bool(body.get("fresh")))
    return _json(app.state())


@_post("/api/workflow/validate")
def _p_workflow_validate(app, body):
    from .workflow import unknown_operations, validate_workflow, verify_file_hashes
    ok, msg = validate_workflow(body)
    actual_hashes = {f["name"]: f.get("sha256", "") for f in app.files()}
    mismatches = verify_file_hashes(body, actual_hashes) if ok else []
    return _json({"valid": ok, "message": msg, "mismatches": mismatches, "unknown_ops": unknown_operations(body) if ok else []})


@_post("/api/export/mgf")
def _p_export_mgf(app, body):
    from .workflow import format_mgf
    text = format_mgf(body.get("spectra", []), lr=bool(body.get("lr")))
    return 200, "text/plain; charset=utf-8", text.encode("utf-8"), {"Content-Disposition": 'attachment; filename="export.mgf"'}


@_post("/api/export/msp")
def _p_export_msp(app, body):
    from .workflow import format_msp
    text = format_msp(body.get("spectra", []), lr=bool(body.get("lr")))
    return 200, "text/plain; charset=utf-8", text.encode("utf-8"), {"Content-Disposition": 'attachment; filename="export.msp"'}


def dispatch(app, method: str, path: str, q: dict, stream=None, length: int = 0):
    """(status, content type, body bytes, extra headers) for one request. stream/length: body of a POST."""
    if method == "POST":
        try:
            if path == "/api/upload":
                app.save_upload(q.get("name", ""), stream, length)
                return _json({"files": app.files(), "methods": app.methods()})
            if length > MAX_BODY:  # a JSON body is small; refuse anything huge instead of reading it into memory
                raise UserError("err.request.tooLarge", text="request too large")
            body = json.loads((stream.read(length) if stream is not None else b"") or b"{}")
            if path in POST:
                return POST[path](app, body)
            return _json({"error": "unknown endpoint"}, 404)
        except MemoryError:
            return _json(too_big(), 507)
        except Exception as e:  # noqa: BLE001
            return _fail(e) if isinstance(e, ValueError) else _json({"error": f"{type(e).__name__}: {e}"}, 400)
    try:
        if _has_ref(q) and path in ("/api/xic", "/api/chrom", "/api/mrm"):
            try:
                _diff_opts(q)                # a wrong reference setting is the user's: a JSON error with a key, not a server fault
            except ValueError as e:
                return _fail(e)
        if path in GET:
            fn, fail = GET[path]
            try:
                return fn(app, q)
            except fail as e:
                return _fail(e)
        return _json({"error": "unknown endpoint"}, 404)
    except MemoryError:   # a file too big for the browser's memory: say what to do, not a bare exception name
        return _json(too_big(), 507)
    except Exception as e:  # noqa: BLE001
        return _fail(e, 500) if isinstance(e, UserError) else _json({"error": f"{type(e).__name__}: {e}"}, 500)
