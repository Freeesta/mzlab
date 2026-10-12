"""Reference of the Python engine for the parity test of the Rust port of the method (`.dam`) reader, the gradient parsers and the analog traces
(crates/mzlab-core/tests/dam_parity.rs).

    python3 tools/dam_riferimento.py OUT

Writes synthetic `.dam` files (tools/cfb_scrivi.py) into OUT/ and OUT/riferimento.json: for each file `read_methods`, the one-line summary, `method_kind`, the gradient
and `check_against` for a few `Item.method()` dictionaries; the gradient parsers on a list of texts; pressure units, reduction and `detector_delay` on synthetic traces.
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import cfb_scrivi  # noqa: E402
from mzlab import analog  # noqa: E402
from mzlab.app import _method_summary  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402
from mzlab.reader.methodinfo import check_against, method_kind, read_methods  # noqa: E402


def u16(s: str) -> bytes:
    return s.encode("utf-16le")


def f32(v: float) -> bytes:
    return struct.pack("<f", v)


def mrm_stream(rows) -> bytes:
    """MassRangeEx of an MRM experiment: Q1, 0, Q3, dwell, 0, length, 0, name, then DP and CE (each as the compound parameters Analyst writes)."""
    d = bytes(40)
    for q1, q3, dwell, name, dp, ce in rows:
        n = u16(name)
        d += f32(q1) + bytes(4) + f32(q3) + f32(dwell) + bytes(4) + bytes([len(n)]) + b"\x00" + n + b"\x04\x00D\x00P\x00" + f32(dp) + b"\x04\x00C\x00E\x00" + f32(ce) + bytes(6)
    return d


def scan_stream(lo: float, hi: float, ces=()) -> bytes:
    d = bytearray(432)
    d[40:44] = f32(lo)
    d[240:244] = f32(hi)
    for i, ce in enumerate(ces):
        at = 100 + i * 20
        d[at:at + 6] = b"\x04\x00C\x00E\x00"
        d[at + 6:at + 10] = f32(ce)
    return bytes(d)


def source_stream(name: str, v: float) -> bytes:
    return bytes(16) + u16(name) + f32(v)


def lc_xml(extra_events="") -> bytes:
    def par(n, v, u=""):
        return f"<Parameter><Name>{n}</Name><Value>{v}</Value><Unit>{u}</Unit></Parameter>"

    def row(*f):
        return "<row>" + "".join(f"<Field{i}>{x}</Field{i}>" for i, x in enumerate(f, 1)) + "</row>"

    prog = row("Time", "Module", "Device", "Event", "Value") + row("----", "----", "----", "----", "----")
    for t, ev, v in (("0.00", "Total Flow", "0.3"), ("2.00", "Pump B Conc.", "10"), ("12.00", "Pump B Conc.", "60"), ("15.00", "Pump B Conc.", "100"),
                     ("17.00", "Pump B Conc.", "100"), ("17.10", "Pump B Conc.", "10"), ("20.00", "Stop", "")):
        prog += row(t, "Pump", "dev", ev, v)
    prog += extra_events
    return ("junk<SCIEX_ANALYST_ACQUISITION_METHOD><Categories>"
            f"<Category><Title>Pumps</Title>{par('Total Flow', '0.3', 'mL/min')}{par('B Concentration', '10', '%')}</Category>"
            f"<Category><Title>System Controller</Title><Table><Name>Time Program</Name>{prog}</Table></Category>"
            f"<Category><Title>PDA Detector</Title>{par('Start WL', '200')}{par('Stop WL', '400')}{par('Wave Step', '2')}{par('Sampling Frequency', '5')}"
            f"{par('Lamp', 'D2')}{par('Cell Temperature', '30')}{par('Slit Width', '4')}{par('Run Time', '19.5')}"
            "<Table><Name>DA Channel Array</Name>" + row("Ch", "Bw") + row("--", "--") + row("254", "8") + row("280", "4") + "</Table></Category>"
            f"<Category><Title>Oven</Title>{par('Temperature', '40')}</Category>"
            "</Categories></SCIEX_ANALYST_ACQUISITION_METHOD>tail").encode()


def file_stream() -> bytes:
    parts = [r"D:\Analyst Data\Projects\Lab\Acquisition Methods\MRM_prova.dam", "Tuesday, March 11, 2025 10:22:33 AM", "Analyst 1.6.3 (build 4)", "LAB_PC1"]
    return b"\x01\x02".join(u16(p) for p in parts) + b"\x00\x00\x01"


def build(out: Path) -> list:
    base = "DeviceMethod0/Period0/Experiment"
    ion = {f"{base}0/IonSourceParamsTable/Parameter{i}/ParameterData": source_stream(n, v) for i, (n, v) in enumerate((("CUR", 20.0), ("GS1", 40.5), ("GS2", 50.0), ("CAD", 5.0), ("IS", 5500.0), ("TEM", 450.0), ("ihe", 1.0), ("XYZ", 3.0)))}
    mrm = [(300.2, 150.1, 100.0, "T1 quant", 40.0, 25.0), (300.2, 120.0, 150.0, "T1 conf", 40.0, 30.0), (255.05, 90.3, 50.0, "IS", 55.5, 35.5)]
    files = {
        "MRM prova.dam": {**ion, f"{base}0/MassRangeEx/MassRangeEx": mrm_stream(mrm), "DeviceMethod1/VendorAppMethod": lc_xml(), "AcqMethodFileInfoStm": file_stream()},
        "Q1 full scan.dam": {f"{base}0/MassRangeEx/MassRangeEx": scan_stream(50, 600), **ion},
        "MS2 EPI.dam": {f"{base}0/MassRangeEx/MassRangeEx": scan_stream(100, 400, (20.0, 35.0)), f"{base}1/MassRangeEx/MassRangeEx": scan_stream(100, 300, (35.0,))},
        "two scans.dam": {f"{base}0/MassRangeEx/MassRangeEx": scan_stream(50, 600), f"{base}1/MassRangeEx/MassRangeEx": scan_stream(60, 500), "FileRec_Str": file_stream()},
        "no range.dam": {f"{base}0/MassRangeEx/MassRangeEx": bytes(100), "x/y": b"abc"},
        "big stream.dam": {"AcqMethodFileInfoStm": file_stream(), "pad/data": bytes(range(256)) * 30, f"{base}0/MassRangeEx/MassRangeEx": mrm_stream(mrm[:1])},
    }
    names = []
    for n, streams in files.items():
        cfb_scrivi.write(streams, out / n)
        names.append(n)
    (out / "garbage.dam").write_bytes(b"this is not an OLE file at all" * 20)
    return names + ["garbage.dam"]


DATA = [
    {"kind": "mrm", "transitions": [{"q1": 300.2, "q3": 150.1, "ce": 25.0, "dwell": 100.0}, {"q1": 300.2, "q3": 120.0, "ce": 30.0, "dwell": 0.15}, {"q1": 255.05, "q3": 90.3, "ce": 35.5, "dwell": 50}],
     "rt_max": 19.4, "pda": True},
    {"kind": "mrm", "transitions": [{"q1": 300.2, "q3": 150.1, "ce": 20.0, "dwell": 100.0}, {"q1": 301.0, "q3": 99.0, "ce": 20.0, "dwell": 100.0}], "rt_max": 25.0, "pda": False},
    {"kind": "full", "scan_window": [50.0, 600.0], "rt_max": 19.9, "pda": True},
    {"kind": "full", "scan_window": [100.0, 400.0], "rt_max": 5.0, "pda": False},
    {"kind": "full", "scan_window": None},
    {"kind": "ms2", "scan_window": [100.0, 400.0], "ce": [35.0, 20.0], "precursors": [301.1, 255.0], "rt_max": 10.0, "pda": False},
    {"kind": "ms2", "scan_window": [100.0, 300.0], "ce": [35.0], "precursors": [301.1], "rt_max": 10.0, "pda": True},
    {"kind": "empty"},
]

CHROMELEON = [
    "0.000 PumpModule.Pump.%B.Value: 10.0 [%]\nPumpModule.Pump.Flow.Nominal: 0.300 [ml/min]\n2.000 %B.Value = 10\n12,000 PumpModule.Pump.%B.Value: 60.0 [%]\nCurve: 5\n"
    "15.0 min %B: 100\n17.0 %B.Value: 100\n17.100 %B.Value: 10\n20.000 %B.Value: 10\n20.0 Run Time: 20.0\n%A.Equate = \"Water 0.1% FA\"\n%B_Equate: \"Acetonitrile; x\"\n",
    "Initial\nPump.%B.Value: 5\nFlow: 0.4\n1.0 Pump.%B.Value: 5\n10.0 Pump.%B.Value: 95 Curve: 3\n12.0 Pump.%B.Value: 95\n12.1 Pump.%B.Value: 5\n",
    "PumpModule.Pump.%B.Value = 20\n5.0 [min] PumpModule.Pump.Flow.Nominal: 0.2\n8.0 min\n%B.Value: 80\nRun Time: 12.5\n",
    "nothing here\n",
    "1.0 %B.Value: 10\n",
    "0 Pump.%B.Value: 10\x00\x01\x7f\ufffd\r\n3 Pump.%B.Value: 50\r\n",
    "0.5 %B.Value: 10\n0.5 Flow: 0.5\n4,5 %B.Value:55,5\n4.5 Curve: 7\n",
]
CSV = [
    "t;B\n0;10\n2;10\n12;60\n13;70\n15;100\n17;100\n17,1;10\n20;10\n",
    "time,%B,flow\n0,10,0.3\n5,50,0.3\n10,90\n",
    "0\t5\n10\t95\n",
    "",
    "0;10\n5;120\n",
    "0;10\n5;50\n3;60\n",
    "0;10\nabc;def\n",
    "0;10\n",
    "# comment\n0;10\n6;60;0,25\n6;90;0.5\n",
]
MODELS = [
    {"source": "manual", "points": [{"t": 0, "b": 5}, {"t": "10", "b": "95", "flow": "0,3", "curve": "3"}, {"t": 12, "b": 95, "curve": 5}], "run_time": 14, "solvents": {"a": "water", "b": "ACN"}},
    {"points": [{"t": 0, "b": 5}, {"t": 5, "b": 5.123456}]},
    {"points": [{"t": 0, "b": 5}]},
    {"points": [{"t": 1, "b": 5}, {"t": 0, "b": 5}]},
    {"points": [{"t": 0, "b": 105}, {"t": 5, "b": 5}]},
    {"points": [{"t": 0, "b": None}, {"t": 5, "b": 5}]},
    {"points": [{"t": 0, "b": 5, "flow": ""}, {"t": 5, "b": 5, "curve": ""}], "run_time": None},
]


class _Item:
    def __init__(self, data, file):
        self.data, self.file, self.path = data, file, Path(file.split("#")[0])

    def method(self):
        return dict(self.data)

    def kind(self):
        return self.data.get("kind")


def _fake_app(labs, items):
    from mzlab.app import App

    class Fake(App):
        def _item(self, k):
            return items[k]

        def lab_methods(self, paths=None):
            return labs

    a = Fake.__new__(Fake)
    a.session = type("S", (), {"items": items})()
    return a


def method_cases(out: Path, names) -> list:
    """`App.method` and `App.method_warnings` with some of the methods loaded and the file called differently."""
    labs = {}
    for n in names:
        try:
            m = read_methods(out / n)[0]
            m["name"] = n
        except Exception as e:  # noqa: BLE001
            m = {"name": n, "error": f"cannot read this file ({e})", "error_key": "err.dam.unreadable", "params": {"message": str(e)}, "source": [], "compound": [], "file": {}}
        labs[n] = m
    sets = [names, names[:1], names[1:3], [names[-1]], [names[2], names[1]], []]
    cases = []
    for i, d in enumerate(DATA):
        data = {"file": "x.mzML", "polarity": "positive", "scans": 10, **d}
        for j, ls in enumerate(sets):
            for fn in ("sample.mzML", "MRM prova.mzML", "Q1 full scan.mzML#MS1", "ms2 epi.mzML"):
                it = _Item(data, fn)
                app = _fake_app([labs[n] for n in ls], [it])
                cases.append({"data": data, "file": fn, "labs": ls, "result": app.method(0)})
    for have in (["full"], ["mrm"], ["ms2"], ["full", "ms2"], ["mrm", "full", "ms2"], ["empty"]):
        its = [_Item({"kind": k}, "a.mzML") for k in have]
        app = _fake_app([labs[n] for n in names], its)
        cases.append({"warn": have, "result": app.method_warnings()})
    return cases


def grad(fn, *a):
    try:
        return {"ok": fn(*a)}
    except UserError as e:
        return {"error_key": e.key, "params": {k: str(v) for k, v in e.params.items()}}


def traces():
    rs = np.random.RandomState(7)

    def peaks(t, centres, shift):
        y = 0.02 * t + 5
        for c, h, w in centres:
            y = y + h * np.exp(-0.5 * ((t - c - shift) / w) ** 2)
        return y + rs.normal(0, 0.05, len(t))

    cs = [(3.0, 100.0, 0.05), (5.5, 40.0, 0.08), (8.2, 250.0, 0.06), (11.0, 60.0, 0.07), (14.5, 180.0, 0.05), (16.0, 30.0, 0.09)]
    ta = np.arange(0, 20, 0.0125)
    tb = np.arange(0.01, 20, 0.35 / 60)
    out = []
    for shift in (0.0, 0.12, -0.2):
        out.append({"ta": ta, "ya": peaks(ta, cs, 0.0), "tb": tb, "yb": peaks(tb, cs, shift) * 1000})
    out.append({"ta": ta[:10], "ya": ta[:10], "tb": tb, "yb": tb})
    out.append({"ta": ta[:500], "ya": peaks(ta[:500], cs, 0), "tb": tb[:500], "yb": peaks(tb[:500], cs, 0.05)})
    out.append({"ta": ta, "ya": np.full(len(ta), 3.0), "tb": tb, "yb": np.full(len(tb), 7.0)})
    return out


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    names = build(out)
    ref = {"files": [], "gradients": [], "analog": {}}
    for n in names:
        entry = {"file": n}
        try:
            m = read_methods(out / n)[0]
            m["name"] = n
            entry |= {"method": m, "kind": method_kind(m), "summary": _method_summary(m)}
            g = grad(analog.from_dam, m.get("lc"))
            entry["gradient"] = g
            entry["checks"] = [check_against(d, m) for d in DATA]
        except Exception as e:  # noqa: BLE001 -- an unreadable file is part of the reference
            entry |= {"error": str(e), "kind": method_kind({"name": n}), "summary": _method_summary({"error": "x"})}
        ref["files"].append(entry)
    ref["data"] = DATA
    ref["methods"] = method_cases(out, names)
    for fmt, texts, fn in (("chromeleon", CHROMELEON, analog.parse_chromeleon), ("csv", CSV, analog.parse_gradient_csv)):
        for t in texts:
            ref["gradients"].append({"fmt": fmt, "text": t, "result": grad(fn, t)})
    for m in MODELS:
        ref["gradients"].append({"fmt": "model", "model": m, "result": grad(lambda m: analog.gradient_model(m.get("source", "manual"), m.get("points", []), m.get("run_time"), m.get("solvents")), m)})
    ref["gradients"].append({"fmt": "course", "result": grad(analog.course_gradient)})
    rs = np.random.RandomState(3)
    ref["analog"]["pressure"] = [{"y": list(map(float, y)), "unit": u, "result": [list(map(float, analog.pressure_to_bar(y, u)[0])), analog.pressure_to_bar(y, u)[1]]}
                                 for y, u in ((rs.uniform(0, 300, 20), "bar"), (rs.uniform(0, 30, 20), "megapascal"), (rs.uniform(1e5, 3e7, 20), "pascal"), (rs.uniform(1e6, 9e8, 20), "Pascal"),
                                              (rs.uniform(0, 4000, 20), "psi"), (rs.uniform(0, 100, 20), ""), (rs.uniform(0, 100, 20), "kPa [x]"))]
    red = []
    for n_pts, n in ((100, 10), (1000, 50), (4001, 100), (50, 100), (10000, 1), (777, 13)):
        t = np.sort(rs.uniform(0, 30, n_pts))
        y = rs.normal(0, 1, n_pts)
        tt, yy = analog.reduce_minmax(t, y, n)
        red.append({"t": list(map(float, t)), "y": list(map(float, y)), "n": n, "result": [list(map(float, tt)), list(map(float, yy))]})
    ref["analog"]["reduce"] = red
    ref["analog"]["delay"] = [{"ta": list(map(float, c["ta"])), "ya": list(map(float, c["ya"])), "tb": list(map(float, c["tb"])), "yb": list(map(float, c["yb"])),
                               "result": analog.detector_delay(c["ta"], c["ya"], c["tb"], c["yb"])} for c in traces()]
    (out / "riferimento.json").write_text(json.dumps(ref), encoding="utf-8")
    print("\n".join(str(out / n) for n in names))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
