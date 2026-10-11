"""Elemental composition of an ion from its exact m/z: the formulas that fit, within a tolerance, with the limits of the student's choice.

Plain Python + numpy. The result is a list of CANDIDATES (formula, error, RDB, rules passed, isotope check): never a verdict.

How it works (vectorised, no loop over the formulas): the grid of the elements other than C and H is built once (``itertools.product`` of
their ranges, as an array); for every combination and every number of carbons the number of hydrogens that closes the mass is solved
(rounded value +- 1) and the mass error, the ranges, the RDB, the nitrogen rule, the "Seven Golden Rules" and the optional sub-formula
condition are applied to arrays.

The ranges refer to the formula of the ION as it is written in the results (C13H25N4O3S for the [M+H]+ of C13H24N4O3S): this is how the
elements in use of the Xcalibur Qual Browser are read. The neutral formula is given beside it.
"""
from __future__ import annotations

import math
import re

import numpy as np

from .elements import ELECTRON, MASS, parse_formula
from ..i18n import UserError

# monoisotopic masses of the isotopes that can be added as separate elements (labelled molecules, isotopologues)
ISOTOPES = {
    "2H": (2.01410177812, "H"), "13C": (13.00335483507, "C"), "15N": (15.00010889888, "N"), "18O": (17.99915961286, "O"),
    "34S": (33.96786701, "S"), "37Cl": (36.96590260, "Cl"), "81Br": (80.9162897, "Br"),
}
# contribution of an atom to the ring and double bond equivalents: (valence - 2) / 2 (S can be changed: valence 4 or 6)
DBEQ = {"C": 1.0, "H": -0.5, "N": 0.5, "O": 0.0, "S": 0.0, "P": 0.5, "F": -0.5, "Cl": -0.5, "Br": -0.5, "I": -0.5, "Si": 1.0, "Na": -0.5, "K": -0.5}
VALENCE = {"C": 4, "H": 1, "N": 3, "O": 2, "S": 2, "P": 3, "F": 1, "Cl": 1, "Br": 1, "I": 1, "Si": 4, "Na": 1, "K": 1}
DEFAULT_ELEMENTS = {"C": (0, 100), "H": (0, 200), "N": (0, 10), "O": (0, 15), "S": (0, 3), "P": (0, 3), "F": (0, 6), "Cl": (0, 3), "Br": (0, 2)}
# adducts: name -> (atoms added to the neutral M, charge). "ion as it is": the formula IS the ion (M+•, an even-electron cation, an anion)
ADDUCT_ATOMS = {
    "[M+H]+": ({"H": 1}, 1), "[M+NH4]+": ({"N": 1, "H": 4}, 1), "[M+Na]+": ({"Na": 1}, 1), "[M+K]+": ({"K": 1}, 1),
    "[M-H]-": ({"H": -1}, -1), "[M+Cl]-": ({"Cl": 1}, -1), "[M+HCOO]-": ({"C": 1, "H": 1, "O": 2}, -1),
    "M+.": ({}, 1), "M-.": ({}, -1), "[M]+": ({}, 1), "[M]-": ({}, -1),
}
ODD_ELECTRON = {"M+.", "M-."}
# Kind & Fiehn 2007: limits of the element ratios to C (the "Seven Golden Rules"), H/C 0.2-3.1 included
RATIO_RULES = {"HC": ("H", 0.2, 3.1), "NC": ("N", 0.0, 1.3), "OC": ("O", 0.0, 1.2), "PC": ("P", 0.0, 0.3), "SC": ("S", 0.0, 0.8),
               "FC": ("F", 0.0, 1.5), "ClC": ("Cl", 0.0, 0.8), "BrC": ("Br", 0.0, 0.8)}
RULES = list(RATIO_RULES) + ["LEWIS", "SENIOR"]
ATOMIC_NUMBER = {"H": 1, "C": 6, "N": 7, "O": 8, "F": 9, "Na": 11, "Si": 14, "P": 15, "S": 16, "Cl": 17, "K": 19, "Br": 35, "I": 53, "Se": 34}
MAX_GRID = 3_000_000                                # combinations of the elements other than C and H


