"""Arithmetic of the header calculator (mzlab/web/calcola.js): expression or formula, Italian signs and commas, results with 4 decimals."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "calcola.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const c=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_expressions_are_recognised_and_formulas_are_not():
    r = run("console.log(JSON.stringify(['364.4-194.2','364,4 − 194,2','(229.1-171.2)*2','305+18','5x3','8 ÷ 2','Ans+1','1,5','',' ','C9H10Cl2N2O','c9h10n2o','CO','Xe','H2O','2','(','+','×',')',',','Ans'].map(c.calcIsExpr)))")
    assert r == [True, True, True, True, True, True, True, True, False, False, False, False, False, False, False, True] + [True] * 6          # an unfinished expression is still an expression: the keypad stays


def test_values():
    cases = {"364.4-194.2": 170.2, "364,4 − 194,2": 170.2, "(229.1-171.2)*2": 115.8, "305+18": 323, "5x3": 15, "8 ÷ 2": 4, "2+3*4": 14, "-3+5": 2, "10/4": 2.5, "1,5+1,5": 3}
    r = run(f"console.log(JSON.stringify(Object.entries({json.dumps(cases)}).map(([k,v])=>[c.calcFmt(c.calcEval(k).v),v])))")
    assert all(float(a) == pytest.approx(b) for a, b in r), r
    assert run("console.log(JSON.stringify(c.calcFmt(c.calcEval('364.4-194.2').v)))") == "170.2"          # not 170.20000000000002
    assert run("console.log(JSON.stringify([c.calcFmt(1/3),c.calcFmt(-0.00001),c.calcFmt(1234567.5),c.calcFmt(2)]))") == ["0.3333", "0", "1234567.5", "2"]


def test_ans_and_errors():
    assert run("console.log(JSON.stringify(c.calcEval('Ans*2', 170.2)))") == {"v": 340.4}
    assert run("console.log(JSON.stringify(c.calcEval('1/0')))") == {"err": "divisione per zero"}
    assert run("console.log(JSON.stringify([c.calcEval('364.4-'),c.calcEval('(1+2'),c.calcEval('1+2)')]))") == [{"err": ""}, {"err": ""}, {"err": "parentesi chiusa di troppo"}]
