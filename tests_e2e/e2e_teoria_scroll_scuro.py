"""Teoria in the dark theme: scrolling down and up never shows a light band at the top. The canvas (html/body) is dark, html has no elastic overscroll
(overscroll-behavior-y none: macOS rubber-band cannot expose anything), and after wheel scrolls no full-width element at the top of the viewport has a light
background. The iframe of the program (#tframe) takes its background from the theme, not a fixed white."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_scroll_scuro.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8979, wd="/tmp/wd_teoria_scroll_scuro")
LIGHT = """() => { const lum = c => { const m = c.match(/[\\d.]+/g); return m && (c.indexOf('rgba') < 0 || +m[3] > 0) ? (+m[0] + +m[1] + +m[2]) / 3 : null; };
  const out = [];
  for (const y of [2, 20, 40, 60, 100]) for (const e of document.elementsFromPoint(innerWidth / 2, y)) {
    const l = lum(getComputedStyle(e).backgroundColor), w = e.getBoundingClientRect().width;
    if (l !== null && l > 170 && w >= innerWidth * 0.5) out.push([y, e.tagName + '#' + e.id, l, Math.round(w)]);
  }
  return out; }"""
try:
    with sync_playwright() as p:
        r.page(p)
        B = f"http://127.0.0.1:{r.port}/static/"
        def doc():
            c = r.b.new_context(viewport={"width": 1280, "height": 800}, color_scheme="dark"); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            q.goto(B + "teoria/03-cromatografia.html"); q.wait_for_function("!!window.TEORIA && !!document.querySelector('#tbar')", timeout=10000)
            v = q.evaluate("""() => { const f = e => { const m = getComputedStyle(e).backgroundColor.match(/\\d+/g); return (+m[0] + +m[1] + +m[2]) / 3; };
              return { html: f(document.documentElement), body: f(document.body), ob: getComputedStyle(document.documentElement).overscrollBehaviorY, cs: getComputedStyle(document.documentElement).colorScheme }; }""")
            assert v["html"] < 100 and v["body"] < 100 and v["ob"] == "none" and "dark" in v["cs"], v
            q.mouse.move(640, 400)
            for dy in (1500, 1500, 1500, -1200, -1200, -3000, 800, -800):
                before = q.evaluate("scrollY"); q.mouse.wheel(0, dy)
                q.wait_for_function("(b) => scrollY !== b || (b === 0 && %s) || document.documentElement.scrollHeight - innerHeight <= scrollY" % ("true" if dy < 0 else "false"), arg=before, timeout=5000)
                assert q.evaluate(LIGHT) == [], (dy, q.evaluate(LIGHT))
            c.close()
        step("dark: dark canvas, no elastic overscroll, no light full-width element at the top after scrolling down/up", doc)
        def frame():
            c = r.b.new_context(viewport={"width": 1280, "height": 800}, color_scheme="dark"); q = c.new_page()
            q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#tframe", state="attached", timeout=15000)
            l = q.evaluate("(() => { const m = getComputedStyle(document.querySelector('#tframe')).backgroundColor.match(/\\d+/g); return (+m[0] + +m[1] + +m[2]) / 3; })()")
            assert l < 100, l; c.close()
        step("program dark: #tframe background follows the theme", frame)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
