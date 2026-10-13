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
from .cancello_misure import _identita_di, _parametro
from .nativo import Serie

CODICE_GENERAZIONE = {1: 0, 2: 1, 3: 1}                 # true generation (1, 2, 3) -> code of the kinetic evidence (0 first, 1 later)


def _identita_tp(t: dict, serie: Serie):
    """The identity with a member at the mz of the product and any of its RT (the m/z alone if it has none)."""
    mz = float(t["mz"])
    for rt in (t.get("rt_obs") or [None]):
        for i in serie.identita:
            if any(abs(m["mz"] - mz) <= mz * PPM * 1e-6 and (rt is None or abs(m["rt"] - rt) <= RT_TOL) for m in i["membri"]):
                return i
    return None


def _livello(i: dict | None) -> int | None:
    m = re.match(r"(\d)", str(i.get("livello_max", ""))) if i else None
    return int(m.group(1)) if m else None


def rilevabili(verita: dict, campo: str) -> list[dict]:
    """The products of the truth that have `campo`, are detectable and are not flagged weak."""
    return [t for t in verita.get("tps", []) if isinstance(t, dict) and t.get(campo) is not None and t.get("rilevabile", True) and not t.get("debole")]


def misure_motore(verita: dict, serie: Serie, serie2: Serie | None = None) -> dict:
    """The numbers of the criteria; a key is missing when the truth has no field for it, `None` when nothing could be counted."""
    m: dict = {}
    gen = rilevabili(verita, "gen_attesa")
    if gen:
        ok = n = 0
        for t in gen:
            i = _identita_tp(t, serie)
            if i is None:
                continue
            g = _parametro(i, "generation")
            n += 1
            ok += int(g is not None and int(g) in (CODICE_GENERAZIONE.get(int(t["gen_attesa"]), 1), 2))
        m["generazione"], m["generazione_n"] = (ok / n if n else None), n
    liv = rilevabili(verita, "level")
    if liv:
        ok = n = 0
        for t in liv:
            lv = _livello(_identita_tp(t, serie))
            if lv is not None:
                n += 1
                ok += int(lv >= int(t["level"]))
        m["livelli"], m["livelli_n"] = (ok / n if n else None), n
    reg = [t for t in verita.get("tps", []) if isinstance(t, dict) and t.get("loc_test") and t.get("region")]
    if reg:
        m["regione"], m["regione_n"] = None, len(reg)
    if serie2 is not None:
        uguale = serie.sha == serie2.sha and serie.tp is not None and serie2.tp is not None and all(np.array_equal(serie.tp[k], serie2.tp[k]) for k in serie.tp)
        m["determinismo_tp"] = int(uguale)
    return m
