#!/usr/bin/env python3
"""Reference for the candidate sites of `chem::posizioni` (test only, never in the site): with RDKit, lists the atoms where +O, -CH2 and -2H can sit on
20 textbook molecules, as the number of atoms per kind and the symmetry classes (Morgan ranks with breakTies=False), to compare with the Rust side."""
import json
import sys

SMILES = [
    "CCc1ccc(O)cc1", "CCCC", "CCCO", "COc1ccccc1", "CN(C)c1ccccc1", "CCN(CC)CC", "c1ccc2ccccc2c1", "Cc1ccccc1C",
    "CC(C)Cc1ccc(C(C)C(=O)O)cc1", "CSC", "CCOC(=O)C", "c1ccncc1", "CC(C)(C)NCC(O)COc1ccccc1", "C1CCOCC1",
    "COc1ccc(CCN)cc1", "CCCCN", "c1ccc(Cl)cc1", "OCC(O)CO", "CN1CCCC1", "CCSCC",
]


def sites(m, transform):
    """Atom indices (and bonds for -2H) as the Rust `sites()` defines them."""
    out = []
    for a in m.GetAtoms():
        h, i = a.GetTotalNumHs(), a.GetIdx()
        if transform == "+O":
            if a.GetIsAromatic() and a.GetSymbol() == "C" and h:
                out.append(("aromatic_oh", i))
            elif a.GetSymbol() == "C" and not a.GetIsAromatic() and h and a.GetHybridization().name == "SP3":
                out.append(("aliphatic_oh", i))
            elif a.GetSymbol() == "S" and not a.GetIsAromatic() and a.GetDegree() == 2:
                out.append(("s_oxide", i))
            elif a.GetSymbol() == "N" and not a.GetIsAromatic() and a.GetDegree() == 3 and not any(
                    n.GetSymbol() == "C" and any(b.GetBondTypeAsDouble() == 2 for b in n.GetBonds()) for n in a.GetNeighbors()):
                out.append(("n_oxide", i))
        elif transform == "-CH2":
            if a.GetSymbol() == "C" and a.GetDegree() == 1 and a.GetNeighbors()[0].GetSymbol() in "NOS":
                out.append(("demethyl", i))
    if transform == "-2H":
        for b in m.GetBonds():
            x, y = b.GetBeginAtom(), b.GetEndAtom()
            ok = lambda t: t.GetSymbol() == "C" and t.GetHybridization().name == "SP3" and t.GetTotalNumHs() >= 1
            if b.GetBondTypeAsDouble() == 1 and ok(x) and ok(y):
                out.append(("dehydro", min(x.GetIdx(), y.GetIdx()), max(x.GetIdx(), y.GetIdx())))
    return out


def classes(m, transform):
    """The sites grouped by (kind, symmetry class of their atoms)."""
    from rdkit import Chem
    ranks = list(Chem.CanonicalRankAtoms(m, breakTies=False))
    groups = {}
    for s in sites(m, transform):
        key = (s[0],) + tuple(sorted(ranks[i] for i in s[1:]))
        groups.setdefault(key, []).append(s[1:])
    return len(groups), sum(len(v) for v in groups.values())


def table():
    from rdkit import Chem
    rows = []
    for smi in SMILES:
        m = Chem.MolFromSmiles(smi)
        rows.append({"smiles": smi, **{t: classes(m, t) for t in ("+O", "-CH2", "-2H")}})
    return rows


if __name__ == "__main__":
    try:
        print(json.dumps(table()))
    except ImportError:
        print("RDKit not installed", file=sys.stderr)
        sys.exit(2)
