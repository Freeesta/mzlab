"""Endpoints P1-P5 and secondary numbers of a `bench/1` table against a truth, measured PER COMPOUND (lists aligned with the compounds, so that two tools can be
paired and the compounds resampled). Matching is the deterministic one-to-one of `valida.abbina` (+-ppm, +-RT). Everything is a pure function of its inputs."""
from __future__ import annotations

import numpy as np

from valida.abbina import match_points, match_wide, wide_window

from . import PPM, RT_TOL
from .adatta import matrices
from .verita import FEATURE_ROLES, EXPLAINING_ROLES, with_role, esche, true_features, products
from .verita import larghi as wide_compounds

HIGH_LEVELS = ("1", "2a", "2b", "3")             # better than level 4
THRESHOLD_RANK = 0.9                               # share of the products to reach for "candidates to scan"


def _points(comp: list[dict]) -> tuple[np.ndarray, np.ndarray]:
    return np.array([c["mz"] for c in comp], float), np.array([c["rt"] if c["rt"] is not None else np.nan for c in comp], float)


def row_of(comp: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> np.ndarray:
    """For each compound the row matched to it one-to-one (-1 if none)."""
    out = np.full(len(comp), -1, int)
    if not comp or not t["righe"]:
        return out
    mz, rt, _ = matrices(t)
    cm, cr = _points(comp)
    ia, ib = match_points(cm, cr, mz, rt, ppm, rt_tol)
    out[ia] = ib
    return out


def nearby_rows(comp: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> list[np.ndarray]:
    """For each compound all the rows within the tolerance (not one-to-one)."""
    mz, rt, _ = matrices(t)
    return [np.flatnonzero((np.abs(mz - c["mz"]) <= c["mz"] * ppm * 1e-6) & (np.abs(rt - c["rt"]) <= rt_tol)) for c in comp]


def p1(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Recall of the true features: 1 if the compound has at least one matched row. Also by intensity band when the truth gives `fascia`."""
    comp = true_features(truth)
    hit = (row_of(comp, t, ppm, rt_tol) >= 0).astype(int)
    out = {"per_composto": hit.tolist(), "richiamo": float(hit.mean()) if len(hit) else None, "n": len(hit), "per_fascia": {}}
    for f in sorted({c["fascia"] for c in comp if c["fascia"]}):
        v = [h for h, c in zip(hit.tolist(), comp) if c["fascia"] == f]
        out["per_fascia"][str(f)] = {"n": len(v), "richiamo": sum(v) / len(v)}
    return out


def p2(truth: list[dict], t: dict, groups: dict | None = None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Quantitative accuracy per compound, NaN where it cannot be measured (no row, or a zero area). With `groups` ({"A": [columns], "B": [columns]}) and a
    compound `atteso` (ratio A/B): |log2(observed ratio / expected)| of the group means (D1). Else with `area_attesa` (one value per file): the relative error of
    the mean area over the files with a known value (D2). The summary is the median."""
    comp = true_features(truth)
    riga = row_of(comp, t, ppm, rt_tol)
    _, _, area = matrices(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    err = np.full(len(comp), np.nan)
    for i, (c, r) in enumerate(zip(comp, riga)):
        if r < 0:
            continue
        if groups and c.get("atteso"):
            a, b = area[r, groups["A"]].mean(), area[r, groups["B"]].mean()
            if a > 0 and b > 0 and c["atteso"] > 0:
                err[i] = abs(np.log2(a / b / c["atteso"]))
        elif not groups and c.get("area_attesa"):
            att = np.array([np.nan if v is None else v for v in c["area_attesa"]], float)
            ok = att > 0
            if ok.any():
                err[i] = abs(area[r, ok].mean() - att[ok].mean()) / att[ok].mean()
    v = err[~np.isnan(err)]
    return {"per_composto": [None if np.isnan(x) else float(x) for x in err], "mediana": float(np.median(v)) if len(v) else None, "n_misurati": int(len(v)), "n": len(err)}


def p3(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Integrity of the rows: 1 if the compound is represented by exactly one row and that row is not also within the tolerance of another true compound
    (penalises double rows from a bad alignment and merged isomers)."""
    comp = true_features(truth)
    vic = nearby_rows(comp, t, ppm, rt_tol)
    uso: dict[int, int] = {}
    for v in vic:
        for r in v.tolist():
            uso[r] = uso.get(r, 0) + 1
    ok = [int(len(v) == 1 and uso[int(v[0])] == 1) for v in vic]
    return {"per_composto": ok, "integrita": sum(ok) / len(ok) if ok else None, "n": len(ok)}


def candidate_order(t: dict) -> list[int]:
    """Rows in the order they are scanned: for a table with candidate flags the candidates by points then largest area (the order of the panel); for any other
    table all the rows by largest area. Ties by row number."""
    _, _, area = matrices(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    amax = area.max(1) if area.size else np.zeros(len(t["righe"]))
    righe = t["righe"]
    ha_flag = any(r["candidato"] is not None for r in righe)
    choices = [i for i, r in enumerate(righe) if (r["candidato"] if ha_flag else True)]
    return sorted(choices, key=lambda i: (-(righe[i]["punteggio"] or 0.0), -amax[i], i))


def p4(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """True products found among the candidates (flag 1 = matched to a candidate row) and their rank in the scanning order; `per_90`: the fewest candidates to
    scan to reach 90 % of ALL the products (None if never reached)."""
    comp = products(truth)
    riga = row_of(comp, t, ppm, rt_tol)
    rank = {r: n for n, r in enumerate(candidate_order(t), 1)}
    inside = [int(r >= 0 and r in rank) for r in riga.tolist()]
    ranks = sorted(rank[r] for r in riga.tolist() if r >= 0 and r in rank)
    k = int(np.ceil(THRESHOLD_RANK * len(comp) - 1e-9))
    return {"per_composto": inside, "richiamo_candidati": sum(inside) / len(inside) if inside else None, "n": len(inside), "trovati": int((riga >= 0).sum()), "ranghi": ranks,
            "per_90": (ranks[k - 1] if 0 < k <= len(ranks) else None), "candidati": len(rank)}


def _candidates(t: dict, only_high_levels: bool) -> np.ndarray:
    """True for the rows that are candidates (the flag; a table with no flag offers every row); optionally only those with a level better than 4."""
    out = np.zeros(len(t["righe"]), bool)
    for i, r in enumerate(t["righe"]):
        if r["candidato"] is None or r["candidato"]:
            out[i] = not only_high_levels or r["livello"] is None or r["livello"] in HIGH_LEVELS
    return out


def p5(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """False products: 1 if a decoy has a candidate row with a level better than 4 (decoys with no signal, `esca`, are looked for by m/z at any RT; `isf_finto`
    by m/z and RT). Precision of the candidates: the share of all the candidate rows that match a true compound."""
    dec = esche(truth)
    mz, rt, _ = matrices(t) if t["righe"] else (np.zeros(0), np.zeros(0), None)
    cand = _candidates(t, True)
    wrong = []
    for c in dec:
        dm = np.abs(mz - c["mz"]) <= c["mz"] * ppm * 1e-6
        if c["rt"] is not None:
            dm &= np.abs(rt - c["rt"]) <= rt_tol
        wrong.append(int(bool((dm & cand).any())))
    true = [c for c in truth if c["ruolo"] in FEATURE_ROLES + ("isotopo",) and c["rt"] is not None]
    explained = np.zeros(len(mz), bool)
    for r in (np.concatenate(nearby_rows(true, t, ppm, rt_tol)) if true and len(mz) else []):
        explained[int(r)] = True
    everything = _candidates(t, False)
    nc = int(everything.sum())
    return {"per_composto": wrong, "falsi": sum(wrong) / len(wrong) if wrong else None, "n": len(wrong), "candidati": nc,
            "precisione_candidati": float((everything & explained).sum() / nc) if nc else None}


def secondarie(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Precision of the feature table (rows that no compound with signal explains), rows per matched compound, products' isomers kept apart."""
    comp = true_features(truth)
    vic = nearby_rows(comp, t, ppm, rt_tol)
    spieg = [c for c in truth if c["ruolo"] in EXPLAINING_ROLES and c["rt"] is not None]
    cov = np.zeros(len(t["righe"]), bool)
    for v in nearby_rows(spieg, t, ppm, rt_tol):
        cov[v] = True
    n = len(t["righe"])
    found = [len(v) for v in vic if len(v)]
    iso = [c for c in with_role(truth, ("isomero",)) if c["rt"] is not None]
    ir = row_of(iso, t, ppm, rt_tol)
    return {"righe": n, "righe_non_spiegate": int(n - cov.sum()), "precisione_righe": float(cov.sum() / n) if n else None,
            "righe_per_composto": float(np.mean(found)) if found else None, "isomeri_separati": int((ir >= 0).sum()), "isomeri": len(iso)}


# ---------------------------------------------------------------------------------------------------------------- suspect screening (identification endpoints)
# Declared in docs/agenti/mzfinder-benchmark.md before the first real run. `strutture` maps a true compound id to the name its suspect list carries.
# Levels are the tool's own strings: they are listed side by side per tool and never converted into each other.
def hits_of(t: dict) -> dict[int, list[dict]]:
    """The suspect hits of each row of a table (empty when the tool did not screen suspects)."""
    return {int(x["riga"]): x["hits"] for x in t.get("identificazioni", [])}


def identification(truth: list[dict], t: dict, structures: dict[str, str], ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Per standard (a true compound with a name in `structures`): `colpo` 1 if its matched row has a hit with the true name; `rango` of that hit among the hits of
    the row (1 = best, None if absent); `livello` the tool's level of that hit. Per decoy (`esca` by m/z, `isf_finto` by m/z and RT): `falso` 1 if any row
    on it carries a hit. Summary: hit recall, false-positive rate on decoys, mean reciprocal rank of the true structure (0 when absent), and `livelli` =
    number of standards per level reached (key `none` = no hit with the true name)."""
    hits = hits_of(t)
    std = [c for c in true_features(truth) if c["id"] in structures]
    riga = row_of(std, t, ppm, rt_tol)
    colpo, rank, livello = [], [], []
    for c, r in zip(std, riga.tolist()):
        h = next((x for x in hits.get(r, []) if x["nome"] == structures[c["id"]]), None) if r >= 0 else None
        colpo.append(int(h is not None))
        rank.append(h["rango"] if h else None)
        livello.append(h["livello"] if h else None)
    dec = esche(truth)
    mz, rt, _ = matrices(t) if t["righe"] else (np.zeros(0), np.zeros(0), None)
    with_hit = np.array([bool(hits.get(i)) for i in range(len(mz))], bool)
    wrong = []
    for c in dec:
        dm = np.abs(mz - c["mz"]) <= c["mz"] * ppm * 1e-6
        if c["rt"] is not None:
            dm &= np.abs(rt - c["rt"]) <= rt_tol
        wrong.append(int(bool((dm & with_hit).any())))
    levels: dict[str, int] = {}
    for lv in livello:
        levels[lv or "none"] = levels.get(lv or "none", 0) + 1
    return {"standard": [c["id"] for c in std], "colpo": colpo, "rango": rank, "livello": livello, "n": len(std),
            "richiamo_colpi": sum(colpo) / len(colpo) if colpo else None, "mrr": sum(1 / r for r in rank if r) / len(rank) if rank else None,
            "esche": [c["id"] for c in dec], "falso": wrong, "falsi": sum(wrong) / len(wrong) if wrong else None, "livelli": dict(sorted(levels.items()))}


def endpoint_nts(truth: list[dict], t: dict, structures: dict[str, str], groups: dict | None = None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """The endpoints of the comparison with patRoon: feature recall (P1), area accuracy (P2) and the identification endpoints above."""
    return {"P1": p1(truth, t, ppm, rt_tol), "P2": p2(truth, t, groups, ppm, rt_tol), "ID": identification(truth, t, structures, ppm, rt_tol)}


def tutte(truth: list[dict], t: dict, groups: dict | None = None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    return {"P1": p1(truth, t, ppm, rt_tol), "P2": p2(truth, t, groups, ppm, rt_tol), "P3": p3(truth, t, ppm, rt_tol), "P4": p4(truth, t, ppm, rt_tol),
            "P5": p5(truth, t, ppm, rt_tol), "secondarie": secondarie(truth, t, ppm, rt_tol)}


# ---------------------------------------------------------------------------------------------------------------- smeared peaks (broad bands)
def _intervalli(t: dict) -> tuple[np.ndarray, np.ndarray]:
    """[rt_min, rt_max] of the rows (NaN where the tool gives none)."""
    f = lambda k: np.array([np.nan if r.get(k) is None else r[k] for r in t["righe"]], float)
    return f("rt_min"), f("rt_max")


def _true_arrays(comp: list[dict]) -> tuple:
    """Arrays of the true compounds read by `match_wide` / `wide_window`: m/z, RT, FWHM (min), broad?, rt_min, rt_max."""
    g = lambda c, k: np.nan if c.get(k) is None else c[k]
    return (np.array([c["mz"] for c in comp], float), np.array([c["rt"] for c in comp], float), np.array([g(c, "fwhm_s") for c in comp], float) / 60.0,
            np.array([c["ruolo"] == "broad" for c in comp], bool), np.array([g(c, "rt_min") for c in comp], float), np.array([g(c, "rt_max") for c in comp], float))


def larghi(truth: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Endpoints of the smeared-peak lot, on the broad bands and the narrow peaks of the truth (`ruolo` broad / narrow) matched TOGETHER one-to-one to the rows
    (a row cannot serve a band and a peak on it; `match_wide`: a band accepts |dRT| <= max(tol, 0.35 FWHM) or an interval overlap >= 0.40):
    - P1_narrow, P1_broad: recall (also by morphology);
    - P2_broad: relative error of the mean area of the matched row against `area_attesa` (NaN if no row or no area; summary = median);
    - P3_split: rows emitted per true band = rows that accept the band and are not matched to a narrow peak (0 = lost; summary mean, median and share > 1);
    - P5_fp: rows that no true compound accepts (share of all the rows), e.g. the gradient background."""
    comp = wide_compounds(truth)
    mz, rt, rmin, rmax = (*matrices(t)[:2], *_intervalli(t)) if t["righe"] else (np.zeros(0),) * 4
    cm, cr, fw, wide, c0, c1 = _true_arrays(comp)
    ia, ib = match_wide(cm, cr, fw, wide, c0, c1, mz, rt, rmin, rmax, ppm, rt_tol)
    riga = np.full(len(comp), -1, int)
    riga[ia] = ib
    ok, _ = wide_window(cm, cr, fw, wide, c0, c1, mz, rt, rmin, rmax, ppm, rt_tol) if len(comp) and len(mz) else (np.zeros((len(comp), len(mz)), bool), None)
    narrow_rows = {int(r) for r, b in zip(riga.tolist(), wide.tolist()) if r >= 0 and not b}

    def richiamo(maschera):
        hit = [int(riga[i] >= 0) for i in np.flatnonzero(maschera)]
        morf: dict = {}
        for i in np.flatnonzero(maschera):
            morf.setdefault(str(comp[i].get("morfologia")), []).append(int(riga[i] >= 0))
        return {"per_composto": hit, "richiamo": sum(hit) / len(hit) if hit else None, "n": len(hit),
                "per_morfologia": {k: {"n": len(v), "richiamo": sum(v) / len(v)} for k, v in sorted(morf.items())}}

    _, _, area = matrices(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    err = []
    for i in np.flatnonzero(wide):
        att = np.array([np.nan if v is None else v for v in (comp[i].get("area_attesa") or [])], float)
        sel = att > 0
        err.append(abs(area[riga[i], sel].mean() - att[sel].mean()) / att[sel].mean() if riga[i] >= 0 and sel.any() else np.nan)
    v = [x for x in err if not np.isnan(x)]
    rows_band = [int(sum(1 for r in np.flatnonzero(ok[i]).tolist() if r not in narrow_rows)) for i in np.flatnonzero(wide)]
    spurie = int((~ok.any(0)).sum()) if len(comp) else len(mz)
    return {"P1_narrow": richiamo(~wide), "P1_broad": richiamo(wide),
            "P2_broad": {"per_composto": [None if np.isnan(x) else float(x) for x in err], "mediana": float(np.median(v)) if v else None, "n_misurati": len(v), "n": len(err)},
            "P3_split": {"per_composto": rows_band, "media": float(np.mean(rows_band)) if rows_band else None,
                         "mediana": float(np.median(rows_band)) if rows_band else None, "divise": sum(1 for x in rows_band if x > 1) / len(rows_band) if rows_band else None,
                         "n": len(rows_band)},
            "P5_fp": {"righe": len(mz), "spurie": spurie, "frazione": spurie / len(mz) if len(mz) else None}}
