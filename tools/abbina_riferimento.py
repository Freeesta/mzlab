"""Reference generator for deterministic m/z-RT bipartite matching (abbina).

    python3 tools/abbina_riferimento.py [OUT.json]

Generates synthetic test cases covering edge cases (empty inputs, singletons,
boundary values, exact ties on m/z and RT, greedy contention chains) and seeded
random cases, with ground truth matching computed via tools/valida/abbina.py.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from tools.valida.abbina import abbina


def make_case(name: str, mz_a, rt_a, mz_b, rt_b, ppm: float = 5.0, rt_tol: float = 0.15) -> dict:
    mz_a = np.asarray(mz_a, float)
    rt_a = np.asarray(rt_a, float)
    mz_b = np.asarray(mz_b, float)
    rt_b = np.asarray(rt_b, float)
    out_a, out_b = abbina(mz_a, rt_a, mz_b, rt_b, ppm=ppm, rt_tol=rt_tol)
    return {
        "name": name,
        "mz_a": mz_a.tolist(),
        "rt_a": rt_a.tolist(),
        "mz_b": mz_b.tolist(),
        "rt_b": rt_b.tolist(),
        "ppm": float(ppm),
        "rt_tol": float(rt_tol),
        "out_a": out_a.tolist(),
        "out_b": out_b.tolist(),
    }


def edge_cases() -> list[dict]:
    cases = [
        make_case("empty_both", [], [], [], []),
        make_case("empty_a", [], [], [100.0], [1.0]),
        make_case("empty_b", [100.0], [1.0], [], []),
        make_case("single_match_exact", [200.0], [5.0], [200.0], [5.0]),
        make_case("single_mz_mismatch_high", [200.0], [5.0], [200.05], [5.0]),
        make_case("single_mz_mismatch_low", [200.0], [5.0], [199.95], [5.0]),
        make_case("single_rt_mismatch_high", [200.0], [5.0], [200.0], [5.20]),
        make_case("single_rt_mismatch_low", [200.0], [5.0], [200.0], [4.80]),
        make_case("exact_boundary_mz_high", [200.0], [5.0], [200.0 * (1.0 + 5.0 * 1e-6)], [5.0]),
        make_case("exact_boundary_mz_low", [200.0], [5.0], [200.0 * (1.0 - 5.0 * 1e-6)], [5.0]),
        make_case("exact_boundary_rt_high", [200.0], [5.0], [200.0], [5.15]),
        make_case("exact_boundary_rt_low", [200.0], [5.0], [200.0], [4.85]),
        make_case("just_outside_mz_high", [200.0], [5.0], [200.0 * (1.0 + 5.001 * 1e-6)], [5.0]),
        make_case("just_outside_mz_low", [200.0], [5.0], [200.0 * (1.0 - 5.001 * 1e-6)], [5.0]),
        make_case("just_outside_rt_high", [200.0], [5.0], [200.0], [5.1501]),
        make_case("just_outside_rt_low", [200.0], [5.0], [200.0], [4.8499]),
        make_case("duplicate_in_a", [300.0, 300.0], [2.0, 2.0], [300.0], [2.0]),
        make_case("duplicate_in_b", [300.0], [2.0], [300.0, 300.0], [2.0, 2.0]),
        make_case("cross_tie_2x2", [300.0, 300.0], [2.0, 2.0], [300.0, 300.0], [2.0, 2.0]),
        make_case("duplicate_3a_2b", [300.0, 300.0, 300.0], [2.0, 2.0, 2.0], [300.0, 300.0], [2.0, 2.0]),
        make_case("duplicate_2a_3b", [300.0, 300.0], [2.0, 2.0], [300.0, 300.0, 300.0], [2.0, 2.0, 2.0]),
        make_case("equidistant_rt_tie", [200.0], [5.0], [200.0, 200.0], [5.1, 4.9]),
        make_case("equidistant_mz_tie", [200.0], [5.0], [200.0 * (1.0 + 2e-6), 200.0 * (1.0 - 2e-6)], [5.0, 5.0]),
        make_case("greedy_contention_2x2", [100.0, 100.0], [5.0, 5.08], [100.0, 100.0], [5.06, 5.12]),
        make_case(
            "greedy_chain_4",
            [100.0, 100.0, 100.0, 100.0],
            [1.00, 1.10, 1.20, 1.30],
            [100.0, 100.0, 100.0, 100.0],
            [1.04, 1.12, 1.22, 1.35],
        ),
        make_case("unsorted_mz_inputs", [500.0, 100.0, 300.0], [1.0, 2.0, 3.0], [300.0, 500.0, 100.0], [3.0, 1.0, 2.0]),
        make_case("custom_tol_wide", [150.0, 250.0], [1.0, 2.0], [150.003, 250.005], [1.3, 1.8], ppm=25.0, rt_tol=0.5),
        make_case("custom_tol_tight", [150.0, 250.0], [1.0, 2.0], [150.0001, 250.001], [1.01, 2.05], ppm=1.0, rt_tol=0.02),
    ]
    return cases


def seeded_random_cases() -> list[dict]:
    rng = np.random.default_rng(20261016)
    cases = []
    configs = [
        (5, 5, 5.0, 0.15),
        (8, 6, 5.0, 0.15),
        (10, 12, 10.0, 0.20),
        (15, 15, 3.0, 0.05),
        (25, 20, 5.0, 0.15),
        (30, 35, 7.5, 0.25),
        (50, 45, 5.0, 0.15),
        (80, 80, 5.0, 0.15),
        (120, 100, 10.0, 0.30),
        (150, 160, 5.0, 0.15),
    ]
    for cfg_idx, (na, nb, ppm, rt_tol) in enumerate(configs):
        for rep in range(6):
            mza = rng.uniform(80.0, 1200.0, na)
            rta = rng.uniform(0.5, 25.0, na)
            mzb = rng.uniform(80.0, 1200.0, nb)
            rtb = rng.uniform(0.5, 25.0, nb)
            k = min(na, nb, 4 + rep * 2)
            noise_ppm = rng.uniform(-ppm * 0.9, ppm * 0.9, k)
            noise_rt = rng.uniform(-rt_tol * 0.9, rt_tol * 0.9, k)
            mzb[:k] = mza[:k] * (1.0 + noise_ppm * 1e-6)
            rtb[:k] = rta[:k] + noise_rt
            if rep % 2 == 1 and k >= 2:
                mzb[k - 1] = mzb[k - 2]
                rtb[k - 1] = rtb[k - 2]
            cases.append(make_case(f"random_cfg{cfg_idx}_rep{rep}", mza, rta, mzb, rtb, ppm=ppm, rt_tol=rt_tol))
    return cases


def generate_all() -> dict:
    all_cases = edge_cases() + seeded_random_cases()
    return {"cases": all_cases}


def main(argv: list[str]) -> int:
    data = generate_all()
    payload = json.dumps(data, indent=1)
    if len(argv) > 0:
        Path(argv[0]).write_text(payload, encoding="utf-8")
        print(f"wrote {len(data['cases'])} test cases to {argv[0]}")
    else:
        print(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
