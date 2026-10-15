"""«Only points» display of the spectrum: every point is a dot, no line and no sticks between them; reachable from the parameters popover (low resolution)
and from the display options (DISP), kept in the panel state."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_spettro_punti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8983, wd="/tmp/wd_punti")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0")])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("window.__arcs=0; const a=CanvasRenderingContext2D.prototype.arc; CanvasRenderingContext2D.prototype.arc=function(){window.__arcs++; return a.apply(this,arguments)}; 0")
        SP = "E.panels.find(p=>p.type==='spec')"
        redraw = f"(async()=>{{window.__arcs=0; await draw({SP}); return window.__arcs}})()"
        def off():
            attendi(pg, f"{SP} && {SP}._a")
            assert pg.evaluate(redraw) == 0, "no dots by default"
        step("by default the spectrum has no dots", off)
        def popover():
            pg.evaluate(f"{SP}.el.querySelector('[data-a=par]').click()"); attendi(pg, f"!!{SP}.el.querySelector('.sp-pop [data-s=dots]')")
            pg.evaluate(f"{SP}.el.querySelector('.sp-pop [data-s=dots]').click()")
            attendi(pg, f"{SP}.dots===true")
            n = pg.evaluate(redraw); assert n > 5, n
        step("popover: «Points only» draws a dot for every point", popover)
        def nolines():
            pg.evaluate(f"{SP}.sticks=true"); n = pg.evaluate(redraw); assert n > 5
            pg.evaluate(f"{SP}.sticks=false")
        step("dots win over the sticks", nolines)
        def save():
            pg.evaluate("uiSave(true)"); attendi(pg, "JSON.stringify(NB.ui||{}).includes('\"dots\":true')")
        step("the session keeps it", save)
        def back():
            pg.evaluate(f"{SP}.el.querySelector('.sp-pop [data-s=dots]').click()"); attendi(pg, f"{SP}.dots===false")
            assert pg.evaluate(redraw) == 0
        step("unticked: back to the line", back)
        step("state is coherent", lambda: pg.evaluate("STATO.verifica()") == [] or (_ for _ in ()).throw(AssertionError("stato")))
    errs = [e for e in r.errs if not (e[0].startswith("console") and ("GL" in str(e[1]) or "sandbox" in str(e[1])))]
    steps.append(("no browser errors", "ok" if not errs else "FAIL " + str(errs[:3])))
finally:
    r.close()
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
