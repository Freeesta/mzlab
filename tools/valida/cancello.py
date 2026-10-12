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
FORMATI_LETTI = ("cancello/1", "cancello/2")
BASE_TIMOLOLO = {"tp": 21, "tp_candidati": 19, "candidati": 494, "gruppi": 25606}      # V0: mzFinder Python (21/23 found, 19 among the candidates, 494, 32 008 - 20 %)
PRESTAZIONI = ("prestazioni_13_file", "prestazioni_wasm_file", "prestazioni_memoria_worker")


def crit(id: str, valore, soglia, verso: str) -> dict:
    """A criterion: `verso` '>=' | '<=' | '=='; `ok` is None when `valore` is None (not measured)."""
    ok = None if valore is None else bool({">=": valore >= soglia, "<=": valore <= soglia, "==": valore == soglia}[verso])
    v = None if valore is None else (round(float(valore), 4) if isinstance(valore, (float, np.floating)) else int(valore))
    return {"id": id, "ok": ok, "soglia": soglia, "valore": v}


def versione_crate() -> str:
    """Version of the crate `mzlab-feat` (the workspace one: `version.workspace = true`)."""
    t = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version\s*=\s*"([^"]+)"', t, re.M).group(1)


def criteri_timololo(tp: dict | None, n_righe_baseline: int = 32008) -> list[dict]:
    """The four criteria of the real series from `cancello_misure.tp_timololo` (None: series not available)."""
    t = tp or {}
    return [crit("timololo_tp_trovati", t.get("trovati"), BASE_TIMOLOLO["tp"], ">="), crit("timololo_tp_nei_candidati", t.get("nei_candidati"), BASE_TIMOLOLO["tp_candidati"], ">="),
            crit("timololo_candidati", t.get("candidati"), BASE_TIMOLOLO["candidati"], "<="), crit("timololo_gruppi", t.get("righe"), BASE_TIMOLOLO["gruppi"], "<=")]


def misure_sintetiche(c, serie: nativo.Serie, debole=None) -> dict:
    """All the measures of a synthetic series as {name: (value, n)}; `debole`: (c, serie) of the same series with weak products, for the band 3-10 x the noise."""
    m = {}
    m["richiamo_forte"] = M.richiamo(c, serie, 10 * M.FLOOR)
    m["richiamo_debole"] = M.richiamo(debole[0], debole[1], 3 * M.FLOOR, 10 * M.FLOOR) if debole else (None, 0)
    m["isomeri"] = M.isomeri_separati(c, serie.tabella)
    m["area"] = M.errore_area(c, serie)
    m["rumore"] = M.righe_false_rumore(c, serie)
    m["famiglie"] = M.famiglie_giuste(c, serie)
    m["isf"] = M.isf_senza_ms2(c, serie)
    m["esche"] = M.esche_alte(c, serie)
    forma, gen, n = M.cinetica(c, serie)
    m["cinetica_forma"], m["cinetica_generazione"] = (forma, n), (gen, n)
    med, dentro, n = M.purezza(c, serie)
    m["purezza"], m["purezza_dentro"] = (med, n), (dentro, n)
    return m


def criteri_sintetici(m: dict) -> list[dict]:
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
    return [crit(i, None, s, "<=") for i, s in zip(PRESTAZIONI, (60, 3, 128))]


ID_RICHIESTI = ("timololo_tp_trovati", "timololo_tp_nei_candidati", "timololo_candidati", "timololo_gruppi", "sintetici_richiamo_sopra_10x", "sintetici_richiamo_3x_10x",
                "sintetici_isomeri_separati", "sintetici_errore_area_mediano", "sintetici_righe_false_rumore", "sintetici_famiglie_giuste", "sintetici_isf_senza_ms2",
                "sintetici_esche_oltre_livello_4", "sintetici_cinetica_forma", "sintetici_cinetica_generazione", "sintetici_purezza_errore_mediano",
                "determinismo_due_esecuzioni", "determinismo_nativo_wasm")


