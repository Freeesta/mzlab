"""e2e_pbar_phone: the shared panel bar #pbar at 390 px and with the keyboard.
390 px: one row of 44 px targets with the main tools of the kind in priority order, «More» opens a bottom sheet with the other tools (the same buttons, still working), a tool used in the sheet or Esc closes it, no sideways page scroll, axe (serious, critical) in the row and with the sheet open.
Keyboard: Shift+P moves the focus into the bar (one tab stop), arrows move, Enter acts, Esc gives the focus back to the chart of the selected panel; the key is in the Shortcuts panel; the panel is announced in #sb-live when the selection changes.
Wide: nothing changes (no «More», every tool in the bar)."""
# covers: mzlab/web/pbar.js, mzlab/web/app.css, mzlab/web/azioni-tasti.js, mzlab/web/lang/en/settings.js
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
AXE = axe_path()
if not AXE:
    print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=9152, wd="/tmp/wd_pbar_phone")
ORD = "tabPanels().filter(q=>q.el).sort(LAYOUT.ordine)"
RUN = """() => axe.run(document.querySelector('#pbar'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
VIS = "[...document.querySelectorAll('#pbar .pbtools [data-a], #pbar .pbmore')].filter(b=>!b.hidden && b.offsetParent!==null)"
def select(pg, i):
    pg.evaluate(f"setActive(({ORD})[{i}])"); wait_for_js(pg, f"E.active===({ORD})[{i}] && document.querySelector('#pbar .pbchip').textContent.startsWith('{i + 1} ')")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1000})
        pg.add_init_script("try{localStorage.setItem('mzlab.pbar','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#pick", state="attached")
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); pg.wait_for_selector("#opbtn", state="visible"); open_first(pg); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.click("#np-map"); ready(pg)
        wait_for_js(pg, "E.panels.some(q=>q.type==='spec') && E.panels.some(q=>q.type==='map') && E.panels.some(q=>q.type==='xic') && !document.querySelector('#pbar').hidden")
        KIND = pg.evaluate(f"{ORD}.map(q=>q.type)")
        def wide():
            select(pg, 0)
            assert pg.evaluate("document.querySelector('#pbar .pbmore').offsetParent===null"), "no More on a wide window"
            assert pg.evaluate("document.querySelectorAll('#pbar .pbtools [data-pm]').length")==0
        step("wide: no «More», every tool in the bar", wide)
        def keys():
            select(pg, 0); pg.evaluate("document.activeElement.blur()")
            pg.keyboard.press("Shift+P")
            wait_for_js(pg, "!!document.activeElement.closest('#pbar')")
            assert pg.evaluate("document.querySelectorAll('#pbar [tabindex=\"0\"]').length")==1, "one tab stop"
            a = pg.evaluate("document.activeElement.dataset.a")
            pg.keyboard.press("ArrowRight"); assert pg.evaluate("document.activeElement.dataset.a") != a
            pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.a") == a
            pg.keyboard.press("Escape")
            wait_for_js(pg, "document.activeElement===E.active.cv")
        step("Shift+P focuses the bar, arrows move, Esc returns to the chart of the selected panel", keys)
        def act():
            select(pg, 0); pg.keyboard.press("Shift+P")
            pg.focus("#pbar [data-a=smooth]")
            was = pg.evaluate("!!E.active.smooth"); pg.keyboard.press("Enter")
            wait_for_js(pg, f"!!E.active.smooth==={str(not was).lower()}")
            pg.keyboard.press("Enter"); wait_for_js(pg, f"!!E.active.smooth==={str(was).lower()}")
            pg.keyboard.press("Escape"); wait_for_js(pg, "document.activeElement===E.active.cv")
        step("Enter acts on the focused tool", act)
        def listed():
            pg.keyboard.press("?"); pg.wait_for_selector("#scorc[open]")
            txt = pg.evaluate("[...document.querySelectorAll('#scorc li')].map(l=>l.textContent)")
            assert any("Shift+P" in t or "⇧P" in t for t in txt), txt
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#scorc[open]')")
        step("the key is in the Shortcuts panel", listed)
        def say():
            select(pg, 0); pg.evaluate("document.getElementById('sb-live').textContent=''")
            select(pg, 1)
            wait_for_js(pg, "/Panel 2 selected/.test(document.getElementById('sb-live').textContent)")
        step("the panel is announced in #sb-live when the selection changes", say)
        pg.set_viewport_size({"width": 390, "height": 800}); frames(pg, 4)
        def row():
            for i, k in enumerate(KIND):
                select(pg, i)
                assert not pg.evaluate("document.documentElement.scrollWidth>document.documentElement.clientWidth"), (k, "page scrolls sideways")
                vis = pg.evaluate(f"{VIS}.map(b=>[b.dataset.a||'more', b.getBoundingClientRect().width, b.getBoundingClientRect().height])")
                assert vis and vis[-1][0] == "more", (k, vis)
                for a, w, h in vis: assert w >= 44 and h >= 44, (k, a, w, h)
                ids = [v[0] for v in vis[:-1]]; assert 1 <= len(ids) <= 4, (k, ids)
                assert pg.evaluate("document.querySelector('#pbar').scrollWidth <= document.querySelector('#pbar').clientWidth + 400"), k
            select(pg, 0); pg.add_script_tag(path=AXE); bad = pg.evaluate(RUN); assert not bad, bad
            order = pg.evaluate(f"{VIS}.filter(b=>b.dataset.a).sort((a,b)=>a.getBoundingClientRect().left-b.getBoundingClientRect().left).map(b=>b.dataset.a)")
            assert order[0] == "fit", order
        step("390 px: one row, 44 px targets, at most 4 tools and «More», no sideways page scroll, on every kind", row)
        def sheet():
            select(pg, 0); n = pg.evaluate("document.querySelectorAll('#pbar .pbtools [data-pm]:not([hidden])').length"); assert n > 0, n
            pg.click("#pbar .pbmore"); wait_for_js(pg, "document.querySelector('#pbar').dataset.more==='1'")
            assert pg.evaluate("document.querySelector('#pbar .pbmore').getAttribute('aria-expanded')") == "true"
            box = pg.evaluate("(()=>{const r=document.querySelector('#pbar .pbtools').getBoundingClientRect();return [r.left,r.right,r.bottom,innerWidth,innerHeight]})()")
            assert box[0] >= 0 and box[1] <= box[3] and abs(box[2] - box[4]) < 2, ("sheet at the bottom", box)
            assert pg.evaluate("[...document.querySelectorAll('#pbar .pbtools [data-a]')].filter(b=>!b.hidden).every(b=>b.offsetParent!==null && b.getBoundingClientRect().height>=44)"), "all tools in the sheet, 44 px"
            assert not pg.evaluate("document.documentElement.scrollWidth>document.documentElement.clientWidth"), "page scrolls sideways with the sheet"
            bad = pg.evaluate(RUN); assert not bad, bad
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#pbar').dataset.more")
            wait_for_js(pg, "document.activeElement===document.querySelector('#pbar .pbmore')")
        step("«More» opens the bottom sheet with every tool (axe clean), Esc closes it and returns to «More»", sheet)
        def use():
            select(pg, 0); pg.click("#pbar .pbmore"); wait_for_js(pg, "document.querySelector('#pbar').dataset.more==='1'")
            was = pg.evaluate("!!E.active.smooth")
            assert pg.evaluate("document.querySelector('#pbar [data-a=smooth]').offsetParent!==null"), "smoothing is in the sheet"
            pg.click("#pbar [data-a=smooth]"); wait_for_js(pg, f"!!E.active.smooth==={str(not was).lower()}")
            wait_for_js(pg, "!document.querySelector('#pbar').dataset.more")
            pg.click("#pbar .pbmore"); wait_for_js(pg, "document.querySelector('#pbar').dataset.more==='1'")
            pg.mouse.click(190, 100); wait_for_js(pg, "!document.querySelector('#pbar').dataset.more")
            pg.click("#pbar .pbmore"); pg.click("#pbar [data-a=smooth]"); wait_for_js(pg, f"!!E.active.smooth==={str(was).lower()}")
        step("a tool of the sheet acts on the panel and closes the sheet; a tap outside closes it", use)
        def shift_p():
            select(pg, 0); pg.evaluate("document.activeElement.blur()"); pg.keyboard.press("Shift+P")
            wait_for_js(pg, "!!document.activeElement.closest('#pbar')"); pg.keyboard.press("Escape"); wait_for_js(pg, "document.activeElement===E.active.cv")
        step("the key works at 390 px", shift_p)
        def back():
            pg.set_viewport_size({"width": 1440, "height": 1000}); frames(pg, 4); select(pg, 0)
            assert pg.evaluate("document.querySelectorAll('#pbar .pbtools [data-pm]').length")==0 and pg.evaluate("document.querySelector('#pbar .pbmore').offsetParent===null")
        step("widening the window gives the full bar back", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
