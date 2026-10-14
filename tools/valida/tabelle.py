"""Normalised feature tables and the code that produces them: the mzFinder Python pipeline run file by file (detect, align, filters, fill_gaps, the
same calls as `tpmine.hr.engine`) and the reader of an mzmine CSV (column rules of `UNT.detect`: `row m/z`, `row retention time`,
`datafile:<F>:area` or `<F> Peak area`)."""
from __future__ import annotations

import csv
import gc
import re
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np


@dataclass
class Tabella:
    """Rows of (m/z, RT) with one area column per file. `lungo` tables (detected features of all files stacked) have `file` (column of every row) and a 1-d `area`."""
    nome: str
    campioni: list
    mz: np.ndarray
    rt: np.ndarray
    area: np.ndarray
    file: np.ndarray | None = None

    def __len__(self):
        return len(self.mz)

    @property
    def lungo(self) -> bool:
        return self.file is not None

    def presenti(self, k: int) -> np.ndarray:
        """Rows that have a feature in file column k."""
        if self.lungo:
            return np.flatnonzero(self.file == k)
        return np.flatnonzero(self.area[:, k] > 0)

    def area_di(self, k: int, righe) -> np.ndarray:
        return self.area[righe] if self.lungo else self.area[righe, k]

    def massimo(self, colonne) -> np.ndarray:
        """Largest area of each row over the columns (aligned tables only)."""
        colonne = list(colonne)
        if self.lungo or not colonne:
            return np.zeros(len(self))
        return self.area[:, colonne].max(1)


def nome_file(path) -> str:
    return re.sub(r"\.mzml$", "", Path(path).name, flags=re.I)


def descrivi_file(path) -> dict:
    """time (min, None if unknown) and type of a file from its name: tNNmin, t0/dark/adsorption = reference at time 0, blank/bianco/control."""
    n = nome_file(path)
    low = n.lower()
    tipo = "blank" if re.search(r"blank|bianc", low) else "control" if "control" in low else "sample"
    m = re.search(r"(?:^|[_\-\s])t(\d+(?:[.,]\d+)?)\s*min", low) or re.search(r"(?:^|[_\-\s])t(\d+(?:[.,]\d+)?)(?:$|[_\-\s])", low)
    tempo = float(m.group(1).replace(",", ".")) if m else (0.0 if re.search(r"dark|adsorb", low) else None)
    return {"nome": n, "path": str(path), "time": tempo, "type": tipo}


def trova_serie(cartella) -> list[dict]:
    """The HR LC series of a folder: mzML files classified like the engine does (the MSn infusion of a standard and unit-resolution files are left out),
    sorted by time, then name."""
    from mzlab.reader.mzml import Run
    from tpmine.hr.engine import classify
    out = []
    for p in sorted({*Path(cartella).glob("*.mzML"), *Path(cartella).glob("*.mzml")}):      # a set: on Windows and macOS both patterns match every file
        r = Run(p)
        kind = classify(r)
        r.close()
        if kind == "hr":
            out.append(descrivi_file(p))
    out.sort(key=lambda x: (x["time"] is None, x["time"] if x["time"] is not None else 0.0, x["nome"]))
    if not [x for x in out if x["time"] is not None and x["time"] > 0]:
        raise ValueError(f"{cartella}: no treated samples (time > 0) among the high-resolution files")
    return out


def tipi_e_tempi(serie: list[dict]) -> tuple[np.ndarray, list]:
    """(times for the filters, types): the same convention as the engine (-1 for a reference without time)."""
    ref = lambda x: (x["time"] is not None and x["time"] <= 0) or x["type"] in ("blank", "control")      # noqa: E731
    tempi = np.array([x["time"] if x["time"] is not None else (-1.0 if ref(x) else np.nan) for x in serie], float)
    return tempi, [x["type"] for x in serie]


# ---------------------------------------------------------------------------------------------------------------------- mzFinder
@dataclass
class EsitoMzfinder:
    rilevate: Tabella
    allineata: Tabella
    riempita: Tabella
    candidati: np.ndarray                      # rows of the aligned table that survive the filters
    passi: list
    riempiti: int
    tempi_s: dict = field(default_factory=dict)


