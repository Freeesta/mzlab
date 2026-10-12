"""Native outputs -> the common table `bench/1`: `{formato, strumento, dataset, config, file[], righe[]}`, a row being `{id, mz, rt, rt_min, rt_max, aree[], altezze[],
ms2_scan, gruppo, candidato, livello, punteggio}` (fields a tool does not give are null). Rows are sorted by (m/z, RT, area) and numbered from 0, so the same
content gives the same bytes. Also here: mzFinder on a synthetic series, through the native run of the pipeline of the site (`valida.nativo`)."""
from __future__ import annotations

import csv
import json
import re
from pathlib import Path

import numpy as np

from . import FORMATO

CAMPI = ("id", "mz", "rt", "rt_min", "rt_max", "aree", "altezze", "ms2_scan", "gruppo", "candidato", "livello", "punteggio")


def tabella(strumento: str, dataset: str, config: str, file: list[str], righe: list[dict]) -> dict:
    """The document `bench/1`; `righe` need `mz`, `rt` and `aree` (one per file), the rest is optional."""
    r2 = []
    for r in righe:
        if len(r["aree"]) != len(file):
            raise ValueError("one area per file is required")
        r2.append({k: r.get(k) for k in CAMPI if k != "id"})
    r2.sort(key=lambda r: (r["mz"], r["rt"], -float(max(r["aree"], default=0.0))))
    for i, r in enumerate(r2):
        r["id"] = i
    return {"formato": FORMATO, "strumento": strumento, "dataset": dataset, "config": config, "file": list(file), "righe": [{k: r[k] for k in CAMPI} for r in r2]}


def a_json(t: dict) -> str:
    return json.dumps(t, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def leggi(path) -> dict:
    t = json.loads(Path(path).read_text(encoding="utf-8"))
    if t.get("formato") != FORMATO:
        raise ValueError(f"not a {FORMATO} table")
    return t


def matrici(t: dict) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """(m/z, RT, areas (rows x files)) of a table as arrays."""
    r = t["righe"]
    return (np.array([x["mz"] for x in r], float), np.array([x["rt"] for x in r], float), np.array([x["aree"] for x in r], float).reshape(len(r), len(t["file"])))


# ---------------------------------------------------------------------------------------------------------------- mzFinder
def da_serie(serie, dataset: str, config: str = "predefinita") -> dict:
    """The table of a native run (`valida.nativo.Serie`): the areas of the aligned table (absent cells 0), and for each row whether its identity is a candidate
    (a proof in favour), its best level and its points."""
    tab = serie.tabella
    cand = {int(i) for i in serie.candidati}
    ident = {int(i["id"]): i for i in serie.identita}
    righe = []
    for i in range(len(tab)):
        x = ident.get(i)
        righe.append({"mz": float(tab.mz[i]), "rt": float(tab.rt[i]), "aree": [float(a) for a in tab.area[i]], "candidato": i in cand,
                      "livello": None if x is None else str(x["livello_max"]), "punteggio": None if x is None else float(x["totale"])})
    return tabella("mzfinder", dataset, config, list(tab.campioni), righe)


def corri_mzfinder_sintetico(seme: int, profilo: str, lavoro, dataset: str = "D2", config: str = "predefinita") -> tuple[dict, list[dict]]:
    """Generates the synthetic series (D2), runs mzFinder on it and returns (the `bench/1` table, the truth in the common format)."""
    from valida import cancello
    from valida import sintetici_identita as si
    from .verita import da_righe
    c, serie, _, _ = cancello.esegui_sintetico(seme, profilo, Path(lavoro))
    return da_serie(serie, dataset, config), da_righe(si.righe_verita(c))


# ---------------------------------------------------------------------------------------------------------------- other tools
# column rules: (m/z column, RT column, regex of an area column with the file name in group 1, RT in seconds)
REGOLE = {
    "mzmine": ("row m/z", "row retention time", r"^(?:datafile:)?(.+?)(?::area| Peak area)$", False),
    "xcms": ("mzmed", "rtmed", r"^(?!mz|rt|npeaks)(.+)$", True),
    "openms": ("mz", "rt", r"^intensity_(.+)$", True),
    "asari": ("mz", "rtime", r"^(?!id_|mz$|rtime|rt_|snr|selectivity|detection|goodness|cSelectivity|cValue|charge|ion_|parent_|neutral|name|identifier|m_|mass_|ion$)(.+)$", True),
    "patroon": ("mz", "ret", r"^(?!mz$|ret$|group$|ret_|mz_|ppm)(.+)$", True),
}


def _nome(x: str) -> str:
    return re.sub(r"\.(mzml|raw|mzxml)$", "", x.strip(), flags=re.I)


def da_csv(strumento: str, path, file: list[str], dataset: str, config: str = "predefinita", sep: str | None = None) -> dict:
    """A feature table written by `strumento` (mzmine CSV, XCMS `featureDefinitions` joined with `featureValues`, OpenMS consensus as CSV, asari
    `full_feature_table`, patRoon `featureGroups` as CSV) -> `bench/1`. `file`: the names of the files, in the output order; their area columns are found
    by name (extension ignored); a missing file gets zeros."""
    mz_c, rt_c, rx, secondi = REGOLE[strumento]
    sep = sep or ("\t" if str(path).endswith((".tsv", ".txt")) else ",")
    with open(path, newline="", encoding="utf-8") as f:
        rd = csv.DictReader(f, delimiter=sep)
        col = {_nome(m.group(1)): c for c in rd.fieldnames or [] if (m := re.match(rx, c)) and c not in (mz_c, rt_c)}
        righe = []
        for r in rd:
            try:
                mz, rt = float(r[mz_c]), float(r[rt_c])
            except (KeyError, ValueError, TypeError):
                continue
            aree = []
            for n in file:
                try:
                    aree.append(float(r[col[_nome(n)]] or 0.0))
                except (KeyError, ValueError):
                    aree.append(0.0)
            righe.append({"mz": mz, "rt": rt / 60.0 if secondi else rt, "aree": [a if np.isfinite(a) else 0.0 for a in aree]})
    return tabella(strumento, dataset, config, [_nome(n) for n in file], righe)
