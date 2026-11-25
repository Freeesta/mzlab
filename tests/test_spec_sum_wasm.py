"""`spectrum get --sum` in the site: the lazy .wasm `rawxic` (mzlab-rawxic-wasm, through Node: `spec_sum(size, read, pol, query)`, the call rust-raw.js makes for the
route `/api/specsum` over the .mzidx of OPFS) gives the evidence of a sum, byte for byte, as `mzlab-cli spectrum get --sum` prints it inside `evidence.sum`, on the
synthetic files of `tools/dati_sintetici.py` (full scan, profile, MS2); a fill time the route refuses has the same `error_key` in both. The desktop answers the same
route (`crates/mzlab-desktop/tests/parity_routes/route_steps.rs`, app against `.wasm`). Skipped on a machine without cargo, wasm-bindgen, node or numpy; in CI
(GITHUB_ACTIONS) those are required and a missing one FAILS (`test_the_tools_are_there_in_ci`). Run by the `rest` shards of test.yml."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NODE = """const [dir, idxPath, pol, ...queries] = process.argv.slice(2);
const { spec_sum } = require(dir + '/mzlab_rawxic_wasm.js'), fs = require('fs');
const b = fs.readFileSync(idxPath), read = (off, len) => new Uint8Array(b.subarray(off, off + len));
const out = queries.map(q => { try { return { s: spec_sum(b.length, read, Number(pol), q) }; } catch (x) { return { e: String(x) }; } });
process.stdout.write(JSON.stringify(out));"""
# (file, CLI flags, route query) of the same question
ASKS = [
    ("FS.mzML", ["--rt", "2.5,3.5"], "rt0=2.5&rt1=3.5&level=1"),
    ("FS.mzML", ["--rt", "2.5,3.5", "--fill-ms", "20"], "rt0=2.5&rt1=3.5&level=1&fill_ms=20"),
    ("FS_profile.mzML", ["--rt", "2.5,3.5"], "rt0=2.5&rt1=3.5&level=1"),
    ("MS2.mzML", ["--rt", "5,9", "--ms-level", "2"], "rt0=5&rt1=9&level=2"),
    ("MS2.mzML", ["--rt", "5,9", "--ms-level", "2", "--fill-ms", "50"], "rt0=5&rt1=9&level=2&fill_ms=50"),
    ("MS2.mzML", ["--rt", "100,101", "--ms-level", "2"], "rt0=100&rt1=101&level=2"),
]


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    try:
        import numpy as np
    except ImportError:
        if CI:
            pytest.fail("numpy must be installed in CI: tools/dati_sintetici.py needs it")
        pytest.skip("numpy not available")
    import dati_sintetici as d
    w = tmp_path_factory.mktemp("spec_sum")
    r = np.random.default_rng(5)
    d.full_scan(w / "FS.mzML", 3.0, r)
    d.full_scan_profile(w / "FS_profile.mzML", 3.0, r)
    d.ms2(w / "MS2.mzML", r, 15)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-rawxic-wasm")))
    build("-p", "mzlab-cli")
    (w / "run.js").write_text(NODE, encoding="utf-8")
    return w


def answers(work, name, queries):
    """The wasm's answers to `queries` over the index of `name`."""
    f, cache = work / name, work / "cache"
    key = json.loads(cli("raw", "cache", str(f), "--cache", str(cache)).stdout)["files"][0]["key"]
    return json.loads(subprocess.run(["node", str(work / "run.js"), str(work / "pkg"), str(cache / key), "0", *queries], capture_output=True, encoding="utf-8", check=True).stdout)


@pytest.mark.parametrize("name", ["FS.mzML", "FS_profile.mzML", "MS2.mzML"])
def test_the_wasm_evidence_is_inside_the_cli_document_byte_for_byte(work, name):
    asks = [a for a in ASKS if a[0] == name]
    for (_, flags, query), got in zip(asks, answers(work, name, [q for _, _, q in asks])):
        assert "s" in got, f"{name} {query}: {got}"
        doc = cli("spectrum", "get", str(work / name), *flags, "--sum").stdout
        assert f'"sum":{got["s"]}}}' in doc, f"{name} {flags}:\nwasm {got['s'][:500]}\ncli  {doc[-900:]}"
        assert json.loads(got["s"])["background"]["applied"] == "none"


def test_a_refused_fill_time_has_the_same_error_key(work):
    (got,) = answers(work, "MS2.mzML", ["rt0=5&rt1=9&level=2&fill_ms=0"])
    assert json.loads(got["e"])["error_key"] == "err.spec.fill", got
    r = cli("spectrum", "get", str(work / "MS2.mzML"), "--rt", "5,9", "--ms-level", "2", "--sum", "--fill-ms", "0", runner=lambda *a: subprocess.run(a, capture_output=True, encoding="utf-8"))
    doc = json.loads(r.stdout)
    assert r.returncode == 2 and doc["schema"] == "error/1" and doc["error_key"] == "err.spec.fill", r.stdout[:300]
