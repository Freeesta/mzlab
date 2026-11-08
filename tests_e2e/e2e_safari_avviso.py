"""Safari first-visit notice (docs/agenti/compat-safari.md, avviso-safari.js): Safari may delete the saved data after 7 days without a visit, the original
files are never touched; on a Mac it recommends another browser, on an iPad (all WebKit) it speaks of the Home Screen instead. It is shown once per browser
storage (flag in localStorage), again after the storage is cleared, also when the storage throws; never in the other browsers. Safari is told by
WebKit's GestureEvent: with --browser webkit it is WebKit's own, elsewhere a stand-in is set before the page runs (a WebKit-like page; the WebKit
round itself is webkit-mac.yml / notte.yml). The tour invitation stays usable beside it. No browser errors."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_safari_avviso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8906, wd="/tmp/wd_safari_avviso")
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}"
GESTURE = "if(!window.GestureEvent){window.GestureEvent=function GestureEvent(){};}"
BOX = "#safaribox"
STATE = """() => { const b = document.querySelector('#safaribox'), x = b && b.querySelector('button.x'), q = x && x.getBoundingClientRect(), s = b && b.getBoundingClientRect();
  let seen = null; try { seen = localStorage.getItem('mzlab.safari.seen'); } catch (_) {}
  return { on: !!b, text: b ? b.textContent : '', role: b && b.getAttribute('role'), live: b && b.getAttribute('aria-live'), css: b ? b.style.cssText : '',
           xw: q ? q.width : 0, xh: q ? q.height : 0, xlabel: x && x.getAttribute('aria-label'), focus: !!(b && b.contains(document.activeElement)),
           right: s ? innerWidth - s.right : -1, left: s ? s.left : -1, kind: b && b.dataset.kind, seen }; }"""

def ctx(b, gesture=True, tour=False, **k):
    k.setdefault("viewport", {"width": 1440, "height": 900})
    c = b.new_context(reduced_motion="reduce", safari_notice=True, **k)
    if not tour: c.add_init_script(SKIP_TOUR)
    else: c.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    if gesture: c.add_init_script(GESTURE)
    pg = c.new_page()
    pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
    return c, pg

def shown(pg):
    wait_for_js(pg, f"!!document.querySelector('{BOX}')", 30000)
    return pg.evaluate(STATE)

try:
    from playwright.sync_api import sync_playwright
    print(f"  browser: {BROWSER}" + ("" if BROWSER == "webkit" else " (GestureEvent stubbed: a WebKit-like page)"))
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        c, pg = ctx(b)
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        def first():
            s = shown(pg)
            assert s["kind"] == "mac" and "Chrome, Edge o Firefox" in s["text"] and "7 giorni" in s["text"] and "file originali" in s["text"], s
            assert "non funziona" not in s["text"].lower(), s["text"]
            assert s["role"] == "status" and s["live"] == "polite", s
            assert not s["focus"], "the notice took the focus"
            assert "var(--panel)" in s["css"] and "var(--ink)" in s["css"] and "var(--line)" in s["css"], s["css"]
            assert s["xw"] >= 24 and s["xh"] >= 24 and s["xlabel"], s
            assert 0 <= s["right"] <= 20, f"the notice is not in the bottom right corner: {s}"
            assert s["seen"] == "1", s
        step("first visit: the calm notice (IT), polite status, no focus taken, colours from variables, X of 24 px", first)
        def keyboard():
            pg.focus(f"{BOX} button.x"); pg.keyboard.press("Escape")
            wait_for_js(pg, f"!document.querySelector('{BOX}')", 5000)
            assert pg.evaluate("!!document.querySelector('header')"), "the page is gone"
        step("keyboard: Esc inside the notice closes it", keyboard)
        def again():
            pg.reload(); ready(pg); hold(pg, 1500)
            assert not pg.evaluate(STATE)["on"], "the notice came back on a second visit"
        step("second visit: no notice", again)
        def reset():
            pg.evaluate("localStorage.removeItem('mzlab.safari.seen')"); pg.reload(); ready(pg)
            s = shown(pg); assert s["seen"] == "1", s
            pg.click(f"{BOX} button.x"); wait_for_js(pg, f"!document.querySelector('{BOX}')", 5000)
        step("storage cleared: shown again; the X closes it", reset)
        c.close()

        c, pg = ctx(b)
        pg.add_init_script("(() => { const S = Storage.prototype, g = S.getItem, s = S.setItem; S.getItem = function (k) { if (k === 'mzlab.safari.seen') throw new Error('blocked'); return g.call(this, k); }; S.setItem = function (k, v) { if (k === 'mzlab.safari.seen') throw new Error('blocked'); return s.call(this, k, v); }; })()")      # the notice's own reads and writes refused (a blocked storage)
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        step("storage that throws: the notice is shown and the page works", lambda: shown(pg))
        c.close()

        c, pg = ctx(b, locale="en-US")
        pg.add_init_script("try{localStorage.setItem('mzlab.lang','en')}catch(_){}")      # the stored choice: the seeding of lib.py stays out once other keys are there
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        def english():
            s = shown(pg); assert "Chrome, Edge or Firefox" in s["text"] and "7 days" in s["text"] and "original files" in s["text"], s["text"]
        step("English page: the English text", english)
        c.close()

        c, pg = ctx(b, tour=True)
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        def with_tour():
            s = shown(pg)
            pg.evaluate("TOUR.mostraInvito()")          # the invitation comes once a file is open: called here, the notice still on screen
            wait_for_js(pg, "!!document.getElementById('tour-invito')", 10000)
            t = pg.evaluate("(() => { const a = document.getElementById('tour-invito').getBoundingClientRect(), n = document.querySelector('#safaribox').getBoundingClientRect(); return { overlap: !(a.right <= n.left || n.right <= a.left || a.bottom <= n.top || n.bottom <= a.top) }; })()")
            assert not t["overlap"], "the notice covers the tour invitation"
            pg.click("#tour-inv-no"); wait_for_js(pg, "!document.getElementById('tour-invito')", 5000)
            assert pg.evaluate(STATE)["on"], "closing the tour invitation closed the notice"
        step("the tour invitation beside the notice: no overlap, each closes on its own", with_tour)
        c.close()

        dev = {kk: v for kk, v in p.devices["iPad Pro 11 landscape"].items() if kk != "default_browser_type"}
        if BROWSER == "firefox": dev.pop("is_mobile", None)
        c, pg = ctx(b, **dev)
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        def ipad():
            s = shown(pg)
            assert s["kind"] == "ipad", s
            assert "schermata Home" in s["text"] and "7 giorni" in s["text"] and "file originali" in s["text"], s["text"]
            assert "Chrome" not in s["text"], "on an iPad every browser is WebKit: no other browser to recommend"
            pg.tap(f"{BOX} button.x"); wait_for_js(pg, f"!document.querySelector('{BOX}')", 5000)
        step("iPad: the Home Screen text, no browser recommended; a tap on the X closes it", ipad)
        c.close()

        if BROWSER != "webkit":
            c, pg = ctx(b, gesture=False)
            pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
            def other():
                hold(pg, 2500); assert not pg.evaluate(STATE)["on"], "the Safari notice in a browser that is not Safari"
            step(f"{BROWSER}: no notice", other)
            c.close()
        b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:5])
sys.exit(1 if bad or r.errs else 0)