# the table view (rows L1-L7 and exports): the criteria on the aligned rows, their recall and area, and the determinism; the candidate view needs every required criterion
ID_TABELLA = ("timololo_tp_trovati", "timololo_gruppi", "sintetici_richiamo_sopra_10x", "sintetici_richiamo_3x_10x", "sintetici_isomeri_separati", "sintetici_errore_area_mediano",
              "sintetici_righe_false_rumore", "determinismo_due_esecuzioni", "determinismo_nativo_wasm")


def concordanza(criteri: list[dict]) -> str:
    z = [c for c in criteri if c["id"].startswith("mzmine_")]
    if any(c["ok"] is False for c in z):
        return "non_passa"
    return "non_misurata" if any(c["ok"] is None for c in z) else "passa"


def cancello(criteri: list[dict], data: str, crate: str) -> dict:
    """The document `cancello/2`: the table view opens if the `ID_TABELLA` criteria were measured and pass; the candidate view only if every required criterion did (so it implies the table)."""
    esito = {c["id"]: c["ok"] for c in criteri}
    tabella = all(esito.get(i) is True for i in ID_TABELLA)
    vista = all(esito.get(i) is True for i in ID_RICHIESTI)
    return {"concordanza_mzmine": concordanza(criteri), "crate_version": crate, "criteri": criteri, "data": data, "format": FORMATO, "vista_candidati": vista, "vista_tabella": tabella}


def a_json(c: dict) -> str:
    return json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def valido(c: dict | None, crate: str) -> bool:
    """The test of the panel (`PIPE.gateState`) for the candidate view: `cancello/1` or `/2`, `vista_candidati` true and the version of the engine that was run."""
    return bool(c) and c.get("format") in FORMATI_LETTI and c.get("vista_candidati") is True and str(c.get("crate_version")) == str(crate)


def valido_tabella(c: dict | None, crate: str) -> bool:
    """The test of the panel for the table view: `cancello/2` with `vista_tabella` true; a `cancello/1` has no table of its own and follows `vista_candidati`."""
    if not c or c.get("format") not in FORMATI_LETTI or str(c.get("crate_version")) != str(crate):
        return False
    return (c.get("vista_tabella") if c["format"] == FORMATO else c.get("vista_candidati")) is True


# ---------------------------------------------------------------------------------------------------------------- synthetic series
def genera_sintetico(seme: int, profilo: str, cartella: Path, scala: float | None = None):
    """Writes the mzML and the indexes of a synthetic identity series (the heights of the products times `scala`, if given); returns (truth, index paths in file order)."""
    from . import sintetici_identita as si
    c = si.costruisci(seme, profilo)
    for o in c.ioni:
        if scala is not None and o.ruolo in ("tp1", "tp2", "isomero") and o.amp is not None:
            o.amp = o.amp * scala
    cartella.mkdir(parents=True, exist_ok=True)
    mzml = []
    for k, (stem, _, _) in enumerate(c.file):
        p = cartella / f"SIN_{stem}.mzML"
        scrivi_file(p, k, c, seme)
        mzml.append(p)
    return c, nativo.indici(mzml, cartella)


def esegui_sintetico(seme: int, profilo: str, cartella: Path, scala: float | None = None):
    """(truth, native run, index paths, parent) of a synthetic series."""
    c, idx = genera_sintetico(seme, profilo, cartella, scala)
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    parent = (nativo.neutra_da_ione(padre.ion), padre.mz, padre.rt)
    return c, nativo.esegui(idx, *parent, cartella / "nativo"), idx, parent


def due_esecuzioni(idx: list[str], parent: tuple, cartella: Path, prima: nativo.Serie) -> bool:
    """The `.mzfeat` and `.mzfeatf` of a second run have the same SHA-256 as those of the first."""
    return nativo.esegui(idx, *parent, cartella / "nativo_bis").sha == prima.sha


