# TPMINE-PRIVATE
"""Network of the products, priority score and confidence levels (WP11).

Every candidate is a dict filled by the engine (see `CANDIDATE_KEYS`); this module turns it into a ranked, explained result: a score of priority 0-100
with five visible components, a confidence level (5, 4, 3: never 2a or 1, which need a standard or a library spectrum; 2b is the chemist's decision, see `livelli.js`) with the criteria that
support it, and a predecessor in the network of transformations. Every result is a hypothesis with its evidence, not an identification."""
from __future__ import annotations

import itertools
import math

import numpy as np

from tpmine import chem

from . import formula as F
from . import rules as RU

WEIGHTS = {"ms2": 0.45, "loc": 0.15, "kin": 0.15, "form": 0.10, "int": 0.15}
PENALTY = 0.3                         # isotope, adduct, in-source fragment or dimer of a stronger ion of the family: shown last, never removed
MIN_MATCHED = 3                       # peaks matched in the modified cosine for the MS2 component
LOC_MARGIN = 0.25                     # log-likelihood margin that counts as a determined site
COS_RELATED = 0.6
COVERAGE_FRACTION = 0.5               # a profile present in this fraction of the treated samples (at least 4) has the full kinetic component
COVERAGE_FLOOR = 0.4                  # ... and one present in a single sample would keep this fraction of it
LEVELS = {5: "5", 4: "4", 3: "3"}
LEVEL_ORDER = {3: 1, 4: 2, 5: 3, None: 4}
LEVEL_TEXT = {
    5: "Level 5: mass of interest. Appears with the treatment and passes the filters; nothing more.",
    4: "Level 4: unique formula within 3 ppm, derivable from the parent, consistent isotopes.",
    3: "Level 3: tentative candidate. In addition, the MS2 is linked to that of the parent and the modification is localised at group level.",
}
DIAGNOSTIC_TEXT = ("Proposed level 3, diagnostic evidence: site determined with margin, RT and kinetics consistent with the modification. "
                   "Level 2b is only written by the chemist's decision, never by the program.")
CANDIDATE_KEYS = ("id", "mz", "rt", "formula", "n_formulas", "ppm", "area_max", "kinetics", "ms2", "localization", "iimn", "isotopes", "derivation")

# neutral changes besides the user's transformation list (names in Italian): ordered (name, delta as signed formula text)
HR_TRANSFORMATIONS = [("hydroxylation", "+O"), ("dehydrogenation", "-H2"), ("oxidation to carbonyl", "+O-H2"), ("loss of C4H8", "-C4H8"),
                      ("loss of C2H2", "-C2H2"), ("loss of CO", "-CO"), ("hydration", "+H2O"),
                      ("dehydration", "-H2O"), ("loss of C2H4", "-C2H4"), ("demethylation", "-CH2"), ("dihydroxylation", "+O2"), ("trihydroxylation", "+O3")]
NATURAL = {"C13": 0.0107, "N15": 0.00364, "S33": 0.0075, "H2": 0.000115, "O17": 0.00038}


# ---------------------------------------------------------------------------------------------------------------------- transformations
def transformation_table(els: list[str], extra=None) -> list[tuple[str, np.ndarray]]:
    """(name, signed element vector) of the known changes: the HR list, the transformations of `tpmine.chem`, and the user's own (`extra`: list of
    {"name", "delta" (dict)})."""
    rows = [(n, d) for n, d in HR_TRANSFORMATIONS]
    out = []
    for name, text in rows:
        out.append((name, _signed(chem.parse_delta(text), els)))
    for t in chem.default_transformations() + list(extra or []):
        out.append((t["name"], _signed(t["delta"], els)))
    seen, uniq = set(), []
    for n, v in out:
        key = tuple(v.tolist())
        if v is not None and any(v) and key not in seen:
            seen.add(key)
            uniq.append((n, v))
    return uniq


def _signed(delta: dict, els: list[str]) -> np.ndarray:
    return np.array([int(delta.get(e, 0)) for e in els], dtype=np.int64)


def derive(delta: np.ndarray, table, max_steps: int = 2) -> str | None:
    """Name of the known transformation (or the two-step combination) that gives this change of formula, else None."""
    d = np.asarray(delta)
    if not d.any():
        return None
    for name, v in table:
        if np.array_equal(v, d):
            return name
    if max_steps >= 2:
        for (n1, v1), (n2, v2) in itertools.combinations_with_replacement(table, 2):
            if np.array_equal(v1 + v2, d):
                return f"{n1} + {n2}"
    return None


