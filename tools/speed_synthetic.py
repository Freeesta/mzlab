#!/usr/bin/env python3
"""Synthetic unit-resolution triple-quad series for the speed study (bench only, numpy, every number invented): OUT_DIR/QQQ_###.mzML + targets.csv.
    python3 tools/speed_synthetic.py OUT_DIR [--files 245] [--targets 275] [--scans 2400] [--seed 7] [--jobs 4]
Q1 centroids on a 1 Da grid (m/z 50-1000 +0.1), a scan every 0.5 s over 20 min, Poisson background (~50 channels per scan), [M+H]+ and M+1 at 6 s FWHM,
10 % absent per file, intensity = counts x 20. targets.csv is `mz,rt_lo,rt_hi` (min). Same arguments = same bytes."""
from __future__ import annotations

import argparse
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_sintetici import _spec, _write  # noqa: E402  (helpers only)

GRID = np.arange(50.0, 1000.0 + 1e-9, 1.0) + 0.1
GAIN = 20.0
FWHM_S = 6.0
RT_END = 20.0
BG_LAMBDA = 50.0 / len(GRID)


def structure(n_targets: int, seed: int) -> list[tuple[float, float, float]]:
    """(m/z channel, apex RT min, counts at apex) of every target compound."""
    rng = np.random.default_rng(seed)
    ch = rng.choice(np.arange(60, len(GRID) - 2), size=n_targets, replace=False)
    rt = rng.uniform(1.0, RT_END - 1.0, n_targets)
    h = 10 ** rng.uniform(1.5, 4.0, n_targets)
    return [(float(GRID[c]), float(r), float(a)) for c, r, a in zip(ch, rt, h)]


def write_file(path: Path, comps: list, n_scans: int, seed: int) -> None:
    rng = np.random.default_rng(seed)
    rts = np.arange(n_scans) * (RT_END / n_scans)
    sigma = FWHM_S / 60.0 / 2.3548
    lam = np.full((n_scans, len(GRID)), BG_LAMBDA)
    index = {mz: k for k, mz in enumerate(np.round(GRID, 1))}
    for mz, rt0, h in comps:
        if rng.random() < 0.1:
            continue
        a = h * rng.uniform(0.3, 1.5)
        rt = rt0 + rng.normal(0.0, 0.03)
        prof = a * np.exp(-0.5 * ((rts - rt) / sigma) ** 2)
        k = index[round(mz, 1)]
        lam[:, k] += prof
        lam[:, k + 1] += prof * 0.11
    counts = rng.poisson(lam)
    specs = []
    for i in range(n_scans):
        nz = np.nonzero(counts[i])[0]
        specs.append(_spec(i, float(rts[i]), np.round(GRID[nz], 1), counts[i, nz] * GAIN))
    _write(path, specs, [])


def _one(job):
    path, comps, n_scans, seed = job
    write_file(Path(path), comps, n_scans, seed)
    return path


def make(out: Path, files: int = 245, targets: int = 275, scans: int = 2400, seed: int = 7, jobs: int = 4) -> list[Path]:
    out.mkdir(parents=True, exist_ok=True)
    comps = structure(targets, seed)
    with open(out / "targets.csv", "w", encoding="utf-8") as f:
        f.write("mz,rt_lo,rt_hi\n")
        for mz, rt, _ in comps:
            f.write(f"{mz:.1f},{max(rt - 0.5, 0.0):.3f},{rt + 0.5:.3f}\n")
    work = [(str(out / f"QQQ_{k:03d}.mzML"), comps, scans, seed * 1000 + k) for k in range(files)]
    with ProcessPoolExecutor(max_workers=jobs) as ex:
        return [Path(p) for p in ex.map(_one, work)]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("out", type=Path)
    ap.add_argument("--files", type=int, default=245)
    ap.add_argument("--targets", type=int, default=275)
    ap.add_argument("--scans", type=int, default=2400)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--jobs", type=int, default=4)
    a = ap.parse_args()
    paths = make(a.out, a.files, a.targets, a.scans, a.seed, a.jobs)
    size = sum(p.stat().st_size for p in paths)
    print(f"{len(paths)} files, {size / 1e6:.1f} MB, targets {a.out / 'targets.csv'}")


if __name__ == "__main__":
    main()
