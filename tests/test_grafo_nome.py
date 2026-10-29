"""Display form of molecule names in the Fragmentation graph (tests/grafo_nome_node.js). Needs node."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]


@pytest.mark.skipif(shutil.which("node") is None, reason="node not available")
def test_first_letter_capital_only_for_plain_lowercase_names():
    r = subprocess.run(["node", str(ROOT / "tests" / "grafo_nome_node.js")], capture_output=True, text=True, check=False)
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-2000:]
