#!/usr/bin/env python3
"""Rewrites tools/pytest_lenti.txt (the files ci_pytest_ordine.py collects first) from the real times of the 3.11 job recorded by stat-ci.

    python3 tools/ci_pytest_lenti.py [--dati eventi.jsonl] [--ultimi 8] [--minimo 5] [--scrivi]   # default data: git show origin/stat-ci:eventi.jsonl; without --scrivi it only prints

A file enters the list when its mean time (setup + call + teardown, last runs) is at least `--minimo` seconds; the list is sorted slowest first.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

LISTA = Path(__file__).with_name("pytest_lenti.txt")


def lenti(ev: list[dict], ultimi: int = 8, minimo: float = 5.0) -> list[str]:
    """Test files whose mean time over the last `ultimi` 3.11 runs with phase data is at least `minimo` seconds, slowest first."""
    # since the 3.11 job runs as shards, a run has several rows (one per shard) with disjoint files: they are merged per run
    righe = [r for r in ev if r.get("tipo") == "test_durate" and r.get("python") == "3.11" and isinstance(r.get("fasi"), dict)]
    per_run: dict = {}
    for r in sorted(righe, key=lambda r: r["ts_utc"]):
        per_run.setdefault(r.get("run_id", r["ts_utc"]), {}).update(r["fasi"])
    runs = list(per_run.values())[-ultimi:]
    tempi: dict[str, list[float]] = defaultdict(list)
    for fasi_run in runs:
        for f, fasi in fasi_run.items():
            tempi[f].append(sum(fasi))
    media = {f: sum(v) / len(runs) for f, v in tempi.items()}
    return [f for f, s in sorted(media.items(), key=lambda x: (-x[1], x[0])) if s >= minimo]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dati")
    ap.add_argument("--ultimi", type=int, default=8)
    ap.add_argument("--minimo", type=float, default=5.0)
    ap.add_argument("--scrivi", action="store_true")
    a = ap.parse_args()
    txt = Path(a.dati).read_text(encoding="utf-8") if a.dati else subprocess.run(["git", "show", "origin/stat-ci:eventi.jsonl"], capture_output=True, text=True).stdout
    ev = [json.loads(x) for x in txt.splitlines() if x.strip()]
    elenco = lenti(ev, a.ultimi, a.minimo)
    if not elenco:
        sys.exit("no 3.11 run with phase data: the list is left as it is")
    print("\n".join(elenco))
    if a.scrivi:
        LISTA.write_text("\n".join(elenco) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
