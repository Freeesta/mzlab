"""Reference answers of `ionfamily.estimate_components` for crates/mzlab-core/tests/components_parity.rs.

    python3 tools/componenti_riferimento.py OUT.json

Writes a list of {n, m, max_k, min_var, d (row-major), result} from fixed seeds: matrices of 1 to 4 Gaussian elution profiles with random
spectra plus noise (and noise-free and too-small ones). The result is the dict of Python, as is. No data of the laboratory.
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.ionfamily import estimate_components  # noqa: E402


def matrix(rng, n, m, comps, noise):
    t = np.arange(n, dtype=float)
    d = np.zeros((n, m))
    for _ in range(comps):
        c, w = rng.uniform(0.15, 0.85) * n, rng.uniform(0.04, 0.12) * n
        d += np.outer(np.exp(-0.5 * ((t - c) / w) ** 2), rng.uniform(0, 1, m) * rng.uniform(50, 500))
    return d + rng.normal(0, noise, (n, m)) if noise else d


def main(argv) -> int:
    rng = np.random.default_rng(20261012)
    cases = []
    for n, m, comps, noise in [(120, 30, 1, 1.0), (120, 30, 2, 1.0), (200, 40, 3, 2.0), (300, 25, 2, 0.5), (90, 60, 4, 3.0), (400, 20, 1, 5.0),
                               (150, 12, 2, 0.0), (60, 8, 3, 1.0), (500, 50, 3, 1.5), (3, 5, 1, 1.0), (50, 1, 1, 1.0), (40, 6, 0, 1.0)]:
        d = matrix(rng, n, m, comps, noise)
        for max_k, min_var in ((4, 0.002), (2, 0.01)):
            cases.append({"n": n, "m": m, "max_k": max_k, "min_var": min_var, "d": d.ravel().tolist(), "result": estimate_components(d, max_k, min_var)})
    open(argv[0], "w", encoding="utf-8").write(json.dumps(cases))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
