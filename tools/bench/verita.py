"""Truths in the common format: a list of compounds `{id, mz, rt, ruolo, fascia, atteso, area_attesa}` (rt None when the compound has no signal at a known time).

- D2 (synthetic, `verita.json` / rows of `sintetici_identita.righe_verita`) and D3 (real series, `tps` of the truth file): `da_righe`.
- D1 (spike-in with known concentration ratios, the MAF table of MetaboLights): `da_maf`, columns given by name.
Roles: the true features are the compounds with signal (parent, products, isomers, adducts, in-source fragments); the true products are tp1, tp2, isomero;
the decoys are `esca` (hypotheses absent from the data, looked for by m/z only) and `isf_finto` (a fragment that is not one of its parent). Isotopes, chimeric
and background rows are not compounds of the truth: they only explain rows."""
from __future__ import annotations

import csv
import json
from pathlib import Path

RUOLI_FEATURE = ("progenitore", "tp1", "tp2", "isomero", "addotto", "isf")
RUOLI_TP = ("tp1", "tp2", "isomero")
RUOLI_ESCA = ("esca", "isf_finto")
RUOLI_SPIEGANO = RUOLI_FEATURE + ("isotopo", "isf_finto", "chimerica", "coincidenza", "fondo")


def _rts(r: dict) -> list:
    v = r.get("rt_obs", r.get("rt"))
    if v is None or v == "":
        return []
    return [float(x) for x in v] if isinstance(v, (list, tuple)) else [float(v)]


def da_righe(righe: list[dict], ruolo_predefinito: str = "tp1") -> list[dict]:
    """Compounds of the rows of a synthetic truth (`ioni`) or of the `tps` of a real one (no `ruolo`: products). A compound observed at several times gives one
    entry per time (`id`, `id#2`, ...). Rows with `presente` 0 have no RT: they are decoys by m/z. Optional per-row `fascia` and `area_attesa` are kept."""
    out = []
    for r in righe:
        ruolo = r.get("ruolo") or ("progenitore" if r.get("parent") else ruolo_predefinito)
        note = dict(x.split("=", 1) for x in str(r.get("note") or "").split(";") if "=" in x)
        rts = _rts(r) if r.get("presente", 1) else []
        for k, rt in enumerate(rts or [None]):
            out.append({"id": str(r.get("id", r.get("name"))) + ("" if k == 0 else f"#{k + 1}"), "mz": float(r["mz"]), "rt": rt, "ruolo": ruolo,
                        "fascia": r.get("fascia", note.get("fascia")), "atteso": r.get("atteso"), "area_attesa": r.get("area_attesa")})
    return sorted(out, key=lambda c: (c["mz"], c["rt"] if c["rt"] is not None else -1.0, c["id"]))


def da_json(path) -> list[dict]:
    """Truth file of D2 (`ioni`) or D3 (`parent` + `tps`)."""
    d = json.loads(Path(path).read_text(encoding="utf-8"))
    if "ioni" in d:
        return da_righe(d["ioni"])
    return da_righe([dict(d["parent"], ruolo="progenitore")] + list(d["tps"]))


def rapporto_atteso(v) -> float:
    """Expected concentration ratio: a number, or `a\\b` as written in the MAF of D1 (SA\\SB), read as a/b."""
    t = str(v).strip().replace(",", ".")
    if "\\" in t or "/" in t:
        a, b = t.replace("/", "\\").split("\\")
        return float(a) / float(b)
    return float(t)


def da_maf(path, mz: str = "mass_to_charge", rt: str = "retention_time", atteso: str = "expected_ratio", nome: str = "metabolite_identification", rt_secondi: bool = False,
           sep: str = "\t") -> list[dict]:
    """Compounds of the MAF table of D1: one per row with an m/z, an RT and the expected concentration ratio (SA/SB). Rows without a number are skipped."""
    out = []
    with open(path, newline="", encoding="utf-8") as f:
        for i, r in enumerate(csv.DictReader(f, delimiter=sep)):
            try:
                m, t, a = float(r[mz]), float(r[rt]), rapporto_atteso(r[atteso])
            except (KeyError, ValueError, TypeError, ZeroDivisionError):
                continue
            out.append({"id": str(r.get(nome) or f"maf{i}") + f"@{i}", "mz": m, "rt": t / 60.0 if rt_secondi else t, "ruolo": "progenitore", "fascia": None, "atteso": a, "area_attesa": None})
    return sorted(out, key=lambda c: (c["mz"], c["rt"], c["id"]))


def con_ruolo(verita: list[dict], ruoli) -> list[dict]:
    return [c for c in verita if c["ruolo"] in ruoli]


def feature_vere(verita: list[dict]) -> list[dict]:
    """The compounds P1, P2 and P3 are measured on: those with a known RT."""
    return [c for c in con_ruolo(verita, RUOLI_FEATURE) if c["rt"] is not None]


def prodotti(verita: list[dict]) -> list[dict]:
    return [c for c in con_ruolo(verita, RUOLI_TP) if c["rt"] is not None]


def esche(verita: list[dict]) -> list[dict]:
    return con_ruolo(verita, RUOLI_ESCA)
