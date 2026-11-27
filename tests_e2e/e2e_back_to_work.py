"""«Back to work!» after «Sample table» gives the panels back once, in their final size and place (motion on, Chromium, 1400 and 390 px, synthetic files).
Measured from the click to 700 ms later (the entry animations last 200 ms), in the page, one sample per animation frame:
  - draws: every panel canvas is cleared by exactly one draw (CanvasRenderingContext2D.clearRect on the canvases of #dpanels), also not before the click's own frame;
  - layout shift: the sum of the 'layout-shift' entries (PerformanceObserver, input-excluded) is below 0.001;
  - geometry: from the first frame in which every panel has a size, left/top/width/height (getBoundingClientRect, viewport coordinates) move by at most 0.5 px;
  - scroll: scrollY is the one the page had before «Sample table» in the first frame already and never changes (the old code painted one frame at the top first);
  - no second entry: no «tr-drop» animation starts on a panel and the opacity of every panel stays 1 (the old code replayed the fade and the 6 px drop when the class opn-rev left)."""
# covers: mzlab/web/area-lavoro.js, mzlab/web/apertura-area.js, mzlab/web/transizioni.css, mzlab/web/eventi.js, mzlab/web/shell.js
import sys, os, shutil, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
import synth

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))

tmp = Path(tempfile.mkdtemp(prefix="backwork_"))
FILES = []
for k, n in enumerate(["S_t000min", "S_t010min", "S_t030min"]):
    p = tmp / f"{n}.mzML"; synth.write_full(p, 10 * k, seed=k + 1); FILES.append(str(p))

# installed once the panels are up; run(ms) clicks «Back to work!» and samples every frame for ms
MEASURE = r"""(() => {
  const L = window.__bw = {draws: new Map(), shifts: 0, anim: 0};
  const clear = CanvasRenderingContext2D.prototype.clearRect;
  CanvasRenderingContext2D.prototype.clearRect = function (...a) { const c = this.canvas; if (L.on && c && c.closest && c.closest('#dpanels')) L.draws.set(c, (L.draws.get(c) || 0) + 1); return clear.apply(this, a); };
  new PerformanceObserver(l => { for (const e of l.getEntries()) if (L.on && !e.hadRecentInput) L.shifts += e.value; }).observe({type: 'layout-shift'});
  document.addEventListener('animationstart', e => { if (L.on && e.animationName === 'tr-drop' && e.target.closest && e.target.closest('.pnl')) L.anim++; }, true);
  window.__bwRun = ms => new Promise(done => {
    const t0 = performance.now(), frames = [];
    L.on = true; L.draws.clear(); L.shifts = 0; L.anim = 0;
    document.querySelector('#backwork').click();
    const f = () => {
      const ps = [...document.querySelectorAll('#dpanels .pnl')];
      frames.push({scroll: scrollY, panels: ps.map(p => { const b = p.getBoundingClientRect(); return {x: b.x, y: b.y, w: b.width, h: b.height, o: +getComputedStyle(p).opacity}; })});
      if (performance.now() - t0 < ms) requestAnimationFrame(f);
      else { L.on = false; done({frames, draws: [...L.draws.values()], canvases: document.querySelectorAll('#dpanels .pnl canvas').length, shifts: L.shifts, anim: L.anim}); }
    };
    requestAnimationFrame(f);
  });
})()"""

def measure(width, height):
    r = Run(port=8975, wd=f"/tmp/wd_back_to_work_{width}")
    try:
        with sync_playwright() as p:
            pg = r.page(p, motion=True, viewport={"width": width, "height": height})
            pg.wait_for_selector("#drop", state="attached")
            try: wait_for_js(pg, "window.qqReady === true", 60000)
            except Exception: raise AssertionError(f"the page did not start: {r.errs[:6]}")
            pg.set_input_files("#pick", FILES)
            wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === 3", 60000)
            pg.wait_for_selector("#dpanels .pnl.chrom canvas", timeout=60000)
            pg.evaluate("document.querySelector('#np-spec').click()")          # a second panel: the page is taller than the window
            wait_for_js(pg, "document.querySelectorAll('#dpanels .pnl canvas').length >= 2", 30000)
            ready(pg)
            pg.evaluate("(() => { const m = document.documentElement.scrollHeight - innerHeight; window.scrollTo(0, Math.min(250, m)); })()"); frames(pg, 3)
            pg.evaluate("document.querySelector('#samptab').addEventListener('click', () => { window.__y0 = scrollY; }, true)")
            y0 = pg.evaluate("scrollY")
            assert y0 >= 100, f"the page must scroll for this test ({y0} px): the scroll that comes back is part of what is measured"
            if width < 900: pg.click("#sb-toggle"); frames(pg, 6)                # the side panel is a sheet on a phone
            pg.click("#samptab"); pg.wait_for_selector("#wtable .samphead", state="visible", timeout=10000); ready(pg)
            pg.wait_for_selector("#backwork", state="visible")
            y = pg.evaluate("window.__y0")                                      # the scroll applyView keeps for the way back
            pg.evaluate(MEASURE)
            m = pg.evaluate("window.__bwRun(700)")
            assert not r.errs or all(e[0] not in ("pageerror",) for e in r.errs), f"page errors: {r.errs}"
            return y, m
    finally:
        r.close()

def check(width, height):
    y, m = measure(width, height)
    fr = [f for f in m["frames"] if f["panels"] and all(q["w"] > 0 and q["h"] > 0 for q in f["panels"])]
    assert fr, "the panels never showed"
    assert m["canvases"] >= 2, f"two panels expected, {m['canvases']} canvases"
    assert m["draws"] == [1] * m["canvases"], f"draws per panel canvas: {m['draws']} (every one must be 1, {m['canvases']} canvases)"
    assert m["shifts"] < 0.001, f"layout shift {m['shifts']:.4f}"
    assert m["anim"] == 0, f"{m['anim']} entry animations (tr-drop) started on the panels"
    ops = [q["o"] for f in fr for q in f["panels"]]
    assert min(ops) == 1, f"a panel faded again (opacity {min(ops)})"
    sc = [f["scroll"] for f in fr]
    assert all(abs(s - y) <= 0.5 for s in sc), f"scrollY {min(sc)}..{max(sc)} after the click, {y} before it"
    for i in range(len(fr[0]["panels"])):
        for k in "xywh":
            v = [f["panels"][i][k] for f in fr]
            assert max(v) - min(v) <= 0.5, f"panel {i} {k} moved {max(v) - min(v):.1f} px between frames"
    print(f"  {width} px: draws {m['draws']}, layout shift {m['shifts']:.5f}, {len(fr)} frames, scrollY {y}")

step("1400 px: the panels come back once, in place (draws, layout shift, scroll, no second fade)", lambda: check(1400, 700))
step("390 px: the panels come back once, in place (draws, layout shift, scroll, no second fade)", lambda: check(390, 700))
shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
sys.exit(1 if bad else 0)
