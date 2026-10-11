"""Metrics of a feature table against the truth and against another table. Everything is a pure function of the tables (numpy only, no randomness)."""
from __future__ import annotations

import numpy as np

from . import PPM, RT_TOL
from .abbina import abbina
from .isotopologhi import K, Picco
from .tabelle import Tabella


def righe(tab: Tabella) -> dict:
    per_file = [int(len(tab.presenti(k))) for k in range(len(tab.campioni))]
    return {"righe": int(len(tab)), "per_file": dict(zip(tab.campioni, per_file)), "mediana_per_file": float(np.median(per_file)) if per_file else 0.0}


def _trova_picchi(picchi: list[Picco], tab: Tabella, ppm: float, rt_tol: float) -> dict:
    """{(index of the Picco, k): row of the table}: each reference peak is matched, in its reference file, with the rows present in that file."""
    out = {}
    for f in sorted({p.ref_file for p in picchi if p.ref_file >= 0}):
        righe_f = tab.presenti(f)
        if len(righe_f) == 0:
            continue
        for k in K:
            idx = [i for i, p in enumerate(picchi) if p.ref_file == f and p.area[k, f] > 0]
            if not idx:
                continue
            ia, ib = abbina([picchi[i].mz[k] for i in idx], [picchi[i].apex[k, f] for i in idx], tab.mz[righe_f], tab.rt[righe_f], ppm, rt_tol)
            for a, b in zip(ia.tolist(), ib.tolist()):
                out[(idx[a], k)] = int(righe_f[b])
    return out


