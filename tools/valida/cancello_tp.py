"""The gate of the transformation-product (TP) view, `cancello_tp/1` (written as `cancello_tp.json` beside `cancello.json`): the measures of the table §5.2 of the TP
architecture against a truth `verita_tp/1` (or the historical `timolol_truth.json` keys). The gate of quality of the feature table (`cancello.py`) stays as it is.

Without the block `tp/1` the "raw" view is the list of candidates of the pipeline (identities with a proof in favour, in the order of the panel), the starting point
that the TP engine has to beat. Generation, level, region and determinism come from `cancello_tp_motore.py` and need a truth that has the fields (the synthetic V1X one); the
others (decoys, low-resolution, weak recall) and every measure the truth cannot support are not measured (`ok` null) and keep the gate closed; `lr_tp` has no threshold and does not count. Aggregated numbers only: no names, no data."""
from __future__ import annotations

import numpy as np

from . import PPM, RT_TOL
from .cancello import crit
from .cancello_misure import _search
from .cancello_tp_motore import engine_measures
from .nativo import Series

FORMAT = "cancello_tp/1"
THRESHOLDS = {"tp_trovati": 22, "tp_in_vista": 20, "vista_dimensione": 200, "rango_mediano": 10, "rango_90": 40, "isomeri_righe_per_gruppo": 2, "ereditati_come_tp": 0,
          "generazione": 0.9, "regione": 0.8, "livelli": 1.0, "richiamo_deboli": 0.8, "esche": 0.01}
NON_MISURABILI = ("generazione", "regione", "livelli", "richiamo_deboli", "esche")          # `generazione`, `livelli`, `regione` come from cancello_tp_motore (truth with fields)
WITHOUT_THRESHOLD = ("lr_tp",)


def position(series: Series, mz: float, rt: float | None) -> int:
    """Row of the table (identity id when the point is a member of an identity) at (mz, rt); any RT when `rt` is None; -1 if none."""
    tol = mz * PPM * 1e-6
    for i in series.identita:
        for m in i["membri"]:
            if abs(m["mz"] - mz) <= tol and (rt is None or abs(m["rt"] - rt) <= RT_TOL):
                return int(i["id"])
    return _search(series.tabella, mz, rt)


def ranks_view(series: Series, handled_columns: list[int]) -> dict[int, int]:
    """Rank (1-based) of every row of the view. With the block `tp/1` in the table the view is the rows with status 0, in the order kinship, points, largest treated
    area; without it, the raw view: the candidates of the pipeline in the order of the panel (points, then the largest treated area)."""
    amax = series.tabella.massimo(handled_columns)
    points = {int(i["id"]): i["totale"] for i in series.identita}
    if series.tp is not None:
        cand = sorted((int(r) for r in np.flatnonzero(series.tp["stato"] == 0)), key=lambda r: (-int(series.tp["parentela"][r]), -points.get(r, 0.0), -amax[r], r))
        return {r: n for n, r in enumerate(cand, 1)}
    cand = sorted((int(r) for r in series.candidati), key=lambda r: (-points[r], -amax[r], r))
    return {r: n for n, r in enumerate(cand, 1)}


def _rts(x) -> list:
    """The expected RT of an inherited ion: a number, a list (`verita_tp/1`) or nothing (any RT)."""
    return list(x) if isinstance(x, (list, tuple)) and x else [None] if x is None or isinstance(x, (list, tuple)) else [x]


def _positions_tp(t: dict, series: Series) -> list[int]:
    rts = t.get("rt_obs") or [None]
    return [p for p in (position(series, float(t["mz"]), rt) for rt in rts) if p >= 0]


