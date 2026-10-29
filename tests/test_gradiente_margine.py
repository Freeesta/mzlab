"""mzlab/web/gradiente.js: the right axis of the gradient / pressure overlay keeps its rotated title inside the panel, at any text size and tick label width."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_right_axis_title_is_never_clipped():
    r = subprocess.run(["node", str(ROOT / "tests" / "gradiente_margine_node.js")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    o = json.loads(r.stdout)
    assert o["bad"] == []
    assert all(x["right"] <= x["r"] for x in o["rows"])
    # the old fixed margin let the title ascent run past the canvas edge at every size
    assert all(b["overhang"] > 0 for b in o["before"])
    fits = dict(o["fits"])
    assert fits["%B"] == "%B" and fits["bar"] == "bar"
    assert fits["A very long axis title that cannot fit"].endswith("…") and len(fits["A very long axis title that cannot fit"]) <= 8
