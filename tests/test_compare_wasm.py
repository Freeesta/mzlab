"""`table.compare@1` in the `flowstat` .wasm (mzlab-feat-wasm, feature `flow_stat`, through Node: `flow_compare`, the function the browser calls) gives the same bytes as
`mzlab-cli table compare` on the same `.mzfeat` of the synthetic unit-resolution files of `tools/lr_synthetic.py` (with and without a samples file) and as the native step of a `flow run` on them.
The golden file `tests/golden/compare_lr.json` is checked by `table_compare_cmd_tests.rs`. Skipped without cargo, wasm-bindgen, node or numpy."""
import json
import shutil
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NAMES = ["LR_bench-noise-s1.mzML", "LR_bench-blank-s1.mzML", "LR_bench-s1.mzML"]
PARAMS = {"ref": "LR_bench-noise*", "blank": "LR_bench-blank*"}
NODE = """const w = require(process.argv[1] + '/mzlab_feat_wasm.js'), fs = require('fs');
const [dir, params, name, table, samples] = process.argv.slice(1);
try { process.stdout.write(w.flow_compare(params, name, fs.readFileSync(table), samples ? fs.readFileSync(samples) : new Uint8Array(0))); }
catch (e) { process.stdout.write('ERROR ' + String(e)); }"""


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    pytest.importorskip("numpy")
    d = tmp_path_factory.mktemp("compare")
    import lr_synthetic
    lr_synthetic.make(d, 1)
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_stat")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    build("-p", "mzlab-cli")
    names = [str(d / n) for n in NAMES]
    (d / "samples.json").write_text(cli("sample", "table", *names).stdout, encoding="utf-8")
    plan = {"format": "mzflow/2", "inputs": [{"name": n, "type": "raw"} for n in NAMES] + [{"name": "samples.json", "type": "samples"}],
            "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in NAMES]}},
                      {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                      {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                      {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
                      {"id": "cmp", "step": "table.compare@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": PARAMS}]}
    (d / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    cli("flow", "run", str(d / "plan.json"), "--in", str(d), "--out", str(d / "run"))
    return d


def wasm(work, table, samples=None, params=PARAMS):
    args = ["node", "-e", NODE, str(work / "pkg"), json.dumps(params), table.name, str(table), str(samples) if samples else ""]
    return run(*args).stdout


def native(work, table, samples=None):
    args = ["table", "compare", str(table), "--ref", PARAMS["ref"], "--blank", PARAMS["blank"]]
    return cli(*args, *(["--samples", str(samples)] if samples else [])).stdout


def test_wasm_and_cli_give_the_same_bytes_with_and_without_samples(work):
    table = work / "run" / "gap.mzfeat"
    for samples in (None, work / "samples.json"):
        got = wasm(work, table, samples)
        assert not got.startswith("ERROR"), got
        assert got == native(work, table, samples)
        assert json.loads(got)["schema"] == "compare/1" and json.loads(got)["rows"]


def test_the_step_of_the_flow_writes_the_same_bytes(work):
    table = work / "run" / "gap.mzfeat"
    assert (work / "run" / "cmp.json").read_text(encoding="utf-8") == wasm(work, table, work / "samples.json")


def test_an_error_has_the_same_key_in_the_wasm(work):
    got = wasm(work, work / "run" / "gap.mzfeat", params={"ref": "nothing*", "blank": "LR_bench-blank*"})
    assert got.startswith("ERROR") and "err.compare.selector" in got, got

