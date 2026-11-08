"""A .wiff sample of a scan type the engine does not read (`mistopi` of tools/wiff_misto_pi.py, synthetic: sample 1 Full Scan, sample 2 code 6) on the site:
the engine (mzlab-wasm, `build_mzidx_native`) refuses sample 2 (`err.file.wiff`, `scan type code 6`) instead of indexing it as level-1 scans, and still reads
sample 1. The `sciex` module serves that pair (tests/test_wiff_programmati_sito.py, crates/mzlab-lettori/tests/sciex_sonda.rs). Skipped without cargo, wasm-bindgen or node."""
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
    d = tmp_path_factory.mktemp("codice")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "eng"), str(WASM / "mzlab_wasm.wasm"))
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto_pi.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def engine(work, c):
    pair = [work / "ref" / "mistopi.wiff", work / "ref" / "mistopi.wiff.scan"]
    return subprocess.run(["node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "eng"), str(work / f"s{c}.mzidx"), str(c), "mistopi.wiff", *map(str, pair)],
                          cwd=ROOT, capture_output=True, encoding="utf-8")


def test_the_engine_refuses_a_scan_type_it_does_not_read(work):
    p = engine(work, 1)
    assert p.returncode != 0 and "scan type code 6 (experiment 1)" in p.stderr, (p.returncode, p.stdout[-500:], p.stderr[-1500:])


def test_the_engine_still_reads_the_native_sample(work):
    p = engine(work, 0)
    assert p.returncode == 0, p.stderr[-1500:]
