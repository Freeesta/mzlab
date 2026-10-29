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
        pg.set_input_files("#pick", synth.make_series("/tmp/s26", [0, 15, 60])); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        def header():
            assert "mzLab" not in pg.inner_text("header").replace("\n", " ").split("Dati")[0]
            assert pg.get_attribute("header .logo", "alt") == "mzLab"
            for w in (360, 768, 1500):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3)
                if w < 600:      # very narrow: everything but File is in «Altro»
                    assert not pg.is_visible("#np-nl"); pg.click("#tb-more"); pg.locator("#stmenu button", has_text="Perdite neutre").click()
                else:
                    assert pg.is_visible("#np-nl"), w
                    pg.click("#np-nl")
                attendi(pg, "!!document.querySelector('#sb-panel-losses') && document.querySelector('#sb-panel-losses').offsetParent !== null")
                assert pg.is_visible("#sb-panel-losses") and pg.inner_text("#sb-title") == "Perdite neutre", w
                pg.click("#sb-x"); frames(pg, 3); assert not pg.is_visible("#dfiles"), w
            pg.set_viewport_size({"width": 1400, "height": 900})
        step("header: Perdite neutre opens its tab at 360/768/1500 px, no title text", header)
        pg.click("#np-chrom"); attendi(pg, "E.panels.some(p=>p.type==='chrom')"); pg.click("#np-spec"); attendi(pg, "E.panels.some(p=>p.type==='spec')")
        def fs():
            n = pg.evaluate("document.querySelectorAll('.pnl').length"); assert n >= 2, n
            pg.click("#np-xic"); frames(pg, 3); pg.keyboard.press("Escape"); attendi(pg, "document.querySelectorAll('.pnl').length>=3")
            n = pg.evaluate("document.querySelectorAll('.pnl').length"); assert n >= 3, n
            for i in range(n):
                pg.evaluate("document.querySelectorAll('.pnl .fsb')[%d].click()" % i); attendi(pg, "document.querySelectorAll('.pnl')[%d].classList.contains('max')" % i)
                box = pg.evaluate("(()=>{const e=document.querySelectorAll('.pnl')[%d];const r=e.getBoundingClientRect();return [r.width,r.height,e.classList.contains('max')]})()" % i)
                assert box[2] and box[0] > 1000 * 0.9 and box[1] > 600, (i, box)
                pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('.pnl.max')")
                assert pg.evaluate("!document.querySelector('.pnl.max')"), (i, pg.evaluate("[!!document.querySelector('dialog[open]'), !!document.querySelector('#helppop:not([hidden])'), !!document.querySelector('#ctx:not([hidden])')]"))
        step("full screen on every panel (incl. XIC added later), Esc leaves it", fs)
        def tip():
            btn = pg.locator(".pnl .fsb").first; pg.mouse.move(5, 5); hold(pg, 100); btn.hover(); hold(pg, 1000)      # a pointer that is already on the button (left there by the step before) makes no mouseover, so no tooltip
            assert pg.evaluate("document.querySelector('#qtip').hidden"), "too early"
            attendi(pg, "!document.querySelector('#qtip').hidden", 10000)      # the delay is a timer of the page: on a loaded machine it fires late, so wait for the state (the «too early» check above still holds the lower bound)
            assert "Schermo intero" in pg.inner_text("#qtip")
            pg.mouse.move(5, 5); attendi(pg, "document.querySelector('#qtip').hidden"); assert pg.evaluate("document.querySelector('#qtip').hidden")
        step("tooltip after ~1.7 s, not before, gone when the pointer leaves", tip)
        def tipbox():
            # the tooltip never covers the element it describes (selects, buttons, table cells)
            sel = pg.locator("select[title], select[data-tip], .pnl .fsb, #tools button[title]")
            n = min(sel.count(), 6); assert n, "no element with a tooltip"
            for i in range(n):
                el = sel.nth(i)
                if not el.is_visible(): continue
                pg.mouse.move(5, 5); hold(pg, 100); el.hover()
                attendi(pg, "!document.querySelector('#qtip').hidden", 10000)
                ov = pg.evaluate("(el)=>{const a=el.getBoundingClientRect(),b=document.querySelector('#qtip').getBoundingClientRect();return a.left<b.right&&b.left<a.right&&a.top<b.bottom&&b.top<a.bottom}", el.element_handle())
                assert not ov, ("tooltip overlaps its element", i)
            pg.mouse.move(5, 5)
        step("tooltip never overlaps its element", tipbox)
        def longpress():
            # touch has no hover: a long press on a button shows its tooltip; a short one does not
            btn = pg.locator(".pnl .fsb").first; box = btn.bounding_box(); x, y = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2
            fire = "(a)=>{const b=document.querySelector('.pnl .fsb');for(const t of a){b.dispatchEvent(new PointerEvent(t,{pointerType:'touch',bubbles:true,cancelable:true,clientX:%f,clientY:%f}))}}" % (x, y)
            pg.mouse.move(5, 5); hold(pg, 100)
            pg.evaluate(fire, ["pointerdown"]); hold(pg, 200); pg.evaluate(fire, ["pointerup"])
            assert pg.evaluate("document.querySelector('#qtip').hidden"), "a short touch must not show the tooltip"
            pg.evaluate(fire, ["pointerdown"])
            attendi(pg, "!document.querySelector('#qtip').hidden", 5000)
            assert "Schermo intero" in pg.inner_text("#qtip")
            pg.evaluate(fire, ["pointerup"])
            attendi(pg, "document.querySelector('#qtip').hidden", 8000)
        step("long press on a button shows its tooltip on touch", longpress)
        def rt():
            ci = pg.evaluate("E.panels.findIndex(p=>p.type==='chrom'&&p.tab===E.tab)"); assert ci >= 0
            pg.evaluate(f"E.panels[{ci}].el.scrollIntoView({{block:'center'}})"); frames(pg, 3)
            b = pg.evaluate(f"(()=>{{const r=E.panels[{ci}].cv.getBoundingClientRect(),a=E.panels[{ci}]._a;return {{x:r.left,w:r.width,y:r.top+r.height/2,l:a.X(a.x0+(a.x1-a.x0)*0.5)-0,mid:r.left+a.X(a.x0+(a.x1-a.x0)*0.8)}}}})()")
            state = lambda: pg.evaluate(f"(()=>{{const q=E.panels[{ci}];return [q.sel,q._a.full]}})()")
            pg.mouse.move(b["mid"], b["y"]); pg.mouse.down(); pg.mouse.move(b["x"] - 200, b["y"], steps=8); pg.mouse.up(); attendi(pg, f"E.panels[{ci}].sel!=null")
            s, full = state(); assert s is not None, "the drag must create a selection (otherwise the test proves nothing)"
            assert s[0] >= full[0] - 1e-9 and s[1] <= full[1] + 1e-9 and s[0] - full[0] < 0.1 * (full[1] - full[0]), (s, full)           # never before the start of the run, and it did reach the left end
            pg.mouse.move(b["mid"], b["y"]); pg.mouse.down(); pg.mouse.move(b["x"] + b["w"] + 300, b["y"], steps=8); pg.mouse.up(); frames(pg, 3)
            s, full = state(); assert s is not None and s[1] <= full[1] + 1e-9 and full[1] - s[1] < 0.1 * (full[1] - full[0]) and s[0] >= full[0] - 1e-9, (s, full)    # and never after the end
        step("RT selection dragged outside the chromatogram stays inside its range", rt)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
