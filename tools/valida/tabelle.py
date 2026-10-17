"""Normalised feature tables, the HR series of a folder and the reader of an mzmine CSV (column rules of `UNT.detect`: `row m/z`, `row retention time`,
`datafile:<F>:area` or `<F> Peak area`)."""
from __future__ import annotations

import csv
import re
from dataclasses import dataclass
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
    altezza: np.ndarray | None = None      # height (counts) of every cell, the shape of `area`; None when the table has no heights

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

    def massimo_altezza(self, colonne) -> np.ndarray:
        """Largest height of each row over the columns (aligned tables with heights only)."""
        colonne = list(colonne)
        if self.altezza is None or self.lungo:
            raise ValueError(f"table {self.nome}: no heights")
        return np.nan_to_num(self.altezza[:, colonne]).max(1) if colonne else np.zeros(len(self))


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
    """The HR LC series of a folder: mzML files with high-resolution MS1 (the MSn infusion of a standard and unit-resolution files are left out),
    sorted by time, then name."""
    from mzlab.reader.mzml import Run
    out = []
    for p in sorted({*Path(cartella).glob("*.mzML"), *Path(cartella).glob("*.mzml")}):      # a set: on Windows and macOS both patterns match every file
        r = Run(p)
        levels = {s.level for s in r.scans}
        hr = max(levels, default=1) < 3 and getattr(r, "hr1", False) and 1 in levels     # not an MSn infusion, high-resolution MS1
        r.close()
        if hr:
            out.append(descrivi_file(p))
    out.sort(key=lambda x: (x["time"] is None, x["time"] if x["time"] is not None else 0.0, x["nome"]))
    if not [x for x in out if x["time"] is not None and x["time"] > 0]:
        raise ValueError(f"{cartella}: no treated samples (time > 0) among the high-resolution files")
    return out


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
    cols, cols_h = {}, {}
    for i, c in enumerate(h):
        m = re.match(r"^datafile:(.+):area$", c, re.I) or re.match(r"^(.*?)\s*Peak area$", c, re.I)
        if m and m.group(1):
            sn = re.sub(r"\.mzml$", "", m.group(1).strip(), flags=re.I).lower()
            if sn in pos:
                cols[pos[sn]] = i
        m = re.match(r"^datafile:(.+):height$", c, re.I) or re.match(r"^(.*?)\s*Peak height$", c, re.I)
        if m and m.group(1):
            sn = re.sub(r"\.mzml$", "", m.group(1).strip(), flags=re.I).lower()
            if sn in pos:
                cols_h[pos[sn]] = i
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
    altezza = None
    if cols_h:
        altezza = np.zeros((len(body), len(serie)))
        for k, i in cols_h.items():
            altezza[:, k] = [num(r[i]) if i < len(r) else 0.0 for r in body]
    return Tabella("mzmine", [x["nome"] for x in serie], np.array([num(r[imz]) for r in body]), np.array([num(r[irt]) for r in body]), area, altezza=altezza)
