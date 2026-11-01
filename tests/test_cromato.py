"""Chromatographic figures (mzlab/web/cromato.js) on synthetic peaks with known width, plates, tailing and noise."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "cromato.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const c=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_gaussian_peak():
    r = run("""const xs=[],ys=[];for(let t=0;t<=10;t+=0.005){xs.push(t);ys.push(1000*Math.exp(-0.5*((t-5)/0.1)**2)+50)}
      console.log(JSON.stringify(c.chromPeak(xs,ys,4,6)))""")
    w = 2.3548 * 0.1
    assert abs(r["w50"] - w) < 0.003 and abs(r["rt"] - 5) < 0.006
    assert abs(r["N"] - 5.54 * (5 / w) ** 2) / r["N"] < 0.03
    assert abs(r["tailing"] - 1.0) < 0.03 and abs(r["height"] - 1000) < 5


def test_tailing_is_known():
    r = run("""const xs=[],ys=[];for(let t=0;t<=10;t+=0.002){xs.push(t);const s=t<5?0.05:0.10;ys.push(1000*Math.exp(-0.5*((t-5)/s)**2))}
      console.log(JSON.stringify(c.chromPeak(xs,ys,4,6.5)))""")
    assert abs(r["tailing"] - 1.5) < 0.04                          # back sigma twice the front one: (f + b) / 2f = 1.5


def test_noise_ignores_drift():
    r = run("""let s=12345;const rnd=()=>{s=(s*1103515245+12345)%2147483648;return s/2147483648};
      const xs=[],ys=[];for(let t=0;t<=2;t+=0.01){xs.push(t);ys.push(100+30*t+(rnd()-0.5)*20*Math.sqrt(12)/Math.sqrt(12)*2)}
      console.log(JSON.stringify({sd:c.noiseSD(xs,ys,0,2),few:c.noiseSD([1,2],[1,2],0,3)}))""")
    assert 9.5 < r["sd"] < 13.5 and r["few"] is None              # uniform noise of width 20 -> SD = 20/sqrt(12) = 5.8 x ... drift removed


PICCHI = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "picchi.js"


def run_integ(code):
    """integ() lives in picchi.js, a classic script of the page: cut its text out and run it with nearIdx of numeri.js."""
    src = PICCHI.read_text(encoding="utf-8")
    fn = src[src.index("function integ("):src.index("// Parameters of the automatic integration")]
    num = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "numeri.js"
    out = subprocess.run(["node", "-e", f"const {{nearIdx}}=require({str(num)!r});{fn}\n{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_integ_area_of_a_gaussian_is_known():
    """height 1000 counts, sigma 0.1 min = 6 s -> area = H * sigma * sqrt(2 pi) = 15039.8 counts*s; a flat or a sloped baseline is taken away"""
    r = run_integ("""const xs=[],flat=[],slope=[];for(let t=0;t<=10;t+=0.005){xs.push(t);const g=1000*Math.exp(-0.5*((t-5)/0.1)**2);flat.push(g+50);slope.push(g+50+20*t)}
      console.log(JSON.stringify({flat:integ({x:xs,y:flat},4,6),slope:integ({x:xs,y:slope},4,6)}))""")
    want = 1000 * 6 * (2 * 3.141592653589793) ** 0.5
    for k in ("flat", "slope"):
        assert abs(r[k]["area"] - want) / want < 0.002, (k, r[k], want)
        assert abs(r[k]["height"] - 1000) < 12 and abs(r[k]["rt"] - 5) < 0.006, (k, r[k])
