#!/usr/bin/env python3
"""Synthetic unit-resolution (QTRAP-like Q1 profile) LC-MS benchmark with ground truth and decoys. A TOOL for the tests (numpy only), not runtime code:
`crates/mzlab-cli/src/flow_lr_bench_tests.rs` runs it into a temp dir and scores the feature chain against `LR_bench-s<N>_truth.json`.

    python3 tools/lr_sintetico.py OUT_DIR [--seed N] [--jitter DA]

Writes `LR_bench-s<N>.mzML` (60 compounds + 12 decoy pairs + background), `LR_bench-blank-s<N>.mzML` (same background, no compounds, other noise draw)
`LR_bench-noise-s<N>.mzML` (no compounds and no background: only the Poisson counts and the single-scan spikes) and the truth. Same seed and jitter = same bytes (structure from the seed, noise from separate streams; scalar libm exp as in `dati_sintetici`).
Intensity = ion counts x GAIN (counts per second at a 50 ms dwell); heights in the truth are in counts.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_sintetici import _exp, _spec, _write  # noqa: E402  (helpers only: their own output stays untouched)

FWHM_MZ = 0.7                   # Da, Gaussian profile of a unit-resolution quadrupole
FWHM_RT = 6.0                   # s
TAIL_S = 2.4                    # s, scale of the exponential tail on the right of the chromatographic peak
OFFSET = 0.1                    # Da, calibration offset of the whole file
GAIN = 20.0                     # counts -> intensity (cps)
GRID = np.round(np.arange(100.0, 400.0 + 1e-9, 0.1), 1)
N_SCANS = 900                   # ~1 s/scan over 15 min
SIG_MZ = FWHM_MZ / 2.3548
SIG_RT = FWHM_RT / 2.3548
HALF = 30                       # grid points either side of an ion (3 Da, ~10 sigma)
OFFS = np.arange(-HALF, HALF + 1)
N_COMP, N_PAIRS, N_BG, N_SPIKES = 60, 12, 40, 300


def _structure(seed: int) -> dict:
    """Ions of the sample: compounds (mono + isotopes), decoy pairs and the constant background."""
    rng = np.random.default_rng(seed)
    ions, comps, pairs = [], [], []

    def add(cid, kind, mz, rt, h):
        ions.append({"compound": cid, "kind": kind, "mz_exact": round(float(mz), 4), "mz": round(float(mz) + OFFSET, 4),
                     "rt_s": round(float(rt), 3), "height": round(float(h), 1)})

    for c in range(N_COMP):
        mz = rng.uniform(120.0, 380.0)
        rt = rng.uniform(60.0, 840.0)
        h = float(np.exp(rng.uniform(np.log(100.0), np.log(20000.0))))
        cl = bool(rng.random() < 0.25)
        nc = int(mz / 13.0)
        add(c, "mono", mz, rt, h)
        add(c, "iso", mz + 1.0034, rt, h * 0.011 * nc)
        if cl:
            add(c, "iso", mz + 1.9970, rt, h * 0.32)
        comps.append({"id": c, "cl": cl, "mz": round(mz + OFFSET, 4), "rt_s": round(rt, 3), "height": round(h, 1)})
    for p in range(N_PAIRS):
        a = N_COMP + 2 * p
        mz = rng.uniform(130.0, 370.0)
        rt = rng.uniform(80.0, 820.0)
        dm = rng.uniform(0.4, 0.6)
        drt = rng.uniform(0.0, 8.0)
        h = float(np.exp(rng.uniform(np.log(600.0), np.log(6000.0))))
        add(a, "decoy", mz, rt, h)
        add(a + 1, "decoy", mz + dm, rt + drt, h * rng.uniform(0.5, 2.0))
        pairs.append({"a": a, "b": a + 1, "dm": round(float(dm), 4), "drt_s": round(float(drt), 3)})
    bg = [{"mz": round(float(m), 4), "height": round(float(h), 1)}
          for m, h in zip(rng.uniform(105.0, 395.0, N_BG), np.exp(rng.uniform(np.log(20.0), np.log(300.0), N_BG)))]
    return {"ions": ions, "compounds": comps, "pairs": pairs, "background": bg}


def _chrom(rt_s: float, apex: float):
    """Gaussian of FWHM 6 s with an exponential tail on the right, maximum 1 at the apex."""
    x = rt_s - apex
    return _exp(-0.5 * (x / SIG_RT) ** 2) * np.where(x > 0, _exp(-x / TAIL_S * 0.15), 1.0)


def _write_run(path: Path, st: dict, noise_seed: int, jitter: float, compounds: bool, background: bool = True) -> None:
    rng = np.random.default_rng(noise_seed)
    ions = st["ions"] if compounds else []
    bg = st["background"] if background else []
    mzs = np.array([i["mz"] for i in ions] + [b["mz"] for b in bg])
    rts = np.array([i["rt_s"] for i in ions] + [0.0] * len(bg))
    hts = np.array([i["height"] for i in ions] + [b["height"] for b in bg])
    const = np.array([False] * len(ions) + [True] * len(bg))
    # spikes: one scan, one position (electronic or chemical bursts), other draw in each file
    sp_scan = rng.integers(0, N_SCANS, N_SPIKES)
    sp_mz = rng.uniform(100.0, 400.0, N_SPIKES)
    sp_h = np.exp(rng.uniform(np.log(50.0), np.log(2000.0), N_SPIKES))
    specs = []
    for s in range(N_SCANS):
        t = s + 0.5
        shift = rng.normal(0.0, jitter) if jitter > 0 else 0.0
        amp = hts * np.where(const, 1.0, _chrom(t, rts))
        k = amp > 0.2
        m = np.concatenate([mzs[k], sp_mz[sp_scan == s]]) + shift
        a = np.concatenate([amp[k], sp_h[sp_scan == s]])
        lam = np.zeros(GRID.size)
        if m.size:
            c0 = np.rint((m - GRID[0]) / 0.1).astype(int)
            idx = c0[:, None] + OFFS[None, :]
            x = GRID[np.clip(idx, 0, GRID.size - 1)] - m[:, None]
            v = a[:, None] * _exp(-0.5 * (x / SIG_MZ) ** 2)
            v = np.where((idx >= 0) & (idx < GRID.size), v, 0.0)
            np.add.at(lam, np.clip(idx, 0, GRID.size - 1).ravel(), v.ravel())
        counts = rng.poisson(lam)
        keep = counts > 0
        specs.append(_spec(s, t / 60.0, GRID[keep], counts[keep] * GAIN, profile=True, filt="+ p ESI Q1MS [100.000-400.000]"))
    _write(path, specs, [])


def make(out: Path, seed: int = 1, jitter: float = 0.05) -> list[Path]:
    out.mkdir(parents=True, exist_ok=True)
    st = _structure(seed)
    a, b, tr = out / f"LR_bench-s{seed}.mzML", out / f"LR_bench-blank-s{seed}.mzML", out / f"LR_bench-s{seed}_truth.json"
    _write_run(a, st, seed + 1000, jitter, True)
    _write_run(b, st, seed + 2000, jitter, False)
    nz = out / f"LR_bench-noise-s{seed}.mzML"
    _write_run(nz, st, seed + 3000, jitter, False, background=False)
    tr.write_text(json.dumps({"seed": seed, "jitter": jitter, "offset": OFFSET, "fwhm_mz": FWHM_MZ, "fwhm_rt_s": FWHM_RT, "gain": GAIN, **st},
                             indent=1), encoding="utf-8")
    return [a, b, nz, tr]


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out", type=Path)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jitter", type=float, default=0.05, help="sigma of the per-scan mass jitter, Da")
    n = ap.parse_args()
    for f in make(n.out, n.seed, n.jitter):
        print(f)
