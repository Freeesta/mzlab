"""Reference of the substructure matcher of mzlab-mol (crates/mzlab-mol/src/substruct.rs): RDKit matches of synthetic query/target pairs.

    python3 tools/sottostruttura_riferimento.py [OUT.json]   (default crates/mzlab-mol/tests/data/golden_sub.json; needs `pip install rdkit`)

Queries are SMILES or molfiles written here with the query features of a drawing editor (atom lists, generic atoms, query bonds);
targets are SMILES of textbook molecules. For every pair: the distinct sets of target atoms RDKit matches (uniquify), with and
without chirality. Explicit query hydrogens are merged into their neighbour (`MergeQueryHs`), the rule of the Rust matcher.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

from rdkit import Chem, RDLogger

RDLogger.DisableLog("rdApp.*")
OUT = Path(__file__).resolve().parents[1] / "crates/mzlab-mol/tests/data/golden_sub.json"


def v2000(atoms, bonds, props=()):
    """A V2000 molfile: atoms as symbols (or (symbol, charge)), bonds as (a, b, type) 1-based, extra property lines."""
    head = "\n  sub\n\n%3d%3d  0  0  0  0  0  0  0  0999 V2000\n" % (len(atoms), len(bonds))
    rows = []
    for i, a in enumerate(atoms):
        sym = a if isinstance(a, str) else a[0]
        rows.append(f"{1.5 * i:10.4f}{(i % 2) * 0.8:10.4f}{0:10.4f} {sym:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n")
    rows += ["%3d%3d%3d  0  0  0  0\n" % b for b in bonds]
    chg = [(i + 1, a[1]) for i, a in enumerate(atoms) if not isinstance(a, str)]
    if chg:
        rows.append("M  CHG%3d" % len(chg) + "".join("%4d%4d" % c for c in chg) + "\n")
    rows += [p + "\n" for p in props]
    return head + "".join(rows) + "M  END\n"


def als(atom, els, neg=False):
    return "M  ALS%4d%3d %s " % (atom, len(els), "T" if neg else "F") + "".join(f"{e:<4}" for e in els)


def v3000(atoms, bonds):
    lines = ["", "  sub", "", "  0  0  0     0  0            999 V3000", "M  V30 BEGIN CTAB", f"M  V30 COUNTS {len(atoms)} {len(bonds)} 0 0 0", "M  V30 BEGIN ATOM"]
    lines += [f"M  V30 {i + 1} {s} {1.5 * i:.4f} {(i % 2) * 0.8:.4f} 0 0" for i, s in enumerate(atoms)]
    lines += ["M  V30 END ATOM", "M  V30 BEGIN BOND"]
    lines += [f"M  V30 {k + 1} {t} {a} {b}" for k, (a, b, t) in enumerate(bonds)]
    lines += ["M  V30 END BOND", "M  V30 END CTAB", "M  END"]
    return "\n".join(lines) + "\n"


RING6 = [(1, 2, 2), (2, 3, 1), (3, 4, 2), (4, 5, 1), (5, 6, 2), (6, 1, 1)]
AROM6 = [(a, b, 4) for a, b, _ in RING6]
PHENYL_Y = RING6 + [(1, 7, 1)]

ARENES = ["c1ccccc1", "c1ccncc1", "c1cncnc1", "C1CCCCC1", "C1=CCCCC1", "Oc1ccccc1", "Cc1ccccc1", "Clc1ccccc1", "Brc1ccccc1", "Fc1ccccc1", "Nc1ccccc1", "c1ccc2ccccc2c1"]
ACIDS = ["CC(=O)O", "CC(=O)[O-]", "CC(=O)OC", "OC(=O)c1ccccc1", "CC(=O)N", "CC=O", "OCC(=O)O"]
SULFA = ["Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1", "CS(=O)(=O)N", "CS(=O)(=O)O", "NS(N)(=O)=O", "CS(=O)(=O)NC(C)=O", "CSC"]
PHENOLS = ["Oc1ccc(Cl)cc1Cl", "Oc1ccccc1Cl", "Oc1cccc(Cl)c1", "Oc1ccccc1", "Clc1ccccc1Cl", "COc1ccccc1Cl"]
SMALL = ["CO", "CCO", "COC", "C[O-]", "OCCO", "C=O", "CC#N", "C=CC", "CC(C)(C)C", "CC(C)C", "C1CC1", "CC1CC1", "C1CCC1", "[13CH4]", "C", "CC[N+](C)(C)C", "C[C@H](N)O", "C[C@@H](N)O", "CC(N)O"]
ALA = ["C[C@@H](N)C(=O)O", "C[C@H](N)C(=O)O", "N[C@@H](C)C(=O)O", "OC(=O)[C@H](C)N", "C[C@@H](NC)C(=O)O", "CC(N)C(=O)O", "C[C@@](N)(O)C(=O)O"]
ALKENES = ["F/C=C/F", "F/C=C\\F", "FC=CF", "F/C=C/C", "C/C(F)=C/F", "F/C(C)=C/F", "FC=C"]

CASES = [
    ("benzene", "c1ccccc1", ARENES),
    ("pyridine", "c1ccncc1", ARENES),
    ("cyclohexane", "C1CCCCC1", ARENES),
    ("kekule benzene", "C1=CC=CC=C1", ARENES),
    ("naphthalene", "c1ccc2ccccc2c1", ARENES + ["c1ccc2cc3ccccc3cc2c1"]),
    ("aliphatic N matches aromatic n", "N", ARENES),
    ("carbonyl", "C=O", ACIDS),
    ("carboxylic acid", "C(=O)O", ACIDS),
    ("carboxylate charge", "C(=O)[O-]", ACIDS),
    ("ester", "C(=O)OC", ACIDS),
    ("sulfonamide", "S(=O)(=O)N", SULFA),
    ("aryl sulfonamide", "c1ccccc1S(=O)(=O)N", SULFA),
    ("chlorophenol ortho", "Oc1ccccc1Cl", PHENOLS),
    ("chlorophenol any", "Oc1ccccc1", PHENOLS),
    ("chlorobenzene", "Clc1ccccc1", PHENOLS),
    ("anion", "[O-]", SMALL + ACIDS),
    ("isotope", "[13C]", SMALL),
    ("quaternary N", "[N+]", SMALL),
    ("alkene", "C=C", SMALL + ARENES),
    ("nitrile", "C#N", SMALL),
    ("cyclopropane", "C1CC1", SMALL),
    ("isobutane symmetry", "CC(C)C", SMALL),
    ("oxygen", "O", SMALL),
    ("two pieces", "C.O", ["CO", "CCO", "C=O", "OCCO"]),
    ("hydroxyl H", "[H]OC", SMALL),
    ("methyl H3", "[H]C([H])([H])C", SMALL),
    ("chiral amino alcohol", "C[C@H](N)O", SMALL),
    ("alanine", "C[C@@H](N)C(=O)O", ALA),
    ("alanine without stereo", "CC(N)C(=O)O", ALA),
    ("trans difluoroethene", "F/C=C/F", ALKENES),
    ("cis difluoroethene", "F/C=C\\F", ALKENES),
    ("ring list C,N", v2000(["C", "C", "C", "L", "C", "C"], RING6, [als(4, ["C", "N"])]), ARENES),
    ("ring list not C", v2000(["C", "C", "C", "L", "C", "C"], RING6, [als(4, ["C"], True)]), ARENES),
    ("any atom on phenyl", v2000(["C"] * 6 + ["A"], PHENYL_Y), ARENES + PHENOLS),
    ("hetero on phenyl", v2000(["C"] * 6 + ["Q"], PHENYL_Y), ARENES + PHENOLS),
    ("halogen on phenyl", v2000(["C"] * 6 + ["X"], PHENYL_Y), ARENES + PHENOLS),
    ("aromatic bonds ring", v2000(["C"] * 6, AROM6), ARENES),
    ("any bond C~O", v2000(["C", "O"], [(1, 2, 8)]), ACIDS + SMALL),
    ("single or double C-C", v2000(["C", "C"], [(1, 2, 5)]), ["C=C", "CC", "C#C", "c1ccccc1", "C=CC"]),
    ("single or aromatic C-N", v2000(["C", "N"], [(1, 2, 6)]), ["CN", "C=N", "c1ccncc1", "CC#N"]),
    ("double or aromatic C-N", v2000(["C", "N"], [(1, 2, 7)]), ["CN", "C=N", "c1ccncc1", "CC#N"]),
    ("charged list atom molfile", v2000(["C", ("O", -1)], [(1, 2, 1)]), ACIDS + SMALL),
    ("list S,P with oxygens", v2000(["L", "O", "O"], [(1, 2, 2), (1, 3, 2)], [als(1, ["S", "P"])]), SULFA + ["COP(=O)(OC)OC", "O=S=O"]),
    ("V3000 list", v3000(["C", "C", "C", "[C,N]", "C", "C"], RING6), ARENES),
    ("V3000 not list", v3000(["C", "C", "C", "NOT[C,N]", "C", "C"], RING6), ARENES + ["c1ccoc1", "c1ccsc1"]),
    ("V3000 any bond", v3000(["C", "*"], [(1, 2, 8)]), SMALL),
    ("V3000 aromatic ring", v3000(["C"] * 5 + ["N"], AROM6), ARENES),
]


def rdkit_query(text):
    if "\n" in text:
        q = Chem.MolFromMolBlock(text, removeHs=False)
    elif "[H]" in text:
        p = Chem.SmilesParserParams()
        p.removeHs = False
        q = Chem.MolFromSmiles(text, p)
    else:
        q = Chem.MolFromSmiles(text)
    if q is None:
        raise SystemExit(f"RDKit cannot read the query {text!r}")
    return Chem.MergeQueryHs(q) if any(a.GetAtomicNum() == 1 for a in q.GetAtoms()) else q


def matches(q, t, chiral):
    sets = {tuple(sorted(m)) for m in t.GetSubstructMatches(q, uniquify=True, useChirality=chiral, maxMatches=1000)}
    return sorted(list(s) for s in sets)


def main(out=OUT):
    rows = []
    for name, query, targets in CASES:
        q = rdkit_query(query)
        for smi in targets:
            t = Chem.MolFromSmiles(smi)
            rows.append({"name": name, "query": query, "target": smi, "plain": matches(q, t, False), "stereo": matches(q, t, True)})
    Path(out).write_text(json.dumps({"source": "RDKit " + Chem.rdBase.rdkitVersion, "pairs": rows}, indent=0) + "\n")
    print(f"{len(rows)} pairs from {len(CASES)} queries -> {out}")


if __name__ == "__main__":
    main(*sys.argv[1:])
