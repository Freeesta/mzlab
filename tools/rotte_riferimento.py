"""Reference answers of the Python engine for the routes that Rust answers from the `.mzidx` (crates/mzlab-core/tests/routes_parity.rs).

    python3 tools/rotte_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic mzML files of tools/mzidx_riferimento.py in OUT/, their `.mzidx` and, for each file (those and the given ones),
OUT/<stem>.rotte.json = [{path, query, status, body}] with the answer of `mzlab.api.dispatch` (one whole-file item, as the page opens a
plain file), and prints "<mzML path>" lines. No compound names.
"""
import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Item  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def queries(item: Item) -> list[tuple[str, dict]]:
    """The queries worth comparing for this file."""
    out = []
    scans = item.run.scans
    levels = sorted({s.level for s in scans})
    keys = {}
    for s in scans:
        keys.setdefault((s.level, __import__("mzlab.explore", fromlist=["scan_key"]).scan_key(s)), s)
    out.append(("/api/filters", {"k": "0"}))
    n = len(scans)
    for sid in sorted({0, n // 3, n // 2, n - 1}) if n else []:
        out.append(("/api/scaninfo", {"k": "0", "sid": str(sid)}))
    for lv in levels:
        base = {"k": "0", "level": str(lv)}
        out += [("/api/chrom", {**base}), ("/api/chrom", {**base, "kind": "bpc"})]
        out += [("/api/chrom", {**base, "mz0": "100", "mz1": "300"}), ("/api/chrom", {**base, "mz0": "150"}),
                ("/api/chrom", {**base, "mz1": "250", "kind": "bpc"}), ("/api/chrom", {**base, "mz0": "120", "mz1": "280", "kind": "bpc"})]
        out += [("/api/scanlist", {**base})]
        if item.spectrum_mode(lv) == "centroid" and not item.hr_on(lv, True):       # the spectra of a window: unit-resolution centroids (the rest is Python's)
            for r0, r1 in ((0, 1), (1.3, 2.9), (3, 3.0), (0, 1000), (5.5, 2.2), (500, 600)):
                for extra in ({}, {"merge": "1"}, {"bin": "0.5"}):
                    out.append(("/api/spectrum", {**base, "rt0": str(r0), "rt1": str(r1), **extra}))
            for p0 in sorted({round(s.precursor, 1) for s in scans if s.level == lv and s.precursor})[:2] if lv > 1 else []:
                out.append(("/api/spectrum", {**base, "rt0": "0", "rt1": "1000", "precursor": str(p0)}))
            for (l2, key) in list(keys)[:4]:
                if l2 == lv:
                    out.append(("/api/spectrum", {**base, "rt0": "0", "rt1": "1000", "filt": key, "merge": "1"}))
        precs = sorted({round(s.precursor, 1) for s in scans if s.level == lv and s.precursor})[:3]
        for p in precs:
            out += [("/api/chrom", {**base, "prec": str(p)}), ("/api/nearest_scan", {**base, "rt": "3.3", "precursor": str(p)})]
        for (l2, key) in list(keys)[:6]:
            if l2 == lv:
                out += [("/api/chrom", {**base, "filt": key}), ("/api/scanlist", {**base, "filt": key}), ("/api/nearest_scan", {**base, "rt": "5.1", "filter": key})]
        for rt in ("0", "2.2", "9.9", "1000"):
            out.append(("/api/nearest_scan", {**base, "rt": rt}))
    t = item.run.table(levels[0], None) if levels else None
    if t is not None and len(t.mz):
        import numpy as np
        mzs = np.asarray(t.mz[: len(t.mz)])
        for q in (0.1, 0.4, 0.7, 0.95):
            mz = float(mzs[int(len(mzs) * q)])
            for tol in (0.35, 0.5, 0.02):
                out.append(("/api/xic", {"k": "0", "mz": repr(round(mz, 4)), "tol": str(tol), "level": str(levels[0])}))
        out.append(("/api/xic", {"k": "0", "mz": "99999", "tol": "0.35", "level": str(levels[0])}))
    return out


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        item = Item(f.name, None, None, "x", f)
        app = App()
        app.session = type("S", (), {"items": [item]})()
        rows = []
        extra = [("/api/method", {"k": "0"}), ("/api/delay", {"k": "0"}), ("/api/chrom", {"k": "0", "kind": "pda"}), ("/api/analog", {"k": "0", "i": "99"})]
        for a in item.analog_list():
            extra += [("/api/analog", {"k": "0", "i": str(a["i"]), "n": n}) for n in ("2000", "50", "5")]
        for path, q in (queries(item) if item.run.scans else []) + [("/api/fileinfo", {"k": "0"}), ("/api/mrm", {"k": "0"})] + extra:
            status, _ctype, body, _h = dispatch(app, "GET", path, q)
            rows.append({"path": path, "query": q, "status": status, "body": json.loads(body)})
        from mzlab.explore import file_parts
        for part in file_parts(item.run):        # the experiments of a mixed file: each one is an item of the session
            pit = Item(f"{f.name}#{part['tag']}", None, None, "x", f, part=part, run=item.run)
            papp = App()
            papp.session = type("S", (), {"items": [pit]})()
            status, _ctype, body, _h = dispatch(papp, "GET", "/api/method", {"k": "0"})
            rows.append({"path": "/api/method", "query": {"k": "0"}, "file": pit.file, "status": status, "body": json.loads(body)})
        (out / (f.stem + ".rotte.json")).write_text(json.dumps(rows), encoding="utf-8")
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
