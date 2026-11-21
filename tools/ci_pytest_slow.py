#!/usr/bin/env python3
"""Rewrites tools/pytest_lenti.txt (file and mean seconds: the files ci_pytest_ordine.py collects first, split between rest-a and rest-b by ci_pytest_meta.py) from the real times of the 3.11 job recorded by stat-ci.

    python3 tools/ci_pytest_slow.py [--data-file eventi.jsonl] [--last 8] [--minimum 5] [--write]   # default data: every events file of origin/stat-ci; without --write it only prints

A file enters the list when its mean time (setup + call + teardown, last runs) is at least `--minimum` seconds; the list is sorted slowest first.
"""
from __future__ import annotations

import argparse
import sys
from collections import defaultdict
from pathlib import Path

import ci_metrics

LIST = Path(__file__).with_name("pytest_lenti.txt")


def lenti(ev: list[dict], ultimi: int = 8, minimo: float = 5.0) -> list[tuple[str, float]]:
    """(file, mean seconds) of the test files whose mean time over the last `ultimi` 3.11 runs with phase data is at least `minimo` seconds, slowest first."""
    # since the 3.11 job runs as shards, a run has several rows (one per shard) with disjoint files: they are merged per run
    rows = [r for r in ev if r.get("tipo") == "test_durate" and r.get("python") == "3.11" and isinstance(r.get("fasi"), dict)]
    per_run: dict = {}
    for r in sorted(rows, key=lambda r: r["ts_utc"]):
        per_run.setdefault(r.get("run_id", r["ts_utc"]), {}).update(r["fasi"])
    runs = list(per_run.values())[-ultimi:]
    times: dict[str, list[float]] = defaultdict(list)
    for phases_run in runs:
        for f, phases in phases_run.items():
            times[f].append(sum(phases))
    media = {f: sum(v) / len(runs) for f, v in times.items()}
    return [(f, round(s, 1)) for f, s in sorted(media.items(), key=lambda x: (-x[1], x[0])) if s >= minimo]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-file")
    ap.add_argument("--last", type=int, default=8)
    ap.add_argument("--minimum", type=float, default=5.0)
    ap.add_argument("--write", action="store_true")
    a = ap.parse_args()
    ev = ci_metrics.read_all(a.data_file) if a.data_file else ci_metrics.branch_events()
    listing = lenti(ev, a.last, a.minimum)
    if not listing:
        sys.exit("no 3.11 run with phase data: the list is left as it is")
    text = "".join(f"{f}\t{s}\n" for f, s in listing)
    print(text, end="")
    if a.write:
        LIST.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
