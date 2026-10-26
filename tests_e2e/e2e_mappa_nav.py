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
ZNULL = f"{M}.zoom == null && {M}.zoomY == null"      # the map shows the whole view
ZSET = f"{M}.zoom && {M}.zoomY"
CTX_OPEN, CTX_SHUT = "!document.getElementById('ctx').hidden", "document.getElementById('ctx').hidden"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}, has_touch=True, permissions=["clipboard-read", "clipboard-write"])
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        stage(pg, [EX / "FullScan_t0.mzML", EX / "FullScan_t30.mzML"])
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); frames(pg, 3)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())
        z = lambda: pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
        npan = lambda t: pg.evaluate(f"E.panels.filter(q=>q.type==='{t}').length")
        menu_items = lambda: pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(d=>[d.textContent.trim(),d.getAttribute('aria-label'),d.classList.contains('dim'),d.getAttribute('aria-checked')])")
        def pick(label, done=None):
            """Choose a menu item and wait until the menu is shut and, if given, the JavaScript condition `done` (its effect) holds."""
            pg.click(f"#ctx [role^=menuitem]:has-text('{label}')"); attendi(pg, CTX_SHUT)
            if done: attendi(pg, done)
        def rclick(x, y): pg.mouse.click(x, y, button="right"); attendi(pg, CTX_OPEN)
        def settled(n=5):
            """Wait until the zoom has not changed for `n` frames (the gesture has finished easing)."""
            pg.evaluate("window.__zs = null")
            pg.wait_for_function(f"(() => {{ const k = JSON.stringify([{M}.zoom, {M}.zoomY]), w = window.__zs && window.__zs.k === k ? window.__zs : (window.__zs = {{k, n: 0}}); return ++w.n >= {n}; }})()")
        def changed(before): pg.wait_for_function(f"function (b) {{ return JSON.stringify([{M}.zoom, {M}.zoomY]) !== JSON.stringify(b); }}", arg=before)
        def nxic(n): return f"E.panels.filter(q=>q.type==='xic').length === {n}"
        def zoom_in():
            x, y = at(.5, .45); pg.mouse.move(x, y)
            for _ in range(3): ctrl_wheel(pg, 0, -200); frames(pg, 2)
            attendi(pg, ZSET); assert z()[0] and z()[1], z()
        def menu():
            x, y = at(.4, .5); rclick(x, y)
            its = menu_items(); names = [i[0] for i in its]
            for want in ["XIC", "Spectrum", "Lock the sight", "Mark the point", "Reset zoom", "Copy coordinates", "Gestures"]:
                assert any(want in n for n in names), (want, names)
            assert not any("both" in n or "axis only" in n or "selection" in n or "How it is" in n or "Extract" in n for n in names), names     # a short menu: no duplicates, no teaching view in it
            assert sum(1 for i in its if i[1]) >= 7 and len([n for n in names]) <= 8, its                # seven commands with an aria-label (plus the line of the point)
            assert pg.evaluate("document.querySelectorAll('#ctx hr').length") >= 3
            assert "At the pointer" in names[0]
            pg.keyboard.press("Escape"); attendi(pg, CTX_SHUT); assert pg.evaluate("document.getElementById('ctx').hidden")
        step("right click: the menu with every command, separators, aria-labels, Esc closes", menu)
        def extract():
            n0 = (npan("xic"), npan("spec")); x, y = at(.4, .5)
            rclick(x, y); pick("XIC", nxic(n0[0] + 1))
            assert (npan("xic"), npan("spec")) == (n0[0] + 1, n0[1]), (n0, npan("xic"), npan("spec"))
            rclick(x, y); pick("Spectrum", f"E.panels.filter(q=>q.type==='spec').length === {n0[1] + 1}")
            assert (npan("xic"), npan("spec")) == (n0[0] + 1, n0[1] + 1)
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        step("XIC and Spectrum from the menu: new panels", extract)
        def locked():
            x, y = at(.45, .4); rclick(x, y); pick("Lock the sight", f"{M}._mz.lock")
            lk = pg.evaluate(f"{M}._mz.lock"); assert lk
            x2, y2 = at(.7, .6); rclick(x2, y2)
            its = menu_items(); assert any("Unlock the sight" in i[0] and i[3] == "true" for i in its), its
            assert "Locked sight" in its[0][0]
            n0 = npan("xic"); pick("XIC", nxic(n0 + 1)); assert npan("xic") == n0 + 1               # still works with the sight locked, on the locked point
            mz = pg.evaluate("E.panels.filter(q=>q.type==='xic').sort((a,b)=>b.id-a.id)[0].traces[0].mz"); assert abs(mz - lk["mz"]) < 1.0, (mz, lk)
            rclick(x2, y2); pick("Unlock the sight", f"{M}._mz.lock == null")
            assert pg.evaluate(f"{M}._mz.lock") is None
        step("lock from the menu: state on the bar (aria-pressed), commands still work, unlock", locked)
        def resets():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); zoom_in()
            x, y = at(.5, .5); rclick(x, y); pick("Reset zoom", ZNULL); assert z() == [None, None], z()
        step("Reset zoom from the menu (the axes one by one are on double click and Esc)", resets)
        def copy():
            x, y = at(.5, .5); pg.mouse.move(x, y); frames(pg, 2); rclick(x, y); pick("Copy coordinates")
            attendi(pg, "navigator.clipboard.readText().then(t => t.startsWith('RT '))"); t = pg.evaluate("navigator.clipboard.readText()"); assert t.startswith("RT ") and "m/z" in t, t
        step("Copy coordinates", copy)
        def bar():
            btns = pg.evaluate("[...document.querySelectorAll('.pnl.map .mznav button')].filter(b=>!b.hidden).map(b=>[b.dataset.nav,b.getAttribute('aria-label'),b.title])")
            assert [b[0] for b in btns] == ["mapzin", "mapzout", "maphelp"], btns
            assert all(b[1] for b in btns), btns
            zoom_in(); pg.click(".pnl.map [data-a=fit]"); attendi(pg, ZNULL); assert z() == [None, None]
            pg.click(".pnl.map [data-nav=mapzin]"); attendi(pg, ZSET); zz = z(); assert zz[0] and zz[1], zz
            pg.click(".pnl.map [data-nav=mapzout]"); changed(zz)
            pg.evaluate("MAPPA.whole(" + M + ")"); attendi(pg, ZNULL)
        step("bar on the map: reset, +, −, gestures (no padlock)", bar)
        def sheet():
            pg.click(".pnl.map [data-nav=maphelp]"); attendi(pg, "document.getElementById('bigdlg').open")
            assert pg.evaluate("document.getElementById('bigdlg').open")
            assert pg.evaluate("document.querySelectorAll('#bigdlg table.mzhelp tbody tr').length") >= 12
            heads = pg.evaluate("[...document.querySelectorAll('#bigdlg table.mzhelp thead th')].map(t=>t.textContent)"); assert heads == ["Action", "Mouse", "Trackpad", "Touch", "Keyboard"], heads
            pg.keyboard.press("Escape"); attendi(pg, "!document.getElementById('bigdlg').open")
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("?"); attendi(pg, "document.getElementById('bigdlg').open"); assert pg.evaluate("document.getElementById('bigdlg').open")
            pg.keyboard.press("Escape"); attendi(pg, "!document.getElementById('bigdlg').open")
        step("«?» and the ? key: the table of the gestures", sheet)
        def keys():
            pg.evaluate(M + ".cv.focus()"); x, y = at(.5, .5); pg.mouse.move(x, y)
            pg.keyboard.press("+"); attendi(pg, ZSET); z1 = z(); assert z1[0] and z1[1]
            pg.keyboard.press("+"); changed(z1); z2 = z(); assert (z2[0][1] - z2[0][0]) < (z1[0][1] - z1[0][0])
            pg.keyboard.press("Escape"); frames(pg, 8); assert z() == z2, (z(), z2)                  # Esc only unlocks the mirror: the zoom stays (Ctrl/Cmd+Z takes it back)
            pg.keyboard.press("-"); changed(z2)
            pg.keyboard.press("Home"); attendi(pg, ZNULL); assert z() == [None, None]
            zoom_in(); pg.keyboard.press("0"); attendi(pg, ZNULL); assert z() == [None, None]
            zoom_in(); pg.keyboard.press("Backspace"); attendi(pg, f"{M}.zoom == null && {M}.zoomY == null"); assert z() == [None, None]          # Backspace is the whole view too
            pg.keyboard.press("ContextMenu"); attendi(pg, CTX_OPEN + " && document.activeElement.getAttribute('role') === 'menuitem'")
            assert not pg.evaluate("document.getElementById('ctx').hidden"); focus = pg.evaluate("document.activeElement.getAttribute('role')"); assert focus == "menuitem", focus
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Escape"); attendi(pg, CTX_SHUT)
            n0 = npan("xic"); pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("ContextMenu"); attendi(pg, CTX_OPEN); pg.keyboard.press("Enter"); attendi(pg, nxic(n0 + 1))
            assert npan("xic") == n0 + 1, "Enter chooses the first live item: XIC"
        step("keyboard: + − Esc Home 0 Backspace, menu key, arrows and Enter in the menu", keys)
        def wheels():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.evaluate("MAPPA.whole(" + M + ")"); x, y = at(.5, .45); pg.mouse.move(x, y)
            pg.keyboard.down("Shift"); pg.mouse.wheel(0, -300); pg.keyboard.up("Shift"); attendi(pg, f"{M}.zoom && {M}.zoomY == null"); a = z(); assert a[0] and a[1] is None, a        # Shift = RT only
            pg.click(".pnl.map [data-a=fit]"); pg.mouse.move(x, y)
            pg.keyboard.down("Alt"); pg.mouse.wheel(0, -300); pg.keyboard.up("Alt"); attendi(pg, f"{M}.zoomY && {M}.zoom == null"); a = z(); assert a[1] and a[0] is None, a             # Alt = m/z only
            pg.click(".pnl.map [data-a=fit]"); pg.mouse.move(x, y); ctrl_wheel(pg, 0, -300); attendi(pg, ZSET); a = z(); assert a[0] and a[1], a
            before = z(); pg.mouse.wheel(0, -300); pg.mouse.wheel(120, 0); frames(pg, 8); assert z() == before, ("the wheel alone does not touch the map", before, z())      # it scrolls the page
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))"); x, y = at(.5, .45)
            pg.mouse.dblclick(x, y); attendi(pg, ZNULL); assert z() == [None, None]                                                                             # double click = reset
        step("wheel: Ctrl/⌘ zoom, Shift = RT, Alt = m/z, the wheel alone leaves the map, double click = reset", wheels)
        cdp = ctx.new_cdp_session(pg)
        def touch(kind, pts):
            cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": a, "y": c, "id": i} for i, (a, c) in enumerate(pts)]})
        def touches():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); frames(pg, 2)
            c = box(); cx, cy = c[0] + c[2] / 2, c[1] + c[3] / 2
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx - 40 - k * 12, cy), (cx + 40 + k * 12, cy)]); frames(pg, 2)
            touch("touchEnd", []); attendi(pg, ZSET); settled(); a = z(); assert a[0] and a[1], ("pinch out must zoom in", a)
            span = a[0][1] - a[0][0]
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx - 40 + k * 15, cy), (cx + 40 + k * 15, cy)]); frames(pg, 2)
            touch("touchEnd", []); pg.wait_for_function(f"function (a) {{ return {M}.zoom && {M}.zoom[0] < a[0][0]; }}", arg=a); settled(); b2 = z(); assert abs((b2[0][1] - b2[0][0]) - span) < span * 0.02 and b2[0][0] < a[0][0], ("two fingers move, same scale", a, b2)
            touch("touchStart", [(cx, cy)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx + k * 10, cy + k * 6)]); frames(pg, 2)
            assert pg.evaluate("!document.querySelector('.pnl.map .zr').hidden"), "the box of the finger is drawn"
            touch("touchEnd", []); pg.wait_for_function(f"function (b) {{ return {M}.zoom && {M}.zoom[1] - {M}.zoom[0] < (b[0][1] - b[0][0]) * 0.8 && document.querySelector('.pnl.map .zr').hidden; }}", arg=b2); settled(); b3 = z(); assert b3[0][1] - b3[0][0] < (b2[0][1] - b2[0][0]) * 0.8 and pg.evaluate("document.querySelector('.pnl.map .zr').hidden"), ("one finger draws a zoom box", b2, b3)
            for _ in range(2): touch("touchStart", [(cx, cy)]); frames(pg, 3); touch("touchEnd", []); frames(pg, 5)
            attendi(pg, ZNULL); assert z() == [None, None], ("double tap = reset", z())
        step("touch: pinch, two fingers move, one finger draws the zoom box, double tap = reset", touches)
        def longpress():
            c = box(); cx, cy = c[0] + c[2] * .4, c[1] + c[3] * .5
            touch("touchStart", [(cx, cy)]); pg.wait_for_function("!document.getElementById('ctx').hidden", timeout=4000); touch("touchEnd", []); frames(pg, 10)       # the menu must stay open after the lift: a few frames are the whole wait
            its = menu_items(); assert any(i[0] == "XIC" for i in its), its
            assert not pg.evaluate("document.getElementById('ctx').hidden"), "the menu stays open after the finger is lifted"
            pg.keyboard.press("Escape")
        step("touch: long press opens the menu", longpress)
        def targets():
            assert pg.evaluate("matchMedia('(pointer:coarse)').matches"), "touch context"
            sizes = pg.evaluate("[...document.querySelectorAll('.pnl.map .mznav button')].filter(b=>!b.hidden).map(b=>{const r=b.getBoundingClientRect();return [r.width,r.height]})")
            assert len(sizes) == 3 and all(w >= 40 and h >= 40 for w, h in sizes), sizes
        step("touch screen: the buttons of the bar are at least 40 px, like the others of the head", targets)
        def palette():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); x, y = at(.5, .5); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("Control+k"); pg.wait_for_timeout(400)      # kept: the point under the pointer is lost when the palette covers the map (mouseleave), so the typing races with it
            pg.keyboard.type("xic from the map"); pg.wait_for_timeout(300)
            assert "XIC from the map" in pg.inner_text("#pal-l"), pg.inner_text("#pal-l")
            n0 = npan("xic"); pg.keyboard.press("Enter"); attendi(pg, "document.getElementById('paletta').hidden")
            assert npan("xic") == n0 + 1 or pg.evaluate("!document.getElementById('paletta').hidden") is False
            for q in ["unlock", "gestures", "reset zoom", "copy the coord"]:
                pg.keyboard.press("Control+k"); attendi(pg, "!document.getElementById('paletta').hidden && document.activeElement.id === 'pal-in'"); pg.keyboard.type(q); attendi(pg, "document.querySelectorAll('#pal-l li').length >= 1"); assert pg.evaluate("document.querySelectorAll('#pal-l li').length") >= 1, q
                pg.keyboard.press("Escape"); attendi(pg, "document.getElementById('paletta').hidden")
        step("palette Ctrl/⌘ K finds the commands of the map", palette)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
