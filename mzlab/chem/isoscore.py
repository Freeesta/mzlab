"""Isotope-pattern score and fragment constraint of a candidate formula (high resolution only).

Evidence, not a verdict. Two independent terms, both between 0 and 1, both shown with their parts:

* isotope score: the theoretical fine structure (every isotopologue with its exact mass) is fused at the resolving power of the instrument
  (same rule as ``HR.mergeRes`` in hr.js: neighbours closer than FWHM = m / R(m) become one peak, R(m) = R200 * sqrt(200 / m)) and compared with
  ALL the observed isotope peaks, in m/z and in intensity, with declared expected errors (sigma_ppm, sigma_int). The probabilistic idea of
  SIRIUS (Boecker et al. 2009): each peak is a Gaussian in mass and a log-normal in intensity; peaks the theory predicts but the spectrum lacks, and
  peaks the spectrum shows but the theory does not predict, both count against the formula.
* fragment constraint (BUDDY/msbuddy idea, Xing et al. 2023): the share of the MS2 intensity that sub-formulas of the candidate explain.

No learned model. The weights that combine the terms are the user's.
"""
from __future__ import annotations

import math

import numpy as np

from .elements import ELECTRON, MASS
from .subformulas import submasses
from ..i18n import UserError

# exact masses and natural abundances (fraction) of the isotopes: element -> [(mass, abundance)]
ISO = {
    "H": [(1.00782503223, 0.999885), (2.01410177812, 0.000115)],
    "C": [(12.0, 0.9893), (13.00335483507, 0.0107)],
    "N": [(14.00307400443, 0.99636), (15.00010889888, 0.00364)],
    "O": [(15.99491461957, 0.99757), (16.99913175650, 0.00038), (17.99915961286, 0.00205)],
    "F": [(18.99840316273, 1.0)],
    "Na": [(22.989769282, 1.0)],
    "Si": [(27.9769265350, 0.92223), (28.9764946653, 0.04685), (29.973770136, 0.03092)],
    "P": [(30.97376199842, 1.0)],
    "S": [(31.9720711744, 0.9499), (32.9714589098, 0.0075), (33.967867004, 0.0425), (35.96708071, 0.0001)],
    "Cl": [(34.968852682, 0.7576), (36.965902602, 0.2424)],
    "K": [(38.9637064864, 0.932581), (40.9618252579, 0.067302)],
    "Br": [(78.9183376, 0.5069), (80.9162897, 0.4931)],
    "I": [(126.9044719, 1.0)],
}
MERGE_TOL = 0.0005            # Da: isotopologues closer than this are one peak (as in hr.js)
WEIGHT_POW = 0.0              # a peak weighs rel ** WEIGHT_POW in the score (0: every peak the same, the monoisotopic one is fitted by construction; 1: its share of the theory)
MAX_SHIFT = 6                 # nominal mass units above the monoisotopic peak that are considered


def _join(items: list[tuple[float, float]]) -> list[tuple[float, float]]:
    items.sort()
    out: list[list[float]] = []
    for m, p in items:
        if out and m - out[-1][2] <= MERGE_TOL:
            out[-1][0] += m * p; out[-1][1] += p; out[-1][2] = m
        else:
            out.append([m * p, p, m])
    return [(s / p, p) for s, p, _ in out]


def _conv(a: list[tuple[float, float]], b: list[tuple[float, float]], floor: float, mono: float) -> list[tuple[float, float]]:
    r = [(ma + mb, pa * pb) for ma, pa in a for mb, pb in b if pa * pb > floor]
    r = _join(r)
    return [(m, p) for m, p in r if m - mono <= MAX_SHIFT + 0.5 - 0.0]


