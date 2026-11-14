"""Draw, Ketcher's «New» (`clear-canvas`, disegno-nuovo.js): an empty canvas is cleared without a question; a non-empty one asks (alertdialog of
memoria-avviso.js: «Cancel» has the focus, Tab stays inside, Esc and «Cancel» keep the drawing, «New drawing» clears it); Ctrl+Delete asks the same;
undo after the clear works as in Ketcher."""
# covers: mzlab/web/disegno-nuovo.js, mzlab/web/memoria-avviso.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:260]))
def poll(cond, n=40):
    for _ in range(n):
        if cond(): return
        frames(pg, 15)
    raise AssertionError("state not reached")
r = Run(port=8871, wd="/tmp/wd_dnuovo")
try:
    with sync_playwright() as p:
        global pg
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=90000)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        blank = lambda: pg.evaluate(f"{K}.editor.struct().isBlank()")
        fr = pg.frame_locator("#kframe")
        new = fr.locator('button[data-testid="clear-canvas"]:visible')     # another one sits in the hidden macromolecules bar
        dlg = lambda: pg.locator("#draw-new").count() == 1
        def add(): pg.evaluate(f"{K}.addFragment('CCO')"); poll(lambda: not blank())

        def empty():
            assert blank(), "fresh page: nothing drawn"
            new.click(); frames(pg, 20)
            assert not dlg(), "no question on an empty canvas"
        step("empty canvas: «New» asks nothing", empty)

        def ask_cancel():
            add(); new.click(); pg.wait_for_selector("#draw-new", timeout=5000)
            box = pg.locator("#draw-new .mm-box")
            assert box.get_attribute("role") == "alertdialog" and box.get_attribute("aria-modal") == "true"
            assert pg.inner_text("#draw-new #mm-h") == "Cancellare il disegno?", pg.inner_text("#draw-new #mm-h")
            assert pg.locator("#draw-new p").count() == 0, "a question, no explanation"
            assert pg.evaluate("document.activeElement.id") == "mm-no", "the safe button has the focus"
            for b in ("#mm-yes", "#mm-no"):
                bb = pg.locator(f"#draw-new {b}").bounding_box(); assert bb["width"] >= 24 and bb["height"] >= 24, (b, bb)
            pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "mm-yes", "Tab wraps inside"
            pg.keyboard.press("Shift+Tab"); assert pg.evaluate("document.activeElement.id") == "mm-no"
            pg.keyboard.press("Escape"); poll(lambda: not dlg())
            assert not blank(), "Esc keeps the drawing"
            new.click(); pg.wait_for_selector("#draw-new", timeout=5000)
            pg.click("#draw-new #mm-no"); poll(lambda: not dlg())
            assert not blank(), "Cancel keeps the drawing"
        step("non-empty: dialog, Esc and Cancel keep the drawing", ask_cancel)

        def confirm():
            new.click(); pg.wait_for_selector("#draw-new", timeout=5000)
            pg.click("#draw-new #mm-yes"); poll(lambda: not dlg())
            poll(blank)
            pg.evaluate("TPDraw.undo(false)"); poll(lambda: not blank())
        step("«New drawing» clears; undo brings the drawing back", confirm)

        def keys():
            if blank(): add()
            fr.locator("body").click(position={"x": 700, "y": 450})
            pg.keyboard.press("Control+Delete"); pg.wait_for_selector("#draw-new", timeout=5000)
            pg.keyboard.press("Enter"); poll(lambda: not dlg())
            assert not blank(), "Enter on the focused Cancel keeps the drawing"
            fr.locator("body").click(position={"x": 700, "y": 450})
            pg.keyboard.press("Control+Delete"); pg.wait_for_selector("#draw-new", timeout=5000)
            pg.keyboard.press("Tab"); pg.keyboard.press("Enter"); poll(lambda: not dlg())
            poll(blank)
            fr.locator("body").click(position={"x": 700, "y": 450})
            pg.keyboard.press("Control+Delete"); frames(pg, 20)
            assert not dlg(), "keys on an empty canvas ask nothing"
        step("Ctrl+Delete asks the same, by keyboard only", keys)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
