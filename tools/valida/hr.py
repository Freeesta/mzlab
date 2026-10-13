"""The high-resolution table view (`vista_tabella_hr`) on files without a truth and on the agreement with mzmine (D5).

Robustness, on every file of a folder (DDA Orbitrap Exploris/Fusion, run one by one): the pipeline runs and finds rows, two runs give the same bytes, a
threshold is proposed (`noise.floor` of the preview), no cell has an incoherent state. Agreement with mzmine (only if its CSV is given): recall >= 0.90,
precision >= 0.80, Spearman >= 0.9, median |log2| <= 0.25. Without files or CSV the criteria are skipped (`ok` null), never passed.
Aggregated numbers only; the document is `cancello_hr/1`, written in the work folder and never committed by this tool."""
from __future__ import annotations

import json
import os
from pathlib import Path

import numpy as np

from . import cancello, cancello_misure as M, hr_nativo as H, nativo
from .tabelle import Tabella, leggi_mzmine

FORMATO = "cancello_hr/1"
ID_ROBUSTEZZA = ("hr_robustezza_gira", "hr_robustezza_deterministica", "hr_robustezza_soglia", "hr_robustezza_stati_coerenti")
ID_MZMINE = ("mzmine_richiamo", "mzmine_precisione", "mzmine_spearman", "mzmine_log2_mediano")
SOGLIA_AREA = 50.0             # counts x s, as the gate of the candidate view (`cancello.serie_reale`)


def stati_coerenti(stato: np.ndarray, area: np.ndarray) -> tuple[int, int]:
    """(incoherent, total) cells: detected, targeted or filled = finite area >= 0 (detected: > 0); absent = no area. States out of 0-3 are incoherent."""
    ok = (((stato == 0) & np.isfinite(area) & (area > 0)) | (np.isin(stato, (1, 2)) & np.isfinite(area) & (area >= 0)) | ((stato == 3) & (~np.isfinite(area) | (area == 0))))
    return int((~ok).sum()), int(stato.size)


def un_file(mzidx: str, lavoro: Path) -> dict:
    """The robustness measures of one file: {gira, righe, deterministica, soglia, incoerenti, celle}; a run that fails is `gira` false (the error is kept)."""
    r: dict = {"gira": False, "righe": 0, "deterministica": False, "soglia": None, "incoerenti": 0, "celle": 0}
    try:
        a = H.esegui_senza_genitore([mzidx], lavoro / "a")
        b = H.esegui_senza_genitore([mzidx], lavoro / "b")
        r.update(righe=len(a["tabella"]), gira=len(a["tabella"]) > 0, deterministica=a["sha"] == b["sha"])
        r["incoerenti"], r["celle"] = stati_coerenti(a["stato"], a["area_cruda"])
        r["soglia"] = (H.anteprima(mzidx).get("noise") or {}).get("floor")
    except Exception as e:                                  # noqa: BLE001 - a robustness measure reports the failure, it does not stop
        r["errore"] = f"{type(e).__name__}: {str(e)[:120]}"
    return r


def criteri_robustezza(misure: list[dict] | None) -> list[dict]:
    """The four robustness criteria over the files measured (fractions that must be 1); None or no file: not measured."""
    if not misure:
        return [cancello.crit(i, None, 1.0, ">=") for i in ID_ROBUSTEZZA]
    f = lambda k: sum(1 for m in misure if k(m)) / len(misure)             # noqa: E731
    celle = sum(m["celle"] for m in misure)
    return [cancello.crit("hr_robustezza_gira", f(lambda m: m["gira"]), 1.0, ">="), cancello.crit("hr_robustezza_deterministica", f(lambda m: m["gira"] and m["deterministica"]), 1.0, ">="),
            cancello.crit("hr_robustezza_soglia", f(lambda m: m["soglia"] is not None and np.isfinite(m["soglia"]) and m["soglia"] > 0), 1.0, ">="),
            cancello.crit("hr_robustezza_stati_coerenti", 1.0 - sum(m["incoerenti"] for m in misure) / celle if celle else None, 1.0, ">=")]


def concordanza_d5(nativa: Tabella, csv: str | None, colonne: list[int]) -> tuple[list[dict], dict]:
    """The four mzmine criteria of the native table against the CSV of mzmine (columns of the files of `nativa`); no CSV: not measured."""
    if not csv:
        return cancello.criteri_mzmine(None), {}
    serie = [{"nome": c} for c in nativa.campioni]
    d = M.concordanza_mzmine(nativa, leggi_mzmine(csv, serie), colonne, SOGLIA_AREA)
    return cancello.criteri_mzmine(d), d


