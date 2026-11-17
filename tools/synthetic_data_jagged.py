"""Synthetic jagged low-resolution SIM/MRM traces in the style of a 3200 QTRAP (cycle ~1.5 s, 10-20 points per peak, counting noise, single-point
spikes, shoulders, pairs with a valley, tailing peaks), with their truth: the apexes, the true areas above the flat background and the valley of the
noise-free trace between two touching peaks. Every number is invented. Used by tests/test_lr_auto.py (CLI against an independent trapezoid) and by
tools/dati_sintetici.py (`jagged_file`, the mzML of the e2e test); `python3 tools/synthetic_data_jagged.py` prints one case per line.
"""
from __future__ import annotations

import math
from pathlib import Path

import numpy as np

CYCLE = 1.5 / 60                    # min: one point every 1.5 s
BACK = 200.0                        # counts: flat chemical background of the SIM trace
JITTER = 0.04                       # point-to-point spray jitter (relative): the jagged tops


def _shape(t: np.ndarray, c: float, s: float, tau: float) -> np.ndarray:
    """A Gaussian of unit height (tau = 0) or an exponentially modified Gaussian (tail tau min on the right) scaled to unit height."""
    g = np.array([math.exp(-0.5 * ((v - c) / s) ** 2) for v in t])
    if tau <= 0:
        return g
    k = np.array([math.exp(-i * CYCLE / tau) for i in range(t.size)])
    e = np.convolve(g, k)[: t.size]
    return e / e.max()


# name -> peaks (centre min, sigma min, height counts, tail min), expected peaks; sigma 3-5 s gives 10-20 points across 4 sigma
CASES: dict[str, list[tuple[float, float, float, float]]] = {
    "singles": [(1.2, 0.06, 2.0e4, 0.0), (2.6, 0.07, 6.0e3, 0.0), (4.0, 0.05, 4.0e4, 0.0), (5.6, 0.08, 1.5e3, 0.0)],
    "pair_rs1": [(2.0, 0.06, 2.0e4, 0.0), (2.0 + 4.0 * 0.06, 0.06, 1.2e4, 0.0)],
    "pair_rs08": [(3.0, 0.07, 3.0e4, 0.0), (3.0 + 3.4 * 0.07, 0.07, 1.5e4, 0.0)],
    "shoulder": [(2.5, 0.06, 3.0e4, 0.0), (2.5 + 1.6 * 0.06, 0.06, 0.8e4, 0.0)],
    "tailing": [(2.0, 0.05, 2.5e4, 0.12), (4.5, 0.06, 1.0e4, 0.08)],
    "triple": [(2.0, 0.06, 1.5e4, 0.0), (2.25, 0.06, 2.5e4, 0.0), (2.5, 0.06, 1.0e4, 0.0)],
    "low_sn": [(2.5, 0.07, 3.5e2, 0.0)],
    "noise": [],
}
# peaks a careful analyst integrates separately: a shoulder without a minimum is one peak
EXPECTED = {"singles": 4, "pair_rs1": 2, "pair_rs08": 2, "shoulder": 1, "tailing": 2, "triple": 3, "low_sn": 1, "noise": 0}


def trace(name: str, seed: int, t_end: float = 7.0):
    """(t, y, truth): t in min; y = (background + peaks) x (1 + 4 % jitter) + counting noise (sd = sqrt of the counts x 1.5) + spikes on ~1.5 % of the points
    (3-8x the local noise, never a peak); truth = {"apex": [...], "area": [... counts x min above BACK], "valley": [min of the clean trace between two touching peaks]}."""
    rng = np.random.default_rng(seed)
    t = np.arange(0.0, t_end, CYCLE)
    parts = [h * _shape(t, c, s, tau) for c, s, h, tau in CASES[name]]
    clean = BACK + (np.sum(parts, axis=0) if parts else np.zeros(t.size))
    y = clean * (1 + JITTER * rng.normal(0, 1, t.size)) + rng.normal(0, 1, t.size) * 1.5 * np.sqrt(clean)
    spikes = rng.random(t.size) < 0.015
    y[spikes] += rng.uniform(3, 8, int(spikes.sum())) * 1.5 * np.sqrt(clean[spikes])
    y = np.maximum(y, 0.0)
    area = [float(np.sum((p[1:] + p[:-1]) / 2 * np.diff(t))) for p in parts]
    apex = [float(t[int(np.argmax(p))]) for p in parts]
    valley = []
    for a, b in zip(apex, apex[1:]):
        i, j = int(round(a / CYCLE)), int(round(b / CYCLE))
        k = i + int(np.argmin(clean[i:j + 1]))
        if 0 < k - i < j - i and clean[k] - BACK > 0.02 * min(clean[i], clean[j]):        # touching: the trace does not come back to the background
            valley.append(float(t[k]))
    if name == "shoulder":
        apex, area, valley = [apex[0]], [sum(area)], []
    return t, y, {"apex": apex, "area": area, "valley": valley, "expected": EXPECTED[name]}


def jagged_file(path: Path, seed: int = 3) -> dict:
    """A 3200 QTRAP-like MRM mzML (TIC + one transition per case of `cases`, positive ESI) for the e2e test; returns {transition name: truth}."""
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from dati_sintetici import _chrom, _write
    cases = ("triple", "pair_rs08", "singles")
    pol = '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
    out, chroms, tic = {}, [], None
    for n, name in enumerate(cases):
        t, y, tr = trace(name, seed + n)
        tic = y if tic is None else tic + y
        cid = f"SRM SIC Q1={300.1 + 10 * n:.1f} Q3={150.1 + 5 * n:.1f} sample=1 period=1 experiment=1 transition={n} ce=20 name={name}"
        chroms.append(_chrom(n + 1, cid, t, y, pol)); out[name] = tr
    _write(path, [], [_chrom(0, "TIC", t, tic)] + chroms)
    return out


def write_csv(path: Path, t, y) -> Path:
    path.write_text("".join(f"{a:.6f},{b:.3f}\n" for a, b in zip(t, y)), encoding="utf-8")
    return path


if __name__ == "__main__":
    for n in CASES:
        t, y, tr = trace(n, 1)
        print(n, len(t), "points", tr)