def completezza(picchi: list[Picco], tab: Tabella, min_height: float, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Share of the detectable reference peaks (height >= min_height in the reference file) that have a row in the table, for M, M+1, M+2."""
    trovati = _trova_picchi(picchi, tab, ppm, rt_tol)
    out = {}
    for k in K:
        det = [i for i, p in enumerate(picchi) if p.ref_file >= 0 and p.height[k, p.ref_file] >= min_height]
        got = [i for i in det if (i, k) in trovati]
        out[f"M+{k}" if k else "M"] = {"rilevabili": len(det), "trovati": len(got), "frazione": round(len(got) / len(det), 4) if det else None}
    return out


def rapporto_isotopico(picchi: list[Picco], tab: Tabella, min_height: float, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Error of the M+1/M area ratio, relative to the formula (obs / theory - 1), measured on the table rows and, as a baseline, on the reference traces."""
    trovati = _trova_picchi(picchi, tab, ppm, rt_tol)
    err_tab, err_rif = [], []
    for i, p in enumerate(picchi):
        f = p.ref_file
        if f < 0 or p.height[1, f] < min_height or p.area[0, f] <= 0:
            continue
        teo = p.rel[1] / p.rel[0]
        err_rif.append(p.area[1, f] / p.area[0, f] / teo - 1)
        if (i, 0) in trovati and (i, 1) in trovati:
            a0, a1 = tab.area_di(f, trovati[(i, 0)]), tab.area_di(f, trovati[(i, 1)])
            if a0 > 0:
                err_tab.append(float(a1 / a0) / teo - 1)

    def stat(e):
        e = np.array(e, float)
        return {"n": int(len(e)), "mediana": round(float(np.median(e)), 4) if len(e) else None, "mediana_assoluta": round(float(np.median(np.abs(e))), 4) if len(e) else None}
    return {"tabella": stat(err_tab), "riferimento": stat(err_rif)}


def tp_veri(ioni: list[dict], tab: Tabella, colonne_trattate, candidati=None, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """True products (not the parent) found in the aligned table within ppm / RT of the observed RT, with the rank by the largest treated area among all rows
    (`rango_tutte`) and, if `candidati` (rows that survived the filters) is given, among the candidates (`rango_candidati`)."""
    amax = tab.massimo(colonne_trattate)
    ordine_tutte = np.argsort(-amax, kind="stable")
    rango_tutte = np.empty(len(tab), int)
    rango_tutte[ordine_tutte] = np.arange(1, len(tab) + 1)
    rango_cand = {}
    if candidati is not None:
        c = np.asarray(candidati, int)
        for r, g in enumerate(c[np.argsort(-amax[c], kind="stable")].tolist(), 1):
            rango_cand[g] = r
    voci = [(str(x["id"]), float(x["mz"]), float(rt)) for x in ioni if not x.get("parent") for rt in (x.get("rt_obs") or [])]
    senza_rt = [str(x["id"]) for x in ioni if not x.get("parent") and not x.get("rt_obs")]
    if voci:
        ia, ib = abbina([v[1] for v in voci], [v[2] for v in voci], tab.mz, tab.rt, ppm, rt_tol)
        hit = dict(zip(ia.tolist(), ib.tolist()))
    else:
        hit = {}
    dett = []
    for i, (nid, mz, rt) in enumerate(voci):
        g = hit.get(i)
        d = {"id": nid, "mz": mz, "rt": rt, "trovato": g is not None}
        if g is not None:
            d.update(ppm=round((tab.mz[g] - mz) / mz * 1e6, 3), drt=round(float(tab.rt[g] - rt), 4), area_max=float(amax[g]), rango_tutte=int(rango_tutte[g]))
            if candidati is not None:
                d["candidato"] = g in rango_cand
                d["rango_candidati"] = rango_cand.get(g)
        dett.append(d)
    trovati = [d for d in dett if d["trovato"]]
    riass = {"valutabili": len(dett), "trovati": len(trovati), "senza_rt_osservato": senza_rt}
    if candidati is not None:
        riass["nei_candidati"] = sum(1 for d in trovati if d.get("candidato"))
        rc = sorted(d["rango_candidati"] for d in trovati if d.get("rango_candidati"))
        riass["rango_candidati_mediano"] = float(np.median(rc)) if rc else None
        riass["candidati_totali"] = int(len(candidati))
    return {"riassunto": riass, "dettaglio": dett}


def classi_intensita(area_max: np.ndarray, soglia: float) -> np.ndarray:
    """0: < 10 x threshold, 1: 10-100 x, 2: > 100 x (rows below the threshold itself fall in class 0)."""
    return np.digitize(area_max, [10 * soglia, 100 * soglia])


def jaccard(a: Tabella, b: Tabella, colonne, soglia: float, ppm: float = PPM, rt_tol: float = RT_TOL) -> dict:
    """Jaccard index of two aligned tables (rows matched one-to-one within ppm / RT) overall and per intensity class of the row in `a` (of `b` when only there)."""
    ma, mb = a.massimo(colonne), b.massimo(colonne)
    ia, ib = abbina(a.mz, a.rt, b.mz, b.rt, ppm, rt_tol)
    ca, cb = classi_intensita(ma, soglia), classi_intensita(mb, soglia)
    match_b = np.zeros(len(b), bool)
    match_b[ib] = True

    def calc(m_ab, na, nb):
        u = na + nb - m_ab
        return {"comuni": int(m_ab), "solo_a": int(na - m_ab), "solo_b": int(nb - m_ab), "jaccard": round(m_ab / u, 4) if u else None}
    out = {"totale": calc(len(ia), len(a), len(b)), "classi": {}}
    for c, nome in enumerate(("<10x", "10-100x", ">100x")):
        com = int((ca[ia] == c).sum())
        na = int((ca == c).sum())
        nb = int(((cb == c) & ~match_b).sum()) + com           # b's rows alone count in their own class; matched pairs in a's class
        out["classi"][nome] = calc(com, na, nb)
    return out


def righe_nel_bianco(tab: Tabella, colonne_bianco) -> dict | None:
    cols = list(colonne_bianco)
    if not cols or tab.lungo:
        return None
    present = (tab.area > 0).any(1)
    nel = (tab.area[:, cols] > 0).any(1)
    return {"righe_nel_bianco": int(nel.sum()), "righe_con_segnale": int(present.sum()), "frazione": round(float(nel.sum() / max(present.sum(), 1)), 4)}
