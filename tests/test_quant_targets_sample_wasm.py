"""`quant targets --sample LIST` on a multi-sample `.wiff`: the table and the log of the CLI, of a `flow run` over `NAME@ID` inputs and of the `quanttargets` .wasm
(`flow_quant_targets`, through Node, the function the browser calls over the indexes of OPFS) are the same bytes. The wasm is given the names the CLI's `--dry-run` says
(`members[].name`, the label rule `mzlab_core::readers::label`), also for two samples of one name (the tie-break `NAME (FILE #ID)`). Synthetic files of `tools/wiff_grande.py`.
Skipped only on a machine without cargo, wasm-bindgen, node or python3; in CI (GITHUB_ACTIONS) those are required and a missing one fails the run, never skips it."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node", "python3"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen, node or python3 not available")

PARAMS = {"da": 0.5, "min_sn": 3, "min_points": 5, "min_shape": 0.5, "min_half": 0, "max_fwhm": 0, "max_asym": 0, "offset": "0", "blank_pattern": "", "ms_level": 1, "smooth_half": 0, "valley": 10}
NODE = """const [dir, params, names, targets, ...paths] = process.argv.slice(1);
const w = require(dir + '/mzlab_feat_wasm.js'), fs = require('fs');
const fds = paths.map(p => fs.openSync(p, 'r')), sizes = paths.map(p => fs.statSync(p).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
try { const r = w.flow_quant_targets(read, sizes, names, fs.readFileSync(targets), params); process.stdout.write(JSON.stringify({ csv: r.csv(), log: r.log() })); }
catch (e) { process.stdout.write('ERROR ' + String(e)); }"""
TARGETS = "id,mz,rt_min,rt_max\nT1,229.1,0.2,1.0\nAway,555.5,0.2,1.0\n"


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen, node and python3 must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("quant_sample_pkg")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "quant_targets")))
    build("-p", "mzlab-cli")
    return d


def make(d, names):
    made = subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(d), "--samples", str(len(names)), "--scans", "40", "--points", "50", "--names", ",".join(names)],
                          capture_output=True, encoding="utf-8")
    assert made.returncode == 0, made.stderr
    (d / "targets.csv").write_text(TARGETS, encoding="utf-8")
    return d / "grande.wiff"


@pytest.mark.parametrize("names, ids", [(["Alpha", "Beta", "Gamma"], [1, 3]), (["Same", "Same", "Other"], [1, 2, 3])], ids=["own names", "a tie"])
def test_cli_flow_and_wasm_give_the_same_table_and_log_for_the_chosen_samples(tmp_path, pkg, names, ids):
    wiff = make(tmp_path, names)
    pick = ",".join(map(str, ids))
    log = tmp_path / "cli.log.json"
    table = cli("quant", "targets", "--targets", str(tmp_path / "targets.csv"), str(wiff), "--sample", pick, "--log", str(log)).stdout
    plan = json.loads(cli("quant", "targets", "--targets", str(tmp_path / "targets.csv"), str(wiff), "--sample", pick, "--dry-run").stdout)
    members = plan["members"]
    assert [m["member"] for m in members] == [f"grande.wiff@{i}" for i in ids], members
    cols = [m["name"] for m in members]
    assert len(set(cols)) == len(cols) and [r.split(",")[0] for r in table.splitlines()[1:]] == [c for c in cols for _ in range(2)], (cols, table)
    flow = {"format": "mzflow/2", "inputs": [{"name": f"grande.wiff@{i}", "type": "raw"} for i in ids] + [{"name": "targets.csv", "type": "report"}],
            "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": [f"file:grande.wiff@{i}" for i in ids]}},
                      {"id": "qt", "step": "quant.targets@1", "in": {"raw": ["idx.index"], "targets": "file:targets.csv"}}]}
    (tmp_path / "plan.json").write_text(json.dumps(flow), encoding="utf-8")
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(tmp_path / "run"))
    assert (tmp_path / "run" / "qt.csv").read_text(encoding="utf-8") == table
    assert (tmp_path / "run" / "qt.log.json").read_text(encoding="utf-8") == log.read_text(encoding="utf-8")
    paths = [str(tmp_path / "run" / f"idx-grande-{i}.mzidx") for i in ids]
    out = run("node", "-e", NODE, str(pkg), json.dumps(PARAMS), json.dumps(cols), str(tmp_path / "targets.csv"), *paths).stdout
    assert not out.startswith("ERROR"), out
    got = json.loads(out)
    assert got["csv"] == table and got["log"] == log.read_text(encoding="utf-8")
