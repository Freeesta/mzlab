"""Order of the files and unit of the times (mzlab/web/ordine.js, run by node) and the time read from a name (mzlab/project.py)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

from mzlab.project import guess_sample

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "ordine.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const o=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def order(files):
    return run(f"console.log(JSON.stringify(o.orderFiles({json.dumps(files)}).map(f=>f.name)))")


def f(name, **kw):
    return {"name": name, "time": None, "conc": None, "cunit": None, **kw}


@needs_node
def test_time_order_mixed_units():
    # times arrive already in minutes (2 h = 120, 1 d = 1440): growing time whatever the input order
    names = [("c_1d", 1440), ("a_2h", 120), ("d_t0", 0), ("b_30min", 30), ("e_none", None)]
    assert order([f(n, time=t) for n, t in names]) == ["d_t0", "b_30min", "a_2h", "c_1d", "e_none"]


@needs_node
def test_time_wins_over_concentration_and_ties_by_name():
    fs = [f("z", time=5, conc=1), f("a", time=5, conc=9), f("m", time=1)]
    assert order(fs) == ["m", "a", "z"]


@needs_node
def test_standards_by_concentration_units_converted():
    fs = [f("s_a", conc=1, cunit="mg/L"), f("s_b", conc=500, cunit="µg/L"), f("s_c", conc=2, cunit="mg/L"), f("s_d", conc=0.4, cunit="g/L")]
    assert order(fs) == ["s_b", "s_a", "s_c", "s_d"]
    assert order([f("p", conc=1, cunit="ppb"), f("q", conc=1, cunit="ppm")]) == ["p", "q"]


@needs_node
def test_natural_alphabetical_and_stable():
    assert order([f("file10"), f("file2"), f("File1"), f("file02")]) == ["File1", "file2", "file02", "file10"]
    assert order([f("b"), f("a"), f("B")]) == ["a", "b", "B"]
    twin = [{**f("same"), "id": 1}, {**f("same"), "id": 2}]
    assert run(f"console.log(JSON.stringify(o.orderFiles({json.dumps(twin)}).map(f=>f.id)))") == [1, 2]
    assert order(list(reversed([f(f"x{i}") for i in range(12)]))) == [f"x{i}" for i in range(12)]


@needs_node
def test_deterministic_whatever_the_input_order():
    fs = [f("t_a", time=10), f("t_b", time=5), f("t_c", time=5), f("t_d")]
    assert order(fs) == order(list(reversed(fs))) == ["t_b", "t_c", "t_a", "t_d"]


@needs_node
def test_time_units_conversion_and_parsing():
    assert run("console.log(JSON.stringify([o.timeShown(90,'h'),o.timeShown(2880,'d'),o.timeShown(30,'min'),o.timeShown(null,'h'),o.timeStored(1.5,'h'),o.timeStored(2,'d')]))") == [1.5, 2, 30, None, 90, 2880]
    assert run("console.log(JSON.stringify(['','12','0,5','1.25',' 3 ','-1','1e3','abc','1,2,3','.5','5.'].map(o.parseDecimal)))") == [None, 12, 0.5, 1.25, 3, None, None, None, None, 0.5, 5]


def test_time_read_from_the_name_with_units_and_raw():
    assert guess_sample("irr_t30.mzML")[1] == 30
    assert guess_sample("irr_2h.mzML")[1] == 120
    assert guess_sample("irr_1d.mzML")[1] == 1440
    assert guess_sample("irr_2giorni.mzML")[1] == 2880
    assert guess_sample("file10.mzML")[1] is None
    # a .raw gets the same reading as the mzML it is converted to
    for stem in ("irr_t30", "campione_2h", "plain"):
        assert guess_sample(stem + ".raw") == guess_sample(stem + ".mzML")


@needs_node
def test_hr_group_table():
    cases = [({"kind": "full"}, "full"), ({"kind": "ms2"}, "ms2"), ({"kind": "full", "dda": True}, "dda"), ({"kind": "ms2", "dda": True}, "dda"),
             ({"kind": "full", "infusion": True}, "infusion"), ({"kind": "ms2", "dda": True, "infusion": True}, "infusion"), ({"kind": "full", "dda": False}, "full")]
    assert run(f"console.log(JSON.stringify({json.dumps([c for c, _ in cases])}.map(o.hrGroup)))") == [g for _, g in cases]
    assert run("console.log(JSON.stringify(['a.mzML','a.mzML#MS2','b#MS1'].map(o.fileBase)))") == ["a.mzML", "a.mzML", "b"]


def test_infusion_like():
    from types import SimpleNamespace as S

    from mzlab.explore import infusion_like
    assert not infusion_like([])
    assert infusion_like([S(rt=0.5)])
    assert infusion_like([S(rt=1.0), S(rt=1.0), S(rt=1.0)])
    assert not infusion_like([S(rt=1.0), S(rt=1.5)])
