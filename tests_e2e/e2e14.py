"""Window sizes: phone, tablet, laptop, large screen, and live resizing. No horizontal page scroll, panels follow the window, nothing unreadable."""
import sys
from playwright.sync_api import sync_playwright
from lib import Run, mz, SH, drawer

SIZES = [(360, 740), (768, 1024), (1024, 700), (1500, 900), (2560, 1300)]
R = []
def step(name, fn):
    try: fn(); R.append((name, "ok"))
    except Exception as e: R.append((name, "FAIL " + str(e)[:400]))

def overflow(pg):
    return pg.evaluate("[document.documentElement.scrollWidth, innerWidth]")

with sync_playwright() as p:
    run = Run(port=8814, wd="/tmp/wd14")
    pg = run.page(p)
    pg.set_viewport_size({"width": 1500, "height": 900})
    pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_MS2-t15"), mz("B_MRM-t0")])
    pg.wait_for_function("document.querySelectorAll('#flist tr').length >= 6", timeout=60000)
    for w, h in SIZES[:1]:
        pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(300)
        def start_small():
            sw, iw = overflow(pg); assert sw <= iw + 1, ("start screen overflows", sw, iw)
            pg.screenshot(path=SH + f"140_start_{w}.png")
        step(f"start screen at {w}px", start_small)
    pg.set_viewport_size({"width": 1500, "height": 900}); pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=60000); pg.wait_for_timeout(2500)
    pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
    for w, h in SIZES:
        def at():
            pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(900)
            sw, iw = overflow(pg); assert sw <= iw + 2, ("page scrolls sideways", sw, iw)
            host = pg.evaluate("Q('#dpanels').clientWidth")
            bad = pg.evaluate("E.panels.filter(p=>p.x + p.w > Q('#dpanels').clientWidth + 2).map(p=>[p.type,p.x,p.w])"); assert not bad, ("panel wider than the area", bad, host)
            full = pg.evaluate("E.panels.filter(p=>p.full && Math.abs(p.w - Q('#dpanels').clientWidth) > 2).length"); assert full == 0, ("full-width panels did not follow", full)
            cw = pg.evaluate("E.panels.map(p=>p.cv.clientWidth)"); assert all(c > 150 for c in cw), cw
            hdr = pg.evaluate("(()=>{const h=document.querySelector('header');return [h.scrollWidth,h.clientWidth,h.getBoundingClientRect().height]})()"); assert hdr[0] <= hdr[1] + 2, ("header clipped", hdr)
            pg.screenshot(path=SH + f"141_data_{w}.png")
        step(f"data view at {w}x{h}", at)
    def live():
        for w in (1200, 600, 1800, 420, 1500):
            pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.every(p=>!p.full || Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)"), w
        pg.wait_for_timeout(600)
        assert pg.evaluate("E.panels[0]._a.sr.length") >= 3
    step("live resize keeps panels fitted", live)
    def fold():
        pg.set_viewport_size({"width": 700, "height": 900}); pg.wait_for_timeout(600)      # below 900 px the file list is a sheet that replaces the workspace
        drawer(pg); pg.wait_for_timeout(400)
        assert pg.is_visible("#dfiles") and pg.evaluate("BARRA.sheet") and not pg.is_visible("#dpanels")
        assert pg.evaluate("document.querySelector('#dfiles').getBoundingClientRect().width") <= 700 + 1
        pg.keyboard.press("Escape"); pg.wait_for_timeout(400)
        assert pg.evaluate("E.panels.every(p=>!p.full || Math.abs(p.w - Q('#dpanels').clientWidth) <= 2)"); pg.keyboard.press("Escape"); pg.wait_for_timeout(400)
    step("file list is only a drawer at narrow width", fold)
    def tabs():
        for w in (360, 1500):
            pg.set_viewport_size({"width": w, "height": 800})
            for v in ("draw", "theory", "data"):
                pg.click(f"#nav button[data-v={v}]"); pg.wait_for_timeout(500)
                sw, iw = overflow(pg); assert sw <= iw + 2, (v, w, sw, iw)
    step("tabs at 360 and 1500", tabs)
    def hidpi():
        c2 = run.b.new_context(device_scale_factor=2, viewport={"width": 1100, "height": 800}); q = c2.new_page(); q.goto(f"http://127.0.0.1:{run.port}/"); q.wait_for_timeout(1500)
        q.wait_for_selector(".pnl.chrom canvas", timeout=30000); q.wait_for_timeout(1500)
        r = q.evaluate("(()=>{const c=E.panels[0].cv;return c.width/c.clientWidth})()"); assert abs(r - 2) < 0.05, r
        c2.close()
    step("sharp canvases on a high-density screen (2x)", hidpi)
    errs = [e for e in run.errs if e[0] == "pageerror"]
    run.close()
for r in R: print(r)
print("PAGEERRORS", errs)
