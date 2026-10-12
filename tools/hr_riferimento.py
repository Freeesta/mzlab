"""Reference answers of `api/dda` and of `api/spectrum` on HIGH-RESOLUTION centroid files for the parity test of the Rust port
(crates/mzlab-core/tests/hr_parity.rs).

    python3 tools/hr_riferimento.py OUT

Writes the synthetic Orbitrap DDA files (Exploris-like, Fusion-like with an ion trap MS2, Exploris-like with PROFILE survey scans) with their `.mzidx` in OUT/ and OUT/hr.json: a list of
{k, path, query, status, result, single, py_s} with the answer of `api.dispatch` (`single` = a window of one scan: Rust leaves it to Python, which reads
the centroids of the scan from the file). MZLAB_HR_FILES=a.mzML,b.mzML replaces the synthetic files by real ones. Prints nothing. No compound names.
"""
import json
import os
import sys
import time
from pathlib import Path
from urllib.parse import quote, unquote

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

QUERIES = [
    ("/api/dda", ""), ("/api/dda", "hr=0"),
    ("/api/spectrum", "rt0=14.2&rt1=14.4&level=1"), ("/api/spectrum", "rt0=0&rt1=1000&level=1"), ("/api/spectrum", "rt0=0&rt1=1000&level=1&merge=1"),
    ("/api/spectrum", "rt0=14.2&rt1=14.4&level=1&hr=0"), ("/api/spectrum", "rt0=14.4&rt1=14.2&level=1"), ("/api/spectrum", "rt0=500&rt1=600&level=1"),
    ("/api/spectrum", "rt0=0&rt1=1000&level=2"), ("/api/spectrum", "rt0=0&rt1=1000&level=2&hr=0"), ("/api/spectrum", "rt0=13&rt1=16&level=2&precursor=PREC"),
    ("/api/spectrum", "rt0=13&rt1=16&level=2&precursor=PREC&hr=0"), ("/api/spectrum", "rt0=0&rt1=1000&level=2&precursor=305.0702"),
    ("/api/spectrum", "rt0=0&rt1=1000&level=2&precursor=305.1066"), ("/api/spectrum", "rt0=0&rt1=1000&level=1&filt=FILT"),
]

SPECTRA = [("/api/spectra", q) for q in (
    "level=1&i0=0&i1=3", "level=1&i0=5&i1=70", "level=2&i0=0&i1=2", "level=2&i0=2&i1=1", "level=1&i0=0&i1=2&hr=0", "level=1&i0=0&i1=2&hr=0&merge=1",
    "level=1&i0=1&i1=3&hr=0&bin=0.5", "level=2&i0=0&i1=4&prec=PREC", "level=2&i0=0&i1=2&prec=PREC&hr=0", "level=1&i0=0&i1=2&filt=FILT",
    "level=2&i0=1&i1=3&precursor=PREC", "level=1&i0=100000&i1=100001", "level=1&i0=-1&i1=2", "level=1&i0=0&i1=59")]

SCANBIN = [("/api/scanbin", q) for q in (
    "level=1&i0=0&i1=3", "level=2&i0=0&i1=9", "level=2&i0=2&i1=1", "level=1&i0=0&i1=2&hr=0", "level=1&i0=0&i1=2&hr=0&merge=1", "level=2&i0=0&i1=4&prec=PREC",
    "level=2&i0=0&i1=2&prec=PREC&hr=0", "level=1&i0=0&i1=2&filt=FILT", "level=1&i0=0&i1=2&filt=FILT&hr=0", "level=1&i0=100000&i1=100001", "level=1&i0=0&i1=59")]


def decode_bin(body: bytes) -> dict:
    """What `App.scanbin` sends, as JSON: the header and, for every scan, its m/z (float64) and intensities (float32)."""
    hl = int.from_bytes(body[:4], "little")
    head = json.loads(body[4:4 + hl])
    off, scans = 4 + hl, []
    for m in head["scans"]:
        mz = np.frombuffer(body, "<f8", m["n"], off); off += 8 * m["n"]
        y = np.frombuffer(body, "<f4", m["n"], off); off += 4 * m["n"]
        off += 12 * m["np"]                                          # the profile line (pmz, py) of a profile scan: Rust leaves those to Python
        scans.append({"mz": mz.tolist(), "y": y.astype(float).tolist()})
    return {"head": head, "scans": scans}


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    real = [x for x in os.environ.get("MZLAB_HR_FILES", "").split(",") if x]
    names = []
    for i, kind in enumerate(("exploris", "fusion", "profilo")):      # the third: survey scans in PROFILE (single-precision peak table, spacing read from the mzML)
        f = out / f"h{i}.mzML"
        if real and i < 2:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            ds.hr_dda(f, np.random.default_rng(31 + i), "exploris" if kind == "profilo" else kind, profile=kind == "profilo")
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    res = []
    for k in (0, 1, 2):                                # file k: a DDA file is two items of the session (survey and product ions), 2k and 2k + 1
        item = app._item(2 * k)
        ms2 = [s for s in item.run.scans if s.level > 1 and s.precursor]
        prec = f"{ms2[len(ms2) // 2].precursor:.4f}" if ms2 else "300"
        filt = next((s.filter for s in item.run.scans if s.level == 1 and s.filter), "")
        rt1 = next(s.rt for s in item.run.scans if s.level == 1 and s.rt > 14.0)
        rt2 = next((s.rt for s in item.run.scans if s.level > 1), 1.0)
        one = [("/api/spectrum", f"rt0={rt1 - 1e-4}&rt1={rt1 + 1e-4}&level=1"), ("/api/spectrum", f"rt0={rt2 - 1e-4}&rt1={rt2 + 1e-4}&level=2")]
        s1 = [s.index for s in item.run.scans if s.level == 1]
        s2 = [s.index for s in item.run.scans if s.level > 1]
        scans = [("/api/scan", f"sid={s1[3]}"), ("/api/scan", f"sid={s2[5]}"), ("/api/scan", f"sid={s2[5]}&hr=0"), ("/api/scan", f"sid={s1[0]}&hr=0"),
                 ("/api/scan", "sid=-1"), ("/api/scan", "sid=99999999"),
                 ("/api/scanavg", "sids=" + ",".join(map(str, s2[3:8]))), ("/api/scanavg", "sids=" + ",".join(map(str, s1[2:9])) + "&hr=0"),
                 ("/api/scanavg", "sids=" + ",".join(map(str, s1[:3] + s1[:3]))), ("/api/scanavg", "sids="), ("/api/scanavg", "sids=-4,99999999"),
                 ("/api/scanavg", "sids=" + ",".join(map(str, range(201))))]
        for path, query in [(p, q.replace("PREC", prec).replace("FILT", quote(filt))) for p, q in QUERIES + SPECTRA + SCANBIN] + one + scans:
            q = {**{a: unquote(b) for a, b in (p.split("=", 1) for p in query.split("&") if p)}}
            q = {"k": str(2 * k + (q.get("level") == "2")), **q}               # the product ions are the second item of the file
            t = time.perf_counter()
            code, _, body = dispatch(app, "GET", path, q)[:3]
            result = decode_bin(body) if path == "/api/scanbin" and code == 200 else json.loads(body)
            res.append({"k": k, "path": path, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": result,
                        "single": path == "/api/spectrum" and result.get("scans") == 1 and item.hr_on(int(q.get("level", 1)), q.get("hr") != "0")})
    (out / "hr.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
