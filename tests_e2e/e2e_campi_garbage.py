"""Garbage in the typed numeric fields (campi.js): letters, 1e999, empty, a number outside the bounds go into every visible field of the panel headers,
the spectrum parameters, the peak table and Settings; each is refused with one calm note (role=status), the old value comes back and nothing changes in the
state or in a request; a comma decimal («12,5») is taken. The parser itself is checked on the same cases."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_campi import sweep, typed
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_campi_garbage.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = synth("campi_garbage")
r = Run(port=8912, wd="/tmp/wd_campi_garbage")
SPEC = "E.panels.find(q => q.type === 'spec' && q.tab === E.tab)"
STATE = ("JSON.stringify([E.panels.map(q => [q.thr, q.nlab, q.gamma, q.gdelay, q.pdaDelay, q.bw0, q.bw1, q.mz0, q.mz1, q.fondo && q.fondo.w, q.fondo && q.fondo.zd]),"
         " localStorage.getItem('mzlab.prefs'), localStorage.getItem('mzlab.llm')])")
urls = []
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.on("request", lambda q: urls.append(q.url))
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        def parser():
            v = pg.evaluate("""[['250,8', '250.8', ' 7 ', '-3', '−3', '1e3', '.5', '', 'abc', '1e999', '12abc', '1,2,3', '1,2.3', '--1', 'Infinity', 'NaN'].map(t => CAMPI.num(t)),
              [numIt('1e999'), numIt('5 ppm'), numIt('0,5')]].map(a => a.map(x => Number.isNaN(x) ? 'NaN' : x))""")
            assert v[0][:8] == [250.8, 250.8, 7, -3, -3, 1000, 0.5, None], v
            assert v[0][8:] == ["NaN"] * 8, v
            assert v[1] == ["NaN", 5, 0.5], v
        step("the parser: comma or point, sign and exponent taken; letters, 1e999, two separators refused; numIt never gives Infinity", parser)
        def headers():
            pg.evaluate(f"(() => {{ const p = {SPEC}; p.bg = 'w'; ctl(p); }})()")   # «background of a stretch»: the two time boxes of the spectrum header
            n = sweep(pg, "#dpanels .pnl", STATE)
            print("header fields", n)
            typed(pg, "#dpanels .pnl [data-o=bw0]", "1,5")
            wait_for_js(pg, f"{SPEC}.bw0 === 1.5", 5000)
        step("panel headers: every visible numeric field refuses garbage with a note, old value back, state unchanged", headers)
        def spec_params():
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=par]').click()"); wait_for_js(pg, "!!document.querySelector('.sp-pop [data-s=thr]')", 5000)
            sweep(pg, ".sp-pop", STATE)
            typed(pg, ".sp-pop [data-s=thr]", "12,5")
            wait_for_js(pg, f"{SPEC}.thr === 13", 5000)
        step("spectrum parameters: garbage refused; «12,5» is taken", spec_params)
        def peak_table():
            pg.keyboard.press("Escape")
            pg.evaluate(f"{SPEC}.el.querySelector('[data-a=tabp]').click()"); wait_for_js(pg, "!!document.querySelector('.tp-pop')", 5000)
            sweep(pg, ".tp-pop", STATE)
            pg.keyboard.press("Escape")
        step("peak table: garbage refused", peak_table)
        def settings():
            pg.click("#np-set"); wait_for_js(pg, "!!document.querySelector('#uip-ppm')", 5000)
            sweep(pg, "#uipset", STATE)
            typed(pg, "#uip-ppm", "7,0")
            wait_for_js(pg, "JSON.parse(localStorage.getItem('mzlab.prefs') || '{}').hrPpm === 7", 5000)
            pg.keyboard.press("Escape")
        step("Settings: garbage refused; «7,0» ppm is taken", settings)
        def requests():
            bad = [u for u in urls if any(s in u for s in ("NaN", "Infinity", "undefined"))]
            assert not bad, bad[:3]
        step("no request carried NaN, Infinity or undefined", requests)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
