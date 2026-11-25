"""features.union@1: the flowtab .wasm (`flow_union`, through Node) gives the bytes of `mzlab-cli flow run` on the two synthetic tables committed under
crates/mzlab-feat/tests/data/union (the golden of the engine test), for the defaults and two sets of parameters, and the same error key.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-feat" / "tests" / "data" / "union"
TABLES = ["a.mzfeat", "b.mzfeat"]

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowtab")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_tab")))
    return d


def wasm(pkg, params):
    out = run("node", str(ROOT / "tests" / "flowtab_union_node.js"), str(pkg / "pkg"), json.dumps(params), *[str(DATA / t) for t in TABLES]).stdout
    return json.loads(out)


def native(tmp_path, params):
    for t in TABLES:
        shutil.copy2(DATA / t, tmp_path / t)
    plan = {"format": "mzflow/2", "inputs": [{"name": t, "type": "table"} for t in TABLES],
            "steps": [{"id": "u", "step": "features.union@1", "in": {"table": ["file:" + t for t in TABLES]}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(out))
    return (out / "u.csv").read_text(encoding="utf-8"), (out / "u.union.json").read_text(encoding="utf-8")


DEFAULTS = {"da": 0.5, "rt_tol": 0.1, "rt_warp": "off", "rt_gap": 0.5, "min_series": 1, "pad": 0, "polarity": "any"}


@pytest.mark.parametrize("params", [{}, {"min_series": 2, "polarity": "+"}, {"rt_tol": 0.02, "pad": 0.05}])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params):
    got = wasm(pkg, {**DEFAULTS, **params})
    assert (got["csv"], got["doc"]) == native(tmp_path, params)
    if not params:
        assert got["csv"] == (DATA / "union.csv").read_text(encoding="utf-8")
        assert got["doc"] == (DATA / "union.json").read_text(encoding="utf-8")


def test_a_bad_value_is_the_same_key(pkg):
    with pytest.raises(Exception) as e:
        wasm(pkg, {**DEFAULTS, "da": 9})
    assert "err.union.param" in str(e.value)
