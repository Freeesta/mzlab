"""Smartphone (telefono.js): in portrait and landscape the site shows only the header (logo, Teoria, info) and the card that sends to
the Teoria; nothing heavy is downloaded (no drawing module, OpenChemLib, Ketcher, example files); the Teoria fits the screen,
its header goes away scrolling down and comes back scrolling up; the games say to use a computer, the oral questions work.
On a computer nothing changes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:200]))
IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
r = Run(port=8903, wd="/tmp/wdtel")
try:
    with sync_playwright() as p:
        pg = r.page(p)                                       # a computer first: unchanged
        U = f"http://127.0.0.1:{r.port}/"
        def desktop():
            assert not pg.evaluate("document.documentElement.classList.contains('phone')")
            assert pg.locator("#nav button:visible").count() == 3 and pg.is_hidden("#phone")
        step("computer: three tabs, no phone card", desktop)
        for vw, vh, tag in [(390, 844, "verticale"), (844, 390, "orizzontale")]:
            ctx = r.b.new_context(viewport={"width": vw, "height": vh}, device_scale_factor=2, is_mobile=True, has_touch=True, user_agent=IPHONE)
            q = ctx.new_page(); reqs = []
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            q.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
            q.on("request", lambda x: reqs.append(x.url))
            def home():
                q.goto(U); q.wait_for_timeout(1500)
                assert q.evaluate("document.documentElement.classList.contains('phone')")
                vis = q.evaluate("[...document.querySelectorAll('header > *, #nav button')].filter(e=>e.offsetParent!==null).map(e=>e.id||e.dataset.v||e.className)")
                assert vis == ["logo", "nav", "theory", "phinfo"], vis
                assert q.is_visible("#phone") and q.is_hidden("#loading") and "schermo grande" in q.inner_text("#phone") and "MB" not in q.inner_text("#phone")
                heavy = [u for u in reqs if any(k in u for k in ("draw.js", "openchemlib", "ketcher", "esempi/", "pyodide", "browser-worker", ".wasm", "explore.js", "grafici.js", "hr.js", "banco.js"))]
                assert not heavy, heavy
                assert q.evaluate("document.documentElement.scrollWidth") <= vw
                q.click("#phinfo"); assert q.is_visible("#phmore")
            step(f"{tag}: header with logo, Teoria and info only, nothing heavy downloaded", home)
            q.screenshot(path=SH + f"tel_{tag}_home.png")
            def theory():
                q.click("#phone .phb"); q.wait_for_timeout(1200)
                assert q.url.endswith("static/teoria/index.html"), q.url
                for name in ("index.html", "09-quadrupolo.html", "15-ei-metodo.html", "19-hr-dda.html", "22-formulario.html"):
                    q.goto(U + "static/teoria/" + name); q.wait_for_timeout(700)
                    w = q.evaluate("document.documentElement.scrollWidth")
                    assert w <= vw + 1, f"{name}: page {w} px wide on a {vw} px screen"
                q.mouse.wheel(0, 1500); q.wait_for_timeout(500)
                assert q.evaluate("document.querySelector('#top').classList.contains('away')"), "header still there scrolling down"
                q.mouse.wheel(0, -300); q.wait_for_timeout(500)
                assert not q.evaluate("document.querySelector('#top').classList.contains('away')"), "header not back scrolling up"
                q.click("#info"); assert q.is_visible("#phnote")
                q.click("#menu"); assert q.evaluate("getComputedStyle(document.querySelector('#side')).position") == "fixed"
            step(f"{tag}: the Teoria fits the screen, the header hides on scroll, info and index work", theory)
            q.screenshot(path=SH + f"tel_{tag}_teoria.png")
            def exercises():
                q.goto(U + "static/teoria/pratica-ei.html"); q.wait_for_timeout(1200)
                assert q.is_visible(".phex") and q.is_hidden("#game")
                q.goto(U + "static/teoria/pratica-orale.html"); q.wait_for_timeout(800)
                assert q.is_visible("#exam")
            step(f"{tag}: games sent to a computer, oral questions available", exercises)
            ctx.close()

        def look(scheme):
            ctx = r.b.new_context(viewport={"width": 390, "height": 844}, device_scale_factor=2, is_mobile=True, has_touch=True, user_agent=IPHONE, color_scheme=scheme)
            q = ctx.new_page(); reqs = []; q.on("request", lambda x: reqs.append(x.url))
            q.goto(U); q.wait_for_selector("#phone:not([hidden])", timeout=5000)
            vals = q.evaluate("""() => { const lum = c => { const m = c.match(/[\\d.]+/g).map(Number); const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4 }; return .2126 * f(m[0]) + .7152 * f(m[1]) + .0722 * f(m[2]) };
                const bgOf = e => { for (; e; e = e.parentElement) { const c = getComputedStyle(e).backgroundColor; if (!/rgba\\(.*, 0\\)|transparent/.test(c)) return c } return "rgb(255,255,255)" };
                const ratio = e => { const a = lum(getComputedStyle(e).color), b = lum(bgOf(e)); return (Math.max(a, b) + .05) / (Math.min(a, b) + .05) };
                return [...document.querySelectorAll("#phone h1, #phone .phw, #phone .phb, #phone .pho")].map(e => { const b = e.getBoundingClientRect(); return [e.className || e.tagName, ratio(e), b.height] }) }""")
            for name, ratio, h in vals: assert ratio >= 4.5, (scheme, name, ratio)
            for name, ratio, h in vals[2:]: assert h >= 44, (scheme, name, h)
            assert q.evaluate("document.documentElement.scrollWidth") <= 390
            q.wait_for_timeout(800)
            q.screenshot(path=SH + f"tel_scheda_{scheme}.png")
            heavy = [u for u in reqs if any(k in u for k in ("pyodide", ".wasm", "browser-worker", "explore.js", "draw.js", "openchemlib"))]
            assert not heavy, heavy
            assert any("teoria/teoria.js" in u for u in reqs) and any("teoria/index.html" in u for u in reqs), "Teoria not warmed up"
            t0 = q.evaluate("performance.now()")
            q.click("#phone .phb"); q.wait_for_url("**/static/teoria/index.html"); q.wait_for_selector("h1")
            ctx.close()
        step("phone: card readable in light theme (contrast >= 4.5, targets >= 44 px), no engine, Teoria warmed up", lambda: look("light"))
        step("phone: card readable in dark theme (contrast >= 4.5, targets >= 44 px), no engine, Teoria warmed up", lambda: look("dark"))
        def open_anyway():
            ctx = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, user_agent=IPHONE)
            q = ctx.new_page(); q.goto(U); q.wait_for_selector("#phone:not([hidden])")
            q.click("#phopen"); q.wait_for_function("!document.documentElement.classList.contains('phone')", timeout=15000)
            q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.is_hidden("#phone")
            q.reload(); q.wait_for_function("window.qqReady===true", timeout=60000)      # the choice lasts for the tab
            ctx.close()
        step("phone: Apri comunque loads the full program and remembers the choice", open_anyway)
        def ipad():
            ctx = r.b.new_context(viewport={"width": 1024, "height": 1366}, is_mobile=True, has_touch=True, device_scale_factor=2,
                                  user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15")
            q = ctx.new_page(); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
            assert not q.evaluate("document.documentElement.classList.contains('phone')") and q.is_hidden("#phone")
            ctx.close()
        step("iPad is not a phone: full program, no card", ipad)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
