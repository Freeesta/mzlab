"""Header walk (4 of 4): movement over time, on pages of their own: with motion on, coming back to Data does not slide; Data and Drawing are never taller than the window; the first file keeps the width of the side panel; the Flow bar does not jump when its late part arrives; no console error, no failed request (see e2e_giro_intestazione.py)."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from lib_giro import *

steps = []
step = make_step(steps)
r = Run(port=8996, wd="/tmp/wd_giro_intestazione_tempo")
VIEWS = ["draw", "flow", "tpmine"]
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        wait_for_js(pg, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(pg)
        wait_for_js(pg, "!!document.querySelector('#nav [data-v=flow]') && !!document.querySelector('#nav [data-v=tpmine]')")
        # ---- movement over time, on pages of their own (with motion on, the first opening of Flow, a high-resolution file)
        SAMPLE = """(sels) => { window.__mv = {}; (function f() { for (const [k, s] of Object.entries(sels)) { const e = document.querySelector(s); if (e && e.offsetWidth) (window.__mv[k] = window.__mv[k] || []).push(Math.round(e.getBoundingClientRect().x * 10) / 10); } window.__go !== false && requestAnimationFrame(f); })(); window.__go = true; }"""
        def spread(pg, key):
            xs = pg.evaluate("(k) => window.__mv[k] || []", key)
            assert xs, f"{key}: never seen"
            return max(xs) - min(xs), sorted(set(xs))[:6]
        def fresh(motion=False):
            q = r.page(p, motion=motion); q.set_viewport_size({"width": 1440, "height": 900})
            wait_for_js(q, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(q); return q

        def no_slide_back():
            q = fresh(True); stage(q, [mz("B_FullMass-t15")]); open_first(q); ready(q)
            for v in ("draw", "flow"):
                view(q, v); q.evaluate(SAMPLE, {"pane": "#sb-pane", "work": "#v-workspace"})
                q.click('#nav [data-v="data"]'); wait_for_js(q, "S.view === 'data'"); hold(q, 1200)          # only time shows a slide
                for k in ("pane", "work"):
                    d, xs = spread(q, k); assert d <= MOVE_PX, f"back from {v}: {k} moved {d} px over time {xs}"
        step("one sample, motion on: coming back to Data the side panel and the work area do not slide", no_slide_back)

        def not_taller():
            q = fresh(); bad = []
            for v in ("data", "draw"):
                view(q, v); q.evaluate("window.scrollTo(0, 0)")
                over = q.evaluate("document.documentElement.scrollHeight - innerHeight")
                if over > 0: bad.append(f"{v}: {over} px taller than the window")
            stage(q, [mz("B_FullMass-t15")]); open_first(q); ready(q)
            over = q.evaluate("document.documentElement.scrollHeight - innerHeight")
            if over > 0: bad.append(f"data with a sample: {over} px taller")
            assert not bad, " | ".join(bad)
        step("Data and Drawing are never taller than the window (no page scroll bar)", not_taller)

        def keeps_width():
            q = fresh(); D = synth("giro_larg"); load(q, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
            w, dw = q.evaluate("document.querySelector('#dfiles').getBoundingClientRect().width"), q.evaluate("(BANCO.on() ? 340 : 280)")
            assert w >= dw - 1, f"the first file narrowed the side panel to {w} px (landing width {dw})"
        step("the first file keeps the landing width of the side panel", keeps_width)

        def flow_bar_still():
            q = fresh(); view(q, "flow")
            q.evaluate(SAMPLE, {"run": "#flx-run", "cli": "#flx-cli-show", "more": "#flx-more"})
            hold(q, 4000)          # the late modules arrive within a second or two
            for k in ("run", "cli", "more"):
                d, xs = spread(q, k); assert d <= MOVE_PX, f"{k} moved {d} px when the late part arrived {xs}"
        step("Flow, first opening: Run, Terminal and More do not shift when the late part of the bar arrives", flow_bar_still)

        def clean():
            errs = [e for e in r.errs if "wasm-feat/manifest.json" not in str(e)          # the optional modules are not built for the e2e site (tools/prepara_sito.py)
                    and not (e[0] == "console.error" and "404" in str(e[1]))               # its console twin
                    and "allow-scripts and allow-same-origin" not in str(e)]               # Chromium's note on the sandboxed Ketcher iframe (docs/agenti/drawing.md)
            assert not errs, errs[:5]
        step("no console error, no failed request", clean)
finally:
    try: r.close()
    except Exception: pass
finish(steps)
