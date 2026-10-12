"""The golden data of the pressure (tests/golden/sintetico_Pressione.json): the units and the tenfold rule are frozen as intended."""
import json
from pathlib import Path

G = json.loads((Path(__file__).resolve().parent / "golden" / "sintetico_Pressione.json").read_text(encoding="utf-8"))


def _case(unit, name):
    return next(c for c in G["cases"] if c["unit"] == unit and c["set"] == name)


def test_unit_factors():
    assert _case("bar", "normal")["bar"] == [0.0, 120.5, 250.0, 399.9] and _case("bar", "normal")["note"] is None
    assert abs(_case("psi", "normal")["bar"][2] - 17.23690) < 1e-9
    assert _case("MPa", "normal")["note"] == "analog.unit.tenfold" and abs(_case("MPa", "normal")["bar"][2] - 250.0) < 1e-9
    assert _case(None, "normal")["bar"] == _case("", "normal")["bar"] == _case("unknown", "normal")["bar"]


def test_tenfold_and_edges():
    c = _case("pascal", "x10")
    assert c["note"] == "analog.unit.tenfold" and abs(c["bar"][0] - 27.8) < 1e-9
    assert _case("pascal", "empty")["bar"] == [] and _case("pascal", "negative")["note"] is None


def test_analog_trace():
    a = G["analog"]
    assert a["unit"] == "bar" and a["note"] == "analog.unit.tenfold" and len(a["t"]) == len(a["y"]) <= 400
    assert G["list"][0]["unit"] == "pascal" and G["list"][0]["n"] == 6000
