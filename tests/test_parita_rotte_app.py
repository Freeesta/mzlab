"""The common routes answer the same in the desktop engine and in the engine .wasm (mzlab-wasm, through Node): crates/mzlab-desktop/tests/parity_routes.rs
sends one plan of routes to both on the same example and index; profile_hr_spectrum.rs does the same on a high-resolution profile file, profile_sparse_wiff.rs on sparse product-ion scans of a .wiff, parity_ticbg.rs api/tic_background on a synthetic run with chemical background. Skipped without cargo, wasm-bindgen or node."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a, env=None):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", env=env)


def test_the_app_and_the_wasm_give_the_same_answers(tmp_path):
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "-p", "mzlab-rotte-wasm", "-p", "mzlab-ticfondo-wasm", runner=run)
    for crate in ("mzlab_wasm", "mzlab_rotte_wasm", "mzlab_ticfondo_wasm"):   # the engine and its lazy modules (statistics, Data and Flow routes; api/tic_background), side by side in one folder
        run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(tmp_path / "pkg"), f"target/wasm32-unknown-unknown/release/{crate}.wasm")
    run("cargo", "test", "--locked", "-p", "mzlab-desktop", "--test", "parity_routes", "--test", "profile_hr_spectrum", "--test", "profile_sparse_wiff", "--test", "parity_ticbg", env={**os.environ, "MZLAB_WASM_PKG": str(tmp_path / "pkg")})