def derivation_name(tp_vec: np.ndarray, parent_vec: np.ndarray, els: list[str], table, h_slack: int = 3) -> str | None:
    """How a product derives from the parent: a known transformation (one or two steps), or a cleavage: the heavy atoms of the product are a part of the
    parent's (every element not above the parent's, hydrogens within `h_slack`), so the product can be a substructure of it; with up to three more oxygens
    it is a cleaved and oxidised part."""
    name = derive(tp_vec - parent_vec, table)
    if name:
        return name
    heavy = [i for i, e in enumerate(els) if e != "H"]
    iH, iO = els.index("H"), els.index("O")
    rest = [i for i in heavy if i != iO]
    if not (np.all(tp_vec[rest] <= parent_vec[rest]) and np.any(tp_vec[rest] < parent_vec[rest]) and tp_vec[iH] <= parent_vec[iH] + h_slack):
        return None
    if tp_vec[iO] <= parent_vec[iO]:
        return "cleavage (substructure of the parent)"
    return f"cleavage and oxidation (+{int(tp_vec[iO] - parent_vec[iO])} O relative to the parent)" if tp_vec[iO] - parent_vec[iO] <= 3 else None


# ---------------------------------------------------------------------------------------------------------------------- isotopes
def expected_isotopes(vec: np.ndarray, els: list[str]) -> dict:
    """Expected (M+1)/M and (M+2)/M of an ion formula from the natural abundances (13C, 15N, 33S for M+1; 34S, 13C2 for M+2)."""
    n = {e: int(v) for e, v in zip(els, vec)}
    m1 = n.get("C", 0) * NATURAL["C13"] + n.get("N", 0) * NATURAL["N15"] + n.get("S", 0) * NATURAL["S33"] + n.get("H", 0) * NATURAL["H2"] + n.get("O", 0) * NATURAL["O17"]
    c = n.get("C", 0)
    m2 = n.get("S", 0) * 0.0425 + c * (c - 1) / 2 * NATURAL["C13"] ** 2 + n.get("O", 0) * 0.002
    return {"m1": m1, "m2": m2, "has_s": n.get("S", 0) > 0}


def mz_index(al):
    """(m/z sorted, order) of the alignment groups: build once, pass to `observed_isotopes`."""
    o = np.argsort(al.mz, kind="stable")
    return al.mz[o], o


def observed_isotopes(al, g: int, index=None, tol_da: float = 0.003, rt_tol: float = 0.05) -> dict:
    """(M+1)/M and (M+2)/M observed for the alignment group g: the group at +1.003355 (and +1.9958 with sulfur, +2.00671) with the same retention
    time, using the file where g is strongest. None where the isotope is not there (too weak to be detected)."""
    mzs, order = index if index is not None else mz_index(al)
    k = int(np.argmax(al.area[g]))
    base = al.area[g, k]
    out = {"file": k, "m1": None, "m2": None}
    if base <= 0:
        return out
    for key, deltas in (("m1", (1.003355,)), ("m2", (1.99580, 2.00671))):
        best = 0.0
        for d in deltas:
            lo = np.searchsorted(mzs, al.mz[g] + d - tol_da)
            hi = np.searchsorted(mzs, al.mz[g] + d + tol_da, "right")
            for j in order[lo:hi]:
                if abs(al.rt[j] - al.rt[g]) <= rt_tol:
                    best = max(best, float(al.area[j, k]))
        out[key] = best / base if best > 0 else None
    return out


def isotopes_coherent(observed: dict, expected: dict, tol_rel: float = 0.5) -> tuple[str, str]:
    """('pass' | 'fail' | 'n/a', text): the M+1 / M ratio within `tol_rel` of the expected one (M+2 too when the formula has sulfur). Weak isotopes that are not
    detected cannot contradict: n/a."""
    m1o, m1e = observed.get("m1"), expected["m1"]
    if m1o is None:
        return "n/a", "M+1 not detected (too weak)"
    ok1 = abs(m1o - m1e) <= tol_rel * m1e
    txt = f"(M+1)/M = {m1o:.3f}, atteso {m1e:.3f}"
    if expected["has_s"] and observed.get("m2") is not None:
        ok2 = abs(observed["m2"] - expected["m2"]) <= max(tol_rel * expected["m2"], 0.02)
        txt += f"; (M+2)/M = {observed['m2']:.3f}, atteso {expected['m2']:.3f}"
        return ("pass" if ok1 and ok2 else "fail"), txt
    return ("pass" if ok1 else "fail"), txt


