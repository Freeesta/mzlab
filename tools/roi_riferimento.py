"""Reference ROIs and traces of the Python engine for the parity test of the Rust port (crates/mzlab-core/tests/rois_parity.rs).

    python3 tools/roi_riferimento.py OUT [FILE.mzML ...]

Like tools/mzidx_riferimento.py (synthetic files + the given ones, their `.mzidx` in OUT/), and for each file OUT/<stem>.rois.json = a list of
{params, result} for `ionfamily.build_rois` (several windows and settings) and `extract_trace` on the survey table. Prints "<mzML path>" lines.
"""
import json
import multiprocessing
import os
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab import ionfamily  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def one(job) -> str:
    f, out = job
    run = Run(f, index_path=out / (f.stem + ".mzidx"))
    mzidx.build(run, out / (f.stem + ".mzidx"))
    cases = []
    t = run.table(1, None) if any(s.level == 1 for s in run.scans) else None
    if t is not None and len(t.rt) > 10 and len(t.mz):
        r0, r1 = float(t.rt[0]), float(t.rt[-1])
        span = r1 - r0
        hr = bool(run.hr1)
        tol = 0.02 if hr else 0.35
        for rt0, rt1 in ((r0, r1), (r0 + span * 0.25, r0 + span * 0.75), (r0 + span * 0.4, r0 + span * 0.45)):
            for kw in ({}, {"min_scans": 3, "max_gap": 1}, {"mz_range": (150.0, 400.0), "min_int": 1000.0}):
                p = {"rt0": rt0, "rt1": rt1, "tol": tol, "min_scans": 4, "max_gap": 2, "mz_range": None, "min_int": 0.0, **kw}
                w = ionfamily.build_rois(t, p["rt0"], p["rt1"], p["tol"], p["min_scans"], p["max_gap"], p["mz_range"], p["min_int"])
                cases.append({"kind": "rois", "params": p, "result": {"rt": [float(v) for v in w.rt], "mz": [float(v) for v in w.mz], "x": [float(v) for v in np.asarray(w.X).ravel()],
                                                                          "n_roi": int(len(w.mz)), "n_scans": int(len(w.rt)), "scan_lo": int(w.scan_lo),
                                                                          "mz_lo": w.mz_lo, "mz_hi": w.mz_hi, "n_peaks": int(w.n_peaks)}})
        mzs = np.asarray(t.mz[: len(t.mz)])
        for q in (0.1, 0.5, 0.9):
            mz = float(mzs[int(len(mzs) * q)])
            for win in (None, (r0 + span * 0.2, r0 + span * 0.6)):
                rt, y = ionfamily.extract_trace(t, mz, tol, *(win or (None, None)))
                cases.append({"kind": "trace", "params": {"mz": mz, "tol": tol, "win": win}, "result": {"rt": [float(v) for v in rt], "y": [float(v) for v in y]}})
    run.close()
    (out / (f.stem + ".rois.json")).write_text(json.dumps(cases), encoding="utf-8")
    return str(f)


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    # every file is independent (it writes only its own <stem>.mzidx and <stem>.rois.json): one process per CPU, the listing keeps the order of the files
    with multiprocessing.Pool(min(len(files), os.cpu_count() or 1)) as pool:
        for name in pool.imap(one, [(f, out) for f in files]):
            print(name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
