"""First start of the site: the title and <html lang> are English before anything is drawn whatever the browser language and a language stored
by an older version (qqq.lang / mzlab.lang = 'it'), which is ignored; the first loading phrase stays on screen for at least 4 s; the header
icons are drawn at the first paint. Build first: python tools/build_site.py"""
# covers: mzlab/web/browser
from pathlib import Path
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

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def ctx(locale, old):
            c = b.new_context(locale=locale, viewport={"width": 1300, "height": 900})
            c.add_init_script(f"try {{ localStorage.setItem('qqq.lang', '{old}'); localStorage.setItem('mzlab.lang', '{old}') }} catch (e) {{}}")
            c.add_init_script(WATCH); return c
        def english_from_the_start():
            c = ctx("it-IT", "it"); pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
            reqs = []; pg.on("request", lambda r: reqs.append(r.url))
            pg.goto(URL, wait_until="commit"); seen = set(); t0 = time.time()
            while time.time() - t0 < 4.0:
                try: w = pg.evaluate(WHICH)
                except Exception: w = -2          # page not ready yet
                if w >= 0: seen.add(w)
                pg.wait_for_timeout(120)
            assert len(seen) <= 1, f"the phrase changed within 4 s: {seen}"
            pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("I18N.lang") == "en" and pg.evaluate("document.documentElement.lang") == "en"
            titles = pg.evaluate("window.__titles"); assert titles and titles[-1] == "MS Analysis · mzLab" and len(titles) <= 2, titles
            assert any(u.endswith("/lang/en.js") for u in reqs) and not any(u.endswith("/lang/it.js") for u in reqs), [u for u in reqs if "/lang/" in u]
            c.close()
        step("browser it-IT + stored it: English title and page from the first instant, one phrase for 4 s, only en.js", english_from_the_start)
        def stored_language_is_ignored():
            c = ctx("en-US", "it"); pg = c.new_page(); pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("I18N.lang") == "en"; titles = pg.evaluate("window.__titles")
            assert titles and titles[-1] == "MS Analysis · mzLab", titles      # the stored language is ignored: English
            c.close()
        step("a stored language (qqq.lang / mzlab.lang = it) is ignored without an error: English", stored_language_is_ignored)
        def header_first_paint():
            # while the texts wait (class i18n-wait) the buttons without text are already visible, in their place; the title never changes
            c = ctx("en-US", "en"); pg = c.new_page(); pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("window.__titles") == ["MS Analysis \u00b7 mzLab"], pg.evaluate("window.__titles")
            pg.evaluate("document.documentElement.classList.add('i18n-wait')")
            vis = pg.evaluate("['.logo','#np-set','#fs-btn','.hq'].map(s => getComputedStyle(document.querySelector('header ' + s)).visibility)")
            assert vis == ["visible"] * 4, vis
            assert pg.evaluate("getComputedStyle(document.querySelector('header #nav')).visibility") == "hidden"
            c.close()
        step("the header icons are drawn at the first paint and do not move; the title is already right", header_first_paint)
        b.close()
finally:
    srv.terminate()
sys.exit(1 if errs else 0)
