import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import genera_contaminanti as g  # noqa: E402


def test_static_file_equals_the_route():
    from mzlab.api import dispatch
    from mzlab.app import App
    code, _ctype, body, _h = dispatch(App(), "GET", "/api/contaminants", {})
    assert code == 200
    assert g.OUT.read_text(encoding="utf-8") == body.decode("utf-8") == g.text()
    assert len(json.loads(g.OUT.read_text(encoding="utf-8"))["lists"]) == 1
