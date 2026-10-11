"""Reference reports of `ionfamily.origin_report` for the parity test of the Rust port (crates/mzlab-core/tests/origin_parity.rs).

    python3 tools/origin_riferimento.py OUT

Writes a synthetic series of Full Scan files (treatment times 0, 10, 30, 60) with their `.mzidx` in OUT/ and OUT/origin.json: a list of
{params, result} (the report without its timings). Prints nothing. No compound names.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab import ionfamily  # noqa: E402
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
