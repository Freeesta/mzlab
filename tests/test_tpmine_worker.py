"""The worker of mzFinder (TP_Mine/files/tpmine-worker.js) answers default_transformations without Pyodide: its copy of the text is the Python one."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "TP_Mine" / "py"))
from tpmine import chem  # noqa: E402


def test_default_transformations_match_python():
    js = (ROOT / "TP_Mine" / "files" / "tpmine-worker.js").read_text(encoding="utf-8")
    m = re.search(r"const DEFAULT_TR = `(.*?)`;", js, re.S)
    assert m and m.group(1) == chem.transformations_text()
