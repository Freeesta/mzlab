"""`feature-slots/1` of a row in the site: the lazy .wasm `slots` (mzlab-feat-slots-wasm, through Node: `table_slots(read, size, samples, id, output)`, the call
TP_Mine/js/36_feature_qc.js makes) gives the same bytes as the native example `slots` (the app function) and as `mzlab-cli table slots`, on the synthetic series of
tools/feat_mirato.py aligned by the native example `native`. An id the table does not have is exit 5. Skipped on a machine without cargo, wasm-bindgen, node or numpy; in CI
(GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build, cli, cli_argv, example, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NODE = """const [dir, table, samples, id, output] = process.argv.slice(2);
const { table_slots } = require(dir + '/mzlab_feat_slots_wasm.js'), fs = require('fs');
const bytes = fs.readFileSync(table);
let out; try { out = { s: table_slots((_, off, len) => bytes.subarray(off, off + len), bytes.length, fs.readFileSync(samples, 'utf8'), Number(id), output) }; } catch (x) { out = { e: String(x) }; }
process.stdout.write(JSON.stringify(out));"""


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


def samples_json(idx):
    rows = [{"blank_kind": None, "compound": "", "concentration": None, "concentration_unit": None, "file": Path(p).stem + ".mzML", "injection_order": i + 1, "istd": False,
             "kind": "sample", "note": "", "replicate": 1, "series": "", "sha256": "00" * 32, "time": None, "time_confirmed": False, "time_min": None, "time_unit": "min"} for i, p in enumerate(idx)]
    return json.dumps({"schema": "samples/1", "samples": rows})


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    try:
        import numpy  # noqa: F401  (tools/feat_mirato.py needs it)
    except ImportError:
        if CI:
            raise
        pytest.skip("numpy not available")
    root = tmp_path_factory.mktemp("slots")
    ser = root / "ser"
    run("python3", "tools/feat_mirato.py", "--seed", "2", "--profile", "piccolo", "--scale", "1.0", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    build("-p", "mzlab-feat-wasm", "--example", "native")
    build("-p", "mzlab-feat-slots-wasm", "--example", "slots")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-slots-wasm")))
    series = root / "series"
    series.mkdir()
    run("target/release/examples/native", str(series), *idx)
    (root / "samples.json").write_text(samples_json(idx))
    (root / "run.js").write_text(NODE, encoding="utf-8")
    return root, pkg, series / "series.mzfeat"


def test_cli_table_slots_gives_the_bytes_of_the_app_and_of_the_wasm(built):
    """FEATURE-DETAIL: `mzlab-cli table slots` prints feature-slots/1 with the bytes of the native example (the app function) and of the .wasm (`table_slots`)."""
    root, pkg, table = built
    nat = example("mzlab-feat-slots-wasm", "slots", str(table), str(root / "samples.json"), "3", "table.mzfeat").stdout
    got = json.loads(subprocess.run(["node", str(root / "run.js"), str(pkg), str(table), str(root / "samples.json"), "3", "table.mzfeat"], capture_output=True, encoding="utf-8", check=True).stdout)
    assert got["s"] == nat
    out = cli("table", "slots", str(table), "--id", "3", "--samples", str(root / "samples.json")).stdout
    # the example names the table "table.mzfeat"; the CLI names it after the file it was given
    assert out.replace(f'"output":"{table.name}"', '"output":"table.mzfeat"') == nat + "\n"
    doc = json.loads(out)
    assert doc["schema"] == "feature-slots/1" and doc["id"] == 3 and [s["slot"] for s in doc["slots"]][:3] == ["structure", "xic", "kinetics"]


def test_an_id_the_table_does_not_have_is_the_same_error_everywhere(built):
    root, pkg, table = built
    bad = subprocess.run([*cli_argv("table", "slots", str(table), "--id", "99999999")], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert bad.returncode == 5 and json.loads(bad.stdout)["error_key"] == "error.rowNotFound"
    w = json.loads(subprocess.run(["node", str(root / "run.js"), str(pkg), str(table), str(root / "samples.json"), "99999999", ""], capture_output=True, encoding="utf-8", check=True).stdout)
    assert json.loads(w["e"])["error_key"] == "error.rowNotFound"
