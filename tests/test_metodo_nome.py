"""A method (.dam) goes with the data file of the same base name, before any guess on the content."""
from types import SimpleNamespace
from pathlib import Path
from mzlab.app import App


def test_method_with_same_base_name_wins(monkeypatch):
    app = App.__new__(App)
    item = SimpleNamespace(path=Path("campione_B.mzML"), method=lambda: {"kind": "mrm"})
    monkeypatch.setattr(App, "_item", lambda self, k: item)
    labs = [{"name": "campione_B.dam"}, {"name": "mrm_generico.dam"}]   # the second one would win by kind
    monkeypatch.setattr(App, "lab_methods", lambda self, paths=None: labs)
    monkeypatch.setattr("mzlab.app.check_against", lambda m, x: [])
    assert app.method(0)["lab_pick"] == 0
    item.path = Path("altro.mzML")
    assert app.method(0)["lab_pick"] == 1
