"""The native run of the pipeline of the site (`mzlab-feat-wasm`, example `nativo`) on a series of mzML files, and the readers of what it writes.

`.mzfeat` (series) and `.mzfeatf` (one file) are the containers of crates/mzlab-feat/src/store.rs: the magic `MZFEAT01`, the length of the JSON meta, the meta,
padding to 64 bytes, the arrays at the offsets of the meta. The table is read into the `Tabella` of tabelle.py; the candidates are the identities that
have a proof in favour (`totale` above 0 in the `evid/1` section): the list that the panel shows once the gate is open, the others are «discarded»."""
from __future__ import annotations

import hashlib
import json
import re
import struct
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from . import ROOT
from .tabelle import Tabella

DTYPE = {"f64": "<f8", "f32": "<f4", "u32": "<u4", "u16": "<u2", "u8": "u1", "i8": "i1"}
CELLA_RILEVATA, CELLA_MIRATA, CELLA_RIEMPITA, CELLA_ASSENTE = 0, 1, 2, 3
ESEMPIO = ROOT / "target" / "release" / "examples" / "nativo"
REGOLE = ROOT / "TP_Mine" / "data" / "regole_tp.json"


def apri(dati: bytes) -> tuple[dict, dict]:
    """(meta, arrays) of a container: the arrays by name as numpy views."""
    if dati[:8] != b"MZFEAT01":
        raise ValueError("not a .mzfeat / .mzfeatf container")
    n = struct.unpack("<Q", dati[8:16])[0]
    meta = json.loads(dati[16:16 + n])
    inizio = -(-(16 + n) // 64) * 64
    arr = {}
    for a in meta["arrays"]:
        dt = np.dtype(DTYPE[a["dtype"]])
        arr[a["name"]] = np.frombuffer(dati, dtype=dt, count=a["count"], offset=inizio + a["offset"])
    return meta, arr


@dataclass
class Serie:
    """The result of a native run: table, states of the cells, identities, features of each file, digests."""
    tabella: Tabella
    stato: np.ndarray                      # (rows, files) uint8: 0 detected, 1 targeted, 2 filled, 3 absent
    identita: list                         # the identities of evid/1 (dicts), in the order of the graph
    candidati: np.ndarray                  # rows of the identities with a proof in favour
    per_file: list                         # {"nome", "mz", "rt", "purezza", "altezza"} of every file
    sha: dict = field(default_factory=dict)
    versione_crate: str = ""
    feat_version: int = 0
    tp: dict | None = None                 # the block tp/1 (status and kinship of every row) when the run had a parent and controls


def leggi_serie(cartella, nomi: list[str]) -> Serie:
    """Reads `series_evid.mzfeat` and the `.mzfeatf` of the files (`nomi`: names of the index files, in the order of the run)."""
    cartella = Path(cartella)
    dati = (cartella / "series_evid.mzfeat").read_bytes()
    meta, a = apri(dati)
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    campioni = [re.sub(r"\.mzidx$", "", x) for x in nomi]
    tab = Tabella("nativo", campioni, a["mz"].copy(), a["rt"].copy(), area)
    ev = json.loads(a["evid"].tobytes()) if len(a["evid"]) else {"identita": []}
    ident = ev["identita"]
    cand = np.array(sorted(i["id"] for i in ident if i["totale"] > 0), dtype=int)
    per_file = []
    for nome in nomi:
        m, f = apri((cartella / f"{nome}.mzfeatf").read_bytes())
        per_file.append({"nome": nome, "mz": f["mz"].astype(float), "rt": f["rt"].astype(float), "purezza": f["purity"].astype(float), "altezza": f["height"].astype(float)})
    sha = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(cartella.glob("*.mzfeat*"))}
    tp = {"stato": a["tp_status"].astype(int), "parentela": a["tp_kinship"].astype(int), "note": a["tp_notes"].astype(int)} if "tp_status" in a else None
    return Serie(tab, stato, ident, cand, per_file, sha, meta["crate"], meta["feat_version"], tp)


def indici(mzml: list[Path], cartella: Path) -> list[str]:
    """Builds (if missing) the `.mzidx` of every mzML in `cartella` and returns their paths. `..._t05` becomes `..._t05min`: the example reads the minutes from the name."""
    from mzlab.reader import mzidx
    from mzlab.reader.mzml import Run
    cartella.mkdir(parents=True, exist_ok=True)
    out = []
    for p in mzml:
        stem = re.sub(r"_t(\d+(?:\.\d+)?)$", r"_t\1min", p.stem)
        dest = cartella / f"{stem}.mzidx"
        if not dest.exists():
            run = Run(p, index_path=dest)
            mzidx.build(run, dest)
            run.close()
        out.append(str(dest))
    return out


def costruisci_esempio() -> None:
    subprocess.run(["cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo"], cwd=ROOT, check=True)


def esegui(percorsi_idx: list[str], formula_neutra: str, mz_genitore: float, rt_genitore: float, uscita: Path) -> Serie:
    """Runs the example on the indexes (in this order) and reads the result from `uscita`."""
    costruisci_esempio()
    uscita.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(ESEMPIO), str(uscita), *percorsi_idx, "--parent", formula_neutra, "--rules", str(REGOLE), "--parent-mz", repr(mz_genitore), "--parent-rt", repr(rt_genitore)],
                   cwd=ROOT, check=True, capture_output=True)
    return leggi_serie(uscita, [Path(x).name for x in percorsi_idx])


def neutra_da_ione(ione: str) -> str:
    """Neutral formula of an [M+H]+ ion formula (one hydrogen more than the ion has in the neutral)."""
    from mzlab.chem import elements as E
    f = E.parse_formula(ione)
    f["H"] = f.get("H", 0) - 1             # the ion formula counts the added proton: M = ion - H
    return E.fmt({e: n for e, n in f.items() if n})
