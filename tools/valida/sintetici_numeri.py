#!/usr/bin/env python3
"""Starting numbers of the synthetic identity series: mzFinder Python (V0) run on the files, recall by role of the truth, rows, candidates, decoys found.

    python3 tools/valida/sintetici_numeri.py --serie DIR [--uscita FILE.json]

DIR holds the files written by sintetici_identita.py (mzML + verita.json). A truth row is found when the aligned and gap-filled table has a row within +-5 ppm
and +-0.15 min (one-to-one matching, `abbina`); it is a candidate when that row survives the filters. Decoys (esca) have no signal, so they are looked for by m/z
only: a hit is a row of the table within +-5 ppm at any RT. The numbers are measures, not requirements: they are the starting point of the packages that follow."""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from valida import PPM, RT_TOL, tabelle                  # noqa: E402
from valida.abbina import abbina                         # noqa: E402
from mzlab.chem import elements as E                     # noqa: E402

CON_RT = ("progenitore", "tp1", "tp2", "tp3", "progenitore_alog", "isomero", "isotopo", "addotto", "dimero", "isf", "isf_finto", "chimerica", "coincidenza")


def ricerca_mz(tab, mz) -> np.ndarray:
    """For each m/z the index of the closest table row within +-PPM at any RT, -1 if none."""
    o = np.argsort(tab.mz)
    s = tab.mz[o]
    i = np.clip(np.searchsorted(s, mz), 1, len(s) - 1)
    best = np.where(np.abs(s[i] - mz) < np.abs(s[i - 1] - mz), i, i - 1)
    ok = np.abs(s[best] - mz) / mz <= PPM * 1e-6
    return np.where(ok, o[best], -1)


def per_ruolo(righe: list[dict], tab, cand: set) -> dict:
    out = {}
    for ruolo in CON_RT:
        r = [x for x in righe if x["ruolo"] == ruolo and x["presente"] and x["rt_obs"]]
        if not r:
            continue
        ia, ib = abbina([x["mz"] for x in r], [x["rt_obs"][0] for x in r], tab.mz, tab.rt, PPM, RT_TOL)
        out[ruolo] = {"righe": len(r), "trovate": int(len(ia)), "candidate": int(sum(1 for j in ib if int(j) in cand))}
    esche = [x for x in righe if x["ruolo"] == "esca"]
    if esche:
        idx = ricerca_mz(tab, np.array([x["mz"] for x in esche]))
        out["esca"] = {"righe": len(esche), "con_segnale_a_qualunque_RT": int((idx >= 0).sum()), "candidate": int(sum(1 for j in idx if j >= 0 and int(j) in cand))}
    return out


def numeri(cartella, nome: str | None = None) -> dict:
    cartella = Path(cartella)
    d = json.loads((cartella / "verita.json").read_text(encoding="utf-8"))
    t0 = time.perf_counter()
    serie = tabelle.trova_serie(cartella)
    es = tabelle.esegui_mzfinder(serie, E.parse_formula(d["parent"]["ion"]))
    cand = {int(i) for i in es.candidati}
    tab = es.riempita
    return {"serie": nome or cartella.name, "file": len(serie), "profilo": d["meta"]["profilo"], "righe_rilevate": len(es.rilevate), "righe_allineate": len(tab),
            "candidati": len(cand), "riempiti": es.riempiti, "per_ruolo": per_ruolo(d["ioni"], tab, cand), "coincidenze": d["meta"]["coincidenze"],
            "imbuto": [{"passo": p["text"], "n": p["n"]} for p in es.passi], "tempo_s": round(time.perf_counter() - t0, 1)}


def markdown(r: dict) -> str:
    L = [f"# {r['serie']} ({r['profilo']}, {r['file']} file)", "", f"Righe rilevate (tutti i file) {r['righe_rilevate']}; righe allineate {r['righe_allineate']}; candidati {r['candidati']}; tempo {r['tempo_s']} s.", "",
         "| ruolo | righe | trovate | candidate |", "|---|---|---|---|"]
    for k, v in r["per_ruolo"].items():
        L.append(f"| {k} | {v['righe']} | {v.get('trovate', v.get('con_segnale_a_qualunque_RT'))} | {v['candidate']} |")
    c = r["coincidenze"]
    L += ["", f"Coincidenze: {c['osservate']} osservate, {c['attese']} attese per caso ({c['n_ipotesi']} ipotesi, {c['n_rumore']} picchi di rumore).", "", "Imbuto dei filtri:"]
    return "\n".join(L + [f"- {p['passo']}: {p['n']}" for p in r["imbuto"]]) + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--serie", required=True)
    ap.add_argument("--uscita")
    a = ap.parse_args(argv)
    r = numeri(a.serie)
    if a.uscita:
        Path(a.uscita).write_text(json.dumps(r, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(markdown(r))
    return 0


if __name__ == "__main__":
    sys.exit(main())