# ---------------------------------------------------------------------------------------------------------------------- kinetics
def kinetic_coherence(kin: dict | None) -> tuple[float, str]:
    """Component 'kin' in [0, 1]: 1 when the product is absent (or tiny) in dark / t0 and its profile is unimodal or accumulates (a rise to a maximum, then
    a fall or a plateau), scaled by the coverage of the series (0.4 + 0.6 x the fraction of treated samples that have it, 1 from half of them: a product
    seen in two or three samples only is a weak case, in the measured series a third of the candidates are like that and one real product of 17);
    0.5 when it cannot be decided (fewer than four treated samples); 0 when it is present before the treatment or noisy."""
    if not kin or not kin.get("ok"):
        return 0.5, "profile not assessable"
    n = len(kin.get("times", []))
    if n < 4:
        return 0.5, "fewer than four samples: undecidable"
    if not kin.get("absent_in_reference", True):
        return 0.0, "present in dark/t0"
    if kin.get("unimodal") or kin.get("class") and "persistent" in kin["class"]:
        text = f"{kin.get('class_text', '')}: sale e poi scende o si accumula"
        if kin.get("profile") is None:
            return 1.0, text
        t, y = np.asarray(kin["times"], float), np.asarray(kin["profile"], float)
        n_tr, n_in = int((t > 0).sum()), int(((t > 0) & (y > 0)).sum())
        cov = min(1.0, n_in / max(4.0, COVERAGE_FRACTION * n_tr))
        return COVERAGE_FLOOR + (1 - COVERAGE_FLOOR) * cov, f"{text}; present in {n_in} of {n_tr} treated samples"
    return 0.0, "profilo irregolare"


# ---------------------------------------------------------------------------------------------------------------------- score
def components(c: dict, log_area_norm: float) -> dict:
    """The five components of the priority, each in [0, 1], with the reason of each."""
    ms2 = c.get("ms2") or {}
    cos = ms2.get("modcos")
    n_ms2 = ms2.get("n_matched", 0)
    s_ms2 = float(cos) if (cos is not None and n_ms2 >= MIN_MATCHED and math.isfinite(cos)) else 0.0
    loc = c.get("localization") or {}
    n_atoms = loc.get("n_atoms")
    s_loc = 0.0
    if loc.get("ok") and loc.get("margin", 0.0) >= LOC_MARGIN and n_atoms:
        s_loc = max(0.0, 1.0 - len(loc["region_atoms"]) / n_atoms)
    s_kin, kin_text = kinetic_coherence(c.get("kinetics"))
    uniq = c.get("formula") is not None and c.get("n_formulas", 0) == 1
    s_form = 1.0 if (uniq and c.get("derivation")) else 0.5 if uniq else 0.0
    return {"ms2": s_ms2, "loc": s_loc, "kin": s_kin, "form": s_form, "int": float(np.clip(log_area_norm, 0, 1)),
            "text": {"ms2": (f"modified cosine {cos:.2f} with the parent ({n_ms2} peaks)" if cos is not None and n_ms2 >= MIN_MATCHED else "no MS2 linked to the parent"),
                     "loc": (f"region of {len(loc['region_atoms'])} atoms out of {n_atoms}, margin {loc['margin']:.2f}" if s_loc else "region not determined"),
                     "kin": kin_text,
                     "form": ("unique and derivable formula" if s_form == 1 else "unique formula, not derivable" if s_form else "non-unique formula"),
                     "int": "area massima normalizzata sui candidati"}}


def priority(c: dict, log_area_norm: float, weights: dict | None = None) -> dict:
    """Score 0-100 = sum of weight x component, times PENALTY when the ion is explained by a family (isotope, adduct, in-source fragment, dimer).
    Candidates without MS2 are not penalised beyond a zero in 'ms2' and 'loc': the list of inclusion proposes them for a second injection."""
    w = weights or WEIGHTS
    comp = components(c, log_area_norm)
    s = sum(w[k] * comp[k] for k in w) / max(sum(w.values()), 1e-12)
    role = (c.get("iimn") or {}).get("role", "ion")
    pen = PENALTY if role != "ion" else 1.0
    return {"score": 100.0 * s * pen, "raw": 100.0 * s, "penalty": pen, "components": comp, "explained_by_family": role != "ion"}


