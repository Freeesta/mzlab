"""Truth `verita_tp/1` of the extended synthetic series V1X (schema in the private data repository, studio/validazione/VERITA_TP_schema.md) and the manifest.

Written next to verita.csv / verita.json by tools/valida/sintetici_identita.py when the profile is extended:
- `verita_tp.json`: dataset D-V1X, `parent`, `progenitori_alogenati`, `tps` (one entry per product, one per group of isomers), `righe_isomeri`, `ereditati`, `esche`,
  `metriche_attese` (counted from the model; the tests count again from the file). A file over 25 KB moves its largest lists to `righe_esterne.json`.
- `manifest.json`: seed, profile, parameters, SHA-256 of every other file.
The rows are read from the ions of the model (`c.ioni`) and from the side data of the extension (`c.vx`)."""
from __future__ import annotations

import hashlib
import json
from collections import Counter
from pathlib import Path

from mzlab.chem import elements as E
from valida.sintetici_esteso import DUE_PASSI, REGOLE, chiave, testo_delta
from valida.sintetici_identita import valle_relativa

LIMITE = 25_000
TIPI = {"isf": "isf", "addotto": "addotto", "isotopo": "isotopologo", "dimero": "dimero"}
RUOLI_TP = ("tp1", "tp2", "tp3")
FWHM = 2.3548200450309493


def delta_ioni(o, pred) -> dict:
    f = dict(E.parse_formula(o.ion))
    for e, n in E.parse_formula(pred.ion).items():
        f[e] = f.get(e, 0) - n
    return {e: n for e, n in f.items() if n}


def alogeni_di(o) -> list:
    f = E.parse_formula(o.ion)
    return sorted(sum(([e] * f[e] for e in ("Br", "Cl") if e in f), []))


def classe_di(delta: dict) -> str | None:
    return next((cl for cl, d in REGOLE if chiave(d) == chiave(delta)), None)


def riga_tp(c, o, per_id: dict, principale) -> dict:
    """One product: change from the ion it forms from, generation, route and the side data of the extension."""
    st = c.vx.meta.get(o.id, {})
    pred = per_id.get(o.progenitore, principale)
    d = delta_ioni(o, pred)
    return {"id": o.id, "mz": round(o.mz, 6), "ion": o.ion, "rt_obs": [round(o.rt, 4)], "change": testo_delta(d), "delta_formula": testo_delta(d), "gen_attesa": o.generazione,
            "pred_atteso": "parent" if pred is principale else pred.id, "gen_origine": "costruita", "region": None, "loc_test": False, "level": 4,
            "rilevabile": bool(o.amp.max() >= 1e6), "classe": st.get("classe") or classe_di(d), "via": st.get("via", "regola" if classe_di(d) else "non_in_tabella"),
            "alogeni": alogeni_di(o), "forma": o.forma, "t_max": st.get("t_max"), "catena": st.get("catena"), "delta_totale": st.get("delta_totale"),
            "raggiungibile_2_passi": st.get("raggiungibile"), "debole": bool(st.get("debole", False)), "isf_di": st.get("isf_di")}


def righe_isomeri(c) -> tuple[list, list]:
    """(rows of the isomers, one product entry per group). The id of a member is `<nominal m/z>-A/B/C`; a nominal m/z met again gets `~n` after the number."""
    gruppi: dict[str, list] = {}
    for o in c.ioni:
        if o.ruolo == "isomero":
            gruppi.setdefault(o.famiglia, []).append(o)
    righe, prodotti, visti = [], [], Counter()
    for fam, mem in gruppi.items():
        mem.sort(key=lambda o: o.rt)
        nom = int(round(mem[0].mz))
        visti[nom] += 1
        base = f"{nom}" if visti[nom] == 1 else f"{nom}~{visti[nom]}"
        d = round(mem[1].rt - mem[0].rt, 4)
        sep = valle_relativa(d, mem[0].sg, 0.8) < 0.98
        for k, o in enumerate(mem):
            righe.append({"id": f"{base}-{'ABC'[k]}", "gruppo": base, "mz": round(o.mz, 6), "ion": o.ion, "rt_obs": round(o.rt, 4), "regione": None, "rt_separabile": d,
                          "separabile_atteso": sep, "fwhm": round(o.sg * FWHM, 4), "origine": o.id, "note": ""})
        prodotti.append((mem[0], base, d))
    return righe, prodotti


def ereditati(c) -> list:
    """Isotopologues, adducts, dimers and true in-source fragments of the progenitor and of the products."""
    st, per_id, out = c.vx, {o.id: o for o in c.ioni}, []
    for o in c.ioni:
        if o.ruolo not in TIPI or o.forma == "isf_variabile" and o.id not in st.ere:
            continue
        padre = per_id.get(o.progenitore)
        extra = st.ere.get(o.id, {})
        note = dict(p.split("=", 1) for p in o.note.split(";") if "=" in p)
        out.append({"id": o.id, "tipo": TIPI[o.ruolo], "ereditato": True, "mz": round(o.mz, 6), "ion": o.ion, "delta": testo_delta(delta_ioni(o, padre)) if padre and o.ion else None,
                    "rt_atteso": [round(o.rt, 4)], "rapporto_costante": extra.get("rapporto_costante", True), "coeluisce_con_tp": extra.get("coeluisce_con_tp"),
                    "rel_atteso": float(note["ratio"]) if "ratio" in note else float(note["rel"]) if "rel" in note else None,
                    "padre": "parent" if padre is not None and padre.ruolo == "progenitore" else o.progenitore, **{k: v for k, v in extra.items() if k not in ("rapporto_costante", "coeluisce_con_tp")}})
    return out


