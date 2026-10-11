"""A3: the header stays in view while the page scrolls; changing view or tab never scrolls the page."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_fisso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8837, wd="/tmp/wd_fisso")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1280, "height": 700})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(600)
        for sel in ["#np-chrom", "#np-spec", "#np-chrom"]: pg.click(sel); pg.wait_for_timeout(900)
        top = lambda: pg.evaluate("document.querySelector('header').getBoundingClientRect().top")
        def sticky():
            assert pg.evaluate("document.documentElement.scrollHeight") > 1100, pg.evaluate("document.documentElement.scrollHeight")
            pg.evaluate("window.scrollTo(0, 700)"); pg.wait_for_timeout(300)
            assert pg.evaluate("scrollY") > 300 and top() == 0, (pg.evaluate("scrollY"), top())
            assert pg.evaluate("document.querySelector('header').classList.contains('scrolled')")
            hh = pg.evaluate("document.querySelector('header').offsetHeight")
            drawer(pg, pin=False); assert pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().top") >= hh, "the file list sits below the header"; pg.evaluate("BARRA.close(false)")
            pg.evaluate("window.scrollTo(0, 0)"); pg.wait_for_timeout(200); assert not pg.evaluate("document.querySelector('header').classList.contains('scrolled')")
        step("header sticky, shadow only when scrolled, file list below it", sticky)
        def noscroll():
            pg.evaluate("window.__sc = 0; const o = window.scrollTo; window.scrollTo = function () { if (!/UtilityScript/.test(new Error().stack)) window.__sc++; return o.apply(window, arguments); }   // calls made by Playwright itself are not the page's")
            for v in ("draw", "theory", "data"):
                pg.click(f"#nav [data-v={v}]"); pg.wait_for_timeout(500)
            assert pg.evaluate("scrollY") == 0 and top() == 0 and pg.evaluate("window.__sc") == 0, (pg.evaluate("scrollY"), pg.evaluate("window.__sc"))
            pg.evaluate("window.scrollTo(0, 500)"); pg.evaluate("window.__sc = 0"); pg.wait_for_timeout(200); y0 = pg.evaluate("scrollY")
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(300); pg.click("#nav [data-v=data]"); pg.wait_for_timeout(600)
            assert pg.evaluate("window.__sc") == 0 and top() == 0, pg.evaluate("window.__sc")
            pg.evaluate("setTab('ms2')"); pg.wait_for_timeout(1500); pg.evaluate("setTab('full')"); pg.wait_for_timeout(800)
            assert pg.evaluate("window.__sc") == 0, "switching Full Scan / MS2 does not scroll"
        step("Dati from Disegno/Teoria and tab changes: no scrolling", noscroll)
        def layers():
            pg.evaluate("window.scrollTo(0, 400)"); pg.wait_for_timeout(300)
            hh = pg.evaluate("document.querySelector('header').offsetHeight")
            # right-click on the chart right under the header: the menu is drawn above it
            xy = pg.evaluate("(() => { const c = Q('.pnl.chrom canvas'), q = c.getBoundingClientRect(); return [q.left + q.width * 0.5, Math.max(q.top + 40, 0)]; })()")
            pg.mouse.click(xy[0], max(xy[1], hh + 5), button="right"); pg.wait_for_timeout(400)
            z = pg.evaluate("[+getComputedStyle(Q('#ctx')).zIndex, +getComputedStyle(document.querySelector('header')).zIndex]"); assert z[0] > z[1], z
            pg.keyboard.press("Escape")
            maxz = pg.evaluate("Math.max(...E.panels.map(p => +p.el.style.zIndex || 0))"); assert maxz < z[1], (maxz, z)
        step("menus above the header, panels below it", layers)
        def full():
            pg.evaluate("window.scrollTo(0, 300)"); pg.locator('.pnl [data-a="max"]').first.click(); pg.wait_for_timeout(500)
            t = pg.evaluate("Q('.pnl.max').getBoundingClientRect().top"); hh = pg.evaluate("document.querySelector('header').getBoundingClientRect().bottom")
            assert t >= hh - 1 and top() == 0, (t, hh)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
        step("full-screen panel does not hide the header", full)
        def goto_panel():
            pg.evaluate("window.scrollTo(0, 0)"); pg.wait_for_timeout(200)
            n = pg.evaluate("tabPanels().length"); pg.keyboard.press(str(min(n, 3))); pg.wait_for_timeout(1200)
            t = pg.evaluate("Q('.pnl.act, .pnl.active, .pnl[data-active]')?.getBoundingClientRect().top ?? (E.active && E.active.el.getBoundingClientRect().top)")
            hh = pg.evaluate("document.querySelector('header').offsetHeight"); assert t is None or t >= hh - 2, (t, hh)
        step("number key scrolls to a panel below the header", goto_panel)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
