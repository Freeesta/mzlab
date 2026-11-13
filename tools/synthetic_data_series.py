"""The DIA file and the QC/blank series of the synthetic data (see `dati_sintetici.py`, which calls them from `make`): fixed isolation windows,
and eleven files in the order of injection with a linear RT drift and a contaminant only in the blanks. Every number is invented."""
from __future__ import annotations

from pathlib import Path

import numpy as np

from dati_sintetici import _chrom, _g, _log, _scan_ions, _spec, _write


def dia(path: Path, rng) -> None:
    """DIA look-alike (Q-TOF style): at every cycle one survey scan and eight product-ion scans over fixed, adjacent isolation windows of 45 Da
    (m/z 120-480); the windows repeat unchanged cycle after cycle, no precursor is chosen from the survey."""
    ions = [(364.4, 8.0, 0.06, 2e7), (250.2, 3.0, 0.06, 1e6), (310.2, 5.5, 0.06, 8e5)]
    rts = np.arange(0.5, 14.0, 0.12); specs, tic, i = [], [], 0
    for rt in rts:
        mz, it = _scan_ions(rt, rng, ions); specs.append(_spec(i, rt, mz, it)); tic.append(it.sum()); i += 1
        for k in range(8):
            lo, hi = 120.0 + 45 * k, 165.0 + 45 * k
            frag = np.sort(rng.uniform(60, hi, 6)); it2 = rng.lognormal(_log(2e3), 0.6, 6)
            specs.append(_spec(i, rt + 0.004 * (k + 1), frag, it2, level=2, prec=round((lo + hi) / 2, 1), ce=25.0, iso=(lo, hi))); i += 1
    _write(path, specs, [_chrom(0, "TIC", rts, tic)])


# injection order of the QC series: kind of each file (the QC files sit at 2, 6 and 10)
QC_KINDS = ["blank", "QC", "S", "S", "S", "QC", "S", "S", "S", "QC", "blank"]


def qc_series(out: Path, seed: int = 53) -> list[Path]:
    """Files Q_blank1, Q_QC1, Q_S1.. Q_S3, Q_QC2, Q_S4.. Q_S6, Q_QC3, Q_blank2 in the order of injection: three analytes (m/z 250.2 at 3.0 min, 310.2 at
    5.5, 364.4 at 8.0), a RT drift that grows linearly with the injection (0.05 min from the first QC to the last), a loss of 6.1 % of the area over the same
    QC span, and a contaminant at m/z 391.3 (RT 3.42) only in the two blanks. Every number is invented."""
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(seed)
    rts = np.arange(0.5, 12.0, 0.04)
    paths, counts = [], {}
    for k, kind in enumerate(QC_KINDS, start=1):
        counts[kind] = counts.get(kind, 0) + 1
        name = f"Q_{kind}{counts[kind]}" if kind != "S" else f"Q_S{counts[kind]}"
        drift = 0.00625 * (k - 2); gain = 1.0 - 0.0076 * (k - 2)
        amp = 0.0 if kind == "blank" else (1.0 if kind == "QC" else 0.8 + 0.1 * (counts["S"] % 5))
        specs, tic = [], []
        for i, rt in enumerate(rts):
            mz = list(rng.uniform(120, 480, 25)); it = list(rng.lognormal(_log(3e3), 0.8, 25))
            ions = [(250.2, 3.0 + drift, 1.0e6), (310.2, 5.5 + drift, 8e5), (364.4, 8.0 + drift, 1.2e6)]
            if kind == "blank":
                ions = [(391.3, 3.42, 5e5)]
            for m, c, a in ions:
                v = (a * amp * gain if kind != "blank" else a) * float(_g(rt, c, 0.05, 1e6)) * rng.normal(1, 0.01)
                if v > 200:
                    mz.append(m + rng.normal(0, 0.02)); it.append(v)
            o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]
            specs.append(_spec(i, float(rt), mz, it)); tic.append(it.sum())
        p = out / f"{name}.mzML"
        _write(p, specs, [_chrom(0, "TIC", rts, tic)])
        paths.append(p)
    return paths
