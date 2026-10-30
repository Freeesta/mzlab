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


# The data repository was tidied (reorganize-step1): folders moved. Each rule is (old prefix, new prefix, name test);
# `percorso` tries the path as asked, then the moved/unmoved twin, so code and data can merge in any order.
MOVES = (
    ("esperimenti/", "esperimenti/2026-10/", lambda n: n.startswith(("loc_", "coo_"))),
    ("esperimenti/", "esperimenti/altro/", lambda n: n in ("Computazio", "Sostanze psicoattive", "motore_wasm")),
    ("coordinamento/", "coordinamento/documenti/", lambda n: n in ("studi", "proposte", "consegne", "riferimenti", "gemini_esiti")),
    ("coordinamento/prompt/raw", "prompt/raw", lambda n: True),
)


def _gemelli(rel: str) -> list[str]:
    """The other spellings of `rel` (old layout <-> new layout)."""
    out = []
    for old, new, ok in MOVES:
        if rel.startswith(old) and ok(rel[len(old):].split("/", 1)[0] if old.endswith("/") else ""):
            out.append(new + rel[len(old):])
        elif rel.startswith(new) and ok(rel[len(new):].split("/", 1)[0] if new.endswith("/") else ""):
            out.append(old + rel[len(new):])
    return out


def percorso(rel: str) -> Path | None:
    """Existing path for `rel` (or its moved/unmoved twin), or None when there is no data repository or no such path."""
    r = radice()
    if r is None:
        return None
    return next((r / c for c in [rel, *_gemelli(rel)] if (r / c).exists()), None)
