"""A chromatogram trace keeps exactly the same line width while the pointer is over it (TIC and XIC overlay): the widths of every stroke of the
panel and its canvas pixels are the same before and during a real mouse hover; a legend row under the pointer fades the others but changes no width."""
# covers: mzlab/web/grafici.js, mzlab/web/sim-ui.js, mzlab/web/dad-mrm.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_traccia_spessore.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("traccia_spessore")
r = Run(port=8884, wd="/tmp/wd_traccia_spessore")
TIC = "E.panels.find(q => q.type === 'chrom' && q.tab === E.tab)"
XIC = "E.panels.find(q => q.type === 'xic')"
# records, per redraw of a panel, the line width of every stroke on its canvas (a hook on the 2D context: the draw-call parameters)
HOOK = """() => {
  if (window.__lw) return;
  window.__lw = { on: false, w: [] };
  const st = CanvasRenderingContext2D.prototype.stroke;
  CanvasRenderingContext2D.prototype.stroke = function (...a) { if (window.__lw.on) window.__lw.w.push(+this.lineWidth.toFixed(3) + '@' + this.globalAlpha); return st.apply(this, a); };
}"""
def widths(pg, panel):
    """redraws the panel once (the pointer stays where it is) and returns the sorted line widths of its strokes and a digest of its canvas pixels"""
    return pg.evaluate(
        f"""async () => {{ const p = {panel}, L = window.__lw; L.w = []; L.on = true; try {{ await draw(p); }} finally {{ L.on = false; }}
      const c = p.cv, d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let h = 0; for (let i = 0; i < d.length; i += 7) h = (h * 31 + d[i]) | 0;
      return {{ w: L.w.slice().sort(), px: h }}; }}""")
def over(pg, panel):
    """moves the real mouse onto the highest visible point of the first trace of the panel (it comes in from the side, so the tooltip logic runs)"""
    x, y = pg.evaluate(f"""() => {{ const p = {panel}, a = p._a, b = p.cv.getBoundingClientRect(), S = a.sr[0];
      let j = 0, ym = -1; for (let i = 0; i < S.y.length; i++) if (S.y[i] > ym && S.y[i] <= a.ymax * 0.9 && S.x[i] >= a.x0 && S.x[i] <= a.x1) {{ ym = S.y[i]; j = i; }}
      return [b.left + a.X(S.x[j]), b.top + a.Y(ym)]; }}""")
    pg.mouse.move(x - 40, y + 30); pg.mouse.move(x, y, steps=4); frames(pg, 4)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        pg.evaluate("openXic(null,{})"); pg.wait_for_selector("#xic-mz", state="visible"); pg.fill("#xic-mz", "364.4"); pg.click("#xic-go"); wait_for_js(pg, "E.panels.some(q => q.type === 'xic')"); frames(pg, 4); ready(pg)
        pg.evaluate(HOOK)
        def probe(panel, label):
            pg.evaluate(f"{panel}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 4)
            pg.mouse.move(2, 2); frames(pg, 3)                                   # pointer away from the plot
            base = widths(pg, panel); assert base["w"], f"{label}: the hook saw no stroke"
            over(pg, panel)
            assert pg.evaluate(f"!!{panel}.tip && !{panel}.tip.hidden"), f"{label}: the hover tooltip is not shown (the pointer is not over the trace)"
            hov = widths(pg, panel)                                              # a redraw with the pointer on the trace
            assert hov["w"] == base["w"], f"{label}: stroke widths change on hover {sorted(set(base['w']))} -> {sorted(set(hov['w']))}"
            assert hov["px"] == base["px"], f"{label}: canvas pixels change on hover"
            frames(pg, 3)
            again = widths(pg, panel); assert again["w"] == base["w"], f"{label}: width changes after the frame that follows the hover"
            pg.mouse.move(2, 2); frames(pg, 3)
        step("TIC: the line width is the same with the pointer over the trace", lambda: probe(TIC, "TIC"))
        step("XIC: the line width is the same with the pointer over the trace", lambda: probe(XIC, "XIC"))
        def legend():
            pg.evaluate(f"{XIC}.el.scrollIntoView({{block: 'center'}})"); frames(pg, 3)
            base = widths(pg, XIC)
            key = pg.evaluate(f"(() => {{ const p = {XIC}; const s = p._a ? p._a.sr[0] : null; return s ? (s.tk || s.tf) : null; }})()")
            assert key, "no trace key to light"
            pg.evaluate(f"SIMUI.lit({XIC}, {key!r})"); frames(pg, 4)              # what a pointer over a row of the channel list does
            lit = widths(pg, XIC)
            assert [w.split("@")[0] for w in lit["w"]] == [w.split("@")[0] for w in base["w"]], f"a row under the pointer changes widths: {sorted(set(base['w']))} -> {sorted(set(lit['w']))}"
            pg.evaluate(f"SIMUI.lit({XIC}, null)"); frames(pg, 3)
        step("a legend row under the pointer changes no line width", legend)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
