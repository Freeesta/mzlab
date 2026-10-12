"""Header of the standalone Teoria and Pratica: the tabs Dati · Disegno · Teoria (Teoria current), logo to the program, always visible at 360/768/1280 px,
dark theme, keyboard, one click from any page of the Teoria and of the Pratica to Dati and to Disegno, the session of the program is found again; phone: only logo and Teoria."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_nav.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
PAGES = ["index.html", "00-uso.html", "06-esi.html", "14-frammentazione-esi.html", "22-formulario.html", "pratica.html", "pratica-ei.html", "pratica-orale.html", "pratica-formula.html"]
r = Run(port=8972, wd="/tmp/wd_tnav", motore="python")
try:
    with sync_playwright() as p:
        r.page(p)
        U = f"http://127.0.0.1:{r.port}/"; T = U + "static/teoria/"
        def ctx(w, h=800, **k):
            c = r.b.new_context(viewport={"width": w, "height": h}, **k); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e)))); return c, q
        def idle(q):                                   # let the requests in flight finish: an aborted one makes the dev server print a traceback
            try: q.wait_for_load_state("networkidle", timeout=8000)
            except Exception: pass
        def go(q, url):
            idle(q); q.goto(url)
        def tabs_ok(q, w):
            idle(q)
            q.wait_for_selector("#tabs")
            names = q.eval_on_selector_all("#tabs a", "e=>e.map(x=>x.textContent)"); assert names == ["Dati", "Disegno", "Teoria"], names
            cur = q.eval_on_selector_all("#tabs a[aria-current=page]", "e=>e.map(x=>x.textContent)"); assert cur == ["Teoria"], cur
            for a in q.locator("#tabs a").all():
                b = a.bounding_box(); assert b and b["height"] >= 44 and b["width"] >= 44 and b["x"] >= 0 and b["x"] + b["width"] <= w, (a.inner_text(), b)
            lb = q.locator("#logo").bounding_box(); assert lb["height"] >= 44 and lb["width"] >= 28, lb
            assert q.evaluate("document.documentElement.scrollWidth") <= w, "horizontal scroll"
            assert q.evaluate("document.querySelector('#top').getBoundingClientRect().height") < 80
            assert q.get_attribute("#tab-dati", "href") == "../../#/dati" and q.get_attribute("#tab-disegno", "href") == "../../#/disegno" and q.get_attribute("#logo", "href") == "../../#/dati"
        def widths():
            for w in (360, 768, 1280):
                c, q = ctx(w); go(q, T + "06-esi.html"); tabs_ok(q, w)
                if w <= 640: assert q.is_hidden("#top .t"), "the title gives way first"
                else: assert q.is_visible("#top .t")
                go(q, T + "pratica-ei.html"); tabs_ok(q, w); idle(q); c.close()
        step("tabs Dati · Disegno · Teoria visible and ≥ 44 px at 360, 768, 1280 px (Teoria and Pratica), title shortened first", widths)
        def dark():
            c, q = ctx(360, color_scheme="dark"); go(q, T + "12-dati.html"); tabs_ok(q, 360)
            L = "(c=>{const m=c.match(/[0-9.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])})"
            k = q.evaluate("(()=>{const L=" + L + ";const bg=getComputedStyle(document.querySelector('#top')).backgroundColor;return ['#tab-dati','#tab-teoria'].map(s=>{const a=L(getComputedStyle(document.querySelector(s)).color),b=L(bg);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)})})()")
            assert min(k) >= 4.5, k; idle(q); c.close()
        step("dark theme: tab text readable (≥ 4.5)", dark)
        def keyboard():
            c, q = ctx(1280); go(q, T + "13-formula.html"); q.wait_for_selector("#tabs")
            q.evaluate("document.querySelector('#logo').focus()"); q.keyboard.press("Tab")
            ids = []
            for _ in range(3):
                ids.append(q.evaluate("document.activeElement.id")); q.keyboard.press("Tab")
            assert "tab-dati" in ids and "tab-disegno" in ids and "tab-teoria" in ids, ids
            q.evaluate("document.querySelector('#tab-disegno').focus()"); idle(q); q.keyboard.press("Enter")
            q.wait_for_url("**/#/disegno"); idle(q); c.close()
        step("keyboard: Tab reaches the three tabs, Enter on Disegno opens it", keyboard)
        def everywhere():
            c, q = ctx(1280)
            for pgname in PAGES:
                go(q, T + pgname); q.wait_for_selector("#tabs")
                assert q.get_attribute("#tab-dati", "href") == "../../#/dati" and q.get_attribute("#tab-disegno", "href") == "../../#/disegno", pgname
                assert q.is_visible("#tab-dati") and q.is_visible("#tab-disegno"), pgname
            idle(q); c.close()
        step("every page of the Teoria and of the Pratica has the tabs", everywhere)
        def reach():
            for tab, h, ready_sel in (("tab-dati", "data", "#nav button.on"), ("tab-disegno", "draw", "#nav button.on")):
                c, q = ctx(1280); go(q, T + "pratica-formula.html"); q.wait_for_selector("#" + tab)
                idle(q); q.click("#" + tab); q.wait_for_function("window.qqReady===true", timeout=60000)
                assert q.locator(ready_sel).get_attribute("data-v") == h, (tab, q.url)
                go(q, T + "08-vuoto.html"); idle(q); q.click("#logo"); q.wait_for_url("**/#/dati"); q.wait_for_function("window.qqReady===true", timeout=60000); idle(q); c.close()
        step("one click from a page to Dati and to Disegno; the logo leads to Dati", reach)
        def session():
            c, q = ctx(1500, 1000); go(q, U + "#/dati"); q.wait_for_function("window.qqReady===true", timeout=60000)
            stage(q, [mz(f) for f in FILES[:1]]); q.click("text=Carica dati"); ready(q)
            n = q.evaluate("E.files.length"); assert n >= 1, n
            q.click("#nav button[data-v=theory]"); q.wait_for_selector("#tframe"); q.wait_for_timeout(800)
            go(q, T + "04-lc.html"); q.wait_for_selector("#tab-dati"); idle(q); q.click("#tab-dati")
            q.wait_for_function("window.qqReady===true", timeout=60000); ready(q)
            assert q.evaluate("E.files.length") == n, "the session was lost"
            idle(q); c.close()
        step("the session of the program (a loaded file) is found again after the Teoria", session)
        def phone():
            c = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, user_agent=IPHONE); q = c.new_page()
            go(q, T + "04-lc.html"); q.wait_for_selector("#tabs")
            names = q.eval_on_selector_all("#tabs a", "e=>e.map(x=>x.textContent)"); assert names == ["Teoria"], names
            assert q.get_attribute("#logo", "href") == "../../"
            assert q.locator("#logo").bounding_box()["height"] >= 44 and q.locator("#tab-teoria").bounding_box()["height"] >= 44
            assert q.evaluate("document.documentElement.scrollWidth") <= 390
            idle(q); c.close()
        step("phone: only the logo (to the program's notice) and Teoria", phone)
        def filep():
            import pathlib
            c, q = ctx(1280); go(q, (pathlib.Path(__file__).resolve().parent.parent / "mzlab/web/teoria/04-lc.html").as_uri()); q.wait_for_selector("#top")
            assert q.locator("#tabs").count() == 0 and q.get_attribute("#logo", "href") == "index.html"; idle(q); c.close()
        step("from file:// no link to the program (as before)", filep)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
