"""Window resize: one draw() per panel (the window listener, the workspace observer and the panel observer used to draw it three times), and the
picture is the one a fresh draw gives. A panel whose canvas changes size is still redrawn, and so is one whose last draw was skipped while hidden."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ridimensiona_un_disegno.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("ridimensiona_un_disegno")
r = Run(port=8881, wd="/tmp/wd_ridimensiona_un_disegno")
COUNT = "window.__dr = []; if (!window.__drWrapped) { window.__drWrapped = true; const d0 = window.draw; window.draw = async function (p) { window.__dr.push(p && p.id); return d0.apply(this, arguments); }; } 0"
PANELS = "E.panels.filter(p => p.tab === E.tab && p.cv && p.cv.clientWidth > 30)"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        ids = pg.evaluate(f"{PANELS}.map(p => p.id)")
        counts = lambda: pg.evaluate("(() => { const c = {}; window.__dr.forEach(i => { c[i] = (c[i] || 0) + 1; }); return c; })()")
        shots = lambda: pg.evaluate(f"{PANELS}.map(p => p.cv.toDataURL())")

        def resize():
            assert len(ids) >= 2, f"{len(ids)} panels shown"
            for w in (1200, 1100, 1240):
                pg.evaluate(COUNT); pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 5); ready(pg)
                c = counts()
                assert all(c.get(str(i), 0) == 1 for i in ids), f"draw() calls per panel for one resize to {w} px: {c}"
                a = shots(); pg.evaluate(f"{PANELS}.forEach(p => draw(p))"); frames(pg); ready(pg)
                assert a == shots(), "the picture after the resize differs from a fresh draw"
        step("window resize: one draw() per panel, same picture as a fresh draw", resize)

        def by_hand():
            pg.evaluate(COUNT); pg.evaluate(f"(() => {{ const p = {PANELS}[0]; p.el.style.height = (p.el.offsetHeight + 60) + 'px'; }})()"); frames(pg, 5); ready(pg)
            assert counts() == {str(ids[0]): 1}, f"a taller panel is redrawn once: {counts()}"
        step("a panel made taller by hand is redrawn once", by_hand)

        def hidden():
            # a draw asked while the panel is hidden is skipped: shown again at the same size, the panel is redrawn with the new state
            pid = pg.evaluate("E.panels.find(p => p.type === 'chrom' && p.tab === E.tab).id")
            pg.evaluate(f"(() => {{ const p = E.panels.find(q => q.id === {pid}); p.el.style.display = 'none'; const x = p._a.sr[0].x; p.zoom = [x[0], x[Math.floor(x.length / 3)]]; draw(p); }})()"); frames(pg, 3)
            pg.evaluate(COUNT); pg.evaluate(f"E.panels.find(q => q.id === {pid}).el.style.display = ''"); frames(pg, 5); ready(pg)
            assert sum(counts().values()) >= 1, "the panel shown again was not redrawn"
            a = pg.evaluate(f"E.panels.find(q => q.id === {pid}).cv.toDataURL()")
            pg.evaluate(f"draw(E.panels.find(q => q.id === {pid}))"); frames(pg); ready(pg)
            assert a == pg.evaluate(f"E.panels.find(q => q.id === {pid}).cv.toDataURL()"), "the panel shown again keeps the picture of before it was hidden"
        step("a draw skipped while hidden is done when the panel shows again", hidden)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
