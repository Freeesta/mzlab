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


def generate_cases() -> list[dict]:
    rng = np.random.default_rng(12345)
    cases = []

    def add_case(name: str, dp: list[float], f_int: list[float], p_int: list[float]) -> None:
        res = source_ramp(dp, f_int, p_int)
        cases.append({
            "name": name,
            "input": {
                "dp": dp,
                "f_int": f_int,
                "p_int": p_int,
            },
            "output": sanitize_result(res),
        })

    # Group 1: n < 3 (casi limite < 3)
    add_case("n0_empty", [], [], [])
    add_case("n1_single", [25.0], [100.0], [500.0])
    add_case("n2_two_pts", [10.0, 30.0], [50.0, 200.0], [450.0, 300.0])
    add_case("n2_unsorted", [40.0, 15.0], [150.0, 30.0], [200.0, 400.0])
    add_case("n_in5_valid1", [10.0, 20.0, 30.0, 40.0, 50.0], [0.0, 100.0, 0.0, 0.0, 0.0], [0.0, 400.0, 0.0, 0.0, 0.0])
    add_case("n_in4_valid2", [10.0, 20.0, 30.0, 40.0], [0.0, 50.0, 0.0, 120.0], [0.0, 200.0, 0.0, 150.0])

    # Group 2: n == 3 (warning dp4, solo correlazione)
    add_case("n3_increasing", [10.0, 25.0, 45.0], [20.0, 80.0, 250.0], [500.0, 400.0, 200.0])
    add_case("n3_flat", [15.0, 30.0, 50.0], [100.0, 100.0, 100.0], [400.0, 400.0, 400.0])
    add_case("n3_decreasing", [10.0, 20.0, 30.0], [300.0, 150.0, 40.0], [100.0, 250.0, 400.0])
    add_case("n3_unsorted", [40.0, 10.0, 25.0], [200.0, 30.0, 90.0], [200.0, 450.0, 350.0])
    add_case("n3_ties_dp", [20.0, 20.0, 50.0], [50.0, 70.0, 250.0], [400.0, 380.0, 200.0])
    add_case("n_in6_valid3", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [0.0, 30.0, 0.0, 90.0, 0.0, 200.0], [0.0, 300.0, 0.0, 250.0, 0.0, 150.0])

    # Group 3: n = 4..12 realistic sigmoids (ISF chiari)
    for n in range(4, 13):
        dp_raw = np.sort(rng.uniform(10.0, 80.0, n))
        ymax = rng.uniform(0.5, 0.95)
        dp50 = rng.uniform(25.0, 60.0)
        s = rng.uniform(3.0, 12.0)
        ratio_true = ymax / (1.0 + np.exp(-(dp_raw - dp50) / s))
        ratio_obs = np.clip(ratio_true + rng.normal(0.0, 0.015, n), 0.01, 0.99)
        tot = 1000.0
        f_vals = ratio_obs * tot
        p_vals = (1.0 - ratio_obs) * tot
        # Randomly shuffle input order to test sorting
        perm = rng.permutation(n)
        add_case(f"sigmoid_n{n}", dp_raw[perm].tolist(), f_vals[perm].tolist(), p_vals[perm].tolist())

    # Group 4: Flat profiles (non-ISF)
    for n in range(4, 11):
        dp_raw = np.linspace(15.0, 75.0, n)
        ratio_const = 0.12 + rng.normal(0.0, 0.005, n)
        ratio_obs = np.clip(ratio_const, 0.01, 0.99)
        tot = 2000.0
        f_vals = ratio_obs * tot
        p_vals = (1.0 - ratio_obs) * tot
        add_case(f"flat_n{n}", dp_raw.tolist(), f_vals.tolist(), p_vals.tolist())

    # Group 5: Noisy profiles
    for n in range(4, 11):
        dp_raw = np.linspace(10.0, 70.0, n)
        ratio_obs = rng.uniform(0.1, 0.7, n)
        tot = 1500.0
        f_vals = ratio_obs * tot
        p_vals = (1.0 - ratio_obs) * tot
        add_case(f"noisy_n{n}", dp_raw.tolist(), f_vals.tolist(), p_vals.tolist())

    # Group 6: Edge cases with zeros / saturation
    add_case("all_f_zero_n4", [10.0, 20.0, 30.0, 40.0], [0.0, 0.0, 0.0, 0.0], [100.0, 200.0, 300.0, 400.0])
    add_case("all_p_zero_n4", [10.0, 20.0, 30.0, 40.0], [100.0, 200.0, 300.0, 400.0], [0.0, 0.0, 0.0, 0.0])
    add_case("low_dp_zero_f_n6", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [0.0, 0.0, 10.0, 40.0, 120.0, 250.0], [500.0, 450.0, 400.0, 300.0, 200.0, 50.0])
    add_case("high_dp_zero_p_n5", [15.0, 30.0, 45.0, 60.0, 75.0], [20.0, 80.0, 250.0, 500.0, 600.0], [450.0, 350.0, 150.0, 0.0, 0.0])
    add_case("constant_ratio_zero_var_n5", [10.0, 20.0, 30.0, 40.0, 50.0], [50.0, 50.0, 50.0, 50.0, 50.0], [50.0, 50.0, 50.0, 50.0, 50.0])
    add_case("extreme_ratios_n4", [10.0, 25.0, 50.0, 80.0], [1e-4, 1.0, 100.0, 1e4], [1e4, 100.0, 1.0, 1e-4])

    # Group 7: Internal NaNs / zero denominators
    add_case("nan_in_middle_n6", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [10.0, 30.0, 0.0, 90.0, 180.0, 300.0], [400.0, 350.0, 0.0, 250.0, 150.0, 50.0])
    add_case("two_nans_n7_valid5", [10.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0], [10.0, 0.0, 35.0, 0.0, 90.0, 180.0, 300.0], [400.0, 0.0, 320.0, 0.0, 250.0, 150.0, 50.0])
    add_case("three_nans_n8_valid5", [10.0, 20.0, 25.0, 30.0, 35.0, 40.0, 50.0, 60.0], [10.0, 0.0, 35.0, 0.0, 0.0, 90.0, 180.0, 300.0], [400.0, 0.0, 320.0, 0.0, 0.0, 250.0, 150.0, 50.0])
    add_case("nan_at_ends_n6_valid4", [10.0, 20.0, 30.0, 40.0, 50.0, 60.0], [0.0, 40.0, 90.0, 180.0, 250.0, 0.0], [0.0, 360.0, 310.0, 220.0, 150.0, 0.0])

    # Group 8: Inverted / decreasing ramps n >= 4
    for n in (4, 5, 7, 9):
        dp_raw = np.linspace(15.0, 65.0, n)
        ratio_obs = np.linspace(0.8, 0.1, n) + rng.normal(0.0, 0.01, n)
        ratio_obs = np.clip(ratio_obs, 0.01, 0.99)
        tot = 1200.0
        f_vals = ratio_obs * tot
        p_vals = (1.0 - ratio_obs) * tot
        add_case(f"inverted_n{n}", dp_raw.tolist(), f_vals.tolist(), p_vals.tolist())

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
