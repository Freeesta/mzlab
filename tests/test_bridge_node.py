"""mzlab/web/rust-bridge.js with a fake Worker: a stale open after a worker death leaves the respawned slot alone; xic/mrm requests overlap."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_bridge_stale_open_and_overlapping_requests():
    r = subprocess.run(["node", str(ROOT / "tests" / "bridge_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-2000:]
