"""The classes of candidate sites of +O, -CH2 and -2H (crates/mzlab-feat/src/chem/posizioni) against RDKit on 20 textbook molecules. Skipped without RDKit."""
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

pytest.importorskip("rdkit")
RAIZ = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(RAIZ / "tools"))
import interp_riferimento as ref  # noqa: E402


def test_classes_of_sites_match_rdkit(tmp_path):
    entrada, uscita = tmp_path / "in.json", tmp_path / "out.json"
    entrada.write_text(json.dumps(ref.SMILES))
    env = {**os.environ, "MZLAB_SITES_IN": str(entrada), "MZLAB_SITES_OUT": str(uscita)}
    subprocess.run(
        ["cargo", "test", "--release", "-p", "mzlab-feat", "--features", "interp", "--test", "posizioni_riferimento"],
        cwd=RAIZ, env=env, check=True, capture_output=True,
    )
    rust = {r["smiles"]: r for r in json.loads(uscita.read_text())}
    for r in ref.table():
        for t in ("+O", "-CH2", "-2H"):
            assert list(r[t]) == rust[r["smiles"]][t], (r["smiles"], t)