# ---------------------------------------------------------------------------------------------------------------------- confidence
def _crit(name, status, text):
    return {"name": name, "status": status, "text": text}


def confidence(c: dict, parent_rt: float | None = None, groups_of_region: int | None = None) -> dict:
    """Confidence level and the criteria behind it. Level 5: passes the filters. 4: unique formula within 3 ppm, coherent isotopes, derivable. 3: also an
    MS2 related to the parent's (modified cosine >= 0.6 or >= 4 pieces of evidence) and a region localised to the level of a group. With also a site
    determined with a margin (region within two groups, >= 60 % of the MS2 intensity explained), a retention time coherent with the modification
    (more polar elutes earlier) and a kinetics coherent with the predecessor, the proposed level stays 3 and `diagnostic` is true: 2b is written only
    by the chemist (field «decided level»). Never 2a, 2b or 1."""
    crit = [_crit("appears with the treatment", "pass", "passes the filters (fold, isotopologues, feasible formula, profile)")]
    f_ok = c.get("formula") is not None and c.get("n_formulas", 0) == 1
    crit.append(_crit("unique formula within 3 ppm", "pass" if f_ok else "fail", (f"{c['formula']} ({c.get('ppm', 0):+.1f} ppm)" if c.get("formula") else "no formula") + ("" if f_ok or not c.get("n_formulas") else f"; {c['n_formulas']} possible formulas")))
    iso = c.get("isotopes") or {}
    crit.append(_crit("consistent isotopes", iso.get("status", "n/a"), iso.get("text", "not assessed")))
    der = c.get("derivation")
    crit.append(_crit("derivable from the parent", "pass" if der else "fail", der or "no known transformation or substructure"))
    ms2 = c.get("ms2") or {}
    loc = c.get("localization") or {}
    n_ev = len(loc.get("evidence", [])) if loc else 0
    related = (ms2.get("modcos") is not None and ms2.get("n_matched", 0) >= MIN_MATCHED and ms2["modcos"] >= COS_RELATED) or n_ev >= 4
    if ms2.get("n_scans"):
        crit.append(_crit("MS2 linked to the parent", "pass" if related else "fail",
                          f"modified cosine {ms2['modcos']:.2f} ({ms2.get('n_matched', 0)} peaks)" + (f"; {n_ev} shifted/unshifted fragments" if n_ev else "") if ms2.get("modcos") is not None else f"{n_ev} shifted/unshifted fragments"))
    else:
        crit.append(_crit("MS2 linked to the parent", "n/a", "no DDA MS2 for this ion"))
    has_loc = bool(loc.get("ok"))
    n_groups = groups_of_region
    crit.append(_crit("localised region (group)", "pass" if has_loc else ("fail" if ms2.get("n_scans") else "n/a"), loc.get("region", "not localised") if has_loc else loc.get("note", "MS2 required")))
    determined = has_loc and loc.get("margin", 0.0) >= LOC_MARGIN and (n_groups is None or n_groups <= 2) and loc.get("fraction_explained", 0.0) >= 0.6
    crit.append(_crit("site determined with margin", "pass" if determined else ("fail" if has_loc else "n/a"),
                      (f"region {loc['region']}, margin {loc['margin']:.2f}, {100 * loc['fraction_explained']:.0f}% of the intensity explained" if has_loc else "no localisation")))
    rt_ok = _rt_coherent(c, parent_rt)
    crit.append(_crit("RT consistent with the modification", rt_ok[0], rt_ok[1]))
    kin = c.get("kinetics")
    pk = c.get("predecessor_kinetics")
    k_ok = "n/a" if pk is None or not kin or not kin.get("ok") else ("pass" if pk <= kin["tmax"] + c.get("time_step", 5.0) else "fail")
    crit.append(_crit("kinetics consistent with the predecessor", k_ok, "no predecessor with kinetics" if k_ok == "n/a" else f"predecessor: maximum at {pk:g} min, this one at {kin['tmax']:g} min"))
    st = {x["name"]: x["status"] for x in crit}
    level, diagnostic = 5, False
    if st["unique formula within 3 ppm"] == "pass" and st["consistent isotopes"] != "fail" and st["derivable from the parent"] == "pass":
        level = 4
        if st["MS2 linked to the parent"] == "pass" and st["localised region (group)"] == "pass":
            level = 3
            if st["site determined with margin"] == "pass" and st["RT consistent with the modification"] != "fail" and st["kinetics consistent with the predecessor"] != "fail":
                diagnostic = True
    return {"level": level, "diagnostic": diagnostic, "text": DIAGNOSTIC_TEXT if diagnostic else LEVEL_TEXT[level], "criteria": crit}


