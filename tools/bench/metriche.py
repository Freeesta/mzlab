"""Endpoints P1-P5 and secondary numbers of a `bench/1` table against a truth, measured PER COMPOUND (lists aligned with the compounds, so that two tools can be
paired and the compounds resampled). Matching is the deterministic one-to-one of `valida.abbina` (+-ppm, +-RT). Everything is a pure function of its inputs."""
from __future__ import annotations

import numpy as np

from valida.abbina import abbina, abbina_larghi, finestra_larghi

from . import PPM, RT_TOL
from .adatta import matrici
from .verita import RUOLI_FEATURE, RUOLI_SPIEGANO, con_ruolo, esche, feature_vere, prodotti
from .verita import larghi as composti_larghi

LIVELLI_ALTI = ("1", "2a", "2b", "3")             # better than level 4
SOGLIA_RANGO = 0.9                               # share of the products to reach for "candidates to scan"


def _punti(comp: list[dict]) -> tuple[np.ndarray, np.ndarray]:
    return np.array([c["mz"] for c in comp], float), np.array([c["rt"] if c["rt"] is not None else np.nan for c in comp], float)


def riga_di(comp: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> np.ndarray:
    """For each compound the row matched to it one-to-one (-1 if none)."""
    out = np.full(len(comp), -1, int)
    if not comp or not t["righe"]:
        return out
    mz, rt, _ = matrici(t)
    cm, cr = _punti(comp)
    ia, ib = abbina(cm, cr, mz, rt, ppm, rt_tol)
    out[ia] = ib
    return out


def righe_vicine(comp: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> list[np.ndarray]:
    """For each compound all the rows within the tolerance (not one-to-one)."""
    mz, rt, _ = matrici(t)
    return [np.flatnonzero((np.abs(mz - c["mz"]) <= c["mz"] * ppm * 1e-6) & (np.abs(rt - c["rt"]) <= rt_tol)) for c in comp]


def p1(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Recall of the true features: 1 if the compound has at least one matched row. Also by intensity band when the truth gives `fascia`."""
    comp = feature_vere(verita)
    hit = (riga_di(comp, t, ppm, rt_tol) >= 0).astype(int)
    out = {"per_composto": hit.tolist(), "richiamo": float(hit.mean()) if len(hit) else None, "n": len(hit), "per_fascia": {}}
    for f in sorted({c["fascia"] for c in comp if c["fascia"]}):
        v = [h for h, c in zip(hit.tolist(), comp) if c["fascia"] == f]
        out["per_fascia"][str(f)] = {"n": len(v), "richiamo": sum(v) / len(v)}
    return out


def p2(verita: list[dict], t: dict, gruppi: dict | None = None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Quantitative accuracy per compound, NaN where it cannot be measured (no row, or a zero area). With `gruppi` ({"A": [columns], "B": [columns]}) and a
    compound `atteso` (ratio A/B): |log2(observed ratio / expected)| of the group means (D1). Else with `area_attesa` (one value per file): the relative error of
    the mean area over the files with a known value (D2). The summary is the median."""
    comp = feature_vere(verita)
    riga = riga_di(comp, t, ppm, rt_tol)
    _, _, area = matrici(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    err = np.full(len(comp), np.nan)
    for i, (c, r) in enumerate(zip(comp, riga)):
        if r < 0:
            continue
        if gruppi and c.get("atteso"):
            a, b = area[r, gruppi["A"]].mean(), area[r, gruppi["B"]].mean()
            if a > 0 and b > 0 and c["atteso"] > 0:
                err[i] = abs(np.log2(a / b / c["atteso"]))
        elif not gruppi and c.get("area_attesa"):
            att = np.array([np.nan if v is None else v for v in c["area_attesa"]], float)
            ok = att > 0
            if ok.any():
                err[i] = abs(area[r, ok].mean() - att[ok].mean()) / att[ok].mean()
    v = err[~np.isnan(err)]
    return {"per_composto": [None if np.isnan(x) else float(x) for x in err], "mediana": float(np.median(v)) if len(v) else None, "n_misurati": int(len(v)), "n": len(err)}


def p3(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Integrity of the rows: 1 if the compound is represented by exactly one row and that row is not also within the tolerance of another true compound
    (penalises double rows from a bad alignment and merged isomers)."""
    comp = feature_vere(verita)
    vic = righe_vicine(comp, t, ppm, rt_tol)
    uso: dict[int, int] = {}
    for v in vic:
        for r in v.tolist():
            uso[r] = uso.get(r, 0) + 1
    ok = [int(len(v) == 1 and uso[int(v[0])] == 1) for v in vic]
    return {"per_composto": ok, "integrita": sum(ok) / len(ok) if ok else None, "n": len(ok)}


def ordine_candidati(t: dict) -> list[int]:
    """Rows in the order they are scanned: for a table with candidate flags the candidates by points then largest area (the order of the panel); for any other
    table all the rows by largest area. Ties by row number."""
    _, _, area = matrici(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    amax = area.max(1) if area.size else np.zeros(len(t["righe"]))
    righe = t["righe"]
    ha_flag = any(r["candidato"] is not None for r in righe)
    scelte = [i for i, r in enumerate(righe) if (r["candidato"] if ha_flag else True)]
    return sorted(scelte, key=lambda i: (-(righe[i]["punteggio"] or 0.0), -amax[i], i))


def p4(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """True products found among the candidates (flag 1 = matched to a candidate row) and their rank in the scanning order; `per_90`: the fewest candidates to
    scan to reach 90 % of ALL the products (None if never reached)."""
    comp = prodotti(verita)
    riga = riga_di(comp, t, ppm, rt_tol)
    rango = {r: n for n, r in enumerate(ordine_candidati(t), 1)}
    nei = [int(r >= 0 and r in rango) for r in riga.tolist()]
    ranghi = sorted(rango[r] for r in riga.tolist() if r >= 0 and r in rango)
    k = int(np.ceil(SOGLIA_RANGO * len(comp) - 1e-9))
    return {"per_composto": nei, "richiamo_candidati": sum(nei) / len(nei) if nei else None, "n": len(nei), "trovati": int((riga >= 0).sum()), "ranghi": ranghi,
            "per_90": (ranghi[k - 1] if 0 < k <= len(ranghi) else None), "candidati": len(rango)}


def _candidati(t: dict, solo_livelli_alti: bool) -> np.ndarray:
    """True for the rows that are candidates (the flag; a table with no flag offers every row); optionally only those with a level better than 4."""
    out = np.zeros(len(t["righe"]), bool)
    for i, r in enumerate(t["righe"]):
        if r["candidato"] is None or r["candidato"]:
            out[i] = not solo_livelli_alti or r["livello"] is None or r["livello"] in LIVELLI_ALTI
    return out


def p5(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """False products: 1 if a decoy has a candidate row with a level better than 4 (decoys with no signal, `esca`, are looked for by m/z at any RT; `isf_finto`
    by m/z and RT). Precision of the candidates: the share of all the candidate rows that match a true compound."""
    dec = esche(verita)
    mz, rt, _ = matrici(t) if t["righe"] else (np.zeros(0), np.zeros(0), None)
    cand = _candidati(t, True)
    falso = []
    for c in dec:
        dm = np.abs(mz - c["mz"]) <= c["mz"] * ppm * 1e-6
        if c["rt"] is not None:
            dm &= np.abs(rt - c["rt"]) <= rt_tol
        falso.append(int(bool((dm & cand).any())))
    vere = [c for c in verita if c["ruolo"] in RUOLI_FEATURE + ("isotopo",) and c["rt"] is not None]
    spiegata = np.zeros(len(mz), bool)
    for r in (np.concatenate(righe_vicine(vere, t, ppm, rt_tol)) if vere and len(mz) else []):
        spiegata[int(r)] = True
    tutti = _candidati(t, False)
    nc = int(tutti.sum())
    return {"per_composto": falso, "falsi": sum(falso) / len(falso) if falso else None, "n": len(falso), "candidati": nc,
            "precisione_candidati": float((tutti & spiegata).sum() / nc) if nc else None}


def secondarie(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Precision of the feature table (rows that no compound with signal explains), rows per matched compound, products' isomers kept apart."""
    comp = feature_vere(verita)
    vic = righe_vicine(comp, t, ppm, rt_tol)
    spieg = [c for c in verita if c["ruolo"] in RUOLI_SPIEGANO and c["rt"] is not None]
    cov = np.zeros(len(t["righe"]), bool)
    for v in righe_vicine(spieg, t, ppm, rt_tol):
        cov[v] = True
    n = len(t["righe"])
    trovati = [len(v) for v in vic if len(v)]
    iso = [c for c in con_ruolo(verita, ("isomero",)) if c["rt"] is not None]
    ir = riga_di(iso, t, ppm, rt_tol)
    return {"righe": n, "righe_non_spiegate": int(n - cov.sum()), "precisione_righe": float(cov.sum() / n) if n else None,
            "righe_per_composto": float(np.mean(trovati)) if trovati else None, "isomeri_separati": int((ir >= 0).sum()), "isomeri": len(iso)}


def tutte(verita: list[dict], t: dict, gruppi: dict | None = None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    return {"P1": p1(verita, t, ppm, rt_tol), "P2": p2(verita, t, gruppi, ppm, rt_tol), "P3": p3(verita, t, ppm, rt_tol), "P4": p4(verita, t, ppm, rt_tol),
            "P5": p5(verita, t, ppm, rt_tol), "secondarie": secondarie(verita, t, ppm, rt_tol)}


# ---------------------------------------------------------------------------------------------------------------- smeared peaks (broad bands)
def _intervalli(t: dict) -> tuple[np.ndarray, np.ndarray]:
    """[rt_min, rt_max] of the rows (NaN where the tool gives none)."""
    f = lambda k: np.array([np.nan if r.get(k) is None else r[k] for r in t["righe"]], float)
    return f("rt_min"), f("rt_max")


def _vero(comp: list[dict]) -> tuple:
    """Arrays of the true compounds read by `abbina_larghi` / `finestra_larghi`: m/z, RT, FWHM (min), broad?, rt_min, rt_max."""
    g = lambda c, k: np.nan if c.get(k) is None else c[k]
    return (np.array([c["mz"] for c in comp], float), np.array([c["rt"] for c in comp], float), np.array([g(c, "fwhm_s") for c in comp], float) / 60.0,
            np.array([c["ruolo"] == "broad" for c in comp], bool), np.array([g(c, "rt_min") for c in comp], float), np.array([g(c, "rt_max") for c in comp], float))


def larghi(verita: list[dict], t: dict, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Endpoints of the smeared-peak lot, on the broad bands and the narrow peaks of the truth (`ruolo` broad / narrow) matched TOGETHER one-to-one to the rows
    (a row cannot serve a band and a peak on it; `abbina_larghi`: a band accepts |dRT| <= max(tol, 0.35 FWHM) or an interval overlap >= 0.40):
    - P1_narrow, P1_broad: recall (also by morphology);
    - P2_broad: relative error of the mean area of the matched row against `area_attesa` (NaN if no row or no area; summary = median);
    - P3_split: rows emitted per true band = rows that accept the band and are not matched to a narrow peak (0 = lost; summary mean, median and share > 1);
    - P5_fp: rows that no true compound accepts (share of all the rows), e.g. the gradient background."""
    comp = composti_larghi(verita)
    mz, rt, rmin, rmax = (*matrici(t)[:2], *_intervalli(t)) if t["righe"] else (np.zeros(0),) * 4
    cm, cr, fw, larga, c0, c1 = _vero(comp)
    ia, ib = abbina_larghi(cm, cr, fw, larga, c0, c1, mz, rt, rmin, rmax, ppm, rt_tol)
    riga = np.full(len(comp), -1, int)
    riga[ia] = ib
    ok, _ = finestra_larghi(cm, cr, fw, larga, c0, c1, mz, rt, rmin, rmax, ppm, rt_tol) if len(comp) and len(mz) else (np.zeros((len(comp), len(mz)), bool), None)
    del_stretto = {int(r) for r, b in zip(riga.tolist(), larga.tolist()) if r >= 0 and not b}

    def richiamo(maschera):
        hit = [int(riga[i] >= 0) for i in np.flatnonzero(maschera)]
        morf: dict = {}
        for i in np.flatnonzero(maschera):
            morf.setdefault(str(comp[i].get("morfologia")), []).append(int(riga[i] >= 0))
        return {"per_composto": hit, "richiamo": sum(hit) / len(hit) if hit else None, "n": len(hit),
                "per_morfologia": {k: {"n": len(v), "richiamo": sum(v) / len(v)} for k, v in sorted(morf.items())}}

    _, _, area = matrici(t) if t["righe"] else (None, None, np.zeros((0, len(t["file"]))))
    err = []
    for i in np.flatnonzero(larga):
        att = np.array([np.nan if v is None else v for v in (comp[i].get("area_attesa") or [])], float)
        sel = att > 0
        err.append(abs(area[riga[i], sel].mean() - att[sel].mean()) / att[sel].mean() if riga[i] >= 0 and sel.any() else np.nan)
    v = [x for x in err if not np.isnan(x)]
    righe_banda = [int(sum(1 for r in np.flatnonzero(ok[i]).tolist() if r not in del_stretto)) for i in np.flatnonzero(larga)]
    spurie = int((~ok.any(0)).sum()) if len(comp) else len(mz)
    return {"P1_narrow": richiamo(~larga), "P1_broad": richiamo(larga),
            "P2_broad": {"per_composto": [None if np.isnan(x) else float(x) for x in err], "mediana": float(np.median(v)) if v else None, "n_misurati": len(v), "n": len(err)},
            "P3_split": {"per_composto": righe_banda, "media": float(np.mean(righe_banda)) if righe_banda else None,
                         "mediana": float(np.median(righe_banda)) if righe_banda else None, "divise": sum(1 for x in righe_banda if x > 1) / len(righe_banda) if righe_banda else None,
                         "n": len(righe_banda)},
            "P5_fp": {"righe": len(mz), "spurie": spurie, "frazione": spurie / len(mz) if len(mz) else None}}
