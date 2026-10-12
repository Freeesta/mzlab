"""The measures behind the criteria of the gate (`cancello.py`): the real series against the truth, the synthetic identity series against its truth, an mzmine
table against the native one. Pure functions of the tables and of the truth; they return numbers (`None` when there is nothing to measure), never a verdict."""
from __future__ import annotations

import re

import numpy as np

from . import PPM, RT_TOL, metriche
from .abbina import abbina
from .nativo import CELLA_RILEVATA, Serie

FLOOR = 2e4                    # noise floor of the engine (crates/mzlab-feat/src/trace.rs): "10 x the noise" is the minimum height of 2e5
BASI = ("progenitore", "tp1", "tp2", "isomero")
DERIVATI = ("isotopo", "addotto")
SHAPE_TRUTH = {"monotona": (0,), "bateman": (1,), "piatto": (2,), "decrescente": (3,), "ritardo": (0, 1)}      # shape codes of kinetic::Form accepted as right
GEN_TRUTH = {"tp1": 0, "isomero": 0, "tp2": 1}                                                                # generation codes: 0 first, 1 second, 2 undecidable
LIVELLI_ALTI = ("1", "2a", "2b", "3")


def _frazione(n: int, d: int):
    return None if d == 0 else n / d


def _cerca(tab, mz: float, rt: float | None = None):
    """Closest row of the table within the matching tolerance (any RT when `rt` is None), -1 if none."""
    ok = np.abs(tab.mz - mz) <= mz * PPM * 1e-6
    if rt is not None:
        ok &= np.abs(tab.rt - rt) <= RT_TOL
    idx = np.flatnonzero(ok)
    return -1 if len(idx) == 0 else int(idx[np.argmin(np.abs(tab.mz[idx] - mz))])


def altezza_max(o) -> float:
    """Largest height of the most abundant component of a synthetic ion over the files."""
    h = o.amp if o.clip is None else np.minimum(o.amp, o.clip)
    return float(np.max(h))


def tp_timololo(ioni: list[dict], serie: Serie, colonne_trattate: list[int]) -> dict:
    """True products of the real series (not the parent, one entry per observed RT): found (a row at their m/z and RT, or a member of the identity of a row), among
    the candidates (the identity has a proof in favour: the list of the panel), and the median rank among the candidates in the order of the panel (points, then the largest treated area). The baseline is the
    mzFinder Python of V0, which counted rows of the table."""
    tab = serie.tabella
    voci = [(str(x["id"]), float(x["mz"]), float(rt)) for x in ioni if not x.get("parent") for rt in (x.get("rt_obs") or [])]
    amax = tab.massimo(colonne_trattate)
    punti = {int(i["id"]): i["totale"] for i in serie.identita}
    cand = sorted((int(r) for r in serie.candidati), key=lambda r: (-punti[r], -amax[r], r))
    rango = {r: n for n, r in enumerate(cand, 1)}
    trovati = nei_cand = 0
    ranghi = []
    for _, mz, rt in voci:
        r = _cerca(tab, mz, rt)
        i = _identita_di(serie, mz, rt)
        if r < 0 and i is None:
            continue
        trovati += 1
        r = int(i["id"]) if i is not None else r
        if r in rango:
            nei_cand += 1
            ranghi.append(rango[r])
    return {"righe": len(tab), "candidati": len(cand), "valutabili": len(voci), "trovati": trovati, "nei_candidati": nei_cand, "candidati_con_ipotesi": sum(1 for i in serie.identita if i["ipotesi"] is not None and i["totale"] > 0), "rango_candidati_mediano": float(np.median(ranghi)) if ranghi else None}


def _valle(o) -> float:
    m = re.search(r"valle=([0-9.]+)", o.note)
    return float(m.group(1)) if m else 0.0


def ione_riportato(o, serie: Serie, per_id: dict) -> bool:
    """The synthetic ion is in the output: a row of the table at its m/z and RT, or a member of an identity (a row of its family). An isomer fused with its partner
    (valley of 50 % or more: the two cannot be told apart) counts through the row of the partner."""
    cerca = lambda x: _cerca(serie.tabella, x.mz, x.rt) >= 0 or _identita_di(serie, x.mz, x.rt) is not None        # noqa: E731
    if cerca(o):
        return True
    return o.ruolo == "isomero" and _valle(o) >= 0.5 and o.isomero_di in per_id and cerca(per_id[o.isomero_di])


def richiamo(c, serie: Serie, minimo: float, massimo: float = np.inf):
    """Share of the synthetic ions (parent, products, isomers) with a height in [minimo, massimo) that are reported (`ione_riportato`)."""
    ioni = [o for o in c.ioni if o.ruolo in BASI and minimo <= altezza_max(o) < massimo]
    if not ioni:
        return None, 0
    per_id = {o.id: o for o in c.ioni}
    return sum(1 for o in ioni if ione_riportato(o, serie, per_id)) / len(ioni), len(ioni)


