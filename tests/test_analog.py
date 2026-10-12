"""Analog traces (pump pressure...), the gradient model and the parsers of a gradient (invented texts: no lab data)."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab import analog  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402

SCRIPT = """Run Time: 20.0 [min]
%A_Equate: "Water"
%B_Equate: "Acetonitrile"
  0.000  PumpModule.Pump.%B.Value: 10.0 [%]
         PumpModule.Pump.Flow.Nominal: 0,3 [ml/min]
         Curve: 5
  2.000  PumpModule.Pump.%B.Value: 10.0 [%]
 12.000  PumpModule.Pump.%B.Value: 60.0 [%]
 13.000  PumpModule.Pump.%B.Value: 70.0 [%]
 15.000  PumpModule.Pump.%B.Value: 100.0 [%]
 17.000  PumpModule.Pump.%B.Value: 100.0 [%]
 17.100  PumpModule.Pump.%B.Value: 10.0 [%]
 20.000  PumpModule.Pump.%B.Value: 10.0 [%]
"""


def test_chromeleon_script():
    g = analog.parse_chromeleon(SCRIPT)
    assert [p["b"] for p in g["points"]] == [10, 10, 60, 70, 100, 100, 10, 10]
    assert [p["t"] for p in g["points"]][:3] == [0, 2, 12] and g["run_time"] == 20 and g["points"][0]["flow"] == 0.3
    assert g["solvents"] == {"a": "Water", "b": "Acetonitrile"} and g["source"] == "raw" and not g["nonlinear"]


def test_chromeleon_tolerant():
    broken = SCRIPT.replace("PumpModule", "Mod\x00ule�").replace("Water", "Wa\x00ter").replace(" 12.000", " 12.000 [min]")
    g = analog.parse_chromeleon(broken)
    assert len(g["points"]) == 8
    g2 = analog.parse_chromeleon(SCRIPT.replace("Curve: 5", "Curve: 3"))
    assert g2["nonlinear"] and g2["points"][0]["curve"] == 3
    with pytest.raises(UserError) as e:
        analog.parse_chromeleon("nothing useful here")
    assert e.value.key == "gradient.err.noScript"


def test_csv():
    g = analog.parse_gradient_csv("tempo;%B;flusso\n0;10;0,3\n2;10\n2;20\n12,5;60\n")
    assert [(p["t"], p["b"]) for p in g["points"]] == [(0, 10), (2, 10), (2, 20), (12.5, 60)] and g["points"][0]["flow"] == 0.3
    assert analog.parse_gradient_csv("0,10\n5,50")["points"][1] == {"t": 5.0, "b": 50.0}
    for text, key in (("0;10\n3;120", "gradient.err.bRange"), ("0;10\n5;20\n3;30", "gradient.err.order"), ("0;10", "gradient.err.few"), ("0;10\nx;y", "gradient.err.row"), ("", "gradient.err.empty")):
        with pytest.raises(UserError) as e:
            analog.parse_gradient_csv(text)
        assert e.value.key == key, text


def test_course_and_reading():
    g = analog.course_gradient()
    assert g["source"] == "course" and len(g["points"]) == 8


def test_pressure_unit_rule():
    y, note = analog.pressure_to_bar([2.78e7], "pascal")
    assert abs(y[0] - 278) < 1e-6 and note is None
    y, note = analog.pressure_to_bar([2.78e8], "pascal")
    assert abs(y[0] - 278) < 1e-6 and note["key"] == "analog.unit.tenfold"
    assert abs(analog.pressure_to_bar([4000], "psi")[0][0] - 275.79) < 0.01 and abs(analog.pressure_to_bar([27.8], "MPa")[0][0] - 278) < 1e-6
    assert analog.pressure_to_bar([250], "bar")[1] is None


def test_reduce_keeps_peaks():
    t = np.linspace(0, 10, 100000); y = np.zeros_like(t); y[50000] = 5.0
    rt, ry = analog.reduce_minmax(t, y, 500)
    assert len(rt) <= 1000 and ry.max() == 5.0


@pytest.fixture()
def pressure_app(tmp_path):
    p = tmp_path / "P_t0.mzML"
    ds.full_scan(p, 0, np.random.default_rng(1), pressure=True)
    app = App(); app.workdir = tmp_path
    app.open_session({"samples": [{"file": p.name, "time": 0, "type": "sample"}]})
    return app


def _get(app, path, **q):
    code, _, body, _ = dispatch(app, "GET", path, {k: str(v) for k, v in q.items()})
    return code, json.loads(body)


def test_analog_route(pressure_app):
    tr = pressure_app._item(0).analog_list()
    assert len(tr) == 1 and tr[0]["kind"] == "pressure" and tr[0]["unit"] == "pascal" and tr[0]["n"] == 6000 and tr[0]["t0"] == 0 and abs(tr[0]["t1"] - 15) < 1e-3
    code, a = _get(pressure_app, "/api/analog", k=0, i=tr[0]["i"], n=500)
    assert code == 200 and a["unit"] == "bar" and a["unit_note"]["key"] == "analog.unit.tenfold" and len(a["t"]) <= 1000
    y1 = np.interp(1.0, a["t"], a["y"])
    assert abs(y1 - 278) < 12            # 270 + 8 t + wiggle, min/max envelope
    assert _get(pressure_app, "/api/analog", k=0, i=99)[1]["error_key"] == "err.analog.none"


def test_gradient_route(pressure_app):
    assert _get(pressure_app, "/api/gradient", k=0)[1] == {"gradient": None}
    code, _, body, _ = dispatch(pressure_app, "POST", "/api/gradient", {}, __import__("io").BytesIO(json.dumps({"k": 0, "text": SCRIPT, "fmt": "chromeleon"}).encode()), len(json.dumps({"k": 0, "text": SCRIPT, "fmt": "chromeleon"})))
    assert code == 200 and len(json.loads(body)["gradient"]["points"]) == 8
    g = _get(pressure_app, "/api/gradient", k=0)[1]["gradient"]
    assert g["source"] == "raw"
    payload = json.dumps({"k": 0, "model": None}).encode()
    dispatch(pressure_app, "POST", "/api/gradient", {}, __import__("io").BytesIO(payload), len(payload))
    assert _get(pressure_app, "/api/gradient", k=0)[1] == {"gradient": None}


def test_detector_delay_known():
    rng = np.random.default_rng(1)
    g = lambda t, c, s: np.exp(-0.5 * ((t - c) / s) ** 2)
    ta, tb = np.arange(0, 22, 0.004), np.linspace(0.5, 20, 1111)
    peaks = [(2.1, 6), (7.07, 3), (13.0, 5), (14.3, 22), (18.3, 4)]
    for true in (0.0, 0.07, 0.25, -0.1):
        ya = 2000 + 30 * rng.normal(size=ta.size) + sum(a * 1e3 * g(ta, c, 0.07) for c, a in peaks)
        yb = 4000 + 60 * rng.normal(size=tb.size) + sum(a * 2e3 * g(tb, c + true, 0.06) for c, a in peaks)
        r = analog.detector_delay(ta, ya, tb, yb)
        assert abs(r["delay"] - true) <= 0.002 and r["peaks"] >= 3 and r["corr"] > 0.9, (true, r)
    assert analog.detector_delay(ta[:10], ya[:10], tb, yb) is None


def test_delay_route(pressure_app):
    code, j = _get(pressure_app, "/api/delay", k=0)
    assert code == 200 and j["delay"] is not None and abs(j["delay"]["delay"]) < 0.03 and j["delay"]["corr"] > 0.8, j     # the synthetic PDA and TIC peak together
