# e2e_doppio_clic: double click on a graph: zoomed = whole view (undoable), not zoomed = the main action (cases D of the shortcuts matrix)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8869, wd="/tmp/wd_doppio")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def dbl(sel, fx=0.5, fy=0.5):
            c = pg.locator(sel + " canvas").first; c.scroll_into_view_if_needed(); b = c.bounding_box()
            pg.mouse.dblclick(b["x"] + b["width"] * fx, b["y"] + b["height"] * fy); pg.wait_for_timeout(900)
        nspec = lambda: pg.evaluate("E.panels.filter(p=>p.type==='spec').length")
        CH = "E.panels.find(p=>p.type==='chrom')"
        def tic():
            open_kind(pg, "full"); ready(pg)
            pg.evaluate(f"{CH}.zoom=[1,2]"); n = nspec()
            dbl(".pnl.chrom")
            assert pg.evaluate(f"{CH}.zoom") is None and nspec() == n, "D1: zoomed double click = whole view, no new spectrum"
            pg.evaluate(f"undoZoom({CH})"); assert pg.evaluate(f"{CH}.zoom") == [1, 2], "D1: the whole view can be undone (pushZh)"
            pg.evaluate(f"{CH}.zoom=null"); dbl(".pnl.chrom")
            assert pg.evaluate(f"{CH}.cur") is not None, "D2: not zoomed = spectrum at that RT (cursor set)"
        step("D1/D2: TIC zoomed = whole view and undo; not zoomed = spectrum at the RT", tic)
        def mrm():
            open_kind(pg, "mrm"); ready(pg)
            M = "E.panels.find(p=>p.type==='mrm')"; n = pg.evaluate("E.panels.length")
            pg.evaluate(f"{M}.zoom=[1,2]"); dbl(".pnl.mrm")
            assert pg.evaluate(f"{M}.zoom") is None; pg.evaluate(f"undoZoom({M})"); assert pg.evaluate(f"{M}.zoom") == [1, 2], "D6: undoable"
            pg.evaluate(f"{M}.zoom=null"); dbl(".pnl.mrm")
            assert pg.evaluate("E.panels.length") == n and pg.evaluate("E.tab") == "mrm", "D7: nothing happens"
        step("D6/D7: MRM zoomed = whole view and undo; not zoomed = nothing", mrm)
        def dblp(P, fx=0.5, fy=0.5):                      # double click on the canvas of that very panel
            pg.evaluate(f"{P}.cv.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(200)
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top,r.width,r.height]}})()")
            pg.mouse.dblclick(b[0] + b[2] * fx, b[1] + b[3] * fy); pg.wait_for_timeout(900)
        def spec():
            open_kind(pg, "full"); ready(pg)
            S = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
            if not pg.evaluate("!!" + S): dbl(".pnl.chrom"); pg.wait_for_timeout(500)
            pg.evaluate(f"(()=>{{const p={S};p.scrollTo&&0;p.zoom=[p._a.x0+10,p._a.x1-10];draw(p)}})()")
            dblp(S)
            assert pg.evaluate(f"{S}.zoom") is None, "D8: zoomed spectrum = whole view"
            pg.evaluate(f"undoZoom({S})"); assert pg.evaluate(f"!!{S}.zoom"), "D8: undoable"
            pg.evaluate(f"(()=>{{const p={S};p.zoom=null;p.meas={{ref:100,list:[]}};draw(p)}})()"); dblp(S, 0.5, 0.2)
            assert pg.evaluate(f"{S}.meas") is None, "D9: not zoomed = takes off the measures"
        step("D8/D9: spectrum zoomed = whole view and undo; not zoomed = measures off", spec)
        def annot():
            open_kind(pg, "full"); ready(pg)
            pg.evaluate(f"(()=>{{const p={CH};p.anns.push({{x:(p._a.x0+p._a.x1)/2,text:'prova'}});draw(p)}})()"); pg.wait_for_timeout(300)
            pos = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect(),l=p._a.lbls.find(b=>b.ann);return [q.left+l.x+l.w/2,q.top+l.y+l.h/2]}})()")
            pg.mouse.dblclick(pos[0], pos[1]); pg.wait_for_timeout(400)
            assert "Annotazione" in pg.inner_text("#asktxt"), pg.inner_text("#asktxt"); pg.click("#askno")
        step("D15: annotation window with the translated title", annot)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
