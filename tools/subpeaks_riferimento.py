"""Reference answers of subpeaks (TP_Mine/py/tpmine/hr/spectra.py) for crates/mzlab-core/tests/subpeaks_parity.rs.

    python3 tools/subpeaks_riferimento.py OUT.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from TP_Mine.py.tpmine.hr.spectra import subpeaks  # noqa: E402


def short_cases() -> list[dict]:
    cases = []
    cases.append({"case": "empty", "rt": [], "y": [], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len1", "rt": [1.5], "y": [10.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len2", "rt": [1.0, 2.0], "y": [5.0, 15.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len3_mid", "rt": [1.0, 2.0, 3.0], "y": [10.0, 50.0, 20.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len3_tie", "rt": [1.0, 2.0, 3.0], "y": [50.0, 50.0, 20.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len4_apex", "rt": [1.0, 1.5, 2.0, 2.5], "y": [5.0, 25.0, 30.0, 10.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    cases.append({"case": "len4_zeros", "rt": [1.0, 2.0, 3.0, 4.0], "y": [0.0, 0.0, 0.0, 0.0], "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    return cases


def flat_cases() -> list[dict]:
    cases = []
    for n in (10, 25, 50):
        rt = np.linspace(1.0, 1.0 + 0.02 * n, n).tolist()
        cases.append({"case": f"flat_zeros_{n}", "rt": rt, "y": [0.0] * n, "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    for n in (15, 30, 60):
        rt = np.linspace(2.0, 2.0 + 0.01 * n, n).tolist()
        cases.append({"case": f"flat_const_{n}", "rt": rt, "y": [100.0] * n, "min_rel": 0.15, "smooth_min": 0.03, "min_sep": 0.08})
    rng = np.random.default_rng(123)
    for n in (20, 40):
        rt = np.linspace(0.5, 0.5 + 0.015 * n, n).tolist()
        y = (100.0 + rng.uniform(-0.001, 0.001, n)).tolist()
        cases.append({"case": f"flat_noise_{n}", "rt": rt, "y": y, "min_rel": 0.2, "smooth_min": 0.03, "min_sep": 0.08})
    return cases


def isolated_cases(rng: np.random.Generator) -> list[dict]:
    cases = []
    for i in range(12):
        n = int(rng.integers(30, 80))
        step = float(rng.choice([0.005, 0.01, 0.02]))
        rt = np.arange(0.0, n * step, step)
        c = float(rng.uniform(rt[5], rt[-6]))
        w = float(rng.uniform(0.015, 0.04))
        h = float(rng.uniform(1e2, 1e5))
        y = h * np.exp(-0.5 * ((rt - c) / w) ** 2)
        if i % 2 == 1:
            y += rng.normal(0, 0.01 * h, len(rt))
            y = np.maximum(y, 0.0)
        cases.append({
            "case": f"isolated_{i}",
            "rt": rt.tolist(),
            "y": y.tolist(),
            "min_rel": float(rng.choice([0.1, 0.15, 0.25])),
            "smooth_min": float(rng.choice([0.02, 0.03, 0.05])),
            "min_sep": float(rng.choice([0.05, 0.08, 0.1])),
        })
    return cases


def isomer_cases(rng: np.random.Generator) -> list[dict]:
    cases = []
    for i in range(12):
        n = int(rng.integers(40, 100))
        step = 0.01
        rt = np.arange(0.0, n * step, step)
        c1 = float(rng.uniform(0.2, rt[-1] - 0.3))
        sep = float(rng.choice([0.04, 0.06, 0.09, 0.12]))
        c2 = c1 + sep
        w1 = float(rng.uniform(0.015, 0.025))
        w2 = float(rng.uniform(0.015, 0.025))
        h1 = 1000.0
        h2 = float(rng.uniform(200.0, 900.0))
        y = h1 * np.exp(-0.5 * ((rt - c1) / w1) ** 2) + h2 * np.exp(-0.5 * ((rt - c2) / w2) ** 2)
        cases.append({
            "case": f"isomer_{i}",
            "rt": rt.tolist(),
            "y": y.tolist(),
            "min_rel": 0.15,
            "smooth_min": 0.03,
            "min_sep": 0.08,
        })
    return cases


def close_peaks_cases(rng: np.random.Generator) -> list[dict]:
    cases = []
    for i in range(12):
        n = int(rng.integers(60, 120))
        step = 0.01
        rt = np.arange(0.0, n * step, step)
        y = np.zeros_like(rt)
        k = int(rng.choice([3, 4]))
        base_c = float(rng.uniform(0.2, 0.3))
        for j in range(k):
            c = base_c + j * float(rng.uniform(0.04, 0.10))
            h = float(rng.uniform(300.0, 1500.0))
            w = float(rng.uniform(0.012, 0.025))
            y += h * np.exp(-0.5 * ((rt - c) / w) ** 2)
        cases.append({
            "case": f"close_cluster_{i}",
            "rt": rt.tolist(),
            "y": y.tolist(),
            "min_rel": float(rng.choice([0.1, 0.15, 0.2])),
            "smooth_min": float(rng.choice([0.02, 0.03, 0.04])),
            "min_sep": float(rng.choice([0.05, 0.08, 0.10])),
        })
    return cases


def multi_chromatogram_cases(rng: np.random.Generator) -> list[dict]:
    cases = []
    for i in range(15):
        n = 150
        step = 0.02
        rt = np.arange(0.0, n * step, step)
        y = np.zeros_like(rt)
        n_peaks = int(rng.integers(4, 9))
        for _ in range(n_peaks):
            c = float(rng.uniform(0.2, rt[-1] - 0.2))
            h = float(rng.uniform(100.0, 2000.0))
            w = float(rng.uniform(0.015, 0.035))
            y += h * np.exp(-0.5 * ((rt - c) / w) ** 2)
        y += rng.normal(0, 5.0, len(rt))
        y = np.maximum(y, 0.0)
        cases.append({
            "case": f"chrom_{i}",
            "rt": rt.tolist(),
            "y": y.tolist(),
            "min_rel": float(rng.choice([0.10, 0.15, 0.20])),
            "smooth_min": float(rng.choice([0.03, 0.04])),
            "min_sep": float(rng.choice([0.06, 0.08, 0.10])),
        })
    return cases


def build_all() -> list[dict]:
    rng = np.random.default_rng(777)
    raw = (
        short_cases()
        + flat_cases()
        + isolated_cases(rng)
        + isomer_cases(rng)
        + close_peaks_cases(rng)
        + multi_chromatogram_cases(rng)
    )
    for row in raw:
        rt = np.asarray(row["rt"], float)
        y = np.asarray(row["y"], float)
        ans = subpeaks(
            rt,
            y,
            min_rel=row["min_rel"],
            smooth_min=row["smooth_min"],
            min_sep=row["min_sep"],
        )
        row["expected"] = [float(x) for x in ans]
    return raw


def main() -> None:
    cases = build_all()
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("subpeaks_ref.json")
    out_path.write_text(json.dumps(cases, indent=2), encoding="utf-8")
    print(f"Written {len(cases)} cases to {out_path}")


if __name__ == "__main__":
    main()
