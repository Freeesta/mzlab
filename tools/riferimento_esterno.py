#!/usr/bin/env python3
"""Vendor-reference harness: `mzlab-cli convert` output against an mzML written by an independent converter (msconvert,
ThermoRawFileParser) for the same vendor file. Datasets, tolerances and the expected-fail list are in
`tools/riferimento_esterno.json` (public dataset names only; the files live in the private data repository).
    python3 tools/riferimento_esterno.py [--solo NAME] [--cli PATH] [--json FILE] [--keep DIR]
Without the data repository it prints «SKIPPED: no data» and exits 0. Exit 1 when a check fails that is not expected to,
or when an expected failure passes (the list only shrinks). Standard library only; a dev tool, not part of the program."""
import argparse
import base64
import json
import math
import os
import re
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_percorsi  # noqa: E402

MANIFEST = ROOT / "tools" / "riferimento_esterno.json"

MS_LEVEL, MS_POS, MS_NEG = "MS:1000511", "MS:1000130", "MS:1000129"
MS_CENTROID, MS_PROFILE = "MS:1000127", "MS:1000128"
MS_RT, MS_ISO_TARGET, MS_ISO_LOW, MS_ISO_HIGH = "MS:1000016", "MS:1000827", "MS:1000828", "MS:1000829"
MS_SEL_MZ, MS_CHARGE, MS_CE = "MS:1000744", "MS:1000041", "MS:1000045"
# Energy terms are compared as a value, not as the dissociation method: msconvert writes Thermo's normalized energy
# under MS:1000045, mzLab under MS:1000138 (normalized collision energy).
ENERGY = {MS_CE, "MS:1000138", "MS:1000509", "MS:1002013", "MS:1002680"}
MS_MZ_ARR, MS_INT_ARR, MS_TIME_ARR = "MS:1000514", "MS:1000515", "MS:1000595"
MS_F32, MS_F64, MS_ZLIB = "MS:1000521", "MS:1000523", "MS:1000574"
NUMPRESS = {"MS:1002312", "MS:1002313", "MS:1002314", "MS:1002746", "MS:1002747", "MS:1002748"}
MS_TIC_CHROM, MS_SRM_CHROM, MS_SIM_CHROM = "MS:1000235", "MS:1001473", "MS:1001472"
UO_MIN, UO_SEC = "UO:0000031", "UO:0000010"
CHECKS = ("well_formed", "count", "level", "polarity", "representation", "rt", "precursors", "isolation", "selected_ion", "activation",
          "collision_energy", "points", "mz", "intensity", "chrom_count", "chrom_paired", "chrom_q1q3", "chrom_polarity")


def banner(text: str) -> None:
    line = "=" * max(40, len(text) + 4)
    for out in (sys.stdout, sys.stderr):
        print(f"{line}\n  {text}\n{line}", file=out, flush=True)


# --- mzML reading -----------------------------------------------------------------------------------------------

def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _params(el, groups: dict) -> dict:
    """cvParams of `el` (own and from referenceableParamGroupRef), accession -> (value, unitAccession)."""
    out = {}
    for ch in el:
        t = _local(ch.tag)
        if t == "referenceableParamGroupRef":
            out.update(groups.get(ch.get("ref"), {}))
        elif t == "cvParam":
            out[ch.get("accession")] = (ch.get("value") or "", ch.get("unitAccession") or "")
    return out


