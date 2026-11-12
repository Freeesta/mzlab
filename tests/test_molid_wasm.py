"""The molid .wasm (crates/mzlab-molid-wasm) answers «is this drawing in the catalogue?» with the same `molid/1` bytes as `mzlab-cli mol-id` (the page = the
command), for SMILES and molfiles, exact and near matches; a structure that cannot be read is a keyed JSON error. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
CATALOGUE = ROOT / "mzlab" / "web" / "molecole" / "catalogo.csv"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
# synthetic writings of catalogue structures (other atom order, aromatic form, a labelled methyl, the enantiomer, a nitro group, an ion that is not there)
SMILES = ["Oc1ccc(NC(C)=O)cc1", "COc1ccc2cc([C@@H](C)C(O)=O)ccc2c1", "[2H]C([2H])([2H])C(=O)Nc1ccc(O)cc1", "OCCn1c(C)ncc1[N+](=O)[O-]", "C[NH+]1CCC[C@H]1c1cccnc1"]
ETHANOL = "\n  synthetic\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n" + "".join(f"{i * 1.5:10.4f}    0.0000    0.0000 {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for i, e in enumerate("CCO")) + "  1  2  1  0\n  2  3  1  0\nM  END\n"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("molid")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-molid-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d), "target/wasm32-unknown-unknown/release/mzlab_molid_wasm.wasm")
    run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-cli")
    return d


def site(pkg, tmp_path, request):
    f = tmp_path / "req.json"
    f.write_text(json.dumps(request), encoding="utf-8")
    return subprocess.run(["node", str(ROOT / "tests" / "molid_wasm_node.js"), str(pkg), str(f)], capture_output=True, encoding="utf-8")


def cli(*args):
    return run(str(ROOT / "target" / "release" / "mzlab-cli"), "mol-id", *args, "--set", f"catalogue={CATALOGUE}").stdout


@pytest.mark.parametrize("smiles", SMILES)
def test_smiles_through_wasm_equals_the_cli_byte_for_byte(pkg, tmp_path, smiles):
    r = site(pkg, tmp_path, {"smiles": smiles, "catalogo": CATALOGUE.read_text(encoding="utf-8")})
    assert r.returncode == 0, r.stderr
    assert r.stdout == cli("--set", f"smiles={smiles}")
    assert json.loads(r.stdout)["format"] == "molid/1"


def test_molfile_through_wasm_equals_the_cli_byte_for_byte(pkg, tmp_path):
    mf = tmp_path / "ethanol.mol"
    mf.write_text(ETHANOL, encoding="utf-8")
    r = site(pkg, tmp_path, {"molfile": ETHANOL, "catalogo": CATALOGUE.read_text(encoding="utf-8")})
    assert r.returncode == 0, r.stderr
    assert r.stdout == cli(str(mf))


def test_a_bad_structure_is_a_keyed_error(pkg, tmp_path):
    r = site(pkg, tmp_path, {"smiles": "C1CC", "catalogo": ""})
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.molid.read"
