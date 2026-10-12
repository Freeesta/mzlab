"""Reference answers of the map routes for the parity test of the Rust port (crates/mzlab-core/tests/map_parity.rs).

    python3 tools/mappa_riferimento.py OUT

Writes a synthetic series of Full Scan files (treatment times 0, 10, 30) with their `.mzidx` in OUT/ and OUT/mappa.json: a list of
{path, query, result} with the answer of `api.dispatch` (the JSON of `api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d`).
MZLAB_MAP_FILES=a.mzML,b.mzML,c.mzML replaces the synthetic files by real ones. Prints nothing. No compound names.
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

TIMES = (0, 10, 30)
REGION = "rt0=13&rt1=16&mz0=140&mz1=400"
FINE = "rt0=14&rt1=14.8&mz0=190&mz1=200&nrt=40&nmz=40"
STICKS = "rt0=13&rt1=16&mz0=150&mz1=370"
QUERIES = [
    ("/api/map", "k=0"), ("/api/map", "k=1&agg=mean"), ("/api/map", "k=0&bg=1"), ("/api/map", "k=2&bg=1&agg=mean"),
    ("/api/map", "k=0&bgrt0=1&bgrt1=3"), ("/api/map", "k=0&ref=1"), ("/api/map", "k=2&ref=0&norm=max&rttol=0.2"),
    ("/api/map", "k=1&ref=2&norm=tic&rttol=0.05&bg=1"), ("/api/map", f"k=0&{REGION}&nrt=120&nmz=100"),
    ("/api/map", f"k=1&{REGION}&nrt=60&nmz=20&agg=mean&bg=1"), ("/api/map", f"k=0&{FINE}"), ("/api/map", f"k=2&{FINE}&agg=mean"),
    ("/api/map", f"k=0&{FINE}&bgrt0=1&bgrt1=3"), ("/api/map", f"k=0&ref=1&{REGION}&nrt=80&nmz=60&norm=max"),
    ("/api/map", "k=0&level=2"), ("/api/map", "k=0&agg=sum"), ("/api/map", "k=0&rt0=3&rt1=2&mz0=100&mz1=200"),
    ("/api/mapsnap", "k=0&rt=14.25&mz=364.4"), ("/api/mapsnap", "k=1&rt=14.5&mz=194.2&dmz=1"), ("/api/mapsnap", "k=2&rt=7.1&mz=229.3"),
    ("/api/mapsnap", "k=0&rt=2.0&mz=100"), ("/api/mapsnap", "k=0&rt=18.3&mz=391.6&dmz=0.5"),
    ("/api/mappunti", "k=0&pts=14.3,364.3;14.32,194.2;7.07,229.1;18.3,391.3&ppm=0&grp=0.05"),
    ("/api/mappunti", "k=2&ref=0&pts=14.3,364.3;14.32,194.2;14.29,152.2;7.07,229.1&grp=0.1"),
    ("/api/mappunti", "k=1&ref=2&pts=14.7,305.3;2.1,279.2&ppm=0"), ("/api/mappunti", "k=0&pts=14.3,364.3&ppm=5"),
    ("/api/mappunti3d", f"k=0&{STICKS}"), ("/api/mappunti3d", f"k=1&{STICKS}&max=300"), ("/api/mappunti3d", "k=2&rt0=0&rt1=1&mz0=100&mz1=130&max=50"),
]


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    names = []
    real = [x for x in os.environ.get("MZLAB_MAP_FILES", "").split(",") if x]       # three real mzML files instead of the synthetic ones (measures, never committed)
    for i, t in enumerate(TIMES):
        f = out / f"s{i}.mzML"
        if real:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            ds.full_scan(f, float(t), np.random.default_rng(10 + i))
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    res = []
    for path, query in QUERIES:
        q = dict(p.split("=", 1) for p in query.split("&"))
        t = time.perf_counter()
        code, _, body = dispatch(app, "GET", path, q)[:3]
        res.append({"path": path, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body)})
    (out / "mappa.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
