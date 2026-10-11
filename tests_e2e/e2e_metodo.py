"""One field «Load method (optional)» with the formats read today: chips «Method: name» with ×, from the keyboard too, one notice for the files it cannot read, it/en."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_metodo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
def blob(name, data=b"X"): return {"name": name, "mimeType": "application/octet-stream", "buffer": data}
r = Run(port=8992, wd="/tmp/wd_metodo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def field():
            t = pg.inner_text("#dropdam"); assert "Carica il metodo (facoltativo) — .dam SCIEX" in t, t
            assert pg.get_attribute("#pickdam", "accept") == ".dam" and pg.locator("#start .tag.opt").count() == 0
            assert pg.evaluate("METHOD_FORMATS.length") == 1
        step("one field with the formats from a single constant", field)
        def chips():
            pg.set_input_files("#pickdam", [blob("m1.dam"), blob("m2.dam")])
            pg.wait_for_function("document.querySelectorAll('#mlist .chip').length===2", timeout=15000)
            t = pg.inner_text("#mlist"); assert "Metodo: m1.dam" in t and "Metodo: m2.dam" in t, t
        step("two methods in one go: two chips", chips)
        def remove():
            pg.focus("#mlist .chip button >> nth=0"); pg.keyboard.press("Enter")
            pg.wait_for_function("document.querySelectorAll('#mlist .chip').length===1", timeout=10000)
            assert "m2.dam" in pg.inner_text("#mlist")
        step("× removes a method (keyboard)", remove)
        def keyboard():
            pg.evaluate("window.__c=0;document.querySelector('#pickdam').addEventListener('click',e=>{window.__c++;e.preventDefault()})")
            pg.focus("#dropdam"); pg.keyboard.press("Enter"); pg.keyboard.press("Space")
            assert pg.evaluate("window.__c") == 2, pg.evaluate("window.__c")
            pg.set_input_files("#pickdam", [blob("m3.dam")])
            pg.wait_for_function("document.querySelectorAll('#mlist .chip').length===2", timeout=10000)
        step("the field opens the chooser from the keyboard", keyboard)
        def unknown():
            pg.set_input_files("#pickdam", [blob("a.meth"), blob("b.meth"), blob("c.xyz")])
            pg.wait_for_function("!document.querySelector('#unopen').hidden", timeout=10000)
            t = pg.inner_text("#unopen"); assert "3 file ignorati" in t, t
            assert pg.locator("#unopen").count() == 1 and pg.locator("#mlist .chip").count() == 2
        step("unrecognised files: one notice", unknown)
        def en():
            pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); pg.wait_for_timeout(800)
            t = pg.inner_text("#dropdam"); assert "Load method (optional) — .dam SCIEX" in t, t
            pg.set_input_files("#pickdam", [blob("q.dam")]); pg.wait_for_function("document.querySelector('#mlist').innerText.includes('Method: q.dam')", timeout=10000)
            assert "Method: q.dam" in pg.inner_text("#mlist")
        step("English", en)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
