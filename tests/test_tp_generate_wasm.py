"""tp.generate@1 in the browser: `tp_generate` of the mzlab-feat-wasm .wasm (through Node) gives the bytes of `mzlab-cli flow run` for a formula parent, with the defaults and with the
optional formula filter (`formula_filter`, `filter_steps`, `m2_tol`, `m2_ref`), and the filtered registry carries the `filter` block that the default one lacks.
Skipped on a machine without cargo, wasm-bindgen or node, never in CI."""
import json
import os
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
RULES = ROOT / "TP_Mine" / "data" / "regole_tp_3.json"
PARENT = "C13H24N4O3S"

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS") or os.environ.get("CI"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")
PARAMS = [{}, {"max_steps": 1}, {"formula_filter": True}, {"formula_filter": True, "filter_steps": 1, "m2_tol": 0.05, "m2_ref": 0.1}]


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("hr")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", runner=run)))
    return d / "pkg"


def wasm(pkg, params):
    return run("node", str(ROOT / "tests" / "tp_generate_node.js"), str(pkg), str(RULES), PARENT, json.dumps(params)).stdout


def native(tmp_path, params):
    (tmp_path / "parent.txt").write_text(PARENT + "\n", encoding="utf-8")
    plan = {"format": "mzflow/2", "inputs": [{"name": "parent.txt", "type": "structure"}],
            "steps": [{"id": "g", "step": "tp.generate@1", "in": {"parent": "file:parent.txt"}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(out))
    return (out / "g.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("params", PARAMS, ids=["defaults", "max_steps", "filter", "filter_set"])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params):
    got = wasm(pkg, params)
    assert got == native(tmp_path, params)
    assert ('"filter":{' in got) == bool(params.get("formula_filter"))