def _rt_coherent(c: dict, parent_rt: float | None) -> tuple[str, str]:
    """More polar (oxygen gained, a part lost) elutes before the parent on a reversed-phase column; fewer polar (dehydrogenation) after or close."""
    der = (c.get("derivation") or "")
    rt, prt = c.get("rt"), parent_rt
    if rt is None or prt is None:
        return "n/a", "RT of the parent unknown"
    polar = any(w in der for w in ("hydroxyl", "oxidation", "hydration", "loss", "demethyl"))
    if not polar:
        return "n/a", "no prediction for this modification"
    return ("pass", f"RT {rt:.2f} before the parent ({prt:.2f})") if rt < prt else ("fail", f"RT {rt:.2f} after the parent ({prt:.2f}), expected before")


# ---------------------------------------------------------------------------------------------------------------------- network
def predecessors(cands: list[dict], els: list[str], parent_formula, table, cos=None, step: float = 5.0) -> list[dict]:
    """For every candidate with a formula, the best predecessor among the parent and the other candidates: the change of formula is a known transformation
    (one or two steps), or the modified cosine of their MS2 is >= 0.6 (`cos[(i, j)]` = (score, matched), optional), and the maximum of the predecessor is
    not later than its own plus one sampling step. Among several, the largest of (modified cosine + kinetic coherence). Returns one dict per candidate
    {"id", "predecessor" (id or 'progenitore'), "transformation", "cos", "kinetic"}; predecessor None when nothing qualifies."""
    n = len(cands)
    vec = [F.vec(c["formula"], els) if c.get("formula") else None for c in cands]
    pv = F.vec(parent_formula, els)
    out = []
    for i, c in enumerate(cands):
        res = {"id": c["id"], "predecessor": None, "transformation": None, "cos": None, "kinetic": None}
        if vec[i] is None:
            out.append(res)
            continue
        tmax_i = (c.get("kinetics") or {}).get("tmax")
        best, best_val = None, -1.0
        # the parent
        name = derive(vec[i] - pv, table)
        if name:
            best, best_val = ("progenitore", name, None, 1.0), 0.5
        for j in range(n):
            if j == i or vec[j] is None:
                continue
            name = derive(vec[i] - vec[j], table, max_steps=1)
            cs = cos.get((j, i)) if cos else None
            related = cs is not None and cs[1] >= MIN_MATCHED and cs[0] >= COS_RELATED
            if not name and not related:
                continue
            tmax_j = (cands[j].get("kinetics") or {}).get("tmax")
            if tmax_j is not None and tmax_i is not None and tmax_j > tmax_i + step:
                continue
            coh = 1.0 if (tmax_j is None or tmax_i is None or tmax_j <= tmax_i) else 0.5
            val = (cs[0] if related else 0.0) + coh
            if val > best_val:
                best, best_val = (cands[j]["id"], name, cs[0] if related else None, coh), val
        if best:
            res.update(predecessor=best[0], transformation=best[1], cos=best[2], kinetic=best[3])
        out.append(res)
    return out


