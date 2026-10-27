"""mzlab/web/decima.js: the window search, the pixel columns and the range max/min of the drawing loop agree with the point-by-point loop they replaced."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_decima_matches_the_point_by_point_loop():
    r = subprocess.run(["node", str(ROOT / "tests" / "decima_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    o = json.loads(r.stdout)
    assert o["windows"] > 500 and o["traces"] >= 10
    assert o["mismatches"] == []
    assert o["lost_apex"] == []
    assert o["window_idx_bad"] == 0
    assert o["unsorted_ok"] is True
    assert o["big"]["max"] == 5e9 and o["big"]["min"] == -7e9 and abs(o["big"]["range"]) <= 1e3
    assert o["sorted"] == [True, False]
