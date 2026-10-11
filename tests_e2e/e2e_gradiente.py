"""Gradient and analog traces: the Method window offers the gradient section when the file has none (mzML files never do), the course method and the editor store
it for the file, the file lists its pressure trace; the answers of api/analog and api/gradient."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gradiente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "gradiente_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "GRAD_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3), pressure=True)
r = Run(port=8865, wd="/tmp/wd_gradiente")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def listed():
            a = pg.evaluate("E.files[0].analog"); assert len(a) == 1 and a[0]["kind"] == "pressure" and a[0]["unit"] == "pascal" and a[0]["n"] == 6000, a
            j = pg.evaluate("(async () => await (await fetch('api/analog?k=0&i=%d&n=300')).json())()" % a[0]["i"])
            assert j["unit"] == "bar" and j["unit_note"]["key"] == "analog.unit.tenfold" and abs(np.interp(1.0, j["t"], j["y"]) - 278) < 12, j["unit_note"]
        step("the file lists its pressure trace; the unit is corrected", listed)
        def section():
            pg.click("#np-method"); pg.wait_for_selector("#grad-sec"); 
            assert pg.is_visible("#grad-course") and pg.is_visible("#grad-raw") and pg.is_visible("#grad-csv") and pg.is_visible("#grad-new")
            assert pg.evaluate("(async () => (await GRADIENTE.get(E.files[0])))()") is None
        step("Method window: gradient section, no gradient yet", section)
        def course():
            pg.click("#grad-course"); pg.wait_for_selector("#grad-sec .lct"); ready(pg)
            g = pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()")
            assert g["source"] == "course" and [p["b"] for p in g["points"]] == [10, 10, 60, 70, 100, 100, 10, 10], g
            assert pg.evaluate("Object.keys(NB.gradienti||{}).length") == 1
            assert pg.evaluate("!!localStorage.getItem('qqq.gradiente.ultimo')")
        step("«Metodo del corso» stores the gradient for the file (and the last used)", course)
        def editor():
            pg.click("#grad-rm"); pg.wait_for_selector("#grad-new"); ready(pg)
            assert pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()") is None
            pg.click("#grad-new"); pg.wait_for_selector("#grad-ok")
            pg.fill('#grad-ed input[data-c=t][data-i="0"]', "0"); pg.fill('#grad-ed input[data-c=b][data-i="0"]', "5")
            pg.fill('#grad-ed input[data-c=t][data-i="1"]', "10"); pg.fill('#grad-ed input[data-c=b][data-i="1"]', "95")
            pg.click("#grad-ok"); pg.wait_for_selector("#grad-sec .lct"); ready(pg)
            g = pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()")
            assert g["source"] == "manual" and [(p["t"], p["b"]) for p in g["points"]] == [(0, 5), (10, 95)], g
        step("the table editor stores a typed gradient; «Togli» removes it", editor)
        def paste():
            j = pg.evaluate("(async () => await (await fetch('api/gradient_parse', {method:'POST', body: JSON.stringify({fmt:'csv', text:'0\\t10\\n5,5\\t60\\n'})})).json())()")
            assert j["gradient"]["points"][1] == {"t": 5.5, "b": 60.0} or j["gradient"]["points"][1]["t"] in (5, 5.5), j
        step("pasted text from Excel becomes a gradient", paste)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
