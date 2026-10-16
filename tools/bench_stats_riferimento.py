"""Reference generator for crates/mzlab-core/tests/bench_stats_parity.rs.

    python3 tools/bench_stats_riferimento.py [OUT.json]

Generates reference test cases for exact McNemar, Holm step-down correction,
non-inferiority, superiority, and bootstrap intervals/differences.
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

# Deterministic SplitMix64 matching Rust bench_stats::SplitMix64
class SplitMix64:
    def __init__(self, seed: int):
        self.state = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

    def gen_range(self, n: int) -> int:
        return 0 if n == 0 else self.next_u64() % n


def mcnemar_p_value_exact(n10: int, n01: int) -> float:
    n = n10 + n01
    if n == 0:
        return 1.0
    k = min(n10, n01)
    if k * 2 >= n:
        return 1.0
    s = sum(math.comb(n, i) for i in range(k + 1))
    return min(1.0, 2.0 * s / (2 ** n))


def holm_exact(p: list[float], alfa: float = 0.05) -> tuple[list[float], list[bool]]:
    m = len(p)
    if m == 0:
        return [], []
    ordine = sorted(range(m), key=lambda i: (p[i], i))
    adj = [0.0] * m
    corrente = 0.0
    for r, i in enumerate(ordine):
        corrente = max(corrente, min(1.0, (m - r) * p[i]))
        adj[i] = corrente
    return adj, [x < alfa for x in adj]


def non_inferiore(ic_basso: float, ic_alto: float, direzione_alto: bool, margine: float) -> bool:
    return bool(ic_basso > margine) if direzione_alto else bool(ic_alto < margine)


def superiore(ic_basso: float, ic_alto: float, direzione_alto: bool, rifiutata: bool) -> bool:
    lato = ic_basso > 0.0 if direzione_alto else ic_alto < 0.0
    return bool(lato and rifiutata)


def bootstrap_indices(n: int, n_boot: int, seed: int) -> list[list[int]]:
    rng = SplitMix64(seed)
    return [[rng.gen_range(n) for _ in range(n)] for _ in range(n_boot)]


def nanpercentile(vals: list[float], q: float) -> float | None:
    valid = sorted(x for x in vals if not math.isnan(x))
    if not valid:
        return None
    n = len(valid)
    if n == 1:
        return valid[0]
    q_clamped = max(0.0, min(100.0, q))
    idx = (n - 1) * (q_clamped / 100.0)
    i0 = int(math.floor(idx))
    i1 = min(i0 + 1, n - 1)
    f = idx - i0
    return (1.0 - f) * valid[i0] + f * valid[i1]


def make_mcnemar_cases() -> list[dict]:
    cases = []
    # Test grid covering n from 0 to 500
    for n in range(501):
        if n == 0:
            cases.append({"n10": 0, "n01": 0, "p": 1.0})
            continue
        ks = sorted({0, 1, n // 4, n // 2, (n * 3) // 8})
        for k in ks:
            n10, n01 = k, n - k
            p = mcnemar_p_value_exact(n10, n01)
            cases.append({"n10": n10, "n01": n01, "p": p})
    return cases


def make_mcnemar_vector_cases() -> list[dict]:
    rng = SplitMix64(42)
    cases = []
    for length in [0, 1, 5, 20, 50, 100, 300]:
        a = [rng.gen_range(2) for _ in range(length)]
        b = [rng.gen_range(2) for _ in range(length)]
        n10 = sum(1 for x, y in zip(a, b) if x == 1 and y == 0)
        n01 = sum(1 for x, y in zip(a, b) if x == 0 and y == 1)
        p = mcnemar_p_value_exact(n10, n01)
        cases.append({"a": a, "b": b, "n10": n10, "n01": n01, "p": p})
    return cases


def make_holm_cases() -> list[dict]:
    cases = []
    # Edge cases
    for p_vec in [
        [],
        [0.03],
        [0.0, 0.0, 0.0],
        [1.0, 1.0],
        [0.01, 0.04, 0.03],
        [0.05, 0.05, 0.05],
        [0.001, 0.008, 0.039, 0.041, 0.06],
    ]:
        for alfa in [0.01, 0.05, 0.10]:
            adj, rej = holm_exact(p_vec, alfa)
            cases.append({"p": p_vec, "alfa": alfa, "adj": adj, "rifiutata": rej})

    # Random p-value vectors
    rng = SplitMix64(12345)
    for length in [2, 4, 7, 10, 15, 25]:
        p_vec = [round((rng.next_u64() % 10000) / 10000.0, 4) for _ in range(length)]
        for alfa in [0.05]:
            adj, rej = holm_exact(p_vec, alfa)
            cases.append({"p": p_vec, "alfa": alfa, "adj": adj, "rifiutata": rej})
    return cases


def make_decision_cases() -> list[dict]:
    cases = []
    triplets = [
        (-0.05, 0.02, -0.02),
        (0.01, 0.08, -0.02),
        (-0.10, -0.01, 0.05),
        (-0.02, 0.04, 0.05),
        (0.02, 0.09, 0.01),
        (-0.08, -0.01, 0.01),
        (0.00, 0.05, 0.0),
        (-0.05, 0.00, 0.0),
    ]
    for lo, hi, margine in triplets:
        for dir_alto in [True, False]:
            for rej in [True, False]:
                ni = non_inferiore(lo, hi, dir_alto, margine)
                sup = superiore(lo, hi, dir_alto, rej)
                cases.append({
                    "ic_basso": lo, "ic_alto": hi,
                    "direzione_alto": dir_alto, "margine": margine,
                    "rifiutata": rej, "non_inferiore": ni, "superiore": sup,
                })
    return cases


def py_nanmedian(v: list[float]) -> float | None:
    valid = sorted(x for x in v if not math.isnan(x))
    if not valid:
        return None
    m = len(valid)
    if m % 2 == 1:
        return valid[m // 2]
    return (valid[m // 2 - 1] + valid[m // 2]) / 2.0


def make_bootstrap_cases() -> list[dict]:
    cases = []
    rng = SplitMix64(999)
    # Media cases
    for length in [10, 20, 30]:
        x = [round((rng.next_u64() % 1000) / 100.0, 2) for _ in range(length)]
        mean_pt = sum(x) / len(x)
        ix = bootstrap_indices(length, 200, 20261012)
        means = []
        for row in ix:
            sample = [x[i] for i in row]
            means.append(sum(sample) / len(sample))
        lo = nanpercentile(means, 2.5)
        hi = nanpercentile(means, 97.5)
        cases.append({"tipo": "media", "x": x, "valore": mean_pt, "ic_basso": lo, "ic_alto": hi})

    # Mediana cases
    for length in [10, 20, 30]:
        x = [round((rng.next_u64() % 1000) / 100.0, 2) for _ in range(length)]
        if length >= 20:
            x[3] = float("nan")
        med_pt = py_nanmedian(x)
        ix = bootstrap_indices(length, 200, 20261012)
        meds = []
        for row in ix:
            sample = [x[i] for i in row]
            m = py_nanmedian(sample)
            meds.append(float("nan") if m is None else m)
        lo = nanpercentile(meds, 2.5)
        hi = nanpercentile(meds, 97.5)
        x_json = [None if math.isnan(v) else v for v in x]
        cases.append({"tipo": "mediana", "x": x_json, "valore": med_pt, "ic_basso": lo, "ic_alto": hi})
    return cases


def main() -> None:
    data = {
        "mcnemar_cases": make_mcnemar_cases(),
        "mcnemar_vectors": make_mcnemar_vector_cases(),
        "holm_cases": make_holm_cases(),
        "decision_cases": make_decision_cases(),
        "bootstrap_cases": make_bootstrap_cases(),
    }
    payload = json.dumps(data)
    if len(sys.argv) > 1 and sys.argv[1] != "-":
        Path(sys.argv[1]).write_text(payload, encoding="utf-8")
        total_cases = sum(len(v) for v in data.values())
        print(f"Generated {total_cases} reference cases in {sys.argv[1]}")
    else:
        sys.stdout.write(payload)


if __name__ == "__main__":
    main()
