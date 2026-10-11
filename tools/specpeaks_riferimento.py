"""Reference answers of mzlab/peaks.py (profile_peaks, merge_unit, pad_zeros) for crates/mzlab-core/tests/specpeaks_parity.rs.

    python3 tools/specpeaks_riferimento.py OUT.json

Synthetic spectra only (seeded numpy generator): [{kind, mz, y, step?, peaks: [[mz, y], ...]}].
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.peaks import merge_unit, pad_zeros, profile_peaks  # noqa: E402


def profile(rng, step, n_peaks, noise, gaps):
    centres = np.sort(rng.uniform(60, 600, n_peaks))
    mz = np.arange(50.0, 620.0, step)
    y = np.zeros_like(mz)
    for c in centres:
        y += rng.uniform(1e2, 1e5) * np.exp(-0.5 * ((mz - c) / rng.uniform(0.15, 0.35)) ** 2)
    y += rng.uniform(0, noise, len(mz))
    keep = y > noise * 0.3 if gaps else np.ones(len(mz), bool)
    return mz[keep], np.round(y[keep], 3)


def main() -> None:
    rng = np.random.default_rng(7)
    out = []
    for k in range(60):
        step = float(rng.choice([0.05, 0.1, 0.07]))
        mz, y = profile(rng, step, int(rng.integers(0, 40)), float(rng.choice([0.0, 5.0, 200.0])), bool(k % 2))
        out.append({"kind": "profile", "mz": mz.tolist(), "y": y.tolist(), "min_rel": float(rng.choice([0.002, 0.0, 0.05])),
                    "peaks": [a.tolist() for a in profile_peaks(mz, y, 0.002 if False else 0.002)]})
        out[-1]["peaks"] = [a.tolist() for a in profile_peaks(mz, y, out[-1]["min_rel"])]
        out.append({"kind": "pad", "mz": mz.tolist(), "y": y.tolist(), "step": step, "peaks": [a.tolist() for a in pad_zeros(mz, y, step)]})
    for n in [0, 1, 2, 4, 5, 6]:
        mz = np.sort(rng.uniform(100, 110, n))
        y = rng.uniform(0, 10, n)
        out.append({"kind": "profile", "mz": mz.tolist(), "y": y.tolist(), "min_rel": 0.002, "peaks": [a.tolist() for a in profile_peaks(mz, y, 0.002)]})
    for k in range(40):
        n = int(rng.integers(0, 300))
        mz = rng.uniform(50, 500, n)
        y = rng.uniform(0, 1e5, n)
        out.append({"kind": "merge", "mz": mz.tolist(), "y": y.tolist(), "peaks": [a.tolist() for a in merge_unit(mz, y)]})
    Path(sys.argv[1]).write_text(json.dumps(out), encoding="utf-8")
    print(len(out))


if __name__ == "__main__":
    main()
