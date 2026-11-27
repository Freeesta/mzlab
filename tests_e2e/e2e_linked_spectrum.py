"""A UV spectrum opened by a double click on the MRM panel says where it comes from, at 1400 and 390 px: the subtitle names the source panel and the way it
was asked for, the «→ n» / «← n» badges say what they link, hovering a badge or the spectrum lights the dashed cursor of the source, and the cursor dot has a
tooltip naming the spectrum. «Align DAD and mass» adds the delay between the chart time and the DAD time. Synthetic MRM `.wiff` of tools/wiff_dad_mrm.py. With
MZLAB_SHOTS=<dir> the screenshots (device scale factor 2) are written there. Run from the repo root: python3 tests_e2e/e2e_linked_spectrum.py"""
# covers: mzlab/web/uv-spettro.js, mzlab/web/pannelli.js, mzlab/web/scroll.js
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="linkedsp_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
SHOTS = os.environ.get("MZLAB_SHOTS")
MRM = "E.panels.find(q => q.type === 'mrm')"
UV = "E.panels.find(q => q.type === 'spec' && q.uv)"

def load(pg):
    pg.set_input_files("#pick", [str(D / "dadmrm.wiff"), str(D / "dadmrm.wiff.scan")])
    wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Load data"); ready(pg)
    wait_for_js(pg, "E.files.length >= 1", timeout=120000); open_kind(pg, "mrm")
    wait_for_js(pg, f"!!{MRM} && !!{MRM}._a", timeout=60000)

