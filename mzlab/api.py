"""The endpoints of the program, independent of the transport.

`dispatch` is used by the test server (tools/dev_server.py) and by the browser version (Pyodide, web/browser-worker.js):
same routes, same answers, one implementation.
"""
from __future__ import annotations

import base64
import json

import numpy as np

from .bigfiles import too_big
from .i18n import UserError
from .chem.elements import formula_mz
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
            return subtract_constant(m, it.map_background(level, mz0, mz1, nmz, log, agg)) if bg else m
    else:
        grid = app.session.grid(level)
        log = False

        def get(kk):
            m = app._item(kk).ionmap(level, grid, agg)
            it = app._item(kk)
            return subtract_constant(m, it.map_background(level, grid["mz0"], grid["mz0"] + grid["nmz"] * grid["dmz"], grid["nmz"], False, agg)) if bg else m
    m = get(k)
    out = {**grid, "level": level, "region": region, "agg": agg, "log": log, "bg": bg, "hr": hr}
    if ref is not None:
        norm = q.get("norm", "abs")
        w = int(round(float(q.get("rttol", 0.1)) / max((grid["rt1"] - grid["rt0"]) / grid["nrt"], 1e-9)))
        m = diff_tolerant(m * _map_factor(app, k, level, norm, agg), get(ref) * _map_factor(app, ref, level, norm, agg), w)
        out.update(ref=ref, rttol=float(q.get("rttol", 0.1)), w=w)
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
            if path == "/api/explore":
                app.open_session(body)
                return _json(app.session_state())
            if path == "/api/meta":          # metadata of the open files (sample table): type, time, concentration, replicate group
                return _json(app.set_meta(body))
            if path == "/api/stat/profile":    # time profile of one ion chosen by the user (values already integrated by the page): order, normalization
                from .statistica import time_profile
                return _json(time_profile(body.get("points", []), body.get("metric", "area"), body.get("istd")))
            if path == "/api/stat/blank":      # blank subtraction of a feature table: threshold = blank x factor; two lists (kept, discarded)
                from .statistica import blank_filter
                return _json(blank_filter(body.get("matrix", []), body.get("roles", []), float(body.get("factor", 3)), body.get("stat", "max"), int(body.get("min_detect", 1))))
            if path == "/api/stat/pca":        # exploratory PCA of a feature table (filters, imputation, scaling, loadings stability): a view, not a test
                from .statistica import pca_analysis
                return _json(pca_analysis(body.get("matrix", []), body.get("roles", []), body.get("groups"), float(body.get("factor", 3)), body.get("stat", "max"),
                                          int(body.get("min_detect", 1)), float(body.get("min_intensity", 0)), int(body.get("min_samples", 2)),
                                          body.get("transform", "log"), body.get("scaling", "pareto"), int(body.get("ncomp", 5))))
            if path == "/api/remove":
                app.remove_file(body.get("name", ""))
                return _json({"files": app.files(), "methods": app.methods()})
            if path == "/api/notebook":
                app.save_notebook(body)
                return _json({"ok": True})
            if path == "/api/new":
                app.reset(bool(body.get("fresh")))
                return _json(app.state())
            if path == "/api/workflow/validate":
                from .workflow import unknown_operations, validate_workflow, verify_file_hashes
                ok, msg = validate_workflow(body)
                actual_hashes = {f["name"]: f.get("sha256", "") for f in app.files()}
                mismatches = verify_file_hashes(body, actual_hashes) if ok else []
                return _json({"valid": ok, "message": msg, "mismatches": mismatches, "unknown_ops": unknown_operations(body) if ok else []})
            if path == "/api/qaqc/features":      # sample / blank ratio of every feature of an untargeted table: values, no list of products
                from .qaqc import feature_ratios
                return _json({"rows": feature_ratios(body.get("features", []), body.get("blanks", []), body.get("samples", []), float(body.get("thr", 3)))})
            if path == "/api/export/mgf":
                from .workflow import format_mgf
                text = format_mgf(body.get("spectra", []), lr=bool(body.get("lr")))
                return 200, "text/plain; charset=utf-8", text.encode("utf-8"), {"Content-Disposition": 'attachment; filename="export.mgf"'}
            if path == "/api/export/msp":
                from .workflow import format_msp
                text = format_msp(body.get("spectra", []), lr=bool(body.get("lr")))
                return 200, "text/plain; charset=utf-8", text.encode("utf-8"), {"Content-Disposition": 'attachment; filename="export.msp"'}
            return _json({"error": "unknown endpoint"}, 404)
        except MemoryError:
            return _json(too_big(), 507)
        except Exception as e:  # noqa: BLE001
            return _fail(e) if isinstance(e, ValueError) else _json({"error": f"{type(e).__name__}: {e}"}, 400)
    try:
        if path == "/api/state":
            return _json(app.state())
        if path == "/api/notebook":
            return _json(app.notebook())
        if path == "/api/perf":
            return _json(app.perf())
        if path == "/api/session":
            return _json(app.session_state())
        if path == "/api/chrom":
            return _json(app.chrom(int(q["k"]), q.get("kind", "tic"), int(q.get("level", 1)),
                                   float(q["mz0"]) if q.get("mz0") else None, float(q["mz1"]) if q.get("mz1") else None,
                                   float(q["prec"]) if q.get("prec") else None, q.get("filt") or None))
        if path == "/api/qaqc":          # QA/QC measures of one ion over the series (standards, blanks, mass accuracy, sample / blank): numbers only
            try:
                return _json(app.qaqc_ion([int(x) for x in q["k"].split(",") if x], [x for x in q.get("roles", "").split(",")],
                                          float(q["mz"]), float(q["rt"]), float(q.get("win", 0.5)), float(q.get("tol", 0.5)), q.get("unit", "da"),
                                          int(q.get("level", 1)), float(q.get("thr", 3)), q.get("ratio") == "1"))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/filters":       # the scan types of a file (scan filters) with their counts: what the filter menu of a cell lists
            return _json(app.filters(int(q["k"])))
        if path == "/api/scaninfo":      # every name / value pair of the header of a scan, as the file has it
            try:
                return _json(app.scaninfo(int(q["k"]), int(q["sid"])))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/scanlist":      # the scans of a level / scan type in columns (the list of scans of the bench)
            try:
                return _json(app.scanlist(int(q["k"]), int(q.get("level", 1)), q.get("filt") or None))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/fileinfo":      # what the mzML says about the file and the instrument
            try:
                return _json(app.fileinfo(int(q["k"])))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/xic":
            return _json(app.xic([int(x) for x in q["k"].split(",") if x], float(q["mz"]),
                                 float(q.get("tol", 0.35)), int(q.get("level", 1))))
        if path == "/api/subxic":        # combined XIC of all the sub-formulas of a formula (high resolution only): measures, no labels
            try:
                return _json(app.subxic(int(q["k"]), q["f"], float(q.get("ppm", 5)), float(q.get("min", 100)), int(q.get("z", 1))))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/origin":        # evidence on where an ion comes from (ionfamily); never a verdict
            return _json(app.origin(int(q.get("k", -1)), float(q["mz"]), float(q["parent"]),
                                    float(q["rt0"]) if q.get("rt0") else None, float(q["rt1"]) if q.get("rt1") else None, q.get("formula")))
        if path == "/api/mrm":
            return _json(app.mrm([int(x) for x in q["k"].split(",") if x]))
        if path == "/api/method_warnings":
            return _json(app.method_warnings())
        if path == "/api/method":
            return _json(app.method(int(q["k"])))
        if path == "/api/nearest_scan":
            return _json(app.nearest_scan(
                int(q["k"]), float(q["rt"]),
                filter=q.get("filter"), level=int(q.get("level", 1)),
                precursor=float(q["precursor"]) if q.get("precursor") else None
            ))
        if path == "/api/spectrum":
            pr = q.get("precursor")
            bg = None
            if q.get("bgk") not in (None, ""):
                bg = {"k": int(q["bgk"]), "rt0": float(q["bgrt0"]), "rt1": float(q["bgrt1"]), "factor": float(q.get("bgf", 1.0))}
            return _json(app.spectrum(int(q["k"]), float(q["rt0"]), float(q["rt1"]), int(q.get("level", 1)),
                                      float(pr) if pr not in (None, "") else None, float(q.get("bin", 0.1)), bg, q.get("merge") == "1", q.get("hr") != "0", q.get("filt") or None))
        if path == "/api/spectra":       # single scans i0..i1 (at most 60), for scan-by-scan navigation
            pr = q.get("prec", q.get("precursor"))
            try:
                return _json(app.spectra(int(q["k"]), int(q["i0"]), int(q["i1"]), int(q.get("level", 1)),
                                         float(pr) if pr not in (None, "") else None, float(q.get("bin", 0.1)), q.get("merge") == "1", q.get("hr") != "0", q.get("filt") or None))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/scanbin":       # the same scans as /api/spectra, as a binary block (no JSON, no decimals): see App.scanbin
            pr = q.get("prec", q.get("precursor"))
            try:
                body = app.scanbin(int(q["k"]), int(q["i0"]), int(q["i1"]), int(q.get("level", 1)), float(pr) if pr not in (None, "") else None,
                                   q.get("filt") or q.get("filter") or None, q.get("hr") != "0", q.get("merge") == "1")
                return 200, "application/octet-stream", body, {}
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path in ("/api/dda", "/api/scan", "/api/scanavg"):       # DDA and single scans as stored in the file (high resolution, B2)
            try:
                if path == "/api/dda":
                    return _json(app.dda(int(q["k"]), q.get("hr") != "0"))
                if path == "/api/scan":
                    return _json(app.scan(int(q["k"]), int(q["sid"]), q.get("hr") != "0"))
                return _json(app.scanavg(int(q["k"]), [int(x) for x in q["sids"].split(",") if x], q.get("hr") != "0"))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/formula":
            try:
                return _json(formula_mz(q.get("f", ""), q.get("adduct") or None))
            except ValueError as e:      # a typo in the formula is the user's, not a server fault
                return _fail(e)
        if path == "/api/composition":    # formulas that fit an exact m/z (mzlab.chem.composition): candidates, never a verdict
            try:
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
            except (ValueError, KeyError, IndexError) as e:
                return _fail(e)
        if path == "/api/contaminants":   # built-in list of known contaminants, expanded into ions (mzlab.chem.contaminants); matching is done in the browser (web/liste.js)
            from .chem.contaminants import builtin
            return _json({"lists": [builtin()]})
        if path == "/api/msntree":      # tree of the fragmentation paths of an MSn file (consensus spectra and formulas): candidates, not identifications
            try:
                return _json(app.msn_tree(int(q["k"]), q.get("formula") or None, float(q.get("ppm", 5))))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/map":
            try:
                return _json(_map(app, q))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/mappunti3d":  # true points of a region (RT, m/z, intensity) for the sticks of the 3D viewer
            try:
                return _json(_map_sticks(app, q))
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/mapsnap":     # marked point of the map: the local maximum near the click (+-0.2 min, +-1 bin)
            try:
                return _json(app._item(int(q["k"])).map_snap(int(q.get("level", 1)), float(q["rt"]), float(q["mz"]), float(q.get("dmz", 1.0))) or {})
            except (ValueError, KeyError) as e:
                return _fail(e)
        if path == "/api/mappunti":    # measures of the marked points (A, B, A - B, A/B, S/N, RT groups, delta m): numbers only
            try:
                return _json(_map_points(app, q))
            except (ValueError, KeyError) as e:
                return _fail(e)
        return _json({"error": "unknown endpoint"}, 404)
    except MemoryError:   # a file too big for the browser's memory: say what to do, not a bare exception name
        return _json(too_big(), 507)
    except Exception as e:  # noqa: BLE001
        return _fail(e, 500) if isinstance(e, UserError) else _json({"error": f"{type(e).__name__}: {e}"}, 500)
