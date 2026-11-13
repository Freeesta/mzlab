#!/usr/bin/env python3
"""Times the Flow (index -> detect -> align -> gapfill) on the same synthetic series written in each input format the CLI reads without a vendor file.
`genera OUT BASE_DIR [--n N]` writes OUT/<format>/ from the zlib, non-indexed mzML of BASE_DIR (made by tools/dati_sintetici.py); `misura BIN DIR...` runs
`BIN flow run` on each directory with N workers and prints one Markdown row per directory (MB, s per step, MB/s of the whole). Used by docs/agenti/misure-formati.md."""
import argparse, base64, json, re, struct, subprocess, sys, tempfile, time, zlib
from pathlib import Path


def spectra(text):
    for x in re.findall(r"<spectrum .*?</spectrum>", text, re.S):
        def cv(acc, x=x):
            m = re.search(r'accession="%s"[^>]*?value="([^"]*)"' % acc, x)
            return m.group(1) if m else None
        arrs = {}
        for b in re.findall(r"<binaryDataArray.*?</binaryDataArray>", x, re.S):
            raw = base64.b64decode(re.search(r"<binary>(.*?)</binary>", b, re.S).group(1))
            if "MS:1000574" in b:
                raw = zlib.decompress(raw)
            f = "d" if "MS:1000523" in b else "f"
            vals = struct.unpack("<%d%s" % (len(raw) // struct.calcsize(f), f), raw)
            arrs["mz" if "MS:1000514" in b else "it"] = vals
        yield x, cv, arrs["mz"], arrs["it"]


def mzml_variant(text, compress, indexed):
    """Rewrites the binary arrays of an mzML with or without zlib; `indexed` appends the indexedmzML wrapper msconvert writes."""
    def one(m):
        b = m.group(0)
        raw = base64.b64decode(re.search(r"<binary>(.*?)</binary>", b, re.S).group(1))
        data = zlib.decompress(raw) if "MS:1000574" in b else raw
        if compress:
            return b
        b = b.replace("MS:1000574", "MS:1000576").replace("zlib compression", "no compression")
        return re.sub(r"<binary>.*?</binary>", lambda _: "<binary>%s</binary>" % base64.b64encode(data).decode(), b, flags=re.S)
    out = re.sub(r"<binaryDataArray.*?</binaryDataArray>", one, text, flags=re.S)
    if not indexed:
        return out
    head, body = out.split("<mzML", 1)
    out = head + "<indexedmzML>" + "<mzML" + body
    pos = [m.start() for m in re.finditer(r"<spectrum ", out)]
    ids = re.findall(r'<spectrum index="\d+" id="([^"]*)"', out)
    out_b = out.encode()
    pos = [m.start() for m in re.finditer(rb"<spectrum ", out_b)]
    off = len(out_b)
    idx = '<indexList count="1"><index name="spectrum">' + "".join('<offset idRef="%s">%d</offset>' % (i, p) for i, p in zip(ids, pos)) + "</index></indexList>"
    return out_b.decode() + idx + '<indexListOffset>%d</indexListOffset></indexedmzML>' % off


def mzxml(text, zl):
    sp = list(spectra(text))
    out = ['<?xml version="1.0" encoding="ISO-8859-1"?><mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="%d">' % len(sp)]
    for i, (x, cv, mz, it) in enumerate(sp, 1):
        rt = float(cv("MS:1000016")) * 60
        raw = struct.pack(">%dd" % (2 * len(mz)), *[v for p in zip(mz, it) for v in p])
        attr = ' compressionType="zlib" compressedLen="%d"' % len(zlib.compress(raw)) if zl else ' compressionType="none"'
        if zl:
            raw = zlib.compress(raw)
        out.append('<scan num="%d" msLevel="1" peaksCount="%d" polarity="+" retentionTime="PT%sS" filterLine="%s" lowMz="100.0" highMz="900.0"><peaks precision="64" byteOrder="network" pairOrder="m/z-int"%s>%s</peaks></scan>'
                   % (i, len(mz), repr(rt), cv("MS:1000512") or "", attr, base64.b64encode(raw).decode()))
    return "".join(out) + "</msRun></mzXML>"


def _pad(b):
    return b + b"\0" * (-len(b) % 4)


def _name(s):
    return struct.pack(">i", len(s)) + _pad(s.encode())


def _attrs(items):
    if not items:
        return struct.pack(">ii", 0, 0)
    out = struct.pack(">ii", 12, len(items))
    for k, v in items:
        out += _name(k) + struct.pack(">ii", 2, len(v)) + _pad(v.encode())
    return out


def cdf(text):
    """ANDI/AIA netCDF-3 (CDF1, fixed dimensions) of the centroided scans: mass_values/intensity_values in float, one scan per `scan_index` entry."""
    sp = list(spectra(text))
    mz = [v for _, _, m, _ in sp for v in m]
    it = [v for _, _, _, i in sp for v in i]
    counts = [len(m) for _, _, m, _ in sp]
    starts = [sum(counts[:i]) for i in range(len(counts))]
    times = [float(cv("MS:1000016")) * 60 for _, cv, _, _ in sp]
    tic = [sum(i) for _, _, _, i in sp]
    n, ns = len(mz), len(sp)
    dims = [("point_number", n), ("scan_number", ns)]
    gatt = [("experiment_title", "synthetic"), ("experiment_type", "Centroided Mass Spectrum"), ("test_ionization_polarity", "Positive Polarity"),
            ("test_scan_function", "Mass Scan")]
    vars_ = [("mass_values", 5, 0, struct.pack(">%df" % n, *mz)), ("intensity_values", 5, 0, struct.pack(">%df" % n, *it)),
             ("scan_acquisition_time", 6, 1, struct.pack(">%dd" % ns, *times)), ("total_intensity", 5, 1, struct.pack(">%df" % ns, *tic)),
             ("scan_index", 4, 1, struct.pack(">%di" % ns, *starts)), ("point_count", 4, 1, struct.pack(">%di" % ns, *counts))]
    head = b"CDF\x01" + struct.pack(">i", 0) + struct.pack(">ii", 10, len(dims)) + b"".join(_name(k) + struct.pack(">i", v) for k, v in dims) + _attrs(gatt)
    def vlist(offsets):
        out = struct.pack(">ii", 11, len(vars_))
        for (k, t, d, data), off in zip(vars_, offsets):
            out += _name(k) + struct.pack(">ii", 1, d) + _attrs([]) + struct.pack(">iii", t, len(_pad(data)), off)
        return out
    size = len(head) + len(vlist([0] * len(vars_)))
    offs, o = [], size
    for _, _, _, data in vars_:
        offs.append(o)
        o += len(_pad(data))
    return head + vlist(offs) + b"".join(_pad(d) for _, _, _, d in vars_)


def genera(a):
    out, files = Path(a.out), sorted(Path(a.base).glob("*.mzML"))[: a.n]
    kinds = {"mzml-zlib": ("mzML", lambda t: t), "mzml-zlib-indexed": ("mzML", lambda t: mzml_variant(t, True, True)),
             "mzml-raw": ("mzML", lambda t: mzml_variant(t, False, False)), "mzml-raw-indexed": ("mzML", lambda t: mzml_variant(t, False, True)),
             "mzxml-raw": ("mzXML", lambda t: mzxml(t, False)), "mzxml-zlib": ("mzXML", lambda t: mzxml(t, True))}
    for k, (ext, f) in kinds.items():
        (out / k).mkdir(parents=True, exist_ok=True)
        for p in files:
            (out / k / (p.stem + "." + ext)).write_text(f(p.read_text()), encoding="latin-1" if ext == "mzXML" else "utf-8")
    (out / "cdf").mkdir(parents=True, exist_ok=True)
    for p in files:
        (out / "cdf" / (p.stem + ".cdf")).write_bytes(cdf(p.read_text()))


def misura(a):
    print("| format | files | MB | scans | peaks | index s | detect s | align s | gapfill s | total s | MB/s |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for d in map(Path, a.dirs):
        files = sorted(p for p in d.iterdir() if p.is_file())
        names = [p.name for p in files]
        plan = {"format": "mzflow/2", "inputs": [{"name": n, "type": "raw"} for n in names], "steps": [
            {"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in names]}},
            {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
            {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
            {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]}
        with tempfile.TemporaryDirectory() as t:
            Path(t, "plan.json").write_text(json.dumps(plan))
            s = time.perf_counter()
            r = subprocess.run([a.bin, "flow", "run", t + "/plan.json", "--in", str(d), "--out", t + "/o", "--workers", str(a.workers)], capture_output=True, text=True)
            wall = time.perf_counter() - s
            if r.returncode:
                print("| %s | failed: %s |" % (d.name, (r.stderr or r.stdout).strip().splitlines()[-1][:120] if (r.stderr or r.stdout).strip() else r.returncode))
                continue
            ms = {}
            def walk(x):
                if isinstance(x, dict):
                    if "ms" in x and ("step" in x or "id" in x):
                        k = str(x.get("step", x.get("id"))).split("@")[0]
                        ms[k] = ms.get(k, 0) + x["ms"]
                    for v in x.values():
                        walk(v)
                elif isinstance(x, list):
                    for v in x:
                        walk(v)
            walk(json.loads(Path(t, "o", "prov.json").read_text()))
        mb = sum(p.stat().st_size for p in files) / 1e6
        scans, peaks = a.scans, a.peaks
        print("| %s | %d | %.0f | %s | %s | %.2f | %.2f | %.2f | %.2f | %.2f | %.1f |" % (
            d.name, len(files), mb, scans * len(files), peaks * len(files), ms.get("index.mzidx", 0) / 1e3, ms.get("features.detect", 0) / 1e3,
            ms.get("features.align", 0) / 1e3, ms.get("features.gapfill", 0) / 1e3, wall, mb / wall))


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(required=True)
    g = sub.add_parser("genera"); g.add_argument("out"); g.add_argument("base"); g.add_argument("--n", type=int, default=12); g.set_defaults(f=genera)
    m = sub.add_parser("misura"); m.add_argument("bin"); m.add_argument("dirs", nargs="+"); m.add_argument("--workers", type=int, default=4)
    m.add_argument("--scans", type=int, default=1365); m.add_argument("--peaks", type=int, default=185544); m.set_defaults(f=misura)
    a = ap.parse_args()
    a.f(a)


if __name__ == "__main__":
    sys.exit(main())
