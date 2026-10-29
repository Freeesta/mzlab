"""mzlab/web/rust-supervisor.js: the module reaches the worker right after compile(), before the probe; the rest keeps its order."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
ORDER = ["module", "policy", "budget", "down", "lr_open", "lr_xic"]


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_module_is_posted_before_the_probe_and_the_order_holds():
    r = subprocess.run(["node", str(ROOT / "tests" / "supervisor_ordine_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    o = json.loads(r.stdout)
    for k in ("slowProbe", "fastProbe"):
        assert [m["op"] for m in o[k]] == ORDER
    slow = {m["op"]: m["at"] for m in o["slowProbe"]}
    assert slow["module"] < 200, "the module must not wait for the 400 ms probe"
    assert slow["policy"] >= 380, "the policy still waits for the probe"
    assert o["idleTimeout"] < 800
    assert o["whenProbedMs"] >= 750
