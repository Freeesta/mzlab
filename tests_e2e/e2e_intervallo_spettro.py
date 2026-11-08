"""Double click (mouse), double tap (touch) and Enter INSIDE the dragged range of a chromatogram open the averaged spectrum of that range in a new spectrum panel;
outside the range or without one the old behaviour stays; selection beats zoom. Synthetic data only."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_intervallo_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("intervallo")
r = Run(port=8862, wd="/tmp/wd_intervallo")
CH = "E.panels.find(p => p.type === 'chrom' && p.tab === 'full')"
NS = "E.panels.filter(p => p.type === 'spec').length"
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1024, "height": 1366}, has_touch=True, locale="it-IT")
        pg = ctx.new_page(); pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(e){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#pick", state="attached")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def X(rt): return pg.evaluate(f"(() => {{ const p = {CH}, q = p.cv.getBoundingClientRect(); return q.left + p._a.X({rt}); }})()")
        def Ymid(): return pg.evaluate(f"(() => {{ const q = {CH}.cv.getBoundingClientRect(); return q.top + q.height * 0.5; }})()")
        pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); frames(pg, 3)
        rts = pg.evaluate(f"{CH}._a.sr[0].x"); lo, hi = rts[len(rts) // 4], rts[len(rts) // 2]
        def drag(a, b):
            pg.mouse.move(X(a), Ymid()); pg.mouse.down()
            for k in range(1, 9): pg.mouse.move(X(a) + (X(b) - X(a)) * k / 8, Ymid()); frames(pg, 1)
            pg.mouse.up(); ready(pg)
        def sel(): return pg.evaluate(f"{CH}.sel")
        def ids(): return pg.evaluate("E.panels.filter(p => p.type === 'spec').map(p => p.id)")
        def new_spec(before): return pg.evaluate("(b => { const q = E.panels.find(p => p.type === 'spec' && !b.includes(p.id)); return q ? [q.r0, q.r1] : null; })", before)      # the new panel, wherever it goes: in the DDA trio the MS2 panel stays last
        def inside():
            drag(lo, hi); s = sel(); assert s and abs(s[0] - lo) < 0.5 and abs(s[1] - hi) < 0.5, s
            n0, b = pg.evaluate(NS), ids(); pg.mouse.dblclick((X(lo) + X(hi)) / 2, Ymid()); ready(pg)
            assert pg.evaluate(NS) == n0 + 1, "a new spectrum panel"
            r01 = new_spec(b); assert abs(r01[0] - s[0]) < 1e-6 and abs(r01[1] - s[1]) < 1e-6, (r01, s)
            assert sel() == s, "the selection stays"
        step("double click inside the range: spectrum of that range in a new panel", inside)
        def outside():
            n0 = pg.evaluate(NS); pg.mouse.dblclick(X(hi) + 80, Ymid()); ready(pg)
            assert sel() is None, "outside: old behaviour (spectrum here, selection cleared)"
        step("double click outside the range keeps the old behaviour", outside)
        def zoomed():
            drag(lo, hi); s = sel()
            pg.evaluate(f"zoomTo({CH}, {rts[0]}, {rts[-1] * 0.9})"); ready(pg); frames(pg, 3)
            z = pg.evaluate(f"{CH}.zoom"); assert z
            n0 = pg.evaluate(NS); pg.mouse.dblclick((X(lo) + X(hi)) / 2, Ymid()); ready(pg)
            assert pg.evaluate(f"{CH}.zoom") == z and pg.evaluate(NS) == n0 + 1, "selection wins over the zoom"
            pg.mouse.dblclick(X(hi) + 60, Ymid()); ready(pg)
            assert pg.evaluate(f"{CH}.zoom") is None, "outside the range the zoomed chart still resets"
        step("zoomed + selection: the selection wins; outside it the zoom resets", zoomed)
        def enter():
            drag(lo, hi); n0, b = pg.evaluate(NS), ids()
            assert pg.evaluate(f"!!{CH}.cv.getAttribute('aria-label')")
            pg.evaluate(f"{CH}.cv.focus()"); pg.keyboard.press("Enter"); ready(pg)
            assert pg.evaluate(NS) == n0 + 1, "Enter opens the range spectrum"
            r01 = new_spec(b); s = sel(); assert abs(r01[0] - s[0]) < 1e-6 and abs(r01[1] - s[1]) < 1e-6, (r01, s)
        step("Enter on the focused chart with a range opens its spectrum", enter)
        def late():
            # the click of the pair asks for its nearest scan; a reply that comes after the double click must not overwrite the range spectrum
            drag(lo, hi); s, b = sel(), ids()
            pg.evaluate("(() => { const f = nearScan; window._nsF = f; window._nsGo = []; window.nearScan = (k, x) => new Promise(ok => window._nsGo.push(() => f(k, x).then(ok))); })()")
            pg.mouse.dblclick((X(lo) + X(hi)) / 2, Ymid()); ready(pg)
            assert pg.evaluate("window._nsGo.length") >= 1, "the click asked for its nearest scan"
            pg.evaluate("(async () => { const g = window._nsGo; window.nearScan = window._nsF; for (const go of g) await go(); })()"); frames(pg, 3); ready(pg)
            r01 = new_spec(b); assert r01 and abs(r01[0] - s[0]) < 1e-6 and abs(r01[1] - s[1]) < 1e-6, (r01, s)
            assert sel() == s, "the selection stays"
        step("a late nearest-scan reply after the double click leaves the range spectrum alone", late)
        def tap2():
            drag(lo, hi); n0 = pg.evaluate(NS); x, y = (X(lo) + X(hi)) / 2, Ymid()
            for _ in range(2): T("touchStart", [(x, y)]); T("touchEnd", [])
            ready(pg); assert pg.evaluate(NS) == n0 + 1, "double tap inside the range"
        step("double tap inside the range (touch)", tap2)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
