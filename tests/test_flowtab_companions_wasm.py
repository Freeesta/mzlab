"""features.companions@1: the flowtab .wasm (`flow_companions`, through Node) gives the bytes of `mzlab-cli flow run` on the synthetic unit-mass table committed under
crates/mzlab-feat/tests/data/companions (the golden of the engine test), for the defaults and three sets of parameters (one with the isotope grouping on), and the same error key and warning.
Skipped on a machine without cargo, wasm-bindgen or node, never in CI."""
import json
import os
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-feat" / "tests" / "data" / "companions"

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowtab")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_tab")))
    return d


def wasm(pkg, params):
    out = run("node", str(ROOT / "tests" / "flowtab_companions_node.js"), str(pkg / "pkg"), json.dumps(params), str(DATA / "table.mzfeat")).stdout
    return json.loads(out)


def native(tmp_path, params):
    shutil.copy2(DATA / "table.mzfeat", tmp_path / "table.mzfeat")
    plan = {"format": "mzflow/2", "inputs": [{"name": "table.mzfeat", "type": "table"}],
            "steps": [{"id": "c", "step": "features.companions@1", "in": {"table": "file:table.mzfeat"}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(out))
    return (out / "c.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("params", [{}, {"losses": "", "min_corr": 0.95}, {"min_corr_samples": 6, "da_tol": 0.2}, {"iso_group": True, "spill_max": 0.9}])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params):
    got = wasm(pkg, params)
    assert got["text"] == native(tmp_path, params)
    if not params:
        assert got["text"] == (DATA / "companions.json").read_text(encoding="utf-8")
    if params.get("iso_group"):
        assert '"isotopes":{"params":' in got["text"]
    if params.get("min_corr_samples") == 6:
        assert got["warnings"] == ["warn.flow.companions.corrFiles"]


def test_a_bad_list_is_the_same_key(pkg):
    with pytest.raises(Exception) as e:
        wasm(pkg, {"adducts": "17,x"})
    assert "err.flow.companions.list" in str(e.value)
