"""Reference answers of `api/dda` and of `api/spectrum` on HIGH-RESOLUTION centroid files for the parity test of the Rust port
(crates/mzlab-core/tests/hr_parity.rs).

    python3 tools/hr_riferimento.py OUT

Writes the synthetic Orbitrap DDA files (Exploris-like, Fusion-like with an ion trap MS2) with their `.mzidx` in OUT/ and OUT/hr.json: a list of
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


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    real = [x for x in os.environ.get("MZLAB_HR_FILES", "").split(",") if x]
    names = []
    for i, kind in enumerate(("exploris", "fusion")):
        f = out / f"h{i}.mzML"
        if real:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            ds.hr_dda(f, np.random.default_rng(31 + i), kind)
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    res = []
    for k in (0, 1):                                   # file k: a DDA file is two items of the session (survey and product ions), 2k and 2k + 1
        item = app._item(2 * k)
        ms2 = [s for s in item.run.scans if s.level > 1 and s.precursor]
        prec = f"{ms2[len(ms2) // 2].precursor:.4f}" if ms2 else "300"
        filt = next((s.filter for s in item.run.scans if s.level == 1 and s.filter), "")
        rt1 = next(s.rt for s in item.run.scans if s.level == 1 and s.rt > 14.0)
        rt2 = next((s.rt for s in item.run.scans if s.level > 1), 1.0)
        one = [("/api/spectrum", f"rt0={rt1 - 1e-4}&rt1={rt1 + 1e-4}&level=1"), ("/api/spectrum", f"rt0={rt2 - 1e-4}&rt1={rt2 + 1e-4}&level=2")]
        for path, query in [(p, q.replace("PREC", prec).replace("FILT", quote(filt))) for p, q in QUERIES] + one:
            q = {"k": str(2 * k), **{a: unquote(b) for a, b in (p.split("=", 1) for p in query.split("&") if p)}}
            t = time.perf_counter()
            code, _, body = dispatch(app, "GET", path, q)[:3]
            res.append({"k": k, "path": path, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body),
                        "single": path == "/api/spectrum" and json.loads(body).get("scans") == 1 and item.hr_on(int(q.get("level", 1)), q.get("hr") != "0")})
    (out / "hr.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
