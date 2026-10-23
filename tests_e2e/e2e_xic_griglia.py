"""«Extract ions (XIC)» window: label | m/z-formula | adduct | X sit on one grid, so every row lines up (left/width), also on a phone; no intro paragraph."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8847, wd="/tmp/wd_xicgr")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0")])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("openXic(null)"); attendi(pg, "document.querySelector('#xicdlg').open && document.querySelectorAll('#xic-rows .xrw').length === 2")
        box = lambda sel: pg.evaluate("s => { const o = [...document.querySelectorAll(s)].map(e => { const b = e.getBoundingClientRect(); return [b.left, b.width]; }); return o; }", sel)
        def aligned():
            pg.fill("#xic-mz", "C9H10Cl2N2O"); attendi(pg, "!document.querySelector('#xic-rows .xrw select').hidden")
            for sel in ("#xic-rows .xrw > label", "#xic-rows .xrw input", "#xic-rows .xrw select"):
                a, b = box(sel)
                assert abs(a[0] - b[0]) < 0.5 and abs(a[1] - b[1]) < 0.5, (sel, a, b)
        step("rows are aligned (left and width) in label, field and adduct", aligned)
        def labels():
            t = pg.evaluate("[...document.querySelectorAll('#xic-rows .xrw > label')].map(l => l.textContent)")
            assert t == ["Ion 1", "Ion 2"] or t == ["Ione 1", "Ione 2"], t
            assert "optional" not in " ".join(t).lower() and "facoltativo" not in " ".join(t).lower(), t
            assert pg.locator("#xicdlg [data-i18n-html='xicdlg.intro']").count() == 0
            assert pg.evaluate("document.querySelector('#xicdlg').getAttribute('aria-description').length") > 40
        step("labels without «(optional)», no intro paragraph, description in aria-description", labels)
        def phone():
            pg.set_viewport_size({"width": 390, "height": 800}); frames(pg)
            d = pg.evaluate("(() => { const b = document.querySelector('#xicdlg').getBoundingClientRect(); return [b.left, b.right, innerWidth]; })()")
            assert d[0] >= 0 and d[1] <= d[2] + 0.5, d
            a, b = box("#xic-rows .xrw input"); assert abs(a[0] - b[0]) < 0.5 and abs(a[1] - b[1]) < 0.5, (a, b)
        step("phone width: the window fits and the rows stay aligned", phone)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
