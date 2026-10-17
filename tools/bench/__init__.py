"""Benchmark of mzFinder against other feature-table tools: the common table `bench/1`, the truths, the metrics P1-P5, the statistics and the report.

Only the quality of the results is measured (never time or memory). Reuses `tools/valida/` (matching, JSON writer, native run of the pipeline of the site)
without changing it. Modules: adatta (native outputs -> `bench/1`, run of mzFinder on a synthetic series), verita (truths -> common format), metriche
(P1-P5 and secondary numbers, per compound), statistica (cluster bootstrap, exact McNemar, Holm, non-inferiority), rapporto (deterministic JSON + Markdown)."""
from __future__ import annotations

import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parents[1]
ROOT = TOOLS.parent
for _p in (TOOLS, ROOT):                               # `valida` (which puts `mzlab` on the path) and `bench` itself
    if str(_p) not in sys.path:
        sys.path.insert(0, str(_p))

FORMATO = "bench/1"
PPM = 5.0                      # matching tolerance of a row to a true compound
RT_TOL = 0.15                  # minutes (0.2 on D1 once the RT of its truth is checked)
