import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import controlla  # noqa: E402

def test_warns_when_close_to_tolerance():
    base = {"file": {"a.js": 100}, "funzioni": {}}
    out = controlla.vicini_tolleranza({"file": {"a.js": 105}, "funzioni": {}}, base)
    assert out == ["a.js is 5 lines from its tolerance (base 100, max 110): split it before adding more"]


def test_silent_when_far_or_unknown_or_over():
    base = {"file": {"a.js": 200}, "funzioni": {"b.js::f": 100}}
    now = {"file": {"a.js": 200, "new.js": 600}, "funzioni": {"b.js::f": 80}}
    assert controlla.vicini_tolleranza(now, base) == []
    assert controlla.vicini_tolleranza({"file": {"a.js": 221}, "funzioni": {}}, base) == []


def test_functions_and_boundary():
    base = {"file": {}, "funzioni": {"b.js::f": 100}}
    assert controlla.vicini_tolleranza({"file": {}, "funzioni": {"b.js::f": 90}}, base) == []
    assert len(controlla.vicini_tolleranza({"file": {}, "funzioni": {"b.js::f": 91}}, base)) == 1
