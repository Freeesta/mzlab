"""Ground truth `verita/1` (docs/studi/ROSINA.md F.1) from an mzML written by an oracle (msconvert, the authors' converter), run on Windows, Mac or Linux.
A tool of the test bench, never part of the site: no calculation of the program lives here.
    python3 tools/rosina/verita.py FILE.mzML [--oracolo NOME VERSIONE OPZIONI] [--nativo FILE] [--campione N] > FILE.verita.json
Standard library only. Binary arrays: 32/64-bit floats, zlib, no numpress (the oracle is asked for plain arrays)."""
import argparse
import base64
import datetime
import hashlib
import json
import platform
import struct
import sys
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path

SCAN_POINTS = 8
PUNTI_CROMATOGRAMMA = 16

# cvParam accessions that matter here (PSI-MS)
MS_LEVEL, MS_PROFILE, MS_CENTROIDS = "MS:1000511", "MS:1000128", "MS:1000127"
MS_POS, MS_NEG = "MS:1000130", "MS:1000129"
MS_RT, MS_TIC, MS_BPMZ, MS_BPI = "MS:1000016", "MS:1000285", "MS:1000504", "MS:1000505"
MS_ISO_TARGET, MS_ISO_LOW, MS_ISO_HIGH = "MS:1000827", "MS:1000828", "MS:1000829"
MS_SEL_MZ, MS_CE = "MS:1000744", "MS:1000045"
MS_MZ_ARR, MS_INT_ARR, MS_TIME_ARR = "MS:1000514", "MS:1000515", "MS:1000595"
MS_F32, MS_F64, MS_ZLIB = "MS:1000521", "MS:1000523", "MS:1000574"
MS_TIC_CHROM, MS_BPC_CHROM, MS_SRM_CHROM, MS_SIM_CHROM = "MS:1000235", "MS:1000628", "MS:1001473", "MS:1001472"
MS_PRESSURE, MS_ABS_CHROM = "MS:1003019", "MS:1000812"
ACTIVATIONS = {
    "MS:1000133": "CID",
    "MS:1000422": "HCD",
    "MS:1000598": "ETD",
    "MS:1000250": "ECD",
    "MS:1000435": "PQD",
    "MS:1000262": "IRMPD",
}
MINUTO = "UO:0000031"
SECONDO = "UO:0000010"


# fields of the mzML header whose place in metodo/1 is known (metodo_mzml.rs); the others are declared "not mapped" by the bench, never silently skipped
METHOD_MZML = {
    "mzml:run@startTimeStamp": "origine.file_creato",
    "mzml:instrument:instrument serial number": "strumento.serie",
}


