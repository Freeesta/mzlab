"""Garbage in the fields of the calculators (campi.js gate, lib_campi.sweep): «calc here», the composition drawer and its full dialog, the KMD form and the
origin of an ion; each field refuses letters, 1e999, empty, out-of-bound numbers, decimals where a whole number is due and a «from» above its «to» with one
note and the old value, no live recompute and no request on garbage; Enter in the KMD form does not submit a refused field; a comma decimal is taken."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_campi import sweep, typed, NOTE
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_calcolatori_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("calcolatori_garbage")
r, r2 = Run(port=8917, wd="/tmp/wd_calc_garbage"), Run(port=8918, wd="/tmp/wd_calc_garbage2")
COMPST = "JSON.stringify(COMP.rd())"
urls = []
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        def drawer_cmp():
            drawer(pg, "cmp"); pg.wait_for_selector("#cx-out table, #cx-out .muted", timeout=20000)
            sweep(pg, "#hri-body", COMPST)
            typed(pg, "#cx-tol", "2,5"); wait_for_js(pg, "COMP.rd().tol === 2.5", 10000)
        step("composition drawer: m/z, max, tolerance, RDB from/to and the element table refuse garbage; «2,5» ppm is taken", drawer_cmp)
        def dialog_cmp():
            pg.click("#cx-exp"); pg.wait_for_selector("#cmp-mz", timeout=10000)
            sweep(pg, "#bigdlg", COMPST)
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("composition dialog: every numeric field refuses garbage", dialog_cmp)
        def kmd():
            drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
            pg.evaluate("(() => { const d = document.querySelector('#sb-panel-stat details.stat-sec[data-s=kmd]'); d.open = true; d.dispatchEvent(new Event('toggle')); })()")
            wait_for_js(pg, "!!document.querySelector('#kmd-tol') && !!document.querySelector('#kmd-tol').offsetParent", 10000)
            sweep(pg, "#sb-panel-stat")
            before = pg.input_value("#kmd-tol")
            x = pg.locator("#kmd-tol"); x.focus(); x.press("ControlOrMeta+a"); x.press_sequentially("abc"); x.press("Enter")
            wait_for_js(pg, f"{NOTE}(document.querySelector('#kmd-tol')) !== ''", 5000)
            assert pg.input_value("#kmd-tol") == before, pg.input_value("#kmd-tol")
        step("KMD: window and members refuse garbage; Enter does not submit a refused field", kmd)
        pg.evaluate("setView('draw')")
        wait_for_js(pg, "!!document.querySelector('#np-calc2') && !!document.querySelector('#np-calc2').offsetParent")
        def calc_here():
            pg.click("#np-calc2"); wait_for_js(pg, "!document.querySelector('#calcdlg').hidden && typeof CALCQUI !== 'undefined' && CALCQUI.calc")
            sweep(pg, "#calcdlg", "JSON.stringify(CALCQUI.st)")
            typed(pg, "#calctol", "0,3"); wait_for_js(pg, "Object.values(CALCQUI.st).includes('0,3')", 5000)
            pg.keyboard.press("Escape")
        step("«calc here»: the tolerance refuses garbage; «0,3» is taken", calc_here)
        def requests():
            bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
            assert not bad, bad[:3]
        step("no request carried NaN, Infinity or undefined", requests)
        pg2 = r2.page(p)
        pg2.on("request", lambda q: urls.append(q.url))
        load(pg2, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg2)
        def origin():
            pg2.evaluate("openOrigin({mz:194.3,k:E.files.find(f=>f.kind==='full').k,rt:14.3})"); wait_for_js(pg2, "document.getElementById('ogdlg').open", 10000)
            typed(pg2, "#og-par", "250,8"); typed(pg2, "#og-r0", "1"); typed(pg2, "#og-r1", "3")
            assert [pg2.input_value(s) for s in ("#og-par", "#og-r0", "#og-r1")] == ["250,8", "1", "3"]
            sweep(pg2, "#ogdlg")
            bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
            assert not bad, bad[:3]
        step("origin of an ion: m/z, parent and RT from/to refuse garbage (a «from» above «to» too); «250,8» is taken", origin)
    r.close(); r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    for x in (r, r2):
        try: x.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
