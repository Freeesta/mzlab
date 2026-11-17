"""Draw on a touch screen: long-press a molecule -> menu (Select molecule / Copy / Cut / Paste / Delete), copy, then a tap drops the copy centred on the finger (a drag adjusts
it before the release), undo takes it back in one step; «Paste here» on empty canvas; the copy keeps charge, isotope and stereo; a refused system clipboard changes nothing.
Real pointer events through CDP Input.dispatchTouchEvent; synthetic structures only. Screenshots go to $SHOTS when set."""
# covers: mzlab/web/disegno-appunti.js, mzlab/web/touch.js, mzlab/web/draw.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_disegno_appunti_tocco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SHOTS = os.environ.get("SHOTS")
r = Run(port=8860, wd="/tmp/wd_disegno_appunti")
try:
    with sync_playwright() as p:
        pg = r.page(p, touch=True, viewport={"width": 1024, "height": 768}, device_scale_factor=2)
        cdp = pg.context.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def wait(ms=250): hold(pg, ms)
        def tap(x, y): T("touchStart", [(x, y)]); wait(50); T("touchEnd", []); wait(500)
        def longpress(x, y, ms=800): T("touchStart", [(x, y)]); wait(ms); T("touchEnd", []); wait(900)     # the click that follows a long press is swallowed for 700 ms
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        pg.evaluate(f"{KJ}.addFragment('[13CH3][C@H](O)C(=O)[O-]')"); wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()"); wait(1500)
        FR = "document.querySelector('#kframe')"
        def where(ids):
            """screen centre of the atoms `ids` (page coordinates) from the editor's own geometry"""
            return pg.evaluate("""([ids]) => { const K = document.querySelector('#kframe').contentWindow.ketcher, fr = document.querySelector('#kframe').getBoundingClientRect(), rd = K.editor.render;
              // the editor only has page -> object; two probe points give the linear map back (screen = iframe origin + page)
              const o0 = rd.page2obj({ pageX: 100, pageY: 100, clientX: 100, clientY: 100 }), o1 = rd.page2obj({ pageX: 300, pageY: 500, clientX: 300, clientY: 500 });
              const sx = 200 / (o1.x - o0.x), sy = 400 / (o1.y - o0.y);
              const pts = ids.map(i => rd.ctab.atoms.get(i).a.pp); const ox = pts.reduce((a, q) => a + q.x, 0) / pts.length, oy = pts.reduce((a, q) => a + q.y, 0) / pts.length;
              return [fr.left + 100 + (ox - o0.x) * sx, fr.top + 100 + (oy - o0.y) * sy]; }""", [ids])
        def hist(which): pg.evaluate("(w) => { const b = [...document.querySelector('#kframe').contentDocument.querySelectorAll(`[data-testid=${w}]`)].find(e => e.offsetParent && !e.disabled); b && b.click(); }", which)
        ids = lambda: pg.evaluate(f"[...{KJ}.editor.struct().atoms.keys()]")
        nat = lambda: pg.evaluate(f"{KJ}.editor.struct().atoms.size")
        sel = lambda: pg.evaluate(f"({KJ}.editor.selection() || {{}}).atoms || []")
        def menu_rows(): return pg.evaluate("[...document.querySelectorAll('#ctx [role=menuitem]')].filter(e => e.getBoundingClientRect().height > 0).map(e => e.dataset.row)")
        def menu_open(): return pg.evaluate("(() => { const m = document.getElementById('ctx'); return !!m && !m.hidden; })()")
        def tap_row(row): x, y = pg.evaluate("(r) => { const b = document.querySelector(`#ctx [data-row=${r}]`).getBoundingClientRect(); return [b.left + b.width / 2, b.top + b.height / 2]; }", row); tap(x, y)
        atoms0 = where(list(range(6)))
        cx, cy = atoms0; target = where([1])           # the CH carbon

        def press_menu():
            T("touchStart", [tuple(target)]); wait(250)
            assert pg.evaluate("!!document.querySelector('#kframe').contentDocument.querySelector('.tp-ring')"), "no press ring while the press runs"
            wait(500); T("touchEnd", []); wait(900)
            assert menu_open(), "the long press opened no menu"
            rows = menu_rows()
            for need in ("copy", "cut", "delete"): assert need in rows, rows
            assert "paste" not in rows, "nothing copied yet: no Paste row " + str(rows)
            assert len(sel()) == 6, "the long press selects the whole molecule: " + str(sel())
            assert pg.evaluate("document.getElementById('ctx').getAttribute('role')") == "menu"
            assert not pg.evaluate("!!document.querySelector('.tp-ring') || !!document.querySelector('#kframe').contentDocument.querySelector('.tp-ring')"), "the ring stays after the menu opened"
        step("long press on a molecule: ring, whole molecule selected, menu Copy / Cut / Delete (no Paste yet)", press_menu)
        def menu_targets():
            hs = pg.evaluate("[...document.querySelectorAll('#ctx [role=menuitem]')].map(e => [e.getBoundingClientRect().height, (e.title || '').length, e.dataset.row])")
            assert hs and all(h >= 43.5 for h, _, _ in hs), hs
            assert all(n <= 60 for _, n, row in hs if row in ("select-molecule", "copy", "cut", "paste", "delete")), hs
            if SHOTS: pg.screenshot(path=os.path.join(SHOTS, "menu.png"))
        step("menu rows are at least 44 px high with tooltips of at most 60 characters", menu_targets)
        def outside():
            tap(900, 700); assert not menu_open(), "a tap outside leaves the menu open"
            T("touchStart", [tuple(target)]); wait(800); T("touchEnd", []); wait(900); assert menu_open()
            pg.keyboard.press("Escape"); wait(200); assert not menu_open(), "Esc leaves the menu open"
        step("a tap outside and Esc close the menu", outside)
        def keys():
            pg.evaluate("""(() => { const d = document.querySelector('#kframe').contentDocument; d.dispatchEvent(new d.defaultView.KeyboardEvent('keydown', {key: 'F10', shiftKey: true, bubbles: true, cancelable: true})); })()"""); wait(800)
            assert menu_open() and "copy" in menu_rows(), "Shift+F10 does not open the menu"
            pg.keyboard.press("Escape"); wait(200)
            pg.evaluate("""(() => { const d = document.querySelector('#kframe').contentDocument; d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu', {bubbles: true, cancelable: true, clientX: 300, clientY: 300})); })()"""); wait(800)
            assert menu_open(), "right click does not open the menu"
            items = pg.evaluate("[...document.querySelectorAll('#ctx [role=menuitem]')].map(e => e.tabIndex)"); assert items
            pg.keyboard.press("Escape"); wait(200)
        step("Shift+F10 and the right click open the same menu; rows are menuitems", keys)
        def copy_denied():
            pg.evaluate("document.getElementById('errbox') && document.getElementById('errbox').remove()")       # an earlier error (the engine .wasm may not be built here) must not hide this check
            T("touchStart", [tuple(target)]); wait(800); T("touchEnd", []); wait(900)
            tap_row("copy"); wait(500)
            assert pg.evaluate(f"document.querySelector('#kframe').contentWindow.__tpAppunti.has()"), "the internal clipboard is empty"
            assert pg.evaluate("!document.getElementById('tp-paste-chip').hidden"), "no «tap where to paste» chip"
            assert pg.locator("#errbox").count() == 0, "an error box was shown: " + pg.inner_text("#errbox")[:300]
        step("Copy with the system clipboard refused still fills the internal clipboard and arms the paste", copy_denied)
        def drop():
            n0 = nat(); old = ids(); P = (cx + 230, cy + 120) if cx < 560 else (cx - 230, cy + 120)
            T("touchStart", [P]); wait(150); T("touchEnd", []); wait(900)
            assert nat() == n0 + 6, (n0, nat())
            c = where([i for i in ids() if i not in old]); assert abs(c[0] - P[0]) < 30 and abs(c[1] - P[1]) < 30, f"dropped at {c}, tapped {P}"
            assert pg.evaluate("document.getElementById('tp-paste-chip').hidden"), "the chip stays after the drop"
            if SHOTS: pg.screenshot(path=os.path.join(SHOTS, "pasted.png"))
        step("a tap on empty canvas drops the copy centred on the finger", drop)
        def keep():
            a = pg.evaluate(f"(() => {{ const s = {KJ}.editor.struct(); const f = i => [...s.atoms.entries()].filter(([id]) => (i ? id >= 6 : id < 6)).map(([, a]) => [a.label, a.charge, a.isotope || 0].join()).sort(); return [f(0), f(1)]; }})()")
            assert a[0] == a[1], a
            b = pg.evaluate(f"(() => {{ const s = {KJ}.editor.struct(); const f = i => [...s.bonds.entries()].filter(([id]) => (i ? id >= 5 : id < 5)).map(([, b]) => [b.type, b.stereo].join()).sort(); return [f(0), f(1)]; }})()")
            assert b[0] == b[1] and any(x.split(",")[1] != "0" for x in b[0]), b
        step("the copy keeps labels, charge, isotope and the stereo bond", keep)
        def undo():
            hist("undo"); wait(500)
            assert nat() == 6, nat()
            hist("redo"); wait(500)
            assert nat() == 12, nat()
            hist("undo"); wait(500); assert nat() == 6
        step("undo takes the whole paste back in one step, redo restores it", undo)
        def drag():
            pg.evaluate(f"{KJ}.editor.selection(null)"); wait(200)
            T("touchStart", [tuple(target)]); wait(800); T("touchEnd", []); wait(900)
            assert "paste" in menu_rows(), "Paste is missing after a copy: " + str(menu_rows())
            tap_row("paste"); wait(400)
            assert pg.evaluate("!document.getElementById('tp-paste-chip').hidden"), "Paste did not arm"
            old = ids(); A = (cx + 200, cy + 150) if cx < 560 else (cx - 200, cy + 150); B = (A[0] + 40, A[1] + 70)
            T("touchStart", [A]); wait(150)
            for k in range(1, 9): T("touchMove", [(A[0] + (B[0] - A[0]) * k / 8, A[1] + (B[1] - A[1]) * k / 8)]); wait(40)
            wait(300)
            assert nat() == 12, "no preview while the finger is down: " + str(nat())
            c = where([i for i in ids() if i not in old]); assert abs(c[0] - B[0]) < 30 and abs(c[1] - B[1]) < 30, f"preview at {c}, finger at {B}"
            if SHOTS: pg.screenshot(path=os.path.join(SHOTS, "preview.png"))
            T("touchEnd", []); wait(900)
            assert nat() == 12; c = where([i for i in ids() if i not in old]); assert abs(c[0] - B[0]) < 30 and abs(c[1] - B[1]) < 30, f"dropped at {c}, released at {B}"
        step("Paste from the menu, then a drag: the preview follows the finger and the drop lands where it is released", drag)
        def here():
            hist("undo"); wait(500); n0 = nat(); old = ids()
            P = (cx + 40, cy + 230) if cy < 400 else (cx + 40, cy - 230)
            longpress(*P); assert menu_open(), "no menu on empty canvas"
            rows = menu_rows(); assert "paste" in rows and "copy" not in rows, rows
            assert "Paste here" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
            tap_row("paste"); wait(900)
            assert nat() == n0 + 6, (n0, nat())
            c = where([i for i in ids() if i not in old]); assert abs(c[0] - P[0]) < 30 and abs(c[1] - P[1]) < 30, f"pasted at {c}, pressed at {P}"
        step("long press on empty canvas offers «Paste here» and places the copy there", here)
        def cut_delete():
            n0 = nat(); pg.evaluate(f"{KJ}.editor.selection(null)"); wait(200)
            T("touchStart", [tuple(target)]); wait(800); T("touchEnd", []); wait(900)
            tap_row("cut"); wait(800); assert nat() == n0 - 6, ("cut", n0, nat())
            assert pg.evaluate("!document.getElementById('tp-paste-chip').hidden"), "a cut arms the paste too"
            pg.keyboard.press("Escape"); wait(200); assert pg.evaluate("document.getElementById('tp-paste-chip').hidden"), "Esc leaves the paste armed"
            hist("undo"); wait(500); assert nat() == n0
            T("touchStart", [tuple(target)]); wait(800); T("touchEnd", []); wait(900)
            tap_row("delete"); wait(800); assert nat() == n0 - 6, ("delete", n0, nat())
        step("Cut and Delete remove the molecule (undo restores it)", cut_delete)
        def system():
            pg.context.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{r.port}")
            pg.evaluate(f"{KJ}.editor.selection('all')"); wait(1200)
            # the system clipboard needs the editor's document focused: a real click on Ketcher's own Copy button (the menu row calls the same button)
            pg.frame_locator("#kframe").locator("[data-testid=copy-button]").first.click(position={"x": 7, "y": 16}); wait(700)       # the left of the button: the copy menu's triangle covers its right
            assert pg.evaluate("document.querySelector('#kframe').contentWindow.__tpAppunti.has()")
            pg.bring_to_front(); t = pg.evaluate("navigator.clipboard.readText().catch(e => 'ERR ' + e.name + ': ' + e.message)"); assert "V2000" in t or "V3000" in t, t[:80]
        step("with the system clipboard allowed, Copy also puts the molfile there", system)
        def mouse():
            pg.keyboard.press("Escape"); pg.evaluate("document.getElementById('tp-paste-chip').hidden = true"); n0 = nat(); old = ids()
            pg.evaluate("""() => { const d = document.querySelector('#kframe').contentDocument, fr = document.querySelector('#kframe').getBoundingClientRect(); window.__fl = fr.left; window.__ft = fr.top; }""")
            pg.evaluate(f"{KJ}.editor.selection('all')"); wait(500)
            pg.evaluate("""() => { const d = document.querySelector('#kframe').contentDocument; d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu', {bubbles: true, cancelable: true, clientX: 300, clientY: 300})); }"""); wait(900)
            assert "paste" in menu_rows(), menu_rows()
            pg.evaluate("document.querySelector('#ctx [data-row=paste]').click()"); wait(500)
            assert pg.evaluate("document.getElementById('tp-paste-chip').hidden"), "a mouse needs no chip"
            P = (cx - 100 if cx > 500 else cx + 260, cy + 140)
            pg.mouse.move(P[0] - 20, P[1] - 20); pg.mouse.move(*P); wait(300)
            assert nat() == n0 + 6, "the copy is not on the cursor"
            pg.mouse.down(); pg.mouse.up(); wait(700)
            c = where([i for i in ids() if i not in old]); assert abs(c[0] - P[0]) < 40 and abs(c[1] - P[1]) < 40, f"dropped at {c}, clicked {P}"
        step("with a mouse, Paste puts the copy on the cursor and a click drops it", mouse)
        def select_molecule():
            hist("undo"); wait(500)
            one = ids()[1]; pg.evaluate(f"{KJ}.editor.selection({{atoms: [{one}]}})"); wait(300)
            x, y = where([one])
            pg.evaluate("""([x, y]) => { const d = document.querySelector('#kframe').contentDocument, fr = document.querySelector('#kframe').getBoundingClientRect();
              d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu', {bubbles: true, cancelable: true, clientX: x - fr.left, clientY: y - fr.top})); }""", [x, y]); wait(900)
            assert "select-molecule" in menu_rows(), menu_rows()
            tap_row("select-molecule"); wait(300); assert len(sel()) >= 6, sel()
        step("with one atom selected the menu offers «Select molecule»", select_molecule)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
if any("FAIL" in s[1] for s in steps): sys.exit(1)
