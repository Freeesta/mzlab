"""search_with_fdr of the libraries .wasm gives the same bytes as the native engine (crates/mzlab-lib, example fdr_parity), through Node.
Skipped without cargo, the wasm32 target, wasm-bindgen or node. The library is synthetic."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def test_wasm_and_native_give_the_same_fdr_bytes(tmp_path):
    def run(*a, **k):
        return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", **k)
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-lib", "--example", "fdr_parity", "--", str(tmp_path))
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm")
    pkg = tmp_path / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_lib_wasm.wasm")
    got = run("node", str(ROOT / "tests" / "fdr_wasm_node.js"), str(pkg), str(tmp_path)).stdout
    assert got == (tmp_path / "native.json").read_text()
    assert '"note":"lib.fdr.note"' in got and '"pos":null' not in got.split(',"q"')[0]
