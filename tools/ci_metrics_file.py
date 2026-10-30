"""Rotating data files of tools/ci_metrics.py: one `eventi-YYYY-MM[-N].jsonl` per month on the branch `stat-ci`, plus the frozen legacy `eventi.jsonl`.
Standard library only; used by ci_metrics.py, coo_unisci.py and ci_pytest_lenti.py."""
from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from datetime import datetime

import ci_metrics_data as dati_ci

LIMITE_B = 50_000_000   # a data file this big is frozen (never appended to or rewritten): GitHub refuses files over 100 MB


def leggi(path: str) -> list[dict]:
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(riga) for riga in f if riga.strip()]


def segmenti(dati: str) -> list[str]:
    """Data files of one series, oldest first: `dati` itself (the legacy single file, now frozen) then `<stem>-YYYY-MM[-N]<suffix>` of the same folder."""
    base, ext = os.path.splitext(dati)
    cartella = os.path.dirname(dati) or "."
    trovati = []
    for nome in os.listdir(cartella) if os.path.isdir(cartella) else []:
        m = re.fullmatch(re.escape(os.path.basename(base)) + r"-(\d{4}-\d{2})(?:-(\d+))?" + re.escape(ext), nome)
        if m:
            trovati.append((m.group(1), int(m.group(2) or 1), os.path.join(cartella, nome)))
    return ([dati] if os.path.exists(dati) else []) + [p for _, _, p in sorted(trovati)]


def leggi_tutti(dati: str) -> list[dict]:
    return [r for p in segmenti(dati) for r in leggi(p)]


def attivo(dati: str, ora: datetime | None = None) -> str:
    """File that receives the new rows: this month's, or the next part of it once the current one reaches LIMITE_B."""
    base, ext = os.path.splitext(dati)
    mese = (ora or dati_ci.adesso_dt()).strftime("%Y-%m")
    n = 1
    while True:
        p = f"{base}-{mese}{'' if n == 1 else f'-{n}'}{ext}"
        if not os.path.exists(p) or os.path.getsize(p) < LIMITE_B:
            return p
        n += 1


def eventi_ramo(ramo: str = "origin/stat-ci", repo: str | None = None) -> list[dict]:
    """All rows of every events file on a git branch (legacy `eventi.jsonl` and the monthly parts), for the read-only tools."""
    nomi = subprocess.run(["git", "ls-tree", "--name-only", ramo], cwd=repo, capture_output=True, text=True).stdout.split()
    with tempfile.TemporaryDirectory() as d:
        for n in nomi:
            if re.fullmatch(r"eventi(-\d{4}-\d{2}(-\d+)?)?\.jsonl", n):
                with open(os.path.join(d, n), "w", encoding="utf-8") as f:
                    f.write(subprocess.run(["git", "show", f"{ramo}:{n}"], cwd=repo, capture_output=True, text=True).stdout)
        return leggi_tutti(os.path.join(d, "eventi.jsonl"))
