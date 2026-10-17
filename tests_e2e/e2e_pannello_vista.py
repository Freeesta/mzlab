"""Side panel: leaving Dati for Disegno and Teoria and coming back gives the panel as it was left (open or shut, width, scroll)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pannello_vista.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8995, wd="/tmp/wd_pannello_vista")
def round_trip(pg):
    for v in ("draw", "theory", "data"):
        pg.evaluate(f"setView('{v}')")
        attendi(pg, f"S.view === '{v}'")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg)
        def aperto():
            pg.evaluate("BARRA.setTab('file', {noFocus: true, instant: true})"); attendi(pg, "BARRA.open")
            pg.evaluate("BARRA.setWidth(380)")
            pg.evaluate("(() => { const e = document.querySelector('#sb-panels'); e.style.maxHeight = '120px'; e.scrollTop = 40; })()")
            top = pg.evaluate("document.querySelector('#sb-panels').scrollTop")
            round_trip(pg)
            attendi(pg, "BARRA.open")
            assert pg.is_visible("#dfiles"), "the panel must be open again"
            assert pg.evaluate("BARRA.width") == 380, pg.evaluate("BARRA.width")
            attendi(pg, f"document.querySelector('#sb-panels').scrollTop === {top}")
        step("open: Data → Drawing → Theory → Data gives it back open (width, scroll)", aperto)
        def chiuso():
            pg.evaluate("BARRA.close(true)"); attendi(pg, "!BARRA.open")
            round_trip(pg)
            assert not pg.evaluate("BARRA.open") and not pg.is_visible("#dfiles"), "a shut panel must stay shut"
        step("shut: the same trip leaves it shut", chiuso)
        def riavvio():
            pg.evaluate("BARRA.setTab('file', {noFocus: true, instant: true})"); attendi(pg, "BARRA.open")
            assert pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr')).open") is True
        step("the open state is saved for the next start", riavvio)
finally:
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
