"""Reference for `api/isotope_fine` (crates/mzlab-core/tests/isofine_parity.rs): the fine isotope structure of ions, without a cut above the lightest isotopologue.
Usage: python3 tools/isotopi_fini_riferimento.py OUT.json  (list of {query, peaks: [[m/z, %], ...]})."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from mzlab.chem.isoscore import fine_pattern  # noqa: E402

CASES = [
    ({"C": 13, "H": 25, "N": 4, "O": 3, "S": 1}, 1, 0.1),
    ({"C": 20, "H": 30, "Cl": 3, "Br": 1}, 1, 0.1),
    ({"C": 12, "H": 10, "Sn": 2, "Se": 1}, 1, 0.1),
    ({"C": 6, "H": 5, "Hg": 1, "Fe": 1, "Cu": 1}, -1, 0.5),
    ({"C": 400, "H": 650, "N": 110, "O": 120, "S": 6}, 3, 0.1),     # a macromolecule: wider than 6.5 Da
    ({"C": 9, "H": 11, "N": 1, "O": 2}, 2, 0.001),
]


def main():
    out = []
    for n, z, mn in CASES:
        q = "?n=" + ",".join(f"{e}:{k}" for e, k in n.items()) + f"&z={z}&min={mn}"
        out.append({"query": q, "peaks": [[r["mz"], r["rel"]] for r in fine_pattern(n, z, mn, cut=float("inf"))]})
    Path(sys.argv[1]).write_text(json.dumps(out))


if __name__ == "__main__":
    main()
