"""The Crippen descriptors of the RT-logP check of the Banco 2b are the same in the .wasm `attributio` and in the native build (feature `rev`), to 1e-9.
Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
SMILES = [
    "CCO",
    "CC(C)(C)NCC(COc1nsnc1N1CCOCC1)O",
    "CN1C=NC2=C1C(=O)N(C(=O)N2C)C",
    "OC(=O)c1ccccc1O",
    "c1ccc2c(c1)ccc1ccccc12",
    "CC(=O)Nc1ccc(O)cc1.[Na+].[Cl-]",
    "ClC1=CC=C(C=C1)C(Cl)(Cl)C1=CC=C(Cl)C=C1",
    "not a smiles(",
]


def run(*a, env=None):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", env=env).stdout


def test_native_and_wasm_descriptors_agree(tmp_path):
    out = tmp_path / "pkg"
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-attributio-wasm", "--features", "rev")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_attributio_wasm.wasm")
    wasm = json.loads(run("node", "tests/attributio_descr_node.js", str(out), *SMILES))
    native_file = tmp_path / "native.json"
    run("cargo", "test", "--release", "--locked", "-p", "mzlab-attributio-wasm", "--features", "rev", "--test", "descr", "print_for_parity",
        env={**os.environ, "MZLAB_DESCR_SMILES": "\n".join(SMILES), "MZLAB_DESCR_OUT": str(native_file)})
    native = json.loads(native_file.read_text())
    assert len(native) == len(wasm) == len(SMILES) and native[1]["crippen"] is not None and native[-1] == {}
    for n, w in zip(native, wasm):
        assert n.keys() == w.keys()
        for k in n:
            assert n[k] == pytest.approx(w[k], abs=1e-9), k
