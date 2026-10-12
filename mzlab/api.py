"""The endpoints of the program, independent of the transport.

`dispatch` is used by the test server (tools/dev_server.py) and by the references of the tests (tests_e2e/pyref.py); the site and the desktop
app answer with the Rust engine, which the parity tests compare with this one.
"""
from __future__ import annotations

import json

from .api_aiuti import (MAX_BODY, _diff_cut, _diff_opts, _diff_scale, _fail, _json, _map, _map_points, _map_sticks, _mrm_area, _peaks,
                        _weights)
from .bigfiles import too_big
from .chem.elements import formula_mz
from .differenza import norm_factor
from .i18n import UserError


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


@_get("/api/ms2tr")
def _r_ms2tr(app, q):      # precursor > fragment pairs of the MS2 scans (the MS2 counterpart of the MRM transitions): m/z and intensity only
    return _json({"files": [{"k": k, "transitions": app._item(k).ms2_transitions()} for k in _ks(q)]})


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
