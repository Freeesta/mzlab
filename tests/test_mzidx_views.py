"""The bytes the engine (mzlab-wasm, `build_mzidx_native`) hands to `write(offset, bytes)` are views of its own memory, not copies, valid during the call
only: a callback that writes them at once (the worker's `writeFull`, `fs.writeSync` here) gets the same .mzidx as ever, one that keeps them without
copying gets a wrong one (synthetic `mistopi` of tools/wiff_mixed_pi.py). Skipped without cargo, wasm-bindgen or node."""
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
WASM = ROOT / "target/wasm32-unknown-unknown/release"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("views")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_mixed_pi.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def build(work, mode):
    out = work / f"{mode or 'direct'}.mzidx"
    p = subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(out), "0", "mistopi.wiff",
                        str(work / "ref" / "mistopi.wiff"), str(work / "ref" / "mistopi.wiff.scan")],
                       cwd=ROOT, capture_output=True, encoding="utf-8", env={**os.environ, "MZIDX_VIEWS": mode})
    assert p.returncode == 0, p.stderr[-1500:]
    return out.read_bytes(), p.stderr


def test_writes_are_views_valid_during_the_call_only(work):
    ref, _ = build(work, "")
    got, log = build(work, "count")
    n = int(log.split("views ")[1].split(" of ")[1])
    assert n > 0 and f"views {n} of {n}" in log, log
    assert got == ref
    kept, _ = build(work, "keep")
    assert kept != ref
