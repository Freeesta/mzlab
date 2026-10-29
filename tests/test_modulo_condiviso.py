"""compileOnce shares one compile per URL; qualita.js hands the compiled engine to the peaks worker before any operation (tests/modulo_condiviso_node.mjs)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="module")
def out():
    if shutil.which("node") is None:
        pytest.skip("node is not installed")
    r = subprocess.run(["node", str(ROOT / "tests" / "modulo_condiviso_node.mjs")], capture_output=True, text=True, check=False)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads(r.stdout)


def test_two_callers_compile_once(out):
    assert out["shared"] == {"compiles": 1, "same": True}
    assert out["samePromise"] is True


def test_failed_compile_is_dropped_and_retried(out):
    assert out["retry"]["bad"] is None
    assert out["retry"]["retryIsModule"] is True
    assert out["retry"]["compiles"] == 3  # two attempts that fail, then one compile


def test_qualita_posts_the_module_first_and_does_not_compile_again(out):
    q = out["qualita"]
    assert q["posted"][0] == "module" and q["posted"].count("module") == 1
    assert q["compiles"] == 1
    assert q["answered"]


def test_workers_parse():
    if shutil.which("node") is None:
        pytest.skip("node is not installed")
    for f in ("peaks-worker.js", "esporta-dati-worker.js"):
        r = subprocess.run(["node", "--check", "--input-type=module"], input=(ROOT / "mzlab" / "web" / f).read_text(), capture_output=True, text=True, check=False)
        assert r.returncode == 0, f + r.stderr[-500:]
