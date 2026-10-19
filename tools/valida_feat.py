#!/usr/bin/env python3
"""Command line of the quality gates of the native Rust engine (tools/valida/): `--cancello` (feature tables on the synthetic identity series and,
with --serie, a real series), `--cancello-tp` (raw TP view against a truth), `--lr` (low-resolution table view) and `--hr` (high-resolution table view).

    python3 tools/valida_feat.py --cancello [--serie DIR --verita truth.json] [--mzmine table.csv] [--uscita DIR]

The truth is a JSON with the parent ({mz, ion, rt_obs}) and the products ({id, mz, ion, rt_obs[]}); it is never stored in this repository."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from valida import ROOT, rapporto      # noqa: E402


def carica_verita(path) -> list[dict]:
    """Truth ions as [{id, mz, ion, rt_obs[], parent}]: the parent ('parent' key) and the products ('tps' or 'ioni')."""
    d = json.loads(Path(path).read_text(encoding="utf-8"))
    out = []
    if isinstance(d, dict) and d.get("parent"):
        p = d["parent"]
        rt = p.get("rt_obs")
        out.append({"id": str(p.get("name") or "parent"), "mz": float(p["mz"]), "ion": p.get("ion"), "rt_obs": [rt] if isinstance(rt, (int, float)) else list(rt or []), "parent": True, **({"smiles": p["smiles"]} if p.get("smiles") else {})})
    lista = d if isinstance(d, list) else d.get("tps") or d.get("ioni") or []
    for x in lista:
        rt = x.get("rt_obs")
        out.append({"id": str(x["id"]), "mz": float(x["mz"]), "ion": x.get("ion"), "rt_obs": [rt] if isinstance(rt, (int, float)) else list(rt or []), "parent": bool(x.get("parent"))})
    return out


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


def main_lr(a) -> int:
    import datetime
    from valida import lr, nativo
    parent = None
    if a.verita:
        p = next((x for x in carica_verita(a.verita) if x.get("parent")), None)
        if p and p.get("ion"):
            parent = (nativo.neutra_da_ione(p["ion"]), p["mz"], (p["rt_obs"] or [None])[0])
    lavoro = Path(a.lavoro) if a.lavoro else ROOT / ".verifica" / "cancello_lr"
    c, det = lr.misura(a.serie_lr, parent, lavoro, a.data or datetime.date.today().isoformat(), a.seme)
    print(f"vista_tabella_lr: {c['vista_tabella_lr']}  (real data: {det['dati_reali']})")
    for k in c["criteri"]:
        print(f"  {k['id']:<34} {'skipped' if k['ok'] is None else ('ok' if k['ok'] else 'FAIL'):<8} {k['valore']} (threshold {k['soglia']})")
    for p in lr.scrivi(Path(a.uscita) if a.uscita else lavoro, c, det, a.lr_json):
        print(p)
    return 0


def main_hr(a) -> int:
    import datetime
    from valida import hr
    lavoro = Path(a.lavoro) if a.lavoro else ROOT / ".verifica" / "cancello_hr"
    c, det = hr.misura(a.robustezza, a.serie, a.mzmine, lavoro, a.data or datetime.date.today().isoformat())
    print(f"vista_tabella_hr: {c['vista_tabella_hr']}  (DDA files: {det['dati_dda']}; mzmine: {det['csv']})")
    for k in c["criteri"]:
        print(f"  {k['id']:<34} {'skipped' if k['ok'] is None else ('ok' if k['ok'] else 'FAIL'):<8} {k['valore']} (threshold {k['soglia']})")
    for p in hr.scrivi(Path(a.uscita) if a.uscita else lavoro, c, det, a.hr_json):
        print(p)
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--serie", help="folder with the mzML files of a real series (optional for --cancello and --hr, required by --cancello-tp)")
    ap.add_argument("--verita", help="truth JSON (optional)")
    ap.add_argument("--mzmine", help="mzmine CSV export (optional)")
    ap.add_argument("--uscita", help="output folder of the reports (default: the work folder)")
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
    cg.add_argument("--lr", action="store_true", help="measure the low-resolution table view (MRM and full scan; synthetic series always, real ones from --serie-lr or $MZLAB_DATI) and write cancello_lr.json")
    cg.add_argument("--serie-lr", help="folder with the real LR mzML (B_MRM-*, B_FullMass-*, B_MS2-*); default dati/3200 qtrap/mzML of the data repository; --verita gives the parent of the full scan")
    cg.add_argument("--lr-json", help="also write cancello_lr.json here (the tool never commits it)")
    cg.add_argument("--hr", action="store_true", help="measure the high-resolution table view: robustness on the DDA files (--robustezza or dati/HRMS of the data repository) and, with --serie and --mzmine, the agreement with mzmine (D5); writes cancello_hr.json")
    cg.add_argument("--robustezza", help="folder with Orbitrap_{Exploris120,Fusion}_DDApos_*.mzML (no truth)")
    cg.add_argument("--hr-json", help="also write cancello_hr.json here (the tool never commits it)")
    a = ap.parse_args(argv)
    if a.lr:
        return main_lr(a)
    if a.hr:
        return main_hr(a)
    if a.cancello_tp:
        return main_cancello_tp(a)
    if a.cancello:
        return main_cancello(a)
    ap.error("choose one of --cancello, --cancello-tp, --lr, --hr")
    return 2

if __name__ == "__main__":
    sys.exit(main())
