"""The .mzidx of the dependent (IDA) scans of a SCIEX .wiff from the engine .wasm (mzlab-wasm, what the site writes in OPFS; through Node) and from
`mzlab-cli flow run` (what the app writes) hold the same bytes: each EPI scan with the precursor and charge of its `DDERealTimeData` row, never the mass
of the method header. The engine builds mzlab-core without `sciex` or `precursori`; the reading of the stream is in every build (`wiff/dde.rs`).
Synthetic file `ida_epi` of tools/wiff_riferimento.py (sample 1: survey EMS + EPI with header 188.40, rows 493.27, 547.30, 687.85). Skipped without
cargo, wasm-bindgen or node."""
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run
from test_wiff_energia_parita import rows
from test_wiff_tic_parita import parts

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("wiff_ida")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def test_the_engine_and_the_app_index_ida_scans_with_the_precursor_of_their_row(work):
    ref = work / "ref"
    eng = work / "eng.mzidx"
    run("node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "pkg"), str(eng), "0", "epi.wiff", str(ref / "ida_epi.wiff"), str(ref / "ida_epi.wiff.scan"))
    # the app indexes the same sample: a copy of the pair with only sample 1 would change the method, so the plan names sample 1 of the dataset
    plan = work / "plan.json"
    plan.write_text('{"format": "mzflow/2", "inputs": [{"name": "ida_epi.wiff", "type": "raw"}], "steps": [{"id": "idx", "step": "index.mzidx@1", '
                    '"in": {"source": ["file:ida_epi.wiff"]}}]}', encoding="utf-8")
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(plan), "--in", str(ref / "ida_epi.wiff"), "--in",
        str(ref / "ida_epi.wiff.scan"), "--out", str(work / "run"))
    apps = sorted((work / "run").glob("idx-*.mzidx"))
    assert apps, list((work / "run").iterdir())
    app = apps[0]
    got = rows(eng)
    assert got == rows(app)
    assert parts(eng) == parts(app)
    text = repr(got)
    for mz in (493.27, 547.3, 687.85):
        assert str(mz) in text, text
    assert "188.4" not in text, text
