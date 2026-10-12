"""Golden data: the results of the Python engine frozen in JSON (tests/golden/), the reference for any other engine (Rust).

    python3 tools/golden.py --crea        # recompute and WRITE tests/golden/*.json (only when the reference engine changes on purpose)
    python3 tools/golden.py --controlla   # recompute and COMPARE with the JSON; exit 1 on any difference
    python3 tools/golden.py --sintetici DIR   # write only the two synthetic mzML files (the Rust tests read them)

Sources: the anonymous example files of mzlab/web/esempi (Full Scan, MS2, MRM) and two synthetic files written by
tools/dati_sintetici.py with a fixed seed (a Full Scan and an MRM, peaks of known shape). No compound names.
Per file: TIC, XIC of 5 fixed m/z (window [n-0.2, n+0.8], written in the JSON), limits and areas of the XIC peaks,
spectral entropy similarity (Li & Fiehn 2021) of 5 MS2 pairs; pressure: unit -> bar for 13 units x 4 sets of values (note key included) and the reduced analog trace of a synthetic file. Comparison order: peak limits (identical indices), then
areas (1e-4 relative), m/z (1e-6), entropy (1e-9).
"""
from __future__ import annotations

import argparse
import json
import math
import sys
import tempfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tools"))

from mzlab.explore import Item                      # noqa: E402
from mzlab.ionfamily import detect_peaks            # noqa: E402

GOLDEN = ROOT / "tests" / "golden"
EXAMPLES = ROOT / "mzlab" / "web" / "esempi"
XIC_MZ = [152, 194, 229, 305, 364]                  # nominal m/z of the XIC (window n-0.2 .. n+0.8)
XIC_LO, XIC_HI = -0.2, 0.8
FRAG_TOL = 0.5                                      # Da, matching of the fragments in the entropy similarity
TOL_AREA, TOL_MZ, TOL_ENT = 1e-4, 1e-6, 1e-9


def _item(path: Path) -> Item:
    return Item(path.name, None, None, "x", path)


MIN_REL_HEIGHT = 0.05          # noise peaks (a few thousandths of the maximum) flip with the last digits of the numpy build: not frozen


def _peaks(rt, y) -> list[dict]:
    r = detect_peaks(rt, y)
    top = float(np.max(y)) if len(y) else 0.0
    return sorted(({"apex_i": p["apex_i"], "lo": p["lo"], "hi": p["hi"], "area": p["area"]}
                   for p in r["peaks"] if p["height"] >= MIN_REL_HEIGHT * top), key=lambda p: p["apex_i"])


def _trace(rt, y) -> dict:
    # the peaks come from the exact data; the stored traces are rounded (RT 1e-6 min, y 7 significant digits) to keep the JSON small
    return {"rt": [round(float(v), 6) for v in rt], "y": [float(f"{float(v):.7g}") for v in y], "peaks": _peaks(rt, y)}


# ----------------------------------------------------------------------------------------- spectral entropy (reference)
def _entropy(p) -> float:
    return -sum(v * math.log(v) for v in p if v > 0)


def _weighted(it) -> list[float]:
    s = float(sum(it))
    p = [v / s for v in it]
    S = _entropy(p)
    if S < 3:
        w = 0.25 + 0.25 * S
        p = [v ** w for v in p]
        t = sum(p)
        p = [v / t for v in p]
    return p


def _match(a_mz, a_it, b_mz, b_it, tol) -> list[tuple[int, int]]:
    """One-to-one pairs within tol (Da), the most intense pairs first (ties: lower i, then lower j)."""
    cand = [(i, j, a_it[i] * b_it[j]) for i in range(len(a_mz)) for j in range(len(b_mz)) if abs(a_mz[i] - b_mz[j]) <= tol]
    cand.sort(key=lambda c: (-c[2], c[0], c[1]))
    ua, ub, out = set(), set(), []
    for i, j, _ in cand:
        if i not in ua and j not in ub:
            ua.add(i); ub.add(j); out.append((i, j))
    return out


def entropy_similarity(a_mz, a_it, b_mz, b_it, tol=FRAG_TOL) -> float:
    pa, pb = _weighted(a_it), _weighted(b_it)
    pairs = _match(a_mz, a_it, b_mz, b_it, tol)
    ia, ib = {i for i, _ in pairs}, {j for _, j in pairs}
    m = [(pa[i] + pb[j]) / 2 for i, j in pairs] + [v / 2 for i, v in enumerate(pa) if i not in ia] + [v / 2 for j, v in enumerate(pb) if j not in ib]
    s = 1 - (2 * _entropy(m) - _entropy(pa) - _entropy(pb)) / math.log(4)
    return max(0.0, min(1.0, s))


