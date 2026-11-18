"""`--sample LIST` is the same selection from the engine .wasm (`sample_selection`, through Node) and from `mzlab-cli samples select --json`: the same bytes for a list that
fits the file, the same error key and parameters for an id the file does not have or a text that is not a list (the CLI adds its exit code: 5 and 2).
Synthetic data only (tools/wiff_grande.py). python3 is part of the test (it fails without it); skipped only without cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli_argv, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
SELECT = """const w = require(process.argv[2]);
try { console.log(w.sample_selection(Uint32Array.from(JSON.parse(process.argv[3])), process.argv[4])); }
catch (e) { console.log(e && e.message !== undefined ? e.message : (typeof e === "string" ? e : JSON.stringify(e))); process.exitCode = 3; }"""
LISTS = ["1,3", "3,1", "2-3", "1,1,2", "1-3", " 2 , 3", "1,9", "4", "3-1", "1,,2", "x", ""]


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("sample_selection")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d / "big"), "--samples", "3", "--scans", "40", "--points", "50"], check=True, capture_output=True)
    (d / "s.js").write_text(SELECT, encoding="utf-8")
    return d


def site(work, ids, text):
    p = subprocess.run(["node", str(work / "s.js"), str(work / "pkg" / "mzlab_wasm.js"), json.dumps(ids), text], cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout.strip()


def app(work, text):
    p = subprocess.run(cli_argv("samples", "select", str(work / "big" / "grande.wiff"), "--sample", text, "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout.strip()


def test_the_site_and_the_cli_select_the_same_samples(work):
    for text in LISTS:
        code, out = site(work, [1, 2, 3], text)
        rc, line = app(work, text)
        if code == 0:
            assert rc == 0 and line == out, (text, rc, line, out)
        else:
            a, b = json.loads(out), json.loads(line)
            assert rc in (2, 5) and (b["error_key"], b["params"]) == (a["error_key"], a["params"]), (text, out, line)
            assert rc == (5 if a["error_key"] == "err.sample.unknown" else 2), (text, rc)
    assert site(work, [1, 2, 3], "3,1") == (0, '{"samples":[1,3]}')
    assert app(work, "1,9")[0] == 5
