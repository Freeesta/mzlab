"""Reference answers of numpy's linear algebra for crates/mzlab-core/tests/statmat_parity.rs.

    python3 tools/statmat_riferimento.py OUT.json

Writes {svd: [{m, n, a (row-major), s}], eigh2: [{a, b, c, w, v (row-major 2x2)}], eigh: [{n, a, w}]} from fixed seeds: the singular values
and eigenvalues must agree to 1e-12 (relative), the 2x2 eigenvectors also in sign. No data of the laboratory.
"""
import json
import sys

import numpy as np


def main(argv) -> int:
    rng = np.random.default_rng(20261012)
    out = {"svd": [], "eigh2": [], "eigh": []}
    for m, n in [(2, 2), (5, 3), (3, 5), (6, 40), (40, 6), (7, 120), (12, 12), (1, 4)]:
        a = rng.normal(size=(m, n)) * rng.uniform(0.1, 100.0)
        out["svd"].append({"m": m, "n": n, "a": a.ravel().tolist(), "s": np.linalg.svd(a, compute_uv=False).tolist()})
    c = rng.normal(size=(8, 5)); c[:, 0] = c[:, 1] * 2          # a rank-deficient case
    out["svd"].append({"m": 8, "n": 5, "a": c.ravel().tolist(), "s": np.linalg.svd(c, compute_uv=False).tolist()})
    for _ in range(300):
        a, c2 = rng.uniform(0.01, 5, 2)
        b = rng.uniform(-3, 3) if _ % 10 else 0.0
        if abs(b) >= np.sqrt(a * c2) and b != 0:
            continue
        w, v = np.linalg.eigh(np.array([[a, b], [b, c2]]))
        out["eigh2"].append({"a": a, "b": b, "c": c2, "w": w.tolist(), "v": v.ravel().tolist()})
    for n in (3, 4, 7, 15):
        x = rng.normal(size=(n, n)); x = x + x.T
        out["eigh"].append({"n": n, "a": x.ravel().tolist(), "w": np.linalg.eigvalsh(x).tolist()})
    open(argv[0], "w", encoding="utf-8").write(json.dumps(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
