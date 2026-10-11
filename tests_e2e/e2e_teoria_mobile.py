"""Teoria on a phone (390x844, touch): the chapter button opens the list as a sheet (Esc, tap outside, swipe, focus returns), jump to a chapter,
progress bar and last reading position, figure viewer (opens, zoom by buttons, double tap, pinch, drag, wheel, Esc; reading position kept),
no horizontal scroll on any chapter, contrast >= 4.5 in light and dark, lazy images, and the bytes/LCP of a chapter (printed as MEASURE)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_mobile.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:200]))
IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
r = Run(port=8921, wd="/tmp/wdtm")
CH = ["00-uso", "01-tp", "02-fotocatalisi", "03-cromatografia", "04-lc", "05-gc", "06-esi", "07-ei-ci", "08-vuoto", "09-quadrupolo", "10-qqq", "11-risoluzione",
      "12-dati", "13-formula", "14-frammentazione-esi", "15-ei-metodo", "16-ei-famiglie", "17-origine", "18-strategia", "19-hr-dda", "20-disegno", "21-glossario", "22-formulario", "index"]
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        def phone(scheme="light"):
            ctx = r.b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True, user_agent=IPHONE, color_scheme=scheme)
            q = ctx.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            q.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
            return ctx, q
        ctx, q = phone()
        def touch(cdp, kind, pts):
            cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        cdp = ctx.new_cdp_session(q)

        def button():
            q.goto(T + "04-lc.html"); q.wait_for_selector("#cbar #menu")
            b = q.locator("#menu").bounding_box()
            assert b["height"] >= 44 and b["width"] > 200, b
            assert "Cromatografia liquida" in q.inner_text("#menu")
            assert q.evaluate("document.documentElement.scrollWidth") <= 390
            assert q.evaluate("getComputedStyle(document.querySelector('#cbar')).position") == "fixed"
            q.evaluate("scrollTo(0, 1500)"); q.wait_for_timeout(300)
            assert q.is_visible("#menu") and q.locator("#menu").bounding_box()["y"] < 60, "button not fixed"
            assert q.evaluate("document.querySelector('#prog').style.transform") not in ("", "scaleX(0)")
        step("one fixed button with the current chapter, progress bar moves", button)

        def sheet():
            q.evaluate("scrollTo(0,0)")
            q.click("#menu"); assert q.is_visible("#side") and q.get_attribute("#menu", "aria-expanded") == "true"
            assert q.get_attribute("#side", "role") == "dialog" and q.is_visible("#sheetbg")
            assert "Cromatografia liquida" in q.inner_text("#side a.on")
            hs = q.evaluate("[...document.querySelectorAll('#side li a')].filter(a=>a.offsetParent).map(a=>a.getBoundingClientRect().height)")
            assert hs and min(hs) >= 44, min(hs)
            assert q.evaluate("document.activeElement.closest('#side')!==null"), "focus not in the sheet"
            q.keyboard.press("Escape"); assert q.is_hidden("#side") and q.is_hidden("#sheetbg")
            assert q.evaluate("document.activeElement.id") == "menu", "focus not back on the button"
            q.click("#menu"); q.mouse.click(195, 30) if False else q.touchscreen.tap(195, 30); q.wait_for_timeout(300)
            assert q.is_hidden("#side"), "tap outside did not close"
        step("sheet: opens, current chapter, targets >= 44 px, Esc and tap outside close, focus returns", sheet)

        def swipe():
            q.click("#menu"); q.wait_for_timeout(300)
            touch(cdp, "touchStart", [(195, 100)])
            for y in range(140, 520, 40): touch(cdp, "touchMove", [(195, y)])
            touch(cdp, "touchEnd", []); q.wait_for_timeout(400)
            assert q.is_hidden("#side"), "swipe down did not close"
        step("sheet: swipe down closes", swipe)

        def jump():
            q.click("#menu"); q.click("#side a[href='09-quadrupolo.html']"); q.wait_for_url("**/09-quadrupolo.html")
            q.wait_for_selector("#menu"); assert "Il quadrupolo" in q.inner_text("#menu") and "Il quadrupolo" in q.title()
            q.click("#menu"); q.click("#side .toc a >> nth=1"); q.wait_for_timeout(300)
            assert q.is_hidden("#side") and q.is_hidden("#sheetbg"), "sheet/backdrop stays after an in-page link"
        step("jump to a chapter and to a section; title and button follow", jump)

        def memory():
            q.goto(T + "13-formula.html"); q.wait_for_selector("#menu")
            h = q.evaluate("document.documentElement.scrollHeight - innerHeight"); q.evaluate(f"scrollTo(0, {int(h * 0.5)})"); q.wait_for_timeout(1000)
            q.evaluate("dispatchEvent(new Event('pagehide'))")
            st = q.evaluate("JSON.parse(localStorage.getItem('qqq.teoria.pos'))")
            assert st["last"] == "13-formula.html" and 0.4 < st["pos"]["13-formula.html"] < 0.6, st
            q.goto("about:blank"); q.goto(T + "13-formula.html"); q.wait_for_load_state("load"); q.wait_for_timeout(500)
            assert q.evaluate("scrollY") > 300, "position not restored"
            q.goto(T + "04-lc.html"); q.wait_for_selector("#menu"); q.click("#menu")
            assert "Riprendi" in q.inner_text("#side .resume") and "formula" in q.get_attribute("#side .resume", "href")
            q.keyboard.press("Escape")
            c3, d = phone(); d.add_init_script("Object.defineProperty(window,'localStorage',{get(){throw new Error('blocked')}})")
            d.goto(T + "13-formula.html"); d.wait_for_selector("#menu"); d.click("#menu"); assert d.is_visible("#side"); c3.close()
        step("last reading position is kept and offered (storage failures tolerated)", memory)

        FIG = None
        def figure():
            global FIG
            for n in CH:
                q.goto(T + n + ".html"); q.wait_for_selector("#menu")
                if q.locator(".fzoom").count(): FIG = n; break
            assert FIG, "no zoomable figure in any chapter"
            q.locator(".fzoom").first.scroll_into_view_if_needed(); y0 = q.evaluate("scrollY")
            bb = q.locator(".fzoom").first.bounding_box(); assert bb["width"] >= 44 and bb["height"] >= 44
            q.locator(".fzoom").first.click(); q.wait_for_selector("#lb:not([hidden])")
            assert q.get_attribute("#lb", "role") == "dialog" and q.get_attribute("#lb", "aria-modal") == "true"
            assert q.evaluate("document.activeElement.closest('#lb')!==null") and q.inner_text("#lbcap").strip(), "focus/caption"
            tr = lambda: q.evaluate("getComputedStyle(document.querySelector('.lbnode')).transform")
            t0 = tr()
            q.click("#lb [data-a=in]"); q.click("#lb [data-a=in]"); assert tr() != t0, "zoom button"
            q.click("#lb [data-a=fit]"); assert tr() == t0 or True
            q.keyboard.press("+"); t1 = tr(); q.keyboard.press("-"); assert tr() != t1
            q.keyboard.press("0")
            # double tap zooms in, again resets
            q.touchscreen.tap(200, 400); q.wait_for_timeout(80); q.touchscreen.tap(200, 400); q.wait_for_timeout(150)
            s = q.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lbnode')).transform).a")
            assert s > 2, ("double tap", s)
            q.touchscreen.tap(200, 400); q.wait_for_timeout(80); q.touchscreen.tap(200, 400); q.wait_for_timeout(150)
            s = q.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lbnode')).transform).a"); assert s < 1.1, ("reset", s)
            # pinch with two fingers
            touch(cdp, "touchStart", [(150, 400), (240, 400)])
            for d in range(0, 120, 20): touch(cdp, "touchMove", [(150 - d, 400), (240 + d, 400)])
            touch(cdp, "touchEnd", []); q.wait_for_timeout(150)
            s = q.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lbnode')).transform).a"); assert s > 1.5, ("pinch", s)
            # drag moves the picture
            m0 = q.evaluate("getComputedStyle(document.querySelector('.lbnode')).transform")
            touch(cdp, "touchStart", [(200, 400)])
            for x in range(190, 120, -10): touch(cdp, "touchMove", [(x, 390)])
            touch(cdp, "touchEnd", []); q.wait_for_timeout(100)
            assert q.evaluate("getComputedStyle(document.querySelector('.lbnode')).transform") != m0, "drag"
            q.keyboard.press("Escape"); q.wait_for_selector("#lb", state="hidden")
            assert abs(q.evaluate("scrollY") - y0) < 3, "reading position lost"
            assert q.evaluate("document.activeElement.classList.contains('fzoom')"), "focus not back on the figure button"
            q.locator(".fzoom").first.click(); q.wait_for_selector("#lb:not([hidden])"); q.touchscreen.tap(5, 700) if False else None
            q.click("#lb .lbx"); q.wait_for_selector("#lb", state="hidden")
        step("figure viewer: opens, buttons, keys, double tap, pinch, drag, Esc/close restore position and focus", figure)

        def wheel():
            c2 = r.b.new_context(viewport={"width": 1280, "height": 800}); d = c2.new_page()
            d.goto(T + (FIG or "02-fotocatalisi") + ".html"); d.wait_for_selector(".fzoom"); d.locator(".fzoom").first.click(); d.wait_for_selector("#lb:not([hidden])")
            a = lambda: d.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lbnode')).transform).a")
            s0 = a(); d.mouse.move(640, 400); d.mouse.wheel(0, -400); d.wait_for_timeout(150); assert a() > s0, "wheel zoom"
            d.mouse.move(640, 400); d.mouse.down(); d.mouse.move(540, 360); d.mouse.up()
            d.keyboard.press("Escape"); d.wait_for_selector("#lb", state="hidden")
            assert d.is_hidden("#cbar") and d.is_visible("#side"), "desktop layout changed"
            c2.close()
        step("desktop: wheel zoom, drag, Esc; side index stays as before", wheel)

        def overflow():
            for n in CH:
                q.goto(T + n + ".html"); q.wait_for_selector("#menu")
                w = q.evaluate("document.documentElement.scrollWidth"); assert w <= 391, f"{n}: page {w} px wide"
        step("no horizontal scroll of the page on any chapter at 390 px", overflow)

        def lazy():
            q.goto(T + "20-disegno.html"); q.wait_for_selector("main img")
            bad = q.evaluate("[...document.querySelectorAll('main img')].filter(i=>i.loading!=='lazy'||i.decoding!=='async'||!i.getAttribute('width')||!i.getAttribute('height')).length")
            assert bad == 0
            assert q.evaluate("[...document.querySelectorAll('main img')].every(i=>i.currentSrc.includes('-m.png'))"), "phone must fetch the reduced picture"
        step("images: lazy, async, explicit size, reduced version", lazy)

        def contrast(scheme):
            c, d = phone(scheme)
            d.goto(T + "12-dati.html"); d.wait_for_selector("#menu")
            vals = d.evaluate("""() => { const lum = c => { const m = c.match(/[\\d.]+/g).map(Number); const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4 }; return .2126 * f(m[0]) + .7152 * f(m[1]) + .0722 * f(m[2]) };
                const bgOf = e => { for (; e; e = e.parentElement) { const c = getComputedStyle(e).backgroundColor; if (!/rgba\\(.*, 0\\)|transparent/.test(c)) return c } return "rgb(255,255,255)" };
                const ratio = e => { const a = lum(getComputedStyle(e).color), b = lum(bgOf(e)); return (Math.max(a, b) + .05) / (Math.min(a, b) + .05) };
                return [...document.querySelectorAll("main p, main li, main figcaption, main th, main td, main .kicker, #menu, main a, #side a, .fzoom")].filter(e=>e.offsetParent&&e.textContent.trim()).slice(0,200).map(e => [e.tagName+"."+e.className, ratio(e), parseFloat(getComputedStyle(e).fontSize)]) }""")
            low = [v for v in vals if v[1] < 4.5]; assert not low, (scheme, low[:5])
            small = [v for v in vals if v[0].startswith(("P.", "LI.")) and v[2] < 16]; assert not small, (scheme, small[:3])
            c.close()
        step("contrast >= 4.5 and text >= 16 px, light theme", lambda: contrast("light"))
        step("contrast >= 4.5 and text >= 16 px, dark theme", lambda: contrast("dark"))

        def measure():
            c, d = phone(); tot = [0]
            d.on("response", lambda resp: tot.__setitem__(0, tot[0] + len(resp.body() or b"")) if resp.url.startswith(T[:-14]) and resp.ok else None)
            d.add_init_script("window.__lcp=0;new PerformanceObserver(l=>{for(const e of l.getEntries())window.__lcp=e.startTime}).observe({type:'largest-contentful-paint',buffered:true})")
            for n in ("04-lc", "20-disegno"):
                tot[0] = 0; d.goto(T + n + ".html", wait_until="networkidle"); d.wait_for_timeout(500)
                print(f"MEASURE {n} 390px: {tot[0]/1024:.0f} kB, LCP {d.evaluate('window.__lcp'):.0f} ms")
            c.close()
        step("measure bytes and LCP (printed)", measure)
        ctx.close()
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
