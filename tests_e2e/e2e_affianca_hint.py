"""Hint «Trascina qui per affiancare» (affianca.js, suggerimenti.js): shown ONCE per browser (qqq.pannelli.affianca.hint) when two panels could share a row and no other hint is open;
× and Esc close it, so does the first side by side; never together with the tour invitation (one queue); after a reload it does not come back."""
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KEY = "qqq.pannelli.affianca.hint"
r = Run(port=8877, wd="/tmp/wd_affianca_hint")
HINT = "document.getElementById('aff-hint')"
def fresh(p, tour_state=True, aff=None):
    r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": 1440, "height": 1000}, locale="it-IT", reduced_motion="reduce"); pg = ctx.new_page()
    init = "try{localStorage.setItem('qqq.barra.hint','1');" + ("localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));" if tour_state else "") + (f"localStorage.setItem('{KEY}','{aff}');" if aff else "") + "}catch(_){}"
    pg.add_init_script(init); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
    return pg
def load(pg):
    stage(pg, [mz("B_FullMass-t15")]); pg.click("text=Carica dati"); ready(pg)
try:
    with sync_playwright() as p:
        pg = fresh(p)
        def shown():
            load(pg); attendi(pg, f"!!{HINT}", timeout=15000)
            h = pg.evaluate(f"""(() => {{ const e = {HINT}, g = document.querySelector('.aff-grip.aff-hl'); if (!g) return null; const b = e.getBoundingClientRect(), r = g.getBoundingClientRect();
              return {{txt: e.textContent, role: e.getAttribute('role'), below: b.top >= r.bottom, n: document.querySelectorAll('#aff-hint').length, key: localStorage.getItem('{KEY}'), bar: !!document.getElementById('bar-hint'), invite: !!document.getElementById('tour-invito')}} }})()""")
            assert h and "Trascina qui per affiancare" in h["txt"] and h["role"] == "status" and h["below"] and h["n"] == 1 and h["key"] == "1" and not h["bar"] and not h["invite"], h
        step("two panels that fit side by side: the hint appears once, under the handle of the second, marked as seen", shown)
        def esc():
            pg.keyboard.press("Escape"); attendi(pg, f"!{HINT}")
            assert pg.evaluate("document.querySelectorAll('.aff-grip.aff-hl').length") == 0
        step("Esc closes it and the handle is no longer highlighted", esc)
        def again():
            pg.reload(); ready(pg); pg.wait_for_timeout(3500)
            assert pg.evaluate("E.panels.filter(q => q.full).length") >= 2, "the session is back with its panels"
            assert pg.evaluate(f"!{HINT}") and pg.evaluate(f"localStorage.getItem('{KEY}')") == "1", "it comes back after a reload"
        step("after a reload it does not come back", again)
        r.b.close()
        pg = fresh(p)
        def cross():
            load(pg); attendi(pg, f"!!{HINT}", timeout=15000)
            pg.click("#aff-hint button"); attendi(pg, f"!{HINT}")
        step("× closes it", cross)
        r.b.close()
        pg = fresh(p)
        def beside():
            load(pg); attendi(pg, f"!!{HINT}", timeout=15000)
            pg.evaluate("(() => { const l = stackOrder().filter(q => q.full); AFFIANCA.fai(l[1], { k: 'dx', t: l[0].id }, false) })()")
            attendi(pg, f"!{HINT}"); assert pg.evaluate("LAYOUT.righeDi(stackOrder()).some(r => r.length === 2)")
        step("the first side by side closes it", beside)
        r.b.close()
        pg = fresh(p, tour_state=False)
        def queue():
            load(pg); attendi(pg, "!!document.getElementById('tour-invito')", timeout=15000); pg.wait_for_timeout(2500)
            assert pg.evaluate(f"!{HINT}"), "never together with the tour invitation"
            assert pg.evaluate("SUGG.inCoda.includes('affianca')"), pg.evaluate("SUGG.inCoda")
            pg.click("#tour-inv-no"); attendi(pg, f"!!{HINT}", timeout=15000)
            assert pg.evaluate("!document.getElementById('tour-invito')")
        step("the tour invitation goes first; the hint waits in the queue and shows after it", queue)
        r.b.close()
        pg = fresh(p, aff="1")
        def never():
            load(pg); pg.wait_for_timeout(3500); assert pg.evaluate(f"!{HINT}")
        step("already seen in this browser: never shown", never)
        r.b.close()
        pg = fresh(p)
        def narrow():
            pg.set_viewport_size({"width": 700, "height": 1000}); pg.wait_for_timeout(300); load(pg); pg.wait_for_timeout(3000)
            assert pg.evaluate(f"!{HINT}") and pg.evaluate(f"localStorage.getItem('{KEY}')") is None, "no room side by side: no hint, and not marked as seen"
        step("a window too narrow for two panels side by side: no hint, not marked as seen", narrow)
        r.b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
