"""The Table view of the Flow on a features.union list with an `origin` column (flow/lazy/flow-results.js) through Node with a tiny DOM: a chip per combination of series with its count,
a chip filters the rows of the whole file, chips add up, a chip pressed twice goes off, and a list without the column has no chips. Fails in CI without node."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not shutil.which("node") and not os.environ.get("GITHUB_ACTIONS"), reason="node not available")


def test_origin_chips_filter_the_rows():
    p = subprocess.run(["node", str(ROOT / "tests" / "flow_origin_chip_node.js"), str(ROOT)], capture_output=True, encoding="utf-8", cwd=ROOT)
    assert p.returncode == 0, p.stderr
    got = json.loads(p.stdout)
    assert got["chips"] == ["direct+suspect (1)", "direct (2)", "suspect (1)"]
    assert (got["all"], got["direct"], got["pressed"], got["two"], got["back"], got["off"], got["none"]) == (4, 2, "true", 3, 1, "false", 0)