def run(width, port):
    r = Run(port=port, wd=f"/tmp/wd_linkedsp{width}")
    with sync_playwright() as p:
        pg = r.page(p, viewport={"width": 1400, "height": 1400}, device_scale_factor=2 if SHOTS else 1)
        load(pg)       # loaded wide, then narrowed: the file picker of a phone-sized page is another test's business
        pg.set_viewport_size({"width": width, "height": 1400 if width > 600 else 1200}); frames(pg, 8)
        wait_for_js(pg, f"{MRM}.el.getBoundingClientRect().width > 0", timeout=30000)
        def at(rt):
            return pg.evaluate(f"(() => {{ const p = {MRM}, b = p.cv.getBoundingClientRect(), a = p._a; return [b.x + a.X({rt}), b.y + M.t + (a.H - M.t - M.b) / 2]; }})()")
        def open_uv():
            pg.evaluate(f"{MRM}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 3)
            x, y = at(TRUE["a"]["rt"]); pg.mouse.dblclick(x, y)
            wait_for_js(pg, f"!!{UV} && !!{UV}._a && !!{UV}._a.uvd", timeout=60000)
            pg.evaluate(f"{UV}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 3)
        def legend():
            open_uv()
            t = pg.evaluate(f"{UV}.leg2.textContent"); n = pg.evaluate(f"{MRM}.num"); rt = f"{TRUE['a']['rt']:.2f}"
            assert "double click" in t and f"panel {n}" in t and "follows its dashed cursor" in t, t
            assert pg.evaluate(f"{UV}.r0.toFixed(2)") == pg.evaluate(f"{MRM}.cur.toFixed(2)") == rt or abs(pg.evaluate(f"{UV}.r0 - {MRM}.cur")) < 0.05, t
        step(f"{width}px: the subtitle of the UV spectrum names its source panel, the double click and the cursor", legend)
        def badges():
            m, s = pg.evaluate(f"({MRM}.num)"), pg.evaluate(f"({UV}.num)")
            to = pg.evaluate(f"{MRM}.el.querySelector('.ppair').title"); fr = pg.evaluate(f"{UV}.el.querySelector('.ppair').title")
            assert f"Panel {s} is the UV spectrum" in to and "dashed cursor" in to, to
            assert f"dashed cursor of panel {m}" in fr and pg.evaluate(f"{UV}.el.querySelector('.ppair').getAttribute('aria-label')") == fr, fr
        step(f"{width}px: the → n and ← n badges say what they link (title and aria-label)", badges)
        def cursor_dot():
            t = pg.evaluate(f"{MRM}.ch.title"); n = pg.evaluate(f"{UV}.num")
            assert f"spectrum of panel {n} is read here" in t and "follows the pointer" in t, t
            assert pg.evaluate(f"{MRM}.ch.getAttribute('aria-label')") == t
        step(f"{width}px: the cursor dot names the spectrum read at the dashed cursor", cursor_dot)
        def light():
            lw = lambda: pg.evaluate(f"getComputedStyle({MRM}.cl).borderLeftWidth")
            assert lw() == "1px", lw()
            pg.locator(f"#dpanels .pnl:has(.ppair.fol) .ppair.fol").first.hover()            # the badge of the spectrum lights the source
            assert pg.evaluate(f"{MRM}.el.classList.contains('pair-hi')") and lw() == "2px", lw()
            if SHOTS: pg.evaluate(f"{MRM}.el.scrollIntoView({{block: 'start'}})"); frames(pg, 2); pg.screenshot(path=f"{SHOTS}/linked-badge-{width}.png")
            pg.mouse.move(2, 2); frames(pg, 2)
            assert not pg.evaluate(f"{MRM}.el.classList.contains('pair-hi')") and lw() == "1px"
            bx = pg.evaluate(f"(() => {{ const r = {MRM}.el.querySelector('.ppair').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }})()")
            pg.mouse.move(*bx); frames(pg, 2)                                                  # the badge of the source itself (CSS :has)
            assert lw() == "2px", lw()
            pg.mouse.move(2, 2); frames(pg, 2)
            bb = pg.evaluate(f"(() => {{ const r = {UV}.el.getBoundingClientRect(); return [r.x + r.width / 2, r.y + 40]; }})()")
            pg.mouse.move(*bb); frames(pg, 2)
            assert pg.evaluate(f"{MRM}.el.classList.contains('pair-hi')") and lw() == "2px", lw()
            if SHOTS: pg.screenshot(path=f"{SHOTS}/linked-spectrum-{width}.png")
            pg.mouse.move(2, 2)
        step(f"{width}px: hovering the badge or the spectrum lights the dashed cursor of the source", light)
        def align():
            pg.evaluate(f"{MRM}.pdaDelay = 0.5; {MRM}.pdaAlign = true; DISP.redraw ? 0 : 0; {UV}.r0 = {UV}.r0; drawSoon({UV}); drawSoon({MRM})"); frames(pg, 4)
            t = pg.evaluate(f"{UV}.leg2.textContent")
            assert "DAD delay 0.50 min" in t and "DAD time" in t, t
            pg.evaluate(f"{MRM}.pdaAlign = false; {MRM}.pdaDelay = null; drawSoon({UV})"); frames(pg, 3)
        step(f"{width}px: with «Align DAD and mass» the subtitle gives chart time, delay and DAD time", align)
        def fits():
            ov = pg.evaluate("document.documentElement.scrollWidth - innerWidth")
            lg = pg.evaluate(f"(() => {{ const e = {UV}.leg2.getBoundingClientRect(), q = {UV}.el.getBoundingClientRect(); return e.right - q.right; }})()")
            assert ov <= 1 and lg <= 1, (ov, lg)
        step(f"{width}px: nothing overflows the screen", fits)
        def clear():
            # both legends stay clear of the x axis title and of the curve: boxes in canvas CSS pixels, the ink of the curve read back from the canvas
            r = pg.evaluate(f"""(() => {{ const p = {UV}, cv = p.cv, c = cv.getBoundingClientRect(), a = p._a, g = cv.getContext('2d'), s = cv.width / c.width;
              const box = e => {{ const b = e.getBoundingClientRect(); return {{ l: b.left - c.left, r: b.right - c.left, t: b.top - c.top, b: b.bottom - c.top, w: b.width, h: b.height }}; }};
              g.font = getComputedStyle(cv).font; const title = document.createElement('canvas').getContext('2d'); title.font = '12px ' + getComputedStyle(cv).fontFamily;
              const tw = title.measureText(I18N.t('uv.axis.x')).width, cx = (M.l + a.W - M.r) / 2;
              const ink = b => {{ const w = Math.max(1, Math.round((b.r - b.l) * s)), h = Math.max(1, Math.round((b.b - b.t) * s)), d = g.getImageData(Math.max(0, Math.round(b.l * s)), Math.max(0, Math.round(b.t * s)), w, h).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i] > 40) n++; return n; }};
              const l1 = p.leg.textContent.trim() ? box(p.leg) : null, l2 = p.leg2.textContent.trim() ? box(p.leg2) : null;
              return {{ W: a.W, tl: cx - tw / 2, tr: cx + tw / 2, tt: a.H - 6 - 12, l1, l2, ink2: l2 ? ink(l2) : 0, L1: l1 ? p.leg.textContent : '', ov: l1 ? p.leg.scrollWidth - p.leg.clientWidth : 0 }}; }})()""")
            assert r["l1"] and r["l2"], r
            l1 = r["l1"]
            assert l1["r"] <= r["tl"] - 4 or l1["l"] >= r["tr"] + 4 or l1["b"] <= r["tt"], ("the bottom legend overlaps the axis title", r)
            assert r["ink2"] <= 3, ("the boxed legend sits over the curve", r)
            if SHOTS: pg.evaluate(f"{UV}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 2); pg.screenshot(path=f"{SHOTS}/uv-legends-{width}.png")
        step(f"{width}px: the legends stay clear of the axis title and of the curve", clear)
        step(f"{width}px: no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
    return r

SHOTS and os.makedirs(SHOTS, exist_ok=True)
rs = [run(1400, 8886), run(390, 8887)]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
rs[-1].report()
