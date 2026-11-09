"""The native run of the high-resolution pipeline without a parent (detect, align, fill: `series.mzfeat`) and the preview of the parameters of a file.

Used by the robustness criteria (`hr.py`): files without a verified truth (DDA Orbitrap, Q-TOF) where the pipeline must run, repeat and propose a threshold."""
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

import numpy as np

from . import ROOT, nativo
from .tabelle import Table

PREVIEW = ROOT / "target" / "release" / "examples" / "anteprima"


def run_without_parent(idx: list[str], output: Path) -> dict:
    """Runs the native example on `idx` without a parent; returns {'tabella': Tabella, 'stato': (rows, files) array, 'sha': {file: digest}}. Raises on a failed run."""
    nativo.build_example()
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(nativo.EXAMPLE), str(output), *idx], cwd=ROOT, check=True, capture_output=True)
    meta, a = nativo.open_data((output / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k).astype(int)
    area = a["area"].reshape(n, k).astype(float)
    tab = Table("nativo", [Path(x).stem for x in idx], a["mz"].astype(float), a["rt"].astype(float), np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(area)))
    sha = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(output.glob("*.mzfeat*"))}
    return {"tabella": tab, "stato": stato, "area_cruda": area, "sha": sha}


def preview(idx: str) -> dict:
    """The JSON of the preview of one file (`noise.floor` is the proposed threshold, `ppm.value` and `fwhm.value` the estimated tolerances)."""
    subprocess.run(["cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat", "--example", "anteprima"], cwd=ROOT, check=True)
    out = subprocess.run([str(PREVIEW), idx], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out[out.index("{"):])
