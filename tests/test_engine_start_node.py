"""Engine start (issue #3509): a worker lost during lr_boot is retried on its replacement; a second loss is flagged dead; the cache recovery runs once."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_engine_start_retry_and_recovery():
    r = subprocess.run(["node", str(ROOT / "tests" / "engine_start_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, (r.stdout + r.stderr)[-2000:]
