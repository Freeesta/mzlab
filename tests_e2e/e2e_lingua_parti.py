"""The interface catalog in two parts (tools/build_site.py bundle_lang, i18n.js awaitRest/done): in Italian and English, at desktop, iPad and
phone sizes, nothing asks for a key before the rest of the catalog has run (I18N.late empty, no console error), every static text ends in the
language of the page, no bare key is shown, every tab of the header draws with the whole catalog, and the first part stays small.
Build first: python tools/build_site.py"""
import os, re, socket, subprocess, sys
from pathlib import Path
from playwright.sync_api import sync_playwright
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import ROOT, free_port, wait_for_js, frames  # noqa: E402  (lib also patches wait_for_function for the CSP)
sys.path.insert(0, str(ROOT / "tools")); import lang_catalogo  # noqa: E402

PORT = free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
while True:      # until the server listens (refused connections return at once)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
URL = f"http://127.0.0.1:{PORT}/"
SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}"
# every static element shows the catalog text of its key (none left waiting with the Italian of the page)
STATIC = """() => { const P = { app: window.APP_NAME || '' }, bad = [];
  for (const el of document.querySelectorAll('[data-i18n]')) { const k = el.getAttribute('data-i18n'); if (el.textContent !== I18N.t(k, P)) bad.push(k); }
  for (const [a, t] of [['data-i18n-title', 'title'], ['data-i18n-placeholder', 'placeholder'], ['data-i18n-aria-label', 'aria-label']])
    for (const el of document.querySelectorAll('[' + a + ']')) { const k = el.getAttribute(a); if (el.getAttribute(t) !== I18N.t(k, P)) bad.push(k); }
  return bad.slice(0, 10); }"""
SIZES = {"desktop": dict(viewport={"width": 1440, "height": 900}), "ipad": dict(viewport={"width": 820, "height": 1180}, has_touch=True, is_mobile=True),
         "phone": dict(viewport={"width": 390, "height": 844}, has_touch=True, is_mobile=True)}
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        for lang in ("it", "en"):
            keys = [k for k in lang_catalogo.catalog(lang) if re.fullmatch(r"[a-z][\w-]*(\.[\w-]+){2,}", k)]      # dotted keys long enough to never be prose
            for size, kw in SIZES.items():
                def run(lang=lang, size=size, kw=kw, keys=keys):
                    ctx = b.new_context(locale="it-IT", service_workers="block", **kw); pg = ctx.new_page(); pg.add_init_script(SKIP)
                    errs, sizes = [], {}
                    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "i18n" in m.text else None)
                    pg.on("response", lambda r: sizes.__setitem__(r.url.rsplit("/", 1)[-1], len(r.body())) if "/static/lang/" in r.url and r.url.endswith(".js") else None)
                    pg.goto(URL + "?lang=" + lang)
                    wait_for_js(pg, "window.I18N && !I18N.rest && (!!document.querySelector('#drop') || !!window.MZLAB_PHONE)", 120000)
                    wait_for_js(pg, "document.readyState === 'complete'", 60000); frames(pg, 3)
                    assert pg.evaluate("I18N.lang") == lang
                    assert sizes.get(f"{lang}.js", 0) < 64000 and sizes.get(f"{lang}-resto.js", 0) > 100000, sizes      # first part small, the rest fetched
                    views = pg.evaluate("[...document.querySelectorAll('#nav button')].filter(b => b.offsetParent).map(b => b.dataset.v)")
                    for v in views:
                        pg.click(f'#nav button[data-v="{v}"]'); frames(pg, 3)
                    late, bad = pg.evaluate("[...I18N.late]"), pg.evaluate(STATIC)
                    text = pg.evaluate("document.body.innerText")
                    raw = [k for k in keys if k in text][:10]
                    ctx.close()
                    assert not late and not errs, (late, errs)
                    assert not bad, ("static texts not in the language of the page", bad)
                    assert not raw, ("bare keys on screen", raw)
                step(f"{lang}, {size}: no key asked before the rest, static texts in {lang}, no bare key, every tab drawn", run)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
