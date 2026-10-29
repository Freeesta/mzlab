"""The common routes answer the same in the desktop engine and in the engine .wasm (mzlab-wasm, through Node): crates/mzlab-desktop/tests/parita_rotte.rs
sends one plan of routes to both on the same example and index. Skipped without cargo, wasm-bindgen or node."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a, env=None):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", env=env)


def test_the_app_and_the_wasm_give_the_same_answers(tmp_path):
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(tmp_path / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "test", "--locked", "-p", "mzlab-desktop", "--test", "parita_rotte", env={**os.environ, "MZLAB_WASM_PKG": str(tmp_path / "pkg")})