def raw_view_measures(truth: dict, series: Series, handled_columns: list[int]) -> dict:
    """The numbers of the raw view. A truth entry (`tps[]`) counts once: found when any of its RT (any RT if none is known) has a row, in the view when that row is a candidate."""
    rank = ranks_view(series, handled_columns)
    tps = truth.get("tps", [])
    found = in_vista = 0
    ranks = []
    for t in tps:
        pos = _positions_tp(t, series)
        if not pos:
            continue
        found += 1
        vis = [rank[p] for p in pos if p in rank]
        if vis:
            in_vista += 1
            ranks.append(min(vis))
    groups: dict[str, set] = {}
    for r in truth.get("righe_isomeri", []):
        if r.get("rt_obs") is None or r.get("rt_separabile") == "ignoto":
            continue
        p = position(series, float(r["mz"]), float(r["rt_obs"]))
        groups.setdefault(str(r["gruppo"]), set()).add(p if p >= 0 else None)
    rows_group = {g: len({p for p in s if p is not None}) for g, s in sorted(groups.items())}
    eredi = [e for e in truth.get("ereditati", []) if e.get("rapporto_costante")]
    in_view = [e for e in eredi if any(position(series, float(e["mz"]), rt) in rank for rt in _rts(e.get("rt_atteso")))]
    return {"righe": len(series.tabella), "tp_veri": len(tps), "tp_trovati": found, "tp_in_vista": in_vista, "vista_dimensione": len(rank),
            "rango_mediano": float(np.median(ranks)) if ranks else None, "rango_90": float(np.percentile(ranks, 90)) if ranks else None,
            "isomeri_righe_per_gruppo": rows_group,
            "ereditati_costanti": len(eredi), "ereditati_nella_vista": len(in_view),
            "ereditati_come_tp": sum(1 for e in in_view if not e.get("coeluisce_con_tp")),
            "ereditati_coeluiti_nella_vista": sum(1 for e in in_view if e.get("coeluisce_con_tp"))}


def criteria_tp(m: dict | None) -> list[dict]:
    """The criteria of §5.2 from `raw_view_measures` (None: series or truth not available). The ones that need the TP engine stay not measured."""
    m = m or {}
    g = m.get("isomeri_righe_per_gruppo") or {}
    rows_333 = g.get("333")
    out = [crit("tp_trovati", m.get("tp_trovati"), THRESHOLDS["tp_trovati"], ">="), crit("tp_in_vista", m.get("tp_in_vista"), THRESHOLDS["tp_in_vista"], ">="),
           crit("vista_dimensione", m.get("vista_dimensione"), THRESHOLDS["vista_dimensione"], "<="), crit("rango_mediano", m.get("rango_mediano"), THRESHOLDS["rango_mediano"], "<="),
           crit("rango_90", m.get("rango_90"), THRESHOLDS["rango_90"], "<="), crit("isomeri_333_righe", rows_333, THRESHOLDS["isomeri_righe_per_gruppo"], ">="),
           crit("ereditati_come_tp", m.get("ereditati_come_tp"), THRESHOLDS["ereditati_come_tp"], "==")]
    out += [crit(i, m.get(i), THRESHOLDS[i], "<=" if i == "esche" else ">=") for i in NON_MISURABILI]
    out += [crit("determinismo_tp", m.get("determinismo_tp"), 1, "=="), crit("lr_tp", None, None, "==")]
    return out


def cancello_tp(criteria: list[dict], dataset: str, data: str, crate: str) -> dict:
    """The document `cancello_tp/1`: the TP view opens only if every criterion with a threshold was measured and passes."""
    richiesti = [c for c in criteria if c["id"] not in WITHOUT_THRESHOLD]
    vista = all(c["ok"] is True for c in richiesti)
    return {"crate_version": crate, "criteri": criteria, "data": data, "dataset": dataset, "format": FORMAT, "vista_tp": vista}


def misura(truth: dict, series: Series, handled_columns: list[int], dataset: str, data: str, series2: Series | None = None) -> tuple[dict, dict]:
    """Returns (cancello_tp, details) for a run of the native pipeline on a series with truth; `series2` is a second run (determinism of the TP view)."""
    m = raw_view_measures(truth, series, handled_columns)
    motore = engine_measures(truth, series, series2)
    return cancello_tp(criteria_tp({**m, **{k: v for k, v in motore.items() if not k.endswith("_n")}}), dataset, data, series.versione_crate), {"vista_grezza": m, "motore": motore}
