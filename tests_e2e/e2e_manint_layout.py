"""Layout of the card «Manual integrations» under the chart (synthetic SIM .wiff of tools/wiff_sim.py, no laboratory data): the card starts right under the
chart panel (no reserved gap) and its table is as tall as its rows (header + one line per integration) up to 60 % of the viewport, then scrolls inside;
closed it stays one compact line; the chart does not move when a row is added. Checked with 1, 5 and 30 rows, at 1500 px and 390 px, and in full screen.
MZLAB_SHOTS=<dir> also saves a screenshot per case. Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_manint_layout.py"""
# covers: mzlab/web/integrazioni-manuali.js, mzlab/web/app.css
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="manint_layout_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim3.json").read_text())
SHOTS = os.environ.get("MZLAB_SHOTS")
throw = lambda m: (_ for _ in ()).throw(AssertionError(m))
SHOWN = "(() => { const c = document.querySelector('#manint'); return !!c && !c.hidden && c.querySelectorAll('tbody tr').length === %d; })()"
# integrates the window [a, b] (min) of ion `ion` of sample `name` on the SIM panel, as the end of a drag of the user
INTEG = """([name, ion, a, b]) => { const p = E.panels.find(q => q.type === 'mrm' && q._a);
  const f = E.files.find(x => x.label === name || x.file.startsWith(name + '.')); addInt(p, p._a.sr.filter(x => x.k === f.k)[ion], a, b, false); }"""
# [bottom of the last chart panel, card top, table box top, table box height, one row, header, document width, viewport height, panel top]
GEO = """(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a).el.getBoundingClientRect(), lb = document.getElementById('dpanels').lastElementChild.getBoundingClientRect().bottom, c = document.getElementById('manint').getBoundingClientRect(),
  w = document.getElementById('manint-b'), r = w.getBoundingClientRect(), tr = w.querySelector('tbody tr'), th = w.querySelector('thead');
  return { pb: lb, pt: p.top, ct: c.top, wt: r.top, wh: r.height, row: tr ? tr.getBoundingClientRect().height : 0, head: th ? th.getBoundingClientRect().height : 0,
    sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth, vh: innerHeight, scrolls: w.scrollHeight > w.clientHeight + 1, hidden: w.hidden, ch: c.height }; })()"""

