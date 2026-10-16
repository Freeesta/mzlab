"""Deterministic reference answers of `TP_Mine.py.tpmine.hr.spectra.deconvolve_isomers`
for `crates/mzlab-feat/tests/deconvolve_parity.rs`.

Usage:
    python3 tools/deconvolve_riferimento.py [OUT.json]

Generates >= 30 synthetic cases:
- cases < min_scans or fragments < 3
- single pure component with/without XIC and noise
- mixtures of 2 and 3 partially/strongly resolved isomers with XIC
- edge cases (unsorted RTs, custom parameters, jitter/noise)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from TP_Mine.py.tpmine.hr.spectra import deconvolve_isomers  # noqa: E402


def serialize_result(res: dict) -> dict:
    comps = []
    for c in res["components"]:
        mz, rel = c["spectrum"]
        comps.append({
            "rt_apex": float(c["rt_apex"]) if c["rt_apex"] is not None else None,
            "spectrum": {
                "mz": [float(x) for x in mz],
                "rel": [float(x) for x in rel],
            },
            "profile": [float(x) for x in c["profile"]] if c["profile"] is not None else None,
            "xic_corr": float(c["xic_corr"]) if c["xic_corr"] is not None else None,
            "n_scans_main": int(c["n_scans_main"]) if c.get("n_scans_main") is not None else None,
        })
    return {
        "k": int(res["k"]),
        "components": comps,
        "scans": int(res["scans"]),
        "fragments": int(res["fragments"]),
        "lof": float(res["lof"]) if "lof" in res and res["lof"] is not None else None,
    }


def make_mixture(n_scans, isomer_apices, isomer_widths, isomer_heights, isomer_spectra, rng, noise_lvl=0.0, jitter=False):
    rts = np.linspace(1.0, 2.0, n_scans)
    if jitter:
        rts = np.clip(rts + rng.normal(0, 0.003, n_scans), 0.9, 2.1)
    mzs_list, ints_list = [], []
    n_comps = len(isomer_apices)
    all_frag_mzs = sorted({mz for spec in isomer_spectra for mz in spec})

    for t in rts:
        scan_mzs, scan_ints = [], []
        for mz in all_frag_mzs:
            intensity = sum(
                np.exp(-((t - isomer_apices[c]) / isomer_widths[c]) ** 2) * isomer_heights[c] * isomer_spectra[c].get(mz, 0.0)
                for c in range(n_comps)
            )
            if noise_lvl > 0:
                intensity += rng.normal(0, noise_lvl)
            if intensity > 0.01:
                scan_mzs.append(float(mz))
                scan_ints.append(float(intensity))
        mzs_list.append(scan_mzs)
        ints_list.append(scan_ints)

    xr = np.linspace(0.9, 2.1, 100).tolist()
    xy = [
        float(sum(np.exp(-((t - isomer_apices[c]) / isomer_widths[c]) ** 2) * isomer_heights[c] * 100.0 for c in range(n_comps)))
        for t in xr
    ]
    return rts.tolist(), mzs_list, ints_list, xr, xy


def cases_edge_and_single() -> list[dict]:
    p_def = {"min_scans": 4, "max_components": 3, "ppm": 8.0, "min_frac_sig": 0.02, "subpeak_tol": 0.07}
    rts_s = [1.0, 1.1, 1.2, 1.3, 1.4, 1.5]
    mzs_s = [[100.0, 150.0, 200.0, 250.0]] * 6
    ints_s = [[50.0 * np.exp(-((t - 1.25) / 0.12) ** 2) * f for f in [1.0, 2.0, 0.6, 1.6]] for t in rts_s]
    xr1 = np.linspace(0.8, 1.7, 50).tolist()
    xy1 = [float(np.exp(-((t - 1.25) / 0.12) ** 2) * 500.0) for t in xr1]

    return [
        {"name": "empty", "mzs": [], "ints": [], "rts": [], "xic": None, "params": p_def},
        {"name": "one_scan", "mzs": [[100.0, 200.0]], "ints": [[10.0, 20.0]], "rts": [1.0], "xic": None, "params": p_def},
        {"name": "two_scans", "mzs": [[100.0, 200.0], [100.0, 200.0]], "ints": [[10.0, 20.0], [15.0, 25.0]], "rts": [1.0, 1.1], "xic": None, "params": p_def},
        {"name": "three_scans", "mzs": [[100.0, 200.0]] * 3, "ints": [[10.0, 20.0], [20.0, 30.0], [12.0, 22.0]], "rts": [1.0, 1.1, 1.2], "xic": None, "params": p_def},
        {"name": "scans_all_empty_peaks", "mzs": [[]] * 5, "ints": [[]] * 5, "rts": [1.0, 1.1, 1.2, 1.3, 1.4], "xic": None, "params": p_def},
        {"name": "one_fragment", "mzs": [[150.0], [150.0001], [150.0], [150.0002], [150.0]], "ints": [[10.0], [20.0], [30.0], [25.0], [15.0]], "rts": [1.0, 1.1, 1.2, 1.3, 1.4], "xic": None, "params": p_def},
        {"name": "two_fragments", "mzs": [[150.0, 250.0]] * 5, "ints": [[10.0, 20.0], [20.0, 40.0], [30.0, 60.0], [25.0, 50.0], [15.0, 30.0]], "rts": [1.0, 1.1, 1.2, 1.3, 1.4], "xic": None, "params": p_def},
        {"name": "single_pure_no_xic", "mzs": mzs_s, "ints": ints_s, "rts": rts_s, "xic": None, "params": p_def},
        {"name": "single_pure_with_xic", "mzs": mzs_s, "ints": ints_s, "rts": rts_s, "xic": {"rt": xr1, "y": xy1}, "params": p_def},
    ]


def cases_two_isomers(rng) -> list[dict]:
    p_def = {"min_scans": 4, "max_components": 3, "ppm": 8.0, "min_frac_sig": 0.02, "subpeak_tol": 0.07}
    sp_a = {100.0: 1.0, 150.0: 0.8, 200.0: 0.2, 300.0: 0.0}
    sp_b = {100.0: 0.1, 150.0: 0.3, 200.0: 0.9, 300.0: 1.0}
    r0, m0, i0, x0r, x0y = make_mixture(14, [1.3, 1.7], [0.09, 0.09], [1.0, 1.0], [sp_a, sp_b], rng)
    r1, m1, i1, x1r, x1y = make_mixture(15, [1.4, 1.6], [0.08, 0.08], [1.0, 1.0], [sp_a, sp_b], rng)
    r2, m2, i2, x2r, x2y = make_mixture(15, [1.44, 1.56], [0.07, 0.07], [1.0, 0.9], [sp_a, sp_b], rng)
    r3, m3, i3, x3r, x3y = make_mixture(16, [1.35, 1.65], [0.08, 0.08], [0.33, 1.0], [sp_a, sp_b], rng)
    r4, m4, i4, x4r, x4y = make_mixture(16, [1.35, 1.65], [0.08, 0.08], [1.0, 0.33], [sp_a, sp_b], rng)
    r5, m5, i5, x5r, x5y = make_mixture(15, [1.35, 1.65], [0.08, 0.08], [1.0, 1.0], [sp_a, sp_b], rng, noise_lvl=0.01)
    r6, m6, i6, x6r, x6y = make_mixture(15, [1.35, 1.65], [0.08, 0.08], [1.0, 1.0], [sp_a, sp_b], rng, jitter=True)

    return [
        {"name": "two_isomers_clean_no_xic", "mzs": m0, "ints": i0, "rts": r0, "xic": None, "params": p_def},
        {"name": "two_isomers_clean_with_xic", "mzs": m0, "ints": i0, "rts": r0, "xic": {"rt": x0r, "y": x0y}, "params": p_def},
        {"name": "two_isomers_partial_overlap_xic", "mzs": m1, "ints": i1, "rts": r1, "xic": {"rt": x1r, "y": x1y}, "params": p_def},
        {"name": "two_isomers_strong_overlap_xic", "mzs": m2, "ints": i2, "rts": r2, "xic": {"rt": x2r, "y": x2y}, "params": p_def},
        {"name": "two_isomers_unequal_heights_1_3", "mzs": m3, "ints": i3, "rts": r3, "xic": {"rt": x3r, "y": x3y}, "params": p_def},
        {"name": "two_isomers_unequal_heights_3_1", "mzs": m4, "ints": i4, "rts": r4, "xic": {"rt": x4r, "y": x4y}, "params": p_def},
        {"name": "two_isomers_with_noise", "mzs": m5, "ints": i5, "rts": r5, "xic": {"rt": x5r, "y": x5y}, "params": p_def},
        {"name": "two_isomers_with_jitter", "mzs": m6, "ints": i6, "rts": r6, "xic": {"rt": x6r, "y": x6y}, "params": p_def},
    ]


def cases_three_isomers(rng) -> list[dict]:
    p_def = {"min_scans": 4, "max_components": 3, "ppm": 8.0, "min_frac_sig": 0.02, "subpeak_tol": 0.07}
    sp_a = {105.0: 1.0, 160.0: 0.5, 210.0: 0.1, 260.0: 0.0, 310.0: 0.0}
    sp_b = {105.0: 0.1, 160.0: 1.0, 210.0: 0.8, 260.0: 0.2, 310.0: 0.0}
    sp_c = {105.0: 0.0, 160.0: 0.2, 210.0: 0.5, 260.0: 1.0, 310.0: 0.9}
    r0, m0, i0, x0r, x0y = make_mixture(21, [1.25, 1.5, 1.75], [0.07, 0.07, 0.07], [1.0, 0.9, 0.8], [sp_a, sp_b, sp_c], rng)
    r1, m1, i1, x1r, x1y = make_mixture(21, [1.25, 1.5, 1.75], [0.07, 0.07, 0.07], [1.0, 0.5, 1.2], [sp_a, sp_b, sp_c], rng)
    r2, m2, i2, x2r, x2y = make_mixture(21, [1.32, 1.5, 1.68], [0.065, 0.065, 0.065], [1.0, 0.9, 0.8], [sp_a, sp_b, sp_c], rng)
    r3, m3, i3, x3r, x3y = make_mixture(21, [1.25, 1.5, 1.75], [0.07, 0.07, 0.07], [1.0, 0.9, 0.8], [sp_a, sp_b, sp_c], rng, noise_lvl=0.015)

    return [
        {"name": "three_isomers_separated_xic", "mzs": m0, "ints": i0, "rts": r0, "xic": {"rt": x0r, "y": x0y}, "params": p_def},
        {"name": "three_isomers_separated_no_xic", "mzs": m0, "ints": i0, "rts": r0, "xic": None, "params": p_def},
        {"name": "three_isomers_unequal_xic", "mzs": m1, "ints": i1, "rts": r1, "xic": {"rt": x1r, "y": x1y}, "params": p_def},
        {"name": "three_isomers_closer_overlap_xic", "mzs": m2, "ints": i2, "rts": r2, "xic": {"rt": x2r, "y": x2y}, "params": p_def},
        {"name": "three_isomers_noise_xic", "mzs": m3, "ints": i3, "rts": r3, "xic": {"rt": x3r, "y": x3y}, "params": p_def},
        {"name": "three_isomers_max_k_2", "mzs": m0, "ints": i0, "rts": r0, "xic": {"rt": x0r, "y": x0y}, "params": {**p_def, "max_components": 2}},
    ]


def cases_param_variations(rng) -> list[dict]:
    p_def = {"min_scans": 4, "max_components": 3, "ppm": 8.0, "min_frac_sig": 0.02, "subpeak_tol": 0.07}
    sp_a = {100.0: 1.0, 150.0: 0.8, 200.0: 0.2, 300.0: 0.0}
    sp_b = {100.0: 0.1, 150.0: 0.3, 200.0: 0.9, 300.0: 1.0}
    r, m, it, xr, xy = make_mixture(14, [1.3, 1.7], [0.09, 0.09], [1.0, 1.0], [sp_a, sp_b], rng)
    perm = rng.permutation(len(r))
    r_uns, m_uns, it_uns = [r[i] for i in perm], [m[i] for i in perm], [it[i] for i in perm]

    r_mis, m_mis, it_mis, xrm, xym = make_mixture(14, [1.3, 1.7], [0.09, 0.09], [1.0, 1.0], [sp_a, sp_b], rng)
    for sc in range(len(m_mis)):
        if len(m_mis[sc]) > 2 and rng.random() < 0.4:
            m_mis[sc], it_mis[sc] = m_mis[sc][:2], it_mis[sc][:2]

    mz_z = [[100.0, 150.0, 200.0]] * 6
    it_z = [[10.0, 0.0, 20.0], [0.0, 30.0, 40.0], [15.0, 25.0, 0.0], [0.0, 0.0, 50.0], [20.0, 10.0, 10.0], [30.0, 20.0, 15.0]]

    return [
        {"name": "two_isomers_unsorted_rts", "mzs": m_uns, "ints": it_uns, "rts": r_uns, "xic": {"rt": xr, "y": xy}, "params": p_def},
        {"name": "xic_short_points", "mzs": m, "ints": it, "rts": r, "xic": {"rt": [1.0, 1.5, 2.0], "y": [10.0, 50.0, 10.0]}, "params": p_def},
        {"name": "xic_distant_subpeaks", "mzs": [[100.0, 200.0]] * 4, "ints": [[10.0, 20.0]] * 4, "rts": [1.0, 1.1, 1.2, 1.3], "xic": {"rt": [0.1, 0.2, 0.3], "y": [10.0, 50.0, 10.0]}, "params": {**p_def, "subpeak_tol": 0.05}},
        {"name": "custom_ppm_15", "mzs": m, "ints": it, "rts": r, "xic": {"rt": xr, "y": xy}, "params": {**p_def, "ppm": 15.0}},
        {"name": "custom_min_scans_8", "mzs": m, "ints": it, "rts": r, "xic": {"rt": xr, "y": xy}, "params": {**p_def, "min_scans": 8}},
        {"name": "custom_min_frac_sig_0_10", "mzs": m, "ints": it, "rts": r, "xic": {"rt": xr, "y": xy}, "params": {**p_def, "min_frac_sig": 0.10}},
        {"name": "custom_subpeak_tol_0_03", "mzs": m, "ints": it, "rts": r, "xic": {"rt": xr, "y": xy}, "params": {**p_def, "subpeak_tol": 0.03}},
        {"name": "zeros_interspersed", "mzs": mz_z, "ints": it_z, "rts": [1.0, 1.1, 1.2, 1.3, 1.4, 1.5], "xic": None, "params": p_def},
        {"name": "two_isomers_missing_peaks", "mzs": m_mis, "ints": it_mis, "rts": r_mis, "xic": {"rt": xrm, "y": xym}, "params": p_def},
    ]


def evaluate_cases(cases: list[dict]) -> None:
    for c in cases:
        p = c["params"]
        xic_tuple = (np.array(c["xic"]["rt"]), np.array(c["xic"]["y"])) if c["xic"] is not None else None
        res_py = deconvolve_isomers(
            c["mzs"],
            c["ints"],
            c["rts"],
            xic=xic_tuple,
            min_scans=p["min_scans"],
            max_components=p["max_components"],
            ppm=p["ppm"],
            min_frac_sig=p["min_frac_sig"],
            subpeak_tol=p["subpeak_tol"],
        )
        c["expected"] = serialize_result(res_py)


def make_cases() -> list[dict]:
    rng = np.random.default_rng(20261016)
    cases = []
    cases.extend(cases_edge_and_single())
    cases.extend(cases_two_isomers(rng))
    cases.extend(cases_three_isomers(rng))
    cases.extend(cases_param_variations(rng))
    evaluate_cases(cases)
    return cases


def main(argv: list[str]) -> int:
    cases = make_cases()
    payload = json.dumps(cases, indent=2)
    if argv and argv[0] != "-":
        Path(argv[0]).write_text(payload, encoding="utf-8")
    else:
        sys.stdout.write(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
