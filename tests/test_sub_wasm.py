"""The substructure .wasm (crates/mzlab-sub-wasm) answers a request with the same `sub/1` bytes as `mzlab-cli mol-search --sub` (the page = the
command), for a SMILES query and for a molfile query with an atom list and a query bond, and a bad query with a keyed JSON error.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
TARGETS = ["Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1", "Oc1ccc(Cl)cc1Cl", "c1ccncc1", "C[C@@H](N)C(=O)O", "CCO"]
# a six-membered ring with [C,N] at the fourth atom and an any-type bond to a seventh atom
RING = "\n  sub\n\n  7  7  0  0  0  0  0  0  0  0999 V2000\n" + "".join(f"{1.5 * i:10.4f}{(i % 2) * 0.8:10.4f}{0:10.4f} {s:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for i, s in enumerate("CCCLCCA")) + "".join("%3d%3d%3d  0  0  0  0\n" % b for b in ((1, 2, 2), (2, 3, 1), (3, 4, 2), (4, 5, 1), (5, 6, 2), (6, 1, 1), (1, 7, 8))) + "M  ALS   4  2 F C   N   \nM  END\n"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("sub")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-sub-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d), "target/wasm32-unknown-unknown/release/mzlab_sub_wasm.wasm")
    return d


def wasm(pkg, req, tmp_path):
    f = tmp_path / "req.json"
    f.write_text(json.dumps(req))
    return subprocess.run(["node", str(ROOT / "tests" / "sub_wasm_node.js"), str(pkg), str(f)], capture_output=True, encoding="utf-8")


@pytest.mark.parametrize("query,stereo", [("S(=O)(=O)N", False), ("Oc1ccccc1Cl", False), ("C[C@@H](N)C=O", True), (RING, False)])
def test_wasm_equals_the_command(pkg, tmp_path, query, stereo):
    q = tmp_path / "query.mol"
    q.write_text(query)
    got = wasm(pkg, {"query": query, "targets": TARGETS, "stereo": stereo}, tmp_path)
    assert got.returncode == 0, got.stderr
    want = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-search", "--sub", str(q), *TARGETS, "--set", f"stereo={int(stereo)}").stdout
    assert got.stdout + "\n" == want
    assert json.loads(got.stdout)["hits"]


def test_bad_query_is_a_keyed_error(pkg, tmp_path):
    r = wasm(pkg, {"query": "C1C", "targets": ["CC"]}, tmp_path)
    assert r.returncode != 0 and json.loads(r.stderr)["error_key"] == "error.sub.query"
