"""Double click = new spectrum with its RT in the title; up/down arrows and sliding panels; the Metodo button stands out."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8816, wd="/tmp/wd16")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES[:3]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def dbl():
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='spec').length")
            for rt in (10.0, 14.3):
                c = pg.evaluate("""(x)=>{const p=E.panels[0],r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(x),py:r.top+r.height/2}}""", rt)
                pg.mouse.dblclick(c["px"], c["py"]); pg.wait_for_timeout(1200)
            n1 = pg.evaluate("E.panels.filter(p=>p.type==='spec').length"); assert n1 == n0 + 2, (n0, n1)
            t = [x.strip().replace("fermo a ", "") for x in pg.locator(".pnl.spec .rtl").all_inner_texts()]       # the frozen ones say "fermo a RT ..."
            rts = [float(x.split()[1]) for x in t if x.startswith("RT ")]       # the pixel under the click gives 10.0 ± 0.03 min, whatever the width of the page
            assert any(abs(v - 10.0) < 0.1 for v in rts) and any(abs(v - 14.3) < 0.1 for v in rts), t
            hd = pg.evaluate("[...document.querySelectorAll('.pnl.spec')].pop().querySelector('.hd').innerText"); assert "Spettro di massa" in hd and "RT" in hd, hd
        step("each double click opens a new spectrum, RT next to the title", dbl)
        pg.screenshot(path=SH + "161_dbl.png")
        def arrows():
            S = "E.panels.slice().sort((a,b)=>a.y-b.y)"
            order = lambda: pg.evaluate(S + ".map(p=>p.id)")
            move = lambda i, d: pg.evaluate(f"movePanel({S}[{i}], {d})")
            n = pg.evaluate("E.panels.length")
            # panels are moved by dragging the header (no arrow buttons any more); movePanel is the same move: the chromatogram carries its spectra, so the swap is tested on a spectrum
            assert pg.evaluate("document.querySelectorAll('[data-a=up],[data-a=down]').length") == 0
            o0 = order(); move(1, 1); pg.wait_for_function(f"{S}.map(p=>p.id).join() !== '{','.join(map(str, o0))}'"); o1 = order()
            assert o1[0] == o0[0] and o1[1] == o0[2] and o1[2] == o0[1], (o0, o1)
            move(2, -1); pg.wait_for_function(f"{S}.map(p=>p.id).join() === '{','.join(map(str, o0))}'"); assert order() == o0
            move(n - 1, 1); move(0, -1); assert order() == o0, "the last cannot go down, the first cannot go up"
        step("panels swap with their neighbour (the first cannot go up, the last cannot go down)", arrows)
        def method():
            b = pg.locator("#np-method"); assert "imp" in b.get_attribute("class")
            pg.screenshot(path=SH + "162_method.png", clip={"x": 0, "y": 40, "width": 1500, "height": 120})
        step("Metodo button is distinct", method)
finally:
    r.close()
print("\nSTEPS"); [print(" ", s) for s in steps]
print([e for e in r.errs if "ERR_ABORTED" not in str(e)])
