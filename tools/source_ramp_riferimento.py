"""Reference generator for source_ramp parity testing against mzlab.ionfamily.source_ramp.

Usage:
    python3 tools/source_ramp_riferimento.py [OUT.json]
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.ionfamily import source_ramp  # noqa: E402


def f(v: float | None) -> float | None:
    if v is None:
        return None
    val = float(v)
    return None if math.isnan(val) or math.isinf(val) else val


def sanitize_result(res: dict) -> dict:
    warnings = []
    for w in res.get("warnings", []):
        if isinstance(w, dict):
            warnings.append(w.get("key", ""))
        else:
            warnings.append(str(w))

    out: dict = {
        "n": int(res["n"]),
        "dp": [float(x) for x in res["dp"]],
        "ratio": [f(x) for x in res["ratio"]],
        "warnings": warnings,
        "spearman": f(res.get("spearman")),
        "slope_per_v": f(res.get("slope_per_V")),
        "dp_dependence": f(res.get("dp_dependence")),
    }
    sig = res.get("sigmoid")
    if sig is not None:
        out["sigmoid"] = {
            "ymax": float(sig["ymax"]),
            "dp50": float(sig["dp50"]),
            "s": float(sig["s"]),
            "dp_appearance": float(sig["dp_appearance"]),
            "r2": f(sig.get("r2")),
        }
    else:
        out["sigmoid"] = None
    return out


def add_case(cases: list[dict], name: str, dp: list[float], f_int: list[float], p_int: list[float]) -> None:
    res = source_ramp(dp, f_int, p_int)
    cases.append({
        "name": name,
        "input": {"dp": dp, "f_int": f_int, "p_int": p_int},
        "output": sanitize_result(res),
    })


def add_small_cases(cases: list[dict]) -> None:
    add_case(cases, "n0_empty", [], [], [])
    add_case(cases, "n1_single", [25.0], [100.0], [500.0])
    add_case(cases, "n2_two_pts", [10.0, 30.0], [50.0, 200.0], [450.0, 300.0])
    add_case(cases, "n2_unsorted", [40.0, 15.0], [150.0, 30.0], [200.0, 400.0])
    add_case(cases, "n_in5_valid1", [10.0, 20.0, 30.0, 40.0, 50.0], [0.0, 100.0, 0.0, 0.0, 0.0], [0.0, 400.0, 0.0, 0.0, 0.0])
    add_case(cases, "n_in4_valid2", [10.0, 20.0, 30.0, 40.0], [0.0, 50.0, 0.0, 120.0], [0.0, 200.0, 0.0, 150.0])
    add_case(cases, "n3_increasing", [10.0, 25.0, 45.0], [20.0, 80.0, 250.0], [500.0, 400.0, 200.0])
    add_case(cases, "n3_flat", [15.0, 30.0, 50.0], [100.0, 100.0, 100.0], [400.0, 400.0, 400.0])
    add_case(cases, "n3_decreasing", [10.0, 20.0, 30.0], [300.0, 150.0, 40.0], [100.0, 250.0, 400.0])
    add_case(cases, "n3_unsorted", [40.0, 10.0, 25.0], [200.0, 30.0, 90.0], [200.0, 450.0, 350.0])
    add_case(cases, "n3_ties_dp", [20.0, 20.0, 50.0], [50.0, 70.0, 250.0], [400.0, 380.0, 200.0])
    add_case(cases, "n_in6_valid3", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [0.0, 30.0, 0.0, 90.0, 0.0, 200.0], [0.0, 300.0, 0.0, 250.0, 0.0, 150.0])


def add_sigmoid_cases(cases: list[dict], rng: np.random.Generator) -> None:
    for n in range(4, 13):
        dp_raw = np.sort(rng.uniform(10.0, 80.0, n))
        ymax, dp50, s = rng.uniform(0.5, 0.95), rng.uniform(25.0, 60.0), rng.uniform(3.0, 12.0)
        ratio_true = ymax / (1.0 + np.exp(-(dp_raw - dp50) / s))
        ratio_obs = np.clip(ratio_true + rng.normal(0.0, 0.015, n), 0.01, 0.99)
        tot = 1000.0
        f_vals, p_vals = ratio_obs * tot, (1.0 - ratio_obs) * tot
        perm = rng.permutation(n)
        add_case(cases, f"sigmoid_n{n}", dp_raw[perm].tolist(), f_vals[perm].tolist(), p_vals[perm].tolist())


def add_edge_cases(cases: list[dict], rng: np.random.Generator) -> None:
    for n in range(4, 11):
        dp_raw = np.linspace(15.0, 75.0, n)
        ratio_const = np.clip(0.12 + rng.normal(0.0, 0.005, n), 0.01, 0.99)
        add_case(cases, f"flat_n{n}", dp_raw.tolist(), (ratio_const * 2000.0).tolist(), ((1.0 - ratio_const) * 2000.0).tolist())

    add_case(cases, "all_f_zero_n4", [10.0, 20.0, 30.0, 40.0], [0.0] * 4, [100.0, 200.0, 300.0, 400.0])
    add_case(cases, "all_p_zero_n4", [10.0, 20.0, 30.0, 40.0], [100.0, 200.0, 300.0, 400.0], [0.0] * 4)
    add_case(cases, "low_dp_zero_f_n6", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [0.0, 0.0, 10.0, 40.0, 120.0, 250.0], [500.0, 450.0, 400.0, 300.0, 200.0, 50.0])
    add_case(cases, "high_dp_zero_p_n5", [15.0, 30.0, 45.0, 60.0, 75.0], [20.0, 80.0, 250.0, 500.0, 600.0], [450.0, 350.0, 150.0, 0.0, 0.0])
    add_case(cases, "constant_ratio_zero_var_n5", [10.0, 20.0, 30.0, 40.0, 50.0], [50.0] * 5, [50.0] * 5)
    add_case(cases, "nan_in_middle_n6", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [10.0, 30.0, 0.0, 90.0, 180.0, 300.0], [400.0, 350.0, 0.0, 250.0, 150.0, 50.0])
    add_case(cases, "two_nans_n7_valid5", [10.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0], [10.0, 0.0, 35.0, 0.0, 90.0, 180.0, 300.0], [400.0, 0.0, 320.0, 0.0, 250.0, 150.0, 50.0])

    for n in (4, 5, 6, 7, 8, 9):
        dp_raw = np.linspace(15.0, 65.0, n)
        ratio_obs = np.clip(np.linspace(0.8, 0.1, n) + rng.normal(0.0, 0.01, n), 0.01, 0.99)
        add_case(cases, f"inverted_n{n}", dp_raw.tolist(), (ratio_obs * 1200.0).tolist(), ((1.0 - ratio_obs) * 1200.0).tolist())


def generate_cases() -> list[dict]:
    rng = np.random.default_rng(12345)
    cases: list[dict] = []
    add_small_cases(cases)
    add_sigmoid_cases(cases, rng)
    add_edge_cases(cases, rng)
    return cases


def main() -> None:
    cases = generate_cases()
    payload = {"count": len(cases), "cases": cases}
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    else:
        print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    main()
