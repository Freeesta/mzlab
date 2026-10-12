"""Reference reports of `ionfamily.origin_report` for the parity test of the Rust port (crates/mzlab-core/tests/origin_parity.rs).

    python3 tools/origin_riferimento.py OUT

Writes a synthetic series of Full Scan files (treatment times 0, 10, 30, 60) with their `.mzidx` in OUT/ and OUT/origin.json: a list of
{params, result} (the report without its timings), and OUT/origin-api.json: the answers of `api.dispatch` for `api/origin` over three sessions
(unit-resolution centroids u0..u3, unit-resolution profile p0..p3, high-resolution centroids h0..h3, each with a product-ion file: m0 or d0),
a list of {set, query, status, result} (`items`: the items of the session, one per file or part of a mixed file). Prints nothing. No compound names.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab import ionfamily  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

TIMES = (0, 10, 30, 60)
P = 364.37
MS2_P = ([123.1, 171.2, 211.1, 229.1, 364.4], [0.05, 0.8, 0.25, 1.0, 0.3])
MS2_X = ([152.2, 171.2, 194.2, 229.1], [0.3, 1.0, 0.2, 0.6])
CASES = [  # mz, rt0, rt1, ref, formula, ms2
    (194.2, None, None, None, None, None), (305.3, None, None, None, None, None), (229.1, None, None, None, None, None),
    (365.37, None, None, None, None, None), (152.2, 14.0, 14.7, 1, "C18H21N5O4S", None), (194.2, None, None, 2, "C18H21N5O4S", "xp"),
    (124.3, None, None, None, None, "p"), (391.3, None, None, None, None, None),
]


def _hr_parent() -> float:
    from mzlab.chem.elements import ion_mz, mass, parse_formula
    return ion_mz(mass(parse_formula(ds.HR_PARENT)), "[M+H]+")


def _api_sets(out: Path):
    """(name, stem, writers, times, product-ion file writer, queries) of the three sessions of `origin-api.json`."""
    pm = _hr_parent()
    hr = lambda t: [(pm, 14.3, 0.07, 3.0e6 * float(np.exp(-0.035 * t))), (305.0702, 14.7, 0.07, 1.2e6 * (1 - float(np.exp(-0.05 * t)))),
                    (229.05, 14.3, 0.07, 6e5 * (1 - float(np.exp(-0.03 * t)))), (pm + 1.00335, 14.3, 0.07, 0.17 * 3.0e6 * float(np.exp(-0.035 * t))),
                    (279.1591, 14.3, 0.2, 2e5), (412.1, 16.0, 0.07, 8e5)]
    uq = [("mz=194.2&parent=364.37", 1), ("mz=229.1&parent=305.0&k=1&formula=C18H21N5O4S", 1), ("mz=305.3&parent=364.37&k=2&rt0=14&rt1=15", 1),
          ("mz=124.3&parent=305.0", 1), ("mz=391.3&parent=364.37&k=0", 1), ("mz=229.1&parent=305.0&rt0=14&rt1=15.5", 1)]
    hq = [(f"mz=305.0702&parent={pm}&k=1&formula={ds.HR_PARENT}", 1), (f"mz=229.05&parent={pm}", 1), (f"mz={pm + 1.00335}&parent={pm}&k=2&rt0=13.5&rt1=15.5", 1),
          (f"mz=305.0702&parent=229.05&k=0", 1), (f"mz=412.1&parent={pm}", 1)]
    return [("unit", "u", lambda f, t, rng: ds.full_scan(f, t, rng), lambda f, rng: ds.ms2(f, rng, 15), "m0", uq),
            ("prof", "p", lambda f, t, rng: ds.full_scan_profile(f, t, rng), lambda f, rng: ds.ms2(f, rng, 15), "m0", uq),
            ("hr", "h", lambda f, t, rng: ds.hr_series(f, rng, hr(t), rt_end=18.0), lambda f, rng: ds.hr_dda(f, rng), "d0", hq)]


def api_cases(out: Path) -> list:
    res = []
    for name, stem, make, make_ms2, ms2_stem, queries in _api_sets(out):
        names = []
        for i, t in enumerate(TIMES):
            f = out / f"{stem}{i}.mzML"
            make(f, float(t), np.random.default_rng(10 + i))
            names.append(f.name)
        m = out / f"{ms2_stem}.mzML"
        make_ms2(m, np.random.default_rng(7))
        names.append(m.name)
        for n in names:
            run = Run(out / n, index_path=out / (Path(n).stem + ".mzidx"))
            mzidx.build(run, out / (Path(n).stem + ".mzidx"))
            run.close()
        mzidx.REGISTRY.update({n: str(out / (Path(n).stem + ".mzidx")) for n in names})
        app = App(out)
        app.session = Session([{"file": n, "time": float(TIMES[i]) if i < len(TIMES) else None} for i, n in enumerate(names)], out)
        items = [{"file": it.path.name, "label": it.label, "time": it.time, "full": it.kind() != "mrm" and it.type != "blank"} for it in app.session.items]
        for query, _ in queries:
            q = dict(p.split("=", 1) for p in query.split("&"))
            t = time.perf_counter()
            code, _, body = dispatch(app, "GET", "/api/origin", q)[:3]
            r = json.loads(body)
            r.pop("timing", None)
            res.append({"set": name, "items": items, "query": query, "status": code, "py_s": time.perf_counter() - t, "result": r})
    return res


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    runs = []
    for i, t in enumerate(TIMES):
        f = out / f"s{i}.mzML"
        ds.full_scan(f, float(t), np.random.default_rng(10 + i))
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        runs.append((f, run, t))
    samples = [{"label": f.stem, "time": float(t), "table": run.table(1, None), "key": str(f)} for f, run, t in runs]
    res = []
    for mz, rt0, rt1, ref, formula, ms2 in CASES:
        spec = None
        if ms2:
            spec = {"p": (np.array(MS2_P[0]), np.array(MS2_P[1])), "x": (np.array(MS2_X[0]), np.array(MS2_X[1])) if ms2 == "xp" else None}
        rep = ionfamily.origin_report(samples, mz, P, rt0, rt1, ref, formula_p=formula, top=25, ms2=spec)
        rep.pop("timing", None)
        res.append({"ms2_p": {"mz": MS2_P[0], "int": MS2_P[1]}, "ms2_x": {"mz": MS2_X[0], "int": MS2_X[1]}, "params": {"mz": mz, "parent": P, "rt0": rt0, "rt1": rt1, "ref": ref, "formula": formula, "ms2": ms2,
                               "labels": [s["label"] for s in samples], "times": [s["time"] for s in samples]}, "result": rep})
    for _, run, _ in runs:
        run.close()
    (out / "origin.json").write_text(json.dumps(res), encoding="utf-8")
    (out / "origin-api.json").write_text(json.dumps(api_cases(out)), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