def mass_network(cands: list[dict], els: list[str], parent_formula, table, preds=None, sim=None, parent_smiles=None, lit=None, step: float = 5.0,
                 max_edges: int = 1500) -> dict:
    """Network of mass differences among the parent and the candidates: an edge a -> b for every rule of the table whose change of formula takes the
    formula of a to that of b (one step; the pairs of an exact reverse rule, such as hydration and dehydration, give one edge, in the direction of the
    kinetics: the earlier maximum is the source). Each edge has the rule, the exact change of m/z, the prediction scope of the rule, the kinetic
    consistency (maxima of the profiles) and, when `sim(i, j)` (indices in `cands`, -1 = the parent) returns the three components of the similarity of
    their MS2, the shifted fragments. `preds` (output of `predecessors`) marks the edges of the existing network of the products: kind 'both' (a
    predecessor that is also a mass difference), 'mass' (only the mass difference), 'network' (a predecessor of two steps or related by MS2).
    Returns {"nodes": [{id, label, formula, mz, rt, score, level}], "edges": [...], "n_edges": total before the cut}."""
    n = len(cands)
    vecs = [F.vec(parent_formula, els)] + [F.vec(c["formula"], els) for c in cands]       # node 0 = the parent
    ids = ["progenitore"] + [c["id"] for c in cands]
    by_vec: dict = {}
    for k, v in enumerate(vecs):
        by_vec.setdefault(tuple(v.tolist()), []).append(k)
    tmax = [0.0] + [(c.get("kinetics") or {}).get("tmax") for c in cands]
    chosen = {(p["predecessor"], p["id"]): p for p in (preds or []) if p.get("predecessor") is not None}
    best: dict = {}
    for a in range(n + 1):
        for name, v in table:
            for b in by_vec.get(tuple((vecs[a] + v).tolist()), []):
                if b == a or b == 0:
                    continue
                ta, tb = tmax[a], tmax[b]
                kin = "n/a" if ta is None or tb is None else ("consistent" if ta <= tb + step else "against")
                rank = (2 if kin == "consistent" else 1 if kin == "n/a" else 0, 1 if a == 0 else 0)
                key = (min(a, b), max(a, b))
                if key not in best or rank > best[key][0]:
                    best[key] = (rank, a, b, name, kin)
    edges = []
    for _, a, b, name, kin in best.values():
        e = {"from": ids[a], "to": ids[b], "rule": name, "kinetic": kin, "dm": round(float(F.ion_mass(vecs[b], els) - F.ion_mass(vecs[a], els)), 5),
             "scope": RU.step_info(name, table, vecs[0], els, parent_smiles, lit or {})["scope"],
             "kind": "both" if (ids[a], ids[b]) in chosen else "mass"}
        sm = sim(a - 1, b - 1) if sim is not None else None
        if sm:
            e["shifted"] = {"n": sm["n"]["shifted"] + sm["n"]["loss"], "share": round(sm["cos"]["shifted"] + sm["cos"]["loss"], 3)}
        edges.append(e)
    have = {(e["from"], e["to"]) for e in edges}
    for (p, i), pr in chosen.items():
        if (p, i) not in have and (i, p) not in have:
            edges.append({"from": p, "to": i, "rule": pr.get("transformation") or "MS2 related", "kinetic": "consistent", "kind": "network", "scope": None,
                          "dm": round(float(F.ion_mass(vecs[ids.index(i)], els) - F.ion_mass(vecs[ids.index(p)], els)), 5)})
    total = len(edges)
    if total > max_edges:         # the strongest first: predecessors, then edges with shifted fragments, then the rest
        edges.sort(key=lambda e: (e["kind"] == "mass", -(e.get("shifted") or {}).get("share", 0.0)))
        edges = edges[:max_edges]
    used = {e["from"] for e in edges} | {e["to"] for e in edges}
    nodes = [{"id": "progenitore", "label": "parent", "formula": F.fmt(vecs[0], els), "mz": None, "rt": None, "score": None, "level": None}]
    for k, c in enumerate(cands, 1):
        if c["id"] in used:
            nodes.append({"id": c["id"], "label": c.get("derivation") or c["formula"], "formula": c["formula"], "mz": round(c["mz"], 4), "rt": round(c["rt"], 2),
                          "score": round(c["priority"]["score"], 1) if c.get("priority") else None, "level": (c.get("confidence") or {}).get("level")})
    return {"nodes": nodes, "edges": edges, "n_edges": total}


def rank(cands: list[dict], by_level: bool = False) -> list[dict]:
    """Candidates ordered by priority score (the score already carries the evidence of the level: MS2, region, kinetics, formula, intensity). With
    `by_level` ordered by confidence level (level 3 with diagnostic evidence first) and, within it, by score: measured on the reference series that puts all the level-3 candidates
    (an MS2 related to the parent's, many of them not products) above the level-4 products that have no useful MS2 (17 of 20 products in the first 100
    by score, 13 by level). Those explained by a family keep their place by score but stay visible."""
    if by_level:
        return sorted(cands, key=lambda c: (0 if c["confidence"].get("diagnostic") else LEVEL_ORDER[c["confidence"]["level"]], -c["priority"]["score"]))
    return sorted(cands, key=lambda c: -c["priority"]["score"])