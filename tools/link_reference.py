"""Independent reference of `link/1` (the outside lookups of the Draw right-click menu): the rows (the preferred PubChem search first) and the URLs,
percent-encoded with `urllib.parse.quote(safe="")` (RFC 3986 unreserved kept, every other UTF-8 byte as %XX).

    python3 tools/link_reference.py [--scrivi]

Writes tests/dati/link_golden.json with --scrivi, otherwise checks that the file holds what this script computes. The Rust test
crates/mzlab-feat/tests/link.rs compares the engine with the file; the cases cover charges, brackets, stereo, isotopes, #, + and /.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from urllib.parse import quote

GOLDEN = Path(__file__).resolve().parent.parent / "tests" / "dati" / "link_golden.json"

CASES = [
    {"smiles": "CCO", "inchi": "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3", "part": False},
    {"smiles": "[NH4+]", "inchi": "InChI=1S/H3N/h1H3/p+1", "part": True},
    {"smiles": "C#N", "inchi": "InChI=1S/CHN/c1-2/h1H", "part": False},
    {"smiles": "N[C@@H](C)C(=O)O", "inchi": "InChI=1S/C3H7NO2/c1-2(4)3(5)6/h2H,4H2,1H3,(H,5,6)/t2-/m0/s1", "part": False},
    {"smiles": "C/C=C/C", "inchi": "InChI=1S/C4H8/c1-3-4-2/h3-4H,1-2H3/b4-3+", "part": True},
    {"smiles": "CC(=O)[O-]", "part": False},
    {"smiles": "[13CH4]", "inchi": " InChI=1S/CH4/h1H4/i1+1 ", "part": True},
    {"smiles": "c1ccccc1Cl", "inchi": "", "part": False},
]


def link(req: dict) -> dict:
    smiles, part = req["smiles"], bool(req.get("part", False))
    inchi = req.get("inchi", "").strip()
    preferred = "pubchem-sub" if part else "pubchem-exact"
    order = [preferred] + [i for i in ("pubchem-exact", "pubchem-similar", "pubchem-sub") if i != preferred]
    tab = {"pubchem-exact": "", "pubchem-similar": "&tab=similarity", "pubchem-sub": "&tab=substructure"}
    pubchem = [{"id": i, "url": "https://pubchem.ncbi.nlm.nih.gov/#query=" + quote(smiles, safe="") + tab[i], "copy": smiles} for i in order]
    chebi = [{"id": "chebi", "url": "https://www.ebi.ac.uk/chebi/search?query=" + quote(inchi, safe=""), "copy": None}] if inchi.startswith("InChI=") else []
    bio = [{"id": "biotransformer", "url": "https://biotransformer.ca/new", "copy": smiles}]
    return {"format": "link/1", "part": part, "preferred": preferred, "links": pubchem + chebi + bio}


def golden() -> str:
    rows = [{"request": c, "link": json.dumps(link(c), separators=(",", ":"), ensure_ascii=False)} for c in CASES]
    return json.dumps(rows, indent=1, ensure_ascii=False) + "\n"


def main() -> int:
    text = golden()
    if "--scrivi" in sys.argv[1:]:
        GOLDEN.write_text(text, encoding="utf-8")
        return 0
    if not GOLDEN.exists() or GOLDEN.read_text(encoding="utf-8") != text:
        print(f"{GOLDEN} differs from the reference: python3 tools/link_reference.py --scrivi", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
