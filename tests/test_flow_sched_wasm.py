"""The live scheduler in WebAssembly (crates/mzlab-feat-wasm/src/sched.rs) gives the same states, commands, cards and events as the native one on the shared
cases of crates/mzlab-core/tests/data/sched_cases.json, played through Node (tests/flow_sched_nodo.js). Skipped without cargo, the wasm32 target,
wasm-bindgen or node."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "crates/mzlab-core/tests/data/sched_cases.json"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def test_wasm_scheduler_replays_the_shared_cases(tmp_path):
    run("cargo", "build", "--release", "--locked", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "detect")
    pkg = tmp_path / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    out = run("node", str(ROOT / "tests/flow_sched_nodo.js"), str(pkg), str(CASES))
    assert out.startswith("ok "), out
