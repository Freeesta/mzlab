"""The site carries no Python anywhere, mzFinder's folder included: no `pyodide*`, no zip with `.py`, and the build downloads nothing."""
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
    with zipfile.ZipFile(s / "data.zip", "w") as z:
        z.writestr("a.json", "{}")                                      # a zip without Python is fine
    assert build_site.python_files(tmp_path) == ["static/mzfinder/mzlab.zip", "static/mzfinder/pyodide", "static/mzfinder/pyodide/pyodide.mjs"]


def test_the_built_site_has_no_python_and_needs_no_net(tmp_path):
    out = tmp_path / "site"
    env = {"HTTPS_PROXY": "http://127.0.0.1:9", "HTTP_PROXY": "http://127.0.0.1:9", "PATH": "/usr/bin:/bin"}      # a download would fail
    for flags in ([], ["--no-pyodide"], ["--pyodide-dir", str(tmp_path)]):                  # the flags the workflows pass are accepted and do nothing
        subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), *flags, "--out", str(out)], check=True, capture_output=True, env=env)
        assert build_site.python_files(out) == []
        assert not (out / "static" / "mzfinder" / "tpmine.zip").exists() and not (out / "static" / "mzfinder" / "mzlab.zip").exists()
        assert not (out / "static" / "mzfinder" / "pyodide").exists() and "py" not in __import__("json").loads((out / "static" / "mzfinder" / "indice.json").read_text())
        assert not list(out.rglob("*.py"))
