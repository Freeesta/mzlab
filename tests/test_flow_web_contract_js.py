"""The page runners against the real shapes of the workers, through Node: `ident` of feat-worker.js gives the text of an op unless asked for the whole answer (tests/real_pool.js cuts it out of the
worker), so features.companions must ask for the whole answer to get its warnings; and the series name `tableOf` gives for a table or a filter view is the file name the native run reads
(`view-<16 hex of the SHA-256 of the view>.mzfeat`, `<id>.mzfeat`; the Rust side is pinned by `the_file_of_a_view_is_named_after_the_sha_of_its_text` on the same text). Fails in CI without node."""
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not shutil.which("node") and not os.environ.get("GITHUB_ACTIONS"), reason="node not available")


def node(script, *args):
    p = subprocess.run(["node", str(ROOT / "tests" / script), str(ROOT), *args], capture_output=True, encoding="utf-8", cwd=ROOT)
    assert p.returncode == 0, p.stderr
    return json.loads(p.stdout)


def test_companions_keeps_the_text_and_the_warnings_of_the_worker_op():
    got = node("flow_web_companions_node.js")
    assert got["error"] is None
    text = 'companions/1 of 3 bytes with {"k":2}'
    assert got["files"]["c"] == {"id": "c", "name": "c.json", "text": text}
    assert got["remembered"][0]["v"] == {"name": "c.json", "text": text, "warnings": ["w.one"]}
    assert got["notes"] == {"c": "w.one"}


def test_the_series_name_of_a_table_is_the_file_name_of_the_native_run():
    view = "view text of a table.filter"
    got = node("flow_table_name_node.js", view)
    assert got["direct"] == "al.mzfeat"
    assert got["view"] == "view-" + hashlib.sha256(view.encode()).hexdigest()[:16] + ".mzfeat"
    assert node("flow_table_name_node.js", "abc")["view"] == "view-ba7816bf8f01cfea.mzfeat"   # the vector of the Rust test
