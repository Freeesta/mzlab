"""The criteria of the TP view that read what the engine says about each product: generation, level, region and determinism (`cancello_tp.criteri_tp` takes them
from here). They need a truth with the fields `gen_attesa`, `level`, `region` (`verita_tp/1`, synthetic series V1X); a truth without them (the historical keys of the
real series) leaves them not measured (`ok` null). Aggregated numbers only.

- `generazione`: share of the detectable products whose kinetic generation (code 0 first, 1 later, 2 undecidable) is the true one or undecidable.
- `livelli`: share of the detectable products whose reported level is not better than the true one (the engine never claims more than the data allow).
- `regione`: share of the products with a localization test whose region the engine gives right; the engine does not write regions yet, so it is not measured.
- `determinismo_tp`: two runs give the same containers, tp/1 arrays included."""
from __future__ import annotations

import re

import numpy as np

from . import PPM, RT_TOL
from .cancello_misure import _identity_of, _parameter
from .nativo import Series

CODE_GENERATION = {1: 0, 2: 1, 3: 1}                 # true generation (1, 2, 3) -> code of the kinetic evidence (0 first, 1 later)


def _identity_tp(t: dict, series: Series):
    """The identity with a member at the mz of the product and any of its RT (the m/z alone if it has none)."""
    mz = float(t["mz"])
    for rt in (t.get("rt_obs") or [None]):
        for i in series.identita:
            if any(abs(m["mz"] - mz) <= mz * PPM * 1e-6 and (rt is None or abs(m["rt"] - rt) <= RT_TOL) for m in i["membri"]):
                return i
    return None


def _level(i: dict | None) -> int | None:
    m = re.match(r"(\d)", str(i.get("livello_max", ""))) if i else None
    return int(m.group(1)) if m else None


def rilevabili(truth: dict, field: str) -> list[dict]:
    """The products of the truth that have `field`, are detectable and are not flagged weak."""
    return [t for t in truth.get("tps", []) if isinstance(t, dict) and t.get(field) is not None and t.get("rilevabile", True) and not t.get("debole")]


def engine_measures(truth: dict, series: Series, series2: Series | None = None) -> dict:
    """The numbers of the criteria; a key is missing when the truth has no field for it, `None` when nothing could be counted."""
    m: dict = {}
    gen = rilevabili(truth, "gen_attesa")
    if gen:
        ok = n = 0
        for t in gen:
            i = _identity_tp(t, series)
            if i is None:
                continue
            g = _parameter(i, "generation")
            n += 1
            ok += int(g is not None and int(g) in (CODE_GENERATION.get(int(t["gen_attesa"]), 1), 2))
        m["generazione"], m["generazione_n"] = (ok / n if n else None), n
    liv = rilevabili(truth, "level")
    if liv:
        ok = n = 0
        for t in liv:
            lv = _level(_identity_tp(t, series))
            if lv is not None:
                n += 1
                ok += int(lv >= int(t["level"]))
        m["livelli"], m["livelli_n"] = (ok / n if n else None), n
    reg = [t for t in truth.get("tps", []) if isinstance(t, dict) and t.get("loc_test") and t.get("region")]
    if reg:
        m["regione"], m["regione_n"] = None, len(reg)
    if series2 is not None:
        equal = series.sha == series2.sha and series.tp is not None and series2.tp is not None and all(np.array_equal(series.tp[k], series2.tp[k]) for k in series.tp)
        m["determinismo_tp"] = int(equal)
    return m
