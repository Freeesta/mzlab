"""Navigation of the 3D viewer (mappa3d.js): the right-click menu shared with the 2D map (extract XIC / spectrum / both from the point under the pointer,
lock, reset, preset views, copy), the bar, the sheet of the gestures, keyboard and long press. LR example files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_nav3d.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8898, wd="/tmp/wd_mappa_nav3d")
M = "E.panels.find(q=>q.type==='map')"
BTN = ".pnl.map [data-o=view][data-v='3d']"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}, has_touch=True, permissions=["clipboard-read", "clipboard-write"])
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        npan = lambda t: pg.evaluate(f"E.panels.filter(q=>q.type==='{t}').length")
        def open3d():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)
            pg.locator(BTN).click()
            pg.wait_for_function("document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g && window.__m3d.V.Z", timeout=60000); pg.wait_for_timeout(600)
        items = lambda: pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(d=>[d.textContent.trim(),d.classList.contains('dim'),d.getAttribute('aria-checked')])")
        def pick(label): pg.click(f"#ctx [role^=menuitem]:has-text('{label}')"); pg.wait_for_timeout(400)
        # a point of the surface on the screen: where the picking finds a cell
        def spot():
            return pg.evaluate("""(()=>{const c=document.querySelector('.m3d-gl').getBoundingClientRect(), m=window.__m3d;
              for (let fy=.5; fy<.9; fy+=.05) for (let fx=.3; fx<.8; fx+=.05) { const h=m.pick(c.width*fx, c.height*fy); if (h) return [c.left+c.width*fx, c.top+c.height*fy]; } return null })()""")
        def menu():
            open3d(); pt = spot(); assert pt
            pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(300)
            its = items(); names = [i[0] for i in its]
            for want in ["XIC", "Spectrum", "Lock the sight", "Reset zoom", "From above", "Side RT", "Side m/z", "Copy coordinates", "Gestures"]:
                assert any(want in n for n in names), (want, names)
            assert not any("axis" in n or "selection" in n or "both" in n for n in names), names                 # a short menu
            assert pg.evaluate("document.getElementById('ctx').closest('dialog.m3d') !== null"), "the menu is inside the modal dialog"
            assert pg.evaluate("(()=>{const r=document.getElementById('ctx').getBoundingClientRect();const e=document.elementFromPoint(r.left+10,r.top+10);return !!e.closest('#ctx')})()"), "the menu is on top"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate("document.getElementById('ctx').hidden") and pg.evaluate("!!document.querySelector('dialog.m3d[open]')"), "Esc closes only the menu"
        step("3D: right click opens the menu (inside the dialog), Esc closes only the menu", menu)
        def views():
            for label, (az, el) in {"From above": (0, 89), "Side RT": (0, 0), "Side m/z": (90, 0)}.items():
                pt = spot(); pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(250); pick(label)
                c = pg.evaluate("[window.__m3d.V.cam.az, window.__m3d.V.cam.el]"); assert c == [az, el], (label, c)
            az0 = pg.evaluate("window.__m3d.V.cam.az"); box = pg.locator(".m3d-gl").bounding_box(); cx, cy = box["x"] + box["width"] * .4, box["y"] + box["height"] * .5
            pg.mouse.move(cx, cy); pg.mouse.down(); pg.mouse.move(cx + 80, cy + 10, steps=5); pg.mouse.up(); pg.wait_for_timeout(200); assert pg.evaluate("window.__m3d.V.cam.az") != az0
            pt = spot(); pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(250); pick("Reset zoom")
            assert pg.evaluate("[window.__m3d.V.cam.az, window.__m3d.V.cam.el]") == [35, 32]
        step("3D: preset views and reset of the camera", views)
        def pan_right_drag():
            box = pg.locator(".m3d-gl").bounding_box(); cx, cy = box["x"] + box["width"] * .4, box["y"] + box["height"] * .5
            t0 = pg.evaluate("[...window.__m3d.V.cam.t]"); pg.mouse.move(cx, cy); pg.mouse.down(button="right"); pg.mouse.move(cx + 60, cy + 20, steps=5); pg.mouse.up(button="right"); pg.wait_for_timeout(250)
            assert pg.evaluate("[...window.__m3d.V.cam.t]") != t0 and pg.evaluate("document.getElementById('ctx').hidden"), "a right-button drag moves the view and does not open the menu"
        step("3D: dragging with the right button still pans", pan_right_drag)
        def copy():
            pt = spot(); pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(250); pick("Copy coordinates")
            t = pg.evaluate("navigator.clipboard.readText()"); assert t.startswith("RT ") and "m/z" in t and "intensity" in t, t
        step("3D: copy the coordinates", copy)
        def lock():
            pt = spot(); pg.mouse.move(pt[0], pt[1]); pg.wait_for_timeout(250)
            cur = pg.evaluate("window.__m3d.V.cur"); assert cur
            pg.click(".m3d-main [data-nav=maplock]"); pg.wait_for_timeout(200)
            assert pg.get_attribute(".m3d-main [data-nav=maplock]", "aria-pressed") == "true" and pg.evaluate("window.__m3d.V.lock")
            pt2 = spot(); pg.mouse.move(pt2[0] + 40, pt2[1] + 20); pg.wait_for_timeout(250); assert pg.evaluate("window.__m3d.V.cur") == cur, "locked: the cursor stays"
            pg.mouse.click(pt2[0] + 40, pt2[1] + 20, button="right"); pg.wait_for_timeout(250)
            its = items(); assert any("Unlock the sight" in i[0] and i[2] == "true" for i in its), its
            pg.keyboard.press("Escape"); pg.keyboard.press("l"); pg.wait_for_timeout(200); assert not pg.evaluate("window.__m3d.V.lock")
            assert pg.get_attribute(".m3d-main [data-nav=maplock]", "aria-pressed") == "false"
        step("3D: lock freezes the cursor, state on the bar and in the menu, L toggles", lock)
        def bar():
            btns = pg.evaluate("[...document.querySelectorAll('.m3d-main .mznav button')].map(b=>[b.dataset.nav,b.getAttribute('aria-label')])"); assert [x[0] for x in btns] == ["mapreset", "mapzin", "mapzout", "maplock", "maphelp"] and all(x[1] for x in btns), btns
            d0 = pg.evaluate("window.__m3d.V.cam.d"); pg.click(".m3d-main [data-nav=mapzin]"); pg.wait_for_timeout(150); d1 = pg.evaluate("window.__m3d.V.cam.d"); assert d1 < d0
            pg.click(".m3d-main [data-nav=mapzout]"); pg.wait_for_timeout(150); assert pg.evaluate("window.__m3d.V.cam.d") > d1
            pg.click(".m3d-main [data-nav=mapreset]"); pg.wait_for_timeout(150); assert pg.evaluate("window.__m3d.V.cam.az") == 35
            pg.click(".m3d-main [data-nav=maphelp]"); pg.wait_for_timeout(400)
            assert pg.evaluate("document.getElementById('bigdlg').open && document.querySelectorAll('#bigdlg table.mzhelp tbody tr').length >= 8")
            assert "3D" in pg.inner_text("#bigtt"); pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate("!!document.querySelector('dialog.m3d[open]')"), "Esc closes the sheet and not the viewer"
        step("3D: bar (reset, +, −, padlock, gestures), the sheet over the viewer", bar)
        def longpress():
            cdp = ctx.new_cdp_session(pg); pt = spot()
            cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": [{"x": pt[0], "y": pt[1], "id": 0}]}); pg.wait_for_function("!document.getElementById('ctx').hidden", timeout=4000)            # polling keeps the page awake; the menu opens while the finger is down
            cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); pg.wait_for_timeout(300)
            assert any(i[0] == "XIC" for i in items()), items()
            assert not pg.evaluate("document.getElementById('ctx').hidden"), "the menu stays open after the finger is lifted"
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200); assert pg.evaluate("!!document.querySelector('dialog.m3d[open]')")
        step("3D: long press opens the menu", longpress)
        def keymenu():
            pg.locator(".m3d-gl").focus(); pg.keyboard.press("ContextMenu"); pg.wait_for_timeout(300)
            assert not pg.evaluate("document.getElementById('ctx').hidden"); pg.keyboard.press("Escape"); pg.wait_for_timeout(150)
            pg.keyboard.press("?"); pg.wait_for_timeout(300); assert pg.evaluate("document.getElementById('bigdlg').open"); pg.keyboard.press("Escape"); pg.wait_for_timeout(150)
        step("3D: Menu key and ? from the keyboard", keymenu)
        def extract():
            n = (npan("xic"), npan("spec")); pt = spot(); pg.mouse.move(pt[0], pt[1]); pg.wait_for_timeout(250)
            want = pg.evaluate("(()=>{const m=window.__m3d, c=m.V.cur; return [m.rtAt(c.i), m.mzAt(c.j)]})()")
            pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(250); pick("Spectrum")
            pg.wait_for_function("!document.querySelector('dialog.m3d[open]')", timeout=5000); pg.wait_for_timeout(800)
            assert (npan("xic"), npan("spec")) == (n[0], n[1] + 1), (n, npan("xic"), npan("spec"))
            open3d(); pt = spot(); pg.mouse.move(pt[0], pt[1]); pg.wait_for_timeout(250)
            want = pg.evaluate("(()=>{const m=window.__m3d, c=m.V.cur; return [m.rtAt(c.i), m.mzAt(c.j)]})()")
            pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_timeout(250); pick("XIC")
            pg.wait_for_function("!document.querySelector('dialog.m3d[open]')", timeout=5000); pg.wait_for_timeout(800)
            assert (npan("xic"), npan("spec")) == (n[0] + 1, n[1] + 1), (n, npan("xic"), npan("spec"))
            mz = pg.evaluate("E.panels.filter(q=>q.type==='xic').sort((a,b)=>b.id-a.id)[0].traces[0].mz"); assert abs(mz - want[1]) <= 1.0, (mz, want)
            assert pg.evaluate("document.getElementById('ctx').parentElement === document.body"), "the menu goes back to the page when the viewer closes"
        step("3D: Spectrum and XIC from the point under the pointer: the panels appear, the viewer closes", extract)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
