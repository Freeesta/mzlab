"""e2e_pbar: the shared panel bar #pbar (setting «Panel bar», off by default).
Off: no bar, the tools stay in every panel header. On: one toolbar with a chip (number, kind, file) that follows the selected panel (click or keys 1-9),
the common buttons (fit, download, expand, ruler, log, smoothing, zoom link) work on that panel from the bar and leave the header (expand and close stay in both), a panel with a non-default state shows a dot chip in its header, a click on empty space keeps the selection, the first panel is selected when none is."""
# covers: mzlab/web/pbar.js, mzlab/web/azioni-barra.js, tests_e2e/lib.py, mzlab/web/settings.js, mzlab/web/explore.js, mzlab/web/pannelli.js
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=9150, wd="/tmp/wd_pbar")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1000})
        stage(pg, [mz(f) for f in FILES]); pg.click("text=Load data"); ready(pg)
        open_kind(pg, "full"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})")
        wait_for_js(pg, "tabPanels().filter(q=>q.el&&q._a).length>=3")
        ORD = "tabPanels().filter(q=>q.el).sort(LAYOUT.ordine)"
        def off():
            assert pg.evaluate("document.querySelector('#pbar').hidden && !document.documentElement.hasAttribute('data-pbar')"), "off: no bar"
            assert pg.evaluate(f"{ORD}[0].el.querySelector('.tbs .tbg [data-a=fit]').offsetParent!==null"), "off: the tools are in the header"
        step("off by default: no bar, the header keeps its tools", off)
        def turn_on():
            pg.click("#np-set"); pg.wait_for_selector("#uip-pbar"); pg.click("#uip-pbar"); pg.keyboard.press("Escape")
            wait_for_js(pg, "!document.querySelector('#pbar').hidden")
            assert pg.evaluate("document.querySelector('#pbar').getAttribute('role')") == "toolbar"
            assert pg.evaluate("localStorage.getItem(NOMI.KEY('pbar'))") == "1"
            assert pg.evaluate(f"{ORD}[0].el.querySelector('.tbs .tbg [data-a=fit]').offsetParent===null"), "on: the header tool groups are hidden"
            assert pg.evaluate(f"{ORD}[0].el.querySelector('.x').offsetParent!==null"), "close stays in the header"
            h = pg.evaluate("document.querySelector('#pbar').offsetHeight"); assert h == 40, h
        step("the setting turns the bar on; the header keeps close, loses the tool groups", turn_on)
        def follows():
            n = pg.evaluate(f"{ORD}.length"); assert n >= 3, n
            def check(i, how):
                wait_for_js(pg, f"E.active===({ORD})[{i}]")
                wait_for_js(pg, f"document.querySelector('#pbar .pbchip')&&document.querySelector('#pbar .pbchip').textContent.startsWith('{i + 1} ')")
                a = pg.evaluate("document.querySelector('#pbar').getAttribute('aria-label')"); assert f"panel {i + 1}," in a, (how, a)
                assert pg.evaluate("!!document.querySelector('#pbar [data-a=fit]')"), (how, "no fit button")
            for i in range(3):
                pg.evaluate(f"({ORD})[{i}].el.scrollIntoView({{block:'center'}})")
                idx = pg.evaluate("p=>[...document.querySelectorAll('.pnl')].indexOf(p.el)", pg.evaluate_handle(f"({ORD})[{i}]"))
                pg.locator(".pnl").nth(idx).locator(".hd").click(position={"x": 4, "y": 4})
                check(i, "click")
            for i in (2, 0, 1):
                pg.evaluate("document.activeElement.blur()"); pg.keyboard.press(str(i + 1)); check(i, "key")
        step("the bar follows the panel selected by click and by keys 1-3", follows)
        def buttons():
            pg.keyboard.press("1"); wait_for_js(pg, f"E.active===({ORD})[0]")
            kind = pg.evaluate("E.active.type"); assert kind in ("chrom", "xic", "mrm"), kind
            pg.evaluate("(()=>{const p=E.active;p.zoom=[1,2];draw(p)})()"); frames(pg, 3)
            pg.click("#pbar [data-a=fit]"); wait_for_js(pg, "!E.active.zoom")
            was = pg.evaluate("!!E.active.smooth"); pg.click("#pbar [data-a=smooth]")
            wait_for_js(pg, f"!!E.active.smooth==={str(not was).lower()}")
            wait_for_js(pg, f"document.querySelector('#pbar [data-a=smooth]').getAttribute('aria-pressed')==='{str(not was).lower()}'")
            pg.keyboard.press("2"); wait_for_js(pg, f"E.active===({ORD})[1]")
            pg.evaluate("document.activeElement.blur()")
            assert pg.evaluate("document.querySelector('#pbar [data-a=smooth]')!==null") == (pg.evaluate("E.active.type")!="spec"), "the tools follow the kind of the panel"
        step("data-a buttons of the bar act on the selected panel and show its state", buttons)
        def common():
            pg.keyboard.press("1"); wait_for_js(pg, f"E.active===({ORD})[0]")
            ids = pg.evaluate("[...document.querySelectorAll('#pbar [data-a]')].map(b=>b.dataset.a)")
            for i in ("fit", "dl", "max", "smooth", "log", "tlink"): assert i in ids, (i, ids)
            for i in ("fit", "dl", "smooth", "log", "tlink"): assert pg.evaluate(f"{ORD}[0].el.querySelector('.tbs [data-a={i}]').offsetParent===null"), i + " left the header"
            assert pg.evaluate(f"{ORD}[0].el.querySelector('.tbs [data-a=max]').offsetParent!==null"), "expand stays in the header"
            assert pg.evaluate(f"!{ORD}[0].el.querySelector('.hmore [data-a=fit], .hmore [data-a=smooth]')"), "no tool in the overflow menu"
            bar(pg, ".pnl.chrom", "max"); wait_for_js(pg, "!!Q('.pnl.max')"); bar(pg, ".pnl.chrom", "max"); wait_for_js(pg, "!Q('.pnl.max')")
            pg.click("#pbar [data-a=dl]"); wait_for_js(pg, "!!document.querySelector('#ctx') && getComputedStyle(document.querySelector('#ctx')).display!=='none'"); pg.keyboard.press("Escape")
        step("fit, download, expand, ruler, log, smoothing and zoom link are in the bar; expand also in the header", common)
        def dot():
            pg.keyboard.press("1"); wait_for_js(pg, f"E.active===({ORD})[0]")
            k = pg.evaluate("E.active.type"); on = pg.evaluate("!!E.active.log")
            if not on: bar(pg, ".pnl.chrom", "log")
            wait_for_js(pg, f"!!{ORD}[0].el.querySelector('.hd .pdots [data-d=log]')")
            pg.keyboard.press("2"); wait_for_js(pg, f"E.active===({ORD})[1]")
            assert pg.evaluate(f"!!{ORD}[0].el.querySelector('.hd .pdots [data-d=log]')"), "the dot stays on a panel that is not selected"
            pg.keyboard.press("1"); wait_for_js(pg, f"E.active===({ORD})[0]")
            if not on: bar(pg, ".pnl.chrom", "log")
            wait_for_js(pg, f"!{ORD}[0].el.querySelector('.hd .pdots [data-d=log]')")
        step("a non-default state (log) shows a dot chip in the header of its panel", dot)
        def keep():
            pg.keyboard.press("3"); wait_for_js(pg, f"E.active===({ORD})[2]")
            pg.evaluate("document.querySelector('#dpanels').dispatchEvent(new PointerEvent('pointerdown',{bubbles:true}))"); frames(pg, 2)
            assert pg.evaluate(f"E.active===({ORD})[2]"), "a click on empty space keeps the last panel"
            pg.evaluate("setActive(null)"); wait_for_js(pg, f"E.active===({ORD})[0]")
        step("empty space keeps the selection; the first panel is selected when none is", keep)
        def toolbar():
            assert pg.evaluate("document.querySelectorAll('#pbar .pbtools [tabindex=\"0\"]').length")==1, "one tab stop"
            pg.focus("#pbar .pbtools [tabindex='0']"); a = pg.evaluate("document.activeElement.dataset.a")
            pg.keyboard.press("ArrowRight"); b = pg.evaluate("document.activeElement.dataset.a"); assert a != b, (a, b)
            pg.keyboard.press("Home"); assert pg.evaluate("document.activeElement.dataset.a") == a
        step("one tab stop, arrows and Home move inside the bar", toolbar)
        def turn_off():
            pg.click("#np-set"); pg.wait_for_selector("#uip-pbar"); pg.click("#uip-pbar"); pg.keyboard.press("Escape")
            wait_for_js(pg, "document.querySelector('#pbar').hidden")
            assert pg.evaluate(f"{ORD}[0].el.querySelector('.tbs .tbg [data-a=fit]').offsetParent!==null"), "off again: the header tools are back"
        step("the setting off gives today's header back", turn_off)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
