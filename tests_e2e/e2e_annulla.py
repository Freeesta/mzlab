"""STORIA (storia.js): Ctrl/Cmd+Z undoes the last action of the visible tab (zoom, integration, smoothing), Shift+Ctrl+Z / Ctrl+Y redo it;
text fields keep the browser undo; an empty stack says so."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_annulla.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8884, wd="/tmp/wd84")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2500)
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.el.scrollIntoView({{block:'center'}})}})()"); pg.wait_for_timeout(400)
        zoom = lambda: pg.evaluate(f"{XP}.zoom")
        nint = lambda: pg.evaluate(f"{XP}.ints.length")
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        def key(k): pg.keyboard.press(k); pg.wait_for_timeout(500)
        def seq():
            pg.evaluate(f"(()=>{{const p={XP};pushZh(p);p.zoom=[13,17];draw(p)}})()"); pg.wait_for_timeout(500)
            pg.evaluate(f"(()=>{{const p={XP};addInt(p,p._a.sr[0],13.9,14.6)}})()"); pg.wait_for_timeout(300)
            assert nint() == 1 and zoom() == [13, 17]
            key(f"{mod}+z"); assert nint() == 0 and zoom() == [13, 17], "first Ctrl+Z: the integration"
            key(f"{mod}+z"); assert zoom() is None, "second Ctrl+Z: the zoom"
            key(f"{mod}+Shift+z"); assert zoom() == [13, 17], "Shift+Ctrl+Z redoes the zoom"
            key("Control+y"); assert nint() == 1, "Ctrl+Y redoes the integration"
        step("zoom, integration: undo twice, redo twice", seq)
        def cut():
            key(f"{mod}+z"); assert nint() == 0
            pg.evaluate(f"(()=>{{const p={XP};pushZh(p);p.zoom=[14,16];draw(p)}})()"); pg.wait_for_timeout(300)
            key(f"{mod}+Shift+z"); assert nint() == 0, "a new action cuts the redo"
        step("a new action cuts the redo", cut)
        def smooth():
            before = pg.evaluate(f"!!{XP}.smooth")
            pg.evaluate(f"{XP}.el.querySelector('[data-a=smooth]').click()"); pg.wait_for_timeout(300)
            assert pg.evaluate(f"!!{XP}.smooth") != before
            key(f"{mod}+z"); assert pg.evaluate(f"!!{XP}.smooth") == before, "smoothing undone"
        step("smoothing is undoable", smooth)
        def field():
            pg.evaluate("(()=>{const i=document.createElement('input');i.id='tmpin';document.body.appendChild(i);i.focus();window._dp=null;addEventListener('keydown',e=>{window._dp=e.defaultPrevented},false)})()")
            pg.keyboard.press(f"{mod}+z"); pg.wait_for_timeout(200)
            assert pg.evaluate("window._dp") is False, "inside a text field the browser keeps Ctrl+Z"
            pg.evaluate("document.getElementById('tmpin').remove()")
        step("Ctrl+Z in a text field stays the browser's", field)
        def empty():
            pg.evaluate("STORIA.svuota()"); pg.evaluate("document.activeElement&&document.activeElement.blur()")
            key(f"{mod}+z"); pg.wait_for_timeout(200); t = pg.inner_text("#sb-live"); assert "Niente da annullare" in t or "Nothing to undo" in t, t
            assert pg.evaluate("document.getElementById('np-undo').disabled") is True
        step("empty stack: short notice, button disabled", empty)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
