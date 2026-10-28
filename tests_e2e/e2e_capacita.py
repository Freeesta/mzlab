"""Capacities of the browser (capacita.js, avviso-capacita.js): with ?cap-off=<name> the function bound to that capacity is off, with the common notice
(«non disponibile in questo browser. Usa l'app») and the link to the app; without it no notice at all; Dati with an example file works for every capacity that is not
the engine's; the probes of the page cost less than 5 ms (median of 5)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_capacita.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8851, wd="/tmp/wd_capacita")
NOTE = "document.querySelectorAll('.cap-note, #cap-live').length"
LINK = "[...document.querySelectorAll('.cap-note a')].map(a => a.href)"
# capacity -> function bound to it
FN = {"simd128": "motore", "opfs": "motore", "opfsSync": "motore", "cartellaLocale": "cartella", "webgl2": "mappa3d", "isolamento": "indiceParallelo"}

def load(b, q=""):
    pg = b.new_context(locale="it-IT", viewport={"width": 1440, "height": 950}).new_page()
    pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.goto(f"http://127.0.0.1:{r.port}/{q}"); ready(pg); return pg

def open_example(pg):
    pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1")
    pg.click("#opbtn"); ready(pg)
    assert pg.evaluate("E.panels.length") > 0, "Dati opened no panel"

try:
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        def clean():
            pg = load(b)
            assert pg.evaluate(NOTE) == 0, "notice without cap-off"
            assert all(pg.evaluate(f"CAPACITA.stato('{f}')") != "no" for f in set(FN.values()) if f != "indiceParallelo"), "a function is off in a normal Chromium"
            open_example(pg); assert pg.evaluate(NOTE) == 0, "notice after opening a file"
        step("no cap-off: no notice, Dati works", clean)
        def cost():
            pg = load(b); xs = sorted(pg.evaluate("[0,1,2,3,4].map(() => { const t = performance.now(); CAPACITA.rileva(true); return performance.now() - t; })"))
            print(f"  rileva() cost, median of 5: {xs[2]:.2f} ms")
            assert xs[2] <= 5, xs
        step("the probes of the page cost at most 5 ms", cost)
        for cap, fn in FN.items():
            def off(cap=cap, fn=fn):
                pg = load(b, f"?cap-off={cap}")
                assert pg.evaluate(f"CAPACITA.stato('{fn}')") == "no" and not pg.evaluate(f"CAPACITA.ha('{cap}')")
                if fn != "motore": open_example(pg)                               # everything that is not the engine's: Dati works
                if fn == "motore":
                    attendi(pg, "window.MZLAB_RUST && window.MZLAB_RUST.disabled === true"); attendi(pg, "document.getElementById('cap-live')")
                elif fn == "cartella":
                    pg.evaluate("LIB.open()"); attendi(pg, "document.getElementById('lib-fchoose') && !document.getElementById('lib-fchoose').hidden")
                    assert pg.get_attribute("#lib-fchoose", "aria-disabled") == "true"
                elif fn == "mappa3d":
                    pg.click("#np-map"); ready(pg)
                    pg.evaluate("E.panels.find(q=>q.type==='map').el.scrollIntoView({block:'center'})")
                    btn = pg.locator(".pnl.map [data-o=view][data-v='3d']"); btn.click()
                    assert btn.get_attribute("aria-disabled") == "true" and pg.locator("dialog.m3d").count() == 0
                else:
                    pg.evaluate("(() => { const b = document.createElement('button'); b.id = 'probe'; document.body.appendChild(b); CAPACITA.serve('indiceParallelo', b); })()")
                    assert pg.get_attribute("#probe", "aria-disabled") == "true"
                attendi(pg, "document.getElementById('cap-live')"); t = pg.inner_text("#cap-live"); assert "non disponibile in questo browser" in t, t
                assert any("github.com/Freeesta/mzlab/releases" in h for h in pg.evaluate("[...document.querySelectorAll('#cap-live a')].map(a => a.href)")), "no link to the app"
            step(f"cap-off={cap}: «{fn}» off with the notice and the link", off)
        b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:3])
sys.exit(1 if bad else 0)
