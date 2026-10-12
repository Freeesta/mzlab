"""Reference answers of `ref=` on /api/chrom, /api/xic and /api/mrm for crates/mzlab-core/tests/diff_parity.rs.

    python3 tools/differenza_riferimento.py OUT

Writes the synthetic mzML files in OUT/ (three Full Scan files at different times, an MS2 file, three MRM files), their `.mzidx`, and
OUT/diff.json = [{path, query, status, body}] answered by `mzlab.api.dispatch` over a session that holds the files in that order; prints
"<mzML path>" lines in the order of the session. No data of the laboratory, no compound names.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Item  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def files(out: Path) -> list[Path]:
    ds.full_scan(out / "a0.mzML", 0, np.random.default_rng(1))
    ds.full_scan(out / "a1.mzML", 30, np.random.default_rng(11))
    ds.full_scan(out / "a2.mzML", 60, np.random.default_rng(12))
    ds.ms2(out / "b.mzML", np.random.default_rng(2))
    ds.mrm(out / "c0.mzML", 1.0, np.random.default_rng(3), 1)
    ds.mrm(out / "c1.mzML", 0.5, np.random.default_rng(13), 2)
    ds.mrm(out / "c2.mzML", 2.0, np.random.default_rng(14), 3)
    return [out / f"{n}.mzML" for n in ("a0", "a1", "a2", "b", "c0", "c1", "c2")]


def queries() -> list[tuple[str, dict]]:
    out = []
    for k, ref in (("1", "0"), ("2", "0"), ("2", "1"), ("0", "1")):
        base = {"k": k, "ref": ref}
        for extra in ({}, {"kind": "bpc"}, {"norm": "tic"}, {"norm": "istd", "istd": "364.4"}, {"norm": "istd", "istd": "364.4", "tol": "0.5"},
                      {"factor": "0.5", "clip": "1"}, {"rttol": "0"}, {"rttol": "0.5", "factor": "2"}, {"mz0": "100", "mz1": "300", "norm": "tic"},
                      {"mz0": "150", "kind": "bpc", "clip": "1"}, {"level": "2"}, {"norm": "istd", "istd": "99999"}):
            out.append(("/api/chrom", {**base, **extra}))
    for ks, ref in (("1,2", "0"), ("0,2", "1"), ("1", "2")):
        for mz, tol in (("364.4", "0.35"), ("305.3", "0.5"), ("152.2", "0.2"), ("99999", "0.35")):
            base = {"k": ks, "ref": ref, "mz": mz, "tol": tol}
            for extra in ({}, {"norm": "tic"}, {"norm": "istd", "istd": "364.4"}, {"factor": "1.5", "clip": "1"}, {"rttol": "0.3"}):
                out.append(("/api/xic", {**base, **extra}))
    for ks, ref in (("5,6", "4"), ("4", "5"), ("4,5,6", "6"), ("5", "0"), ("4", "3")):
        base = {"k": ks, "ref": ref}
        for extra in ({}, {"norm": "tic"}, {"norm": "istd", "istd": "364.1"}, {"norm": "istd", "istd": "500"}, {"factor": "0.7", "clip": "1"}, {"rttol": "0.4"}):
            out.append(("/api/mrm", {**base, **extra}))
    for path in ("/api/chrom", "/api/xic", "/api/mrm"):      # a wrong setting is the user's error (a key) or Python's own (a text)
        base = {"k": "1", "ref": "0", "mz": "364.4"}
        for extra in ({"norm": "foo"}, {"norm": "istd"}, {"factor": "200"}, {"factor": "-1"}, {"rttol": "11"}, {"ref": "x"}, {"factor": "x"},
                      {"norm": "it's"}, {"norm": "a" * 30}, {"level": "x"}, {"tol": "x"}):
            out.append((path, {**base, **extra}))
    return out


def session(out: Path) -> tuple[list[Path], App]:
    """The files in OUT (with their `.mzidx`) and an app whose session holds them in that order."""
    out.mkdir(parents=True, exist_ok=True)
    paths = files(out)
    items = []
    for f in paths:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        items.append(Item(f.name, None, None, "x", f))
    app = App()
    app.session = type("S", (), {"items": items})()
    return paths, app


def main(argv) -> int:
    out = Path(argv[0])
    paths, app = session(out)
    rows = []
    for path, q in queries():
        status, _ctype, body, _h = dispatch(app, "GET", path, q)
        rows.append({"path": path, "query": q, "status": status, "body": json.loads(body)})
    (out / "diff.json").write_text(json.dumps(rows), encoding="utf-8")
    for f in paths:
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
