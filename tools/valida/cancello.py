"""The gate of quality: the pipeline of the site is run natively on real and synthetic series, measured with the metrics of V0 / V1 and the result is written as
`cancello.json` (format `cancello/2`, `cancello/1` also read, read by the panel Pipeline): `vista_candidati` is true only if every criterion that has to be measured passes, and the file
is valid only for the version of the crate that was run (`crate_version`). Aggregated numbers and versions only: no names, no data.

    criteria: {id, valore, soglia, ok}, `ok` null when the criterion was not measured (no data, or by decision: speed and memory are not measured).
    `concordanza_mzmine`: passa | non_passa | non_misurata (optional, apart from `vista_candidati`).

The synthetic series are built in this process (`sintetici_identita`), so their truth carries the heights and the noise peaks that the JSON truth does not."""
from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path

import numpy as np

from . import ROOT, cancello_misure as M, nativo
from .sintetici_mzml import scrivi_file

FORMATO = "cancello/2"
READ_FORMATS = ("cancello/1", "cancello/2")
BASE_TIMOLOL = {"tp": 21, "tp_candidati": 19, "candidati": 494, "gruppi": 25606}      # V0: mzFinder Python (21/23 found, 19 among the candidates, 494, 32 008 - 20 %)
PERFORMANCE = ("prestazioni_13_file", "prestazioni_wasm_file", "prestazioni_memoria_worker")


def crit(id: str, value, threshold, verso: str) -> dict:
    """A criterion: `verso` '>=' | '<=' | '=='; `ok` is None when `value` is None (not measured)."""
    ok = None if value is None else bool({">=": value >= threshold, "<=": value <= threshold, "==": value == threshold}[verso])
    v = None if value is None else (round(float(value), 4) if isinstance(value, (float, np.floating)) else int(value))
    return {"id": id, "ok": ok, "soglia": threshold, "valore": v}


def versione_crate() -> str:
    """Version of the crate `mzlab-feat` (the workspace one: `version.workspace = true`)."""
    t = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version\s*=\s*"([^"]+)"', t, re.M).group(1)


def criteri_timololo(tp: dict | None, n_rows_baseline: int = 32008) -> list[dict]:
    """The four criteria of the real series from `cancello_misure.tp_timololo` (None: series not available)."""
    t = tp or {}
    return [crit("timololo_tp_trovati", t.get("trovati"), BASE_TIMOLOL["tp"], ">="), crit("timololo_tp_nei_candidati", t.get("nei_candidati"), BASE_TIMOLOL["tp_candidati"], ">="),
            crit("timololo_candidati", t.get("candidati"), BASE_TIMOLOL["candidati"], "<="), crit("timololo_gruppi", t.get("righe"), BASE_TIMOLOL["gruppi"], "<=")]


def synthetic_measures(c, series: nativo.Serie, debole=None) -> dict:
    """All the measures of a synthetic series as {name: (value, n)}; `debole`: (c, serie) of the same series with weak products, for the band 3-10 x the noise."""
    m = {}
    m["richiamo_forte"] = M.richiamo(c, series, 10 * M.FLOOR)
    m["richiamo_debole"] = M.richiamo(debole[0], debole[1], 3 * M.FLOOR, 10 * M.FLOOR) if debole else (None, 0)
    m["isomeri"] = M.isomeri_separati(c, series.tabella)
    m["area"] = M.errore_area(c, series)
    m["rumore"] = M.righe_false_rumore(c, series)
    m["famiglie"] = M.famiglie_giuste(c, series)
    m["isf"] = M.isf_senza_ms2(c, series)
    m["esche"] = M.esche_alte(c, series)
    forma, gen, n = M.cinetica(c, series)
    m["cinetica_forma"], m["cinetica_generazione"] = (forma, n), (gen, n)
    med, inside, n = M.purezza(c, series)
    m["purezza"], m["purezza_dentro"] = (med, n), (inside, n)
    return m


def synthetic_criteria(m: dict) -> list[dict]:
    v = {k: x[0] for k, x in m.items()}
    return [crit("sintetici_richiamo_sopra_10x", v["richiamo_forte"], 0.98, ">="), crit("sintetici_richiamo_3x_10x", v["richiamo_debole"], 0.80, ">="),
            crit("sintetici_isomeri_separati", v["isomeri"], 1.0, ">="), crit("sintetici_errore_area_mediano", v["area"], 0.05, "<="),
            crit("sintetici_righe_false_rumore", v["rumore"], 0.01, "<="), crit("sintetici_famiglie_giuste", v["famiglie"], 0.95, ">="),
            crit("sintetici_isf_senza_ms2", v["isf"], 0.0, "<="), crit("sintetici_esche_oltre_livello_4", v["esche"], 0.01, "<="),
            crit("sintetici_cinetica_forma", v["cinetica_forma"], 0.90, ">="), crit("sintetici_cinetica_generazione", v["cinetica_generazione"], 0.90, ">="),
            crit("sintetici_purezza_errore_mediano", v["purezza"], 0.05, "<=")]