def _ms2_pairs(it: Item, n: int = 10) -> list[dict]:
    """n pairs of MS2 spectra of one file: scans with >= 8 ions, picked at even steps; the last pair is a spectrum with itself."""
    t = it._tbl(2)
    counts = np.bincount(t.pos, minlength=len(t.rt))
    ok = [int(k) for k in np.flatnonzero(counts >= 8)]
    if len(ok) < n:
        raise SystemExit(f"{it.path.name}: too few MS2 spectra with >= 8 ions ({len(ok)})")
    pick = [ok[int(round(v))] for v in np.linspace(0, len(ok) - 1, n)]
    pairs = []
    for k in range(n):
        a, b = pick[k], pick[(k + 1) % n] if k < n - 1 else pick[k]
        sp = []
        for scan in (a, b):
            m = t.pos == scan
            order = np.argsort(t.mz[m], kind="stable")
            sp.append({"scan": scan, "mz": [float(v) for v in t.mz[m][order]], "it": [float(v) for v in t.inten[m][order]]})
        pairs.append({"a": sp[0], "b": sp[1], "entropy": entropy_similarity(sp[0]["mz"], sp[0]["it"], sp[1]["mz"], sp[1]["it"])})
    return pairs


# ----------------------------------------------------------------------------------------------------------- the files
def _full(path: Path) -> dict:
    it = _item(path)
    rt, tic = it.total("tic")
    out = {"kind": "full", "n_scans": int(len(rt)), "tic": _trace(rt, tic), "xic": []}
    for n in XIC_MZ:
        lo, hi = n + XIC_LO, n + XIC_HI
        rt, y = it.xic((lo + hi) / 2, (hi - lo) / 2, 1)
        out["xic"].append({"mz": n, "mz_lo": lo, "mz_hi": hi, **_trace(rt, y)})
    return out


def _ms2(path: Path) -> dict:
    it = _item(path)
    rt, tic = it.total("tic", 2)
    return {"kind": "ms2", "n_scans": int(len(rt)), "tic": _trace(rt, tic), "entropy_pairs": _ms2_pairs(it, 10)}


def _hrms(path: Path) -> dict:
    it = _item(path)
    rt, tic = it.total("tic")
    apex_idx = int(np.argmax(tic))
    t1 = it._tbl(1)
    mask = t1.pos == apex_idx
    top_idx = np.argsort(t1.inten[mask])[-5:][::-1]
    top_mzs = sorted([round(float(v), 5) for v in t1.mz[mask][top_idx]])
    xics = []
    for mz in top_mzs:
        lo, hi = round(mz - 0.01, 5), round(mz + 0.01, 5)
        rt_x, y = it.xic(mz, 0.01, 1)
        xics.append({"mz": mz, "mz_lo": lo, "mz_hi": hi, **_trace(rt_x, y)})
    out = {"kind": "hrms", "n_scans": int(len(rt)), "tic": _trace(rt, tic), "xic": xics}
    t2 = it._tbl(2)
    if len(t2.rt) and len(t2.mz):
        try:
            out["entropy_pairs"] = _ms2_pairs(it, 10)
        except SystemExit:
            pass
    return out


def _mrm(path: Path) -> dict:
    it = _item(path)
    chroms = []
    for c in it.run.chromatograms():
        if c["kind"] != "srm":                       # TIC, BPC and PDA traces of an MRM file are not frozen (size)
            continue
        rt, y = it.run.chromatogram(c["index"])
        chroms.append({"id": str(c.get("id", c["index"])), "kind": c["kind"], **_trace(rt, y)})
    return {"kind": "mrm", "chromatograms": chroms}


PRESSURE_UNITS = ["pascal", "Pa", "PASCAL", "kPa", "kilopascal", "MPa", "megapascal", "psi", "pound per square inch", "bar", "", None, "unknown"]
PRESSURE_VALUES = {"normal": [0.0, 120.5, 250.0, 399.9], "x10": [2.78e7, 1.0e8, 2.78e8], "empty": [], "negative": [-5.0, 3.0]}