def parse_elements(text: str) -> dict[str, tuple[int, int]]:
    """'C:0-13,H:0-25,N:0-4,13C:0-2' -> {'C': (0, 13), ...}. A single number means that exact count ('Na:1')."""
    out: dict[str, tuple[int, int]] = {}
    for part in re.split(r"[,;]", text or ""):
        part = part.strip()
        if not part:
            continue
        m = re.fullmatch(r"(\d*[A-Z][a-z]?)\s*:\s*(\d+)(?:\s*-\s*(\d+))?", part)
        if not m:
            raise UserError("err.comp.element", {"part": part}, f"cannot read the element «{part}» (write C:0-13)")
        sym = m.group(1)
        lo = int(m.group(2)); hi = int(m.group(3)) if m.group(3) else lo
        if sym not in MASS and sym not in ISOTOPES:
            raise UserError("err.comp.unknown", {"symbol": sym}, f"unknown element {sym!r}")
        if hi < lo:
            raise UserError("err.comp.range", {"symbol": sym}, f"{sym}: the maximum is below the minimum")
        out[sym] = (lo, hi)
    return out


def _mass(sym: str) -> float:
    return ISOTOPES[sym][0] if sym in ISOTOPES else MASS[sym]


def _base(sym: str) -> str:
    return ISOTOPES[sym][1] if sym in ISOTOPES else sym


def format_formula(counts: dict[str, int]) -> str:
    """Hill order (C, H, then alphabetical); isotopes written as 13C2 after the element they replace."""
    items = {k: v for k, v in counts.items() if v}
    order = []
    if "C" in items:
        order.append("C")
    if "H" in items:
        order.append("H")
    plain = sorted(k for k in items if k not in ("C", "H") and k not in ISOTOPES)
    iso = sorted(k for k in items if k in ISOTOPES)
    order += plain + iso
    return "".join(k + (str(items[k]) if items[k] != 1 else "") for k in order)


def ion_mass(counts: dict[str, int], z: int = 1) -> float:
    """m/z of an ion of the given formula and charge (z > 0: electrons removed; z < 0: added)."""
    return (sum(_mass(k) * v for k, v in counts.items()) - z * ELECTRON) / abs(z)


def isotope_ratios(counts: dict[str, int]) -> tuple[float, float]:
    """(M+1 / M, M+2 / M) of an ion formula at the natural abundances (the isotopologues of the heaviest nominal mass are summed)."""
    from ..ionfamily import _ISO
    pat = np.array([1.0])
    for el, k in counts.items():
        if k <= 0 or el in ISOTOPES:
            continue
        iso = _ISO.get(el)
        if iso is None:
            continue
        single = np.zeros(max(s for s, _ in iso) + 1)
        for s, a in iso:
            single[s] += a
        single /= single.sum()
        acc, base, e = np.array([1.0]), single, int(k)
        while e:
            if e & 1:
                acc = np.convolve(acc, base)[:5]
            base = np.convolve(base, base)[:5]
            e >>= 1
        pat = np.convolve(pat, acc)[:5]
    pat = np.concatenate([pat, np.zeros(5)])
    return float(pat[1] / pat[0]), float(pat[2] / pat[0])


