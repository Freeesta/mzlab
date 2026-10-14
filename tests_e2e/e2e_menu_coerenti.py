"""e2e_menu_coerenti: the same graph offers the same actions from the right click, from the keyboard (Shift+F10 / Menu key) and from its bar.
Map: the ids of the menu (data-azione) are the actions of the registry azioni/1 for the view, the buttons of the bar are registered actions.
Spectrum, chromatogram and MRM: Shift+F10 on the focused graph opens the menu of the right click at the same point, with the focus inside; Esc gives it back."""
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
MAP = "E.panels.find(q=>q.type==='map')"
OPEN = "!document.querySelector('#ctx').hidden"
IDS = "[...document.querySelectorAll('#ctx [data-azione]')].map(d=>d.dataset.azione)"
LABELS = "[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1).map(d=>d.textContent)"     # without the first line (where the menu acts)
r = Run(port=8871, wd="/tmp/wd_menu_coerenti")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1000})
        stage(pg, [mz(f) for f in FILES]); pg.click("text=Carica dati"); ready(pg)
        def close():
            pg.keyboard.press("Escape"); attendi(pg, "document.querySelector('#ctx').hidden")
        def keyboard(P):
            pg.evaluate(f"(()=>{{const p={P};p.cv.scrollIntoView({{block:'center'}});p.cv.focus()}})()")
            pg.keyboard.press("Shift+F10"); attendi(pg, OPEN)
            assert pg.evaluate("document.activeElement.closest('#ctx')!==null"), "the focus is in the menu"
        def right_at(P, x, y):
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top]}})()")
            pg.mouse.click(b[0] + x, b[1] + y, button="right"); attendi(pg, OPEN)
        def map2d():
            open_kind(pg, "full"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, MAP + "&&" + MAP + "._a")
            want = pg.evaluate(f"AZIONI.per(MAPPA.ad2({MAP},null)).filter(i=>i!=='-').map(i=>i.id)")
            assert "mapxic" in want and "maphelp" in want and "map3top" not in want, want
            keyboard(MAP); kb = pg.evaluate(IDS); close()
            assert pg.evaluate(f"document.activeElement==={MAP}.cv"), "Esc gives the focus back to the map"
            w = pg.evaluate(f"(()=>{{const r={MAP}.cv.getBoundingClientRect();return [r.width,r.height]}})()")
            right_at(MAP, w[0] * 0.5, w[1] * 0.5); ms = pg.evaluate(IDS); close()
            assert kb == ms == want, (kb, ms, want)
            bar = pg.evaluate(f"[...{MAP}.el.querySelectorAll('.mznav [data-nav]')].map(b=>b.dataset.nav)")
            assert bar and all(pg.evaluate(f"(AZIONI.voce('{i}')||{{viste:[]}}).viste.includes('map')") for i in bar), bar
            pal = pg.evaluate("MAPPA.CMDS.filter(c=>MAPPA.paletteOk(c)).map(c=>c.id)")
            assert set(want) <= set(pal), (want, pal)
        step("map 2D: right click = Shift+F10 = the registry; the bar and the palette use registered actions", map2d)
        def same(P, kind):
            open_kind(pg, kind); ready(pg); attendi(pg, P + "&&" + P + "._a")
            pg.evaluate("(()=>{if(window.__menuAt!==undefined)return;window.__menuAt=null;const m=window.menu;window.menu=(ev,items)=>{window.__menuAt=[ev.clientX,ev.clientY];return m(ev,items)}})()")     # where the menu was asked for
            keyboard(P); kb = pg.evaluate(LABELS); at = pg.evaluate("window.__menuAt")
            close(); assert pg.evaluate(f"document.activeElement==={P}.cv"), "Esc gives the focus back to the graph"
            b = pg.evaluate(f"(()=>{{const r={P}.cv.getBoundingClientRect();return [r.left,r.top]}})()")
            right_at(P, at[0] - b[0], at[1] - b[1]); ms = pg.evaluate(LABELS); close()
            assert kb and kb == ms, (kb, ms)
        step("chromatogram (Full Scan): Shift+F10 = right click at the same point", lambda: same("E.panels.find(q=>q.type==='chrom'&&q.tab==='full')", "full"))
        SP = "E.panels.find(q=>q.type==='spec'&&q.el&&q.el.offsetParent)"
        step("spectrum: Shift+F10 = right click at the same peak", lambda: same(SP, "full"))
        def spec_ids():
            pg.evaluate(f"(()=>{{const p={SP};p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
            keyboard(SP); r = pg.evaluate("""(()=>{const its=[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1);
              return {n:its.length,free:its.filter(d=>!d.dataset.azione).map(d=>d.textContent),ids:[...new Set(its.map(d=>d.dataset.azione))],
                bad:its.map(d=>d.dataset.azione).filter(i=>i&&!(AZIONI.voce(i)||{viste:[]}).viste.includes('spec'))}})()"""); close()
            assert r["n"] >= 5 and not r["free"] and not r["bad"], r
            assert "spec.copy" in r["ids"] and "spec.xic" in r["ids"] and "spec.measure" in r["ids"], r["ids"]
        step("spectrum: every entry of the menu is an action of the registry (data-azione)", spec_ids)
        step("MRM: Shift+F10 = right click at the same point", lambda: same("E.panels.find(q=>q.type==='mrm')", "mrm"))
        def trace_ids():
            for sel, vista in (("E.panels.find(q=>q.type==='chrom'&&q.tab==='full')", "chrom"), ("E.panels.find(q=>q.type==='mrm')", "mrm")):
                pg.evaluate(f"(()=>{{const p={sel};p.zoom=null;p.zoomY=null;p.anns=[];draw(p)}})()")
                keyboard(sel); r = pg.evaluate("""(v=>{const its=[...document.querySelectorAll('#ctx div[role^=menuitem]')].slice(1);
                  return {n:its.length,free:its.filter(d=>!d.dataset.azione).map(d=>d.textContent),ids:[...new Set(its.map(d=>d.dataset.azione))],
                    bad:its.map(d=>d.dataset.azione).filter(i=>i&&!(AZIONI.voce(i)||{viste:[]}).viste.includes(v))}})""" + f"('{vista}')"); close()
                assert r["n"] >= 2 and not r["free"] and not r["bad"], (vista, r)
                assert "chrom.annotate" in r["ids"] and "chrom.int" in r["ids"] and "view.resetZoom" in r["ids"] and "ann.edit" in r["ids"], r["ids"]
                assert (vista == "mrm") == ("chrom.specAtRt" not in r["ids"]), r["ids"]
        step("chromatogram and MRM: every entry of the menu is an action of the registry (data-azione)", trace_ids)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
