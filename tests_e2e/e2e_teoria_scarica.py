"""Teoria 'Scarica' dialog: in every row the format label (left) and its two buttons share the same top/bottom (1 px tolerance), at desktop and 400 px
width, in light and dark theme; buttons are at least 40 px tall."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_scarica.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8977, wd="/tmp/wd_teoria_scarica")
ROWS = """() => [...document.querySelectorAll('.exp-tab tbody tr')].map(tr => {
  const b = e => { const q = e.getBoundingClientRect(); return [q.top, q.bottom, q.height]; };
  return { th: b(tr.querySelector('th')), btns: [...tr.querySelectorAll('button')].map(b) };
})"""
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/01-tp.html"
        def check(w, theme):
            c = r.b.new_context(viewport={"width": w, "height": 900}, color_scheme=theme); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            q.goto(T); q.wait_for_function("!!window.TEORIA && !!TEORIA.esporta", timeout=10000)
            q.evaluate("TEORIA.esporta()"); q.wait_for_selector(".exp-tab tbody tr")
            rows = q.evaluate(ROWS); assert len(rows) == 4 and all(len(x["btns"]) == 2 for x in rows)
            for x in rows:
                for bt in x["btns"]:
                    assert bt[0] >= x["th"][0] - 1, x
                    assert bt[1] <= x["th"][1] + 1 and bt[2] >= 40, x
                assert abs(x["btns"][0][0] - x["btns"][1][0]) <= 1, x
            tops = [x["th"][0] for x in rows]; bots = [x["th"][1] for x in rows]
            assert all(abs(tops[i + 1] - bots[i]) <= 1 for i in range(3)), (tops, bots)   # rows are contiguous: shared borders
            c.close()
        for w in (1280, 400):
            for th in ("light", "dark"):
                step(f"rows aligned at {w}px, {th}", lambda w=w, th=th: check(w, th))
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