def criteri_mzmine(conc: dict | None) -> list[dict]:
    c = conc or {}
    return [crit("mzmine_richiamo", c.get("richiamo"), 0.90, ">="), crit("mzmine_precisione", c.get("precisione"), 0.80, ">="), crit("mzmine_spearman", c.get("spearman"), 0.9, ">="),
            crit("mzmine_log2_mediano", c.get("log2_mediano"), 0.25, "<=")]


def criteri_determinismo(due_esecuzioni, wasm) -> list[dict]:
    return [crit("determinismo_due_esecuzioni", None if due_esecuzioni is None else int(due_esecuzioni), 1, "=="), crit("determinismo_nativo_wasm", None if wasm is None else int(wasm), 1, "==")]


def criteri_prestazioni() -> list[dict]:
    """Speed and memory are not measured (decision of the project): the criteria stay in the file, unmeasured."""
    return [crit(i, None, s, "<=") for i, s in zip(PERFORMANCE, (60, 3, 128))]


ID_RICHIESTI = ("timololo_tp_trovati", "timololo_tp_nei_candidati", "timololo_candidati", "timololo_gruppi", "sintetici_richiamo_sopra_10x", "sintetici_richiamo_3x_10x",
                "sintetici_isomeri_separati", "sintetici_errore_area_mediano", "sintetici_righe_false_rumore", "sintetici_famiglie_giuste", "sintetici_isf_senza_ms2",
                "sintetici_esche_oltre_livello_4", "sintetici_cinetica_forma", "sintetici_cinetica_generazione", "sintetici_purezza_errore_mediano",
                "determinismo_due_esecuzioni", "determinismo_nativo_wasm")


# the table view (rows L1-L7 and exports): the criteria on the aligned rows, their recall and area, and the determinism; the candidate view needs every required criterion
ID_TABELLA = ("timololo_tp_trovati", "timololo_gruppi", "sintetici_richiamo_sopra_10x", "sintetici_richiamo_3x_10x", "sintetici_isomeri_separati", "sintetici_errore_area_mediano",
              "sintetici_righe_false_rumore", "determinismo_due_esecuzioni", "determinismo_nativo_wasm")


def concordanza(criteria: list[dict]) -> str:
    z = [c for c in criteria if c["id"].startswith("mzmine_")]
    if any(c["ok"] is False for c in z):
        return "non_passa"
    return "non_misurata" if any(c["ok"] is None for c in z) else "passa"


def cancello(criteria: list[dict], data: str, crate: str) -> dict:
    """The document `cancello/2`: the table view opens if the `ID_TABELLA` criteria were measured and pass; the candidate view only if every required criterion did (so it implies the table)."""
    outcome = {c["id"]: c["ok"] for c in criteria}
    tabella = all(outcome.get(i) is True for i in ID_TABELLA)
    vista = all(outcome.get(i) is True for i in ID_RICHIESTI)
    return {"concordanza_mzmine": concordanza(criteria), "crate_version": crate, "criteri": criteria, "data": data, "format": FORMATO, "vista_candidati": vista, "vista_tabella": tabella}


def a_json(c: dict) -> str:
    return json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def valido(c: dict | None, crate: str) -> bool:
    """The test of the panel (`PIPE.gateState`) for the candidate view: `cancello/1` or `/2`, `vista_candidati` true and the version of the engine that was run."""
    return bool(c) and c.get("format") in READ_FORMATS and c.get("vista_candidati") is True and str(c.get("crate_version")) == str(crate)


def valido_tabella(c: dict | None, crate: str) -> bool:
    """The test of the panel for the table view: `cancello/2` with `vista_tabella` true; a `cancello/1` has no table of its own and follows `vista_candidati`."""
    if not c or c.get("format") not in READ_FORMATS or str(c.get("crate_version")) != str(crate):
        return False
    return (c.get("vista_tabella") if c["format"] == FORMATO else c.get("vista_candidati")) is True


# ---------------------------------------------------------------------------------------------------------------- synthetic series
def genera_sintetico(seme: int, profilo: str, folder: Path, scala: float | None = None):
    """Writes the mzML and the indexes of a synthetic identity series (the heights of the products times `scala`, if given); returns (truth, index paths in file order)."""
    from . import sintetici_identita as si
    c = si.costruisci(seme, profilo)
    for o in c.ioni:
        if scala is not None and o.ruolo in ("tp1", "tp2", "isomero") and o.amp is not None:
            o.amp = o.amp * scala
    folder.mkdir(parents=True, exist_ok=True)
    mzml = []
    for k, (stem, _, _) in enumerate(c.file):
        p = folder / f"SIN_{stem}.mzML"
        scrivi_file(p, k, c, seme)
        mzml.append(p)
    return c, nativo.indici(mzml, folder)


