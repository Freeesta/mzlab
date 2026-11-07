"""Every use of a web API, JS feature or CSS newer than the Safari floor (16.4) in the interface is listed with its guard in
tools/safari_api_ammesse.txt (tools/safari_api.py); the scanner itself finds what it should and skips comments and divisions."""
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("safari_api", ROOT / "tools" / "safari_api.py")
SA = importlib.util.module_from_spec(spec)
spec.loader.exec_module(SA)


def test_every_newer_api_is_covered():
    assert SA.controlla() == []


def _ids(line: str, table) -> set[str]:
    import re
    return {sid for sid, _v, _d, rx in table if re.search(rx, line)}


def test_patterns_find_and_skip():
    assert "Object.groupBy" in _ids("const g = Object.groupBy(a, f);", SA.JS)
    assert "regex v flag" in _ids('if (/[\\p{L}--[a-z]]/v.test(s)) x();', SA.JS)
    assert "regex v flag" in _ids('const r = new RegExp(src, "v");', SA.JS)
    assert "regex v flag" not in _ids("const w = a / b / v;", SA.JS)
    assert "Blob.bytes" not in _ids("return { bytes: out.bytes() };", SA.JS)
    assert "CSS nesting" in _ids(".a{color:red;&:hover{color:blue}}", SA.CSS)
    assert "CSS nesting" not in _ids("<b>x</b> &#9662;</button>", SA.CSS)
    assert "backdrop-filter" in _ids(".m{backdrop-filter:blur(4px)}", SA.CSS)
    assert "backdrop-filter" not in _ids(".m{-webkit-backdrop-filter:blur(4px)}", SA.CSS)