def _pressure(folder: Path) -> dict:
    """Pressure: unit -> bar for every unit x value set (values, note key), and the analog trace of a synthetic file declaring pascal (reduced to about 2 n points)."""
    import dati_sintetici as ds
    from mzlab.analog import pressure_to_bar
    cases = []
    for unit in PRESSURE_UNITS:
        for name, vals in PRESSURE_VALUES.items():
            y, note = pressure_to_bar(vals, unit)
            cases.append({"unit": unit, "set": name, "bar": [float(v) for v in y], "note": note["key"] if note else None})
    p = folder / "sintetico_Pressione.mzML"
    ds.full_scan(p, 0, np.random.default_rng(2026), pressure=True)
    it = _item(p)
    meta = it.analog_list()
    a = it.analog(meta[0]["i"], 200)
    return {"kind": "pressure", "cases": cases, "list": meta,
            "analog": {"unit": a["unit"], "note": (a.get("unit_note") or {}).get("key"), "t": a["t"], "y": a["y"]}}


def write_synthetic(out: Path) -> dict[str, Path]:
    """The three synthetic files of the golden data (fixed seeds), for the tests of other engines."""
    import dati_sintetici as ds
    out.mkdir(parents=True, exist_ok=True)
    full, ms2, mrm = out / "sintetico_FullScan.mzML", out / "sintetico_MS2.mzML", out / "sintetico_MRM.mzML"
    ds.full_scan(full, 15, np.random.default_rng(2026))
    ds.ms2(ms2, np.random.default_rng(2026), 15)
    ds.mrm(mrm, 1.0e4 * 2.4, np.random.default_rng(2026), 3)
    return {"sintetico_FullScan": full, "sintetico_MS2": ms2, "sintetico_MRM": mrm}


def _diff(folder: Path) -> dict:
    """Difference from a reference on the synthetic subtraction series (t10 minus t0, tolerance 0.1 min): TIC and the XIC of the parent and the product."""
    import dati_sintetici as ds
    from mzlab.differenza import subtract_trace
    a, b = (_item(p) for p in ds.subtraction_series(folder, blank=False)[:2])
    chroms = []
    for name, get in (("tic", lambda it: it.total("tic")), ("xic253", lambda it: it.xic(253.3, 0.5, 1)), ("xic301", lambda it: it.xic(301.3, 0.5, 1))):
        (rt, y), (rr, ry) = get(b), get(a)
        d = subtract_trace(rt, y, rr, ry, 1.0, 0.1)
        ok = ~np.isnan(d)
        chroms.append({"id": name, "kind": "diff", **_trace(np.asarray(rt)[ok], d[ok])})
    return {"kind": "diff", "chromatograms": chroms}


def compute(skip_missing: bool = False) -> dict[str, dict]:
    """name -> result for every golden file."""
    res: dict[str, dict] = {}
    for stem, fn in [
        ("FullScan_t10", _full),
        ("MS2_t15", _ms2),
        ("MRM_std_2.4ppm", _mrm),
    ]:
        p = EXAMPLES / f"{stem}.mzML"
        if p.exists():
            res[f"esempio_{stem}"] = fn(p)
        elif not skip_missing:
            raise FileNotFoundError(f"{p} missing")

    for f in sorted(EXAMPLES.glob("HRMS_*.mzML")):
        res[f"esempio_{f.stem}"] = _hrms(f)

    with tempfile.TemporaryDirectory() as d:
        files = write_synthetic(Path(d))
        res["sintetico_FullScan"] = _full(files["sintetico_FullScan"])
        res["sintetico_MS2"] = _ms2(files["sintetico_MS2"])
        res["sintetico_MRM"] = _mrm(files["sintetico_MRM"])
        res["sintetico_Differenza"] = _diff(Path(d) / "sub")
        res["sintetico_Pressione"] = _pressure(Path(d))
    return res


# ------------------------------------------------------------------------------------------------------- comparison
def _close(a, b, rel, absol=0.0) -> bool:
    return abs(a - b) <= max(absol, rel * max(abs(a), abs(b)))


def _cmp_trace(name, new, old, bad: list[str]) -> None:
    if len(new["rt"]) != len(old["rt"]):
        bad.append(f"{name}: {len(new['rt'])} points instead of {len(old['rt'])}"); return
    pn, po = new["peaks"], old["peaks"]
    if [(p["apex_i"], p["lo"], p["hi"]) for p in pn] != [(p["apex_i"], p["lo"], p["hi"]) for p in po]:
        bad.append(f"{name}: peak limits differ {[(p['apex_i'], p['lo'], p['hi']) for p in pn]} vs {[(p['apex_i'], p['lo'], p['hi']) for p in po]}"); return
    for k, (a, b) in enumerate(zip(pn, po)):
        if not _close(a["area"], b["area"], TOL_AREA):
            bad.append(f"{name}: peak {k} area {a['area']} vs {b['area']}")
    top = max([abs(v) for v in old["y"]] + [1.0])
    if any(not _close(a, b, TOL_AREA, 1e-9 * top) for a, b in zip(new["y"], old["y"])):
        bad.append(f"{name}: trace differs")
    if any(not _close(a, b, 0, 2e-6) for a, b in zip(new["rt"], old["rt"])):
        bad.append(f"{name}: RT differs")