def esegui_sintetico(seme: int, profilo: str, folder: Path, scala: float | None = None):
    """(truth, native run, index paths, parent) of a synthetic series."""
    c, idx = genera_sintetico(seme, profilo, folder, scala)
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    parent = (nativo.neutra_da_ione(padre.ion), padre.mz, padre.rt)
    return c, nativo.esegui(idx, *parent, folder / "nativo"), idx, parent


def due_esecuzioni(idx: list[str], parent: tuple, folder: Path, prima: nativo.Serie) -> bool:
    """The `.mzfeat` and `.mzfeatf` of a second run have the same SHA-256 as those of the first."""
    return nativo.esegui(idx, *parent, folder / "nativo_bis").sha == prima.sha


def native_against_wasm(idx: list[str], parent: tuple, folder: Path, prima: nativo.Serie) -> bool | None:
    """The digests of the `.wasm` run through Node equal those of the native run; None if cargo (wasm32 target), wasm-bindgen or node are missing."""
    if not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")):
        return None
    run = lambda *a: subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout       # noqa: E731
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg, out = folder / "pkg", folder / "wasm"
    out.mkdir(parents=True, exist_ok=True)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), str(ROOT / "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm"))
    run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, "--parent", parent[0], "--rules", str(nativo.REGOLE), "--parent-mz", repr(parent[1]), "--parent-rt", repr(parent[2]))
    return nativo.leggi_serie(out, [Path(x).name for x in idx]).sha == prima.sha


# ---------------------------------------------------------------------------------------------------------------- the whole measure
def esegui_reale(folder: Path, ioni: list[dict], work: Path):
    """Runs the native pipeline on a real series (the parent, with its structure when the truth gives one, is the one of the truth). Returns (serie of files, run, columns of the treated samples)."""
    from . import tabelle
    series = tabelle.trova_serie(folder)
    file_ = {x["nome"]: Path(folder) / (x["nome"] + ".mzML") for x in series}
    mzml = [p if p.exists() else p.with_suffix(".mzml") for p in file_.values()]
    idx = nativo.indici(mzml, work / "indici")
    padre = next(x for x in ioni if x.get("parent"))
    run = nativo.esegui(idx, nativo.neutra_da_ione(padre["ion"]), padre["mz"], (padre["rt_obs"] or [None])[0], work / "nativo", padre.get("smiles", ""))
    handled = [k for k, x in enumerate(series) if x["time"] and x["time"] > 0 and x["type"] == "sample"]
    return series, run, handled


def serie_reale(folder: Path, ioni: list[dict], work: Path, mzmine: str | None = None):
    """Runs the native pipeline on a real series and measures it. Returns (criteria, details)."""
    series, run, handled = esegui_reale(folder, ioni, work)
    tp = M.tp_timololo(ioni, run, handled)
    crit_, det = criteri_timololo(tp), {"timololo": tp, "crate": run.versione_crate}
    if mzmine:
        from .tabelle import leggi_mzmine
        det["mzmine"] = M.concordanza_mzmine(run.tabella, leggi_mzmine(mzmine, series), handled, 50.0)
    return crit_ + criteri_mzmine(det.get("mzmine")), det


def misura(real_folder: Path | None, ioni: list[dict], mzmine: str | None, profilo: str, seme: int, weak_scale: float | None, work: Path, data: str, wasm: bool = True):
    """The whole gate: real series (if given), synthetic series (`profilo`, with a weak variant if `weak_scale`), determinism. Returns (cancello, details)."""
    det: dict = {"sintetici": {"profilo": profilo, "seme": seme}}
    c, run, idx, parent = esegui_sintetico(seme, profilo, work / "sintetico")
    debole = None
    if weak_scale:
        cd, rd, _, _ = esegui_sintetico(seme, profilo, work / "sintetico_debole", weak_scale)
        debole = (cd, rd)
    m = synthetic_measures(c, run, debole)
    det["sintetici"]["misure"] = {k: {"valore": v[0], "n": v[1]} for k, v in m.items()}
    det["sintetici"]["righe"], det["sintetici"]["candidati"] = len(run.tabella), int(len(run.candidati))
    real, d_real = serie_reale(real_folder, ioni, work / "reale", mzmine) if real_folder else (criteri_timololo(None) + criteri_mzmine(None), {})
    equal = due_esecuzioni(idx, parent, work / "sintetico", run)
    cw = native_against_wasm(idx, parent, work / "sintetico", run) if wasm else None
    criteria = real + synthetic_criteria(m) + criteri_determinismo(equal, cw) + criteri_prestazioni()
    det.update(d_real)
    return cancello(criteria, data, run.versione_crate), det