r = Run(port=8877, wd="/tmp/wd_manint_layout")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim3.wiff", D / "sim3.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); wait_for_js(pg, "E.files.length >= 3", timeout=120000)
        open_first(pg)
        pg.click("#np-mrm"); wait_for_js(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        pg.evaluate("(() => { E.browse = false; E.files.forEach(f => { f.vis = true; }); const p = E.panels.find(q => q.type === 'mrm'); p.tr = ''; draw(p); })()")
        wait_for_js(pg, "(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); return !!p && new Set(p._a.sr.map(s => s.k)).size === E.files.length; })()", timeout=60000)
        c = TRUE["picchi"][3][0][0]
        names = ["Sample_A", "Sample_B", "Sample_C"]
        made = [0]
        def grow(n):
            # windows apart on different traces: an overlapping window replaces the one under it
            while made[0] < n:
                i = made[0]; s, ion, k = names[i % 3], (i // 3) % 7, i // 21
                pg.evaluate(INTEG, [s, ion, c - 0.15 + 0.6 * k, c + 0.15 + 0.6 * k]); made[0] += 1
            wait_for_js(pg, SHOWN % n, timeout=60000)
            pg.wait_for_timeout(300)
        def shot(tag):
            if SHOTS: Path(SHOTS).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(SHOTS) / f"manint_{tag}.png"), full_page=True)
        need = lambda cond, m: cond or throw(m)
        geo = {}
        def rows_grow():
            for n in (1, 5, 30):
                grow(n); g = geo[n] = pg.evaluate(GEO); shot(f"{n}rows")
                need(abs(g["ct"] - g["pb"]) <= 20, ("the card starts right under the chart panel", n, g["pb"], g["ct"]))
                need(abs(g["wt"] - g["ct"]) <= 60, ("the table follows the heading", n, g))
                need(g["sw"] <= g["cw"], ("no horizontal page scroll", g))
            # the box is the header plus one line per row while it fits, the cap (60 % of the viewport) beyond
            for n in (1, 5):
                g = geo[n]; want = g["head"] + n * g["row"]
                need(not g["scrolls"] and abs(g["wh"] - want) <= 6, ("height = header + rows, no scroll", n, g["wh"], want))
            need(geo[5]["wh"] > geo[1]["wh"] + 3 * geo[1]["row"], ("grows with the rows", geo[1]["wh"], geo[5]["wh"]))
            g = geo[30]; need(g["scrolls"] and g["wh"] <= g["vh"] * 0.6 + 2 and g["wh"] > geo[5]["wh"], ("30 rows: capped at 60 % of the viewport and scrolling inside", g))
        step("the table starts under the chart and grows with its rows (1, 5, 30), capped with an inner scroll", rows_grow)
        def no_shift():
            pg.evaluate("window.scrollTo(0, 0)")
            before = pg.evaluate(GEO)
            made[0] = 30
            pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'mrm' && q._a); delInts(p, p.ints.slice(5)); })()"); wait_for_js(pg, SHOWN % 5, timeout=60000)
            g = pg.evaluate(GEO)
            need(g["pt"] == before["pt"] and abs(g["pb"] - before["pb"]) < 1, ("the chart does not move when rows come and go", before, g))
            made[0] = 5; grow(6)
            h = pg.evaluate(GEO)
            need(abs(h["pb"] - g["pb"]) < 1 and abs(h["ct"] - g["ct"]) < 1, ("a row added moves nothing above the table", g, h))
            need(abs((h["wh"] - g["wh"]) - g["row"]) <= 3, ("the new row is one row of height", g["wh"], h["wh"], g["row"]))
        step("a row added or removed moves nothing above the table", no_shift)
        def collapsed():
            grow(30) if made[0] < 30 else None
            if pg.evaluate("document.getElementById('manint-b').hidden"): pg.click("#manint .manint-tg")
            open_h = pg.evaluate(GEO)["ch"]
            pg.click("#manint .manint-tg"); g = pg.evaluate(GEO); shot("collapsed")
            need(g["hidden"] and g["ch"] < 80 and g["ch"] < open_h, ("collapsed = one compact line", g["ch"], open_h))
            need(abs(g["ct"] - g["pb"]) <= 20, ("still right under the chart", g))
            pg.click("#manint .manint-tg")
        step("closed it is one compact line, still under the chart", collapsed)
        def narrow():
            pg.set_viewport_size({"width": 390, "height": 844}); pg.wait_for_timeout(500)
            for n in (5, 30):
                if made[0] > n:
                    pg.evaluate(f"(() => {{ const p = E.panels.find(q => q.type === 'mrm' && q._a); delInts(p, p.ints.slice({n})); }})()"); made[0] = n
                else: grow(n)
                wait_for_js(pg, SHOWN % n, timeout=60000); pg.wait_for_timeout(300)
                g = pg.evaluate(GEO); shot(f"390px_{n}rows")
                need(abs(g["ct"] - g["pb"]) <= 20, ("390 px: right under the chart", n, g))
                need(g["sw"] <= g["cw"], ("390 px: no horizontal page scroll", g))
                need(g["wh"] <= g["vh"] * 0.6 + 2, ("390 px: capped", g))
            pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(400)
        step("at 390 px: under the chart, capped, the table scrolls inside (no page scroll sideways)", narrow)
        def fullscreen():
            try:
                pg.evaluate("document.documentElement.requestFullscreen()"); pg.wait_for_timeout(600)
            except Exception:
                return          # headless Chromium may refuse: the layout rules above are the same ones
            if not pg.evaluate("!!document.fullscreenElement"): return
            g = pg.evaluate(GEO); shot("fullscreen")
            need(abs(g["ct"] - g["pb"]) <= 20 and g["wh"] <= g["vh"] * 0.6 + 2, ("full screen", g))
            pg.evaluate("document.exitFullscreen()")
        step("in full screen", fullscreen)
        step("no page error", lambda: (lambda e: e and throw(e))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s.startswith("FAIL") for _, s in steps) else 0)
