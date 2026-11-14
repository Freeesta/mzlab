"""Safari / iPad (docs/agenti/compat-safari.md): the site on an emulated iPad (touch only, no hover) is the full site, not the phone card; the file
picker of «Load» opens with every file (compat.js frees the accept list that iPadOS would grey out) and an example file opens with its charts; the page
keeps its width at Split View 1/3 and 1/2 and after a rotation; a tap outside closes the menus and pop-ups; a long press on a button shows its tooltip and the lift does not run it; the side bar handle is 24 px wide and a double tap on a control does not zoom; the legend chip of many traces opens by tap and keyboard; a private window
(storage refused) gets its own loading message; with AbortSignal.any and createWritable taken away (what Safari before 17.4 and 26
lacks) compat.js stands in and an OPFS file written through createWritable reads back the same. No browser errors and no answer of 400 or more.
The real engine of Safari is WebKit: with --browser webkit (the nightly round on macOS) every step runs in WebKit; the cloud has only Chromium, which
emulates the iPad (touch, coarse pointer, mobile viewport) and the output says so."""
# covers: mzlab/web/compat, mzlab/web/capacita, mzlab/web/tooltip.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_safari_ipad.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8853, wd="/tmp/wd_safari_ipad")
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}"
# what Safari before 17.4 / 26 lacks, taken away before any script of the page runs
TAKE_AWAY = "try{delete AbortSignal.any;delete FileSystemFileHandle.prototype.createWritable;}catch(_){}"
# a private window of Safari: the private storage of the site is refused
NO_STORAGE = "try{const e=()=>Promise.reject(Object.assign(new Error('The operation is insecure.'),{name:'SecurityError'}));Object.defineProperty(StorageManager.prototype,'getDirectory',{value:e,configurable:true});}catch(_){}"
WIDE = "document.documentElement.scrollWidth - document.documentElement.clientWidth"
# the visual viewport of iOS, which alone shrinks when the soft keyboard comes up: __kb(height, top) moves it
FAKE_VV = """(() => { const t = new EventTarget(), st = { h: null, top: 0 };
  Object.defineProperties(t, { height: { get: () => st.h ?? innerHeight }, width: { get: () => innerWidth }, offsetTop: { get: () => st.top }, offsetLeft: { value: 0 },
    pageTop: { get: () => st.top }, pageLeft: { value: 0 }, scale: { value: 1 } });
  Object.defineProperty(window, "visualViewport", { value: t, configurable: true });
  window.__kb = (h, top) => { st.h = h; st.top = top; t.dispatchEvent(new Event("resize")); }; })()"""

def ipad(b, w=1180, h=820, extra=""):
    k = dict(viewport={"width": w, "height": h}, has_touch=True, device_scale_factor=2, reduced_motion="reduce")
    if BROWSER != "firefox": k["is_mobile"] = True              # Firefox has no mobile emulation
    c = b.new_context(**k); c.add_init_script(SKIP_TOUR)
    if extra: c.add_init_script(extra)
    pg = c.new_page()
    pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
    pg.on("response", lambda x: r.errs.append((f"http {x.status}", x.url)) if x.status >= 400 else None)   # a worker's 404 is a console error in WebKit only
    pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg); return pg

if BROWSER == "firefox":          # Playwright's Firefox cannot be a touch-only device (no isMobile, the pointer stays fine): nothing here would be an iPad
    print("  skipped in Firefox: it cannot emulate a touch-only device; the iPad steps run in WebKit and Chromium"); r.srv.terminate(); sys.exit(0)