def _array(bda, groups: dict):
    """(kind, values) of one binaryDataArray; values is None when the encoding is not read here (numpress)."""
    p = _params(bda, groups)
    kind = MS_MZ_ARR if MS_MZ_ARR in p else MS_INT_ARR if MS_INT_ARR in p else MS_TIME_ARR if MS_TIME_ARR in p else None
    if NUMPRESS & p.keys():
        return kind, None
    raw = next((ch.text or "" for ch in bda if _local(ch.tag) == "binary"), "")
    data = base64.b64decode(raw.strip()) if raw.strip() else b""
    if MS_ZLIB in p and data:
        data = zlib.decompress(data)
    w, f = (4, "f") if MS_F32 in p else (8, "d")
    return kind, list(struct.unpack(f"<{len(data) // w}{f}", data[: len(data) // w * w]))


def _minutes(val: str, unit: str) -> float | None:
    try:
        v = float(val)
    except ValueError:
        return None
    return v / 60.0 if unit == UO_SEC else v


def _polarity(p: dict) -> str | None:
    return "+" if MS_POS in p else "-" if MS_NEG in p else None


def _precursor(pre, groups: dict) -> dict:
    out = {"target": None, "low": None, "high": None, "selected": [], "charge": None, "activation": [], "ce": None}
    for ch in pre.iter():
        t = _local(ch.tag)
        if t == "isolationWindow":
            p = _params(ch, groups)
            for k, acc in (("target", MS_ISO_TARGET), ("low", MS_ISO_LOW), ("high", MS_ISO_HIGH)):
                if acc in p and p[acc][0]:
                    out[k] = float(p[acc][0])
        elif t == "selectedIon":
            p = _params(ch, groups)
            if MS_SEL_MZ in p and p[MS_SEL_MZ][0]:
                out["selected"].append(float(p[MS_SEL_MZ][0]))
            if MS_CHARGE in p and p[MS_CHARGE][0]:
                out["charge"] = int(float(p[MS_CHARGE][0]))
        elif t == "activation":
            p = _params(ch, groups)
            for acc in (MS_CE, "MS:1000138"):
                if out["ce"] is None and acc in p and p[acc][0]:
                    out["ce"] = float(p[acc][0])
            out["activation"] = sorted(a for a in p if a not in ENERGY and a.startswith("MS:"))
    return out


def _target_of(el, name: str, groups: dict) -> float | None:
    for ch in el:
        if _local(ch.tag) == name:
            for iw in ch.iter():
                if _local(iw.tag) == "isolationWindow":
                    p = _params(iw, groups)
                    if MS_ISO_TARGET in p and p[MS_ISO_TARGET][0]:
                        return float(p[MS_ISO_TARGET][0])
    return None


_ATTR = re.compile('="([^"]*[<>][^"]*)"')
_NOT_XML_CHAR = re.compile("[\x00-\x08\x0b\x0c\x0e-\x1f\ud800-\udfff\ufffe\uffff]")


def read_mzml(path: Path) -> dict:
    """Spectra and chromatograms of an mzML (plain or indexed), as plain dicts. A file that is not well-formed XML
    because of characters outside the XML Char production is read from a copy with those characters replaced, and
    or `<`/`>` inside attribute values, and says so in `well_formed`."""
    try:
        out = _read_mzml(str(path))
        out["well_formed"] = True
        return out
    except ET.ParseError as e:
        first = str(e)
    text = _NOT_XML_CHAR.sub("?", path.read_text(encoding="utf-8", errors="surrogatepass"))
    text = _ATTR.sub(lambda m: '="' + m.group(1).replace("<", "&lt;").replace(">", "&gt;") + '"', text)
    with tempfile.NamedTemporaryFile("w", suffix=".mzML", delete=False, encoding="utf-8", errors="replace") as f:
        f.write(text)
    try:
        out = _read_mzml(f.name)
    finally:
        os.unlink(f.name)
    out["well_formed"] = False
    out["xml_error"] = first
    return out


def _read_mzml(path: str) -> dict:
    groups, spectra, chroms = {}, [], []
    for _ev, el in ET.iterparse(path, events=("end",)):
        t = _local(el.tag)
        if t == "referenceableParamGroup":
            groups[el.get("id")] = _params(el, groups)
        elif t == "spectrum":
            p = _params(el, groups)
            s = {"id": el.get("id"), "level": int(p[MS_LEVEL][0]) if MS_LEVEL in p else None, "polarity": _polarity(p),
                 "representation": "centroid" if MS_CENTROID in p else "profile" if MS_PROFILE in p else None,
                 "rt": None, "precursors": [], "mz": None, "intensity": None}
            for ch in el.iter():
                ct = _local(ch.tag)
                if ct == "scan" and s["rt"] is None:
                    q = _params(ch, groups)
                    if MS_RT in q:
                        s["rt"] = _minutes(*q[MS_RT])
                elif ct == "precursor":
                    s["precursors"].append(_precursor(ch, groups))
                elif ct == "binaryDataArray":
                    kind, vals = _array(ch, groups)
                    if kind == MS_MZ_ARR:
                        s["mz"] = vals
                    elif kind == MS_INT_ARR:
                        s["intensity"] = vals
            spectra.append(s)
            el.clear()
        elif t == "chromatogram":
            p = _params(el, groups)
            c = {"id": el.get("id"), "ms": bool({MS_SRM_CHROM, MS_SIM_CHROM} & p.keys()), "polarity": _polarity(p),
                 "q1": _target_of(el, "precursor", groups), "q3": _target_of(el, "product", groups),
                 "time": None, "intensity": None}
            for ch in el.iter():
                if _local(ch.tag) == "binaryDataArray":
                    kind, vals = _array(ch, groups)
                    if kind == MS_TIME_ARR:
                        c["time"] = vals
                    elif kind == MS_INT_ARR:
                        c["intensity"] = vals
            chroms.append(c)
            el.clear()
    return {"spectra": spectra, "chromatograms": chroms}


# --- comparison -------------------------------------------------------------------------------------------------

class Result:
    """Per check: number compared, number failed, first few examples."""

    def __init__(self):
        self.n, self.bad, self.ex = {}, {}, {}

    def add(self, check: str, ok: bool, example: str = "") -> None:
        self.n[check] = self.n.get(check, 0) + 1
        if not ok:
            self.bad[check] = self.bad.get(check, 0) + 1
            if len(self.ex.setdefault(check, [])) < 3:
                self.ex[check].append(example)

    def failed(self) -> set:
        return set(self.bad)


def _close(a, b, tol: float) -> bool:
    if a is None or b is None:
        return a is None and b is None
    return abs(a - b) <= tol


def _ppm_ok(a: float, b: float, ppm: float) -> bool:
    return abs(a - b) <= abs(b) * ppm * 1e-6


_SCAN = re.compile(r"(?:^|\s)scan=(\d+)$")


def pair_spectra(mine: list, ref: list) -> list:
    """Pairs by the vendor scan number (`scan=N` at the end of the native id) when every spectrum on both sides has
    one, otherwise by position; an extra or missing scan then shows in `count` without shifting every later pair."""
    def nums(lst):
        out = [_SCAN.search(s["id"] or "") for s in lst]
        return None if not all(out) else [int(m.group(1)) for m in out]
    na, nb = nums(mine), nums(ref)
    if na is None or nb is None or len(set(na)) != len(na):
        return list(zip(mine, ref))
    by = dict(zip(na, mine))
    return [(by[k], b) for k, b in zip(nb, ref) if k in by]


def compare_spectra(mine: list, ref: list, tol: dict, res: Result, arrays: bool, nonzero: bool = False) -> None:
    """Header checks against `ref`; with `arrays`, also representation and points (the zeros of both sides dropped
    first with `nonzero`, for readers that do not keep zero-intensity records)."""
    res.add("count", len(mine) == len(ref), f"mzLab {len(mine)} vs reference {len(ref)}")
    for i, (a, b) in enumerate(pair_spectra(mine, ref)):
        where = f"#{i} ({b['id']})"
        res.add("level", a["level"] == b["level"], f"{where}: {a['level']} vs {b['level']}")
        res.add("polarity", a["polarity"] == b["polarity"], f"{where}: {a['polarity']} vs {b['polarity']}")
        if arrays and b["representation"] is not None:
            res.add("representation", a["representation"] == b["representation"],
                    f"{where}: {a['representation']} vs {b['representation']}")
        res.add("rt", _close(a["rt"], b["rt"], tol["rt_min"]), f"{where}: {a['rt']} vs {b['rt']} min")
        res.add("precursors", len(a["precursors"]) == len(b["precursors"]),
                f"{where}: {len(a['precursors'])} vs {len(b['precursors'])}")
        for j, (pa, pb) in enumerate(zip(a["precursors"], b["precursors"])):
            w = f"{where} precursor {j}"
            iso = all(_close(pa[k], pb[k], tol["precursor_mz"]) for k in ("target", "low", "high"))
            res.add("isolation", iso, f"{w}: {pa['target']}-{pa['low']}+{pa['high']} vs {pb['target']}-{pb['low']}+{pb['high']}")
            sel = len(pa["selected"]) == len(pb["selected"]) and all(
                abs(x - y) <= tol["precursor_mz"] for x, y in zip(pa["selected"], pb["selected"]))
            res.add("selected_ion", sel, f"{w}: {pa['selected']} vs {pb['selected']}")
            res.add("activation", pa["activation"] == pb["activation"], f"{w}: {pa['activation']} vs {pb['activation']}")
            if pb["ce"] is not None:
                res.add("collision_energy", _close(pa["ce"], pb["ce"], tol["collision_energy"]), f"{w}: {pa['ce']} vs {pb['ce']}")
        if not arrays or b["mz"] is None or b["intensity"] is None:
            continue
        am, ai = a["mz"] or [], a["intensity"] or []
        if nonzero:
            am, ai = _drop_zeros(am, ai)
            b = dict(b, **dict(zip(("mz", "intensity"), _drop_zeros(b["mz"], b["intensity"]))))
        res.add("points", len(am) == len(b["mz"]), f"{where}: {len(am)} vs {len(b['mz'])} points")
        if len(am) != len(b["mz"]):
            continue
        bad_mz = next((k for k, (x, y) in enumerate(zip(am, b["mz"])) if not _ppm_ok(x, y, tol["mz_ppm"])), None)
        res.add("mz", bad_mz is None, f"{where} point {bad_mz}: {am[bad_mz] if bad_mz is not None else ''} vs "
                f"{b['mz'][bad_mz] if bad_mz is not None else ''}")
        top = max((abs(y) for y in b["intensity"]), default=0.0) or 1.0
        bad_i = next((k for k, (x, y) in enumerate(zip(ai, b["intensity"]))
                      if abs(x - y) > tol["intensity_rel"] * max(abs(y), 1e-12 * top)), None)
        res.add("intensity", bad_i is None, f"{where} point {bad_i}: "
                f"{ai[bad_i] if bad_i is not None else ''} vs {b['intensity'][bad_i] if bad_i is not None else ''}")


def _drop_zeros(mz: list, it: list) -> tuple:
    keep = [k for k, v in enumerate(it) if v != 0]
    return [mz[k] for k in keep], [it[k] for k in keep]


def with_centroid_arrays(mine: list, ref: list, cref: list) -> list:
    """`ref` with the arrays of `cref` (the converter's vendor-centroid output) wherever mzLab wrote a centroid
    spectrum: profile is compared with profile, centroids with the vendor's centroids; `representation` still
    compares with `ref`, the file as stored."""
    cent = {b["id"] for a, b in pair_spectra(mine, ref) if a["representation"] == "centroid"}
    by = {c["id"]: c for c in cref}
    return [dict(b, mz=by[b["id"]]["mz"], intensity=by[b["id"]]["intensity"]) if b["id"] in cent and b["id"] in by
            else b for b in ref]


def _data_key(c: dict) -> tuple:
    """Non-zero intensities at 6 significant digits: two chromatograms with this key carry the same trace."""
    return tuple(float(f"{v:.6g}") for v in (c["intensity"] or []) if v != 0)


def _q_class(a, b, tol: dict) -> str:
    if a is None or b is None:
        return "exact" if a is None and b is None else "missing"
    d = abs(a - b)
    return "exact" if d <= tol["q_exact"] else "rounded" if d <= tol["q_rounded"] else "wrong"


def compare_chromatograms(mine: list, ref: list, tol: dict, res: Result) -> None:
    ref = [c for c in ref if c["ms"]]
    mine = [c for c in mine if c["ms"]]
    res.add("chrom_count", len(mine) == len(ref), f"mzLab {len(mine)} vs reference {len(ref)} (SRM and SIM traces)")
    pool = {}
    for c in mine:
        pool.setdefault(_data_key(c), []).append(c)
    for b in ref:
        cands = pool.get(_data_key(b), [])
        res.add("chrom_paired", bool(cands) and bool(_data_key(b)), f"{b['id']}: no mzLab trace with the same data")
        if not cands or not _data_key(b):
            continue
        # Among traces with equal data, the one whose labels are closest.
        a = min(cands, key=lambda c: abs((c["q1"] or 0) - (b["q1"] or 0)) + abs((c["q3"] or 0) - (b["q3"] or 0)))
        cands.remove(a)
        k1, k3 = _q_class(a["q1"], b["q1"], tol), _q_class(a["q3"], b["q3"], tol)
        res.add("chrom_q1q3", k1 == "exact" and k3 == "exact",
                f"{b['id']}: Q1 {a['q1']} vs {b['q1']} ({k1}), Q3 {a['q3']} vs {b['q3']} ({k3})")
        if b["polarity"] is not None:
            res.add("chrom_polarity", a["polarity"] == b["polarity"], f"{b['id']}: {a['polarity']} vs {b['polarity']}")


# --- running ----------------------------------------------------------------------------------------------------

def find_cli(given: str | None) -> Path:
    if given:
        return Path(given)
    for p in (ROOT / "target" / "release" / "mzlab-cli", ROOT / "target" / "debug" / "mzlab-cli"):
        if p.exists():
            return p
    subprocess.run(["cargo", "build", "--release", "-p", "mzlab-cli"], cwd=ROOT, check=True)
    return ROOT / "target" / "release" / "mzlab-cli"


def convert(cli: Path, src: Path, sample: str | None, peaks: str, out: Path) -> Path:
    arg = f"{src}@{sample}" if sample else str(src)
    rep = out / "report.json"
    p = subprocess.run([str(cli), "convert", arg, "--out", str(out), "--compression", "none", "--peaks", peaks,
                        "--fresh", "--workers", "1", "--report", str(rep)], capture_output=True, text=True, timeout=1800)
    rows = json.loads(rep.read_text()) if rep.exists() else []
    good = [r for r in rows if r.get("uscita")]
    if p.returncode != 0 or not good:
        cause = "; ".join(f"{r.get('errore')} {r.get('causa') or ''}".strip() for r in rows if r.get("errore"))
        raise RuntimeError(f"convert exit {p.returncode}: {cause or p.stderr.strip()[-400:]}")
    return Path(good[0]["uscita"])


def run_dataset(d: dict, tol: dict, cli: Path, keep: Path | None) -> dict:
    src, ref = dati_percorsi.percorso(d["input"]), dati_percorsi.percorso(d["reference"])
    ref_arr = dati_percorsi.percorso(d["reference_arrays"]) if d.get("reference_arrays") else None
    tol = {**tol, **d.get("tolerances", {})}
    if src is None or ref is None or (d.get("reference_arrays") and ref_arr is None):
        return {"name": d["name"], "status": "missing", "detail": "input or reference not in the data repository"}
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(keep or tmp) / d["name"].replace(" ", "_")
        out.mkdir(parents=True, exist_ok=True)
        try:
            mine = read_mzml(convert(cli, src, d.get("sample"), d.get("peaks", "file"), out))
        except (RuntimeError, subprocess.TimeoutExpired) as e:
            mine = None
            err = str(e)
        refd = read_mzml(ref)
    res = Result()
    if mine is None:
        res.add("count", False, f"conversion failed: {err}")
    else:
        res.add("well_formed", mine["well_formed"], mine.get("xml_error", ""))
        what = d.get("compare", ["spectra", "chromatograms"])
        nz = bool(d.get("nonzero_points"))
        if "spectra" in what:
            rs = refd["spectra"]
            if "arrays" in what and ref_arr:
                rs = with_centroid_arrays(mine["spectra"], rs, read_mzml(ref_arr)["spectra"])
            compare_spectra(mine["spectra"], rs, tol, res, "arrays" in what, nz)
        if "chromatograms" in what:
            compare_chromatograms(mine["chromatograms"], refd["chromatograms"], tol, res)
    for c in d.get("ignore", {}):
        res.n.pop(c, None), res.bad.pop(c, None), res.ex.pop(c, None)
    expected = d.get("expected", {})
    failed = res.failed()
    unexpected = sorted(failed - expected.keys())
    fixed = sorted(k for k in expected if k not in failed and k in res.n)
    return {"name": d["name"], "status": "fail" if unexpected or fixed else "ok", "checked": res.n, "failed": res.bad,
            "examples": res.ex, "expected": expected, "unexpected": unexpected, "now_passing": fixed,
            "ignored": d.get("ignore", {})}


def validate_manifest(m: dict) -> list[str]:
    """Problems with the manifest itself (unknown checks, missing fields, duplicate names)."""
    errs, seen = [], set()
    for k in ("rt_min", "precursor_mz", "collision_energy", "mz_ppm", "intensity_rel", "q_exact", "q_rounded"):
        if not isinstance(m.get("tolerances", {}).get(k), (int, float)):
            errs.append(f"tolerance {k} missing")
    for d in m.get("datasets", []):
        n = d.get("name", "?")
        for k in ("name", "source", "input", "reference", "producer"):
            if not d.get(k):
                errs.append(f"{n}: field {k} missing")
        if n in seen:
            errs.append(f"{n}: duplicate name")
        seen.add(n)
        errs += [f"{n}: unknown check {c}" for c in d.get("expected", {}) if c not in CHECKS]
        errs += [f"{n}: expected {c}: {why!r} is not a review ID or a listed observation"
                 for c, why in d.get("expected", {}).items()
                 if not (re.fullmatch(r"[A-Z]{2}-\d{2}", why or "") or why in m.get("observations", {}))]
        errs += [f"{n}: ignored check {c} unknown or without a reason" for c, why in d.get("ignore", {}).items()
                 if c not in CHECKS or not why]
        errs += [f"{n}: tolerance override {k} unknown or without a reason" for k in d.get("tolerances", {})
                 if k not in m.get("tolerances", {}) or not d.get("tolerances_why")]
        if d.get("peaks", "file") not in ("file", "profile", "mzlab"):
            errs.append(f"{n}: peaks must be file, profile or mzlab")
    return errs


def report(r: dict) -> None:
    head = {"ok": "OK  ", "fail": "FAIL", "missing": "MISS"}[r["status"]]
    print(f"{head} {r['name']}")
    if r["status"] == "missing":
        print(f"     {r['detail']}")
        return
    for c, why in r.get("ignored", {}).items():
        print(f"     {c}: not compared ({why})")
    for c in CHECKS:
        if c not in r["checked"]:
            continue
        bad = r["failed"].get(c, 0)
        tag = "expected " + r["expected"][c] if c in r["expected"] and bad else "UNEXPECTED" if bad else \
            "now passing: remove from expected" if c in r["expected"] else ""
        if bad or tag:
            print(f"     {c}: {bad}/{r['checked'][c]} differ  [{tag}]")
            for e in r["examples"].get(c, []):
                print(f"         {e}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--solo", action="append", help="only datasets whose name contains this text (repeatable)")
    ap.add_argument("--cli", help="mzlab-cli binary (default target/release, built when missing)")
    ap.add_argument("--json", help="write the full results here")
    ap.add_argument("--keep", help="keep the converted mzML in this folder")
    a = ap.parse_args(argv)
    m = json.loads(MANIFEST.read_text())
    errs = validate_manifest(m)
    if errs:
        print("manifest errors:\n  " + "\n  ".join(errs), file=sys.stderr)
        return 1
    if dati_percorsi.radice() is None:
        banner("SKIPPED: no data (private data repository not found; see tools/dati_percorsi.py)")
        return 0
    sets = [d for d in m["datasets"] if not a.solo or any(s in d["name"] for s in a.solo)]
    cli = find_cli(a.cli)
    results = []
    for d in sets:
        r = run_dataset(d, m["tolerances"], cli, Path(a.keep) if a.keep else None)
        report(r)
        results.append(r)
    if a.json:
        Path(a.json).write_text(json.dumps(results, indent=1))
    missing = [r["name"] for r in results if r["status"] == "missing"]
    if missing:
        banner(f"SKIPPED: no data for {len(missing)} of {len(results)} datasets")
    bad = [r["name"] for r in results if r["status"] == "fail"]
    print(f"\n{len(results) - len(bad) - len(missing)} ok, {len(bad)} fail, {len(missing)} missing")
    return 1 if bad else 0


if __name__ == "__main__":
    os.environ.setdefault("PYTHONHASHSEED", "0")
    sys.exit(main())
