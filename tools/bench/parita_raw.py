"""Parity of two mzML conversions of the same Thermo `.raw` (D1 control: the native reader of the program, `raw2mzml`, against ThermoRawFileParser).

It is a check, not an advantage: every tool of the benchmark reads the ThermoRawFileParser mzML. Compared level by level (MS1, MS2): number of scans, RT of every
scan, number of peaks per scan, and, where the peak counts agree, the m/z (ppm) and intensity (relative) of every peak after sorting by (scan, m/z).
Peaks of scans whose counts differ are only counted. Exit code 0 always: the verdict is in the JSON report.

    python3 tools/bench/parita_raw.py A.mzML B.mzML [--out report.json]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))


def _per_scan(t):
    o = np.lexsort((t.mz, t.pos))
    return np.asarray(t.pos)[o], np.asarray(t.mz, dtype=float)[o], np.asarray(t.inten, dtype=float)[o]


def confronta_tabelle(a, b, ppm_max: float = 1.0) -> dict:
    """Compare two PeakTable of the same level."""
    out = {"scan": [int(len(a.rt)), int(len(b.rt))], "picchi": [int(len(a.mz)), int(len(b.mz))]}
    if len(a.rt) != len(b.rt):
        out["uguali"] = False
        return out
    out["rt_max_diff_min"] = float(np.max(np.abs(np.asarray(a.rt) - np.asarray(b.rt)))) if len(a.rt) else 0.0
    pa, ma, ia = _per_scan(a)
    pb, mb, ib = _per_scan(b)
    na, nb = np.bincount(pa, minlength=len(a.rt)), np.bincount(pb, minlength=len(b.rt))
    diversi = np.flatnonzero(na != nb)
    out["scan_con_numero_di_picchi_diverso"] = int(len(diversi))
    ok_a = ~np.isin(pa, diversi)
    ok_b = ~np.isin(pb, diversi)
    ma, ia, mb, ib = ma[ok_a], ia[ok_a], mb[ok_b], ib[ok_b]
    out["picchi_confrontati"] = int(len(ma))
    if len(ma):
        dppm = np.abs(ma - mb) / np.maximum(ma, 1e-9) * 1e6
        rel = np.abs(ia - ib) / np.maximum(np.maximum(ia, ib), 1e-12)
        out["mz_ppm_max"], out["mz_ppm_mediana"] = float(dppm.max()), float(np.median(dppm))
        out["intensita_rel_max"], out["intensita_rel_mediana"] = float(rel.max()), float(np.median(rel))
        out["picchi_oltre_ppm"] = int(np.sum(dppm > ppm_max))
    out["uguali"] = bool(len(diversi) == 0 and out.get("picchi_oltre_ppm", 0) == 0)
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("a")
    ap.add_argument("b")
    ap.add_argument("--out")
    x = ap.parse_args(argv)
    from mzlab.reader.mzml import Run
    ra, rb = Run(x.a), Run(x.b)
    rep = {"a": Path(x.a).name, "b": Path(x.b).name}
    for lv in (1, 2):
        na = sum(1 for s in ra.scans if s.level == lv)
        nb = sum(1 for s in rb.scans if s.level == lv)
        rep[f"ms{lv}"] = confronta_tabelle(ra.table(lv), rb.table(lv)) if na and nb else {"scan": [na, nb]}
    print(json.dumps(rep, indent=1))
    if x.out:
        Path(x.out).write_text(json.dumps(rep, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
