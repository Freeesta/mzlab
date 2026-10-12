"""File list: a double click on the colour square opens the colour picker at once (the colour then changes); the rename prompt no longer carries the «only a label» note."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lista_colore.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8992, wd="/tmp/wd_lista_colore")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.click("#stbar [data-cmd=file]"); pg.wait_for_selector("#flst .fl:not(.ghost) > i", timeout=5000)
        def colour():
            k = pg.evaluate("+Q('#flst .fl:not(.ghost) .nm').dataset.k")
            pg.evaluate("window.__picked = 0; HTMLInputElement.prototype.showPicker = function () { window.__picked = this.type === 'color' ? 1 : 0; window.__inp = this; }")
            pg.dblclick("#flst .fl:not(.ghost) > i")
            assert pg.evaluate("window.__picked") == 1, "the picker did not open"
            assert not pg.evaluate("!!Q('#colpop')"), "the swatch popup opened instead"
            pg.evaluate("(() => { window.__inp.value = '#112233'; window.__inp.dispatchEvent(new Event('change')); })()")
            assert pg.evaluate(f"E.files[{k}].color") == "#112233" and pg.evaluate(f"E.files[{k}].colorSet") is True
        step("double click on the colour square opens the picker and sets the colour", colour)
        def rename():
            pg.dblclick("#flst .fl:not(.ghost) .nm"); pg.wait_for_timeout(400)
            t = pg.inner_text("body"); assert "it is only a label" not in t and "è solo un'etichetta" not in t
            pg.keyboard.press("Escape")
        step("the rename prompt has no «only a label» note", rename)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
