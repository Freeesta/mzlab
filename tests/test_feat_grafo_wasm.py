"""The structure bridge of mzFinder in WebAssembly (crates/mzlab-feat-grafo-wasm) answers a SMILES with the formula, the groups and the cleavage products that
the Rust library gives natively (checked by the formula and the sets), and a bad SMILES with a JSON error. Skipped without cargo, the wasm32 target,
wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PARENT = "CC(C)(C)NCC(COc1nsnc1N1CCOCC1)O"


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("grafo")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-grafo-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_feat_grafo_wasm.wasm")
    return out


def analyse(pkg, smiles):
    return json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), smiles))


def test_parent_through_wasm(pkg):
    r = analyse(pkg, PARENT)
    assert r["format"] == "grafo/1" and r["formula"] == "C13H24N4O3S"
    assert {"n_tbutyl", "ring_hetero_saturated", "alcohol", "ether", "sulfide"} <= set(r["groups"])
    assert "C4H11N" in r["fragments"] and r["fragments"] == sorted(set(r["fragments"]))
    assert all(c["kind"] in ("cut", "ether", "ester", "amide", "sulfonamide") for c in r["cuts"])


def test_error_is_json(pkg):
    assert analyse(pkg, "C1CC")["error_key"] == "error.grafo.smiles"
    assert analyse(pkg, "C.C")["error_key"] == "error.grafo.mixture"
