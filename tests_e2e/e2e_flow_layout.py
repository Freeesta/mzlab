"""Flow opens without a jump and with the same side gutter as the rest of the page (docs/agenti/flow-ux.md, «Margini e primo disegno»): at 1000x570, 1440x900 and a phone
the title, the status line, the canvas box and the cards keep the gutter of the header, and from the click on the tab to the settled page nothing moves: no layout shift of
the page, and the first card is where it stays (no slide-in, no animated fit at the first draw). Run from the repo root: python3 tests_e2e/e2e_flow_layout.py"""  # covers: flow/
import subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, wait_for_js; PORT = free_port()
TAB = '#nav button[data-v="flow"]'
VIEWPORTS = [(1000, 570), (1440, 900), (390, 844)]
SHIFT_BUDGET = 0.005        # a whole page of layout-shift score, counted from the click on the tab (the tab bar of the header is not in it: it is there before)
MOVE_BUDGET = 1             # px a card may move between its first frame and the settled page
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))
OBSERVE = """() => { window.__ls = []; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__ls.push({v: e.value, n: e.sources.map(s => s.node && (s.node.id || s.node.className || s.node.nodeName)).join(',')}); }).observe({type: 'layout-shift', buffered: false});
  window.__fr = []; const t0 = performance.now(); const tick = () => { const c = document.querySelector('#v-flow .fln'); if (c && getComputedStyle(c).visibility !== 'hidden') { const r = c.getBoundingClientRect(); window.__fr.push([Math.round(performance.now() - t0), r.x, r.y]); } if (performance.now() - t0 < 1500) requestAnimationFrame(tick); }; requestAnimationFrame(tick); }"""
LEFT = """() => { const x = s => { const e = document.querySelector(s); return e ? Math.round(e.getBoundingClientRect().left * 10) / 10 : null; };
  const h = getComputedStyle(document.querySelector('header')); return {gut: parseFloat(h.paddingLeft), title: x('#flx-root .flx-bar h2'), toast: x('#flx-root .flx-toast'), canvas: x('#flx-root .fl-view'), card: x('#flx-root .fln'), bar: x('#flx-root .flx-bar')}; }"""

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        for w, h in VIEWPORTS:
            c = b.new_context(viewport={"width": w, "height": h}, device_scale_factor=2, reduced_motion="no-preference"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector(TAB, timeout=120000)
            wait_for_js(pg, "typeof FLOW !== 'undefined' && typeof FLOW.shell === 'function'", 60000)
            pg.evaluate(OBSERVE); pg.click(TAB); pg.wait_for_selector("#flx-root .fln", timeout=30000)
            wait_for_js(pg, "performance.now() > 0 && window.__fr.length > 0 && window.__fr[window.__fr.length - 1][0] > 1400", 20000)
            def gutter(w=w):
                g = pg.evaluate(LEFT)
                for k in ("title", "canvas", "card"):
                    assert g[k] is not None and g[k] >= g["gut"] - 0.5, (w, k, g)
                if g["toast"] is not None: assert g["toast"] >= g["gut"] - 0.5, (w, "toast", g)
            step(f"{w}px: title, status line, canvas and cards keep the gutter of the header", gutter)
            def shifts(w=w):
                ls = pg.evaluate("window.__ls"); tot = sum(e["v"] for e in ls)
                assert tot <= SHIFT_BUDGET, (w, round(tot, 4), ls)
            step(f"{w}px: no layout shift while Flow opens", shifts)
            def still(w=w):
                fr = pg.evaluate("window.__fr"); x, y = fr[-1][1], fr[-1][2]
                mv = max(max(abs(f[1] - x), abs(f[2] - y)) for f in fr)
                assert mv <= MOVE_BUDGET, (w, round(mv, 1), fr[:3], fr[-1])
            step(f"{w}px: the first card is where it stays", still)
            c.close()
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
