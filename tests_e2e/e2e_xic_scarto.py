"""«Extract ions (XIC)» window: a signed number after the m/z or the formula (288.2+16, C9H10Cl2N2O+H-18) is added to the m/z after the adduct; the resulting m/z is shown as a candidate; a malformed shift keeps a clear message."""
# covers: mzlab/web/xic-nomi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8848, wd="/tmp/wd_xicscarto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0")])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("openXic(null)"); wait_for_js(pg, "document.querySelector('#xicdlg').open && document.querySelectorAll('#xic-rows .xrw').length === 2")
        sumtxt = lambda: pg.evaluate("document.querySelector('#xic-rows .xr-sum').textContent")
        def number():
            pg.fill("#xic-mz", "288.2+16"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('304.2')")
            assert pg.evaluate("document.querySelector('#xic-rows .xrw select').hidden"), "no adduct applies to a bare m/z"
            pg.fill("#xic-mz", "288.2-18"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('270.2')")
        step("a number with a signed shift gives the shifted m/z as a candidate", number)
        def formula():
            pg.fill("#xic-mz", "C9H10Cl2N2O"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('m/z')")
            base = sumtxt()
            pg.fill("#xic-mz", "C9H10Cl2N2O+16"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('+16')")
            assert sumtxt() != base and "16" in sumtxt(), sumtxt()
        step("a formula takes the shift after the adduct", formula)
        def bad():
            pg.fill("#xic-mz", "288.2+1.2.3"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum .fail')")
            assert "+1.2.3" in sumtxt(), sumtxt()
            pg.click("#xic-go"); wait_for_js(pg, "document.querySelector('#xic-err').textContent.length > 0")
        step("a malformed shift keeps a clear message and blocks the extraction", bad)
        def extract():
            pg.fill("#xic-mz", "288.2+16"); wait_for_js(pg, "document.querySelector('#xic-rows .xr-sum').textContent.includes('304.2')")
            pg.click("#xic-go"); wait_for_js(pg, "E.panels.some(p => p.type === 'xic' && p.traces.some(t => /304\\.3/.test(t.label)))")      # the low-resolution unit window of 304.2 is [303.8, 304.8]: the trace is named by its centre
        step("the XIC is made on the shifted m/z", extract)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
