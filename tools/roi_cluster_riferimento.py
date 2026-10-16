"""Reference answers for ROI correlation and average linkage clustering.

Generates deterministic synthetic cases comparing mzlab/ionfamily.py functions:
`_pairwise_windowed_corr` and `_average_linkage`.

Usage:
    python3 tools/roi_cluster_riferimento.py OUT.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.ionfamily import _average_linkage, _pairwise_windowed_corr  # noqa: E402


def f(v: float) -> float | None:
    v = float(v)
    return None if np.isnan(v) else v


def f_mat(M: np.ndarray) -> list[list[float | None]]:
    return [[f(v) for v in row] for row in M]


def gaussian(n: int, center: float, width: float) -> np.ndarray:
    x = np.arange(n, dtype=float)
    return np.exp(-0.5 * ((x - center) / max(width, 1e-6)) ** 2)


def make_cases() -> tuple[list[dict], list[dict]]:
    rng = np.random.default_rng(20261016)
    trace_cases: list[dict] = []
    linkage_cases: list[dict] = []

    # 1. m = 0
    trace_cases.append({
        "id": "m0_empty",
        "traces": [],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # 2. m = 1
    t1_gauss = gaussian(30, 15.0, 3.0)
    trace_cases.append({
        "id": "m1_gaussian",
        "traces": [t1_gauss.tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m1_zero",
        "traces": [np.zeros(20).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.5,
    })
    trace_cases.append({
        "id": "m1_constant_one",
        "traces": [np.ones(15).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m1_single_point",
        "traces": [[5.0]],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # 3. m = 2
    # 3.1 Identical gaussian
    g1 = gaussian(40, 20.0, 4.0)
    for cut_val in [0.1, 0.3, 0.8]:
        trace_cases.append({
            "id": f"m2_identical_cut_{cut_val}",
            "traces": [g1.tolist(), g1.tolist()],
            "thr": 0.05,
            "min_overlap": 4,
            "cut": cut_val,
        })

    # 3.2 Scaled peaks
    trace_cases.append({
        "id": "m2_scaled",
        "traces": [g1.tolist(), (g1 * 3.5).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # 3.3 Slight noise (correlated ~0.99)
    noise = rng.normal(0, 0.02, size=len(g1))
    g_noisy = np.clip(g1 + noise, 0.0, None)
    trace_cases.append({
        "id": "m2_slight_noise",
        "traces": [g1.tolist(), g_noisy.tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # 3.4 Non-overlapping (separated peaks)
    g_left = gaussian(50, 10.0, 2.0)
    g_right = gaussian(50, 40.0, 2.0)
    for cut_val in [0.3, 0.5]:
        trace_cases.append({
            "id": f"m2_orthogonal_cut_{cut_val}",
            "traces": [g_left.tolist(), g_right.tolist()],
            "thr": 0.05,
            "min_overlap": 4,
            "cut": cut_val,
        })

    # 3.5 Partial overlap: overlap < min_overlap
    t_a = np.zeros(30)
    t_b = np.zeros(30)
    t_a[10:13] = [0.8, 1.0, 0.8]  # length 3 active
    t_b[11:14] = [0.8, 1.0, 0.8]  # union of active is indices 10..13 (4 total)
    trace_cases.append({
        "id": "m2_overlap_less_than_min",
        "traces": [t_a.tolist(), t_b.tolist()],
        "thr": 0.05,
        "min_overlap": 5,  # cnt=4 < 5 -> NaN
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m2_overlap_exact_min",
        "traces": [t_a.tolist(), t_b.tolist()],
        "thr": 0.05,
        "min_overlap": 4,  # cnt=4 >= 4 -> finite
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m2_overlap_greater_min",
        "traces": [t_a.tolist(), t_b.tolist()],
        "thr": 0.05,
        "min_overlap": 3,  # cnt=4 > 3 -> finite
        "cut": 0.3,
    })

    # 3.6 Constant and zero signals
    trace_cases.append({
        "id": "m2_constant_ones",
        "traces": [np.ones(25).tolist(), np.ones(25).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m2_all_zeros",
        "traces": [np.zeros(25).tolist(), np.zeros(25).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m2_one_gaussian_one_zero",
        "traces": [g1.tolist(), np.zeros(len(g1)).tolist()],
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })
    # Spike on flat baseline
    t_spike1 = np.zeros(30)
    t_spike1[15] = 10.0
    t_spike2 = np.zeros(30)
    t_spike2[15] = 5.0
    trace_cases.append({
        "id": "m2_single_spike",
        "traces": [t_spike1.tolist(), t_spike2.tolist()],
        "thr": 0.05,
        "min_overlap": 4,  # cnt=1 < 4 -> NaN
        "cut": 0.3,
    })

    # 4. m = 5
    n5 = 60
    c1 = gaussian(n5, 18.0, 3.5)
    c2 = gaussian(n5, 42.0, 4.0)
    t5_list = [
        c1 + rng.normal(0, 0.01, n5),
        c1 * 0.7 + rng.normal(0, 0.01, n5),
        c1 * 0.3 + rng.normal(0, 0.01, n5),
        c2 + rng.normal(0, 0.01, n5),
        c2 * 0.5 + rng.normal(0, 0.01, n5),
    ]
    t5_arr = [np.clip(t, 0.0, None).tolist() for t in t5_list]
    for cut_val in [0.1, 0.3, 0.5, 0.8]:
        trace_cases.append({
            "id": f"m5_two_clusters_cut_{cut_val}",
            "traces": t5_arr,
            "thr": 0.05,
            "min_overlap": 4,
            "cut": cut_val,
        })

    # 5 orthogonal peaks
    t5_ortho = [gaussian(n5, 8.0 + 10.0 * i, 1.5).tolist() for i in range(5)]
    trace_cases.append({
        "id": "m5_orthogonal_cut_0.5",
        "traces": t5_ortho,
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.5,
    })

    # 5 identical peaks (tie-breaking test)
    t5_identical = [c1.tolist() for _ in range(5)]
    trace_cases.append({
        "id": "m5_identical_ties",
        "traces": t5_identical,
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # 5 random noise traces
    t5_noise = [rng.uniform(0.0, 1.0, size=40).tolist() for _ in range(5)]
    trace_cases.append({
        "id": "m5_noise_cut_0.3",
        "traces": t5_noise,
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # Variations in thr and min_overlap
    trace_cases.append({
        "id": "m5_thr_0.01",
        "traces": t5_arr,
        "thr": 0.01,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m5_thr_0.15",
        "traces": t5_arr,
        "thr": 0.15,
        "min_overlap": 4,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m5_min_overlap_2",
        "traces": t5_arr,
        "thr": 0.05,
        "min_overlap": 2,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m5_min_overlap_8",
        "traces": t5_arr,
        "thr": 0.05,
        "min_overlap": 8,
        "cut": 0.3,
    })

    # 5. m = 10
    n10 = 80
    p1 = gaussian(n10, 20.0, 3.0)
    p2 = gaussian(n10, 40.0, 4.0)
    p3 = gaussian(n10, 60.0, 3.5)
    t10_list = [
        p1 + rng.normal(0, 0.005, n10),
        p1 * 0.8 + rng.normal(0, 0.005, n10),
        p1 * 0.4 + rng.normal(0, 0.005, n10),
        p2 + rng.normal(0, 0.005, n10),
        p2 * 0.6 + rng.normal(0, 0.005, n10),
        p2 * 0.3 + rng.normal(0, 0.005, n10),
        p2 * 0.1 + rng.normal(0, 0.005, n10),
        p3 + rng.normal(0, 0.005, n10),
        p3 * 0.5 + rng.normal(0, 0.005, n10),
        p3 * 0.2 + rng.normal(0, 0.005, n10),
    ]
    t10_arr = [np.clip(t, 0.0, None).tolist() for t in t10_list]
    for cut_val in [0.1, 0.3, 0.5, 0.8]:
        trace_cases.append({
            "id": f"m10_three_clusters_cut_{cut_val}",
            "traces": t10_arr,
            "thr": 0.05,
            "min_overlap": 4,
            "cut": cut_val,
        })

    # m=10 with background noise channels
    t10_mixed = list(t10_arr[:8])
    t10_mixed.append(np.zeros(n10).tolist())
    t10_mixed.append(rng.uniform(0.0, 0.01, size=n10).tolist())
    trace_cases.append({
        "id": "m10_mixed_background",
        "traces": t10_mixed,
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.3,
    })

    # m=10 symmetric tie test
    sym_trace = gaussian(50, 25.0, 5.0)
    t10_sym = [sym_trace.tolist() for _ in range(10)]
    trace_cases.append({
        "id": "m10_symmetric_ties",
        "traces": t10_sym,
        "thr": 0.05,
        "min_overlap": 4,
        "cut": 0.2,
    })

    # 6. m = 20
    n20 = 120
    centers = [20.0, 45.0, 75.0, 100.0]
    widths = [3.0, 4.5, 3.5, 5.0]
    t20_list = []
    # 4 groups of 4 peaks each (16 peaks)
    for g_idx in range(4):
        base_peak = gaussian(n20, centers[g_idx], widths[g_idx])
        scales = [1.0, 0.65, 0.30, 0.12]
        for s in scales:
            noise_g = rng.normal(0, 0.008, n20)
            t20_list.append(np.clip(base_peak * s + noise_g, 0.0, None).tolist())
    # 4 background / odd channels
    t20_list.append(np.zeros(n20).tolist())
    t20_list.append(np.ones(n20).tolist())
    t20_list.append(gaussian(n20, 60.0, 1.0).tolist())  # sharp transient spike
    t20_list.append(rng.uniform(0.0, 0.05, size=n20).tolist())  # noise

    for cut_val in [0.1, 0.3, 0.5, 0.8]:
        trace_cases.append({
            "id": f"m20_complex_cut_{cut_val}",
            "traces": t20_list,
            "thr": 0.05,
            "min_overlap": 4,
            "cut": cut_val,
        })
    trace_cases.append({
        "id": "m20_complex_overlap_3",
        "traces": t20_list,
        "thr": 0.05,
        "min_overlap": 3,
        "cut": 0.3,
    })
    trace_cases.append({
        "id": "m20_complex_overlap_6",
        "traces": t20_list,
        "thr": 0.05,
        "min_overlap": 6,
        "cut": 0.3,
    })

    # Process all trace cases with Python functions
    for case in trace_cases:
        traces = case["traces"]
        thr = case["thr"]
        min_ov = case["min_overlap"]
        cut = case["cut"]
        m = len(traces)

        if m == 0:
            case["corr"] = []
            case["dist"] = []
            case["labels"] = []
            case["heights"] = []
            case["cluster_labels"] = []
            case["cluster_heights"] = []
            continue

        Y = np.asarray(traces, dtype=float)
        R = _pairwise_windowed_corr(Y, thr=thr, min_overlap=min_ov)
        D = np.where(np.isfinite(R), (1.0 - R) / 2.0, 1.0)
        np.fill_diagonal(D, 0.0)

        lbls, hts = _average_linkage(D, cut)
        case["corr"] = f_mat(R)
        case["dist"] = f_mat(D)
        case["labels"] = [int(x) for x in lbls]
        case["heights"] = [float(x) for x in hts]
        case["cluster_labels"] = [int(x) for x in lbls]
        case["cluster_heights"] = [float(x) for x in hts]

    # Standalone linkage cases on direct distance matrices
    # Case L0: m=0
    linkage_cases.append({
        "id": "linkage_m0",
        "dist": [],
        "cut": 0.3,
        "labels": [],
        "heights": [],
    })
    # Case L1: m=1
    linkage_cases.append({
        "id": "linkage_m1",
        "dist": [[0.0]],
        "cut": 0.3,
        "labels": [0],
        "heights": [],
    })
    # Case L2: m=2 below cut
    d2_close = [[0.0, 0.25], [0.25, 0.0]]
    lbl, ht = _average_linkage(np.array(d2_close), 0.3)
    linkage_cases.append({
        "id": "linkage_m2_close",
        "dist": d2_close,
        "cut": 0.3,
        "labels": [int(x) for x in lbl],
        "heights": [float(x) for x in ht],
    })
    # Case L3: m=2 above cut
    lbl, ht = _average_linkage(np.array(d2_close), 0.2)
    linkage_cases.append({
        "id": "linkage_m2_far",
        "dist": d2_close,
        "cut": 0.2,
        "labels": [int(x) for x in lbl],
        "heights": [float(x) for x in ht],
    })
    # Case L4: 3-way tie
    d3_tie = [[0.0, 0.2, 0.2], [0.2, 0.0, 0.2], [0.2, 0.2, 0.0]]
    lbl, ht = _average_linkage(np.array(d3_tie), 0.5)
    linkage_cases.append({
        "id": "linkage_m3_tie",
        "dist": d3_tie,
        "cut": 0.5,
        "labels": [int(x) for x in lbl],
        "heights": [float(x) for x in ht],
    })
    # Case L5: m=4 chain
    d4_chain = [
        [0.0, 0.1, 0.5, 0.8],
        [0.1, 0.0, 0.6, 0.9],
        [0.5, 0.6, 0.0, 0.15],
        [0.8, 0.9, 0.15, 0.0],
    ]
    for cut_val in [0.05, 0.12, 0.2, 0.7]:
        lbl, ht = _average_linkage(np.array(d4_chain), cut_val)
        linkage_cases.append({
            "id": f"linkage_m4_chain_cut_{cut_val}",
            "dist": d4_chain,
            "cut": cut_val,
            "labels": [int(x) for x in lbl],
            "heights": [float(x) for x in ht],
        })
    # Random symmetric distance matrices
    for seed, sz, c_val in [(42, 6, 0.4), (43, 8, 0.35), (44, 12, 0.5), (45, 15, 0.25)]:
        r_mat = rng.uniform(0.05, 0.95, size=(sz, sz))
        r_sym = (r_mat + r_mat.T) / 2.0
        np.fill_diagonal(r_sym, 0.0)
        lbl, ht = _average_linkage(r_sym, c_val)
        linkage_cases.append({
            "id": f"linkage_random_sz_{sz}_cut_{c_val}",
            "dist": f_mat(r_sym),
            "cut": c_val,
            "labels": [int(x) for x in lbl],
            "heights": [float(x) for x in ht],
        })

    return trace_cases, linkage_cases


def main() -> None:
    traces, linkage = make_cases()
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("tools/roi_cluster_riferimento.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "trace_cases": traces,
        "linkage_cases": linkage,
    }
    out_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"Generated {len(traces)} trace cases and {len(linkage)} linkage cases into {out_path}")


if __name__ == "__main__":
    main()
