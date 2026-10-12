"""Reference answers of the map routes for the parity test of the Rust port (crates/mzlab-core/tests/map_parity.rs).

    python3 tools/mappa_riferimento.py OUT

Writes three synthetic series of three files each (treatment times 0, 10, 30) with their `.mzidx` in OUT/ and, for each, a list of
{path, query, result} with the answer of `api.dispatch` (the JSON of `api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d`):
mappa.json (unit-resolution centroids, files s0..s2), mappa-prof.json (unit-resolution profile, p0..p2), mappa-hr.json (high-resolution
centroids, h0..h2). MZLAB_MAP_FILES=a.mzML,b.mzML,c.mzML replaces the synthetic unit-resolution files by real ones (MZLAB_MAP_HR_FILES
the high-resolution series, MZLAB_MAP_PROF_FILES the profile one). Prints nothing. No compound names.
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


HR_REGION = "rt0=5&rt1=7.5&mz0=250&mz1=330"
HR_FINE = "rt0=5.6&rt1=6.4&mz0=301.0&mz1=301.3&nrt=40&nmz=400"
if os.environ.get("MZLAB_MAP_HR_REGION"):               # real files: "rt0,rt1,mz0,mz1" of a region with data; the fine zoom is its centre
    _r0, _r1, _m0, _m1 = (float(x) for x in os.environ["MZLAB_MAP_HR_REGION"].split(","))
    HR_REGION = f"rt0={_r0}&rt1={_r1}&mz0={_m0}&mz1={_m1}"
    HR_FINE = f"rt0={(_r0 + _r1) / 2 - 0.2}&rt1={(_r0 + _r1) / 2 + 0.2}&mz0={(_m0 + _m1) / 2 - 0.1}&mz1={(_m0 + _m1) / 2 + 0.1}&nrt=40&nmz=400"
HR_QUERIES = [
    ("/api/map", "k=0"), ("/api/map", "k=1&agg=mean"), ("/api/map", "k=2&bg=1"), ("/api/map", "k=0&bgrt0=1&bgrt1=3"), ("/api/map", "k=0&ref=1"),
    ("/api/map", "k=2&ref=0&norm=max&rttol=0.2"), ("/api/map", f"k=0&{HR_REGION}&nrt=120&nmz=1000"), ("/api/map", f"k=1&{HR_REGION}&nrt=60&nmz=200&agg=mean&bg=1"),
    ("/api/map", f"k=0&{HR_REGION}&nrt=100&nmz=500&mzscale=log"), ("/api/map", f"k=1&{HR_REGION}&nrt=100&nmz=500&mzscale=log&bg=1&agg=mean"),
    ("/api/map", f"k=0&{HR_REGION}&nrt=100&nmz=400&mzscale=log&bgrt0=1&bgrt1=3"), ("/api/map", f"k=0&ref=1&{HR_REGION}&nrt=80&nmz=600&norm=max"),
    ("/api/map", f"k=0&ref=2&{HR_REGION}&nrt=80&nmz=600&mzscale=log&norm=tic&mztol=5"), ("/api/map", f"k=1&ref=0&{HR_REGION}&nrt=80&nmz=600&mzscale=log&mztol=2&rttol=0.05"),
    ("/api/map", f"k=0&{HR_FINE}"), ("/api/map", f"k=2&{HR_FINE}&mzscale=log&agg=mean"), ("/api/map", f"k=0&{HR_FINE}&bg=1&mzscale=log"),
    ("/api/map", f"k=0&ref=2&{HR_FINE}&mzscale=log&mztol=5"), ("/api/map", f"k=1&ref=0&{HR_FINE}&mzscale=log&mztol=100&norm=max"),
    ("/api/map", "k=0&level=2"), ("/api/map", f"k=0&{HR_REGION}&mzscale=ln"), ("/api/map", f"k=0&{HR_REGION}&mzscale=log&ref=1&mztol=x"),
    ("/api/mapsnap", "k=0&rt=6.0&mz=301.12"), ("/api/mapsnap", "k=1&rt=6.4&mz=285.1&dmz=0.5"), ("/api/mapsnap", "k=2&rt=3.0&mz=150.06&dmz=0.2"), ("/api/mapsnap", "k=0&rt=11.5&mz=900"),
    ("/api/mappunti", "k=0&pts=6.0,301.1234;6.4,285.0987;3.0,150.0555;11,800.1&ppm=5&grp=0.05"),
    ("/api/mappunti", "k=2&ref=0&pts=6.0,301.1234;6.4,285.0987;3.0,150.0555&ppm=10&grp=0.1"), ("/api/mappunti", "k=1&ref=2&pts=6.4,285.0987;2.1,279.2&ppm=0"),
    ("/api/mappunti3d", f"k=0&{HR_REGION}"), ("/api/mappunti3d", f"k=1&{HR_REGION}&max=300"), ("/api/mappunti3d", "k=2&rt0=0&rt1=1&mz0=100&mz1=130&max=50"),
]
HR_FEATURES = lambda t: [(301.1234, 6.0, 0.08, 2.0e6 * float(np.exp(-0.035 * t))), (285.0987, 6.4, 0.08, 1.5e6 * (1 - float(np.exp(-0.06 * t)))),
                         (150.0555, 3.0, 0.1, 8e5), (302.1268, 6.0, 0.08, 3e5 * float(np.exp(-0.035 * t))), (450.2301, 9.0, 0.1, 5e5 * float(np.exp(-0.02 * t)))]


def one_set(out: Path, stem: str, make, queries, real_env: str, res: list) -> None:
    """The three files of a series (`make(path, t, rng)` writes one; real_env names the variable that replaces them) and the answers to `queries`."""
    names = []
    real = [x for x in os.environ.get(real_env, "").split(",") if x]       # three real mzML files instead of the synthetic ones (measures, never committed)
    for i, t in enumerate(TIMES):
        f = out / f"{stem}{i}.mzML"
        if real:
            f.unlink(missing_ok=True)
            f.symlink_to(Path(real[i]).resolve())
        else:
            make(f, float(t), np.random.default_rng(10 + i))
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        names.append(f.name)
    mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})       # each file with its own index (two synthetic files can share size and probe)
    app = App(out)
    app.session = Session([{"file": n} for n in names], out)
    (out / f"{stem}-ordine.json").write_text(json.dumps([{"stem": it.path.stem, "full": it.kind() != "mrm" and it.type != "blank"} for it in app.session.items]), encoding="utf-8")    # a mixed (DDA) file is several items: k is a position in this list
    for path, query in queries:
        q = dict(p.split("=", 1) for p in query.split("&"))
        t = time.perf_counter()
        code, _, body = dispatch(app, "GET", path, q)[:3]
        res.append({"path": path, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": json.loads(body)})


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    sets = [("mappa.json", "s", lambda f, t, rng: ds.full_scan(f, t, rng), QUERIES, "MZLAB_MAP_FILES"),
            ("mappa-prof.json", "p", lambda f, t, rng: ds.full_scan_profile(f, t, rng), QUERIES, "MZLAB_MAP_PROF_FILES"),
            ("mappa-hr.json", "h", lambda f, t, rng: ds.hr_series(f, rng, HR_FEATURES(t)), HR_QUERIES, "MZLAB_MAP_HR_FILES")]
    for name, stem, make, queries, env in sets:
        res: list = []
        one_set(out, stem, make, queries, env, res)
        (out / name).write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