def fine_pattern(counts: dict[str, int], z: int = 1, min_rel: float = 0.001) -> list[dict]:
    """Fine isotope structure of an ion formula: [{mz, rel}] (rel: % of the most intense isotopologue), sorted by m/z."""
    dist = [(0.0, 1.0)]
    floor = 1e-12
    for el, k in counts.items():
        if k <= 0:
            continue
        if el not in ISO:
            raise UserError("err.iso.element", {"symbol": el}, f"no isotope data for {el}")
        base, res, c = [(m - ISO[el][0][0], p) for m, p in ISO[el]], [(0.0, 1.0)], int(k)
        while c:
            if c & 1:
                res = _conv(res, base, floor, 0.0)
            c >>= 1
            if c:
                base = _conv(base, base, floor, 0.0)
        dist = _conv(dist, res, floor, 0.0)
    mono = sum(ISO[el][0][0] * k for el, k in counts.items() if k > 0)
    top = max(p for _, p in dist)
    return [{"mz": (mono + m - z * ELECTRON) / abs(z), "rel": 100.0 * p / top} for m, p in dist if 100.0 * p / top >= min_rel]


def merge_res(pat: list[dict], r200: float) -> list[dict]:
    """Fuse the peaks that the instrument does not resolve (twin of ``HR.mergeRes``); r200 <= 0 keeps the fine structure."""
    if not (r200 > 0) or not pat:
        return pat
    out: list[list[float]] = []
    for r in pat:
        fw = r["mz"] / (r200 * math.sqrt(200.0 / r["mz"]))
        if out and r["mz"] - out[-1][2] <= fw:
            out[-1][0] += r["mz"] * r["rel"]; out[-1][1] += r["rel"]; out[-1][2] = r["mz"]
        else:
            out.append([r["mz"] * r["rel"], r["rel"], r["mz"]])
    top = max(c[1] for c in out)
    return [{"mz": s / p, "rel": 100.0 * p / top} for s, p, _ in out]


def isotope_score(counts: dict[str, int], z: int, obs: list[tuple[float, float]], *, r200: float = 70000.0, sigma_ppm: float = 3.0,
                  sigma_int: float = 0.10, min_rel: float = 1.0, noise_rel: float = 1.0, norm: str = "fit") -> dict:
    """Score of the isotope pattern of `counts` against the observed peaks `obs` = [(mz, intensity)] around the monoisotopic peak.

    sigma_ppm  expected mass error of a peak (ppm); sigma_int  expected relative error of an intensity (0.10 = 10 %, log-normal).
    min_rel    theoretical peaks below this % of the base peak are not required to be seen; noise_rel  observed peaks below this % are ignored.
    Returns {score, peaks: [{mz, rel, obs_mz, obs_rel, d_ppm, t_mass, t_int, t, w}], extra: [{mz, rel, w}], scale, n_missing, n_extra}. Each theoretical
    peak carries its weight w (share of the theory), the Gaussian mass term and the log-normal intensity term (their product t is its contribution).
    The observed intensities are scaled to the theory by the weighted least squares factor (norm="fit", the monoisotopic peak is not privileged) or, with
    norm "mono" / "max" / "sum", so that the monoisotopic peak / the theoretically most intense peak / the sum of the matched peaks agree with the theory.
    """
    th = [p for p in merge_res(fine_pattern(counts, z), r200) if p["rel"] >= min_rel]
    if not th or not obs:
        return {"score": 0.0, "peaks": [{**p, "obs_mz": None, "obs_rel": None, "d_ppm": None, "t_mass": 0.0, "t_int": 0.0, "t": 0.0, "w": 0.0} for p in th],
                "extra": [], "scale": None, "n_missing": len(th), "n_extra": 0}
    o = sorted(obs)
    omax = max(i for _, i in o)
    o = [(m, i) for m, i in o if i >= omax * noise_rel / 100.0]
    used: set[int] = set()
    match: list[tuple[int, float] | None] = []
    for p in th:                                                     # the most intense observed peak inside the window of 3 sigma + half the FWHM
        win = p["mz"] * max(3 * sigma_ppm, 1e-9) * 1e-6 + 0.0005
        best = None
        for j, (m, i) in enumerate(o):
            if j not in used and abs(m - p["mz"]) <= win and (best is None or i > o[best][1]):
                best = j
        if best is not None:
            used.add(best)
        match.append((best, 0.0) if best is not None else None)      # type: ignore[arg-type]
    num = sum(p["rel"] * o[mt[0]][1] for p, mt in zip(th, match) if mt)
    den = sum(p["rel"] ** 2 for p, mt in zip(th, match) if mt)
    scale = num / den if den > 0 else None                          # observed intensity per unit of theoretical %
    if norm in ("mono", "max", "sum"):                              # another normalisation mode: when its anchor is not matched the fit stays
        mt_ = [(p, mt) for p, mt in zip(th, match) if mt]
        if norm == "sum" and mt_:
            scale = sum(o[mt[0]][1] for _, mt in mt_) / sum(p["rel"] for p, _ in mt_)
        else:
            a = th[0] if norm == "mono" else max(th, key=lambda p: p["rel"])
            mt = match[th.index(a)]
            if mt:
                scale = o[mt[0]][1] / a["rel"]
    wt = lambda r: r ** WEIGHT_POW
    tot_th = sum(wt(p["rel"]) for p in th)
    extra = []
    if scale:
        for j, (m, i) in enumerate(o):
            r = i / scale
            if j not in used and r >= min_rel and 0.0 <= (m - th[0]["mz"]) * abs(z) <= MAX_SHIFT + 0.5 and m >= th[0]["mz"] - 0.3 / abs(z):
                extra.append({"mz": m, "rel": r})
    tot_w = tot_th + sum(wt(e["rel"]) for e in extra)
    peaks, s = [], 0.0
    for p, mt in zip(th, match):
        w = wt(p["rel"]) / tot_w if tot_w else 0.0
        row = {"mz": p["mz"], "rel": p["rel"], "obs_mz": None, "obs_rel": None, "d_ppm": None, "t_mass": 0.0, "t_int": 0.0, "t": 0.0, "w": w}
        if mt and scale:
            m, i = o[mt[0]]
            d = (m - p["mz"]) / p["mz"] * 1e6
            r = i / scale
            tm = math.exp(-0.5 * (d / sigma_ppm) ** 2)
            ti = math.exp(-0.5 * (math.log(r / p["rel"]) / sigma_int) ** 2) if r > 0 else 0.0
            row.update({"obs_mz": m, "obs_rel": r, "d_ppm": d, "t_mass": tm, "t_int": ti, "t": tm * ti})
        s += w * row["t"]
        peaks.append(row)
    for e in extra:
        e["w"] = wt(e["rel"]) / tot_w
    return {"score": s, "peaks": peaks, "extra": extra, "scale": scale, "n_missing": sum(1 for r in peaks if r["obs_mz"] is None), "n_extra": len(extra)}


