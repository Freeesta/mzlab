"""`table.compare@1` in the `flowcompare` .wasm (mzlab-feat-wasm, feature `flow_compare`, through Node: `flow_compare`, the function the browser calls) gives the same bytes as
`mzlab-cli table compare` on the same `.mzfeat` of the synthetic unit-resolution files of `tools/lr_synthetic.py` (with and without a samples file) and as the native step of a `flow run` on them.
The golden file `tests/golden/compare_lr.json` is checked by `table_compare_cmd_tests.rs`. A table with one name on two columns (several samples of one file under the file name) is the same error
`err.compare.duplicate` in the wasm and the CLI (`test_a_table_with_one_name_on_two_columns_is_the_same_error_in_the_wasm_and_the_cli`). Skipped on a machine without cargo, wasm-bindgen, node or numpy;
in CI (GITHUB_ACTIONS) a missing tool FAILS `test_the_tools_are_there_in_ci` and numpy is required, so the wasm leg never goes quiet."""
import json
import os
import shutil
import sys
from pathlib import Path

import pytest
from cargo_util import CommandError, build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")

NAMES = ["LR_bench-noise-s1.mzML", "LR_bench-blank-s1.mzML", "LR_bench-s1.mzML"]
PARAMS = {"ref": "LR_bench-noise*", "blank": "LR_bench-blank*"}
NODE = """const w = require(process.argv[1] + '/mzlab_feat_wasm.js'), fs = require('fs');
const [dir, params, name, table, samples] = process.argv.slice(1);
try { process.stdout.write(w.flow_compare(params, name, fs.readFileSync(table), samples ? fs.readFileSync(samples) : new Uint8Array(0))); }
catch (e) { process.stdout.write('ERROR ' + String(e)); }"""


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    try:
        import numpy  # noqa: F401
    except ImportError:
        if CI:
            pytest.fail("numpy must be installed in CI: tools/lr_synthetic.py needs it")
        pytest.skip("numpy not available")
    d = tmp_path_factory.mktemp("compare")
    import lr_synthetic
    lr_synthetic.make(d, 1)
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_compare")))
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
        assert got == native(work, table, samples).removesuffix("\n")   # the command ends its standard output with a newline, the text has none
        assert json.loads(got)["schema"] == "compare/1" and json.loads(got)["rows"]


def test_the_step_of_the_flow_writes_the_same_bytes(work):
    table = work / "run" / "gap.mzfeat"
    assert (work / "run" / "cmp.json").read_text(encoding="utf-8") == wasm(work, table, work / "samples.json")


def test_an_error_has_the_same_key_in_the_wasm(work):
    got = wasm(work, work / "run" / "gap.mzfeat", params={"ref": "nothing*", "blank": "LR_bench-blank*"})
    assert got.startswith("ERROR") and "err.compare.selector" in got, got


def test_a_table_with_one_name_on_two_columns_is_the_same_error_in_the_wasm_and_the_cli(work):
    # the blank column takes the name of the noise column (both names are 22 bytes, so the header keeps its length): what a table of
    # several samples of one file under the file name looks like
    b = (work / "run" / "gap.mzfeat").read_bytes()
    assert NAMES[1].encode() in b and len(NAMES[0]) == len(NAMES[1])
    table = work / "dup.mzfeat"
    table.write_bytes(b.replace(NAMES[1].encode(), NAMES[0].encode()))
    got = wasm(work, table, params={"ref": "LR_bench-s1*", "blank": "nothing"})
    assert got.startswith("ERROR") and "err.compare.duplicate" in got, got
    with pytest.raises(CommandError) as e:
        cli("table", "compare", str(table), "--ref", "LR_bench-s1*", "--blank", "nothing")
    assert e.value.returncode == 4 and "err.compare.duplicate" in e.value.stderr and "features FILE --sample LIST" in e.value.stderr
