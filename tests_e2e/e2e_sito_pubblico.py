"""Public site (mzlab/web/sito.js): with the production values (Teoria on, desktop-app download off) the Teoria opens from the page and
#/theory reaches it, while the served page, the DOM and the help carry no link to releases or installers; ?sito=senza-teoria switches the Teoria off."""
import sys, os, urllib.request; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.environ["MZLAB_SITO_PRODUZIONE"] = "1"      # lib.py then leaves the values of sito.js alone (the other tests turn both on)
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
            for bad in ("releases", "installer"):
                assert bad not in html, bad
        step("the served page has no link to releases", served)
        def values():
            assert pg.evaluate("JSON.stringify(window.SITO_PUBBLICO)") == '{"scaricaApp":false,"teoria":true}'
        step("production values: Teoria on, app download off", values)
        def dom():
            ready(pg)
            assert pg.evaluate("""document.querySelectorAll('#nav [data-v="theory"], #tframe').length""") == 2
            assert pg.evaluate("""document.querySelectorAll('a[href*="releases"], a[href*="installer"]').length""") == 0
        step("the Teoria tab and frame are there, no download link", dom)
        def help_():
            pg.click("button.hq[data-help=header]"); pg.wait_for_timeout(400)
            html = pg.evaluate("document.body.innerHTML")
            assert "releases" not in html and "Scarica l'app desktop" not in html
        step("the help offers no download of the desktop app", help_)
        def hashes():
            for h in ("#/theory", "#/practice"):
                q = r.b.new_page(); q.goto(U + h); q.wait_for_url("**/static/teoria/**", timeout=60000)
                assert "static/teoria" in q.url, (h, q.url)
                q.close()
        step("#/theory and #/practice reach the Teoria", hashes)
        def off():
            q = r.b.new_page(); q.goto(U + "?sito=senza-teoria#/theory"); q.wait_for_function("typeof S !== 'undefined' && !!S.view", timeout=60000)
            assert q.evaluate("JSON.stringify(window.SITO_PUBBLICO)") == '{"scaricaApp":false,"teoria":false}'
            assert q.evaluate("S.view") == "data" and q.query_selector('#nav [data-v="theory"]') is None
            q.close()
        step("with ?sito=senza-teoria the entrances go away", off)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
