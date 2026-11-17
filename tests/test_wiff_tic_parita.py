"""The .mzidx of a SCIEX .wiff sample from the engine .wasm (mzlab-wasm `build_mzidx_native`, what the site writes in OPFS; through Node) and from
`mzlab-cli flow run` (index.mzidx, what the app writes) hold the same bytes, the TIC of each scan included: the TIC the file header gives (not the sum of the
points, not a float32 value), the sum only where the header has none. Synthetic pair `totale` of tools/wiff_riferimento.py (no laboratory data).
The stored chromatograms are left out of the comparison: the app keeps the vendor TIC chromatogram of a full scan experiment (wiff/run_extended.rs), the engine
only the MRM ones. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("wiff_tic")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    return d


def parts(p: Path):
    """(data bytes before the metadata, metadata without the number of stored chromatograms) of a .mzidx (header of mzidx.rs: magic, version, flags, offsets)."""
    b = p.read_bytes()
    assert b[:8] == b"MZIDX001"
    meta_off, meta_len = struct.unpack_from("<QQ", b, 16)
    meta = json.loads(b[meta_off:meta_off + meta_len])
    meta["run"].pop("n_chromatograms")
    return b[:meta_off], meta


def tic(p: Path):
    data, meta = parts(p)
    s = meta["sections"][0]
    return list(struct.unpack_from(f"<{s['n_scans']}d", data, s["off"]["tic"]))


def indexes(work, name):
    """The .mzidx of the first sample of `name`.wiff from the engine (.wasm through Node) and from `mzlab-cli flow run`."""
    ref = work / "ref"
    eng = work / f"eng-{name}.mzidx"
    run("node", str(ROOT / "tests" / "mzidx_nativo_node.js"), str(work / "pkg"), str(eng), "0", f"{name}.wiff", str(ref / f"{name}.wiff"), str(ref / f"{name}.wiff.scan"))
    plan = work / f"plan-{name}.json"
    plan.write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": f"{name}.wiff", "type": "raw"}],
                                "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": [f"file:{name}.wiff"]}}]}), encoding="utf-8")
    cli("flow", "run", str(plan), "--in", str(ref / f"{name}.wiff"), "--in", str(ref / f"{name}.wiff.scan"), "--out", str(work / f"run-{name}"))
    return eng, work / f"run-{name}" / f"idx-{name}.mzidx"


def test_the_engine_and_the_app_index_a_wiff_sample_with_the_same_bytes_and_the_header_tic(work):
    eng, app = indexes(work, "totale")
    want = json.loads((work / "ref" / "riferimento.json").read_text())["totale"]["tic"]
    assert tic(eng) == pytest.approx(want, rel=1e-12) and tic(eng)[0] == 1234567.891, tic(eng)
    assert parts(eng) == parts(app)


def test_an_ida_sample_keeps_every_scan_in_the_engine_and_the_app(work):
    """The IDA sample of `ida` (its index rows hold no point count at +26): all 9 scans, none counted as damaged, the same bytes on site and app."""
    eng, app = indexes(work, "ida")
    assert sum(s["n_scans"] for s in parts(eng)[1]["sections"]) == 9
    assert parts(eng) == parts(app)
