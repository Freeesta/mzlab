"""mzlab/web/compat.js: every fallback for what Safari/iPadOS lacks (AbortSignal.any, createWritable, a refused clipboard, the accept list of the iPad
file picker) is installed only when the API is missing or refuses, and behaves like the API it stands for (fake globals, tests/compat_node.mjs)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
def test_compat_fallbacks():
    r = subprocess.run(["node", str(ROOT / "tests" / "compat_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    o = json.loads(r.stdout)
    assert len(o) == 11, o
    bad = {k: v for k, v in o.items() if v is not True}
    assert not bad, bad
