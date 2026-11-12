"""Reference of the CIP descriptors of mzlab-feat (crates/mzlab-feat/src/chem/cip.rs): R/S, r/s and E/Z from the CIP labeler of RDKit.

    python3 tools/cip_riferimento.py [OUT.json]   (default crates/mzlab-feat/tests/data/golden_cip.json; needs `pip install rdkit`)

Molecules are textbook SMILES written here and every stereoisomer of a few scaffolds that can be embedded in 3D (cages enumerated freely
include impossible ones). `smiles` holds every molecule with the labels of `rdCIPLabeler.AssignCIPLabels` (atoms in SMILES order);
`molfile` holds one in four of them as a 2D molfile (CoordGen depiction, wedges from RDKit), the input the Draw page sends.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import rdkit
from rdkit import Chem, RDLogger
from rdkit.Chem import rdCIPLabeler, rdDepictor
from rdkit.Chem.EnumerateStereoisomers import EnumerateStereoisomers, StereoEnumerationOptions

RDLogger.DisableLog("rdApp.*")
rdDepictor.SetPreferCoordGen(True)
OUT = Path(__file__).resolve().parents[1] / "crates/mzlab-feat/tests/data/golden_cip.json"

EXPLICIT = r"""
C[C@H](N)C(=O)O
C[C@@H](N)C(=O)O
N[C@@H](Cc1ccccc1)C(=O)O
N[C@@H](CO)C(=O)O
N[C@@H](CS)C(=O)O
C[C@@H](O)[C@H](N)C(=O)O
CC[C@H](C)[C@H](N)C(=O)O
O=C(O)[C@@H]1CCCN1
C[C@H](Cl)Br
C[C@@H](O)CC
F[C@](Cl)(Br)I
C/C=C/C
C/C=C\C
F/C=C/Cl
Cl/C=C(/Br)C
OC(=O)/C=C/C(=O)O
OC(=O)/C=C\C(=O)O
C/C=C/C(=O)O
CC/C(C)=C(/C)CC
C[S@](=O)c1ccccc1
C[S@@](=O)CC
O=C[C@H](O)[C@@H](O)[C@H](O)[C@H](O)CO
OC[C@H]1O[C@@H](O)[C@H](O)[C@@H](O)[C@@H]1O
OC[C@@H](O)[C@@H](O)[C@H](O)CO
O[C@H]1CC[C@@H](O)CC1
C[C@H]1CC[C@H](C)CC1
C[C@@H]1CCCC[C@H]1C
C[C@@H]1CCCC[C@@H]1C
CC(C)[C@@H]1CC[C@@H](C)C[C@H]1O
C=C[C@@H](C)O
C#C[C@H](C)O
[2H][C@@H](C)O
[2H][C@](C)([1H])O
[13CH3][C@H](O)C
C[C@H](O)c1ccccc1
C[C@H](N)c1ccccn1
C[C@@H](c1ccccc1)c1ccccn1
CN1CCC[C@H]1c1cccnc1
O=C1N[C@@H](C)C(=O)N[C@H]1C
CC(=O)O[C@H]1C[C@@H]2CC[C@H]1C2
C[C@]12CC[C@H]3[C@@H](CCC4=CC(=O)CC[C@@]43C)[C@@H]1CC[C@@H]2O
OC(=O)[C@@H](O)[C@H](O)C(=O)O
OC(=O)[C@@H](O)[C@@H](O)C(=O)O
CC(C)C[C@H](NC(=O)[C@@H](N)Cc1ccccc1)C(=O)O
C/C=C/C=C/C
C/C=C\C=C/C
CC/C=C\CC/C=C\C/C=C\CCC(=O)O
C/N=C/c1ccccc1
O/N=C/C
C[N@+]1(CC)CCCC1
CC[C@@H](C)Cl
OC[C@H](O)CO
C[C@H](O)[C@@H](C)O
C[C@H](O)[C@H](C)O
OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O
N[C@@H](CCCCN)C(=O)O
N[C@@H](CC(=O)O)C(=O)O
CSCC[C@H](N)C(=O)O
C[C@H](CCl)CBr
FC(F)[C@H](F)C
C[C@H]1CCCO1
C[C@@H]1CCOC1=O
O=C1CC[C@@H](C)C1
C1CC[C@H]2CCCC[C@@H]2C1
C1CC[C@@H]2CCCC[C@@H]2C1
CC1=CC[C@@H](CC1)C(C)=C
CC(C)=CCC/C(C)=C/CO
CC(C)=CCC/C(C)=C\CO
OC[C@@H](O)[C@H](O)[C@@H](O)CO
OC[C@@H](O)C(O)[C@H](O)CO
OC[C@@H](O)[C@@H](O)[C@@H](O)CO
C[C@@H](O)C(O)[C@H](C)O
C[C@@H](O)[C@H](O)[C@H](C)O
C[C@@H](O)[C@@H](O)[C@H](C)O
O[C@H]1[C@H](O)[C@@H](O)[C@H](O)[C@@H](O)[C@@H]1O
C[C@H](c1ccncc1)O
C[C@H](c1ncccc1)c1ccccn1
C[C@H](c1ccncc1)c1cnccc1
C[C@H](c1ccccc1)c1ccccn1
O[C@H](c1ccc2ccccc2c1)c1ccc2ccccc2c1
C[C@H](c1ccco1)c1ccccc1
C[C@H](c1ccccn1)c1cccnc1
C[C@H](c1ccco1)c1ccoc1
C[C@H](c1cccs1)c1ccccc1
O[C@@H](c1ccccc1)c1ccccc1C
O[C@@H](c1ccccc1)c1ccc2ccccc2c1
O[C@H](c1ccc2ccccc2c1)c1cccc2ccccc12
CC[P@](=O)(OC)c1ccccc1
C[P@@](c1ccccc1)CC
C[N@@+](CC)(CCC)c1ccccc1
[11CH3][C@H](O)C
[12CH3][C@H](O)C
[13CH3][C@H](O)[12CH3]
[2H]C([2H])[C@H](O)C
CC[C@H](O)C([2H])([2H])[2H]
COc1cc2c(cc1OC)[C@H](Cc1ccc(OC)c(OC)c1)N(C)CC2
CN1[C@H]2CC[C@@H]1C[C@@H](OC(=O)C(CO)c1ccccc1)C2
CN1[C@H]2CC[C@@H]1[C@H](C(=O)OC)[C@@H](OC(=O)c1ccccc1)C2
CC(C)(C)[S@](=O)N
O=C(O)C[C@](O)(CC(=O)O)C(=O)O
OC(=O)C[C@@H](O)C(=O)O
C[C@H](N)[C@@H](C)c1ccccc1
C/C=C/[C@H](C)/C=C\C
C/C=C/[C@@H](C)/C=C/C
C/C=C\[C@@H](C)/C=C/C
CC/C=C/[C@H](C)CC
C1CCC/C=C/CCC1
C1CCC/C=C\CCC1
C/C(Cl)=C(\F)Br
OC/C=C(/C)CC/C=C(/C)C
N#C/C=C/c1ccccc1
O=C(/C=C/c1ccccc1)O
C(=C\c1ccccc1)\c1ccccc1
C(=C/c1ccccc1)\c1ccccc1
Cc1ccc(/C=N/O)cc1
C[C@@H]1C[C@H]2C[C@@H]1C2
C[C@]1(O)CC[C@H](C(C)C)CC1
C[C@@]1(CCCC1)[C@H](C)O
OC1C[C@H](O)C[C@@H](O)C1
O[C@H]1C[C@@H](O)C[C@H](O)C1
C[C@H]1C[C@@H](C)C[C@@H](C)C1
C[C@H]1C[C@H](C)C[C@@H](C)C1
N[C@@H](Cc1c[nH]c2ccccc12)C(=O)O
N[C@@H](Cc1cnc[nH]1)C(=O)O
N[C@@H](Cc1ccc(O)cc1)C(=O)O
N[C@@H](CCC(=O)N)C(=O)O
N[C@@H](CCCNC(=N)N)C(=O)O
CC(C)[C@H](N)C(=O)O
O=C1O[C@H]([C@@H](O)CO)C(O)=C1O
CC(=O)N[C@@H]1[C@@H](O)C[C@@](O)(C(=O)O)O[C@H]1[C@H](O)[C@H](O)CO
C[C@H]1O[C@@H](O[C@@H]2[C@@H](CO)O[C@H](O)[C@H](O)[C@H]2O)[C@H](O)[C@@H](O)[C@@H]1O
CC1(C)S[C@@H]2[C@H](NC(=O)Cc3ccccc3)C(=O)N2[C@H]1C(=O)O
CN[C@@H](C)[C@H](O)c1ccccc1
CC(C)NC[C@H](O)COc1cccc2ccccc12
C[C@H](CCC(=O)O)[C@H]1CC[C@H]2[C@@H]3[C@H](O)C[C@@H]4C[C@H](O)CC[C@]4(C)[C@H]3C[C@H](O)[C@@]21C
CC1(C)[C@@H]2CC[C@@]1(C)C(=O)C2
CC1=CC[C@@H]2C[C@H]1C2(C)C
CC1(C)[C@H]2CC[C@]1(C)[C@@H](O)C2
C=C[C@H]1CN2CC[C@H]1C[C@@H]2[C@@H](O)c1ccnc2ccc(OC)cc12
CN1CC[C@]23c4c5ccc(O)c4O[C@H]2[C@@H](O)C=C[C@H]3[C@H]1C5
C[C@H]1[C@H]2CC[C@@H](C)[C@H]2[C@H](O)C1
O[C@@H]1C[C@H]2CC[C@@H]1C2
O[C@H]1C[C@H]2CC[C@@H]1C2
C1C[C@H]2C[C@@H]1[C@@H]1CCC[C@H]21
C[C@@H]1C[C@@H]2C[C@H]1C2
C[C@H]1C[C@@H]2C[C@H]1C2
CC(=O)[C@H]1CC[C@H]2[C@@H]3CC=C4C[C@@H](O)CC[C@]4(C)[C@H]3CC[C@]12C
C[C@]12CC[C@@H]3c4ccc(O)cc4CC[C@H]3[C@@H]1CC[C@@H]2O
OC[C@H]1O[C@@H](n2cnc3c(N)ncnc32)[C@H](O)[C@@H]1O
Cc1cn([C@H]2C[C@H](N=[N+]=[N-])[C@@H](CO)O2)c(=O)[nH]c1=O
CC(C)C[C@@H](C(=O)O)c1ccc(cc1)C
C[C@@H](C(=O)O)c1ccc(CC(C)C)cc1
COc1ccc2cc([C@H](C)C(=O)O)ccc2c1
CN[C@@H]1CC[C@@H](c2ccc(Cl)c(Cl)c2)c2ccccc21
O=C(O)[C@H]1CSC(=N1)c1ccccc1
C[C@H]1CC[C@@H]2[C@@H](C)C(=O)O[C@@H]3O[C@]4(C)CC[C@H]1[C@]32OO4
CC(C)[C@@H]1CC[C@@H](C)C[C@@H]1O
CC(C)[C@H]1CC[C@@H](C)C[C@@H]1O
C1C[C@@H]2CC[C@H]1CC2
C[C@H]1C[C@@H]1C
C[C@H]1C[C@H]1C
OC(=O)[C@@H]1C[C@H]1C(=O)O
OC(=O)[C@@H]1C[C@@H]1C(=O)O
C[C@@H]1C[C@H](C)[C@H](C)C1
C[C@@H]1C[C@H](C)[C@@H](C)C1
O=C1C[C@@H]2CC[C@H]1C2
N[C@H]1C[C@@H]2CC[C@H]1C2
C[C@@H]1CC[C@@H](C)O1
C[C@@H]1CC[C@H](C)O1
OC[C@@H]1C[C@H]2C=C[C@@H]1C2
C[C@H](CC=C(C)C)CCO
CC(C)=CCC[C@@H](C)CC=O
N[C@H](C(=O)O)[C@@H](C)O
OC(=O)C[C@@H](O)C[C@@H](O)C(=O)O
OC(=O)C[C@@H](O)C[C@H](O)C(=O)O
OC[C@@H](O)[C@H](O)[C@H](O)[C@H](O)CO
OC[C@@H](O)[C@H](O)[C@@H](O)[C@H](O)CO
C[C@H](Br)[C@@H](C)Br
C[C@H](Br)[C@H](C)Br
C[C@H](Br)C(Br)[C@@H](C)Br
C[C@H](Br)[C@@H](Br)[C@@H](C)Br
C[C@H](Br)[C@H](Br)[C@@H](C)Br
""".split()

SCAFFOLDS = r"""
CC(O)C(O)C(O)C(O)CO
OCC1OC(O)C(O)C(O)C1O
CC1CCC(C)CC1O
CC(Br)C(Br)C(Br)C
OC(=O)C(O)C(O)C(O)C(O)C(=O)O
C1CC2CCC1C2O
CC1CC2CC1C2
OC1C(O)C(O)C(O)C(O)C1O
CC=CC(C)C=CC
CC=CC=CC(O)C=CC
CC(N)C(c1ccncc1)O
CC(c1ccc[nH]1)c1ccco1
CC(c1cncnc1)c1ccncn1
CC1CC(O)CC(C)C1
NC1CCCCC1N
CC1OC(C)OC(C)O1
CC1CCC2C(C)C(=O)OC2C1
CC(C)(C)S(=O)CC(C)O
CP(=O)(OC)C(C)O
CC1CN(C)CC(C)N1
CC=C(C)C(C)=CC
OC1CC2CC1C1CCCC21
CC(O)C1CCC(C(C)O)CC1
CC(c1ccc2ncccc2c1)c1cccc2ncccc12
CC(O)(c1ccco1)c1ccoc1
CC(c1nccs1)c1ncco1
CC=NC(C)C=NC
ON=C(C)C(C)C=CC
CC(Cl)C(Cl)C(Cl)C(Cl)C
FC(Cl)C(F)(Cl)C(F)Cl
CC1(O)CCC(C)(O)CC1
OC1C2CC3CC1CC(C2)C3
OC1C2CC3CC(C2)CC1C3O
CC1(C)C2CCC1(C)C(O)C2
CC1C(C)C(C)C1C
""".split()


def labels(m):
    """CIP labels of a molecule: [[atom, label]] and [[a, b, label]] with a < b."""
    rdCIPLabeler.AssignCIPLabels(m)
    atoms = [[a.GetIdx(), a.GetProp("_CIPCode")] for a in m.GetAtoms() if a.HasProp("_CIPCode")]
    bonds = []
    for b in m.GetBonds():
        if b.HasProp("_CIPCode"):
            x, y = sorted((b.GetBeginAtomIdx(), b.GetEndAtomIdx()))
            bonds.append([x, y, b.GetProp("_CIPCode")])
    return atoms, sorted(bonds)


def molecules():
    seen, out = set(), []
    opts = StereoEnumerationOptions(onlyUnassigned=True, maxIsomers=16, unique=True, tryEmbedding=True)
    found = list(EXPLICIT)
    for s in SCAFFOLDS:
        found += sorted(Chem.MolToSmiles(i) for i in EnumerateStereoisomers(Chem.MolFromSmiles(s), options=opts))
    for s in found:
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    smiles, molfiles = [], []
    for i, s in enumerate(molecules()):
        m = Chem.MolFromSmiles(s)
        atoms, bonds = labels(m)
        smiles.append({"smiles": s, "atoms": atoms, "bonds": bonds})
        if i % 4 == 0:
            rdDepictor.Compute2DCoords(m)
            molfiles.append({"smiles": s, "molfile": Chem.MolToMolBlock(m), "atoms": atoms, "bonds": bonds})
    doc = {"source": f"RDKit {rdkit.__version__} rdCIPLabeler", "smiles": smiles, "molfile": molfiles}
    out.write_text(json.dumps(doc, indent=0) + "\n")
    print(f"{out}: {len(smiles)} molecules, {len(molfiles)} molfiles")


if __name__ == "__main__":
    main()
