"""features.pair@1 in the browser: `flow_pair` of the flowtab .wasm (through Node, as flow-web-pair.js calls it) gives the bytes of `mzlab-cli flow run` and of `mzlab-cli table pair`
on the two synthetic tables committed under crates/mzlab-feat/tests/data/pair (the golden of the engine test), for the defaults and two sets of parameters, and the same error key.
Skipped without cargo, wasm-bindgen or node on a local machine; in CI it fails instead."""
import json
import os
import shutil
from pathlib import Path

import pytest
from cargo_util import cli, run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-feat" / "tests" / "data" / "pair"
TABLES = ["pos.mzfeat", "neg.mzfeat"]

TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")
# the names of the sides are the file names the native run reads (`<id>.mzfeat`, or `view-<16 hex>.mzfeat` of a table.filter view) and the page sends
NAMES = [TABLES, ["al-1.mzfeat", "view-0123456789abcdef.mzfeat"]]
DEFAULTS = {"rt_tol": 0.1, "mass_tol": 5, "mass_unit": "ppm", "pos_adducts": "[M+H]+", "neg_adducts": "[M-H]-"}


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowtab")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "flow_tab")))
    return d


def named(tmp_path, names):
    d = tmp_path / "in"
    d.mkdir(parents=True, exist_ok=True)
    for src, n in zip(TABLES, names):
        shutil.copy2(DATA / src, d / n)
    return d


def wasm(pkg, params, tmp_path, names):
    d = named(tmp_path, names)
    out = run("node", str(ROOT / "tests" / "flowtab_pair_node.js"), str(pkg / "pkg"), json.dumps(params), *[str(d / n) for n in names]).stdout
    return json.loads(out)


def native(tmp_path, params, names):
    d = named(tmp_path, names)
    plan = {"format": "mzflow/2", "inputs": [{"name": t, "type": "table"} for t in names],
            "steps": [{"id": "pr", "step": "features.pair@1", "in": {"pos": "file:" + names[0], "neg": "file:" + names[1]}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(d), "--out", str(out))
    return (out / "pr.csv").read_text(encoding="utf-8"), (out / "pr.pair.json").read_text(encoding="utf-8")


@pytest.mark.parametrize("names", NAMES)
@pytest.mark.parametrize("params", [{}, {"rt_tol": 0.01}, {"mass_tol": 0.2, "mass_unit": "da", "pos_adducts": "[M+H]+,[M+Na]+"}])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params, names):
    got = wasm(pkg, {**DEFAULTS, **params}, tmp_path / "w", names)
    assert (got["csv"], got["doc"]) == native(tmp_path / "n", params, names)
    if not params and names == NAMES[0]:
        assert got["csv"] == (DATA / "pair.csv").read_text(encoding="utf-8")
        assert got["doc"] == (DATA / "pair.json").read_text(encoding="utf-8")


def test_table_pair_writes_the_same_pairs(pkg, tmp_path):
    d = named(tmp_path, TABLES)
    csv = cli("table", "pair", str(d / TABLES[0]), str(d / TABLES[1])).stdout
    assert csv == (DATA / "pair.csv").read_text(encoding="utf-8")
    doc = cli("table", "pair", str(d / TABLES[0]), str(d / TABLES[1]), "--json").stdout
    assert json.loads(doc)["csv"] == csv


def test_a_bad_value_is_the_same_key(pkg, tmp_path):
    with pytest.raises(Exception) as e:
        wasm(pkg, {**DEFAULTS, "rt_tol": -1}, tmp_path, NAMES[0])
    assert "err.pair.param" in str(e.value)
