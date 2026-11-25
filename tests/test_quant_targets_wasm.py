"""`quant.targets@1` in the `quanttargets` .wasm (mzlab-feat-wasm, feature `quant_targets`, through Node: `flow_quant_targets`, the function the browser
calls over the .mzidx of OPFS) gives the same table and log bytes as `mzlab-cli quant targets` on the mzML files and as the native step of a `flow run`
on the synthetic unit-resolution files of `tools/lr_synthetic.py`, with the shape gates off (the defaults) and on (`min_half`). Skipped on a machine without
cargo, wasm-bindgen, node or numpy; in CI (GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import json
import os
import shutil
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

NAMES = ["LR_bench-noise-s1.mzML", "LR_bench-blank-s1.mzML", "LR_bench-s1.mzML"]
BLANK = "LR_bench-blank*"
# the parameters as the page sends them: the defaults of the step merged with the ones the plan writes
PARAMS = {"da": 0.5, "min_sn": 3, "min_points": 5, "min_shape": 0.5, "min_half": 0, "max_fwhm": 0, "max_asym": 0, "offset": "0", "blank_pattern": BLANK, "ms_level": 1, "smooth_half": 0, "valley": 10}
NODE = """const [dir, params, names, targets, ...paths] = process.argv.slice(1);
const w = require(dir + '/mzlab_feat_wasm.js'), fs = require('fs');
const fds = paths.map(p => fs.openSync(p, 'r')), sizes = paths.map(p => fs.statSync(p).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
try { const r = w.flow_quant_targets(read, sizes, names, fs.readFileSync(targets), params); process.stdout.write(JSON.stringify({ csv: r.csv(), log: r.log(), warnings: JSON.parse(r.warnings()) })); }
catch (e) { process.stdout.write('ERROR ' + String(e)); }"""


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"
    if CI:
        import numpy  # noqa: F401  (required in CI: a missing numpy fails, never skips)
    else:
        pytest.importorskip("numpy")
    d = tmp_path_factory.mktemp("quant_targets")
    import lr_synthetic
    lr_synthetic.make(d, 1)
    truth = json.loads((d / "LR_bench-s1_truth.json").read_text(encoding="utf-8"))
    rows = ["id,mz,rt_min,rt_max"] + [f"c{c['id']},{c['mz']},{c['rt_s'] / 60 - 0.3:.3f},{c['rt_s'] / 60 + 0.3:.3f}" for c in truth["compounds"][:15]]
    rows.append("away,333.3,0.1,0.4")   # no peak there: an absent row
    (d / "targets.csv").write_text("\n".join(rows) + "\n", encoding="utf-8")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "quant_targets")))
    build("-p", "mzlab-cli")
    plan = {"format": "mzflow/2", "inputs": [{"name": n, "type": "raw"} for n in NAMES] + [{"name": "targets.csv", "type": "report"}],
            "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in NAMES]}},
                      {"id": "qt", "step": "quant.targets@1", "in": {"raw": ["idx.index"], "targets": "file:targets.csv"}, "params": {"blank_pattern": BLANK}}]}
    (d / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    cli("flow", "run", str(d / "plan.json"), "--in", str(d), "--out", str(d / "run"))
    return d


def wasm(work, targets, params=PARAMS):
    paths = [str(work / "run" / f"idx-{Path(n).stem}.mzidx") for n in NAMES]
    out = run("node", "-e", NODE, str(work / "pkg"), json.dumps(params), json.dumps(NAMES), str(targets), *paths).stdout
    return out if out.startswith("ERROR") else json.loads(out)


def test_wasm_cli_and_flow_run_give_the_same_table_and_log(work):
    got = wasm(work, work / "targets.csv")
    assert not isinstance(got, str), got
    log = work / "cli.log.json"
    table = cli("quant", "targets", "--targets", str(work / "targets.csv"), *[str(work / n) for n in NAMES], "--blank-pattern", BLANK, "--log", str(log)).stdout
    assert got["csv"] == table == (work / "run" / "qt.csv").read_text(encoding="utf-8")
    assert got["log"] == log.read_text(encoding="utf-8") == (work / "run" / "qt.log.json").read_text(encoding="utf-8")
    states = [r.split(",")[7] for r in got["csv"].splitlines()[1:]]
    assert len(states) == 3 * 16 and "detected" in states and "absent" in states, states
    # a peak under a threshold is a forced row: the hint (a warning key, one text for every surface) is the same list in the three
    assert got["warnings"] == (["warn.quant.targets.forced"] if "forced" in states else []), got["warnings"]


def test_the_shape_gates_give_the_same_table_log_and_decisions_in_the_wasm_and_the_cli(work):
    params = {**PARAMS, "min_half": 40, "max_fwhm": 1, "max_asym": 1.001}
    got = wasm(work, work / "targets.csv", params)
    assert not isinstance(got, str), got
    log = work / "gates.log.json"
    table = cli("quant", "targets", "--targets", str(work / "targets.csv"), *[str(work / n) for n in NAMES], "--blank-pattern", BLANK,
                "--set", "min_half=40", "--set", "max_fwhm=1", "--set", "max_asym=1.001", "--log", str(log)).stdout
    assert got["csv"] == table
    assert got["log"] == log.read_text(encoding="utf-8")
    d = json.loads(got["log"])
    assert d["params"]["min_half"] == 40 and d["decisions"], d["decisions"]
    assert {c["name"] for x in d["decisions"] for c in x["checks"]} >= {"half_pts", "fwhm_s", "asym"}


def test_an_error_has_the_same_key_in_the_wasm(work):
    bad = work / "bad.csv"
    bad.write_text("id,mz\nx,1\n", encoding="utf-8")
    got = wasm(work, bad)
    assert isinstance(got, str) and got.startswith("ERROR") and "err.quant.targets.table" in got, got