def fragment_constraint(formula: str, z: int, frags: list[tuple[float, float]], *, ppm: float = 5.0, min_mz: float = 50.0) -> dict:
    """Share of the MS2 intensity explained by sub-formulas of the ion `formula` (z: charge of the ion), and the number of explained peaks.

    frags = [(mz, intensity)] of the MS2 of the precursor. Peaks above the precursor are ignored (it, and its isotopes, are not fragments)."""
    from .elements import parse_formula
    f = parse_formula(formula)
    try:
        sm = submasses(formula, z, min_mz)
    except UserError:
        return {"explained": None, "n_explained": None, "n_peaks": None}
    mono = (sum(MASS[e] * k for e, k in f.items()) - z * ELECTRON) / abs(z)
    peaks = [(m, i) for m, i in frags if min_mz <= m <= mono + 0.5 and i > 0]
    if not peaks:
        return {"explained": None, "n_explained": 0, "n_peaks": 0}
    tot, hit, n = 0.0, 0.0, 0
    for m, i in peaks:
        tot += i
        if len(sm):
            k = np.searchsorted(sm, m)
            near = sm[max(0, k - 1):k + 1]
            if len(near) and np.min(np.abs(near - m)) <= m * ppm * 1e-6:
                hit += i; n += 1
    return {"explained": hit / tot if tot else None, "n_explained": n, "n_peaks": len(peaks)}


def combined(m_term: float, iso: float | None, frag: float | None, w: tuple[float, float, float]) -> float:
    """Weighted mean of the available terms (a missing term does not count): the user's weights, not a verdict."""
    parts = [(w[0], m_term)] + ([(w[1], iso)] if iso is not None else []) + ([(w[2], frag)] if frag is not None else [])
    ws = sum(a for a, _ in parts)
    return sum(a * b for a, b in parts) / ws if ws > 0 else 0.0
