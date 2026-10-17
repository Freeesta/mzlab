"""The worker of mzFinder (TP_Mine/files/tpmine-worker.js): it answers default_transformations with a copy of the frozen text (tests/golden/finder_transf_default.txt), and every function the page calls has its own branch (Rust), so none is left to the Pyodide fallback."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

WORKER = ROOT / "TP_Mine" / "files" / "tpmine-worker.js"


def test_default_transformations_match_the_frozen_text():
    js = WORKER.read_text(encoding="utf-8")
    m = re.search(r"const DEFAULT_TR = `(.*?)`;", js, re.S)
    assert m and m.group(1) == (ROOT / "tests" / "golden" / "finder_transf_default.txt").read_text(encoding="utf-8")


def test_every_function_the_page_calls_has_a_rust_branch_in_the_worker():
    """The functions the page asks of the worker (`call("name"`, not `LIB.call`) must be named in the worker beside the engines, never reach a missing branch."""
    js = WORKER.read_text(encoding="utf-8")
    called = set()
    for p in (ROOT / "TP_Mine" / "js").glob("*.js"):
        called |= set(re.findall(r"(?<![.\w])call\(\s*\"([a-z_0-9]+)\"", p.read_text(encoding="utf-8")))
    assert {"classify", "detail", "tp_row", "tp_table", "transf_rows", "formula_info", "frag_tree"} <= called
    missing = sorted(f for f in called if f'"{f}"' not in js)
    assert not missing, f"the page calls functions the worker only serves with Pyodide: {missing}"


def test_the_worker_has_no_python_left():
    js = WORKER.read_text(encoding="utf-8").lower()
    assert "pyodide" not in js and "topy" not in js
