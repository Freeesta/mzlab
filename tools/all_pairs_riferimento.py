"""Generatore di riferimento per all_pairs (bin_matrices, upper_bound, pairs_above, plain_cosine).

Usa direttamente le implementazioni in TP_Mine/py/tpmine/hr/similarity.py.
Genera casi sintetici deterministici per crates/mzlab-lib/tests/all_pairs_parity.rs.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "TP_Mine" / "py"))
sys.path.insert(0, str(ROOT))

from tpmine.hr.similarity import bin_matrices, upper_bound, pairs_above, plain_cosine  # noqa: E402


def pad_ragged(m_list: list[list[float]], w_list: list[list[float]], dtype=np.float64):
    n = len(m_list)
    k = max((len(row) for row in m_list), default=0)
    if k == 0:
        k = 1
    m_arr = np.zeros((n, k), dtype=np.float64)
    w_arr = np.zeros((n, k), dtype=dtype)
    for i, (m_row, w_row) in enumerate(zip(m_list, w_list)):
        for j, (mz, wt) in enumerate(zip(m_row, w_row)):
            m_arr[i, j] = mz
            w_arr[i, j] = wt
    return m_arr, w_arr


def run_py_case(m, w, p, binw, mmax, rows=None, cols=None, threshold=0.3, upper_triangle=True, tol=0.005, pc_ia=None, pc_ib=None):
    n = len(m)
    p_arr = np.asarray(p, dtype=np.float64)
    if n == 0:
        nb = int(mmax / binw) + 2
        f = np.zeros((0, nb), dtype=np.float32)
        l = np.zeros((0, nb), dtype=np.float32)
        fs = np.zeros((0, nb), dtype=np.float32)
        ls = np.zeros((0, nb), dtype=np.float32)
        u = np.zeros((0, 0), dtype=np.float32)
        return {
            "n_bins": nb, "f": [], "l": [], "fs": [], "ls": [], "u": [],
            "pairs_ia": [], "pairs_ib": [], "cosine_scores": []
        }

    m_pad, w_pad32 = pad_ragged(m, w, dtype=np.float32)
    f, l, fs, ls = bin_matrices(m_pad, w_pad32, p_arr, binw=binw, mmax=mmax)
    u = upper_bound(f, l, fs, ls, rows=rows, cols=cols)
    ia, ib = pairs_above(u, threshold=threshold, upper_triangle=upper_triangle)

    c_ia = ia if pc_ia is None else np.asarray(pc_ia, int)
    c_ib = ib if pc_ib is None else np.asarray(pc_ib, int)

    m_pad64, w_pad64 = pad_ragged(m, w, dtype=np.float64)
    if len(c_ia) == 0:
        scores = np.zeros(0, dtype=np.float64)
    else:
        scores = plain_cosine(m_pad64, w_pad64, c_ia, c_ib, tol=tol)

    return {
        "n_bins": int(f.shape[1]), "f": f.tolist(), "l": l.tolist(),
        "fs": fs.tolist(), "ls": ls.tolist(), "u": u.tolist(),
        "pairs_ia": ia.tolist(), "pairs_ib": ib.tolist(),
        "cosine_scores": scores.tolist(),
    }


def edge_cases() -> list[dict]:
    return [
        {"id": 0, "desc": "empty", "m": [], "w": [], "p": [], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.3, "upper_triangle": True, "tol": 0.005, "pc_ia": [], "pc_ib": []},
        {"id": 1, "desc": "single_no_peaks", "m": [[]], "w": [[]], "p": [200.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.3, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 2, "desc": "two_no_peaks", "m": [[], []], "w": [[], []], "p": [200.0, 300.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.3, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 3, "desc": "single_one_peak", "m": [[50.0]], "w": [[1.0]], "p": [100.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 4, "desc": "identical_peak", "m": [[50.0], [50.0]], "w": [[0.8], [0.8]], "p": [120.0, 120.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.2, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 5, "desc": "diff_peaks", "m": [[50.0], [60.0]], "w": [[1.0], [1.0]], "p": [120.0, 120.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.5, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 6, "desc": "peaks_in_tol", "m": [[50.001], [50.004]], "w": [[0.7], [0.9]], "p": [120.0, 120.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 7, "desc": "bin_boundary", "m": [[49.499, 49.501, 50.499, 50.501]], "w": [[0.1, 0.2, 0.3, 0.4]], "p": [100.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.05, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 8, "desc": "peak_exceed_mmax", "m": [[150.0]], "w": [[0.8]], "p": [200.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 9, "desc": "prec_lt_frag", "m": [[80.0]], "w": [[0.9]], "p": [50.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
    ]


def feature_cases() -> list[dict]:
    return [
        {"id": 10, "desc": "multi_same_bin", "m": [[50.1, 50.2, 50.3]], "w": [[0.3, 0.9, 0.4]], "p": [150.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 11, "desc": "smear_wrap", "m": [[0.1, 99.9]], "w": [[0.5, 0.6]], "p": [100.0], "binw": 1.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 12, "desc": "fractional_binw", "m": [[10.2, 25.7, 48.9]], "w": [[0.4, 0.5, 0.6]], "p": [80.0], "binw": 0.5, "mmax": 60.0,
         "rows": None, "cols": None, "threshold": 0.2, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 13, "desc": "large_binw", "m": [[10.0, 52.0, 95.0]], "w": [[0.3, 0.4, 0.5]], "p": [120.0], "binw": 5.0, "mmax": 100.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0], "pc_ib": [0]},
        {"id": 14, "desc": "ub_subset", "m": [[20.0, 40.0], [30.0, 50.0], [20.0, 50.0], [30.0, 40.0]],
         "w": [[0.5, 0.5], [0.5, 0.5], [0.6, 0.4], [0.4, 0.6]], "p": [100.0, 110.0, 100.0, 110.0], "binw": 1.0, "mmax": 100.0,
         "rows": [0, 2], "cols": [1, 3], "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0, 2], "pc_ib": [1, 3]},
        {"id": 15, "desc": "pairs_above_high", "m": [[20.0], [20.0]], "w": [[0.2], [0.2]], "p": [50.0, 50.0], "binw": 1.0, "mmax": 50.0,
         "rows": None, "cols": None, "threshold": 5.0, "upper_triangle": True, "tol": 0.005, "pc_ia": [], "pc_ib": []},
        {"id": 16, "desc": "pairs_above_full", "m": [[20.0], [20.0]], "w": [[0.5], [0.5]], "p": [50.0, 50.0], "binw": 1.0, "mmax": 50.0,
         "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": False, "tol": 0.005, "pc_ia": [0, 1], "pc_ib": [1, 0]},
        {"id": 17, "desc": "competing_b", "m": [[100.0], [100.001, 100.002]], "w": [[1.0], [0.3, 0.9]], "p": [200.0, 200.0],
         "binw": 1.0, "mmax": 150.0, "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 18, "desc": "competing_a", "m": [[100.001, 100.002], [100.0]], "w": [[0.4, 0.8], [1.0]], "p": [200.0, 200.0],
         "binw": 1.0, "mmax": 150.0, "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
        {"id": 19, "desc": "cross_matches", "m": [[100.0, 100.002], [100.001, 100.003]], "w": [[0.5, 0.6], [0.4, 0.7]],
         "p": [200.0, 200.0], "binw": 1.0, "mmax": 150.0, "rows": None, "cols": None, "threshold": 0.1, "upper_triangle": True, "tol": 0.005, "pc_ia": [0], "pc_ib": [1]},
    ]


def synthetic_random_cases(rng) -> list[dict]:
    cases = []
    for idx in range(30):
        n_spec = int(rng.integers(3, 8))
        k_peaks = int(rng.integers(5, 16))
        m_list, w_list, p_list = [], [], []
        for _s in range(n_spec):
            prec = float(rng.uniform(150.0, 400.0))
            p_list.append(prec)
            mz_vals = np.sort(rng.uniform(20.0, prec - 2.0, k_peaks))
            wt_vals = rng.uniform(0.05, 1.0, k_peaks)
            wt_vals /= np.linalg.norm(wt_vals)
            m_list.append(mz_vals.tolist())
            w_list.append(wt_vals.tolist())

        binw = float(rng.choice([0.5, 1.0, 2.0]))
        mmax = float(rng.choice([400.0, 500.0]))
        thresh = float(rng.choice([0.05, 0.15, 0.25]))
        upper_tri = bool(rng.choice([True, False]))
        tol = float(rng.choice([0.005, 0.01, 0.02]))

        rows, cols = ([0, 2], [1, n_spec - 1]) if (idx % 3 == 0 and n_spec > 3) else (None, None)
        cases.append({
            "id": 20 + idx, "desc": f"synthetic_random_{idx}",
            "m": m_list, "w": w_list, "p": p_list, "binw": binw, "mmax": mmax,
            "rows": rows, "cols": cols, "threshold": thresh,
            "upper_triangle": upper_tri, "tol": tol, "pc_ia": None, "pc_ib": None
        })
    return cases


def generate_cases() -> list[dict]:
    rng = np.random.default_rng(20261016)
    return edge_cases() + feature_cases() + synthetic_random_cases(rng)


def to_jsonable(obj):
    if isinstance(obj, (np.integer, int)):
        return int(obj)
    if isinstance(obj, (np.floating, float)):
        return float(obj)
    if isinstance(obj, np.ndarray):
        return obj.tolist()
    if isinstance(obj, (list, tuple)):
        return [to_jsonable(x) for x in obj]
    if isinstance(obj, dict):
        return {k: to_jsonable(v) for k, v in obj.items()}
    return obj


def main(argv=None) -> int:
    cases = generate_cases()
    results = []
    for c in cases:
        out = run_py_case(
            m=c["m"], w=c["w"], p=c["p"],
            binw=c["binw"], mmax=c["mmax"],
            rows=c["rows"], cols=c["cols"],
            threshold=c["threshold"],
            upper_triangle=c["upper_triangle"],
            tol=c["tol"],
            pc_ia=c["pc_ia"], pc_ib=c["pc_ib"]
        )
        results.append({
            "id": int(c["id"]),
            "desc": str(c["desc"]),
            "m": c["m"],
            "w": c["w"],
            "p": c["p"],
            "binw": float(c["binw"]),
            "mmax": float(c["mmax"]),
            "rows": [int(x) for x in c["rows"]] if c["rows"] is not None else None,
            "cols": [int(x) for x in c["cols"]] if c["cols"] is not None else None,
            "threshold": float(c["threshold"]),
            "upper_triangle": bool(c["upper_triangle"]),
            "tol": float(c["tol"]),
            "pc_ia": [int(x) for x in c["pc_ia"]] if c["pc_ia"] is not None else None,
            "pc_ib": [int(x) for x in c["pc_ib"]] if c["pc_ib"] is not None else None,
            "expected": to_jsonable(out)
        })

    out_file = argv[0] if argv and len(argv) > 0 else "all_pairs_ref.json"
    content = json.dumps(results)
    if out_file == "-":
        sys.stdout.write(content)
    else:
        Path(out_file).write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
