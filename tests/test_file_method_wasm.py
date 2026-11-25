"""`file method` in the site: the lazy .wasm `filemethod` (mzlab-filemethod-wasm, through Node: `file_method(sample, method, items)`, the call rust-filemethod.js
makes with the `method/1` and `items/1` texts the page holds) gives byte for byte what `mzlab-cli file method` prints, on a synthetic .wiff converted to mzML
(`tools/wiff_riferimento.py` `dispersione`: product ion 253.1, CE 35, CES 15, DP 60, a sample named for 237.1); a synthetic mzML (precursors 229.1 and 305.0) named
for 237.1 gives the name warning with no method. A file with no method and a broken items text give the engine's error. The desktop answers the same op (`crates/mzlab-desktop/tests/parity_filemethod.rs`).
Skipped on a machine without cargo, wasm-bindgen or node; in CI (GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import html
import json
import os
import re
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
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NODE = """const [dir, sample, methodPath, itemsPath] = process.argv.slice(2);
const { file_method } = require(dir + '/mzlab_filemethod_wasm.js'), fs = require('fs');
const text = p => p === '-' ? '' : fs.readFileSync(p, 'utf8');
let out; try { out = { s: file_method(Number(sample), text(methodPath), text(itemsPath)) }; } catch (x) { out = { e: String(x) }; }
process.stdout.write(JSON.stringify(out));"""


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
    w = tmp_path_factory.mktemp("file_method")
    d.ms2(w / "Std_237.1.mzML", np.random.default_rng(5), 15)
    run(sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(w / "wiff"))
    build("-p", "mzlab-cli")
    cli("file", "convert", str(w / "wiff" / "dispersione.wiff"), "--out", str(w / "conv"), "--json")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-filemethod-wasm")))
    (w / "run.js").write_text(NODE, encoding="utf-8")
    return w


def method_of(mzml: Path) -> str:
    """The `method/1` document an mzML carries (`userParam mzLab method/1`), as the page reads it."""
    m = re.search(r'userParam name="mzLab method/1" value="([^"]*)"', mzml.read_text(encoding="utf-8"))
    return html.unescape(m.group(1)) if m else ""


def wasm(work, mzml: Path, sample="-1", method=None, items=None):
    """The wasm's answer for the file: its method text and its `file items` text (what the page holds), or the given ones."""
    method = method_of(mzml) if method is None else method
    items = cli("file", "items", str(mzml), "--no-points").stdout.strip() if items is None else items
    (work / "m.json").write_text(method, encoding="utf-8")
    (work / "i.json").write_text(items, encoding="utf-8")
    p = subprocess.run(["node", str(work / "run.js"), str(work / "pkg"), sample, str(work / "m.json"), str(work / "i.json")], capture_output=True, encoding="utf-8", check=True)
    return json.loads(p.stdout)


def converted(work) -> Path:
    return next((work / "conv").glob("*.mzML"))


def test_the_wasm_gives_the_bytes_of_the_cli(work):
    """The converted mzML carries its `method/1`: the page and the CLI read the same text, so the documents are the same bytes."""
    f = converted(work)
    got = wasm(work, f)
    assert "s" in got, got
    want = cli("file", "method", str(f)).stdout
    assert got["s"] + "\n" == want
    doc = json.loads(want)
    assert doc["schema"] == "file_method/1" and doc["segments"]


def test_the_wiff_converted_keeps_ce_ces_and_dp_and_says_what_it_does_not_decode(work):
    doc = json.loads(wasm(work, converted(work))["s"])
    seg = next(s for s in doc["segments"] if s["ms_level"] == 2)
    assert (seg["precursor_mz"], seg["ce_eV"], seg["ces_eV"], seg["dp_V"]) == (253.1, 35, 15, 60)
    assert doc["unavailable"] == {"fill_time_ms": "unknown", "q1_resolution": "unknown"}
    assert seg["fill_time_ms"] is None and seg["q1_resolution"] is None


def test_a_name_mass_that_is_not_a_precursor_is_a_warning_in_the_wasm(work):
    """The sample of the .wiff is named for 237.1, its method and scans have the precursor 253.1."""
    w = json.loads(wasm(work, converted(work))["s"])["warnings"][0]
    assert w["key"] == "warn.file.nameMz" and w["nearest_name_mz"] == 237.1 and w["nearest_precursor"] == 253.1 and w["delta_mz"] == 16
    # a file with no method says it with the scans' precursors alone: the file name carries 237.1, the scans 229.1 and 305.0
    f = work / "Std_237.1.mzML"
    w = json.loads(wasm(work, f, method="")["s"])["warnings"][0]
    assert w["nearest_precursor"] == 229.1 and w["delta_mz"] == -8


def test_a_file_with_no_method_has_null_and_a_reason_and_a_broken_text_has_the_engine_error(work):
    f = work / "Std_237.1.mzML"
    items = cli("file", "items", str(f), "--no-points").stdout.strip()
    doc = json.loads(wasm(work, f, method="", items=items)["s"])
    assert doc["format"] is None and doc["segments"] == [] and doc["unavailable"]["ce_eV"] == "not_in_file"
    bad = wasm(work, f, method="", items="{")
    assert "e" in bad and "error_key" in bad["e"], bad
