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


ETHANOL = "\n  qqq\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n" + "".join(f"{x:10.4f}{0:10.4f}{0:10.4f} {el:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for x, el in ((0, "C"), (1, "C"), (2, "O"))) + "  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END"


def test_fragments_of_a_molfile(pkg):
    r = json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), ETHANOL, "1", "2"))
    (cut,) = r["cuts"]
    assert r["format"] == "frag/1" and (cut["a"], cut["b"]) == (1, 2)
    assert [s["formula"] for s in cut["sides"]] == ["C2H6", "H2O"] and cut["sides"][0]["atoms"] == [0, 1]
    assert cut["sides"][1]["mz"] == pytest.approx(19.0178, abs=1e-3)
    assert len(json.loads(run("node", "tests/feat_grafo_node.js", str(pkg), ETHANOL, "-1", "-1"))["cuts"]) == 2
