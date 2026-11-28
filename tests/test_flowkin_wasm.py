"""quant.kinetics@1: the flowkin .wasm (`flow_kinetics`, through Node) gives the bytes of `mzlab-cli flow run` on the committed synthetic series of
crates/mzlab-cli/tests/data/kinetics (two conditions of known rate sharing one t0, invented injection factors; the golden file is written by
`flow_kinetics_tests.rs`), with and without the UV input and under several parameters. Skipped without cargo, wasm-bindgen or node."""
import json
import os
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-cli" / "tests" / "data" / "kinetics"
GOLDEN_PARAMS = {"ids": "parent;ghost"}   # the plan of flow_kinetics_tests.rs

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowkin")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-flowkin-wasm")))
    return d


def wasm(pkg, params, uv=True):
    args = ["node", str(ROOT / "tests" / "flowkin_node.js"), str(pkg / "pkg"), json.dumps(params), str(DATA / "areas.csv"), str(DATA / "samples.json")]
    return json.loads(run(*args, *([str(DATA / "uv.csv")] if uv else [])).stdout)


def native(tmp_path, params, uv=True):
    for f in ("areas.csv", "samples.json", "uv.csv"):
        shutil.copy2(DATA / f, tmp_path / f)
    inputs = [{"name": "areas.csv", "type": "report"}, {"name": "samples.json", "type": "samples"}] + ([{"name": "uv.csv", "type": "peaks"}] if uv else [])
    wires = {"areas": "file:areas.csv", "samples": "file:samples.json", **({"uv": "file:uv.csv"} if uv else {})}
    plan = {"format": "mzflow/2", "inputs": inputs, "steps": [{"id": "k", "step": "quant.kinetics@1", "in": wires, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(out))
    return (out / "k.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("params,uv", [(GOLDEN_PARAMS, True), ({}, True), ({}, False), ({"uv_mode": "check", "missing": "zero", "use_t0": False}, True), ({"ids": "parent", "boot": 50, "seed": 7, "uv_rt": 4.3, "uv_tol": 5}, True)])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params, uv):
    got = wasm(pkg, params, uv)
    text = native(tmp_path, params, uv)
    assert got["kinetics"] == text
    assert text.startswith('{"format":"kinetics-series/1"')
    if params == GOLDEN_PARAMS:
        assert text == (DATA / "golden.json").read_text(encoding="utf-8")
        assert "warn.flow.kinetics.notFitted" in got["warnings"]


def test_an_error_is_the_same_key(pkg):
    with pytest.raises(Exception) as e:
        wasm(pkg, {"uv_mode": "both"})
    assert "err.flow.kinetics.param" in str(e.value)
