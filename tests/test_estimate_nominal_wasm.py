"""The nominal-resolution hint of `features.estimate` is the same in the site and in the CLI: the `estimates/1` that the mzFinder `.wasm` writes through Node (`flow_estimate`, the export the browser
runner calls) has the same `nominal` member as the one `mzlab-cli estimate` writes for the same unit-resolution file, and the member says what the signature is (candidate rows, the shares between the two
heights and of ions split across the grids, the cuts). The app's leg is the same Rust (`estimate_of`) and `crates/mzlab-desktop/tests/parity_estimate.rs` for the answer put together. Skipped without cargo,
wasm-bindgen or node only outside CI: in CI a missing tool or a missing numpy FAILS the test (this is the wasm leg of the parity rule for the hint)."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import cli, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")) and not os.environ.get("CI"), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    try:
        import numpy  # noqa: F401  (tools/lr_synthetic.py needs it)
    except ImportError:
        if os.environ.get("CI"):
            pytest.fail("numpy is missing: the wasm leg of the nominal hint cannot be skipped in CI")
        pytest.skip("numpy not available")
    root = tmp_path_factory.mktemp("nominal_hint")
    run("python3", "tools/lr_synthetic.py", str(root / "ser"), "--jitter", "0.3")
    file = sorted((root / "ser").glob("LR_bench-s*.mzML"))[0]
    key = json.loads(cli("file", "index", "--cache", str(root / "cache"), str(file)).stdout)["files"][0]["key"]
    module = wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", runner=run)
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), str(module))
    return file, root / "cache" / key, pkg


def test_the_wasm_and_the_cli_write_the_same_nominal_member(built):
    file, idx, pkg = built
    site = json.loads(run("node", str(ROOT / "tests" / "estimate_wasm_node.js"), str(pkg), str(idx), file.name))
    native = json.loads(cli("estimate", str(file), "--json").stdout)["files"][0]
    assert site["axis"] == "Da"
    assert site["nominal"] == native["nominal"]
    n = site["nominal"]
    assert n["rows"] > 0 and (n["height_nominal"], n["reach_nominal"]) == (2500.0, 0.25)
    assert n["hint"] == (n["between_share"] >= n["cut_between"] or n["split_share"] >= n["cut_split"])
    # the cuts are parameters of the step: the browser runner passes them and the member prints them
    cut = json.loads(run("node", str(ROOT / "tests" / "estimate_wasm_node.js"), str(pkg), str(idx), file.name, json.dumps({"hint_between": 0.9, "hint_split": 0.9})))
    assert (cut["nominal"]["cut_between"], cut["nominal"]["cut_split"]) == (0.9, 0.9)
