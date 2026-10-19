"""Adds the mobility table of `timsdata` (Bruker's library, Linux or Windows) to the truth `verita/1` made from the mzML of msconvert.
A tool of the test bench (docs/studi/ROSINA.md F.1). The table is a TSV with a header `scan<TAB>one_over_k0`: one row per mobility scan index, 1/K0 in V s/cm2.
    python3 tools/rosina/verita_timsdata.py FILE.mzML TABELLA.tsv [--timsdata-versione V] [--nativo FILE.d] > FILE.verita.json"""
import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verita


def leggi_tabella(p: Path) -> dict:
    righe = [r.split("\t") for r in p.read_text(encoding="utf-8").splitlines() if r.strip()]
    if not righe or [c.strip() for c in righe[0]] != ["scan", "one_over_k0"]:
        raise SystemExit("the first row must be: scan<TAB>one_over_k0")
    scan, k0 = [], []
    for r in righe[1:]:
        scan.append(int(r[0]))
        k0.append(float(r[1]))
    if not scan:
        raise SystemExit("the table has no rows")
    return {"scan": scan, "one_over_k0": k0}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mzml", type=Path)
    ap.add_argument("tabella", type=Path)
    ap.add_argument("--timsdata-versione", default="?")
    ap.add_argument("--nativo", type=Path)
    a = ap.parse_args(argv)
    d = verita.da_mzml(a.mzml, ("msconvert+timsdata", a.timsdata_versione, "mzML from msconvert; 1/K0 from timsdata"), a.nativo, None)
    d["mobilita"] = leggi_tabella(a.tabella)
    json.dump(d, sys.stdout, indent=1, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
