"""Reference of the ion families of `crates/mzlab-feat/src/family.rs`, in plain Python loops over the rules of `tpmine.hr.iimn` and `edge_evidence`.

    python3 tools/feat_riferimento.py OUT --solo famiglie [--golden]

writes `famiglie_coppie.json` (the label and the evidence of `edge_evidence.edge_evidence` on a grid of m/z steps, MS2 and ratios) and
`famiglie_identita.json` (the peaks of two windows of retention time of one file of the synthetic identity series with their traces and MS2 spectra,
and the pairs, arcs, roles and alternatives that the loops below find): `crates/mzlab-feat/tests/family_parity.rs` gives the same input to the Rust
code and wants the same answer. Differences from `edge_evidence`: no ratio over samples exists in one file (it counts as stable), and a peak that is a
fragment in the MS2 of a heavier co-eluting ion with no step in the tables is read as an in-source fragment («ms2» loss).
"""
from __future__ import annotations

import math

import numpy as np

from tpmine.hr import edge_evidence as EE

PARAMS = {"ppm": 5.0, "abs_da": 0.002, "r_min": 0.9, "coelution_min": 0.05, "parent_factor": 2.0, "trace_ppm": 6.0}
WEIGHT = {"coelution": 1.0, "frag_ms2": 2.0, "ratio": 2.0, "skeleton": 1.0, "typical_loss": 2.0, "no_fragmentation": 0.5}
SIGN = {"evidence": 1, "against": -1, "hint": 0}
NO_PARENT = -1
ROLE = {"M": 1, "isotope": 2, "adduct": 3, "isf": 4, "dimer": 5, "undetermined": 6}


# ---------------------------------------------------------------------------------------------------------------- pair cases
def _spec(mz, it):
    return {"mz": list(mz), "it": list(it)}


def casi_coppie() -> list[dict]:
    """Grid of cases: steps of the tables in the two polarities, with and without MS2, with stable / unstable / few ratios."""
    nh3, na, k, acn = 17.02654910112, 21.98194424977, 37.95588136777, 41.02654910112
    cases = []
    for pol, steps in ((1, [("nh4", nh3), ("na", na), ("k", k), ("acn", acn), ("h2o", 18.01056468403), ("hcooh", 46.0054793036), ("co2", 43.98982923914),
                            ("dimer", None), ("none", 12.345), ("c4h8", 56.06260025784)]),
                       (-1, [("fa", 46.0054793036), ("cl", 35.97667771423), ("nana", na), ("h2o", 18.01056468403), ("dimer", None)])):
        for name, d in steps:
            lo = 250.1234 if name != "none" else 301.2
            up = lo + d if d is not None else 2 * (lo - pol * 1.007276452321) + pol * 1.007276452321
            frag_lo = [60.0561, 120.0813, 160.0752, lo - 40.0313]
            for ms2 in ("none", "up_has_lo", "up_without_lo", "both_shared", "both_apart", "up_loses_nh3", "up_bare"):
                s_up = s_lo = None
                if ms2 == "up_has_lo":
                    s_up = _spec([60.0561, lo, up - 17.0265, 200.1], [10, 40, 25, 60])
                elif ms2 == "up_without_lo":
                    s_up = _spec([61.1, 99.2, 150.3, up - 30.0], [10, 40, 25, 60])
                elif ms2 == "both_shared":
                    s_lo = _spec(frag_lo, [30, 50, 70, 90])
                    s_up = _spec([f + (up - lo) * (i % 2) for i, f in enumerate(frag_lo)] + [up - 50.0], [30, 50, 70, 90, 5])
                elif ms2 == "both_apart":
                    s_lo = _spec(frag_lo, [30, 50, 70, 90])
                    s_up = _spec([61.0, 99.0, 151.0], [30, 50, 70])
                elif ms2 == "up_loses_nh3":
                    s_up = _spec([60.0, up - nh3, 200.0], [10, 100, 20])
                    s_lo = _spec(frag_lo, [30, 50, 70, 90])
                elif ms2 == "up_bare":
                    s_up = _spec([up], [100])
                for ratio in ("none", "stable", "unstable", "few"):
                    a_lo, a_up = {"none": (None, None), "stable": ([1e6, 2e6, 3e6, 4e6], [2e5, 4.1e5, 5.9e5, 8.2e5]),
                                  "unstable": ([1e6, 2e6, 3e6, 4e6], [2e5, 1e5, 9e5, 2e5]), "few": ([1e6, 2e6], [2e5, 4e5])}[ratio]
                    if ratio != "none" and ms2 not in ("none", "up_has_lo", "both_shared"):
                        continue
                    for drt in (0.01, 0.2):
                        if (ratio, drt) not in (("none", 0.01), ("none", 0.2), ("stable", 0.01), ("unstable", 0.01), ("few", 0.01)) or (drt == 0.2 and ms2 != "none"):
                            continue
                        cases.append({"pol": pol, "step": name, "mz_lo": lo, "mz_up": up, "ms2_lo": s_lo, "ms2_up": s_up, "areas_lo": a_lo, "areas_up": a_up, "drt_min": drt})
    return cases