try:
    from playwright.sync_api import sync_playwright
    print(f"  browser: {BROWSER}" + ("" if BROWSER == "webkit" else " (iPad emulated; the WebKit round is the nightly --browser webkit)"))
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        S = {}
        def full_site():
            pg = S["pg"] = ipad(b)
            assert pg.evaluate("matchMedia('(hover: none) and (pointer: coarse)').matches"), "the emulated iPad has a hover or a fine pointer"
            assert pg.evaluate("window.MZLAB_PHONE") is False and pg.locator("#drop").is_visible(), "an iPad got the phone card"
            assert "accept" in pg.evaluate("window.__compat || []"), pg.evaluate("window.__compat")
        step("iPad: the full site, compat.js installed", full_site)
        def picker():
            pg = S["pg"]
            with pg.expect_file_chooser() as fc: pg.evaluate("document.getElementById('pick').click()")
            assert pg.get_attribute("#pick", "accept") is None, "the accept list is still on the input"
            assert ".wiff" in (pg.get_attribute("#pick", "data-accept") or ""), "the list is not kept in data-accept"
            fc.value.set_files([str(EX / "FullScan_t0.mzML")])
            wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1", 60000)
            pg.click("#opbtn"); ready(pg)
            wait_for_js(pg, "E.panels.length > 0 && E.panels.some(q => q.el.querySelector('canvas') && q.el.querySelector('canvas').width > 0)", 60000)
        step("iPad: «Load» shows every file and an example file opens with its charts", picker)
        def widths():
            pg = S["pg"]; out = []
            for w, h in ((1180, 820), (820, 1180), (375, 820), (590, 820), (1180, 820)):      # landscape, portrait, Split View 1/3 and 1/2, back
                pg.set_viewport_size({"width": w, "height": h}); wait_for_js(pg, f"innerWidth === {w}"); frames(pg, 3); ready(pg)
                out.append((w, pg.evaluate(WIDE), pg.evaluate("E.panels[0].el.getBoundingClientRect().width")))
            print("  width, horizontal overflow, first panel width:", out)
            assert all(o <= 1 for _, o, _ in out), f"the page scrolls sideways: {out}"
            assert all(0 < pw <= w for w, _, pw in out), f"a panel is wider than the window: {out}"
            assert pg.evaluate("window.MZLAB_PHONE") is False
        step("iPad: rotation and Split View 1/3, 1/2 keep the page in the window", widths)
        def legend_chip():
            pg = S["pg"]
            pg.click("#np-xic"); pg.fill("#xic-mz", "200"); pg.click("#xic-go"); ready(pg)
            for mz in ("250", "300", "350", "400"):
                pg.evaluate("openXic(E.panels.find(p=>p.type==='xic'))"); pg.fill("#xic-mz", mz); pg.click("#xic-go"); ready(pg)
            wait_for_js(pg, "!!document.querySelector('.pnl.xic .leg .lgc')")
            chip, lst = pg.locator(".pnl.xic .leg .lgc").first, pg.locator(".pnl.xic .leg .lgl").first
            assert pg.evaluate("document.querySelector('.pnl.xic .leg .lgc').tagName") == "BUTTON"
            assert chip.bounding_box()["height"] >= 24, chip.bounding_box()
            assert not lst.is_visible() and chip.get_attribute("aria-expanded") == "false"
            chip.scroll_into_view_if_needed(); chip.tap()                              # a finger has no hover: the tap opens the list and keeps it open
            assert lst.is_visible() and chip.get_attribute("aria-expanded") == "true"
            chip.hover()                                                                # Safari keeps :hover on the tapped element: the list must still close
            chip.focus(); pg.keyboard.press("Escape")
            assert not lst.is_visible() and chip.get_attribute("aria-expanded") == "false"
            pg.keyboard.press("Enter"); assert lst.is_visible()
            pg.keyboard.press("Enter"); assert not lst.is_visible()
        step("iPad: the legend chip of many traces opens by tap and Enter, Escape closes it", legend_chip)
        def drawer_fits():
            pg = S["pg"]; pg.set_viewport_size({"width": 1024, "height": 768}); wait_for_js(pg, "innerWidth === 1024"); frames(pg, 3)
            pg.evaluate("BARRA.show('file',{noFocus:true,instant:true}); document.getElementById('samptab').click()")
            wait_for_js(pg, "(t => !!t && t.offsetWidth > 0)(document.querySelector('#flist .ldtab'))", 20000); frames(pg, 2)
            fw = pg.evaluate("(t => [...t.rows[0].cells].find(c => /^File$/.test(c.textContent.trim())).offsetWidth)(document.querySelector('#flist .ldtab'))")
            pg.evaluate("document.getElementById('samptab').click()")                   # back to the charts: the toggle is left as found
            pg.evaluate("BARRA.show('calc',{noFocus:true,instant:true})"); wait_for_js(pg, "document.querySelector('#al-tabs').offsetWidth > 0", 20000); frames(pg, 2)
            tabs = pg.evaluate("(a => [a.clientWidth, a.scrollWidth])(document.querySelector('#al-tabs'))")
            pg.evaluate("BARRA.close()"); pg.set_viewport_size({"width": 1180, "height": 820}); wait_for_js(pg, "innerWidth === 1180")
            assert fw >= 120, f"the File column of the sample table is {fw} px wide at 1024 px"
            assert tabs[1] <= tabs[0], f"the calculator tabs are clipped (box, content): {tabs}"
        step("iPad 1024x768: the File column of the sample table is readable, the calculator tabs fit the drawer", drawer_fits)
        def targets():
            pg = S["pg"]; small = []
            def size(sel, opener, closer="(()=>{for(const d of document.querySelectorAll('dialog[open]'))d.close()})()"):
                pg.evaluate(opener); wait_for_js(pg, f"(e => !!e && e.offsetWidth > 0)(document.querySelector({sel!r}))", 20000); frames(pg, 2)
                w, h = pg.evaluate(f"(r => [r.width, r.height])(document.querySelector({sel!r}).getBoundingClientRect())")
                if w < 24 or h < 24: small.append((sel, round(w), round(h)))
                pg.evaluate(closer)
            # what the compact rules shrink: a panel title (a double tap renames it), a fold, a check box in its label, the × of an XIC row, while the charts are on screen
            size(".pnl.spec .hd .ttl", "void 0", "void 0")
            size("#bigbody summary", "document.getElementById('np-method').click()")
            size("#lst-src-checks label", "BARRA.show('lists',{noFocus:true,instant:true})", "BARRA.close()")
            size("#xic-rows .xr-x", "document.getElementById('np-xic').click()")
            size("#bigx", "peakParams()")
            size("#lib-x", "LIB.open()")
            size("#refx", "document.getElementById('np-pt').click()")
            size("#flist input[data-k=use]", "BARRA.show('file',{noFocus:true,instant:true}); document.getElementById('samptab').click()", "BARRA.close()")
            size("#tour-btn-skip", "TOUR.inizia(1,0)", "document.getElementById('tour-btn-skip').click()")
            assert not small, f"touch targets under 24x24 px: {small}"
        step("iPad: the × of the dialogs, «Skip the tour», the check boxes, folds, panel titles and XIC rows are at least 24x24 px", targets)
        def tap_outside():
            pg = ipad(b, extra=IOS_TOUCH); cdp = pg.context.new_cdp_session(pg) if BROWSER == "chromium" else None
            def touch(x, y, ms=0):
                if not cdp:     # WebKit has no CDP: a tap is Playwright's, a long press is the right click it ends in (touch.js)
                    if ms: pg.evaluate(f"document.elementFromPoint({x}, {y}).dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true, cancelable: true, clientX: {x}, clientY: {y} }}))")
                    else: pg.touchscreen.tap(x, y)
                    hold(pg, 300); return
                cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": [{"x": x, "y": y}]})
                if ms: hold(pg, ms)
                cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); hold(pg, 300)
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t5.mzML")])
            pg.click("#opbtn"); ready(pg); wait_for_js(pg, "E.panels.some(q => q.type === 'chrom' && q._a)", 60000); ready(pg)
            pg.evaluate("BARRA.close()"); ready(pg)
            cv = pg.locator(".pnl.chrom canvas").first; cv.scroll_into_view_if_needed(); bb = cv.bounding_box()
            at = lambda fx: (bb["x"] + bb["width"] * fx, bb["y"] + bb["height"] * 0.5)
            # a point of the page that answers no click (the margin below the panels): iOS sends no click there
            empty = pg.evaluate("""(() => { for (let y = innerHeight - 8; y > innerHeight / 2; y -= 12) for (const x of [12, innerWidth / 2, innerWidth - 12]) {
              const e = document.elementFromPoint(x, y); if (e === document.documentElement || e === document.body) return [x, y]; } return null; })()""")
            assert empty, "no empty point of the page to tap"
            ctx = "(c => !!c && !c.hidden && c.offsetWidth > 0)(document.getElementById('ctx'))"
            left = []
            for name, opener, isopen, where in (
                    ("chart menu, tap on the chart", lambda: touch(*at(0.5), 800), ctx, at(0.25)),
                    ("chart menu, tap outside", lambda: touch(*at(0.5), 800), ctx, empty),
                    ("file colour", lambda: pg.evaluate("colorFile(E.files[0], { clientX: 300, clientY: 200 })"), "!!document.getElementById('colpop')", empty),
                    ("file chooser of a graph", lambda: pg.evaluate("fileMenu(document.querySelector('.pnl .hd button'))"), "!!document.getElementById('fpop')", empty),
                    ("settings", lambda: touch(*(lambda r: (r["x"] + r["width"] / 2, r["y"] + r["height"] / 2))(pg.locator("#np-set").bounding_box())), "!!document.getElementById('uipset')", empty)):
                opener(); wait_for_js(pg, isopen, 5000)
                touch(*where)
                if pg.evaluate(isopen): left.append(name)
                for _ in range(2): pg.keyboard.press("Escape")
                pg.evaluate("(() => { const c = document.getElementById('ctx'); if (c) c.hidden = true; for (const s of ['#colpop', '#fpop', '#uipset']) document.querySelector(s)?.remove(); })()")
            assert not left, f"still open after a tap outside them: {left}"
            # a tap that comes before any timer of the page has run (a busy page) closes them too; the touch that opened one does not close it
            TAP = "document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerType: 'touch' }))"
            bad = []; SP = "E.panels.find(q => q.type === 'spec' && q.el.querySelector('[data-a=par]'))"
            for sel, opener in (("#colpop", "colorFile(E.files[0], { clientX: 300, clientY: 200 })"), ("#fpop", "fileMenu(document.querySelector('.pnl .hd button'))"),
                                ("#uipset", "uipOpen(document.getElementById('np-set'))"), (".pnl > .sp-pop", f"(q => specParams(q, q.el.querySelector('[data-a=par]')))({SP})"),
                                (".aff-menu", "(q => affOpenMenu(q, q._grip))(E.panels.find(q => q._grip))")):
                shut = f"affCloseMenu(false); document.querySelector({sel!r})?.remove()"          # the menu of «side by side» keeps a reference: closed by its own function
                quick = pg.evaluate(f"(() => {{ {opener}; {TAP}; const o = !!document.querySelector({sel!r}); {shut}; return o; }})()")
                kept = pg.evaluate(f"(() => {{ document.body.addEventListener('pointerdown', () => {opener}, {{ once: true }}); {TAP}; const o = !!document.querySelector({sel!r}); {shut}; return o; }})()")
                if quick: bad.append(f"{sel}: a tap right after the opening leaves it open")
                if not kept: bad.append(f"{sel}: the touch that opened it closed it")
            assert not bad, bad
            touch(bb["x"] + bb["width"] - 4, bb["y"] + bb["height"] * 0.5, 800); wait_for_js(pg, ctx, 5000)       # opened at the right edge, the menu stays in the window
            right = pg.evaluate("document.getElementById('ctx').getBoundingClientRect().right - innerWidth")
            assert right <= 0, f"the chart menu sticks out of the window by {right} px"
            pg.context.close()
        step("iPad: a tap outside closes the menus and pop-ups (iOS sends no click to a plain area, touch.js keeps the click of a chart)", tap_outside)
        def long_press_tip():
            pg = ipad(b)
            cdp = pg.context.new_cdp_session(pg) if BROWSER == "chromium" else None
            LIFT = "(b => ['mouseover', 'mousedown', 'mouseup', 'click'].forEach(t => b.dispatchEvent(new MouseEvent(t, { bubbles: true, cancelable: true, view: window }))))"
            def touch(kind, x, y, sel=None):
                if cdp: cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [] if kind == "touchEnd" else [{"x": x, "y": y}]}); return
                # WebKit has no CDP: the pointer events of a finger, and on the lift what iOS sends to a button (its mouse events and the click)
                pg.evaluate(f"""(e => {{ e.dispatchEvent(new PointerEvent({'pointerdown' if kind == 'touchStart' else 'pointerup'!r}, {{ bubbles: true, pointerType: 'touch', clientX: {x}, clientY: {y} }}));
                  if ({'true' if kind == 'touchEnd' and sel else 'false'}) {LIFT}(e); }})(document.elementFromPoint({x}, {y}))""")
            pg.evaluate("window.__lpClicks = 0; document.getElementById('np-pt').addEventListener('click', () => window.__lpClicks++)")
            bb = pg.locator("#np-pt").bounding_box(); x, y = bb["x"] + bb["width"] / 2, bb["y"] + bb["height"] / 2
            touch("touchStart", x, y)
            # held well past the 0.6 s of the tooltip; frames keep coming meanwhile (a still headless page delivers the touch only with the next frame)
            pg.evaluate("new Promise(r => { const t0 = performance.now(); (function f() { performance.now() - t0 > 1500 ? r() : requestAnimationFrame(f); })(); })")
            shown = pg.evaluate("!document.getElementById('qtip').hidden")
            touch("touchEnd", x, y, "#np-pt"); frames(pg, 2)                           # the lift: a finger sends the click of the button (Chromium and iOS alike)
            clicks = pg.evaluate("window.__lpClicks")
            pg.evaluate("(()=>{for(const d of document.querySelectorAll('dialog[open]'))d.close()})()")
            pg.evaluate("document.getElementById('qtip').hidden = false")              # whatever hid it, the next touch must hide a tooltip that is up
            touch("touchStart", 600, 500); touch("touchEnd", 600, 500); frames(pg, 2)
            left = pg.evaluate("!document.getElementById('qtip').hidden")
            pg.context.close()
            assert shown, "a long press on a button shows no tooltip"
            assert clicks == 0, "the lift after a long press ran the button (the periodic table opened)"
            assert not left, "the tooltip of the long press stays after the next touch"
        step("iPad: a long press on a button shows its tooltip, the lift does not run it, the next touch hides it", long_press_tip)
        def touch_css():
            pg = S["pg"]
            pg.evaluate("BARRA.show('file',{noFocus:true,instant:true})"); wait_for_js(pg, "(h => !!h && h.offsetWidth > 0)(document.getElementById('sidebar-resize-handle'))", 20000); frames(pg, 2)
            hw = pg.evaluate("document.getElementById('sidebar-resize-handle').getBoundingClientRect().width")
            ta = pg.evaluate("""[...document.querySelectorAll('#tools button, header button, #sb-header button, #flist input, select')].filter(e => e.offsetWidth > 0)
                .map(e => [e.id || e.tagName, getComputedStyle(e).touchAction]).filter(([, t]) => t !== 'manipulation')""")
            canv = pg.evaluate("getComputedStyle(document.querySelector('.pnl canvas')).touchAction")
            pg.evaluate("BARRA.close()")
            assert hw >= 24, f"the side bar handle is {hw} px wide on a touch screen"
            assert not ta, f"controls a double tap would zoom: {ta[:6]}"
            assert canv == "none", f"the chart canvas lost its own gestures: {canv}"
        step("iPad: the side bar handle is 24 px wide, a double tap on a control does not zoom the page, the charts keep their gestures", touch_css)
        def keyboard():
            # Chromium has no soft keyboard: the visual viewport of iOS is stood in for, and __kb(height, top) is the keyboard coming up (top: iOS scrolled to the field)
            pg = ipad(b, 1024, 768, extra=FAKE_VV)
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")])
            pg.click("#opbtn"); ready(pg); wait_for_js(pg, "E.panels.some(q => q.type === 'chrom' && q._a)", 60000); ready(pg)
            pg.touchscreen.tap(8, innerh := pg.evaluate("innerHeight") - 8); wait_for_js(pg, "!!window.TOCCO_CONTROLLI", 20000)      # the first touch loads tocco-controlli.js
            box = lambda sel: pg.evaluate(f"(r => [r.top, r.bottom])(document.querySelector({sel!r}).getBoundingClientRect())")
            got = {}
            for name, opener, parts in (("XIC", "document.getElementById('np-xic').click()", ["#xicdlg", "#xic-rows input", "#xic-go"]),
                                        ("library", "LIB.open()", ["#libdlg"])):
                pg.evaluate(opener); wait_for_js(pg, f"(d => !!d && d.open)(document.querySelector({parts[0]!r}))", 20000); frames(pg, 2)
                for top in (0, 200):
                    pg.evaluate(f"__kb({innerh - 400 - top}, {top})"); frames(pg, 3)
                    got[(name, top)] = ([box(s) for s in parts], pg.evaluate(f"(d => d.scrollHeight > d.clientHeight + 1)(document.querySelector({parts[0]!r}))"))
                pg.evaluate(f"__kb({innerh}, 0)"); frames(pg, 3)
                got[(name, "down")] = (box(parts[0]), pg.evaluate("document.documentElement.classList.contains('kb')"))
                pg.evaluate("for (const d of document.querySelectorAll('dialog[open]')) d.close()")
            pg.context.close()
            for (name, top), v in got.items():
                if top == "down": assert abs(v[0][0] - (innerh + 8 - v[0][1])) <= 1 and not v[1], f"{name}: the dialog is not centred again when the keyboard goes {v}"; continue   # a modal dialog is centred: the same margin above and below
                band = (top, innerh - 400)
                out = [p for p, (a, z) in zip(["dialog", "field", "button"], v[0]) if (a < band[0] - 1 or z > band[1] + 1) and not (p == "button" and v[1])]
                assert not out, f"{name}, keyboard up (visible band {band}): {out} outside it {v[0]} (a button may be under the scroll of a dialog that scrolls)"
            assert got[("library", 0)][1], "the library dialog is cut by the keyboard and does not scroll"
        step("iPad: with the soft keyboard up a dialog stays in the part of the window left above it, its field and its button in sight", keyboard)
        def fallbacks():
            pg = ipad(b, extra=TAKE_AWAY)
            got = pg.evaluate("window.__compat || []")
            assert "AbortSignal.any" in got and "createWritable" in got, got
            back = pg.evaluate("""async () => {
              const a = new AbortController(), s = AbortSignal.any([a.signal, new AbortController().signal]); a.abort('r');
              const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('compat-prova', { create: true });
              const fh = await d.getFileHandle('t.txt', { create: true }), w = await fh.createWritable();
              await w.write('m/z '); await w.write(new Blob(['301.1'])); await w.close();
              const text = await (await fh.getFile()).text();
              await (await navigator.storage.getDirectory()).removeEntry('compat-prova', { recursive: true });
              return [s.aborted && s.reason === 'r', text]; }""")
            assert back == [True, "m/z 301.1"], back
        step("without AbortSignal.any and createWritable: compat.js stands in (OPFS write reads back)", fallbacks)
        def private_window():
            k = dict(viewport={"width": 1180, "height": 820}, has_touch=True, reduced_motion="reduce")
            if BROWSER != "firefox": k["is_mobile"] = True
            c = b.new_context(**k); c.add_init_script(SKIP_TOUR)
            c.add_init_script(NO_STORAGE)
            pg = c.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/")
            wait_for_js(pg, "!!window.qqFailed", 60000)
            assert pg.evaluate("window.qqFailed") == "load.err.noStorage", pg.evaluate("[window.qqFailed, window.MZLAB_LR_ERR]")
            wait_for_js(pg, "/finestra privata|private window/.test(document.body.innerText)", 10000)     # the reader sees it, in the language of the page
            c.close()
        step("a private window (the browser storage refused): the loading screen says so", private_window)
        b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:5])
sys.exit(1 if bad or r.errs else 0)
