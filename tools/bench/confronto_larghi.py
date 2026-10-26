#!/usr/bin/env python3
"""Comparison table of the smeared-peak lot: the endpoints of `metriche.larghi` for each tool/configuration, one column per table.

    python3 tools/bench/confronto_larghi.py --verita OUT/verita.json --tabelle "MZmine default=mzmine_default.json" "MZmine wide=mzmine_wide.json" "mzLab default=mzlab_default.json"
The tables are `bench/1` (written by `corri_<tool>.py`); the column order is the order given. It writes numbers only."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
    __package__ = "tools.bench"

from . import metriche  # noqa: E402
from .adatta import leggi  # noqa: E402
from .verita import da_json  # noqa: E402

RIGHE = (("P1_narrow (recall)", "P1_narrow", "richiamo", "{:.2f}"), ("P1_broad (recall)", "P1_broad", "richiamo", "{:.2f}"),
         ("P2_broad (median rel. area error)", "P2_broad", "mediana", "{:.2f}"), ("P3_split (rows per band, mean)", "P3_split", "media", "{:.2f}"),
         ("P3_split (bands split in >1 rows)", "P3_split", "divise", "{:.2f}"), ("P5_fp (spurious rows)", "P5_fp", "spurie", "{:d}"),
         ("P5_fp (share of the rows)", "P5_fp", "frazione", "{:.2f}"))


def confronto(verita: list[dict], tabelle: dict[str, dict]) -> dict:
    """{column name: endpoints} for the tables given, in order."""
    return {nome: metriche.larghi(verita, t) for nome, t in tabelle.items()}


def a_markdown(misure: dict[str, dict]) -> str:
    """Markdown table: one row per endpoint, one column per tool; '-' where the endpoint cannot be measured."""
    cella = lambda m, p, k, f: "-" if m[p][k] is None else f.format(m[p][k])
    out = ["| endpoint | " + " | ".join(misure) + " |", "|---|" + "---|" * len(misure)]
    out += [f"| {nome} | " + " | ".join(cella(m, p, k, f) for m in misure.values()) + " |" for nome, p, k, f in RIGHE]
    return "\n".join(out) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--verita", required=True)
    ap.add_argument("--tabelle", nargs="+", required=True, metavar="NOME=FILE")
    a = ap.parse_args()
    tab = {n: leggi(p) for n, p in (x.split("=", 1) for x in a.tabelle)}
    print(a_markdown(confronto(da_json(a.verita), tab)), end="")


if __name__ == "__main__":
    main()
