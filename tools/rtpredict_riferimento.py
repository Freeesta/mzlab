#!/usr/bin/env python3
"""Golden data for the retention-time prediction of the MS2 engine (SOLO test reference; the engine is Rust).

Replays Attributio `plugins/rt_prediction.py` (Morgan fingerprints radius 2 / 2048 bits with RDKit, RandomForestRegressor(100 trees, random_state=42)) on invented
retention times for the 39 structures of `tests/golden/attributio/sintetica/frammenti.json`: 30 train, 9 test. Writes tests/data/attributio/ms2_engine/rtpredict.json.
Use: python3 tools/rtpredict_riferimento.py   (dev dependencies: pip install rdkit scikit-learn numpy)
"""
import json
from pathlib import Path

import numpy as np
from rdkit import Chem
from rdkit.Chem import AllChem, Descriptors, rdMolDescriptors
from rdkit.DataStructs import ConvertToNumpyArray
from sklearn.ensemble import RandomForestRegressor

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "tests/golden/attributio/sintetica/frammenti.json"
OUT = ROOT / "tests/data/attributio/ms2_engine/rtpredict.json"


def fp(smiles):
    arr = np.zeros((1,), dtype=int)
    ConvertToNumpyArray(AllChem.GetMorganFingerprintAsBitVect(Chem.MolFromSmiles(smiles), 2, nBits=2048), arr)
    return arr


def main():
    rng = np.random.default_rng(7)
    smiles = [m["smiles"] for m in json.loads(SRC.read_text())["molecole"]]
    rows = []
    for s in smiles:
        mol = Chem.MolFromSmiles(s)
        if mol is None:
            continue
        rt = 2.0 + 0.22 * mol.GetNumHeavyAtoms() + 0.7 * rdMolDescriptors.CalcNumAromaticRings(mol) \
            + 0.15 * Descriptors.MolLogP(mol) - 0.4 * rdMolDescriptors.CalcNumHBD(mol) + float(rng.normal(0, 0.15))
        rows.append((s, round(float(rt), 3)))
    order = rng.permutation(len(rows))
    train = [rows[i] for i in order[:30]]
    test = [rows[i] for i in order[30:]]
    rf = RandomForestRegressor(n_estimators=100, random_state=42)
    rf.fit(np.array([fp(s) for s, _ in train]), np.array([r for _, r in train]))
    pred = rf.predict(np.array([fp(s) for s, _ in test]))
    mae = float(np.mean(np.abs(pred - np.array([r for _, r in test]))))
    OUT.write_text(json.dumps({"train": train, "test": test, "sklearn_pred": [float(p) for p in pred], "sklearn_mae": mae}))
    print(len(train), len(test), "mae", round(mae, 3))


if __name__ == "__main__":
    main()
