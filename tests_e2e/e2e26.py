"""Data tab: full screen on every panel (also added later), RT selection clamped to the chromatogram range, delayed tooltip, header buttons Isotopi / Perdite neutre, no 'QqQ lab' text in the header. Synthetic data."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e26.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8826 + 100, wd="/tmp/wd26")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        shutil.rmtree("/tmp/s26", ignore_errors=True)
        pg.set_input_files("#pick", synth.make_series("/tmp/s26", [0, 15, 60])); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def header():
            assert "mzLab" not in pg.inner_text("header").replace("\n", " ").split("Dati")[0]
            assert pg.get_attribute("header .logo", "alt") == "mzLab"
            for w in (360, 768, 1500):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(200)
                if w < 600:      # very narrow: everything but File is in «Altro»
                    assert not pg.is_visible("#np-nl"); pg.click("#tb-more"); pg.locator("#stmenu button", has_text="Perdite neutre").click()
                else:
                    assert pg.is_visible("#np-nl"), w
                    pg.click("#np-nl")
                pg.wait_for_timeout(400)
                assert pg.is_visible("#sb-panel-losses") and pg.inner_text("#sb-title") == "Perdite neutre", w
                pg.click("#sb-x"); pg.wait_for_timeout(300); assert not pg.is_visible("#dfiles"), w
            pg.set_viewport_size({"width": 1400, "height": 900})
        step("header: Perdite neutre opens its tab at 360/768/1500 px, no title text", header)
        pg.click("#np-chrom"); pg.wait_for_timeout(800); pg.click("#np-spec"); pg.wait_for_timeout(800)
        def fs():
            n = pg.evaluate("document.querySelectorAll('.pnl').length"); assert n >= 2, n
            pg.click("#np-xic"); pg.wait_for_timeout(600); pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            n = pg.evaluate("document.querySelectorAll('.pnl').length"); assert n >= 3, n
            for i in range(n):
                pg.evaluate("document.querySelectorAll('.pnl .fsb')[%d].click()" % i); pg.wait_for_timeout(400)
                box = pg.evaluate("(()=>{const e=document.querySelectorAll('.pnl')[%d];const r=e.getBoundingClientRect();return [r.width,r.height,e.classList.contains('max')]})()" % i)
                assert box[2] and box[0] > 1000 * 0.9 and box[1] > 600, (i, box)
                pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
                assert pg.evaluate("!document.querySelector('.pnl.max')"), (i, pg.evaluate("[!!document.querySelector('dialog[open]'), !!document.querySelector('#helppop:not([hidden])'), !!document.querySelector('#ctx:not([hidden])')]"))
        step("full screen on every panel (incl. XIC added later), Esc leaves it", fs)
        def tip():
            btn = pg.locator(".pnl .fsb").first; btn.hover(); hold(pg, 1000)
            assert pg.evaluate("document.querySelector('#qtip').hidden"), "too early"
            hold(pg, 1100)
            assert not pg.evaluate("document.querySelector('#qtip').hidden") and "Schermo intero" in pg.inner_text("#qtip")
            pg.mouse.move(5, 5); pg.wait_for_timeout(200); assert pg.evaluate("document.querySelector('#qtip').hidden")
        step("tooltip after ~1.7 s, not before, gone when the pointer leaves", tip)
        def rt():
            ci = pg.evaluate("E.panels.findIndex(p=>p.type==='chrom'&&p.tab===E.tab)"); assert ci >= 0
            pg.evaluate(f"E.panels[{ci}].el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            b = pg.evaluate(f"(()=>{{const r=E.panels[{ci}].cv.getBoundingClientRect(),a=E.panels[{ci}]._a;return {{x:r.left,w:r.width,y:r.top+r.height/2,l:a.X(a.x0+(a.x1-a.x0)*0.5)-0,mid:r.left+a.X(a.x0+(a.x1-a.x0)*0.8)}}}})()")
            state = lambda: pg.evaluate(f"(()=>{{const q=E.panels[{ci}];return [q.sel,q._a.full]}})()")
            pg.mouse.move(b["mid"], b["y"]); pg.mouse.down(); pg.mouse.move(b["x"] - 200, b["y"], steps=8); pg.mouse.up(); pg.wait_for_timeout(300)
            s, full = state(); assert s is not None, "the drag must create a selection (otherwise the test proves nothing)"
            assert s[0] >= full[0] - 1e-9 and s[1] <= full[1] + 1e-9 and s[0] - full[0] < 0.1 * (full[1] - full[0]), (s, full)           # never before the start of the run, and it did reach the left end
            pg.mouse.move(b["mid"], b["y"]); pg.mouse.down(); pg.mouse.move(b["x"] + b["w"] + 300, b["y"], steps=8); pg.mouse.up(); pg.wait_for_timeout(300)
            s, full = state(); assert s is not None and s[1] <= full[1] + 1e-9 and full[1] - s[1] < 0.1 * (full[1] - full[0]) and s[0] >= full[0] - 1e-9, (s, full)    # and never after the end
        step("RT selection dragged outside the chromatogram stays inside its range", rt)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
