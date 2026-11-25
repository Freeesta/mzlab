"""features.union@1 in the browser: `flow_series_csv` of the flowtab .wasm for each table, then `union_tables` of the rotte .wasm (through Node, as flow-web-union.js does) give the bytes of
`mzlab-cli flow run` on the two synthetic tables committed under crates/mzlab-feat/tests/data/union (the golden of the engine test), for the defaults and two sets of parameters, and the same error key.
Skipped without cargo, wasm-bindgen or node."""
import json
import os
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-feat" / "tests" / "data" / "union"
TABLES = ["a.mzfeat", "b.mzfeat"]

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")
# the names of the series are in the union/1 document: the file names the native run reads (`<id>.mzfeat`, or `view-<16 hex>.mzfeat` of a table.filter view) are what the page sends
NAMES = [["a.mzfeat", "b.mzfeat"], ["view-0123456789abcdef.mzfeat", "al-2.mzfeat"]]


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowtab")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_tab")))
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "rotte"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-rotte-wasm")))
    return d


def named(tmp_path, names):
    d = tmp_path / "in"
    d.mkdir(parents=True, exist_ok=True)
    for src, n in zip(TABLES, names):
        shutil.copy2(DATA / src, d / n)
    return d


def wasm(pkg, params, tmp_path, names):
    d = named(tmp_path, names)
    out = run("node", str(ROOT / "tests" / "flowtab_union_node.js"), str(pkg / "pkg"), str(pkg / "rotte"), json.dumps(params), *[str(d / n) for n in names]).stdout
    return json.loads(out)


def native(tmp_path, params, names):
    d = named(tmp_path, names)
    plan = {"format": "mzflow/2", "inputs": [{"name": t, "type": "table"} for t in names],
            "steps": [{"id": "u", "step": "features.union@1", "in": {"table": ["file:" + t for t in names]}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(d), "--out", str(out))
    return (out / "u.csv").read_text(encoding="utf-8"), (out / "u.union.json").read_text(encoding="utf-8")


DEFAULTS = {"da": 0.5, "rt_tol": 0.1, "rt_warp": "off", "rt_gap": 0.5, "min_series": 1, "pad": 0, "polarity": "any"}


@pytest.mark.parametrize("names", NAMES)
@pytest.mark.parametrize("params", [{}, {"min_series": 2, "polarity": "+"}, {"rt_tol": 0.02, "pad": 0.05}])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params, names):
    got = wasm(pkg, {**DEFAULTS, **params}, tmp_path / "w", names)
    assert (got["csv"], got["doc"]) == native(tmp_path / "n", params, names)
    if not params and names == NAMES[0]:
        assert got["csv"] == (DATA / "union.csv").read_text(encoding="utf-8")
        assert got["doc"] == (DATA / "union.json").read_text(encoding="utf-8")


def test_a_bad_value_is_the_same_key(pkg, tmp_path):
    with pytest.raises(Exception) as e:
        wasm(pkg, {**DEFAULTS, "da": 9}, tmp_path, NAMES[0])
    assert "err.union.param" in str(e.value)
