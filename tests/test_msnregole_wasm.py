"""The msnregole .wasm gives the expectations of the structure rules that the command line computes, and `msn_grafo` fed with them (query key `attese`)
gives the same bytes as `mzlab-cli msn-graph --regole --smiles` (the page = the command). Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path
from urllib.parse import quote

import pytest

ROOT = Path(__file__).resolve().parents[1]
MZML = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
SMILES = "CCOC(=O)c1c[nH]c2ccccc12"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8")


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("msnregole")
    for crate in ("mzlab-msnregole-wasm", "mzlab-msngrafo-wasm"):
        run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", crate)
        run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / crate), f"target/wasm32-unknown-unknown/release/{crate.replace('-', '_')}.wasm")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-core", "--example", "mzidx", "--", str(MZML), str(d / "msn.mzidx"))
    return d


def attese(pkg, smiles):
    return run("node", str(ROOT / "tests" / "msnregole_wasm_node.js"), str(pkg / "mzlab-msnregole-wasm"), smiles).stdout


def test_attese_are_a_list_of_rule_formula_ion(pkg):
    a = json.loads(attese(pkg, SMILES))
    assert a and all(len(x) == 3 and x[2] in (0, 1) for x in a)


def test_a_bad_smiles_is_a_keyed_error(pkg):
    r = subprocess.run(["node", str(ROOT / "tests" / "msnregole_wasm_node.js"), str(pkg / "mzlab-msnregole-wasm"), "C(("], capture_output=True, encoding="utf-8")
    assert r.returncode != 0 and "error_key" in r.stderr


def test_graph_with_the_attese_of_the_wasm_is_the_graph_of_the_command(pkg):
    base = f"formula=C14H15N3O5%2B&regole=1&smiles={quote(SMILES, safe='')}"
    query = f"{base}&attese={quote(attese(pkg, SMILES), safe='')}"
    got = run("node", str(ROOT / "tests" / "msngrafo_wasm_node.js"), str(pkg / "mzlab-msngrafo-wasm"), str(pkg / "msn.mzidx"), str(MZML), query).stdout
    want = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(MZML), "--formula", "C14H15N3O5+", "--smiles", SMILES, "--regole").stdout
    assert got + "\n" == want
    assert '"regole":' in got
