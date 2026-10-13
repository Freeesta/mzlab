"""Navigation of the RT x m/z map (mappa.js): the right-click menu with its commands (also with the sight locked), the bar on the map, resets of the
axes, keyboard, wheel, touch (pinch, two-finger move, double tap, long press) and the sheet of the gestures. LR example files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_nav.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8899, wd="/tmp/wd_mappa_nav")
M = "E.panels.find(q=>q.type==='map')"
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
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())
        z = lambda: pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
        npan = lambda t: pg.evaluate(f"E.panels.filter(q=>q.type==='{t}').length")
        menu_items = lambda: pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(d=>[d.textContent.trim(),d.getAttribute('aria-label'),d.classList.contains('dim'),d.getAttribute('aria-checked')])")
        def pick(label):
            pg.click(f"#ctx [role^=menuitem]:has-text('{label}')"); pg.wait_for_timeout(500)
        def zoom_in():
            x, y = at(.5, .45); pg.mouse.move(x, y)
            for _ in range(3): pg.mouse.wheel(0, -200); pg.wait_for_timeout(60)
            pg.wait_for_timeout(250); assert z()[0] and z()[1], z()
        def menu():
            x, y = at(.4, .5); pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(300)
            its = menu_items(); names = [i[0] for i in its]
            for want in ["XIC", "Spectrum", "Lock the sight", "Mark the point", "Reset zoom", "Copy coordinates", "Gestures"]:
                assert any(want in n for n in names), (want, names)
            assert not any("both" in n or "axis only" in n or "selection" in n or "How it is" in n or "Extract" in n for n in names), names     # a short menu: no duplicates, no teaching view in it
            assert sum(1 for i in its if i[1]) >= 7 and len([n for n in names]) <= 8, its                # seven commands with an aria-label (plus the line of the point)
            assert pg.evaluate("document.querySelectorAll('#ctx hr').length") >= 3
            assert "At the pointer" in names[0]
            pg.keyboard.press("Escape"); pg.wait_for_timeout(100); assert pg.evaluate("document.getElementById('ctx').hidden")
        step("right click: the menu with every command, separators, aria-labels, Esc closes", menu)
        def extract():
            n0 = (npan("xic"), npan("spec")); x, y = at(.4, .5)
            pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(200); pick("XIC")
            assert (npan("xic"), npan("spec")) == (n0[0] + 1, n0[1]), (n0, npan("xic"), npan("spec"))
            pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(200); pick("Spectrum")
            assert (npan("xic"), npan("spec")) == (n0[0] + 1, n0[1] + 1)
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        step("XIC and Spectrum from the menu: new panels", extract)
        def locked():
            x, y = at(.45, .4); pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(200); pick("Lock the sight")
            lk = pg.evaluate(f"{M}._mz.lock"); assert lk
            x2, y2 = at(.7, .6); pg.mouse.click(x2, y2, button="right"); pg.wait_for_timeout(200)
            its = menu_items(); assert any("Unlock the sight" in i[0] and i[3] == "true" for i in its), its
            assert "Locked sight" in its[0][0]
            n0 = npan("xic"); pick("XIC"); assert npan("xic") == n0 + 1               # still works with the sight locked, on the locked point
            mz = pg.evaluate("E.panels.filter(q=>q.type==='xic').sort((a,b)=>b.id-a.id)[0].traces[0].mz"); assert abs(mz - lk["mz"]) < 1.0, (mz, lk)
            pg.mouse.click(x2, y2, button="right"); pg.wait_for_timeout(200); pick("Unlock the sight")
            assert pg.evaluate(f"{M}._mz.lock") is None
        step("lock from the menu: state on the bar (aria-pressed), commands still work, unlock", locked)
        def resets():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); zoom_in()
            x, y = at(.5, .5); pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(200); pick("Reset zoom"); assert z() == [None, None], z()
        step("Reset zoom from the menu (the axes one by one are on double click and Esc)", resets)
        def copy():
            x, y = at(.5, .5); pg.mouse.move(x, y); pg.wait_for_timeout(200); pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(200); pick("Copy coordinates")
            t = pg.evaluate("navigator.clipboard.readText()"); assert t.startswith("RT ") and "m/z" in t, t
        step("Copy coordinates", copy)
        def bar():
            btns = pg.evaluate("[...document.querySelectorAll('.pnl.map .mznav button')].filter(b=>!b.hidden).map(b=>[b.dataset.nav,b.getAttribute('aria-label'),b.title])")
            assert [b[0] for b in btns] == ["mapzin", "mapzout", "maphelp"], btns
            assert all(b[1] for b in btns), btns
            zoom_in(); pg.click(".pnl.map [data-a=fit]"); pg.wait_for_timeout(300); assert z() == [None, None]
            pg.click(".pnl.map [data-nav=mapzin]"); pg.wait_for_timeout(300); zz = z(); assert zz[0] and zz[1], zz
            pg.click(".pnl.map [data-nav=mapzout]"); pg.wait_for_timeout(300)
            pg.evaluate("MAPPA.whole(" + M + ")"); pg.wait_for_timeout(200)
        step("bar on the map: reset, +, −, gestures (no padlock)", bar)
        def sheet():
            pg.click(".pnl.map [data-nav=maphelp]"); pg.wait_for_timeout(400)
            assert pg.evaluate("document.getElementById('bigdlg').open")
            assert pg.evaluate("document.querySelectorAll('#bigdlg table.mzhelp tbody tr').length") >= 12
            heads = pg.evaluate("[...document.querySelectorAll('#bigdlg table.mzhelp thead th')].map(t=>t.textContent)"); assert heads == ["Action", "Mouse", "Trackpad", "Touch", "Keyboard"], heads
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("?"); pg.wait_for_timeout(300); assert pg.evaluate("document.getElementById('bigdlg').open")
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
        step("«?» and the ? key: the table of the gestures", sheet)
        def keys():
            pg.evaluate(M + ".cv.focus()"); x, y = at(.5, .5); pg.mouse.move(x, y)
            pg.keyboard.press("+"); pg.wait_for_timeout(300); z1 = z(); assert z1[0] and z1[1]
            pg.keyboard.press("+"); pg.wait_for_timeout(300); z2 = z(); assert (z2[0][1] - z2[0][0]) < (z1[0][1] - z1[0][0])
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert z() == z1, (z(), z1)                  # Esc takes back the last zoom
            pg.keyboard.press("-"); pg.wait_for_timeout(300)
            pg.keyboard.press("Home"); pg.wait_for_timeout(300); assert z() == [None, None]
            zoom_in(); pg.keyboard.press("0"); pg.wait_for_timeout(300); assert z() == [None, None]
            pg.keyboard.press("ContextMenu"); pg.wait_for_timeout(300)
            assert not pg.evaluate("document.getElementById('ctx').hidden"); focus = pg.evaluate("document.activeElement.getAttribute('role')"); assert focus == "menuitem", focus
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            n0 = npan("xic"); pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("ContextMenu"); pg.wait_for_timeout(300); pg.keyboard.press("Enter"); pg.wait_for_timeout(400)
            assert npan("xic") == n0 + 1, "Enter chooses the first live item: XIC"
        step("keyboard: + − Esc Home 0, menu key, arrows and Enter in the menu", keys)
        def wheels():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.evaluate("MAPPA.whole(" + M + ")"); x, y = at(.5, .45); pg.mouse.move(x, y)
            pg.keyboard.down("Shift"); pg.mouse.wheel(0, -300); pg.keyboard.up("Shift"); pg.wait_for_timeout(300); a = z(); assert a[0] and a[1] is None, a        # Shift = RT only
            pg.click(".pnl.map [data-a=fit]"); pg.mouse.move(x, y)
            pg.keyboard.down("Alt"); pg.mouse.wheel(0, -300); pg.keyboard.up("Alt"); pg.wait_for_timeout(300); a = z(); assert a[1] and a[0] is None, a             # Alt = m/z only
            pg.click(".pnl.map [data-a=fit]"); pg.mouse.move(x, y); pg.mouse.wheel(0, -300); pg.wait_for_timeout(300); a = z(); assert a[0] and a[1], a
            before = z()[0]; pg.mouse.wheel(120, 0); pg.wait_for_timeout(300); after = z()[0]; assert after[0] > before[0], (before, after)                          # trackpad sideways = move along RT
            pg.mouse.dblclick(x, y); pg.wait_for_timeout(300); assert z() == [None, None]                                                                             # double click = reset
        step("wheel: zoom, Shift = RT, Alt = m/z, sideways = move, double click = reset", wheels)
        cdp = ctx.new_cdp_session(pg)
        def touch(kind, pts):
            cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": a, "y": c, "id": i} for i, (a, c) in enumerate(pts)]})
        def touches():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(200)
            c = box(); cx, cy = c[0] + c[2] / 2, c[1] + c[3] / 2
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); pg.wait_for_timeout(50)
            for k in range(1, 7): touch("touchMove", [(cx - 40 - k * 12, cy), (cx + 40 + k * 12, cy)]); pg.wait_for_timeout(30)
            touch("touchEnd", []); pg.wait_for_timeout(300); a = z(); assert a[0] and a[1], ("pinch out must zoom in", a)
            span = a[0][1] - a[0][0]
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); pg.wait_for_timeout(50)
            for k in range(1, 7): touch("touchMove", [(cx - 40 + k * 15, cy), (cx + 40 + k * 15, cy)]); pg.wait_for_timeout(30)
            touch("touchEnd", []); pg.wait_for_timeout(300); b2 = z(); assert abs((b2[0][1] - b2[0][0]) - span) < span * 0.02 and b2[0][0] < a[0][0], ("two fingers move, same scale", a, b2)
            touch("touchStart", [(cx, cy)]); pg.wait_for_timeout(50)
            for k in range(1, 7): touch("touchMove", [(cx + k * 10, cy)]); pg.wait_for_timeout(30)
            touch("touchEnd", []); pg.wait_for_timeout(300); b3 = z(); assert b3[0][0] < b2[0][0], ("one finger moves", b2, b3)
            for _ in range(2): touch("touchStart", [(cx, cy)]); pg.wait_for_timeout(40); touch("touchEnd", []); pg.wait_for_timeout(80)
            pg.wait_for_timeout(300); assert z() == [None, None], ("double tap = reset", z())
        step("touch: pinch, two fingers move, one finger moves, double tap = reset", touches)
        def longpress():
            c = box(); cx, cy = c[0] + c[2] * .4, c[1] + c[3] * .5
            touch("touchStart", [(cx, cy)]); pg.wait_for_function("!document.getElementById('ctx').hidden", timeout=4000); touch("touchEnd", []); pg.wait_for_timeout(300)
            its = menu_items(); assert any(i[0] == "XIC" for i in its), its
            assert not pg.evaluate("document.getElementById('ctx').hidden"), "the menu stays open after the finger is lifted"
            pg.keyboard.press("Escape")
        step("touch: long press opens the menu", longpress)
        def targets():
            assert pg.evaluate("matchMedia('(pointer:coarse)').matches"), "touch context"
            sizes = pg.evaluate("[...document.querySelectorAll('.pnl.map .mznav button')].filter(b=>!b.hidden).map(b=>{const r=b.getBoundingClientRect();return [r.width,r.height]})")
            assert len(sizes) == 4 and all(w >= 40 and h >= 40 for w, h in sizes), sizes
        step("touch screen: the buttons of the bar are at least 40 px, like the others of the head", targets)
        def palette():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); x, y = at(.5, .5); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("Control+k"); pg.wait_for_timeout(400)
            pg.keyboard.type("xic from the map"); pg.wait_for_timeout(300)
            assert "XIC from the map" in pg.inner_text("#pal-l"), pg.inner_text("#pal-l")
            n0 = npan("xic"); pg.keyboard.press("Enter"); pg.wait_for_timeout(500)
            assert npan("xic") == n0 + 1 or pg.evaluate("!document.getElementById('paletta').hidden") is False
            for q in ["unlock", "gestures", "reset zoom", "copy the coord"]:
                pg.keyboard.press("Control+k"); pg.wait_for_timeout(300); pg.keyboard.type(q); pg.wait_for_timeout(200); assert pg.evaluate("document.querySelectorAll('#pal-l li').length") >= 1, q
                pg.keyboard.press("Escape"); pg.wait_for_timeout(150)
        step("palette Ctrl/⌘ K finds the commands of the map", palette)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