def isomeri_separati(c, tab):
    """Pairs of isomers with a valley under 50 % of the lower apex (declared in the truth): share whose two members have each a row of their own."""
    per_id = {o.id: o for o in c.ioni}
    coppie = []
    for o in c.ioni:
        if o.ruolo == "isomero" and o.isomero_di and o.id < o.isomero_di:
            valle = _valle(o)
            altro = per_id[o.isomero_di]
            if valle < 0.5 and min(altezza_max(o), altezza_max(altro)) >= 10 * FLOOR:
                coppie.append((o, altro))
    ok = 0
    for a, b in coppie:
        ia, ib = abbina([a.mz, b.mz], [a.rt, b.rt], tab.mz, tab.rt, PPM, RT_TOL)
        ok += int(len(ia) == 2 and ib[0] != ib[1])
    return _frazione(ok, len(coppie)), len(coppie)


def errore_area(c, serie: Serie):
    """Median relative error of the area of the synthetic ions between files: the area of a row in a file against that of the file where the ion is highest, divided
    by the same ratio of the true heights (the widths do not change, so area follows height). Parent excluded (saturated); only cells detected above 10 x the noise."""
    tab, err = serie.tabella, []
    for o in c.ioni:
        if o.ruolo not in BASI[1:] or o.amp is None:
            continue
        r = _cerca(tab, o.mz, o.rt)
        if r < 0:
            continue
        ref = int(np.argmax(o.amp))
        a_ref = tab.area[r, ref]
        if serie.stato[r, ref] != CELLA_RILEVATA or a_ref <= 0:
            continue
        for k in range(len(o.amp)):
            if k != ref and o.amp[k] >= 10 * FLOOR and serie.stato[r, k] == CELLA_RILEVATA and tab.area[r, k] > 0:
                err.append(abs(tab.area[r, k] / a_ref / (o.amp[k] / o.amp[ref]) - 1))
    return (float(np.median(err)) if err else None), len(err)


def righe_false_rumore(c, serie: Serie):
    """Share of the rows detected in a file with only noise and background (the blank) that match neither a noise peak nor a background ion nor a lobe of it."""
    k = next((i for i, (_, tipo, _) in enumerate(c.file) if tipo == "blank"), None)
    if k is None:
        return None, 0
    tab = serie.tabella
    righe = np.flatnonzero((serie.stato[:, k] == CELLA_RILEVATA) & (tab.area[:, k] > 0))
    if len(righe) == 0:
        return 0.0, 0
    mz_n, rt_n = c.rumore[0], c.rumore[1]
    fondo = np.array([m * (1 + d) for _, m, _, _ in c.fondo for d in (0.0, -2.5e-5, 2.5e-5)])
    falsi = 0
    for r in righe:
        vero_rumore = bool(np.any((np.abs(mz_n - tab.mz[r]) <= tab.mz[r] * 2 * PPM * 1e-6) & (np.abs(rt_n - tab.rt[r]) <= 2 * RT_TOL)))
        vero_fondo = bool(np.any(np.abs(fondo - tab.mz[r]) <= tab.mz[r] * 2 * PPM * 1e-6))
        falsi += int(not (vero_rumore or vero_fondo))
    return falsi / len(righe), len(righe)


def _identita_di(serie: Serie, mz: float, rt: float):
    """The identity that has a member at (mz, rt) (any role), None if none."""
    for i in serie.identita:
        for m in i["membri"]:
            if abs(m["mz"] - mz) <= mz * PPM * 1e-6 and abs(m["rt"] - rt) <= RT_TOL:
                return i
    return None


def famiglie_giuste(c, serie: Serie):
    """Isotopes and adducts of the synthetic ions: share that sit in the same identity as their base ion (when the base has an identity)."""
    per_id = {o.id: o for o in c.ioni}
    giusti = totale = 0
    for o in c.ioni:
        if o.ruolo not in DERIVATI or o.progenitore not in per_id or per_id[o.progenitore].amp is None:
            continue
        base = per_id[o.progenitore]
        ib = _identita_di(serie, base.mz, base.rt)
        if ib is None:
            continue
        totale += 1
        giusti += int(any(abs(m["mz"] - o.mz) <= o.mz * PPM * 1e-6 and abs(m["rt"] - base.rt) <= RT_TOL for m in ib["membri"]))
    return _frazione(giusti, totale), totale


def isf_senza_ms2(c, serie: Serie):
    """In-source fragments of the truth that have no fragment in the MS2 of the parent (`isf_finto`): share declared as ISF of that parent."""
    per_id = {o.id: o for o in c.ioni}
    dichiarati = totale = 0
    for o in c.ioni:
        if o.ruolo != "isf_finto" or o.progenitore not in per_id:
            continue
        base = per_id[o.progenitore]
        ib = _identita_di(serie, base.mz, base.rt)
        if ib is None:
            continue
        totale += 1
        dichiarati += int(any(m["ruolo"] == "isf" and abs(m["mz"] - o.mz) <= o.mz * PPM * 1e-6 for m in ib["membri"]))
    return _frazione(dichiarati, totale), totale


