"""Reference answers of `api/spectrum` and `api/spectra` on PROFILE files for the parity test of the Rust port (crates/mzlab-core/tests/profile_parity.rs).

    python3 tools/profilo_riferimento.py OUT

Writes two synthetic profile Full Scan files with their `.mzidx` in OUT/ and OUT/profilo.json: a list of {k, path, query, status, result, py_s}
with the answer of `api.dispatch`. MZLAB_PROFILE_FILES=a.mzML,b.mzML replaces the synthetic files by real ones. Prints nothing. No compound names.
"""
import json
import os
import sys
import time
from pathlib import Path

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
    ("/api/spectrum", "rt0=14&rt1=14.7&level=1&precursor=&bin=0.1"), ("/api/spectrum", "rt0=14.3&rt1=14.3&level=1"), ("/api/spectrum", "rt0=0&rt1=1000&level=1&merge=1"),
    ("/api/spectrum", "rt0=7&rt1=7.5&level=1&bin=0.5"), ("/api/spectrum", "rt0=500&rt1=600&level=1"), ("/api/spectrum", "rt0=14.7&rt1=14&level=1"),
    ("/api/spectra", "i0=0&i1=0&level=1&bin=0.1"), ("/api/spectra", "i0=150&i1=160&level=1&bin=0.1"), ("/api/spectra", "i0=380&i1=450&level=1"),
    ("/api/spectra", "i0=100&i1=159&level=1&merge=1"), ("/api/spectra", "i0=399&i1=399&level=1"), ("/api/spectra", "i0=9000&i1=9000&level=1"),
    ("/api/spectra", "i0=5&i1=2&level=1"), ("/api/spectra", "i0=0&i1=100&level=1"), ("/api/spectra", "i0=-1&i1=2&level=1"),
]


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    real = [x for x in os.environ.get("MZLAB_PROFILE_FILES", "").split(",") if x]
    names = []
    for i, t in enumerate((0.0, 30.0)):
        f = out / f"s{i}.mzML"
        if real:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            ds.full_scan_profile(f, t, np.random.default_rng(30 + i))
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    res = []
    for k in (0, 1):
        code, _, body = dispatch(app, "GET", "/api/spectra", {"k": str(k), "i0": "280", "i1": "280", "level": "1"})[:3]
        rt = json.loads(body)["scans"][0]["rt"]                                 # a window that holds exactly one scan (the page does this for the scan under the cursor)
        for path, query in QUERIES + [("/api/spectrum", f"rt0={rt - 1e-4}&rt1={rt + 1e-4}&level=1")]:
            q = {"k": str(k), **dict(p.split("=", 1) for p in query.split("&"))}
            t = time.perf_counter()
            code, _, body = dispatch(app, "GET", path, q)[:3]
            res.append({"k": k, "path": path, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body)})
    (out / "profilo.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
