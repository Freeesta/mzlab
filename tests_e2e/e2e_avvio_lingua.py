"""First start of the site: the language and the title are decided before anything is drawn (browser it-IT + the old key qqq.lang='en' of the
QqQ-lab site -> Italian from the first instant), only the catalog of that language is loaded, and the first loading phrase stays on screen
for at least 4 s. Build first: python tools/build_site.py"""
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

# records every title the document ever has, from its creation (before the first script of the page)
WATCH = """(() => { window.__titles = []; const add = () => { const t = document.title; if (t && window.__titles[window.__titles.length - 1] !== t) window.__titles.push(t); };
  new MutationObserver(add).observe(document, { subtree: true, childList: true, characterData: true, attributes: true }); add(); })()"""
# which loading phrase (0-based) the status line shows now, -1 for anything else (a status text, empty)
WHICH = """() => { const el = document.getElementById('ldmsg'), L = document.getElementById('loading'); if (!el || !L || L.hidden) return -2;
  const t = el.textContent.replace(/\\.+$/, ''), d = document.createElement('div');
  for (let i = 1; i <= 17; i++) { d.innerHTML = I18N.t('load.phrase.' + i); if (d.textContent === t) return i - 1; } return -1; }"""

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def ctx(locale, old):
            c = b.new_context(locale=locale, viewport={"width": 1300, "height": 900})
            c.add_init_script(f"try {{ localStorage.setItem('qqq.lang', '{old}') }} catch (e) {{}}")
            c.add_init_script(WATCH); return c
        def italian_from_the_start():
            c = ctx("it-IT", "en"); pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
            reqs = []; pg.on("request", lambda r: reqs.append(r.url))
            pg.goto(URL, wait_until="commit"); seen = set(); t0 = time.time()
            while time.time() - t0 < 4.0:
                try: w = pg.evaluate(WHICH)
                except Exception: w = -2          # page not ready yet
                if w >= 0: seen.add(w)
                pg.wait_for_timeout(120)
            assert len(seen) <= 1, f"the phrase changed within 4 s: {seen}"
            pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("I18N.lang") == "it" and pg.evaluate("document.documentElement.lang") == "it"
            titles = pg.evaluate("window.__titles"); assert titles and all("Analisi MS" in t and "MS Analysis" not in t for t in titles), titles
            assert pg.evaluate("localStorage.getItem('mzlab.lang')") is None, "the old key must not become a choice"
            assert any(u.endswith("/lang/it.js") for u in reqs) and not any(u.endswith("/lang/en.js") for u in reqs), [u for u in reqs if "/lang/" in u]
            c.close()
        step("browser it-IT + old qqq.lang=en: Italian title and page from the first instant, one phrase for 4 s, only it.js", italian_from_the_start)
        def english_ignores_old_key():
            c = ctx("en-US", "it"); pg = c.new_page(); pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("I18N.lang") == "en"; titles = pg.evaluate("window.__titles")
            assert titles and all("MS Analysis" in t or "Analisi MS" in t for t in titles) and "MS Analysis" in titles[-1], titles
            c.close()
        step("browser en-US + old qqq.lang=it: English (the old key is ignored)", english_ignores_old_key)
        def gear_choice():
            c = ctx("en-US", "en"); pg = c.new_page(); pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            pg.evaluate("I18N.set('it')"); pg.wait_for_function("window.I18N && I18N.lang === 'it'", timeout=60000)
            assert pg.evaluate("localStorage.getItem('mzlab.lang')") == "it"; c.close()
        step("the choice is stored in mzlab.lang and wins over the browser", gear_choice)
        b.close()
finally:
    srv.terminate()
sys.exit(1 if errs else 0)
