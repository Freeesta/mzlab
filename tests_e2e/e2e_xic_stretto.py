"""XIC panel at phone width: the header shrinks its m/z range fields (app.css hd-tight); no field runs under the «⋯», full-screen or close buttons, a focused field is in view."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_stretto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("xic_stretto")
r = Run(port=8905, wd="/tmp/wd_xic_stretto")
X = "E.panels.find(q => q.type === 'xic' && q.tab === E.tab)"
# the header buttons that the point at their centre does not reach (something drawn over them)
COVERED = f"""(() => {{ const h = {X}.el.querySelector('.hd'); return [...h.querySelectorAll('button')].filter(b => b.offsetWidth).filter(b => {{
  const r = b.getBoundingClientRect(), e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return !(e === b || b.contains(e)); }}).map(b => b.getAttribute('aria-label') || b.className); }})()"""
# each m/z field shown in the header (not the ones in the closed «⋯» menu), once focused, lies inside the visible part of its label, is at least
# 30 px wide and is not under a header button (the buttons come later in the page, so they are drawn over a field that runs into them)
FOCUSED = f"""(() => {{ const h = {X}.el.querySelector('.hd'), bs = [...h.querySelectorAll('button')].filter(b => b.offsetWidth).map(b => b.getBoundingClientRect());
  return [...h.querySelectorAll('input.mzf')].filter(i => i.offsetWidth).map(i => {{ i.focus(); const r = i.getBoundingClientRect(), l = i.closest('label').getBoundingClientRect();
    const under = bs.some(b => r.left < b.right - 1 && b.left < r.right - 1 && r.top < b.bottom - 1 && b.top < r.bottom - 1);
    const ok = r.left >= l.left - 1 && r.right <= l.right + 1 && r.width >= 30 && !under; i.blur(); return ok; }}); }})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        pg.evaluate("openXic(null,{})"); pg.wait_for_selector("#xic-mz", state="visible"); pg.fill("#xic-mz", "364.4"); pg.click("#xic-go")
        wait_for_js(pg, "E.panels.some(q => q.type === 'xic')"); frames(pg, 4); ready(pg)
        for w in (420, 360):
            def narrow(w=w):
                pg.set_viewport_size({"width": w, "height": 800}); frames(pg, 6)
                pg.evaluate(f"{X}.el.scrollIntoView({{block: 'start'}})"); frames(pg, 2)
                assert pg.evaluate(COVERED) == [], f"header buttons covered: {pg.evaluate(COVERED)}"
                assert all(pg.evaluate(FOCUSED)), f"a focused m/z field is out of view: {pg.evaluate(FOCUSED)}"
            step(f"{w} px: no m/z field under the header buttons, each focused field in view", narrow)
        def wide():
            pg.set_viewport_size({"width": 1400, "height": 900}); frames(pg, 6)
            assert pg.evaluate(f"{X}.el.querySelector('.hd input.mzf').offsetWidth") == 78, "the m/z field keeps its width when there is room"
        step("wide window: the m/z fields keep 78 px", wide)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
