"""A maker's dataset whose index would not fit the budget of the instance ends in the calm `error.memoryBudget` (phase `index`, with the numbers)
before a scan is decoded, not in a tab that dies (`estimate::check_reader` in `build_mzidx_native`): proved with a small fake budget on the synthetic
`mistopi` of tools/wiff_mixed_pi.py; a budget that fits gives the same bytes as no budget. Skipped without cargo, wasm-bindgen or node."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
WASM = ROOT / "target/wasm32-unknown-unknown/release"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("budget")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_mixed_pi.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def build(work, cap):
    out = work / f"cap{cap}.mzidx"
    env = {k: v for k, v in os.environ.items() if k != "MZIDX_CAP"}
    if cap is not None:
        env["MZIDX_CAP"] = str(cap)
    p = subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(out), "0", "mistopi.wiff",
                        str(work / "ref" / "mistopi.wiff"), str(work / "ref" / "mistopi.wiff.scan")],
                       cwd=ROOT, capture_output=True, encoding="utf-8", env=env)
    return p, out


def test_a_tiny_budget_is_a_calm_refusal_before_the_scans(work):
    p, out = build(work, 8)
    assert p.returncode == 3, p.stderr[-1500:]
    err = json.loads(p.stderr)
    assert err["error_key"] == "error.memoryBudget"
    assert err["params"]["fase"] == "index" and err["params"]["budget_mb"] == "8"
    assert out.stat().st_size == 0
    ok, out_ok = build(work, 512)
    assert ok.returncode == 0, ok.stderr[-1500:]
    ref, out_ref = build(work, None)
    assert ref.returncode == 0, ref.stderr[-1500:]
    assert out_ok.read_bytes() == out_ref.read_bytes()