def contatori(c) -> dict:
    """The numbers the tests assert, counted from the ions of the model."""
    st = c.vx
    gen = Counter(o.generazione for o in c.ioni if o.ruolo in RUOLI_TP)
    gen[1] += len({o.famiglia for o in c.ioni if o.ruolo == "isomero"})
    iso = Counter()
    for fam in {o.famiglia for o in c.ioni if o.ruolo == "isomero"}:
        mem = sorted((o for o in c.ioni if o.ruolo == "isomero" and o.famiglia == fam), key=lambda o: o.rt)
        iso[f"{round(mem[1].rt - mem[0].rt, 4):g}"] += 1
    return {"tps_per_generazione": {str(g): n for g, n in sorted(gen.items())}, "catene": {"lunghezza_2": st.cont["catene2"], "lunghezza_3": st.cont["catene3"]},
            "isf_tp": {"costanti": st.cont["isf_costanti"], "variabili": st.cont["isf_variabili"], "travestiti": st.cont["travestiti"]},
            "isomeri_per_delta_rt": dict(sorted(iso.items(), key=lambda kv: float(kv[0]))),
            "alogenati": {"progenitori": {k[5:]: v for k, v in sorted(st.cont.items()) if k.startswith("alog_")},
                          "dealogenati": {k[7:]: v for k, v in sorted(st.cont.items()) if k.startswith("dealog_")}}}


def documento(c, seme: int, profilo: str, budget_mb: float) -> dict:
    per_id = {o.id: o for o in c.ioni}
    principale = next(o for o in c.ioni if o.ruolo == "progenitore")
    righe_iso, gruppi = righe_isomeri(c)
    tps = [riga_tp(c, o, per_id, principale) for o in c.ioni if o.ruolo in RUOLI_TP and o.amp is not None]
    for m, base, d in gruppi:
        r = riga_tp(c, m, per_id, principale)
        r.update(id=base, rt_obs=sorted(round(x.rt, 4) for x in c.ioni if x.ruolo == "isomero" and x.famiglia == m.famiglia), isomeri=base)
        tps.append(r)
    esche = [{"mz": round(o.mz, 6), "rt": round(per_id[o.progenitore].rt, 4)} for o in c.ioni if o.ruolo == "esca"]
    alog = [{"id": o.id, "ion": o.ion, "mz": round(o.mz, 6), "rt_obs": round(o.rt, 4), "formula_neutra": E.fmt(c.vx.neutre[o.id]), "alogeni": c.vx.meta[o.id]["alogeni"]}
            for o in c.ioni if o.ruolo == "progenitore_alog"]
    return {"schema": "verita_tp/1", "dataset": "D-V1X", "stato": "verita", "polarita": "+", "adotto_ref": "[M+H]+",
            "parent": {"id": principale.id, "nome": None, "smiles": None, "ion": principale.ion, "mz": round(principale.mz, 6), "rt_obs": round(principale.rt, 4),
                       "formula_neutra": E.fmt(c.formule[0]), "alogeni": [], "regioni": []},
            "progenitori_alogenati": alog, "tps": tps, "righe_isomeri": righe_iso, "ereditati": ereditati(c), "esche": esche, "regole_ref": "regole_tp/2@fotocatalisi",
            "metriche_attese": contatori(c), "meta": {"seme": seme, "profilo": profilo, "budget_mb": budget_mb, "parametri": c.vx.P},
            "note": "Sintetico: molecole e numeri inventati. `tps` ha un'entry per prodotto e una per gruppo di isomeri (id = numero nominale); `isf_di` = il prodotto coeluisce con l'ISF di quel composto; "
                    "`raggiungibile_2_passi` = la variazione totale dal progenitore e' una o due regole della tabella (falso: serve la rete osservata); nessuna misura di velocita'."}


def dividi(d: dict) -> tuple[dict, dict]:
    """Moves the largest lists to the external file until the document is at most LIMITE bytes."""
    est = {}
    while len(json.dumps(d, ensure_ascii=False, indent=1).encode()) > LIMITE:
        k = max((k for k in ("esche", "ereditati", "tps", "righe_isomeri") if isinstance(d[k], list)), key=lambda k: len(json.dumps(d[k])), default=None)
        if k is None:
            break
        est[k] = d[k]
        d[k] = "righe_esterne"
    return d, est


def scrivi(out: Path, c, seme: int, profilo: str) -> None:
    mzml = sorted(out.glob("*.mzML"))
    budget = round(sum(p.stat().st_size for p in mzml) / 1e6, 1)
    d, est = dividi(documento(c, seme, profilo, budget))
    (out / "verita_tp.json").write_text(json.dumps(d, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    if est:
        (out / "righe_esterne.json").write_text(json.dumps(est, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir()) if p.is_file() and p.name != "manifest.json"}
    (out / "manifest.json").write_text(json.dumps({"seme": seme, "profilo": profilo, "parametri": c.vx.P, "sha256": files}, indent=1, sort_keys=True) + "\n", encoding="utf-8")