def esegui_mzfinder(serie: list[dict], ioni_parent: dict | None = None, soglie: float = 1.0, polarita: int = 1, per_tabella=None) -> EsitoMzfinder:
    """Detection file by file, alignment, RT map, filters and gap filling with the default settings of the engine; `soglie` scales `floor` and `min_height`
    (sensitivity runs); `per_tabella(k, table)` is called while the MS1 table of file k is loaded (reference-peak extraction). No MSn tree is used."""
    from mzlab.reader.mzml import Run
    from tpmine.hr import engine as EN
    from tpmine.hr import features as FT
    from tpmine.hr import filters as FL
    from tpmine.hr import rtalign as RA
    s = dict(EN.DEFAULT_SETTINGS)
    s["floor"], s["min_height"] = s["floor"] * soglie, s["min_height"] * soglie
    n = len(serie)
    campioni = [x["nome"] for x in serie]
    t = {}
    t0 = time.perf_counter()
    feats = []
    for k, x in enumerate(serie):
        r = Run(x["path"])
        tab = r.table(1, polarita)
        feats.append(FT.detect(tab, floor=s["floor"], min_height=s["min_height"]))
        if per_tabella:
            per_tabella(k, tab)
        r._tables.clear()
        r.close()
        del tab
        gc.collect()
    t["rilevamento"] = time.perf_counter() - t0
    t1 = time.perf_counter()
    ril = [(f.mz[~f.artefact], f.rt[~f.artefact], f.area[~f.artefact], np.full((~f.artefact).sum(), k)) for k, f in enumerate(feats)]
    rilevate = Tabella("mzFinder rilevate", campioni, np.concatenate([a[0] for a in ril]), np.concatenate([a[1] for a in ril]),
                       np.concatenate([a[2] for a in ril]), np.concatenate([a[3] for a in ril]).astype(int))
    al = FT.align(feats)
    allineata = Tabella("mzFinder allineata", campioni, al.mz.copy(), al.rt.copy(), al.area.copy())
    rtmap = RA.build(al, min_height=s["min_height"])
    tempi, tipi = tipi_e_tempi(serie)
    fun = FL.run_filters(al, feats, tempi, tipi, ioni_parent, fold=s["fold"], min_treated=s["min_treated"], rt_min=s["rt_min"], min_height=s["min_height"],
                         flat_min_present=s["flat_min_present"])
    _, tr_c = FL.split_columns(tempi, tipi)
    t["allineamento_filtri"] = time.perf_counter() - t1
    t2 = time.perf_counter()
    riempiti = 0
    if len(fun.idx):
        ultimo = []

        def tabelle(k):
            for r in ultimo:                                     # one file in memory at a time
                r._tables.clear()
                r.close()
            ultimo.clear()
            r = Run(serie[k]["path"])
            ultimo.append(r)
            return r.table(1, polarita)
        riempiti = int(FT.fill_gaps(al, fun.idx, tabelle, [int(c) for c in tr_c], floor=s["floor"], rt_tol=s["fill_rt_tol"] if rtmap.ok else 0.1, rtmap=rtmap).sum())
        for r in ultimo:
            r._tables.clear()
            r.close()
    t["riempimento"] = time.perf_counter() - t2
    riempita = Tabella("mzFinder riempita", campioni, al.mz.copy(), al.rt.copy(), al.area.copy())
    return EsitoMzfinder(rilevate, allineata, riempita, np.asarray(fun.idx, int), fun.steps, riempiti, t)


# ---------------------------------------------------------------------------------------------------------------------- mzmine
def leggi_mzmine(path, serie: list[dict]) -> Tabella:
    """mzmine CSV (export 'full' or quant) as a table whose columns are the files of `serie` (by name without .mzML); series files missing from the CSV stay at 0."""
    text = Path(path).read_text(encoding="utf-8-sig")
    head = text.splitlines()[0] if text else ""
    delim = ";" if head.count(";") > head.count(",") else ","
    rows = list(csv.reader(text.splitlines(), delimiter=delim))
    h = [c.strip() for c in rows[0]]
    low = [c.lower() for c in h]

    def col(*names):
        for nme in names:
            if nme in low:
                return low.index(nme)
        raise ValueError(f"{path}: column not found ({' / '.join(names)})")
    imz, irt = col("row m/z", "mz"), col("row retention time", "rt")
    pos = {x["nome"].lower(): k for k, x in enumerate(serie)}
    cols = {}
    for i, c in enumerate(h):
        m = re.match(r"^datafile:(.+):area$", c, re.I) or re.match(r"^(.*?)\s*Peak area$", c, re.I)
        if m and m.group(1):
            sn = re.sub(r"\.mzml$", "", m.group(1).strip(), flags=re.I).lower()
            if sn in pos:
                cols[pos[sn]] = i
    if not cols:
        raise ValueError(f"{path}: no area column matches a file of the series")

    def num(x):
        try:
            return float(x)
        except ValueError:
            return 0.0
    body = [r for r in rows[1:] if len(r) > max(imz, irt)]
    area = np.zeros((len(body), len(serie)))
    for k, i in cols.items():
        area[:, k] = [num(r[i]) if i < len(r) else 0.0 for r in body]
    return Tabella("mzmine", [x["nome"] for x in serie], np.array([num(r[imz]) for r in body]), np.array([num(r[irt]) for r in body]), area)
