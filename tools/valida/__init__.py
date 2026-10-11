"""Validation harness for the feature tables of the high-resolution path (mzFinder Python, optionally an mzmine CSV).

It measures a table against the same yardsticks (isotopologue completeness, M+1/M ratio, true products found and their rank, Jaccard between two tables,
rows in the blank, sensitivity to thresholds, time and memory) so that a new engine can be judged by the numbers of the old one. It never decides what is a
transformation product: the truth is an optional JSON file given on the command line and lives only in the private data repository.
Modules: tabelle (readers and the mzFinder runner), abbina (deterministic matching), isotopologhi (reference peaks), metriche, rapporto."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
for _p in (ROOT, ROOT / "TP_Mine" / "py"):                 # `mzlab` and the `tpmine` package, as in tools/verifica.py
    if str(_p) not in sys.path:
        sys.path.insert(0, str(_p))

PPM = 5.0                      # matching tolerance of a feature to a reference peak (the alignment tolerance of mzFinder)
RT_TOL = 0.15                  # minutes
