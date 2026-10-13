"""Check of the retention times of a truth against the data it describes (D1: the MAF table of MTBLS733 against its mzML).

For every true compound the XIC at m/z +- PPM is cut in the window truth RT +- FINESTRA in each file; the apex is the highest scan of the XIC summed over the files
(the 10 runs of D1 share one gradient up to small shifts) and the compound counts as SEEN when the apex is at least SOGLIA times the median of the non-zero scans
of the XIC in the same window. The report gives, over the seen compounds, the distribution of apex - truth RT (so a RT tolerance can be chosen and written down)
and how many compounds are not seen; it reads the data, never changes the truth.

    python3 tools/bench/controlla_rt.py MAF.tsv MZML_DIR [--atteso "Compound concentration ratio"] [--out report.json]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

try:
    from bench import verita as V
except ImportError:                                    # run as a script
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from bench import verita as V

PPM = 5.0
FINESTRA = 1.0            # minutes around the truth RT where the apex is looked for
SOGLIA = 5.0              # apex / median of the non-zero scans of the window
PERCENTILI = (5, 25, 50, 75, 95)


def apice(rt: np.ndarray, y: np.ndarray, rt0: float, finestra: float = FINESTRA, soglia: float = SOGLIA):
    """(apex RT, apex height, seen) of a summed XIC in rt0 +- finestra, or (None, 0, False) when the window holds no signal."""
    k = np.flatnonzero((rt >= rt0 - finestra) & (rt <= rt0 + finestra))
    if not len(k) or not np.any(y[k] > 0):
        return None, 0.0, False
    j = k[int(np.argmax(y[k]))]
    base = float(np.median(y[k][y[k] > 0]))
    return float(rt[j]), float(y[j]), bool(y[j] >= soglia * base)


def controlla(verita: list[dict], tabelle: list, ppm: float = PPM, finestra: float = FINESTRA, soglia: float = SOGLIA) -> dict:
    """`tabelle`: PeakTable of every file (MS1). Returns the per-compound list and the summary."""
    righe = []
    for c in verita:
        tol = c["mz"] * ppm * 1e-6
        tot, rts = None, None
        for t in tabelle:
            y = t.xic(c["mz"], tol)
            if tot is None:
                tot, rts = y.astype(float).copy(), t.rt
            elif len(y) == len(tot):
                tot += y
            else:                                       # runs of different length: resample on the first run's time axis
                tot += np.interp(rts, t.rt, y)
        rt, h, visto = apice(rts, tot, c["rt"], finestra, soglia)
        righe.append({"id": c["id"], "mz": c["mz"], "rt": c["rt"], "apice": rt, "altezza": h, "visto": visto, "d": None if rt is None else rt - c["rt"]})
    d = np.array([r["d"] for r in righe if r["visto"]], dtype=float)
    ass = np.abs(d)
    return {"n": len(righe), "visti": int(len(d)), "non_visti": len(righe) - int(len(d)), "ppm": ppm, "finestra_min": finestra, "soglia": soglia,
            "d_percentili": {str(p): float(np.percentile(d, p)) for p in PERCENTILI} if len(d) else {},
            "assoluto_percentili": {str(p): float(np.percentile(ass, p)) for p in (50, 90, 95, 99)} if len(d) else {},
            "entro": {str(t): int(np.sum(ass <= t)) for t in (0.05, 0.1, 0.15, 0.2, 0.3, 0.5)}, "compounds": righe}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("maf")
    ap.add_argument("mzml_dir")
    ap.add_argument("--atteso", default="Compound concentration ratio")
    ap.add_argument("--out")
    a = ap.parse_args(argv)
    from mzlab.reader.mzml import Run                    # noqa: E402  (after bench put `mzlab` on the path)
    verita = V.da_maf(a.maf, atteso=a.atteso)
    tabelle = []
    for p in sorted(Path(a.mzml_dir).glob("*.mzML")):
        print(p.name, flush=True)
        tabelle.append(Run(p).table(1, None))
    rep = controlla(verita, tabelle)
    s = {k: v for k, v in rep.items() if k != "compounds"}
    print(json.dumps(s, indent=1))
    if a.out:
        Path(a.out).write_text(json.dumps(rep, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
