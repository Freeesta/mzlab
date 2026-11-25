"""features.union in the page (flow/lazy/flow-web-union.js) through Node with a mock of the page: each table goes to `flow_series_csv` of flowtab as bytes, the CSV texts and the
names of the series go to `lr_union` of the rust worker with the parameters of the instance merged over the defaults, the two outputs are the CSV of the document and the document itself,
and an error of the engine reaches the page as its key. The .wasm halves are checked by tests/test_flowtab_union_wasm.py. Fails in CI without node."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not shutil.which("node") and not os.environ.get("GITHUB_ACTIONS"), reason="node not available")


def page(answer):
    p = subprocess.run(["node", str(ROOT / "tests" / "flow_web_union_node.js"), str(ROOT), json.dumps(answer)], capture_output=True, encoding="utf-8", cwd=ROOT)
    assert p.returncode == 0, p.stderr
    return json.loads(p.stdout)


def test_the_page_sends_the_csv_of_each_table_to_the_join_and_keeps_both_outputs():
    doc = json.dumps({"schema": "union/1", "csv": "id,mz\nU0001,1\n"})
    got = page({"info": doc})
    assert got["error"] is None
    assert [(i["mod"], i["op"], i["table"]) for i in got["ident"]] == [("flowtab", "flow_series_csv", [1, 2]), ("flowtab", "flow_series_csv", [3])]
    (rpc,) = got["rpc"]
    assert rpc["op"] == "lr_union"
    body = json.loads(rpc["body"])
    assert body["tables"] == [{"name": "a.mzfeat", "text": "id,mz,rt\ncsv-of-2\n"}, {"name": "view-0123456789abcdef.mzfeat", "text": "id,mz,rt\ncsv-of-1\n"}]
    assert body["params"] == {"da": 0.3, "polarity": "any"}
    assert got["files"]["u"]["text"] == "id,mz\nU0001,1\n" and got["files"]["u"]["name"] == "u.csv"
    assert got["files"]["u.union"]["text"] == doc and got["files"]["u.union"]["name"] == "u.union.json"
    assert got["remembered"] == ["sha:spec\nka\nkb", "sha:spec\nka\nkb-union"]


def test_an_error_of_the_engine_reaches_the_page_as_its_key():
    got = page({"error": json.dumps({"error_key": "err.union.param", "params": {"name": "da"}})})
    assert got["error"]["key"] == "err.union.param" and got["error"]["params"] == {"name": "da"}


def test_node_is_there_in_ci():
    assert shutil.which("node") or not os.environ.get("GITHUB_ACTIONS")
