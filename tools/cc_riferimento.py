#!/usr/bin/env python3
"""Reference data for crates/mzlab-chem (tool only, never in the site).

For the 20 public molecules of docs/studi/CC_MOTORE_NEL_BROWSER.md section 6: RDKit ETKDGv3 + MMFF94 geometry (seed 42) and tblite GFN2-xTB
charges of N, N+1 and N-1 electrons, in ALPB water and in the gas phase, with f(+) = q(N) - q(N+1) and f(-) = q(N-1) - q(N).
Atoms are in the order of `mzlab_feat::chem::grafo::Mol` (heavy atoms in SMILES order) followed by the hydrogens of each heavy atom in turn.

Needs a venv with tblite 0.7.0, rdkit and numpy:
    python3 tools/cc_riferimento.py [crates/mzlab-chem/tests/data/cc_ref.json]
"""
import json
import sys
import time

import numpy as np
from rdkit import Chem, rdBase
from rdkit.Chem import AllChem
from importlib.metadata import version as pkg_version
from tblite.interface import Calculator

SEED = 42
BOHR = 1 / 0.52917721092

MOLS = {
    "phenol": "Oc1ccccc1",
    "anisole": "COc1ccccc1",
    "toluene": "Cc1ccccc1",
    "benzoic_acid": "OC(=O)c1ccccc1",
    "nitrobenzene": "[O-][N+](=O)c1ccccc1",
    "chlorobenzene": "Clc1ccccc1",
    "aniline": "Nc1ccccc1",
    "naphthalene": "c1ccc2ccccc2c1",
    "carbamazepine": "NC(=O)N1c2ccccc2C=Cc2ccccc21",
    "paracetamol": "CC(=O)Nc1ccc(O)cc1",
    "diclofenac": "OC(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl",
    "sulfamethoxazole": "Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1",
    "atrazine": "CCNc1nc(Cl)nc(NC(C)C)n1",
    "bisphenol_a": "CC(C)(c1ccc(O)cc1)c1ccc(O)cc1",
    "caffeine": "Cn1cnc2c1c(=O)n(C)c(=O)n2C",
    "triclosan": "Oc1cc(Cl)ccc1Oc1ccc(Cl)cc1Cl",
    "ibuprofen": "CC(C)Cc1ccc(C(C)C(=O)O)cc1",
    "metoprolol": "COCCc1ccc(OCC(O)CNC(C)C)cc1",
    "benzophenone_3": "COc1ccc(C(=O)c2ccccc2)c(O)c1",
    "imidacloprid": "Clc1ccc(CN2CCNC2=N[N+](=O)[O-])cn1",
}


def charges(z, x, charge, solvated):
    """Mulliken charges of GFN2-xTB for the given total charge (closed or open shell by electron parity)."""
    uhf = (int(z.sum()) - charge) % 2
    calc = Calculator("GFN2-xTB", z, x * BOHR, charge=charge, uhf=uhf)
    calc.set("verbosity", 0)
    if solvated:
        calc.add("alpb-solvation", "water")
    return calc.singlepoint().get("charges")


def geometry(smiles):
    mol = Chem.AddHs(Chem.MolFromSmiles(smiles))
    params = AllChem.ETKDGv3()
    params.randomSeed = SEED
    if AllChem.EmbedMolecule(mol, params) != 0:
        raise RuntimeError("embedding failed: " + smiles)
    AllChem.MMFFOptimizeMolecule(mol, maxIters=2000)
    z = np.array([a.GetAtomicNum() for a in mol.GetAtoms()])
    bonds = [[b.GetBeginAtomIdx(), b.GetEndAtomIdx(), b.GetBondTypeAsDouble()] for b in mol.GetBonds()]
    return z, mol.GetConformer().GetPositions(), bonds


def main(path):
    out = {
        "meta": {
            "tblite": pkg_version("tblite"),
            "rdkit": rdBase.rdkitVersion,
            "method": "GFN2-xTB, Mulliken charges; f(+) = q(N) - q(N+1), f(-) = q(N-1) - q(N); ALPB water and gas phase",
            "geometry": "RDKit ETKDGv3 + MMFF94, randomSeed %d, Angstrom" % SEED,
            "command": "python3 tools/cc_riferimento.py " + path,
        },
        "molecules": {},
    }
    for name, smiles in MOLS.items():
        z, x, bonds = geometry(smiles)
        rec = {"smiles": smiles, "z": z.tolist(), "xyz": x.tolist(), "bonds": bonds}
        for label, solvated in (("gas", False), ("alpb", True)):
            t0 = time.time()
            q0, qc, qa = charges(z, x, 0, solvated), charges(z, x, 1, solvated), charges(z, x, -1, solvated)
            rec[label] = {
                "q": q0.tolist(),
                "q_cation": qc.tolist(),
                "q_anion": qa.tolist(),
                "f_plus": (q0 - qa).tolist(),
                "f_minus": (qc - q0).tolist(),
                "seconds": round(time.time() - t0, 3),
            }
        out["molecules"][name] = rec
        print(name, len(z), "atoms", file=sys.stderr)
    with open(path, "w") as f:
        json.dump(out, f, separators=(",", ":"))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "crates/mzlab-chem/tests/data/cc_ref.json")