def risposta_coppia(c: dict) -> dict:
    """What `edge_evidence` says. Without areas the ratio is not asked: the reference counts it as stable and leaves its proof out."""
    has = c["areas_lo"] is not None
    r = EE.edge_evidence(c["mz_lo"], c["mz_up"], c["pol"], ms2_lo=c["ms2_lo"], ms2_up=c["ms2_up"], areas_lo=c["areas_lo"] if has else [1.0, 1.0, 1.0],
                         areas_up=c["areas_up"] if has else [2.0, 2.0, 2.0], drt_min=c["drt_min"])
    proofs = [[e["key"], 0 if e["key"] == "dm" else SIGN[e["strength"]]] for e in r["evidence"]
              if e["key"] == "dm" or (e["strength"] in SIGN and (e["key"] != "ratio" or has))]
    return {"kind": r["kind"], "readings": [[x["role"], x["name"]] for x in r["relations"] if x["role"] != "isotope"], "proofs": proofs}


def coppie_json() -> dict:
    return {"cases": [{**c, "want": risposta_coppia(c)} for c in casi_coppie()]}


# ---------------------------------------------------------------------------------------------------------------- file
def _tol(mz: float) -> float:
    return EE.tol(mz)


def _corr(tr: dict, a: int, b: int, apex, fwhm):
    (fa, ya), (fb, yb) = tr[a], tr[b]
    fw = max(int(math.ceil(max(fwhm[a], fwhm[b]))), 2)
    lo = max(min(apex[a], apex[b]) - fw, fa, fb)
    hi = min(max(apex[a], apex[b]) + fw, fa + len(ya) - 1, fb + len(yb) - 1)
    if hi - lo + 1 < 5:
        return None
    x, y = np.array(ya[lo - fa:hi - fa + 1]), np.array(yb[lo - fb:hi - fb + 1])
    dx, dy = x - x.mean(), y - y.mean()
    sxx, syy = float((dx * dx).sum()), float((dy * dy).sum())
    return None if sxx <= 0 or syy <= 0 else float((dx * dy).sum() / math.sqrt(sxx * syy))


def _arc(rows: dict, i: int, j: int, r: float, pol: int, spec: dict) -> dict | None:
    mz, rt = rows["mz"], rows["rt"]
    s_lo, s_up = spec.get(i), spec.get(j)
    drt = float(abs(np.float32(rt[j]) - np.float32(rt[i])))
    kw = dict(ms2_lo=s_lo, ms2_up=s_up, areas_lo=[1.0, 1.0, 1.0], areas_up=[2.0, 2.0, 2.0], drt_min=drt)
    rels = [x for x in EE.relations_of(mz[i], mz[j], pol) if x["role"] != "isotope"]
    if rels:
        e = EE.edge_evidence(mz[i], mz[j], pol, **kw)
        proofs = [[x["key"], 0 if x["key"] == "dm" else SIGN[x["strength"]]] for x in e["evidence"] if x["key"] == "dm" or x["strength"] in SIGN and x["key"] != "ratio"]
        return {"lo": i, "up": j, "r": r, "kind": e["kind"], "readings": [[x["role"], x["name"]] for x in rels], "proofs": proofs}
    sp = EE._spec(s_up) if s_up is not None else None
    if sp is None or EE._near(sp, mz[i], mz[j]) is None:
        return None
    return {"lo": i, "up": j, "r": r, "kind": "isf", "readings": [["loss", "ms2"]],
            "proofs": [["dm", 0], ["coelution", 1 if drt <= PARAMS["coelution_min"] else -1], ["frag_ms2", 1]]}


def _points(arc: dict) -> float:
    return float(np.float32(sum(WEIGHT.get(k, 0.0) * s for k, s in arc["proofs"])))


def _orient(arc: dict, height) -> tuple | None:
    lo, up = arc["lo"], arc["up"]
    hl, hu = np.float32(height[lo]), np.float32(height[up])
    f = np.float32(PARAMS["parent_factor"])
    names = [r[0] for r in arc["readings"]]
    adduct, loss = "adduct" in names or "dimer" in names, "loss" in names
    lo_dep, up_dep = hu >= f * hl, hl >= f * hu
    frag_ok, add_ok = loss and lo_dep, adduct and up_dep
    if arc["kind"] == "adduct" and add_ok:
        return up, lo, ROLE["adduct"] if "adduct" in names else ROLE["dimer"], 0
    if arc["kind"] == "isf" and frag_ok:
        return lo, up, ROLE["isf"], 0
    if frag_ok:
        return lo, up, ROLE["undetermined"], 1
    if add_ok:
        return up, lo, ROLE["undetermined"], 1
    return None


