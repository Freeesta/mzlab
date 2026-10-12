"""The gate of the transformation-product (TP) view, `cancello_tp/1` (written as `cancello_tp.json` beside `cancello.json`): the measures of the table §5.2 of the TP
architecture against a truth `verita_tp/1` (or the historical `timolol_truth.json` keys). The gate of quality of the feature table (`cancello.py`) stays as it is.

There is no TP view yet: the "raw" view is the list of candidates of the pipeline (identities with a proof in favour, in the order of the panel), the starting point
that the TP engine has to beat. Measures that need the TP engine (generation, region, level, decoys, low-resolution, determinism, weak recall) are not measured
(`ok` null) and keep the gate closed; `lr_tp` has no threshold and does not count. Aggregated numbers only: no names, no data."""
from __future__ import annotations

import numpy as np

from . import PPM, RT_TOL
from .cancello import crit
from .cancello_misure import _cerca
from .nativo import Serie

FORMATO = "cancello_tp/1"
SOGLIE = {"tp_trovati": 22, "tp_in_vista": 20, "vista_dimensione": 200, "rango_mediano": 10, "rango_90": 40, "isomeri_righe_per_gruppo": 2, "ereditati_come_tp": 0,
          "generazione": 0.9, "regione": 0.8, "livelli": 1.0, "richiamo_deboli": 0.8, "esche": 0.01}
NON_MISURABILI = ("generazione", "regione", "livelli", "richiamo_deboli", "esche")          # they need the TP engine (or the weak synthetic series)
SENZA_SOGLIA = ("lr_tp",)


def posizione(serie: Serie, mz: float, rt: float | None) -> int:
    """Row of the table (identity id when the point is a member of an identity) at (mz, rt); any RT when `rt` is None; -1 if none."""
    tol = mz * PPM * 1e-6
    for i in serie.identita:
        for m in i["membri"]:
            if abs(m["mz"] - mz) <= tol and (rt is None or abs(m["rt"] - rt) <= RT_TOL):
                return int(i["id"])
    return _cerca(serie.tabella, mz, rt)


def ranghi_vista(serie: Serie, colonne_trattate: list[int]) -> dict[int, int]:
    """Rank (1-based) of every candidate in the order of the panel: points, then the largest treated area."""
    amax = serie.tabella.massimo(colonne_trattate)
    punti = {int(i["id"]): i["totale"] for i in serie.identita}
    cand = sorted((int(r) for r in serie.candidati), key=lambda r: (-punti[r], -amax[r], r))
    return {r: n for n, r in enumerate(cand, 1)}


def _posizioni_tp(t: dict, serie: Serie) -> list[int]:
    rts = t.get("rt_obs") or [None]
    return [p for p in (posizione(serie, float(t["mz"]), rt) for rt in rts) if p >= 0]


def misure_vista_grezza(verita: dict, serie: Serie, colonne_trattate: list[int]) -> dict:
    """The numbers of the raw view. A truth entry (`tps[]`) counts once: found when any of its RT (any RT if none is known) has a row, in the view when that row is a candidate."""
    rango = ranghi_vista(serie, colonne_trattate)
    tps = verita.get("tps", [])
    trovati = in_vista = 0
    ranghi = []
    for t in tps:
        pos = _posizioni_tp(t, serie)
        if not pos:
            continue
        trovati += 1
        vis = [rango[p] for p in pos if p in rango]
        if vis:
            in_vista += 1
            ranghi.append(min(vis))
    gruppi: dict[str, set] = {}
    for r in verita.get("righe_isomeri", []):
        if r.get("rt_obs") is None or r.get("rt_separabile") == "ignoto":
            continue
        p = posizione(serie, float(r["mz"]), float(r["rt_obs"]))
        gruppi.setdefault(str(r["gruppo"]), set()).add(p if p >= 0 else None)
    righe_gruppo = {g: len({p for p in s if p is not None}) for g, s in sorted(gruppi.items())}
    eredi = [e for e in verita.get("ereditati", []) if e.get("rapporto_costante")]
    nella_vista = [e for e in eredi if posizione(serie, float(e["mz"]), e.get("rt_atteso")) in rango]
    return {"righe": len(serie.tabella), "tp_veri": len(tps), "tp_trovati": trovati, "tp_in_vista": in_vista, "vista_dimensione": len(rango),
            "rango_mediano": float(np.median(ranghi)) if ranghi else None, "rango_90": float(np.percentile(ranghi, 90)) if ranghi else None,
            "isomeri_righe_per_gruppo": righe_gruppo,
            "ereditati_costanti": len(eredi), "ereditati_nella_vista": len(nella_vista),
            "ereditati_come_tp": sum(1 for e in nella_vista if not e.get("coeluisce_con_tp")),
            "ereditati_coeluiti_nella_vista": sum(1 for e in nella_vista if e.get("coeluisce_con_tp"))}


def criteri_tp(m: dict | None) -> list[dict]:
    """The criteria of §5.2 from `misure_vista_grezza` (None: series or truth not available). The ones that need the TP engine stay not measured."""
    m = m or {}
    g = m.get("isomeri_righe_per_gruppo") or {}
    righe_333 = g.get("333")
    out = [crit("tp_trovati", m.get("tp_trovati"), SOGLIE["tp_trovati"], ">="), crit("tp_in_vista", m.get("tp_in_vista"), SOGLIE["tp_in_vista"], ">="),
           crit("vista_dimensione", m.get("vista_dimensione"), SOGLIE["vista_dimensione"], "<="), crit("rango_mediano", m.get("rango_mediano"), SOGLIE["rango_mediano"], "<="),
           crit("rango_90", m.get("rango_90"), SOGLIE["rango_90"], "<="), crit("isomeri_333_righe", righe_333, SOGLIE["isomeri_righe_per_gruppo"], ">="),
           crit("ereditati_come_tp", m.get("ereditati_come_tp"), SOGLIE["ereditati_come_tp"], "==")]
    out += [crit(i, None, SOGLIE[i], "<=" if i == "esche" else ">=") for i in NON_MISURABILI]
    out += [crit("determinismo_tp", None, 1, "=="), crit("lr_tp", None, None, "==")]
    return out


def cancello_tp(criteri: list[dict], dataset: str, data: str, crate: str) -> dict:
    """The document `cancello_tp/1`: the TP view opens only if every criterion with a threshold was measured and passes."""
    richiesti = [c for c in criteri if c["id"] not in SENZA_SOGLIA]
    vista = all(c["ok"] is True for c in richiesti)
    return {"crate_version": crate, "criteri": criteri, "data": data, "dataset": dataset, "format": FORMATO, "vista_tp": vista}


def misura(verita: dict, serie: Serie, colonne_trattate: list[int], dataset: str, data: str) -> tuple[dict, dict]:
    """Returns (cancello_tp, details) for a run of the native pipeline on a series with truth."""
    m = misure_vista_grezza(verita, serie, colonne_trattate)
    return cancello_tp(criteri_tp(m), dataset, data, serie.versione_crate), {"vista_grezza": m}
