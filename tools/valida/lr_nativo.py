"""The native runs of the low-resolution pipeline (the full scan one of `nativo.py`, the MRM one of the example `nativo_mrm`) and the readers of their tables.

The example reads the minutes from `..._t30min` in the name, so `B_FullMass-t30` is indexed as `B_FullMass_t30min`; files without a time are controls."""
from __future__ import annotations

import re
import subprocess
from pathlib import Path

import numpy as np

from . import ROOT, nativo
from .lr_criteri import Mrm, classifica, livello_da_nome

ESEMPIO_MRM = ROOT / "target" / "release" / "examples" / "nativo_mrm"


def indici_lr(mzml: list[Path], cartella: Path) -> list[str]:
    """`nativo.indici` for names that carry the time after `-` or `_` (`B_FullMass-t30`): the index is named `..._t30min`."""
    from mzlab.reader import mzidx
    from mzlab.reader.mzml import Run
    cartella.mkdir(parents=True, exist_ok=True)
    out = []
    for p in mzml:
        dest = cartella / (re.sub(r"[_\-]t(\d+(?:\.\d+)?)$", r"_t\1min", p.stem) + ".mzidx")
        if not dest.exists():
            run = Run(p, index_path=dest)
            mzidx.build(run, dest)
            run.close()
        out.append(str(dest))
    return out


def ordina_per_tempo(idx: list[str]) -> list[str]:
    """Files in the order of the run: by treatment time, controls last."""
    def chiave(x):
        m = re.search(r"_t(\d+(?:\.\d+)?)min", x)
        return (m is None, float(m.group(1)) if m else 0.0, x)
    return sorted(idx, key=chiave)


def esegui_mrm(mzml: list[Path], uscita: Path) -> dict:
    """Runs the MRM example on the files (in this order); returns {'sha': {file: digest}, 'tabella': bytes of series.mzfeat}."""
    subprocess.run(["cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo_mrm"], cwd=ROOT, check=True)
    uscita.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(ESEMPIO_MRM), str(uscita), *map(str, mzml)], cwd=ROOT, check=True, capture_output=True)
    sha = {}
    for p in sorted(uscita.glob("*.mzfeat*")):
        import hashlib
        sha[p.name] = hashlib.sha256(p.read_bytes()).hexdigest()
    return {"sha": sha, "tabella": (uscita / "series.mzfeat").read_bytes()}


def leggi_mrm(tabella: bytes, nomi: list[str]) -> Mrm:
    """The `.mzfeat` of an MRM series as `Mrm`: names, kind and level of every file from its name."""
    meta, a = nativo.apri(tabella)
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k).astype(int)
    area = np.nan_to_num(a["area"].reshape(n, k).astype(float), nan=0.0)
    tipo = [classifica(x) for x in nomi]
    lv = [livello_da_nome(x) if t == "std" else (None, False) for x, t in zip(nomi, tipo)]
    conc = bool(lv) and all(c for (v, c), t in zip(lv, tipo) if t == "std") and any(t == "std" for t in tipo)
    return Mrm(nomi, a["mz"].astype(float), a["q3"].astype(float), a["rt"].astype(float), stato, area, tipo, [v for v, _ in lv], conc)