def _loc(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _iter(elem, nome):
    """Descendants named `nome` in any namespace (ElementTree.iter takes no wildcard)."""
    return (e for e in elem.iter() if _loc(e.tag) == nome)


def _cv(elem) -> dict:
    """The cvParams of an element, by accession: (value, unit accession, name)."""
    out = {}
    for c in elem:
        if _loc(c.tag) == "cvParam":
            out.setdefault(c.get("accession"), (c.get("value", ""), c.get("unitAccession", ""), c.get("name", "")))
    return out


def _float(v, default=None):
    try:
        return float(v)
    except (TypeError, ValueError):
        return default


def _minutes(value: str, unit: str):
    x = _float(value)
    if x is None:
        return None
    return x / 60.0 if unit == SECONDO else x


def _array(b: ET.Element):
    """(kind accession, values) of a binaryDataArray."""
    cv = _cv(b)
    kind = next((a for a in (MS_MZ_ARR, MS_INT_ARR, MS_TIME_ARR) if a in cv), None)
    txt = (b.findtext("{*}binary") or "").strip()
    raw = base64.b64decode(txt) if txt else b""
    if MS_ZLIB in cv:
        raw = zlib.decompress(raw)
    if MS_F32 in cv:
        fmt, w = "f", 4
    elif MS_F64 in cv:
        fmt, w = "d", 8
    else:
        raise ValueError("binary array without a float width (numpress is not accepted)")
    n = len(raw) // w
    return kind, list(struct.unpack("<" + fmt * n, raw[: n * w]))


def _spettro(sp: ET.Element) -> dict:
    cv = _cv(sp)
    kind_vals = {}
    for b in _iter(sp, "binaryDataArray"):
        k, v = _array(b)
        if k:
            kind_vals[k] = v
    mz, inten = kind_vals.get(MS_MZ_ARR, []), kind_vals.get(MS_INT_ARR, [])
    scan = sp.find("{*}scanList/{*}scan")
    rt = None
    if scan is not None:
        c = _cv(scan).get(MS_RT)
        if c:
            rt = _minutes(c[0], c[1])
    liv = int(_float(cv.get(MS_LEVEL, ("1",))[0], 1))
    pol = 1 if MS_POS in cv else -1 if MS_NEG in cv else 0
    d = {
        "id": sp.get("id", ""),
        "rt_min": rt if rt is not None else 0.0,
        "livello": liv,
        "polarita": pol,
        "precursore_mz": None,
        "isolamento": None,
        "ce": None,
        "attivazione": None,
        "profilo": MS_PROFILE in cv,
        "n_punti": len(mz),
        "tic": _float(cv.get(MS_TIC, ("",))[0], sum(inten)),
        "bpi_mz": _float(cv.get(MS_BPMZ, ("",))[0]),
        "bpi": _float(cv.get(MS_BPI, ("",))[0]),
    }
    prec = sp.find("{*}precursorList/{*}precursor")
    if prec is not None:
        iso = prec.find("{*}isolationWindow")
        isocv = _cv(iso) if iso is not None else {}
        sel = prec.find("{*}selectedIonList/{*}selectedIon")
        selcv = _cv(sel) if sel is not None else {}
        act = prec.find("{*}activation")
        actcv = _cv(act) if act is not None else {}
        target = _float(isocv.get(MS_ISO_TARGET, ("",))[0])
        d["precursore_mz"] = _float(selcv.get(MS_SEL_MZ, ("",))[0], target)
        if target is not None:
            lo, hi = _float(isocv.get(MS_ISO_LOW, ("",))[0]), _float(isocv.get(MS_ISO_HIGH, ("",))[0])
            d["isolamento"] = {"mz": target, "inf": lo, "sup": hi}
        d["ce"] = _float(actcv.get(MS_CE, ("",))[0])
        d["attivazione"] = next((n for a, n in ACTIVATIONS.items() if a in actcv), None)
    return d, mz, inten


def _type_chromatogram(cv: dict, cid: str) -> str:
    if MS_TIC_CHROM in cv:
        return "tic"
    if MS_BPC_CHROM in cv:
        return "bpc"
    if MS_SRM_CHROM in cv:
        return "srm"
    if MS_SIM_CHROM in cv:
        return "sim"
    if MS_PRESSURE in cv or cid.lower().startswith("pump pressure"):
        return "pressione"
    if MS_ABS_CHROM in cv:
        return "pda"
    return "altro"


def _chromatogram(ch: ET.Element) -> dict:
    cv = _cv(ch)
    arr = {}
    unit_t = ""
    for b in _iter(ch, "binaryDataArray"):
        k, v = _array(b)
        if k:
            arr[k] = v
            if k == MS_TIME_ARR:
                unit_t = _cv(b).get(MS_TIME_ARR, ("", "", ""))[1]
    t, y = arr.get(MS_TIME_ARR, []), arr.get(MS_INT_ARR, [])
    if unit_t == SECONDO:
        t = [x / 60.0 for x in t]
    n = len(y)
    idx = sorted({round(i * (n - 1) / (PUNTI_CROMATOGRAMMA - 1)) for i in range(PUNTI_CROMATOGRAMMA)}) if n > 1 else list(range(n))
    cid = ch.get("id", "")
    return {
        "id": cid,
        "tipo": _type_chromatogram(cv, cid),
        "n": n,
        "somma": sum(y),
        "massimo": max(y) if y else 0.0,
        "campione": [[t[i] if i < len(t) else 0.0, y[i]] for i in idx],
    }


def _choice_points(scans: list) -> list:
    """First, last, median, maximum TIC and one per level, at most 8 distinct indices."""
    n = len(scans)
    if n == 0:
        return []
    choices = [0, n - 1, n // 2, max(range(n), key=lambda i: scans[i]["tic"])]
    seen = set()
    for i, s in enumerate(scans):
        if s["livello"] not in seen:
            seen.add(s["livello"])
            choices.append(i)
    out = []
    for i in choices:
        if i not in out:
            out.append(i)
    return out[:SCAN_POINTS]


def _metadata(root: ET.Element) -> dict:
    """The header of the mzML as the oracle wrote it: name -> value. Keys follow tools/rosina/mappa_metadati.tsv (`mzml:` prefix)."""
    out = {}

    def put(nome, value, vendor=None):
        if value not in (None, ""):
            out[nome] = {"valore": value, **({"produttore": vendor} if vendor else {})}
            if nome in METHOD_MZML:
                out[nome]["metodo"] = METHOD_MZML[nome]

    for sw in _iter(root, "software"):
        put(f"mzml:software:{sw.get('id')}", sw.get("version"))
    for p in _iter(root, "referenceableParamGroup"):
        for v, _u, n in _cv(p).values():
            put(f"mzml:instrument:{n}", v or True)
    for ic in _iter(root, "instrumentConfiguration"):
        for comp in ic.iter():
            if _loc(comp.tag) in ("source", "analyzer", "detector"):
                for v, _u, n in _cv(comp).values():
                    put(f"mzml:{_loc(comp.tag)}", n)
        for v, _u, n in _cv(ic).values():
            put(f"mzml:instrument:{n}", v or True)
    for sf in _iter(root, "sourceFile"):
        put("mzml:sourceFile:nome", sf.get("name"))
        for v, _u, n in _cv(sf).values():
            put(f"mzml:sourceFile:{n}", v or True)
    run = root.find(".//{*}run")
    if run is not None:
        put("mzml:run@startTimeStamp", run.get("startTimeStamp"))
        put("mzml:run@id", run.get("id"))
    return out


def da_mzml(path: Path, oracle: tuple, nativo: Path | None, sample: int | None) -> dict:
    scans, chromatograms = [], []
    all_items = []
    root = ET.parse(path).getroot()
    if _loc(root.tag) == "indexedmzML":
        root = root.find("{*}mzML")
    for sp in _iter(root, "spectrum"):
        d, mz, inten = _spettro(sp)
        scans.append(d)
        all_items.append((mz, inten))
    pt = {}
    for i in _choice_points(scans):
        pt[str(i)] = {"mz": all_items[i][0], "intensita": all_items[i][1]}
    for ch in _iter(root, "chromatogram"):
        chromatograms.append(_chromatogram(ch))
    counts_s, counts_c = {}, {}
    for s in scans:
        counts_s[f"ms{s['livello']}"] = counts_s.get(f"ms{s['livello']}", 0) + 1
    for c in chromatograms:
        counts_c[c["tipo"]] = counts_c.get(c["tipo"], 0) + 1
    sha = None
    if nativo and nativo.is_file():
        h = hashlib.sha256()
        with nativo.open("rb") as f:
            for block in iter(lambda: f.read(1 << 20), b""):
                h.update(block)
        sha = h.hexdigest()
    nome, ver, opz = oracle
    return {
        "schema": "verita/1",
        "file": {"nome": (nativo or path).name, "sha256": sha, "campione": sample},
        "oracolo": {
            "nome": nome,
            "versione": ver,
            "opzioni": opz,
            "data": datetime.date.today().isoformat(),
            "piattaforma": f"{platform.system()} {platform.machine()}",
        },
        "conteggi": {"spettri": counts_s, "cromatogrammi": counts_c},
        "scansioni": scans,
        "punti": pt,
        "cromatogrammi": chromatograms,
        "metadati": _metadata(root),
    }


def valida(doc, schema=None, here="verita") -> list:
    """Problems of `doc` against `verita-1.json` (the subset of JSON Schema that file uses: type, enum, const, required, properties, additionalProperties, items, oneOf,
    minimum, minItems, maxItems, pattern). An empty list: valid."""
    import re

    if schema is None:
        schema = json.loads((Path(__file__).with_name("verita-1.json")).read_text(encoding="utf-8"))
    types = {"object": dict, "array": list, "string": str, "boolean": bool, "null": type(None)}
    errors = []
    if "oneOf" in schema:
        if sum(1 for alt in schema["oneOf"] if not valida(doc, alt, here)) != 1:
            errors.append(f"{here}: matches none or several alternatives")
        return errors
    if "const" in schema and doc != schema["const"]:
        errors.append(f"{here}: must be {schema['const']!r}")
    if "enum" in schema and doc not in schema["enum"]:
        errors.append(f"{here}: {doc!r} not in {schema['enum']}")
    t = schema.get("type")
    if t is not None:
        allowed = t if isinstance(t, list) else [t]

        def ok(nome):
            if nome == "integer":
                return isinstance(doc, int) and not isinstance(doc, bool)
            if nome == "number":
                return isinstance(doc, (int, float)) and not isinstance(doc, bool)
            return isinstance(doc, types[nome]) and not (nome != "boolean" and isinstance(doc, bool))

        if not any(ok(n) for n in allowed):
            errors.append(f"{here}: type {type(doc).__name__} is not {allowed}")
            return errors
    if isinstance(doc, (int, float)) and not isinstance(doc, bool) and "minimum" in schema and doc < schema["minimum"]:
        errors.append(f"{here}: below {schema['minimum']}")
    if isinstance(doc, str) and "pattern" in schema and not re.search(schema["pattern"], doc):
        errors.append(f"{here}: does not match {schema['pattern']}")
    if isinstance(doc, dict):
        for k in schema.get("required", []):
            if k not in doc:
                errors.append(f"{here}: missing {k}")
        extra = schema.get("additionalProperties")
        for k, v in doc.items():
            if k in schema.get("properties", {}):
                errors += valida(v, schema["properties"][k], f"{here}.{k}")
            elif extra is False:
                errors.append(f"{here}: unexpected {k}")
            elif isinstance(extra, dict):
                errors += valida(v, extra, f"{here}.{k}")
    if isinstance(doc, list):
        if "minItems" in schema and len(doc) < schema["minItems"]:
            errors.append(f"{here}: fewer than {schema['minItems']} items")
        if "maxItems" in schema and len(doc) > schema["maxItems"]:
            errors.append(f"{here}: more than {schema['maxItems']} items")
        if "items" in schema:
            for i, v in enumerate(doc):
                errors += valida(v, schema["items"], f"{here}[{i}]")
    return errors


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mzml", type=Path)
    ap.add_argument("--oracolo", nargs=3, metavar=("NOME", "VERSIONE", "OPZIONI"), default=["mzML", "?", ""])
    ap.add_argument("--nativo", type=Path, help="the native file the mzML comes from (name and SHA-256 go in the truth)")
    ap.add_argument("--campione", type=int)
    a = ap.parse_args(argv)
    d = da_mzml(a.mzml, tuple(a.oracolo), a.nativo, a.campione)
    for problema in valida(d):
        print(problema, file=sys.stderr)
    json.dump(d, sys.stdout, indent=1, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
