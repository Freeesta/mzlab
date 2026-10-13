#!/usr/bin/env python3
"""Validation of the high-resolution feature tables (mzFinder Python; optionally an mzmine CSV) with the metrics of tools/valida/.

    python3 tools/valida_feat.py --serie <folder of mzML> [--verita truth.json] [--mzmine table.csv] [--uscita DIR]

Writes <uscita>/rapporto.json and rapporto.md (default: .verifica/valida/<name of the folder>/). The truth is optional, a JSON with the parent
({mz, ion, rt_obs}) and the products ({id, mz, ion, rt_obs[]}); it is never stored in this repository. The JSON is deterministic (two runs give the
same bytes) apart from the "prestazioni" block (time, memory), which `--deterministico` leaves out."""
from __future__ import annotations

import argparse
import json
import resource
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np                                          # noqa: E402

from valida import PPM, ROOT, RT_TOL, isotopologhi, metriche, rapporto, tabelle      # noqa: E402
from mzlab.chem import elements as E                         # noqa: E402


def carica_verita(path) -> list[dict]:
    """Truth ions as [{id, mz, ion, rt_obs[], parent}]: the parent ('parent' key) and the products ('tps' or 'ioni')."""
    d = json.loads(Path(path).read_text(encoding="utf-8"))
    out = []
    if isinstance(d, dict) and d.get("parent"):
        p = d["parent"]
        rt = p.get("rt_obs")
        out.append({"id": str(p.get("name") or "parent"), "mz": float(p["mz"]), "ion": p.get("ion"), "rt_obs": [rt] if isinstance(rt, (int, float)) else list(rt or []), "parent": True})
    lista = d if isinstance(d, list) else d.get("tps") or d.get("ioni") or []
    for x in lista:
        rt = x.get("rt_obs")
        out.append({"id": str(x["id"]), "mz": float(x["mz"]), "ion": x.get("ion"), "rt_obs": [rt] if isinstance(rt, (int, float)) else list(rt or []), "parent": bool(x.get("parent"))})
    return out


def _tabella_metriche(tab, picchi, ioni, colonne_tr, soglia_h, candidati=None, bianchi=()):
    r = {"righe": metriche.righe(tab)}
    if picchi:
        r["completezza"] = metriche.completezza(picchi, tab, soglia_h)
        r["rapporto_isotopico"] = metriche.rapporto_isotopico(picchi, tab, soglia_h)
    if ioni and not tab.lungo:
        r["tp_veri"] = metriche.tp_veri(ioni, tab, colonne_tr, candidati)
    return r


def valida(serie: list[dict], ioni: list[dict] | None = None, mzmine: str | None = None, sensibilita: bool = True, ppm: float = PPM, rt_tol: float = RT_TOL,
           nome: str = "serie") -> dict:
    from tpmine.hr import engine as EN
    from tpmine.hr import filters as FL
    s = EN.DEFAULT_SETTINGS
    t0 = time.perf_counter()
    ioni = ioni or []
    picchi = isotopologhi.picchi_da_verita(ioni)
    parent = next((x for x in ioni if x.get("parent") and x.get("ion")), None)
    ion_parent = E.parse_formula(parent["ion"]) if parent else None
    nf = len(serie)
    tempi, tipi = tabelle.tipi_e_tempi(serie)
    ref_c, tr_c = FL.split_columns(tempi, tipi)
    bianchi = [k for k, x in enumerate(serie) if x["type"] in ("blank", "control")]

    def misura(k, tab):
        isotopologhi.misura_file(tab, picchi, k, nf, s["floor"], ppm)
    es = tabelle.esegui_mzfinder(serie, ion_parent, per_tabella=misura if picchi else None)
    isotopologhi.scegli_riferimento(picchi)
    r = {"serie": {"nome": nome, "file": [{"nome": x["nome"], "time": x["time"], "type": x["type"]} for x in serie]},
         "parametri": {"ppm": ppm, "rt_tol": rt_tol, "floor": s["floor"], "min_height": s["min_height"], "fold": s["fold"], "min_treated": s["min_treated"], "rt_min": s["rt_min"]},
         "verita": {"ioni": len(ioni), "picchi_di_riferimento": len(picchi), "picchi_senza_M": sum(1 for p in picchi if p.ref_file < 0)},
         "tabelle": {}, "funnel": es.passi, "note": ["MSn tree not used: the filter of flat profiles does not protect the in-source fragments of the parent.",
                                                      f"cells filled by gap filling: {es.riempiti}"]}
    sh = s["min_height"]
    r["tabelle"]["mzFinder rilevate"] = _tabella_metriche(es.rilevate, picchi, None, tr_c, sh)
    r["tabelle"]["mzFinder allineata"] = _tabella_metriche(es.allineata, picchi, ioni, tr_c, sh, es.candidati)
    r["tabelle"]["mzFinder riempita"] = _tabella_metriche(es.riempita, picchi, ioni, tr_c, sh, es.candidati)
    r["bianco"] = {"mzFinder allineata": metriche.righe_nel_bianco(es.allineata, bianchi)} if bianchi else {}
    soglia_area = FL.EPS_FACTOR * s["min_height"]
    if mzmine:
        mm = tabelle.leggi_mzmine(mzmine, serie)
        r["tabelle"]["mzmine"] = _tabella_metriche(mm, picchi, ioni, tr_c, sh)
        r["jaccard"] = metriche.jaccard(es.allineata, mm, tr_c, soglia_area, ppm, rt_tol)
        if bianchi:
            r["bianco"]["mzmine"] = metriche.righe_nel_bianco(mm, bianchi)
        r["note"].append(f"Jaccard classes: maximum treated area in multiples of {soglia_area:g} counts x s (EPS_FACTOR x min_height)")
    fasi = dict(es.tempi_s)
    if sensibilita:
        r["sensibilita"] = {}
        for f in (0.8, 1.2):
            t1 = time.perf_counter()
            x = tabelle.esegui_mzfinder(serie, ion_parent, soglie=f)
            tp = metriche.tp_veri(ioni, x.riempita, tr_c, x.candidati)["riassunto"] if ioni else {}
            r["sensibilita"][f"{f:g}"] = {"righe": len(x.allineata), "candidati": int(len(x.candidati)), "tp_trovati": tp.get("trovati"), "tp_nei_candidati": tp.get("nei_candidati")}
            fasi[f"sensibilita_{f:g}"] = time.perf_counter() - t1
    r["prestazioni"] = {"totale_s": time.perf_counter() - t0, "fasi_s": fasi, "memoria_massima_mb": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024}
    return r