def compose(mz: float, *, elements: dict[str, tuple[int, int]] | None = None, ion: str = "[M+H]+", z: int | None = None,
            tol: float = 5.0, unit: str = "ppm", rdb: tuple[float, float] = (-1.0, 100.0), nitrogen: str = "none",
            rules: list[str] | None = None, max_results: int = 10, parent: str | dict | None = None, dbeq: dict[str, float] | None = None,
            m1: float | None = None, m2: float | None = None, iso_tol: float = 0.20, obs: list[tuple[float, float]] | None = None,
            frags: list[tuple[float, float]] | None = None, r200: float = 70000.0, sigma_ppm: float = 3.0, sigma_int: float = 0.15,
            weights: tuple[float, float, float] = (1.0, 1.0, 1.0), pool: int = 100) -> dict:
    """Formulas of the ion at `mz`.

    elements   {symbol: (min, max)} of the ION formula (default: C, H, N, O, S, P, F, Cl, Br up to sensible limits); isotopes (13C, 15N, 34S, 37Cl, 81Br,
               18O, 2H) are separate elements. ion: adduct name of ADDUCT_ATOMS ("M+." = radical cation, "[M]+" = even-electron cation as it is...).
    tol, unit  mass tolerance, "ppm" or "mda" (on the m/z).
    rdb        range of the ring and double bond equivalents of the NEUTRAL formula (of the ion for the "as it is" types).
    nitrogen   "none" | "even" (even-electron ions only) | "odd" (odd-electron ions only): parity of the RDB.
    rules      names of RULES that must hold (the Seven Golden Rules: HC NC OC PC SC FC ClC BrC, LEWIS, SENIOR); None / [] = report them, filter nothing.
    parent     formula of the precursor ion: only sub-formulas of it (for the product ions).
    m1, m2     observed M+1/M and M+2/M of the isotope peaks, if known: the isotope check compares them with the formula (relative tolerance iso_tol).
    obs        observed isotope peaks [(mz, intensity)] around the monoisotopic peak: every candidate gets the full isotope score (isoscore.isotope_score:
               theoretical fine structure fused at r200, compared with ALL the peaks; sigma_ppm and sigma_int are the declared expected errors).
    frags      MS2 peaks [(mz, intensity)] of the precursor: every candidate gets the share of the intensity explained by its sub-formulas and the
               number of peaks explained (fragment constraint). Neither needs a learned model.
    weights    (mass, isotopes, fragments) weights of the combined score the candidates are sorted by when `obs` or `frags` is given (the best `pool`
               by mass error are scored, then sorted); without them the order is the mass error. The terms are always listed, never a verdict.
    Returns {mz, charge, tol, unit, n_candidates, results: [...]}; each result: formula (ion), neutral, mz_calc, delta_mda, delta_ppm, rdb, rules_ok,
    rules_failed, iso {m1, m2, err1, err2, ok, score, peaks, extra, ...}, frag {explained, n_explained, n_peaks}, score {mass, iso, frag, total}.
    """
    if not (mz > 0):
        raise UserError("err.comp.mz", text="m/z must be positive")
    if ion not in ADDUCT_ATOMS:
        raise UserError("err.comp.ion", {"ion": ion}, f"unknown ion type {ion!r}")
    add_atoms, ch = ADDUCT_ATOMS[ion]
    z = abs(z) * (1 if ch > 0 else -1) if z else ch
    if (z > 0) != (ch > 0):
        raise UserError("err.comp.charge", text="the charge does not agree with the ion type")
    if unit not in ("ppm", "mda"):
        raise UserError("err.comp.unit", text="the unit of the tolerance is ppm or mDa")
    db = {**DBEQ, **(dbeq or {})}
    els = {**{k: v for k, v in DEFAULT_ELEMENTS.items()}} if not elements else dict(elements)
    for sym, n in add_atoms.items():                               # the atoms of the adduct are in the ion (Na, K, Cl, ...: at least that many)
        if n > 0:
            lo, hi = els.get(sym, (0, 0)); els[sym] = (max(lo, n), max(hi, n))
    par = parse_formula(parent) if isinstance(parent, str) and parent else (dict(parent) if parent else None)
    T = mz * abs(z) + z * ELECTRON                                 # mass of the atoms of the ion
    tol_da = (mz * tol * 1e-6 if unit == "ppm" else tol / 1000.0) * abs(z)
    if par:
        for sym in els:
            hi = par.get(sym, 0)
            els[sym] = (min(els[sym][0], hi), min(els[sym][1], hi))
        els = {s: v for s, v in els.items() if v[1] >= v[0]}
    cmin, cmax = els.get("C", (0, int(T // 12)))
    hmin, hmax = els.get("H", (0, int(T // MASS["H"]) + 1))
    others = [s for s in els if s not in ("C", "H")]
    ranges = [range(els[s][0], els[s][1] + 1) for s in others]
    size = int(np.prod([len(r) for r in ranges])) if ranges else 1
    if size > MAX_GRID:
        raise UserError("err.comp.many", text="too many combinations: narrow the ranges of the elements")
    grid = (np.stack(np.meshgrid(*[np.arange(r.start, r.stop, dtype=np.int32) for r in ranges], indexing="ij"), -1).reshape(size, len(others))
            if others else np.zeros((1, 0), dtype=np.int32))
    mo = np.array([_mass(s) for s in others], float)
    rest = T - grid @ mo if len(others) else np.full(1, T)
    keep = (rest >= -tol_da) & (rest <= MASS["H"] * hmax + 12.0 * cmax + tol_da)
    grid, rest = grid[keep], rest[keep]
    mH = MASS["H"]
    G, Cc, Hh, E = [], [], [], []                                   # grid row, carbons, hydrogens, mass error (Da) of every hit
    CH = 4096
    for s0 in range(0, len(rest), CH):
        r = rest[s0:s0 + CH]
        # carbons that can still close the mass with hmin..hmax hydrogens: a window of about hmax*1.008/12 values per combination, not all of cmin..cmax
        clo = np.maximum(cmin, np.ceil((r - mH * hmax - tol_da) / 12.0)).astype(np.int64)
        chi = np.minimum(cmax, np.floor((r + tol_da - mH * hmin) / 12.0)).astype(np.int64)
        W = int((chi - clo).max()) + 1 if len(r) else 0
        if W <= 0:
            continue
        cm = clo[:, None] + np.arange(W)[None, :]
        inwin = cm <= chi[:, None]
        h0 = (r[:, None] - 12.0 * cm) / mH
        for d in (-1, 0, 1):
            h = np.rint(h0).astype(np.int64) + d
            err = r[:, None] - 12.0 * cm - h * mH
            gi, ci = np.nonzero(inwin & (np.abs(err) <= tol_da) & (h >= hmin) & (h <= hmax))
            if len(gi):
                G.append(gi + s0); Cc.append(cm[gi, ci]); Hh.append(h[gi, ci]); E.append(err[gi, ci])
    empty = {"mz": mz, "charge": z, "tol": tol, "unit": unit, "ion": ion, "n_candidates": 0, "results": []}
    if not G:
        return empty
    G, Cc, Hh, E = np.concatenate(G), np.concatenate(Cc), np.concatenate(Hh), np.concatenate(E)
    syms = ["C", "H"] + others
    X = np.column_stack([Cc, Hh, grid[G]]).astype(np.int64)         # one row per hit: the counts of every element
    sel = X.sum(axis=1) > 0
    base = [_base(k) for k in syms]
    bases = sorted(set(base))
    B = np.column_stack([X[:, [i for i, b in enumerate(base) if b == bb]].sum(axis=1) for bb in bases])      # counts by element (isotopes folded in)
    if par:                                                          # sub-formula of the precursor: every element, isotopes counted with their element
        pv = np.array([par.get(b, 0) for b in bases])
        sel &= (B <= pv[None, :]).all(axis=1)
    A = np.array([add_atoms.get(k, 0) for k in syms], dtype=np.int64)
    Nn = X - A[None, :]                                              # the neutral (the ion itself for the "as it is" types)
    sel &= (Nn >= 0).all(axis=1)
    Bn = np.column_stack([Nn[:, [i for i, b in enumerate(base) if b == bb]].sum(axis=1) for bb in bases])
    form = Bn if add_atoms else B                                    # the counts the RDB and the rules are about
    r_db = 1.0 + form @ np.array([db.get(b, 0.0) for b in bases])
    sel &= (r_db >= rdb[0]) & (r_db <= rdb[1])
    if nitrogen != "none":                                           # parity of the number of electrons of the ion: even = closed shell
        n_e = B @ np.array([ATOMIC_NUMBER[b] for b in bases]) - z
        sel &= ((n_e % 2 == 0) if nitrogen == "even" else (n_e % 2 == 1))
    failed = _rules_matrix(form, bases, r_db, half_ok=ion in ("[M]+", "[M]-"))
    for rn in (rules or []):
        if rn in failed:
            sel &= ~failed[rn]
    idx = np.nonzero(sel)[0]
    if not len(idx):
        return empty
    nfail = sum(failed[rn][idx].astype(int) for rn in RULES)
    order = idx[np.lexsort((nfail, np.abs(E[idx])))]
    out = []
    scored = bool(obs) or bool(frags)
    for n in order[:max(max_results, pool) if scored else max_results].tolist():
        counts = {k: int(v) for k, v in zip(syms, X[n]) if v}
        neutral = {k: int(v) for k, v in zip(syms, Nn[n]) if v}
        calc = ion_mass(counts, z)
        d_da = calc - mz
        bad = [rn for rn in RULES if failed[rn][n]]
        res = {"formula": format_formula(counts), "neutral": format_formula(neutral) if add_atoms else None, "mz_calc": round(calc, 5),
               "delta_mda": round(d_da * 1000, 3), "delta_ppm": round(d_da / mz * 1e6, 3), "rdb": float(r_db[n]), "rules_failed": bad, "rules_ok": len(RULES) - len(bad)}
        t1, t2 = isotope_ratios(counts)
        iso = {"m1": round(t1, 4), "m2": round(t2, 4)}
        if m1 is not None or m2 is not None:
            e1 = abs(m1 - t1) / max(t1, 1e-9) if m1 is not None else None
            e2 = abs(m2 - t2) / max(t2, 1e-9) if m2 is not None else None
            iso.update({"err1": None if e1 is None else round(e1, 3), "err2": None if e2 is None else round(e2, 3),
                        "ok": all(e <= iso_tol for e in (e1, e2) if e is not None)})
        if obs:
            from .isoscore import isotope_score
            sc = isotope_score(counts, z, obs, r200=r200, sigma_ppm=sigma_ppm, sigma_int=sigma_int)
            iso.update({"score": round(sc["score"], 4), "peaks": sc["peaks"], "extra": sc["extra"], "n_missing": sc["n_missing"], "n_extra": sc["n_extra"]})
        res["iso"] = iso
        if frags:
            from .isoscore import fragment_constraint
            res["frag"] = fragment_constraint(res["formula"], z, frags, ppm=max(tol, 5.0) if unit == "ppm" else 10.0)
        if scored:
            from .isoscore import combined
            mt = math.exp(-0.5 * (res["delta_ppm"] / sigma_ppm) ** 2)
            fr = res.get("frag", {}).get("explained")
            res["score"] = {"mass": round(mt, 4), "iso": iso.get("score"), "frag": None if fr is None else round(fr, 4),
                            "total": round(combined(mt, iso.get("score"), fr, weights), 4)}
        out.append(res)
    if scored:
        out.sort(key=lambda r: -r["score"]["total"])
        out = out[:max_results]
    return {"mz": mz, "charge": z, "tol": tol, "unit": unit, "ion": ion, "n_candidates": int(len(idx)), "results": out}


def _rules_matrix(form: np.ndarray, bases: list[str], r_db: np.ndarray, half_ok: bool = False) -> dict[str, np.ndarray]:
    """For every row of `form` (counts by element, columns = bases): {rule name: True where the rule is BROKEN}."""
    col = {b: i for i, b in enumerate(bases)}
    c = form[:, col["C"]] if "C" in col else np.zeros(len(form))
    out = {}
    for name, (sym, lo, hi) in RATIO_RULES.items():
        n = form[:, col[sym]] if sym in col else np.zeros(len(form))
        with np.errstate(divide="ignore", invalid="ignore"):
            ratio = np.where(c > 0, n / np.where(c > 0, c, 1), 0.0)
        out[name] = (c > 0) & ((ratio < lo - 1e-12) | (ratio > hi + 1e-12))
    whole = (np.abs(r_db - np.rint(r_db)) < 1e-6) | (half_ok & (np.abs(r_db - np.floor(r_db) - 0.5) < 1e-6))
    out["LEWIS"] = (r_db < 0) | ~whole                                # a closed-shell molecule has a non-negative integer RDB (an even-electron ion, half-integer)
    val = np.array([VALENCE.get(b, 0) for b in bases])
    tot = form @ val
    n_atoms = form.sum(axis=1)
    vmax = np.where(form > 0, val[None, :], 0).max(axis=1)
    out["SENIOR"] = (n_atoms > 0) & ~((tot % 2 == 0) & (tot >= 2 * vmax) & (tot >= 2 * (n_atoms - 1)))      # Senior theorem
    return out
