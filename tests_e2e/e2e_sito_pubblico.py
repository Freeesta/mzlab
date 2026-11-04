"""Public site (mzlab/web/sito.js): with the production value (desktop-app download off) the served page, the DOM and the
help carry no link to releases or installers, no Teoria, and #/theory, #/practice stay on the main page; ?sito=tutto brings the download back."""
import sys, os, urllib.request; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.environ["MZLAB_SITO_PRODUZIONE"] = "1"      # lib.py then leaves the values of sito.js alone (the other tests turn it on)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_sito_pubblico.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8931, wd="/tmp/wd_sito")
U = f"http://127.0.0.1:{r.port}/"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def served():
            html = urllib.request.urlopen(U).read().decode("utf-8")
            for bad in ("releases", 'data-v="theory"', 'id="tframe"', "static/teoria"):
                assert bad not in html, bad
            import re
            assert not re.search(r'<a [^>]*href="[^"]*(teoria|BasePeak)', html), "link to the Teoria"
        step("the served page has no link to releases or the Teoria", served)
        def values():
            assert pg.evaluate("JSON.stringify(window.SITO_PUBBLICO)") == '{"scaricaApp":false}'
        step("production values are off", values)
        def dom():
            ready(pg)
            assert pg.evaluate("""document.querySelectorAll('[data-v="theory"], #tframe, a[href*="releases"], a[href*="static/teoria"], a[href*="installer"]').length""") == 0
            assert pg.evaluate("[...document.querySelectorAll('#nav button')].every(b => !/teoria|theory/i.test(b.dataset.v + b.textContent))")
        step("no button, link or frame leads to a Teoria in the DOM", dom)
        def help_():
            pg.click("button.hq[data-help=header]"); pg.wait_for_timeout(400)
            html = pg.evaluate("document.body.innerHTML")
            assert "releases" not in html and "Scarica l'app desktop" not in html
        step("the help offers no download of the desktop app", help_)
        def hashes():
            for h in ("#/theory", "#/theory/5", "#/practice"):
                q = r.b.new_page(); q.goto(U + h); q.wait_for_function("typeof S !== 'undefined' && !!S.view", timeout=60000)
                assert "static/teoria" not in q.url and q.evaluate("S.view") == "data", (h, q.url)
                assert q.evaluate("location.hash") == "", q.url
                q.close()
        step("#/theory and #/practice stay on the main page", hashes)
        def on():
            q = r.b.new_page(); q.goto(U + "?sito=tutto"); q.wait_for_function("window.SITO_PUBBLICO && window.SITO_PUBBLICO.scaricaApp === true", timeout=60000)
            assert q.evaluate("JSON.stringify(window.SITO_PUBBLICO)") == '{"scaricaApp":true}'
            assert q.query_selector("#tframe") is None
            q.close()
        step("with ?sito=tutto the download comes back", on)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