def _cmp_pressure(name, a, b, bad: list[str]) -> None:
    if len(a["cases"]) != len(b["cases"]):
        bad.append(f"{name}: {len(a['cases'])} cases instead of {len(b['cases'])}"); return
    for ca, cb in zip(a["cases"], b["cases"]):
        tag = f"{name}/{cb['unit']!r}/{cb['set']}"
        if ca["note"] != cb["note"] or len(ca["bar"]) != len(cb["bar"]) or any(not _close(x, y, 1e-12, 1e-12) for x, y in zip(ca["bar"], cb["bar"])):
            bad.append(f"{tag}: {ca['bar']} {ca['note']} vs {cb['bar']} {cb['note']}")
    if a["list"] != b["list"]:
        bad.append(f"{name}/list: {a['list']} vs {b['list']}")
    ta, tb = a["analog"], b["analog"]
    if (ta["unit"], ta["note"], len(ta["t"])) != (tb["unit"], tb["note"], len(tb["t"])) or any(not _close(x, y, 1e-9, 1e-9) for k in "ty" for x, y in zip(ta[k], tb[k])):
        bad.append(f"{name}/analog: unit, note, points or values differ")


def controlla(new: dict[str, dict], old: dict[str, dict], skip_missing: bool = False) -> list[str]:
    bad: list[str] = []
    keys = (set(old) & set(new)) if skip_missing else (set(old) | set(new))
    for name in sorted(keys):
        if name not in old or name not in new:
            bad.append(f"{name}: missing in {'golden' if name not in old else 'engine'}")
            continue
        a, b = new[name], old[name]
        if b.get("kind") == "pressure":
            _cmp_pressure(name, a, b, bad); continue
        for k in ("tic",):
            if k in b:
                _cmp_trace(f"{name}/{k}", a[k], b[k], bad)
        for xa, xb in zip(a.get("xic", []), b.get("xic", [])):
            if not (_close(xa["mz_lo"], xb["mz_lo"], 0, TOL_MZ) and _close(xa["mz_hi"], xb["mz_hi"], 0, TOL_MZ)):
                bad.append(f"{name}/xic {xb['mz']}: window differs")
            _cmp_trace(f"{name}/xic {xb['mz']}", xa, xb, bad)
        for ca, cb in zip(a.get("chromatograms", []), b.get("chromatograms", [])):
            _cmp_trace(f"{name}/{cb['id']}", ca, cb, bad)
        for k, (pa, pb) in enumerate(zip(a.get("entropy_pairs", []), b.get("entropy_pairs", []))):
            for side in "ab":
                if any(not _close(x, y, 0, TOL_MZ) for x, y in zip(pa[side]["mz"], pb[side]["mz"])):
                    bad.append(f"{name}/entropy {k}: m/z differ")
            if abs(pa["entropy"] - pb["entropy"]) > TOL_ENT:
                bad.append(f"{name}/entropy {k}: {pa['entropy']} vs {pb['entropy']}")
    return bad


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--crea", action="store_true")
    g.add_argument("--controlla", action="store_true")
    g.add_argument("--sintetici", metavar="DIR", help="only write the two synthetic mzML files of the golden data in DIR")
    ap.add_argument("--skip-missing", action="store_true", help="skip files whose source .mzML is missing")
    args = ap.parse_args(argv)
    if args.sintetici:
        for p in write_synthetic(Path(args.sintetici)).values():
            print(p)
        return 0
    new = compute(skip_missing=args.skip_missing)
    if args.crea:
        GOLDEN.mkdir(parents=True, exist_ok=True)
        for name, data in new.items():
            (GOLDEN / f"{name}.json").write_text(json.dumps(data, separators=(",", ":")) + "\n", encoding="utf-8")
        print(f"written {len(new)} files in {GOLDEN}")
        return 0
    old = {p.stem: json.loads(p.read_text(encoding="utf-8")) for p in sorted(GOLDEN.glob("*.json"))}
    if not old:
        print("no golden files: run --crea"); return 1
    bad = controlla(new, old, skip_missing=args.skip_missing)
    for b in bad:
        print("FAIL", b)
    print(f"golden: {'OK' if not bad else str(len(bad)) + ' differences'} ({len(old)} files)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
