"""The ion-scale method values in the site: the lazy .wasm `ionfacts` (mzlab-ionfacts-wasm, through Node: `ion_facts(method, scan_type)`, the call
rust-filemethod.js makes with the `method/1` text the page holds) gives the document the app answers (op `ionfacts`) and the value and place that
`mzlab-cli raw xic --ions` logs in `provenance.choices` for the same file. A synthetic mzML carries a `method/1` whose transitions have a 5 ms dwell in its `<run>`.
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

NODE = """const [dir, methodPath, scanType] = process.argv.slice(2);
const { ion_facts } = require(dir + '/mzlab_ionfacts_wasm.js'), fs = require('fs');
const text = p => p === '-' ? '' : fs.readFileSync(p, 'utf8');
let out; try { out = { s: ion_facts(text(methodPath), scanType) }; } catch (x) { out = { e: String(x) }; }
process.stdout.write(JSON.stringify(out));"""

METHOD = ('{"schema":"method/1","ms":{"periods":[{"experiments":[{"index":1,"type":"MRM","transitions":'
          '[{"q1":250.3,"q3":100,"dwell_ms":5},{"q1":260.3,"q3":90,"dwell_ms":5}]}]}]}}')


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
    w = tmp_path_factory.mktemp("ion_facts")
    d.ms2(w / "a.mzML", np.random.default_rng(5), 15)
    text = (w / "a.mzML").read_text(encoding="utf-8")
    user = f'<userParam name="mzLab method/1" value="{html.escape(METHOD)}"/>'
    # the document sits in the <run>, where the converter writes it
    (w / "a.mzML").write_text(re.sub(r"(<run [^>]*>)", lambda m: m.group(1) + user, text, count=1), encoding="utf-8")
    build("-p", "mzlab-cli")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(w / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-ionfacts-wasm")))
    (w / "run.js").write_text(NODE, encoding="utf-8")
    (w / "m.json").write_text(METHOD, encoding="utf-8")
    return w


def wasm(work, scan_type="", method="m.json"):
    p = subprocess.run(["node", str(work / "run.js"), str(work / "pkg"), "-" if method is None else str(work / method), scan_type], capture_output=True, encoding="utf-8", check=True)
    return json.loads(p.stdout)


def test_the_wasm_reads_the_dwell_the_cli_logs(work):
    got = wasm(work)
    assert "s" in got, got
    doc = json.loads(got["s"])
    assert doc["query"] == "dwell_ms=5"
    (read,) = doc["read"]
    assert read["field"] == "dwell_ms" and read["value"] == 5
    out = json.loads(cli("raw", "xic", str(work / "a.mzML"), "--mz", "253.1", "--rt", "0.5,3.5", "--ms-level", "2", "--ions").stdout)
    (c,) = [c for c in out["provenance"]["choices"] if c["what"].startswith("ion_method") and c["what"].endswith("dwell_ms")]
    assert c["value"] == read["value"] and read["from"] in c["why"]


def test_a_scan_type_that_does_not_use_the_dwell_reads_nothing_and_errors_are_the_engines(work):
    assert json.loads(wasm(work, "ems")["s"]) == {"query": "", "read": []}
    assert json.loads(wasm(work, method=None)["s"]) == {"query": "", "read": []}
    bad = wasm(work, "ms")
    assert "e" in bad and "error_key" in bad["e"], bad