def esche_alte(c, serie: Serie):
    """Decoys (hypotheses of invented rules, no signal): share whose m/z falls in an identity that reaches a level better than 4."""
    esche = [o for o in c.ioni if o.ruolo == "esca"]
    if not esche:
        return None, 0
    alte = 0
    for o in esche:
        alte += int(any(i["livello_max"] in LIVELLI_ALTI and any(abs(m["mz"] - o.mz) <= o.mz * PPM * 1e-6 for m in i["membri"]) for i in serie.identita))
    return alte / len(esche), len(esche)


def _parametro(identita: dict | None, nome: str):
    if identita is None:
        return None
    for p in identita["contributi"]:
        if p["tipo"] == "cinetica":
            return p["parametri"].get(nome)
    return None


def cinetica(c, serie: Serie):
    """Kinetics of the products above 10 x the noise: share with the right shape and share with the right generation (or undecidable). An isomer fused with its
    partner is read through the identity of the partner."""
    forma = gen = totale = 0
    per_id = {o.id: o for o in c.ioni}
    for o in c.ioni:
        if o.ruolo not in GEN_TRUTH or o.amp is None or altezza_max(o) < 10 * FLOOR:
            continue
        i = _identita_di(serie, o.mz, o.rt) or (_identita_di(serie, per_id[o.isomero_di].mz, per_id[o.isomero_di].rt) if o.isomero_di in per_id else None)
        shape, g = _parametro(i, "shape"), _parametro(i, "generation")
        totale += 1
        istantanea = o.forma == "piatto" and shape is not None and int(shape) == 0 and _parametro(i, "coupling") == 3       # formation too fast to resolve: flat
        forma += int(shape is not None and (int(shape) in SHAPE_TRUTH.get(o.forma, ()) or istantanea))
        gen += int(g is not None and int(g) in (GEN_TRUTH[o.ruolo], 2))
    return _frazione(forma, totale), _frazione(gen, totale), totale


def purezza(c, serie: Serie):
    """Purity of the MS2 of the targets of the chimeric spectra: median absolute error and share within 0.05 of the true purity, over the treated files where the
    target is above 10 x the noise and the MS2 is linked."""
    err = []
    for o in c.ioni:
        if o.ruolo not in ("tp1", "tp2") or o.purezza is None or o.amp is None:
            continue
        for k, (f, (_, tipo, _)) in enumerate(zip(serie.per_file, c.file)):
            if tipo != "trat" or o.amp[k] < 10 * FLOOR:
                continue
            ok = (np.abs(f["mz"] - o.mz) <= o.mz * PPM * 1e-6) & (np.abs(f["rt"] - o.rt) <= RT_TOL) & np.isfinite(f["purezza"])
            idx = np.flatnonzero(ok)
            if len(idx):
                err.append(abs(float(f["purezza"][idx[np.argmax(f["altezza"][idx])]]) - o.purezza))
    if not err:
        return None, None, 0
    e = np.array(err)
    return float(np.median(e)), float(np.mean(e <= 0.05)), len(e)


def _ranghi(x: np.ndarray) -> np.ndarray:
    o = np.argsort(x, kind="stable")
    r = np.empty(len(x))
    r[o] = np.arange(len(x))
    return r


def concordanza_mzmine(nativa, mzmine, colonne, soglia_area: float) -> dict:
    """Native table against an mzmine table on the rows of mzmine with a largest treated area of at least 10 x the threshold: recall, precision, Spearman
    correlation of the log areas and median |log2 ratio| after a single scale factor (match within 5 ppm and 0.1 min)."""
    am, an = mzmine.massimo(colonne), nativa.massimo(colonne)
    fm, fn = np.flatnonzero(am >= 10 * soglia_area), np.flatnonzero(an >= 10 * soglia_area)
    if len(fm) == 0 or len(fn) == 0:
        return {"righe_mzmine": len(fm), "righe_native": len(fn)}
    ia, ib = abbina(mzmine.mz[fm], mzmine.rt[fm], nativa.mz[fn], nativa.rt[fn], PPM, 0.1)
    out = {"righe_mzmine": len(fm), "righe_native": len(fn), "abbinate": int(len(ia)), "richiamo": len(ia) / len(fm), "precisione": len(ia) / len(fn)}
    if len(ia) >= 3:
        x, y = np.log(am[fm][ia]), np.log(an[fn][ib])
        out["spearman"] = float(np.corrcoef(_ranghi(x), _ranghi(y))[0, 1])
        d = (y - x) / np.log(2)
        out["log2_mediano"] = float(np.median(np.abs(d - np.median(d))))
    return out
