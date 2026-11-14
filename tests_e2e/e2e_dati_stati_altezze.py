"""Dati keeps the same heights in every state (work, add files, back to work): the toolbar and the file panel sit at the same top, and the panels do not slide when «Back to work!»
returns them (their first frame is their final place), with no layout shift on «+» and on «Back to work!». Measured at 1000x570 and 1000x700, device scale factor 2."""
# covers: mzlab/web/area-lavoro.js, mzlab/web/app.css, mzlab/web/transizioni.css
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
FILES = sorted(str(f) for f in EX.glob("FullScan*.mzML"))[:5]
WORK = "document.getElementById('v-workspace').dataset.work"
TOP = "(q => { const e = document.querySelector(q); return e && !e.hidden ? Math.round(e.getBoundingClientRect().top * 10) / 10 : null; })"
OBS = """() => { window.__ls = []; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__ls.push(e.value); }).observe({type: 'layout-shift'}); }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + " ".join(str(e).split())[:500]))
r = Run(port=8968, wd="/tmp/wd_dati_stati")
try:
    with sync_playwright() as p:
        for h in (570, 700):
            pg = r.page(p, motion=True); pg.set_viewport_size({"width": 1000, "height": h})          # a fresh page per size: a resized one keeps its scroll
            pg.wait_for_selector("#drop"); wait_for_js(pg, "window.qqReady === true", 60000)
            pg.set_input_files("#pick", FILES)
            wait_for_js(pg, f"UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === {len(FILES)}", 90000)
            pg.click("#flst .fl .nm >> nth=0"); pg.wait_for_selector(".pnl.chrom canvas", timeout=60000); ready(pg); pg.evaluate(OBS)
            def tops(): return pg.evaluate(f"(() => {{ const t = {TOP}; return {{tools: t('#tools'), files: t('#dfiles')}}; }})()")
            def states(h=h):
                pg.evaluate("window.__ls = []")
                work = tops(); pg.evaluate("document.querySelector('#addf').click()"); wait_for_js(pg, f"{WORK} === 'add'"); add = tops()
                assert add == work, (h, "work vs add files", work, add)
                pg.click("#backwork"); wait_for_js(pg, f"{WORK} === 'panels'")
                first = pg.evaluate(f"{TOP}('#dpanels .pnl')"); ready(pg); wait_still(pg, "#dpanels .pnl")
                last = pg.evaluate(f"{TOP}('#dpanels .pnl')"); back = tops()
                assert back == work, (h, "after back to work", work, back)
                assert abs(first - last) <= 0.5, (h, "the panel slid after «Back to work!»", first, last)
                ls = pg.evaluate("window.__ls"); assert sum(ls) <= 0.005, (h, "layout shift", ls)
            step(f"{h}px: same toolbar and file-panel tops in work, add files and back; the panels do not slide", states)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s.startswith("FAIL") for _, s in steps) else 0)
