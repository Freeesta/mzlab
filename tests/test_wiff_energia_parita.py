"""The .mzidx of the product-ion experiments of a SCIEX .wiff from the engine .wasm (mzlab-wasm, what the site writes in OPFS; through Node) and from
`mzlab-cli flow run` (what the app writes) hold the same bytes: the collision energy and the isolation window of each scan included. The engine builds
mzlab-core without `sciex` or `precursori`, and the row of a scan with a maker TIC is read back from the header text the mzML writer gives it.
Synthetic file `energia` of tools/wiff_riferimento.py (full scan; product ions with CE 25, with CE 35, with two energies). Skipped without cargo,
wasm-bindgen or node."""
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run
from test_wiff_tic_parita import parts

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("wiff_energia")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def rows(p: Path):
    """The per-scan rows of a .mzidx (the scans JSON between its offset in the header and the metadata)."""
    b = p.read_bytes()
    meta_off = struct.unpack_from("<Q", b, 16)[0]
    scans_off = struct.unpack_from("<Q", b, 32)[0]
    return json.loads(b[scans_off:meta_off])


def test_the_engine_and_the_app_index_product_ion_scans_with_their_energy_and_window(work):
    ref = work / "ref"
    eng = work / "eng.mzidx"
    run("node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "pkg"), str(eng), "0", "energia.wiff", str(ref / "energia.wiff"), str(ref / "energia.wiff.scan"))
    plan = work / "plan.json"
    plan.write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": "energia.wiff", "type": "raw"}],
                                "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:energia.wiff"]}}]}), encoding="utf-8")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(plan), "--in", str(ref / "energia.wiff"), "--in", str(ref / "energia.wiff.scan"), "--out", str(work / "run"))
    app = work / "run" / "idx-energia.mzidx"
    assert rows(eng) == rows(app)
    assert parts(eng) == parts(app)
