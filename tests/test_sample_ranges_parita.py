"""The copy plan `sample-select/1` with its plan (the byte ranges of a `.wiff.scan` that the chosen samples need) is the same bytes from the engine .wasm (`sample_ranges`, through Node,
what the site copies into OPFS) and from `mzlab-cli sample select --ranges --json`; an id the file does not have or a text that is not a list gives the same error key and
parameters (the CLI adds its exit code: 5 and 2). Synthetic data only (tools/wiff_grande.py). python3 is part of the test (it fails without it); skipped only without
cargo, wasm-bindgen or node."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli_argv, run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
PLAN = """const w = require(process.argv[2]), fs = require("fs"), names = [process.argv[3], process.argv[4]], bs = names.map(n => fs.readFileSync(n));
try { console.log(w.sample_ranges(names.map(n => n.split("/").pop()), new Float64Array(bs.map(b => b.length)), (i, o, n) => bs[i].subarray(o, o + n), process.argv[5])); }
catch (e) { console.log(e && e.message !== undefined ? e.message : (typeof e === "string" ? e : JSON.stringify(e))); process.exitCode = 3; }"""
LISTS = ["1", "3,1", "2-3", "1-4", "1,1,2", "1,9", "3-1", "1,,2", "x", ""]


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("sample_ranges")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d / "big"), "--samples", "4", "--scans", "40", "--points", "50"], check=True, capture_output=True)
    (d / "p.js").write_text(PLAN, encoding="utf-8")
    return d


def site(work, text):
    big = work / "big"
    p = subprocess.run(["node", str(work / "p.js"), str(work / "pkg" / "mzlab_wasm.js"), str(big / "grande.wiff"), str(big / "grande.wiff.scan"), text], cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout.strip()


def app(work, text):
    p = subprocess.run(cli_argv("sample", "select", str(work / "big" / "grande.wiff"), "--ranges", "--sample", text, "--json"), cwd=ROOT, capture_output=True, encoding="utf-8")
    return p.returncode, p.stdout.strip()


def test_the_site_and_the_cli_plan_the_same_ranges(work):
    for text in LISTS:
        code, out = site(work, text)
        rc, line = app(work, text)
        if code == 0:
            assert rc == 0 and line == out, (text, rc, line, out)
        else:
            a, b = json.loads(out), json.loads(line)
            assert rc in (2, 5) and (b["error_key"], b["params"]) == (a["error_key"], a["params"]), (text, out, line)
            assert rc == (5 if a["error_key"] == "err.sample.unknown" else 2), (text, rc)


def test_one_sample_leaves_the_others_out_and_all_of_them_copy_everything(work):
    size = (work / "big" / "grande.wiff.scan").stat().st_size
    one = json.loads(site(work, "2")[1])
    scan = next(m for m in one["members"] if m["name"].endswith(".wiff.scan"))
    assert one["samples"] == [2] and 0 < sum(b - a for a, b in scan["ranges"]) < size // 2, one
    every = json.loads(site(work, "1-4")[1])
    assert next(m for m in every["members"] if m["name"].endswith(".wiff.scan"))["ranges"] == [[0, size]]
    assert every["bytes"] == every["total"]
