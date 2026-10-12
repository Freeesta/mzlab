"""Reference answers of `api/msntree` through `dispatch` for crates/mzlab-core/tests/msntree_route_parity.rs.

    python3 tools/msntree_rotta_riferimento.py OUTDIR

Writes OUTDIR/msn.mzML + msn.mzidx (a synthetic direct infusion with an MS2 and two MS3 paths, or the file named by MZLAB_MSN_FILE, a real one,
symlinked) and OUTDIR/msntree.json: a list of {query, status, result}. No compound names; the real file is never copied.
"""
import json
import os
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici_msn as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

QUERIES = ["k=0", "k=0&formula=C16H17N2O4", "k=0&ppm=2", "k=0&ppm=20&formula=C16H17N2O4", "k=0&formula=Xx9", "k=0&formula=C9H%2B", "k=0&ppm=x"]


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    f = out / "msn.mzML"
    real = os.environ.get("MZLAB_MSN_FILE")
    if real:
        f.unlink(missing_ok=True)
        f.symlink_to(Path(real).resolve())
    else:
        ds.msn_infusion(f, np.random.default_rng(5))
    run = Run(f, index_path=out / "msn.mzidx")
    mzidx.build(run, out / "msn.mzidx")
    run.close()
    mzidx.REGISTRY["msn.mzML"] = str(out / "msn.mzidx")
    app = App(out)
    app.session = Session([{"file": "msn.mzML"}], out)
    res = []
    for query in QUERIES:
        q = dict(p.split("=", 1) for p in query.split("&"))
        code, _, body = dispatch(app, "GET", "/api/msntree", q)[:3]
        res.append({"query": query, "status": code, "result": json.loads(body)})
    (out / "msntree.json").write_text(json.dumps(res), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
