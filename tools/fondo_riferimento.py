"""Reference answers of `api/spectrum` with a background (`bgk`) for the parity test of the Rust port (crates/mzlab-core/tests/specdiff_parity.rs).

    python3 tools/fondo_riferimento.py OUT

Writes synthetic sessions with their `.mzidx` in OUT/ (low-resolution centroids, low-resolution profile, high-resolution centroids) and OUT/fondo.json:
a list of {session, files, k, bgk, fk, fbk, path, query, status, result}: the answer of `api.dispatch` for the item `k` (of file `fk` of the session)
with the background of item `bgk` (file `fbk`). MZLAB_HR_FILES=a.mzML,b.mzML replaces the synthetic high-resolution files. Prints nothing. No compound names.
"""
import json
import os
import sys
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

LR = ["level=1&rt0=14&rt1=14.7&bgrt0=14&bgrt1=14.7", "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&bgf=0.5", "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&bgclip=0",
      "level=1&rt0=14.7&rt1=14&bgrt0=3&bgrt1=1&bin=0.5", "level=1&rt0=14&rt1=14.7&bgrt0=500&bgrt1=600", "level=1&rt0=500&rt1=600&bgrt0=1&bgrt1=3",
      "level=1&rt0=14.2&rt1=14.4&bgrt0=1&bgrt1=3&bgclip=0", "level=1&rt0=14&rt1=14.7", "level=1&rt0=14&rt1=14.7&bgrt0=14", "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&hr=0"]
HR = ["level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3", "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&bgtol=2&bgf=2", "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&bgclip=0",
      "level=1&rt0=14&rt1=14.7&bgrt0=1&bgrt1=3&hr=0", "level=2&rt0=13&rt1=16&bgrt0=1&bgrt1=3&precursor=PREC", "level=2&rt0=13&rt1=16&bgrt0=1&bgrt1=3&precursor=PREC&hr=0",
      "level=1&rt0=14.3&rt1=14.3&bgrt0=1&bgrt1=3", "level=1&rt0=14&rt1=14.7&bgrt0=500&bgrt1=600"]


def build(out: Path, name: str, make) -> str:
    f = out / name
    make(f)
    run = Run(f, index_path=out / (f.stem + ".mzidx"))
    mzidx.build(run, out / (f.stem + ".mzidx"))
    run.close()
    return name


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    real = [x for x in os.environ.get("MZLAB_HR_FILES", "").split(",") if x]
    rng = np.random.default_rng
    hr_file = lambda i, kind: (lambda f: (f.unlink(missing_ok=True), f.symlink_to(Path(real[i]).resolve())) if real else ds.hr_dda(f, rng(31 + i), kind))
    sessions = {
        "lr": [build(out, "a.mzML", lambda f: ds.full_scan(f, 0, rng(1))), build(out, "b.mzML", lambda f: ds.full_scan(f, 30, rng(2)))],
        "pr": [build(out, "p0.mzML", lambda f: ds.full_scan_profile(f, 0, rng(30))), build(out, "p1.mzML", lambda f: ds.full_scan_profile(f, 30, rng(31)))],
        "hr": [build(out, "h0.mzML", hr_file(0, "exploris")), build(out, "h1.mzML", hr_file(1, "fusion"))],
    }
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for ns in sessions.values() for n in ns})
    res = []
    for sid, names in sessions.items():
        app = App(out)
        app.session = Session([{"file": n} for n in names], out)
        per = 2 if sid == "hr" else 1               # a DDA file is two items (survey, product ions)
        item = app._item(0)
        ms2 = [s for s in item.run.scans if s.level > 1 and s.precursor]
        prec = f"{ms2[len(ms2) // 2].precursor:.4f}" if ms2 else "300"
        pairs = [(0, 0), (per, 0), (0, per), (per, per)]                   # (k, bgk): a file against itself, against the other, and back
        for k, bgk in pairs:
            for query in (HR if sid == "hr" else LR):
                q = {"k": str(k), **dict(p.split("=", 1) for p in query.replace("PREC", prec).split("&"))}
                if "bgrt0" in q or "bgrt1" in q:
                    q["bgk"] = str(bgk)
                code, _, body = dispatch(app, "GET", "/api/spectrum", q)[:3]
                res.append({"session": sid, "files": names, "k": k, "bgk": bgk if "bgk" in q else None, "fk": k // per, "fbk": bgk // per, "path": "/api/spectrum",
                            "query": "&".join(f"{a}={b}" for a, b in q.items() if a != "k"), "status": code, "result": json.loads(body)})
    (out / "fondo.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