def main_cancello(a) -> int:
    import datetime
    from valida import cancello
    ioni = carica_verita(a.verita) if a.verita else []
    lavoro = Path(a.lavoro) if a.lavoro else ROOT / ".verifica" / "cancello"
    c, det = cancello.misura(Path(a.serie) if a.serie else None, ioni, a.mzmine, a.sintetico, a.seme, a.debole, lavoro, a.data or datetime.date.today().isoformat(), not a.senza_wasm)
    for p in rapporto.scrivi_cancello(Path(a.uscita) if a.uscita else lavoro, c, det, a.cancello_json):
        print(p)
    return 0


def main_cancello_tp(a) -> int:
    import datetime
    from valida import cancello, cancello_tp
    if not (a.serie and a.verita):
        raise SystemExit("--cancello-tp needs --serie and --verita")
    verita = json.loads(Path(a.verita).read_text(encoding="utf-8"))
    lavoro = Path(a.lavoro) if a.lavoro else ROOT / ".verifica" / "cancello_tp"
    _, run, trattate = cancello.esegui_reale(Path(a.serie), carica_verita(a.verita), lavoro)
    _, run2, _ = cancello.esegui_reale(Path(a.serie), carica_verita(a.verita), lavoro / "bis")          # a second run: determinism of the TP view
    c, det = cancello_tp.misura(verita, run, trattate, a.dataset, a.data or datetime.date.today().isoformat(), run2)
    for p in rapporto.scrivi_cancello_tp(Path(a.uscita) if a.uscita else lavoro, c, det, a.cancello_tp_json):
        print(p)
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--serie", help="folder with the mzML files of the series (required unless --cancello)")
    ap.add_argument("--verita", help="truth JSON (optional)")
    ap.add_argument("--mzmine", help="mzmine CSV export (optional)")
    ap.add_argument("--uscita", help="output folder (default .verifica/valida/<series folder name>)")
    ap.add_argument("--ppm", type=float, default=PPM)
    ap.add_argument("--rt-tol", type=float, default=RT_TOL)
    ap.add_argument("--senza-sensibilita", action="store_true", help="skip the two reruns with thresholds x0.8 / x1.2")
    ap.add_argument("--deterministico", action="store_true", help="leave time and memory out of the JSON")
    cg = ap.add_argument_group("gate of quality (the native pipeline of the site; the series is optional, without it the real-series criteria are not measured)")
    cg.add_argument("--cancello", action="store_true", help="measure the gate and write cancello.json and cancello_rapporto.{json,md}")
    cg.add_argument("--sintetico", choices=("piccolo", "completo"), default="completo", help="profile of the synthetic identity series")
    cg.add_argument("--seme", type=int, default=7)
    cg.add_argument("--debole", type=float, help="factor on the heights of the products for the weak variant (band 3-10 x the noise)")
    cg.add_argument("--cancello-json", default=str(ROOT / "TP_Mine" / "data" / "cancello.json"))
    cg.add_argument("--data", help="date written in cancello.json (default today)")
    cg.add_argument("--lavoro", help="work folder of the gate (default .verifica/cancello)")
    cg.add_argument("--senza-wasm", action="store_true", help="skip the comparison with the .wasm run through Node")
    cg.add_argument("--cancello-tp", action="store_true", help="measure the raw TP view against the truth (--serie, --verita) and write cancello_tp.json and cancello_tp_rapporto.{json,md}")
    cg.add_argument("--cancello-tp-json", default=str(ROOT / "TP_Mine" / "data" / "cancello_tp.json"))
    cg.add_argument("--dataset", default="D-TIM", help="name of the dataset written in cancello_tp.json")
    a = ap.parse_args(argv)
    if a.cancello_tp:
        return main_cancello_tp(a)
    if a.cancello:
        return main_cancello(a)
    if not a.serie:
        ap.error("--serie is required")
    cartella = Path(a.serie)
    serie = tabelle.trova_serie(cartella)
    ioni = carica_verita(a.verita) if a.verita else []
    rep = valida(serie, ioni, a.mzmine, not a.senza_sensibilita, a.ppm, a.rt_tol, cartella.name)
    out = Path(a.uscita) if a.uscita else ROOT / ".verifica" / "valida" / cartella.name
    j, m = rapporto.scrivi(out, rep, not a.deterministico)
    print(f"{j}\n{m}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
