"""Reference of the drift correction of `crates/mzlab-feat/src/drift.rs` (the code of the site is Rust; this one only writes the frozen answers).

    python3 tools/deriva_riferimento.py            # writes tests/golden/feat/deriva.json

A small series with two batches (QC files at the ends of each), areas with a known drift (a line and a bend, plus noise from a fixed seed) and cells without a
peak; the answers are the factors, the curve, the reference level and the CV of the QC before and after, for a line and for a LOESS (span 0.75).
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "tests" / "golden" / "feat" / "deriva.json"


def wline(x, y, w, at):
    sw = sum(w)
    mx = sum(wi * xi for wi, xi in zip(w, x)) / sw
    my = sum(wi * yi for wi, yi in zip(w, y)) / sw
    sxx = sum(wi * (xi - mx) ** 2 for wi, xi in zip(w, x))
    sxy = sum(wi * (xi - mx) * (yi - my) for wi, xi, yi in zip(w, x, y))
    if sxx <= 1e-12 * sw:
        return my
    return my + sxy / sxx * (at - mx)


def loess(x, y, span, at):
    n = len(x)
    k = min(max(math.ceil(span * n), min(3, n)), n)
    h = max(sorted(abs(v - at) for v in x)[k - 1], 1e-12)
    w = []
    for v in x:
        u = abs(v - at) / (h * (1 + 1e-9))
        w.append((1 - u ** 3) ** 3 if u < 1 else 0.0)
    return wline(x, y, w, at)


def cv(v):
    if len(v) < 3:
        return None
    m = sum(v) / len(v)
    var = sum((a - m) ** 2 for a in v) / (len(v) - 1)
    return 100 * math.sqrt(var) / m if m > 0 else None


def median(v):
    v = sorted(v)
    n = len(v)
    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) / 2


def correct(files, areas, method, span=None):
    """files: [(kind, order, batch)], areas: rows x files (0 = no peak). Returns factor, curve (rows x files, None = NaN), reference, cv_before, cv_after."""
    n = len(files)
    batches = sorted({b for _, _, b in files})
    qc = {b: sorted((k for k in range(n) if files[k][2] == b and files[k][0] == "qc"), key=lambda k: files[k][1]) for b in batches}
    need = 3 if method == "linear" else 5
    out = dict(factor=[], curve=[], reference=[], cv_before=[], cv_after=[])
    for row in areas:
        meas = lambda k: row[k] if row[k] > 0 else None             # noqa: E731
        allq = [meas(k) for b in batches for k in qc[b] if meas(k) is not None]
        ref = median(allq) if allq else None
        fac, cur = [None] * n, [None] * n
        for b in batches:
            pts = [(files[k][1], meas(k)) for k in qc[b] if meas(k) is not None]
            if len(pts) < need or len({p[0] for p in pts}) < 2:
                continue
            x, y = [float(p[0]) for p in pts], [p[1] for p in pts]
            for k in range(n):
                if files[k][2] != b:
                    continue
                at = float(files[k][1])
                c = wline(x, y, [1.0] * len(x), at) if method == "linear" else loess(x, y, span, at)
                cur[k] = c
                if c > 0 and ref is not None:
                    fac[k] = ref / c
        after = [meas(k) * fac[k] if meas(k) is not None and fac[k] is not None else None for b in batches for k in qc[b] if meas(k) is not None]
        out["factor"].append(fac)
        out["curve"].append(cur)
        out["reference"].append(ref)
        out["cv_before"].append(cv(allq))
        out["cv_after"].append(cv(after) if all(a is not None for a in after) else None)
    return out


def series():
    rng = np.random.default_rng(5)
    # two batches of 9 files: QC, 7 samples/QC mixed, QC; injection order 1..18
    files = []
    for b, base in (("1", 0), ("2", 9)):
        for i in range(9):
            files.append(("qc" if i in (0, 4, 8) or (b == "1" and i in (2, 6)) or (b == "2" and i in (2, 6)) else "sample", base + i + 1, b))
    areas = []
    for r in range(6):
        level = 10 ** rng.uniform(5, 7)
        slope = rng.uniform(-0.03, 0.04)
        bend = rng.uniform(-0.002, 0.002)
        row = []
        for kind, order, b in files:
            v = level * (1 + slope * order + bend * (order - 9) ** 2) * rng.normal(1, 0.02)
            row.append(float(round(v, 3)))
        areas.append(row)
    areas[2][3] = 0.0              # a sample without a peak
    areas[3][0] = 0.0              # a QC without a peak (first of batch 1)
    areas[4] = [a if f[0] != "qc" or f[2] == "2" else 0.0 for a, f in zip(areas[4], files)]     # no QC peak at all in batch 1
    return files, areas


def main():
    files, areas = series()
    doc = {"_note": "Answers of tools/deriva_riferimento.py for crates/mzlab-feat/src/drift.rs; invented numbers.",
           "files": [{"kind": k, "order": o, "batch": b} for k, o, b in files], "areas": areas,
           "cases": [{"method": "linear", "span": None, **correct(files, areas, "linear")}, {"method": "loess", "span": 0.75, **correct(files, areas, "loess", 0.75)}]}
    TARGET.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print(TARGET)


if __name__ == "__main__":
    sys.exit(main())
