"""The molcheck .wasm (crates/mzlab-mol-wasm) gives the `check/1` of a drawing byte for byte like `mzlab-cli mol-check` and the app fixture
(tests/dati/check_disegno.json, read by crates/mzlab-desktop/tests/labels.rs too), and a bad molfile as a keyed JSON error. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    out = tmp_path_factory.mktemp("molcheck")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-mol-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_mol_wasm.wasm")
    return out


def test_check_through_wasm_equals_the_cli_and_the_app_fixture(pkg, tmp_path):
    fx = json.loads((ROOT / "tests" / "dati" / "check_disegno.json").read_text(encoding="utf-8"))
    req = tmp_path / "req.json"
    req.write_text(json.dumps(fx["request"]), encoding="utf-8")
    site = run("node", "tests/molcheck_wasm_node.js", str(pkg), req.read_text(encoding="utf-8"))
    assert site == run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "mol-check", str(req))
    assert site.strip() == fx["check"]
    assert [w["code"] for w in json.loads(site)["warnings"]] == ["valence", "valenceCharge", "isotope", "wedgeNoCentre", "pieces"]


def test_a_bad_molfile_is_a_keyed_error(pkg):
    r = subprocess.run(["node", "tests/molcheck_wasm_node.js", str(pkg), json.dumps({"molfile": "nonsense"})], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 1 and json.loads(r.stderr)["error_key"] == "error.check.molfile"
