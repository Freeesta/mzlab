"""Independent reference of `anal/1` (Draw analysis card): average mass, mass % of every element and the isotope pattern grouped by nominal mass.

    python3 tools/analisi_riferimento.py [--scrivi]

Own tables (AME2020 masses, IUPAC abundances), exact convolution without pruning, `math.fsum` sums (the same bytes on every Python); writes tests/dati/anal_golden.json with --scrivi, otherwise
checks that the file holds what this script computes. The Rust test crates/mzlab-feat/tests/analysis.rs compares the engine with the file.
"""
from __future__ import annotations

import json
import math
import re
import sys
from pathlib import Path

GOLDEN = Path(__file__).resolve().parent.parent / "tests" / "dati" / "anal_golden.json"
ELECTRON = 0.000548579909
MIN_REL = 0.01

# element -> [(mass number, exact mass, abundance)]
NAT = {
    "H": [(1, 1.00782503223, 0.999885), (2, 2.01410177812, 0.000115)],
    "C": [(12, 12.0, 0.9893), (13, 13.00335483507, 0.0107)],
    "N": [(14, 14.00307400443, 0.99636), (15, 15.00010889888, 0.00364)],
    "O": [(16, 15.99491461957, 0.99757), (17, 16.99913175650, 0.00038), (18, 17.99915961286, 0.00205)],
    "F": [(19, 18.99840316273, 1.0)],
    "Na": [(23, 22.9897692820, 1.0)],
    "P": [(31, 30.97376199842, 1.0)],
    "S": [(32, 31.9720711744, 0.9499), (33, 32.9714589098, 0.0075), (34, 33.967867004, 0.0425), (36, 35.96708071, 0.0001)],
    "Cl": [(35, 34.968852682, 0.7576), (37, 36.965902602, 0.2424)],
    "Br": [(79, 78.9183376, 0.5069), (81, 80.9162897, 0.4931)],
    "I": [(127, 126.9044719, 1.0)],
}

# name, formula with drawn isotopes as [13C] / D, charge
CASES = [
    ("methane", "CH4", 0), ("13C-methane", "[13C]H4", 0), ("methanol-d3", "CD3OH", 0), ("caffeine", "C8H10N4O2", 0),
    ("paracetamol", "C8H9NO2", 0), ("paracetamol+H", "C8H10NO2", 1), ("glyphosate", "C3H8NO5P", 0), ("sulfamethoxazole", "C10H11N3O3S", 0),
    ("chlorobenzene", "C6H5Cl", 0), ("2,4-dichlorophenol", "C6H4Cl2O", 0), ("triclosan", "C12H7Cl3O2", 0), ("bromobenzene", "C6H5Br", 0),
    ("1,2-dibromoethane", "C2H4Br2", 0), ("5-fluorouracil", "C4H3FN2O2", 0), ("PFOA", "C8HF15O2", 0), ("ibuprofen-", "C13H17O2", -1),
    ("malonate2-", "C3H2O4", -2), ("tetramethylammonium+", "C4H12N", 1), ("iodobenzene", "C6H5I", 0), ("sodium-acetate", "C2H3NaO2", 0),
    ("13C6-benzene", "[13C]6H6", 0), ("15N-urea", "C[15N]2H4O", 0),
]


def parse(f: str) -> list[tuple[str, int, int]]:
    """(element, mass number or 0, count); D is hydrogen 2."""
    out = []
    for iso, el, n in re.findall(r"(?:\[(\d+))?([A-Z][a-z]?)\]?(\d*)", f):
        a = int(iso) if iso else 0
        if el == "D":
            el, a = "H", 2
        out.append((el, a, int(n) if n else 1))
    return out


def isotopes(el: str, a: int) -> list[tuple[int, float, float]]:
    if a:
        return [(a, next(m for k, m, _ in NAT[el] if k == a), 1.0)]
    return NAT[el]


def convolve(x: dict, y: dict) -> dict:
    """mass number -> (probability, probability x mass)"""
    out: dict = {}
    for ka, (pa, ma) in x.items():
        for kb, (pb, mb) in y.items():
            p0, m0 = out.get(ka + kb, (0.0, 0.0))
            out[ka + kb] = (p0 + pa * pb, m0 + ma * pb + mb * pa)
    return out


def request(f: str, q: int) -> dict:
    """atoms as the page sends them: every atom explicit (H too), the charge on the first one"""
    atoms = []
    for el, a, n in parse(f):
        for _ in range(n):
            atoms.append({"el": "D" if (el, a) == ("H", 2) else el, "iso": a if a and el != "H" else 0})
    atoms[0]["q"] = q
    return {"atoms": atoms}


def analyse(f: str, q: int) -> dict:
    per: dict[str, list] = {}
    dist = {0: (1.0, 0.0)}
    for el, a, n in parse(f):
        iso = isotopes(el, a)
        avg = math.fsum(m * p for _, m, p in iso)
        c = per.setdefault(el, [0, 0.0])
        c[0] += n
        c[1] += n * avg
        one = {k: (p, m * p) for k, m, p in iso}
        for _ in range(n):
            dist = convolve(dist, one)
    average = math.fsum(c[1] for c in per.values())
    order = sorted(per, key=lambda e: ({"C": 0, "H": 1}.get(e, 2), e))
    top, total, first = max(p for p, _ in dist.values()), math.fsum(p for p, _ in dist.values()), min(dist)
    sticks = []
    for k in sorted(dist):
        p, pm = dist[k]
        if 100 * p / top < MIN_REL:
            continue
        m = pm / p
        sticks.append({"off": k - first, "mz": m if q == 0 else (m - q * ELECTRON) / abs(q), "rel": 100 * p / top, "abund": 100 * p / total})
    return {"average": average, "composition": [{"el": e, "count": per[e][0], "pct": 100 * per[e][1] / average} for e in order], "pattern": sticks}


def build() -> list[dict]:
    return [{"name": n, "formula": f, "charge": q, "request": request(f, q), "expect": analyse(f, q)} for n, f, q in CASES]


def main() -> int:
    cases = build()
    text = json.dumps(cases, indent=1) + "\n"
    if "--scrivi" in sys.argv:
        GOLDEN.write_text(text, encoding="utf-8")
        return 0
    if GOLDEN.read_text(encoding="utf-8") != text:
        print(f"{GOLDEN} differs from the reference: run with --scrivi")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
