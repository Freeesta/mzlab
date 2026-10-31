"""Separate addresses: #/data and #/draw open the right view from cold (title per view, Back/Forward walk the views); the retired Teoria addresses
(#/theory, #/practice, #/teoria) open the start view."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_indirizzi.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:200]))
r = Run(port=8931, wd="/tmp/wdind")
HEAVY = ("pyodide", ".wasm", "explore.js", "grafici.js", "draw.js", "openchemlib", "ketcher", "/api/", "hr.js", "banco.js", "pannelli.js")
try:
    with sync_playwright() as p:
        r.page(p)
        U = f"http://127.0.0.1:{r.port}/"
        def fresh():
            c = r.b.new_context(viewport={"width": 1280, "height": 800}); q = c.new_page(); q.reqs = []
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            q.on("request", lambda x: q.reqs.append(x.url)); q.bytes = [0]
            def add(x):
                try: q.bytes[0] += len(x.body())
                except Exception: pass                      # redirects and evicted bodies have none
            q.on("response", add); return c, q

        def shut(c, q):                      # let the requests in flight finish: an aborted one makes the dev server print a traceback
            try: q.wait_for_load_state("networkidle", timeout=5000)
            except Exception: pass
            c.close()

        def dati():
            c, q = fresh(); q.goto(U + "#/data"); q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.is_visible("#start") or q.is_visible("#v-data")
            assert q.locator("#nav button.on").get_attribute("data-v") == "data" and "Analisi" in q.title(), q.title()
            shut(c, q)
        step("cold #/data: data view, title", dati)

        def disegno():
            c, q = fresh(); q.goto(U + "#/draw"); q.wait_for_selector("#v-draw:not([hidden])", timeout=60000)
            assert q.locator("#nav button.on").get_attribute("data-v") == "draw"
            q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.title().startswith("Disegno") and q.is_hidden("#v-data"), q.title()
            shut(c, q)
        step("cold #/draw: drawing view at once, title", disegno)

        def history():
            c, q = fresh(); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
            q.click("#nav button[data-v=draw]"); assert q.url.endswith("#/draw") and q.title().startswith("Disegno")
            q.click("#nav button[data-v=data]"); assert q.url.endswith("#/data") and "Analisi" in q.title()
            q.go_back(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "draw" and q.is_visible("#v-draw")
            q.go_back(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "data" and "Analisi" in q.title()
            q.go_forward(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "draw"
            shut(c, q)
        step("Back/Forward walk through Dati and Disegno; title follows", history)

        def retired():
            for h in ("#/theory", "#/theory/5", "#/practice", "#/dati", "#/disegno", "#/teoria", "#/teoria/5", "#/pratica"):
                c, q = fresh(); q.goto(U + h); q.wait_for_function("window.qqReady===true", timeout=60000)
                assert "teoria" not in q.url.split("#")[0], (h, q.url)            # nothing forwards to a local Teoria
                assert q.locator("#nav button.on").get_attribute("data-v") == "data", h             # it is just the start view
                shut(c, q)
        step("retired Teoria and old Italian addresses just open the start view", retired)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
