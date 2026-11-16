"""Warm-up of the pointed folder, engine side (mzlab/web/rust-scalda.js), run by node on a copy of the module: the copy is hidden by its marker from the moment it is stored."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def test_warm_copy_is_hidden_during_the_index_build(tmp_path):
    (tmp_path / "rust-scalda.js").write_text((ROOT / "mzlab" / "web" / "rust-scalda.js").read_text(encoding="utf-8"), encoding="utf-8")
    (tmp_path / "package.json").write_text('{"type":"module"}')
    r = subprocess.run(["node", str(ROOT / "tests" / "scalda_node.mjs"), str(tmp_path)], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stdout + r.stderr
