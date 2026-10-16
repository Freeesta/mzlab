"""«Removed · Undo» (storia.js): taking a file out of the session, closing a panel and clearing the marked points of the map ask no question, show a notice with an
Undo button for 6 s (role=status) and are entries of the undo stack (Ctrl/Cmd+Z, Shift+Ctrl/Cmd+Z); a reload empties the stack. Alt+Shift+arrows keep their meaning
inside a text field (the side of the drawer does not move)."""
import sys, os, time; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tolto_annulla.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8899, wd="/tmp/wd_tolto_annulla")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        notice = lambda: pg.locator("#tolto-box .tolto").count()
        dialogs = lambda: pg.evaluate("document.querySelectorAll('dialog[open]').length")
        FK = "E.files.find(f => f.kind === 'full' && !f.gone).k"
        def file_undo():
            k = pg.evaluate(FK); assert pg.evaluate("document.getElementById('tolto-box')") is None
            pg.evaluate(f"removeFile(E.files[{k}])"); attendi(pg, f"E.files[{k}].gone === true && !!document.querySelector('#tolto-box .tolto')")
            assert dialogs() == 0, "no question"
            assert pg.get_attribute("#tolto-box", "role") == "status"
            assert "Undo" in pg.inner_text("#tolto-box") or "Annulla" in pg.inner_text("#tolto-box")
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"E.files[{k}].gone === false")
            assert pg.evaluate(f"E.files[{k}].vis") is True, "Ctrl+Z puts the file back"
            assert notice() == 0, "the notice goes with the undo"
            pg.keyboard.press(f"{mod}+Shift+z"); attendi(pg, f"E.files[{k}].gone === true")
        step("file: no question, notice, Ctrl+Z puts it back, Shift+Ctrl+Z removes it", file_undo)
        def file_button():
            k = pg.evaluate("E.files.findIndex(f => f.gone)")
            pg.evaluate(f"E.files[{k}].gone=false;E.files[{k}].vis=true;renderFileList()"); pg.evaluate(f"removeFile(E.files[{k}])"); attendi(pg, "!!document.querySelector('#tolto-box .tolto button')")
            pg.click("#tolto-box .tolto button"); attendi(pg, f"E.files[{k}].gone === false")
            assert notice() == 0, "the button of the notice puts it back"
        step("file: the button of the notice", file_button)
        def panel_close():
            pg.evaluate("reveal(addPanel('chrom', { k: E.files.find(f => f.kind === 'full' && !f.gone).k }))"); ready(pg)
            pid = pg.evaluate("E.panels.filter(q => q.type === 'chrom').slice(-1)[0].id")
            pg.evaluate(f"(()=>{{const q=E.panels.find(x=>x.id=={pid});q.zoom=[2,6];q.title='Prova tolto'}})()")
            n0 = pg.evaluate("E.panels.length"); y0 = pg.evaluate(f"E.panels.find(x=>x.id=={pid}).y")
            pg.evaluate(f"E.panels.find(x=>x.id=={pid}).el.querySelector('.x').click()"); attendi(pg, f"E.panels.length === {n0 - 1} && !!document.querySelector('#tolto-box .tolto')")
            assert dialogs() == 0
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"E.panels.length === {n0} && E.panels.some(x => x.title === 'Prova tolto')"); ready(pg)
            q = pg.evaluate("(()=>{const q=E.panels.find(x=>x.title==='Prova tolto');return {zoom:q.zoom,tab:q.tab,y:q.y,in:q.el.isConnected}})()")
            assert q["zoom"] == [2, 6] and q["in"], q
            assert abs(q["y"] - y0) < 2, (q, y0)
            pg.keyboard.press(f"{mod}+Shift+z"); attendi(pg, f"E.panels.length === {n0 - 1}")
        step("panel: the X asks nothing, Ctrl+Z reopens it in place with its zoom", panel_close)
        def marks():
            pg.click("#np-map"); ready(pg)
            pg.evaluate(M + ".el.scrollIntoView({block:'start'})")
            pg.evaluate(f"MAPPA.addPointAt({M}, 9.0, 305.07, 0.2)"); pg.evaluate(f"MAPPA.addPointAt({M}, 14.3, 364.4, 0.2)")
            attendi(pg, f"({M}._mz.rows || []).length === 2 && !!document.querySelector('.pnl.map .mztab')")
            pg.click(".pnl.map .mztab [data-t=clear]"); attendi(pg, f"MAPPA.pts({M}).length === 0 && !!document.querySelector('#tolto-box .tolto')")
            assert dialogs() == 0
            pg.click("#tolto-box .tolto button"); attendi(pg, f"MAPPA.pts({M}).length === 2")
            pg.click(".pnl.map .mztab [data-t=clear]"); attendi(pg, f"MAPPA.pts({M}).length === 0")
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"MAPPA.pts({M}).length === 2")
        step("marks: clearing them asks nothing, the notice and Ctrl+Z put them back", marks)
        def timed():
            pg.evaluate("removeFile(E.files[E.files.findIndex(f => !f.gone && f.kind === 'full')])"); attendi(pg, "!!document.querySelector('#tolto-box .tolto')")
            pg.mouse.move(2, 2); t0 = time.time()
            attendi(pg, "!document.querySelector('#tolto-box .tolto')", 12000)
            assert 5 <= time.time() - t0 <= 9, time.time() - t0
        step("the notice lasts 6 s", timed)
        def keys_in_field():
            live = lambda: pg.evaluate("document.getElementById('sb-live').textContent")
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press("Alt+Shift+ArrowRight"); ready(pg); pg.keyboard.press("Alt+Shift+ArrowLeft"); attendi(pg, "document.getElementById('sb-live').textContent !== ''")
            pg.evaluate("(()=>{const i=document.createElement('input');i.id='tmpin';document.body.appendChild(i);i.focus();document.getElementById('sb-live').textContent=''})()")
            pg.keyboard.press("Alt+Shift+ArrowRight"); ready(pg)
            assert live() == "", "inside a text field Alt+Shift+arrows are the field's: " + live()
            pg.evaluate("document.getElementById('tmpin').remove()")
        step("Alt+Shift+arrows in a text field are left alone", keys_in_field)
        def reload():
            pg.evaluate("removeFile(E.files[E.files.findIndex(f => !f.gone && f.kind === 'full')])"); attendi(pg, "STORIA.pila.v.length > 0")
            pg.reload(); attendi(pg, "typeof STORIA !== 'undefined' && STORIA !== null")
            assert pg.evaluate("STORIA.pila.v.length") == 0 and pg.evaluate("STORIA.pila.r.length") == 0, "a reload empties the stack"
        step("a reload empties the stack", reload)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
