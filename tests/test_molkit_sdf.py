"""The molkit .wasm (crates/mzlab-mol-wasm) gives the `sdf/1` text of a drawing byte for byte like `mzlab-cli mol-export` and the app fixture
(tests/dati/sdf_disegno.json, read by crates/mzlab-desktop/tests/labels.rs too), the .mol the same way, and a bad ion as a keyed JSON error.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build, cli_argv

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
FX = json.loads((ROOT / "tests" / "dati" / "sdf_disegno.json").read_text(encoding="utf-8"))


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("molkit")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-mol-wasm", runner=run)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_mol_wasm.wasm")
    return out


def cli(mol, *sets):
    a = cli_argv("mol-export", str(mol))
    for s in sets:
        a += ["--set", s]
    return run(*a)


def test_sdf_and_mol_through_wasm_equal_the_cli_and_the_app_fixture(pkg, tmp_path):
    req = FX["request"]
    mol = tmp_path / "draw.mol"
    mol.write_text(req["molfile"], encoding="utf-8")
    site = run("node", "tests/molkit_sdf_node.js", str(pkg), json.dumps(req))
    assert site == FX["sdf"]
    assert site == cli(mol, f"ion={req['ion']}", f"name={req['name']}")
    assert site.count("$$$$\n") == 2 and "> <MZLAB_MZ>\n90.05495\n" in site and "> <MZLAB_ION>\ndrawn\n" in site
    one = run("node", "tests/molkit_sdf_node.js", str(pkg), json.dumps({**req, "format": "mol"}))
    assert one == cli(mol, "format=mol", f"name={req['name']}")
    assert "$$$$" not in one and one.endswith("M  END\n")


def test_a_bad_ion_is_a_keyed_error(pkg):
    bad = json.dumps({**FX["request"], "ion": "[M+Xe]+"})
    r = subprocess.run(["node", "tests/molkit_sdf_node.js", str(pkg), bad], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.sdf.ion"
