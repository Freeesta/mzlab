#!/usr/bin/env python3
"""The one place that knows where the private data repository (mzlab-dati) is and how it is laid out.

Callers ask for a path of the layout (`percorso("dati/3200 qtrap/mzML")`): top folders `dati/`, `coordinamento/`, `corso/`,
`studio/`, `strumenti/`, `esperimenti/`, truth files in `scratchpad/`. Used by tests, tools and e2e.
"""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("mzlab-dati", "QqQ-lab-dati")
# A folder is the data repository when it has the data index.
MARKERS = ("dati/INDICE.md",)


def _candidati() -> list[Path]:
    cands = [Path(v) for v in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI")) if v]
    for base in (ROOT.parent, ROOT.parent.parent, Path.home(), Path("/home/user"), Path("/workspace"), Path("/workspaces"), Path("/repos"), Path("/tmp")):
        cands += [base / n for n in NAMES]
    return cands


def radice() -> Path | None:
    """The data repository, or None: MZLAB_DATI / QQQ_DATI, then a sibling folder named mzlab-dati (or the old name)."""
    return next((c for c in _candidati() if any((c / m).exists() for m in MARKERS)), None)


def percorso(rel: str) -> Path | None:
    """Existing path for `rel`, or None when there is no data repository or no such path."""
    r = radice()
    return r / rel if r is not None and (r / rel).exists() else None
