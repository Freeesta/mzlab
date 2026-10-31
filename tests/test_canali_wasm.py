"""`api/canali` of the engine .wasm (mzlab-wasm, through Node) gives the same bytes as `mzlab-cli canali` (the same core functions) on public synthetic files:
the MSn infusion of the repository, an SRM acquisition and an SRM + product-ion one. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402

MSN = ROOT / "tests" / "golden" / "ref" / "msntree_rotta" / "msn.mzML"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("canali")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli", "-p", "mzlab-core", "--example", "mzidx")
    ds.mrm(d / "srm.mzML", 2.0e5, np.random.default_rng(7), 1)
    ds.mixed_mrm_epi(d / "srm_epi.mzML", np.random.default_rng(7))
    return d


def files(work):
    return [MSN, work / "srm.mzML", work / "srm_epi.mzML"]


def wasm_canali(work, mzml):
    idx = work / (mzml.stem + ".mzidx")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-core", "--example", "mzidx", "--", str(mzml), str(idx))
    plan = work / "plan.json"
    plan.write_text(json.dumps([{"op": "idxopen", "slot": 0, "name": mzml.name}, {"op": "canali", "slot": 0, "name": mzml.name}]), encoding="utf-8")
    out = json.loads(run("node", str(ROOT / "tests" / "rotte_wasm_node.js"), str(work / "pkg"), str(idx), str(mzml), str(plan)).stdout)
    assert "s" in out[1], out[1]
    return out[1]["s"]


@pytest.mark.parametrize("which", [0, 1, 2])
def test_wasm_and_cli_give_the_same_channels(work, which):
    mzml = files(work)[which]
    got = wasm_canali(work, mzml)
    want = run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "canali", str(mzml)).stdout
    assert got.strip() == want.strip()
    doc = json.loads(got)
    assert doc["v"] == "canali/1" and doc["canali"]
