"""«+D», «-D», «+2D» in the list of transformations: the mzFinder .wasm (through Node) answers `finder_transf_rows` with the same bytes as the native engine, which
crates/mzlab-feat/tests/delta_deuterio.rs compares with the same golden. Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def test_wasm_rows_equal_the_golden(tmp_path):
    run("cargo", "build", "--release", "--locked", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(tmp_path), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    g = json.loads((ROOT / "tests/golden/finder_transf_deuterio.json").read_text())
    js = "const w=require(process.argv[1]+'/mzlab_feat_wasm.js');process.stdout.write(w.finder_transf_rows(process.argv[2]));"
    assert run("node", "-e", js, str(tmp_path), g["text"]) == g["describe"]
