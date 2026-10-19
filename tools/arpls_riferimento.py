"""Independent reference of the arPLS baseline (Baek et al., Analyst 140:250, 2015) as used by mzmine 4.10: dense matrices and numpy.linalg.solve.

    python3 tools/arpls_riferimento.py        # writes tests/data/untargeted/arpls/casi.json (the Rust test reads it)

Rules of mzmine 4.10.90 `ArplsBaseline`: A = W + lambda D2'D2, rhs = W y; d = y - z; mean/std (population) of the d < 0; cutoff = 2 std - mean;
w = 1 / (1 + exp(2 (d - cutoff) / std)); stop when ||dw|| / ||w|| < 1e-6, when no d < 0 or when std == 0; n < 3 gives zeros. Output max(y - z, 0).
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

RATIO = 1e-6
OUT = Path(__file__).resolve().parent.parent / "tests/data/untargeted/arpls/casi.json"


def baseline(y, lam, max_iter):
    y = np.asarray(y, float)
    n = len(y)
    if n < 3:
        return np.zeros(n)
    d2 = np.diff(np.eye(n), 2, axis=0)
    pen = lam * d2.T @ d2
    w = np.ones(n)
    z = np.zeros(n)
    for _ in range(max_iter):
        z = np.linalg.solve(np.diag(w) + pen, w * y)
        d = y - z
        neg = d[d < 0]
        if len(neg) == 0:
            return z
        m = neg.mean()
        s = np.sqrt(((neg - m) ** 2).sum() / len(neg))
        if s == 0:
            return z
        wn = 1 / (1 + np.exp(2 * (d - (2 * s - m)) / s))
        change = np.linalg.norm(wn - w) / np.linalg.norm(w)
        w = wn
        if change < RATIO:
            break
    return z


def cases():
    rng = np.random.default_rng(410)
    t = np.arange(120)
    ramp_peak = 50 + 2.0 * t + 4000 * np.exp(-0.5 * ((t - 60) / 4.0) ** 2)
    noise = 100 + rng.normal(0, 5, 80)
    return [
        ("ramp_and_gaussian_peak", ramp_peak, 1e5, 50),
        ("ramp_and_gaussian_peak_small_lambda", ramp_peak, 1e2, 50),
        ("noise_only", noise, 1e4, 50),
        ("constant", np.full(30, 7.5), 1e5, 50),
        ("zeros", np.zeros(20), 1e5, 50),
        ("n3", [1.0, 5.0, 2.0], 1e5, 50),
        ("n4", [1.0, 5.0, 2.0, 3.0], 1e3, 50),
        ("n2", [4.0, 9.0], 1e5, 50),
        ("two_iterations_only", ramp_peak[:60], 1e5, 2),
    ]


def main():
    out = []
    for name, y, lam, it in cases():
        y = np.asarray(y, float)
        z = baseline(y, lam, it)
        out.append({"name": name, "y": y.tolist(), "lambda": lam, "max_iter": it,
                    "baseline": z.tolist(), "corrected": np.maximum(y - z, 0).tolist()})
    OUT.write_text(json.dumps(out, indent=0) + "\n")


if __name__ == "__main__":
    main()
