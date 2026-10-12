"""The site of the students carries no Python: no `pyodide*`, no zip with `.py` outside static/mzfinder/ (the hidden mzFinder keeps its own Pyodide there)."""
import importlib.util
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("build_site", ROOT / "tools" / "build_site.py")
build_site = importlib.util.module_from_spec(spec)
sys.path.insert(0, str(ROOT / "tools"))
spec.loader.exec_module(build_site)


def test_python_files_finds_what_must_not_be_there(tmp_path):
    s = tmp_path / "static"
    (s / "mzfinder" / "pyodide").mkdir(parents=True)
    (s / "mzfinder" / "pyodide" / "pyodide.mjs").write_text("x")
    with zipfile.ZipFile(s / "mzfinder" / "mzlab.zip", "w") as z:
        z.writestr("mzlab/app.py", "x = 1")
    assert build_site.python_files(tmp_path) == []                      # mzFinder's folder is the one place
    (s / "pyodide").mkdir()
    (s / "pyodide" / "pyodide.asm.wasm").write_text("x")
    with zipfile.ZipFile(s / "mzlab.zip", "w") as z:
        z.writestr("mzlab/app.py", "x = 1")
    with zipfile.ZipFile(s / "data.zip", "w") as z:
        z.writestr("a.json", "{}")                                      # a zip without Python is fine
    assert build_site.python_files(tmp_path) == ["static/mzlab.zip", "static/pyodide", "static/pyodide/pyodide.asm.wasm"]


def test_the_built_site_has_no_python(tmp_path):
    out = tmp_path / "site"
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--no-pyodide", "--out", str(out)], check=True, capture_output=True)
    assert build_site.python_files(out) == []
    assert not (out / "static" / "browser-worker.js").exists() and not (out / "static" / "mzlab.zip").exists()
    assert not list(out.rglob("*.py"))
