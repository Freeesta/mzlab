"""«Back to work!» and Esc of the landing reopened by «+» give the page back exactly as it was: same panels (position, size, zoom), same side panel (open or shut, side), same scroll."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_torna_ai_dati.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:600]))
SNAP = """() => ({
  dv: !document.querySelector('#v-data').hidden, bar: BARRA.open, side: document.querySelector('#v-data').classList.contains('sb-right'), y: Math.round(scrollY),
  panels: E.panels.map(p => { const r = p.el.getBoundingClientRect(); return [p.type, Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height), JSON.stringify(p.zoom)]; })})"""
r = Run(port=8853, wd="/tmp/wd_torna")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")])
        pg.set_viewport_size({"width": 1500, "height": 700})
        pg.click("text=Carica dati"); ready(pg)
        for sel in ["#np-chrom", "#np-spec"]: pg.click(sel); frames(pg, 6)
        pg.evaluate("window.__fp = E.panels.find(q => q.type === 'chrom'); __fp.zoom = [5, 12]; draw(__fp)"); frames(pg, 4)
        def roundtrip(name, leave, **setup):
            def go():
                pg.evaluate(f"BARRA.{'show' if setup.get('bar', True) else 'close'}" + ("('file')" if setup.get("bar", True) else "(false)")); frames(pg, 4)
                if setup.get("right"): pg.evaluate("BARRA.setSide ? BARRA.setSide('right') : 0"); frames(pg, 4)
                pg.evaluate("window.scrollTo(0, 400)"); frames(pg, 3)
                before = pg.evaluate(SNAP)
                # not pg.click: Playwright would first scroll the button into view and move the page
                pg.evaluate("document.querySelector('#addf').click()"); pg.wait_for_selector("#backwork", state="visible"); frames(pg, 4)
                assert pg.is_visible("#start") and not pg.is_visible("#dpanels .pnl"), "the loading screen is not showing"      # the landing now sits inside the work area (#v-data stays shown)
                (pg.keyboard.press("Escape") if leave == "Escape" else pg.click(leave)); frames(pg, 6); fermo(pg, "#dpanels"); frames(pg, 4)
                after = pg.evaluate(SNAP)
                assert before == after, f"\nbefore {before}\nafter  {after}"
            step(name, go)
        roundtrip("Back to the data, side panel open", "#backwork")
        roundtrip("Esc, side panel open", "Escape")
        roundtrip("Back to the data, side panel shut", "#backwork", bar=False)
        roundtrip("Esc, side panel shut", "Escape", bar=False)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
