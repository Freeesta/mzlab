"""The golden data of the pressure (tests/golden/sintetico_Pressione.json): the units are frozen, a value is never rescaled by a guess as intended."""
import json
from pathlib import Path

G = json.loads((Path(__file__).resolve().parent / "golden" / "sintetico_Pressione.json").read_text(encoding="utf-8"))


def _case(unit, name):
    return next(c for c in G["cases"] if c["unit"] == unit and c["set"] == name)


def test_unit_factors():
    assert _case("bar", "normal")["bar"] == [0.0, 120.5, 250.0, 399.9]
    assert abs(_case("psi", "normal")["bar"][2] - 17.23690) < 1e-9
    assert abs(_case("MPa", "normal")["bar"][2] - 2500.0) < 1e-9 and abs(_case("mbar", "normal")["bar"][2] - 0.25) < 1e-9
    assert all(_case(u, "normal")["bar"] is None for u in (None, "", "unknown", "number of detector counts"))


def test_no_magic_factor_and_edges():
    c = _case("pascal", "x10")
    assert abs(c["bar"][0] - 278.0) < 1e-9 and abs(c["bar"][2] - 2780.0) < 1e-9
    assert _case("pascal", "empty")["bar"] == [] and _case("pascal", "negative")["bar"] is not None


def test_analog_trace():
    a = G["analog"]
    assert a["unit"] == "bar" and "note" not in a and len(a["t"]) == len(a["y"]) <= 400
    assert G["list"][0]["unit"] == "pascal" and G["list"][0]["n"] == 6000
