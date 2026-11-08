"""Touch outside the graphs (tocco-controlli.js, mappa.js): a finger double tap on a panel title or a legend trace opens the rename dialog once, as a mouse double
click does; a slow pair or a moved finger does nothing; the page has no double-tap zoom; a long press on a file group header opens its menu, which stays open
after the lift; a tap on a label of the map's side spectrum shows its S/N. Real touches (CDP Input.dispatchTouchEvent), anonymous example files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tocco_controlli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8849, wd="/tmp/wd_tocco_controlli")
SP = "E.panels.find(p => p.type === 'spec')"
XP = "E.panels.find(p => p.type === 'xic')"
MP = "E.panels.find(p => p.type === 'map')"
EX = ROOT / "mzlab" / "web" / "esempi"
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(locale="en-US", viewport={"width": 1280, "height": 900}, has_touch=True)
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); frames(pg, 30)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def wait(ms=250): hold(pg, ms)        # gestures are timed (double tap): real pauses
        def dtap(x, y, gap=110, dx=0):
            T("touchStart", [(x, y)]); T("touchEnd", []); wait(gap); T("touchStart", [(x + dx, y)]); T("touchEnd", []); wait(600)
        pg.set_input_files("#pick", [str(EX / f"FullScan_t{t}.mzML") for t in (0, 15, 30)])      # a group of two files has a header; frames(pg, 30)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-xic"); pg.fill("#xic-mz", "200"); pg.click("#xic-go"); ready(pg)
        pg.evaluate(f"openXic({XP})"); pg.fill("#xic-mz", "250"); pg.click("#xic-go"); ready(pg)      # a second trace: the legend lists them
        T("touchStart", [(4, 4)]); T("touchEnd", []); wait_for_js(pg, "!!window.TOCCO_CONTROLLI", 15000)
        # every rename dialog opened is counted: a double tap must open it once, not twice
        pg.evaluate("(() => { window._opened = 0; const o = HTMLDialogElement.prototype.showModal; HTMLDialogElement.prototype.showModal = function () { if (this.id === 'askdlg') _opened++; return o.call(this); }; })()")
        def centre(sel):
            pg.evaluate(f"{sel}.scrollIntoView({{block:'center'}})"); frames(pg, 6)
            return pg.evaluate(f"(() => {{ const r = {sel}.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }})()")
        opened = lambda: pg.evaluate("_opened")
        def close():
            pg.evaluate("for (const d of document.querySelectorAll('dialog[open]')) d.close()"); frames(pg, 4)
        def no_zoom():
            assert pg.evaluate("getComputedStyle(document.documentElement).touchAction") == "manipulation"
        step("the page has no double-tap zoom (pinch stays)", no_zoom)
        def title():
            x, y = centre(f"{SP}.el.querySelector('.ttl')"); n = opened(); dtap(x, y)
            assert opened() == n + 1 and pg.evaluate("Q('#askdlg').open"), ("one rename dialog", opened() - n)
            close()
        step("a double tap on a panel title opens its rename dialog once", title)
        def legend():
            x, y = centre(f"{XP}.leg.querySelector('[data-t]')"); n = opened(); dtap(x, y)
            assert opened() == n + 1 and pg.evaluate("Q('#askdlg').open"), ("one rename dialog", opened() - n)
            close()
        step("a double tap on a legend trace opens its rename dialog once", legend)
        def not_double():
            x, y = centre(f"{SP}.el.querySelector('.ttl')"); n = opened()
            dtap(x, y, gap=700); dtap(x, y, dx=40)
            assert opened() == n, ("a slow pair or two places is not a double tap", opened() - n)
        step("two slow taps, or taps far apart, are not a double tap", not_double)
        def mouse_same():
            x, y = centre(f"{SP}.el.querySelector('.ttl')"); n, f = opened(), pg.evaluate("TOCCO_CONTROLLI.fired()")
            pg.mouse.dblclick(x, y); frames(pg, 8)
            assert opened() == n + 1 and pg.evaluate("TOCCO_CONTROLLI.fired()") == f, "the mouse double click is the browser's own, untouched"
            close()
        step("a mouse double click works as before", mouse_same)
        def as_ios():
            pg.evaluate("addEventListener('dblclick', e => { if (e.isTrusted) e.stopImmediatePropagation(); }, true)")      # iOS sends no dblclick for a double tap
            x, y = centre(f"{XP}.el.querySelector('.ttl')"); n = opened(); dtap(x, y)
            assert opened() == n + 1, ("the double tap alone opens it", opened() - n)
            close()
        step("without the browser's own double click (as on iOS) a double tap still works", as_ios)
        def group_menu():
            pg.evaluate("async () => { await carica('gruppi.js'); return GRUPPI.set([fileBase(E.files[0].file), fileBase(E.files[1].file)], 'Blanks') }"); ready(pg); drawer(pg)
            pg.wait_for_selector('#flst .fgh[data-ug="Blanks"]', state="attached", timeout=10000)
            x, y = centre("document.querySelector('#flst .fgh[data-ug=\"Blanks\"]')")
            pg.mouse.click(x, y, button="right"); frames(pg, 6)
            assert not pg.evaluate("Q('#ctx').hidden"), "the right click opens the menu"
            n = pg.evaluate("Q('#ctx').querySelectorAll('[role^=menuitem]').length"); pg.keyboard.press("Escape"); frames(pg, 6)
            pg.evaluate("addEventListener('contextmenu', e => { if (e.isTrusted) e.stopImmediatePropagation(); }, true)")      # iOS sends no contextmenu for a long press
            T("touchStart", [(x, y)]); T("touchEnd", []); wait(700)
            assert pg.evaluate("Q('#ctx').hidden"), "a short tap is not a long press"
            T("touchStart", [(x, y)]); pg.wait_for_function("!Q('#ctx').hidden", timeout=4000); T("touchEnd", []); wait(500)
            assert not pg.evaluate("Q('#ctx').hidden"), "the menu stays open after the lift"
            assert pg.evaluate("Q('#ctx').querySelectorAll('[role^=menuitem]').length") == n, "the same menu as the right click"
            pg.keyboard.press("Escape"); frames(pg, 6)
        step("a long press on a file group header opens its menu", group_menu)
        def map_label():
            pg.click("#np-map"); ready(pg); frames(pg, 20)
            pg.evaluate(MP + ".el.scrollIntoView({block:'center'})"); pg.evaluate(MP + ".cv.focus()"); pg.keyboard.press("f"); wait(900); ready(pg)
            c = pg.evaluate(f"(() => {{ const r = {MP}.cv.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }})()")
            pg.mouse.move(c[0], c[1]); wait(500); ready(pg); wait(300)                     # the mouse picks the scan of the side spectrum
            L = pg.evaluate(f"(() => {{ const s = {MP}._mz, r = s.sp.getBoundingClientRect(); return (s.lbls || []).filter(l => s.snr && s.snr.has(l.i)).map(l => [r.left + l.x + l.w / 2, r.top + l.y + l.h / 2]); }})()")
            assert L, "the side spectrum has labels"
            x, y = L[0]; pg.mouse.move(x, y); wait(200)
            tip = pg.evaluate(f"{MP}._mz.sp.title"); assert "S/N" in tip and pg.evaluate(f"{MP}.tip.hidden"), ("the mouse keeps the title tooltip", tip)
            T("touchStart", [(x, y)]); T("touchEnd", []); wait(400)
            assert pg.evaluate(f"[{MP}.tip.hidden, {MP}.tip.textContent]") == [False, tip], "a tap shows the same text"
            wait(3000); assert pg.evaluate(f"{MP}.tip.hidden"), "and it goes after a moment"
        step("a tap on a label of the map's side spectrum shows its S/N", map_label)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
