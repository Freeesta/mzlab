"""A tooltip never covers the toolbar of the control it describes: below it, else above it, else beside it (eventi.js show())."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tip_posto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8984, wd="/tmp/wd_tip_posto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        # a two-row toolbar: the tip of a button of the first row must not cover the second row
        pg.evaluate("""(() => { const d = document.createElement('div'); d.id = 'tbz'; d.setAttribute('role', 'toolbar'); d.style.cssText = 'position:fixed;left:200px;top:120px;width:260px;display:flex;flex-wrap:wrap;gap:20px;background:#eee;z-index:99999';
          d.innerHTML = [...Array(8)].map((_, i) => `<button type="button" title="A rather long description of button ${i} that is wider than the button">B${i}</button>`).join(''); document.body.appendChild(d); })()""")
        def clear():
            n = pg.locator("#tbz button").count(); assert n == 8
            for i in (0, 3, 7):
                pg.locator("#tbz button").nth(i).hover(); attendi(pg, "!document.getElementById('qtip').hidden", 5000)
                v = pg.evaluate("(() => { const a = document.getElementById('qtip').getBoundingClientRect(), z = document.getElementById('tbz').getBoundingClientRect(); return [a.left < z.right && a.right > z.left && a.top < z.bottom && a.bottom > z.top, a.top, a.bottom, z.top, z.bottom]; })()")
                assert v[0] is False, ("the tooltip of button %d covers its toolbar" % i, v)
                pg.mouse.move(5, 5); attendi(pg, "document.getElementById('qtip').hidden", 5000)
        step("the tooltip of a button of a two-row toolbar stays outside the whole toolbar", clear)
        def header():
            bad = []; seen = 0
            for sel in ("#stbar button", "#tools button"):
                L = pg.locator(sel); k = min(L.count(), 6)
                for i in range(k):
                    b = L.nth(i)
                    if not b.is_visible(): continue
                    b.hover(); 
                    try: attendi(pg, "!document.getElementById('qtip').hidden", 3000)
                    except Exception: pg.mouse.move(5, 5); continue
                    v = pg.evaluate(f"(() => {{ const a = document.getElementById('qtip').getBoundingClientRect(); let z = document.querySelectorAll('{sel}')[{i}]; for (let e = z.parentElement; e; e = e.parentElement) if (e.matches('#stbar, #tools, [role=toolbar]')) z = e; const b = z.getBoundingClientRect(); return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top; }})()")
                    seen += 1
                    if v: bad.append((sel, i))
                    pg.mouse.move(5, 5); attendi(pg, "document.getElementById('qtip').hidden", 5000)
            assert seen >= 3, ("too few tooltips were checked", seen)
            assert not bad, bad
        step("the tooltips of the real toolbars (#stbar, #tools) never cover their toolbar", header)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