def nativo_contro_wasm(idx: list[str], parent: tuple, cartella: Path, prima: nativo.Serie) -> bool | None:
    """The digests of the `.wasm` run through Node equal those of the native run; None if cargo (wasm32 target), wasm-bindgen or node are missing."""
    if not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")):
        return None
    run = lambda *a: subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout       # noqa: E731
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg, out = cartella / "pkg", cartella / "wasm"
    out.mkdir(parents=True, exist_ok=True)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), str(ROOT / "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm"))
    run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(out), *idx, "--parent", parent[0], "--rules", str(nativo.REGOLE), "--parent-mz", repr(parent[1]), "--parent-rt", repr(parent[2]))
    return nativo.leggi_serie(out, [Path(x).name for x in idx]).sha == prima.sha


# ---------------------------------------------------------------------------------------------------------------- the whole measure
def serie_reale(cartella: Path, ioni: list[dict], lavoro: Path, mzmine: str | None = None):
    """Runs the native pipeline on a real series (the parent is the one of the truth) and measures it. Returns (criteria, details)."""
    from . import tabelle
    serie = tabelle.trova_serie(cartella)
    file_ = {x["nome"]: Path(cartella) / (x["nome"] + ".mzML") for x in serie}
    mzml = [p if p.exists() else p.with_suffix(".mzml") for p in file_.values()]
    idx = nativo.indici(mzml, lavoro / "indici")
    padre = next(x for x in ioni if x.get("parent"))
    run = nativo.esegui(idx, nativo.neutra_da_ione(padre["ion"]), padre["mz"], (padre["rt_obs"] or [None])[0], lavoro / "nativo")
    trattate = [k for k, x in enumerate(serie) if x["time"] and x["time"] > 0 and x["type"] == "sample"]
    tp = M.tp_timololo(ioni, run, trattate)
    crit_, det = criteri_timololo(tp), {"timololo": tp, "crate": run.versione_crate}
    if mzmine:
        from .tabelle import leggi_mzmine
        det["mzmine"] = M.concordanza_mzmine(run.tabella, leggi_mzmine(mzmine, serie), trattate, 50.0)
    return crit_ + criteri_mzmine(det.get("mzmine")), det


def misura(cartella_reale: Path | None, ioni: list[dict], mzmine: str | None, profilo: str, seme: int, scala_debole: float | None, lavoro: Path, data: str, wasm: bool = True):
    """The whole gate: real series (if given), synthetic series (`profilo`, with a weak variant if `scala_debole`), determinism. Returns (cancello, details)."""
    det: dict = {"sintetici": {"profilo": profilo, "seme": seme}}
    c, run, idx, parent = esegui_sintetico(seme, profilo, lavoro / "sintetico")
    debole = None
    if scala_debole:
        cd, rd, _, _ = esegui_sintetico(seme, profilo, lavoro / "sintetico_debole", scala_debole)
        debole = (cd, rd)
    m = misure_sintetiche(c, run, debole)
    det["sintetici"]["misure"] = {k: {"valore": v[0], "n": v[1]} for k, v in m.items()}
    det["sintetici"]["righe"], det["sintetici"]["candidati"] = len(run.tabella), int(len(run.candidati))
    reali, d_reale = serie_reale(cartella_reale, ioni, lavoro / "reale", mzmine) if cartella_reale else (criteri_timololo(None) + criteri_mzmine(None), {})
    uguale = due_esecuzioni(idx, parent, lavoro / "sintetico", run)
    cw = nativo_contro_wasm(idx, parent, lavoro / "sintetico", run) if wasm else None
    criteri = reali + criteri_sintetici(m) + criteri_determinismo(uguale, cw) + criteri_prestazioni()
    det.update(d_reale)
    return cancello(criteri, data, run.versione_crate), det
