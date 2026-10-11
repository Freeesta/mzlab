"""Toolbar mode (Solo il selezionato / Tutti sovrapposti): file choosers only where they make sense; 2D/3D switch; loading page keeps the loaded files; empty tab links to loading page."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_modo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8834, wd="/tmp/wd_modo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(500)
        for sel in ["#np-chrom", "#np-spec", "#np-map"]: pg.click(sel); pg.wait_for_timeout(1000)
        dis = lambda s: pg.evaluate("s=>[...document.querySelectorAll(s)].map(x=>x.disabled)", s)
        def allmode():
            pg.click("#fmode [data-m=all]"); pg.wait_for_timeout(600)
            assert all(dis("#fprev,#fnext,#fsel")), dis("#fprev,#fnext,#fsel")
            assert not any(dis("[data-o=k]")) and not any(dis("[data-o=all]"))
        step("Tutti sovrapposti: arrows/selector off, panel file choosers on", allmode)
        def selmode():
            pg.click("#fmode [data-m=sel]"); pg.wait_for_timeout(600)
            assert not any(dis("#fprev,#fnext,#fsel"))
            assert all(dis("[data-o=k]")) and all(dis("[data-o=all]")), (dis("[data-o=k]"), dis("[data-o=all]"))
            pg.click("#fnext"); ready(pg)
            cur = pg.evaluate("E.cur"); assert pg.evaluate("[...document.querySelectorAll('.pnl [data-o=k]')].every(s=>+s.value===E.cur)")
        step("Solo il selezionato: arrows on, per-panel file choosers follow and are off", selmode)
        def norm():
            assert all(dis("[data-o=norm]"))
        step("normalisation off without a reference file", norm)
        def toggle():
            on = lambda: pg.evaluate("[...document.querySelectorAll('[data-o=view]')].map(b=>b.classList.contains('on')+':'+b.dataset.v)")
            assert on() == ["false:3d"], on()                                    # a single «Open in 3D» button, no more 2D/3D switch
            pg.locator("[data-o=view][data-v='3d']").first.click(); pg.wait_for_timeout(1500)
            pg.wait_for_function("document.querySelector('dialog.m3d[open]')", timeout=30000)      # the 3D viewer, a modal over the page
            pg.keyboard.press("Escape"); pg.wait_for_timeout(500)
            assert pg.evaluate("!document.querySelector('dialog.m3d')")
        step("«Open in 3D» button opens the viewer, Esc closes it", toggle)
        def adding():
            drawer(pg); pg.click("#addf"); pg.wait_for_timeout(600)
            assert pg.is_visible("#loaded") and "B_FullMass-t0" in pg.inner_text("#loaded")
            pg.click("#backdata"); pg.wait_for_timeout(600)
            assert pg.is_visible("#v-data")
        step("loading page lists loaded files and goes back", adding)
        def empty():
            pg.evaluate("setTab('ms2',true)"); pg.wait_for_timeout(500)
            pg.click("#tabempty [data-load]"); pg.wait_for_timeout(600)
            assert pg.is_visible("#loaded")
        step("empty tab: button to the loading page", empty)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
