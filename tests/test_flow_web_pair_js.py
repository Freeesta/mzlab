"""features.pair in the page (flow/lazy/flow-web-pair.js) through Node with a mock of the page: the bytes of each table go to `flow_pair` of flowtab with the names the page gives them and the
parameters of the instance merged over the defaults, the two outputs are the CSV of the document and the document itself, and an error of the engine reaches the page as its key. The .wasm half
is checked by tests/test_flowtab_pair_wasm.py. Fails in CI without node."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not shutil.which("node") and not os.environ.get("GITHUB_ACTIONS"), reason="node not available")


def page(answer):
    p = subprocess.run(["node", str(ROOT / "tests" / "flow_web_pair_node.js"), str(ROOT), answer], capture_output=True, encoding="utf-8", cwd=ROOT)
    assert p.returncode == 0, p.stderr
    return json.loads(p.stdout)


def test_the_page_sends_both_tables_to_the_engine_and_keeps_both_outputs():
    doc = json.dumps({"schema": "pair/1", "csv": "pos_id,neg_id\n1,1\n"})
    got = page(doc)
    assert got["error"] is None
    (call,) = got["ident"]
    assert (call["posName"], call["negName"], call["pos"], call["neg"]) == ("p.mzfeat", "view-0123456789abcdef.mzfeat", [1, 2], [3])
    assert call["params"] == {"rt_tol": 0.3, "mass_unit": "ppm"}
    assert got["files"]["pr"]["text"] == "pos_id,neg_id\n1,1\n" and got["files"]["pr"]["name"] == "pr.csv"
    assert got["files"]["pr.pair"]["text"] == doc and got["files"]["pr.pair"]["name"] == "pr.pair.json"
    assert got["remembered"] == ["sha:spec\nkp\nkn", "sha:spec\nkp\nkn-pair"]


def test_an_error_of_the_engine_reaches_the_page_as_its_key():
    got = page("ERR" + json.dumps({"error_key": "err.pair.param", "params": {"name": "rt_tol"}}))
    assert got["error"]["key"] == "err.pair.param" and got["error"]["params"] == {"name": "rt_tol"}


def test_node_is_there_in_ci():
    assert shutil.which("node") or not os.environ.get("GITHUB_ACTIONS")
