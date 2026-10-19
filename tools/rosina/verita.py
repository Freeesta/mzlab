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

PUNTI_SCANSIONE = 8
PUNTI_CROMATOGRAMMA = 16

# cvParam accessions that matter here (PSI-MS)
MS_LIVELLO, MS_PROFILO, MS_CENTROIDI = "MS:1000511", "MS:1000128", "MS:1000127"
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
METODO_MZML = {
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


def _minuti(valore: str, unita: str):
    x = _float(valore)
    if x is None:
        return None
    return x / 60.0 if unita == SECONDO else x


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
            rt = _minuti(c[0], c[1])
    liv = int(_float(cv.get(MS_LIVELLO, ("1",))[0], 1))
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
        "profilo": MS_PROFILO in cv,
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


def _tipo_cromatogramma(cv: dict, cid: str) -> str:
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


def _cromatogramma(ch: ET.Element) -> dict:
    cv = _cv(ch)
    arr = {}
    unita_t = ""
    for b in _iter(ch, "binaryDataArray"):
        k, v = _array(b)
        if k:
            arr[k] = v
            if k == MS_TIME_ARR:
                unita_t = _cv(b).get(MS_TIME_ARR, ("", "", ""))[1]
    t, y = arr.get(MS_TIME_ARR, []), arr.get(MS_INT_ARR, [])
    if unita_t == SECONDO:
        t = [x / 60.0 for x in t]
    n = len(y)
    idx = sorted({round(i * (n - 1) / (PUNTI_CROMATOGRAMMA - 1)) for i in range(PUNTI_CROMATOGRAMMA)}) if n > 1 else list(range(n))
    cid = ch.get("id", "")
    return {
        "id": cid,
        "tipo": _tipo_cromatogramma(cv, cid),
        "n": n,
        "somma": sum(y),
        "massimo": max(y) if y else 0.0,
        "campione": [[t[i] if i < len(t) else 0.0, y[i]] for i in idx],
    }


def _scelta_punti(scansioni: list) -> list:
    """First, last, median, maximum TIC and one per level, at most 8 distinct indices."""
    n = len(scansioni)
    if n == 0:
        return []
    scelte = [0, n - 1, n // 2, max(range(n), key=lambda i: scansioni[i]["tic"])]
    visti = set()
    for i, s in enumerate(scansioni):
        if s["livello"] not in visti:
            visti.add(s["livello"])
            scelte.append(i)
    out = []
    for i in scelte:
        if i not in out:
            out.append(i)
    return out[:PUNTI_SCANSIONE]


def _metadati(root: ET.Element) -> dict:
    """The header of the mzML as the oracle wrote it: name -> value. Keys follow tools/rosina/mappa_metadati.tsv (`mzml:` prefix)."""
    out = {}

    def put(nome, valore, produttore=None):
        if valore not in (None, ""):
            out[nome] = {"valore": valore, **({"produttore": produttore} if produttore else {})}
            if nome in METODO_MZML:
                out[nome]["metodo"] = METODO_MZML[nome]

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


def da_mzml(path: Path, oracolo: tuple, nativo: Path | None, campione: int | None) -> dict:
    scansioni, cromatogrammi = [], []
    tutte = []
    root = ET.parse(path).getroot()
    if _loc(root.tag) == "indexedmzML":
        root = root.find("{*}mzML")
    for sp in _iter(root, "spectrum"):
        d, mz, inten = _spettro(sp)
        scansioni.append(d)
        tutte.append((mz, inten))
    pt = {}
    for i in _scelta_punti(scansioni):
        pt[str(i)] = {"mz": tutte[i][0], "intensita": tutte[i][1]}
    for ch in _iter(root, "chromatogram"):
        cromatogrammi.append(_cromatogramma(ch))
    conteggi_s, conteggi_c = {}, {}
    for s in scansioni:
        conteggi_s[f"ms{s['livello']}"] = conteggi_s.get(f"ms{s['livello']}", 0) + 1
    for c in cromatogrammi:
        conteggi_c[c["tipo"]] = conteggi_c.get(c["tipo"], 0) + 1
    sha = None
    if nativo and nativo.is_file():
        h = hashlib.sha256()
        with nativo.open("rb") as f:
            for blocco in iter(lambda: f.read(1 << 20), b""):
                h.update(blocco)
        sha = h.hexdigest()
    nome, ver, opz = oracolo
    return {
        "schema": "verita/1",
        "file": {"nome": (nativo or path).name, "sha256": sha, "campione": campione},
        "oracolo": {
            "nome": nome,
            "versione": ver,
            "opzioni": opz,
            "data": datetime.date.today().isoformat(),
            "piattaforma": f"{platform.system()} {platform.machine()}",
        },
        "conteggi": {"spettri": conteggi_s, "cromatogrammi": conteggi_c},
        "scansioni": scansioni,
        "punti": pt,
        "cromatogrammi": cromatogrammi,
        "metadati": _metadati(root),
    }


def valida(doc, schema=None, qui="verita") -> list:
    """Problems of `doc` against `verita-1.json` (the subset of JSON Schema that file uses: type, enum, const, required, properties, additionalProperties, items, oneOf,
    minimum, minItems, maxItems, pattern). An empty list: valid."""
    import re

    if schema is None:
        schema = json.loads((Path(__file__).with_name("verita-1.json")).read_text(encoding="utf-8"))
    tipi = {"object": dict, "array": list, "string": str, "boolean": bool, "null": type(None)}
    errori = []
    if "oneOf" in schema:
        if sum(1 for alt in schema["oneOf"] if not valida(doc, alt, qui)) != 1:
            errori.append(f"{qui}: matches none or several alternatives")
        return errori
    if "const" in schema and doc != schema["const"]:
        errori.append(f"{qui}: must be {schema['const']!r}")
    if "enum" in schema and doc not in schema["enum"]:
        errori.append(f"{qui}: {doc!r} not in {schema['enum']}")
    t = schema.get("type")
    if t is not None:
        ammessi = t if isinstance(t, list) else [t]

        def ok(nome):
            if nome == "integer":
                return isinstance(doc, int) and not isinstance(doc, bool)
            if nome == "number":
                return isinstance(doc, (int, float)) and not isinstance(doc, bool)
            return isinstance(doc, tipi[nome]) and not (nome != "boolean" and isinstance(doc, bool))

        if not any(ok(n) for n in ammessi):
            errori.append(f"{qui}: type {type(doc).__name__} is not {ammessi}")
            return errori
    if isinstance(doc, (int, float)) and not isinstance(doc, bool) and "minimum" in schema and doc < schema["minimum"]:
        errori.append(f"{qui}: below {schema['minimum']}")
    if isinstance(doc, str) and "pattern" in schema and not re.search(schema["pattern"], doc):
        errori.append(f"{qui}: does not match {schema['pattern']}")
    if isinstance(doc, dict):
        for k in schema.get("required", []):
            if k not in doc:
                errori.append(f"{qui}: missing {k}")
        extra = schema.get("additionalProperties")
        for k, v in doc.items():
            if k in schema.get("properties", {}):
                errori += valida(v, schema["properties"][k], f"{qui}.{k}")
            elif extra is False:
                errori.append(f"{qui}: unexpected {k}")
            elif isinstance(extra, dict):
                errori += valida(v, extra, f"{qui}.{k}")
    if isinstance(doc, list):
        if "minItems" in schema and len(doc) < schema["minItems"]:
            errori.append(f"{qui}: fewer than {schema['minItems']} items")
        if "maxItems" in schema and len(doc) > schema["maxItems"]:
            errori.append(f"{qui}: more than {schema['maxItems']} items")
        if "items" in schema:
            for i, v in enumerate(doc):
                errori += valida(v, schema["items"], f"{qui}[{i}]")
    return errori


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