def _assign(n: int, arcs: list, height, excluded, isotope, iso_group) -> tuple:
    parent, role, alt = [NO_PARENT] * n, [0 if excluded[i] else ROLE["M"] for i in range(n)], [0] * n
    for i in range(n):
        if isotope[i] and iso_group[i] > 0 and not excluded[iso_group[i] - 1]:
            parent[i], role[i] = iso_group[i] - 1, ROLE["isotope"]
    deps = []
    for a in arcs:
        o = _orient(a, height)
        if o:
            deps.append((o[0], o[3], -_points(a), -float(np.float32(height[o[1]])), o[1], o[2]))
    deps.sort()
    for d in sorted({x[0] for x in deps}):
        mine = [x for x in deps if x[0] == d]
        chosen = None
        for x in mine:
            if parent[d] == NO_PARENT and chosen is None and not _reaches(parent, x[4], d) and not excluded[x[4]] and role[d] != 0:
                chosen = x
            elif chosen is not None and chosen[1] == x[1] and chosen[4] != x[4]:
                alt[d] |= 8
        if chosen:
            parent[d], role[d] = chosen[4], chosen[5]
    for a in arcs:
        names = [r[0] for r in a["readings"]]
        if a["kind"] == "undetermined" and ("adduct" in names or "dimer" in names) and "loss" in names:
            nm = [r[1] for r in a["readings"]]
            bit = 1 if any(x in ("NH3", "[M+NH4]+") for x in nm) else 2 if any(x in ("[M+FA-H]-", "HCOOH") for x in nm) else 4
            alt[a["lo"]] |= bit
            alt[a["up"]] |= bit
    fam = []
    for i in range(n):
        x = i
        while parent[x] != NO_PARENT:
            x = parent[x]
        fam.append(0 if role[i] == 0 else x + 1)
    return parent, role, alt, fam


def _reaches(parent, frm, target) -> bool:
    x = frm
    for _ in range(len(parent) + 1):
        if x == target:
            return True
        if parent[x] == NO_PARENT:
            return False
        x = parent[x]
    return True


def riferimento_famiglie(rows: dict, traces: dict, spectra: dict, pol: int = 1) -> dict:
    """Plain loops: pairs (a step of the tables, or the lighter peak a fragment in the MS2 of the heavier), correlation, arcs, roles, alternatives.
    rows: mz, rt (float32 values), apex, fwhm, height, isotope, iso_group, excluded (background / lobe), dt. traces: {row: (first scan, values)}.
    spectra: {row: {mz, it}} (already the strongest peaks)."""
    n = len(rows["mz"])
    mz, rt, dt = rows["mz"], rows["rt"], rows["dt"]
    elig = [not rows["excluded"][i] and not rows["isotope"][i] for i in range(n)]

    def coelute(i, j):
        fw = min(np.float32(rows["fwhm"][i]), np.float32(rows["fwhm"][j])) * dt / 60.0
        return float(abs(np.float32(rt[i]) - np.float32(rt[j]))) <= max(PARAMS["coelution_min"], 0.5 * float(fw))

    cand = []
    for j in range(n):
        for i in range(j):
            if not (elig[i] and elig[j] and coelute(i, j)):
                continue
            table = any(x["role"] != "isotope" for x in EE.relations_of(mz[i], mz[j], pol))
            sp = spectra.get(j)
            frag = sp is not None and any(abs(m - mz[j]) > 1.5 * _tol(mz[j]) and m <= mz[j] - 0.5 and abs(m - mz[i]) <= _tol(m) for m in sp["mz"])
            if table or frag:
                cand.append((i, j))
    pairs, arcs = [], []
    for i, j in cand:
        if i not in traces or j not in traces:
            continue
        r = _corr(traces, i, j, rows["apex"], rows["fwhm"])
        if r is None or r < PARAMS["r_min"]:
            continue
        pairs.append([i, j, r])
        a = _arc(rows, i, j, r, pol, spectra)
        if a:
            arcs.append(a)
    parent, role, alt, fam = _assign(n, arcs, rows["height"], rows["excluded"], rows["isotope"], rows["iso_group"])
    proton = 1.007276452321
    alts = []
    for a in arcs:
        names = [r[0] for r in a["readings"]]
        if a["kind"] == "undetermined" and ("adduct" in names or "dimer" in names) and "loss" in names:
            for row, reading in ((a["lo"], 0), (a["up"], 1)):
                pts = sum(WEIGHT.get(k, 0.0) * s for k, s in a["proofs"] if (k in ("frag_ms2", "ratio") if reading else k in ("skeleton", "typical_loss")))
                alts.append([fam[a["lo"]], row, mz[row] - pol * proton, reading, float(np.float32(pts))])
    alts = sorted({tuple(x) for x in alts}, key=lambda x: (x[0], x[1], x[3]))
    return {"pairs": pairs, "arcs": arcs, "parent": parent, "role": role, "alt": alt, "family": fam, "alternatives": [list(x) for x in alts]}
