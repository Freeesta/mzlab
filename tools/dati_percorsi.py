#!/usr/bin/env python3
"""The one place that knows where the private data repository (mzlab-dati) is and how it is laid out.

Callers ask for a path in the NEW layout (`percorso("dati/3200 qtrap/mzML")`); while the legacy column of
tools/dati_mappa.tsv exists, a path missing in the new layout is looked up in the old one. Used by tests, tools and e2e.
"""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("mzlab-dati", "QqQ-lab-dati")
# A folder is the data repository when it has the new index or one of the legacy top folders.
MARKERS = ("dati/INDICE.md", "nomi_vietati.txt", "mzML", "dati_formati", "3200 qtrap", "HRMS", "scratchpad")


def _mappa() -> list[tuple[str, str]]:
    righe = [r.split("\t") for r in (Path(__file__).with_name("dati_mappa.tsv")).read_text(encoding="utf-8").splitlines()
             if r and not r.startswith("#")]
    return sorted(((r[0], r[1]) for r in righe), key=lambda r: -len(r[0]))


def _candidati() -> list[Path]:
    cands = [Path(v) for v in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI")) if v]
    for base in (ROOT.parent, ROOT.parent.parent, Path.home(), Path("/home/user"), Path("/workspace"), Path("/workspaces"), Path("/repos"), Path("/tmp")):
        cands += [base / n for n in NAMES]
    return cands


def radice() -> Path | None:
    """The data repository, or None: MZLAB_DATI / QQQ_DATI, then a sibling folder named mzlab-dati (or the old name)."""
    return next((c for c in _candidati() if any((c / m).exists() for m in MARKERS)), None)


def percorso(rel: str) -> Path | None:
    """Existing path for `rel` (new layout, else legacy), or None when there is no data repository or no such path."""
    r = radice()
    if r is None:
        return None
    if (r / rel).exists():
        return r / rel
    for nuovo, vecchio in _mappa():
        if rel == nuovo or rel.startswith(nuovo + "/"):
            p = r / (vecchio + rel[len(nuovo):])
            return p if p.exists() else None
    return None
