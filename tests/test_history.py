"""The undo/redo stack of the Data view (mzlab/web/history.js), pure part, in Node."""
import shutil, subprocess, pathlib
import pytest

@pytest.mark.skipif(not shutil.which("node"), reason="node not available")
def test_history_stack():
    r = subprocess.run(["node", str(pathlib.Path(__file__).parent / "history_node.js")], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stdout + r.stderr
