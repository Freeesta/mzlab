"""«Extract ions (XIC)» window: a contaminant name of the lists (with suggestions, keyboard) fills the ion field, and a signed shift after the name (dibutilftalato+16) is added to the m/z after the adduct; a plain formula and an m/z still work. Synthetic data only."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
D = synth("xicnomi")
r = Run(port=8849, wd="/tmp/wd_xicnomi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        pg.evaluate("openXic(null)"); wait_for_js(pg, "document.querySelector('#xicdlg').open && document.querySelectorAll('#xic-rows .xrw').length === 2")
        sumtxt = lambda: pg.evaluate("document.querySelector('#xic-rows .xr-sum').textContent")
        def suggest():
            pg.click("#xic-mz"); pg.fill("#xic-mz", "dibutilftalato"); pg.wait_for_selector("#asksug li")
            assert any("C16H22O4" in o for o in pg.eval_on_selector_all("#asksug li", "e=>e.map(x=>x.textContent)"))
            pg.press("#xic-mz", "ArrowDown"); pg.press("#xic-mz", "Enter")
            assert pg.input_value("#xic-mz").startswith("C16H22O4"), pg.input_value("#xic-mz")
            assert pg.evaluate("document.querySelector('#xicdlg').open")
        step("a name is suggested and a pick fills the formula (Enter does not extract)", suggest)
        def shifted_name():
            pg.select_option("#xic-ad", "[M+H]+"); pg.fill("#xic-mz", "dibutilftalato+16")
            wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('295.159')")
        step("a name with a shift: formula, adduct, then +16 Da (279.159 -> 295.159)", shifted_name)
        def plain():
            pg.fill("#xic-mz", "C16H22O4+16"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('295.159')")
            pg.fill("#xic-mz", "no-such-name+16"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum .fail')")
        step("a formula with a shift gives the same m/z; an unknown name is refused", plain)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
