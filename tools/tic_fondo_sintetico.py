#!/usr/bin/env python3
"""Synthetic unit-resolution LC-MS run with chemical background and known truth, for the background removal of the TIC (`ticbg`, `docs/agenti/tic-fondo.md`).
The same model as `crates/mzlab-core/src/ticbg_synth.rs` (the Rust tests use that one): six background lines (m/z 141, 149, 158, 198, 214, 236)
with a 60 % drift over the run and 5 % noise, eight random noise ions per scan, and real peaks; the default peak at 14.3 min has ions at 198 and
158 (on two background lines) and 305. No lab data.

    python3 tools/tic_fondo_sintetico.py OUT_DIR [--scans N] [--seed N] [--blank]

Writes `tic_fondo-s<seed>.mzML` (or `tic_fondo_bianco-s<seed>.mzML` with --blank: the lines, no peak) and `<same name>_truth.json`
(background m/z, peaks with apex, sigma, ions and the true area of each peak in the TIC).
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_sintetici import _spec, _write  # noqa: E402

LINES = [(141.0, 30e3), (149.0, 80e3), (158.0, 25e3), (198.0, 40e3), (214.0, 20e3), (236.0, 50e3)]
PEAKS = [{"apex": 14.3, "sigma": 0.08, "ions": [(198.0, 50e3), (158.0, 30e3), (305.0, 40e3)]}]
RUN_MIN = 20.0


class Lcg:
    """The generator of `ticbg_synth.rs` (64-bit LCG, 53-bit uniform)."""

    def __init__(self, seed: int) -> None:
        self.s = seed

    def u(self) -> float:
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        return ((self.s >> 11) + 0.5) / (1 << 53)


def bump(t: float, apex: float, sigma: float) -> float:
    """Raised-cosine peak of the same area as a Gaussian of `sigma` (as in the Rust generator)."""
    d = (t - apex) / sigma
    return 0.5 * (1.0 + math.cos(math.pi * d / 2.5)) * 1.0026 if abs(d) < 2.5 else 0.0


def build(n: int, seed: int, blank: bool) -> tuple[list[str], dict]:
    r = Lcg(seed)
    specs, truth_area = [], [0.0 for _ in PEAKS]
    for j in range(n):
        t = RUN_MIN * j / n
        pts = []
        for mz, h in LINES:
            g = (r.u() + r.u() + r.u() + r.u() - 2.0) * 1.732
            pts.append((mz + 0.1 * (r.u() - 0.5), h * (1.0 + 0.6 * t / RUN_MIN) * (1.0 + 0.05 * g)))
        for _ in range(8):
            pts.append((round(100.0 + 300.0 * r.u()) + 0.2 * (r.u() - 0.5), 500.0 * r.u()))
        if not blank:
            for k, p in enumerate(PEAKS):
                g = bump(t, p["apex"], p["sigma"])
                if g > 0.0:
                    for mz, h in p["ions"]:
                        pts.append((mz + 0.1 * (r.u() - 0.5), h * g))
                        truth_area[k] += h * g
        pts.sort()
        specs.append(_spec(j, t, [p[0] for p in pts], [p[1] for p in pts]))
    truth = {"background_mz": [m for m, _ in LINES], "drift": 0.6, "noise": 0.05, "scans": n, "run_min": RUN_MIN,
             "peaks": [] if blank else [{**p, "ions": [list(i) for i in p["ions"]], "area_tic": a} for p, a in zip(PEAKS, truth_area)]}
    return specs, truth


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out", type=Path)
    ap.add_argument("--scans", type=int, default=1200)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--blank", action="store_true")
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    name = f"tic_fondo{'_bianco' if a.blank else ''}-s{a.seed}"
    specs, truth = build(a.scans, a.seed, a.blank)
    _write(a.out / f"{name}.mzML", specs, [])
    (a.out / f"{name}_truth.json").write_text(json.dumps(truth, indent=1) + "\n", encoding="utf-8")
    print(a.out / f"{name}.mzML")


if __name__ == "__main__":
    main()