def trova_hr(cartella: str | None) -> Path | None:
    """The folder of the DDA files: `--robustezza`, else `$MZLAB_DATI/HRMS/Esempio raw e mzML`; None when it does not exist."""
    base = Path(cartella) if cartella else (Path(os.environ["MZLAB_DATI"]) / "HRMS" / "Esempio raw e mzML" if os.environ.get("MZLAB_DATI") else None)
    return base if base and base.is_dir() else None


def robustezza(cartella: Path | None, lavoro: Path, motivo=("Exploris120", "Fusion")) -> tuple[list[dict], dict]:
    """The robustness of the files `Orbitrap_{Exploris120,Fusion}_DDApos_*.mzML` of `cartella`: (measures per file as aggregates, details)."""
    files = sorted(p for k in motivo for p in (cartella.glob(f"Orbitrap_{k}_DDApos_*.mzML") if cartella else []))
    if not files:
        return [], {}
    misure = [un_file(i, lavoro / f"f{k}") for k, i in enumerate(nativo.indici(files, lavoro / "indici"))]
    return misure, {"file": len(files), "righe": [m["righe"] for m in misure], "errori": [m["errore"] for m in misure if "errore" in m]}


def d5(cartella: str | None, csv: str | None, lavoro: Path) -> tuple[list[dict], dict]:
    """The mzmine criteria on a series folder (`cartella`, the files in the order of `tabelle.trova_serie`) and its CSV; either missing: not measured."""
    if not (cartella and csv and Path(cartella).is_dir()):
        return cancello.criteri_mzmine(None), {}
    from . import tabelle
    serie = tabelle.trova_serie(cartella)
    idx = nativo.indici([Path(x["path"]) for x in serie], lavoro / "indici")
    run = H.esegui_senza_genitore(idx, lavoro / "nativo")
    trattate = [k for k, x in enumerate(serie) if x["time"] and x["time"] > 0 and x["type"] == "sample"]
    run["tabella"].campioni = [x["nome"] for x in serie]
    return concordanza_d5(run["tabella"], csv, trattate)


def vista_tabella_hr(criteri: list[dict]) -> bool:
    """True when the robustness criteria were measured and pass, and the mzmine ones too if a CSV was given (all four measured, else all four skipped)."""
    esito = {c["id"]: c["ok"] for c in criteri}
    if not all(esito.get(i) is True for i in ID_ROBUSTEZZA):
        return False
    z = [esito.get(i) for i in ID_MZMINE]
    return all(v is None for v in z) or all(v is True for v in z)


def documento(criteri: list[dict], data: str, crate: str) -> dict:
    return {"concordanza_mzmine": cancello.concordanza(criteri), "criteri": criteri, "crate_version": crate, "data": data, "format": FORMATO, "vista_tabella_hr": vista_tabella_hr(criteri)}


def misura(cartella_dda: str | None, serie: str | None, csv: str | None, lavoro: Path, data: str) -> tuple[dict, dict]:
    """Robustness of the DDA files and, with a CSV, the agreement with mzmine. Returns (document, details)."""
    base = trova_hr(cartella_dda)
    misure, dr = robustezza(base, lavoro / "robustezza")
    cz, dz = d5(serie, csv, lavoro / "d5")
    det = {"robustezza": dr, "mzmine": dz, "dati_dda": "letti" if misure else "saltato: nessun file (MZLAB_DATI o --robustezza)",
           "csv": "dato" if csv and serie else "saltato: serve --serie e --mzmine"}
    return documento(criteri_robustezza(misure) + cz, data, cancello.versione_crate()), det


def markdown(c: dict, det: dict) -> str:
    riga = lambda k: f"| `{k['id']}` | {'saltato' if k['ok'] is None else ('passato' if k['ok'] else 'NON PASSATO')} | {k['valore']} | {k['soglia']} |"      # noqa: E731
    out = [f"# High-resolution table view ({c['data']}, crate {c['crate_version']})", "",
           f"`vista_tabella_hr`: **{'open' if c['vista_tabella_hr'] else 'closed'}**; DDA files: {det['dati_dda']}; mzmine: {det['csv']}", "",
           "| criterion | result | value | threshold |", "|---|---|---|---|"] + [riga(k) for k in c["criteri"]] + ["", "## Details", "```json",
           json.dumps(det, indent=1, sort_keys=True, ensure_ascii=False, default=str), "```", ""]
    return "\n".join(out)


def scrivi(uscita: Path, c: dict, det: dict, json_path: str | None = None) -> list[Path]:
    uscita.mkdir(parents=True, exist_ok=True)
    pj, pm = uscita / "cancello_hr.json", uscita / "cancello_hr_rapporto.md"
    txt = json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    pj.write_text(txt, encoding="utf-8")
    pm.write_text(markdown(c, det), encoding="utf-8")
    if json_path:
        Path(json_path).write_text(txt, encoding="utf-8")
    return [pj, pm]
