"""The undo/redo stack of the Dati view (mzlab/web/storia.js), pure part, in Node."""
import shutil, subprocess, pathlib
import pytest

@pytest.mark.skipif(not shutil.which("node"), reason="node not available")
def test_pila_storia():
    r = subprocess.run(["node", str(pathlib.Path(__file__).parent / "storia_node.js")], capture_output=True, text=True)
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stdout + r.stderr
