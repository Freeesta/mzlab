"""The fingerprints of the members of a vendor dataset given to the engine (mzlab-wasm, `build_mzidx_native`) in each way the site has: taken by the
page during the copy, given late by a function the build calls after its first pass (the copy's hash still running), a late function giving nothing
(the engine reads the members again), or none: the .mzidx is the same, byte for byte (synthetic `mistopi` of tools/wiff_mixed_pi.py). Skipped without
cargo, wasm-bindgen or node."""
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
    d = tmp_path_factory.mktemp("tardive")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_mixed_pi.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def build(work, mode):
    out = work / f"{mode}.mzidx"
    p = subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(out), "0", "mistopi.wiff",
                        str(work / "ref" / "mistopi.wiff"), str(work / "ref" / "mistopi.wiff.scan")],
                       cwd=ROOT, capture_output=True, encoding="utf-8", env={**os.environ, "MZIDX_PRINTS": mode})
    assert p.returncode == 0, p.stderr[-1500:]
    return out.read_bytes(), p.stderr.count("late prints asked")


def test_the_index_is_the_same_however_the_fingerprints_arrive(work):
    ref, _ = build(work, "none")
    for mode in ("given", "late", "bad"):
        got, asked = build(work, mode)
        assert got == ref, mode
        assert asked == (1 if mode in ("late", "bad") else 0), (mode, asked)
