"""Draw, «unsaved changes» (disegno-pulito.js): a clean or empty canvas asks nothing; a drawing that differs from the last saved or loaded state asks «Your drawing is not saved»
before a load replaces it (Cancel keeps it, «Save and replace» downloads a .ket that reopens to the same canvas, «Replace without saving» replaces it, the format choice
goes through the export panel's own code); undo back to the saved state is clean; the browser's `beforeunload` prompt is armed only while the canvas is dirty."""
# covers: mzlab/web/disegno-pulito.js, mzlab/web/memoria-avviso.js, mzlab/web/draw.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:260]))
def poll(cond, n=60):
    for _ in range(n):
        if cond(): return
        frames(pg, 15)
    raise AssertionError("state not reached")
r = Run(port=8872, wd="/tmp/wd_dpulito")
try:
    with sync_playwright() as p:
        global pg
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=90000)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        blank = lambda: pg.evaluate(f"{K}.editor.struct().isBlank()")
        dlg = lambda: pg.locator("#draw-unsaved").count() == 1
        unload = lambda: pg.evaluate("!window.dispatchEvent(new Event('beforeunload', { cancelable: true }))")      # true = a handler asks the browser to prompt
        def add(smi="CCO"): pg.evaluate(f"{K}.addFragment('{smi}')"); poll(lambda: not blank())
        def load(smi):
            pg.evaluate(f"(() => {{ window.__p = TPDraw.load({smi!r}); }})()")
        def done(): return pg.evaluate("Promise.race([window.__p, new Promise(r => setTimeout(() => r('pending'), 15000))])")      # the load, or «pending» when a question is still open

        def clean():
            assert blank() and not unload(), "empty canvas: nothing to warn, no beforeunload handler"
            load("CCO"); assert done() is True and not dlg(), "a load over an empty canvas asks nothing"
            frames(pg, 10); assert not blank() and not unload(), "a loaded file is clean until edited"
            load("CCN"); assert done() is True and not dlg(), "a load over a clean canvas asks nothing"
        step("empty and loaded canvases are clean: no dialog, no beforeunload", clean)

        def dirty_handler():
            add("C"); poll(unload)
            assert unload(), "beforeunload registered while dirty"
            pg.evaluate("TPDraw.undo(false)")
            poll(lambda: not unload())
        step("an edit arms beforeunload; undo back to the loaded state disarms it", dirty_handler)

        def cancel():
            add("O"); poll(unload)
            load("CCC"); pg.wait_for_selector("#draw-unsaved", timeout=5000)
            box = pg.locator("#draw-unsaved .mm-box")
            assert box.get_attribute("role") == "alertdialog" and box.get_attribute("aria-labelledby") == "mm-h" and box.get_attribute("aria-describedby") == "mm-p"
            assert pg.inner_text("#draw-unsaved #mm-h") == "Your drawing is not saved"
            assert pg.evaluate("document.activeElement.id") == "mm-no", "Cancel has the focus"
            texts = [pg.inner_text(f"#draw-unsaved {b}") for b in ("#mm-save", "#mm-drop", "#mm-no")]
            assert texts == ["Save and replace", "Replace without saving", "Cancel"], texts
            assert all(len(pg.get_attribute(f"#draw-unsaved {b}", "title") or "") in range(1, 61) for b in ("#mm-save", "#mm-drop", "#mm-no")), "tooltips <= 60 characters"
            pg.keyboard.press("Escape"); poll(lambda: not dlg())
            assert done() is False and not blank(), "Esc = Cancel keeps the drawing"
            assert pg.evaluate("TPDraw.smiles()") .count(".") >= 0 and unload(), "still dirty"
        step("a load over a dirty canvas asks; Esc cancels", cancel)

        def save_and_replace():
            smi_before = pg.evaluate("TPDraw.smiles()"); n_before = pg.evaluate(f"[{K}.editor.struct().atoms.size, {K}.editor.struct().bonds.size]")
            load("CCC"); pg.wait_for_selector("#draw-unsaved", timeout=5000)
            assert pg.input_value("#draw-unsaved #mm-fmt") == "ket", "the lossless .ket is the default"
            with pg.expect_download(timeout=60000) as d: pg.click("#draw-unsaved #mm-save")
            poll(lambda: not dlg())
            assert d.value.suggested_filename.endswith(".ket"), d.value.suggested_filename
            text = open(d.value.path(), encoding="utf-8").read()
            assert done() is True, "the canvas was replaced after the download"
            assert pg.evaluate("TPDraw.smiles()") == "CCC"
            pg.evaluate(f"(() => {{ window.__p = TPDraw.load({text!r}); }})()"); frames(pg, 20)      # the saved file reopens to the same canvas
            assert done() is True and not dlg(), "a load over a clean canvas asks nothing"
            poll(lambda: pg.evaluate("TPDraw.smiles()") == smi_before)      # Ketcher recentres a loaded file, so the same structure, not the same coordinates
            n = lambda: pg.evaluate(f"[{K}.editor.struct().atoms.size, {K}.editor.struct().bonds.size]")
            assert n() == n_before, (n(), n_before)
        step("«Save and replace» downloads a .ket that reopens to the same canvas", save_and_replace)

        def drop_and_format():
            add("N"); load("CC"); pg.wait_for_selector("#draw-unsaved", timeout=5000)
            pg.select_option("#draw-unsaved #mm-fmt", "svg")
            with pg.expect_download(timeout=60000) as d: pg.click("#draw-unsaved #mm-save")
            poll(lambda: not dlg()); assert d.value.suggested_filename.endswith(".svg"), d.value.suggested_filename
            add("N"); load("CC"); pg.wait_for_selector("#draw-unsaved", timeout=5000)
            pg.click("#draw-unsaved #mm-drop"); poll(lambda: not dlg())
            assert done() is True and pg.evaluate("TPDraw.smiles()") == "CC", "replaced without saving"
        step("the format choice uses the export panel; «Replace without saving» replaces", drop_and_format)

        def new_clean_and_tab():
            new = pg.frame_locator("#kframe").locator('button[data-testid="clear-canvas"]:visible')
            new.click(); pg.wait_for_selector("#draw-new", timeout=5000)
            assert pg.inner_text("#draw-new #mm-h") == "Clear the drawing?", "saved drawing: the plain question of «New»"
            pg.click("#draw-new #mm-no"); poll(lambda: pg.locator("#draw-new").count() == 0)
            add("O"); poll(unload)
            new.click(); pg.wait_for_selector("#draw-unsaved", timeout=5000)
            assert pg.inner_text("#draw-unsaved #mm-save") == "Save and clear"
            pg.click("#draw-unsaved #mm-no"); poll(lambda: not dlg())
        step("«New»: saved asks the plain question, unsaved the «not saved» one", new_clean_and_tab)

        def real_unload():
            pg.mouse.click(700, 400)      # a page needs a user gesture before it may ask on unload
            with pg.expect_event("dialog", timeout=15000) as dd:
                pg.evaluate("setTimeout(() => location.reload(), 50)")
            assert dd.value.type == "beforeunload", dd.value.type
            dd.value.dismiss()            # stay on the page
        step("leaving the page with a dirty canvas raises the browser's beforeunload prompt", real_unload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
