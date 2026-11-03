"""Calculator layout: the input, the result well, the «Cerca in Δm» button and the key grid share the same left/right edges (within 1 px),
and the result text is centred in its well; at desktop and phone width, in light and dark."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, str(e).split("\n")[0][:240]))
EDGES = """() => { const e = s => { const r = document.querySelector(s).getBoundingClientRect(); return [r.left, r.right]; };
  const t = document.createRange(); t.selectNodeContents(document.querySelector('#calcres')); const tr = t.getBoundingClientRect(), w = document.querySelector('#calcres').getBoundingClientRect();
  return { input: e('#calcin'), well: e('#calcres'), nl: e('#calc-nl'), pad: e('#calcpad'), textMid: (tr.left + tr.right) / 2, wellMid: (w.left + w.right) / 2,
    textVMid: (tr.top + tr.bottom) / 2, wellVMid: (w.top + w.bottom) / 2 }; }"""
def check(pg):
    d = pg.evaluate(EDGES)
    for k in ("well", "nl", "pad"):
        assert abs(d[k][0] - d["input"][0]) <= 1 and abs(d[k][1] - d["input"][1]) <= 1, (k, d)
    assert abs(d["textMid"] - d["wellMid"]) <= 1.5 and abs(d["textVMid"] - d["wellVMid"]) <= 3, d
r = Run(port=8830 + 162, wd="/tmp/wd_calc_allinea")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0")])
        pg.click("text=Carica dati"); ready(pg)
        pg.click("#np-calc2"); attendi(pg, "!!document.querySelector('#calcin').offsetParent")
        pg.fill("#calcin", "4+5"); attendi(pg, "document.querySelector('#calcres').innerText.trim() === '9'")
        attendi(pg, "!document.querySelector('#calc-nl').hidden")
        step("desktop, light: same edges, centred result", lambda: check(pg))
        def dark():
            pg.emulate_media(color_scheme="dark"); frames(pg, 3); check(pg)
        step("desktop, dark", dark)
        def phone():
            pg.set_viewport_size({"width": 390, "height": 800}); frames(pg, 5)
            if not pg.evaluate("!!document.querySelector('#calcin').offsetParent"):
                pg.evaluate("BARRA.setTab('calc')"); attendi(pg, "!!document.querySelector('#calcin').offsetParent")
            pg.fill("#calcin", "4+5"); attendi(pg, "document.querySelector('#calcres').innerText.trim() === '9' && !document.querySelector('#calc-nl').hidden")
            check(pg)
        step("phone width", phone)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
sys.exit(1 if bad else 0)
