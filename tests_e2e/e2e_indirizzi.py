"""Separate addresses: #/dati and #/disegno open the right view from cold (title per view, Back/Forward walk the views); #/teoria, #/teoria/5 and
#/pratica leave for the standalone Teoria without fetching the engine or the Dati/Disegno scripts; teoria/#5, #cap-12, #A and #09-quadrupolo
forward to the chapter; the Dati tab returns to the program; every internal link of the Teoria answers (small crawler)."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_indirizzi.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:200]))
r = Run(port=8931, wd="/tmp/wdind", motore="python")
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
            c, q = fresh(); q.goto(U + "#/dati"); q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.is_visible("#start") or q.is_visible("#v-data")
            assert q.locator("#nav button.on").get_attribute("data-v") == "data" and "Analisi" in q.title(), q.title()
            shut(c, q)
        step("cold #/dati: data view, title", dati)

        def disegno():
            c, q = fresh(); q.goto(U + "#/disegno"); q.wait_for_selector("#v-draw:not([hidden])", timeout=60000)
            assert q.locator("#nav button.on").get_attribute("data-v") == "draw"
            q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.title().startswith("Disegno") and q.is_hidden("#v-data"), q.title()
            shut(c, q)
        step("cold #/disegno: drawing view at once, title", disegno)

        def history():
            c, q = fresh(); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
            q.click("#nav button[data-v=draw]"); assert q.url.endswith("#/disegno") and q.title().startswith("Disegno")
            q.click("#nav button[data-v=theory]"); assert q.url.endswith("#/teoria") and q.title().startswith("Teoria") and q.is_visible("#tframe")
            q.go_back(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "draw" and q.is_visible("#v-draw")
            q.go_back(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "data" and "Analisi" in q.title()
            q.go_forward(); q.wait_for_timeout(300); assert q.locator("#nav button.on").get_attribute("data-v") == "draw"
            shut(c, q)
        step("Back/Forward walk through Dati, Disegno and Teoria; title follows", history)

        def cold_teoria():
            for h, end in (("#/teoria", "static/teoria/index.html"), ("#/pratica", "static/teoria/pratica.html"), ("#/teoria/5", "static/teoria/05-gc.html")):
                c, q = fresh(); q.goto(U + h); q.wait_for_url("**/" + end.split("/")[-1], timeout=20000); q.wait_for_selector("h1"); q.wait_for_timeout(500)
                assert "/" + end in q.url, q.url
                heavy = [u for u in q.reqs if any(k in u for k in HEAVY)]
                assert not heavy, (h, heavy)
                print(f"MEASURE cold {h}: {q.bytes[0] / 1024:.0f} kB in {len(q.reqs)} requests")
                shut(c, q)
        step("#/teoria, #/teoria/5 and #/pratica open the standalone Teoria; no engine, no Dati/Disegno scripts fetched", cold_teoria)

        def alias():
            T = U + "static/teoria/index.html"
            for h, end in (("#5", "05-gc.html"), ("#cap-12", "12-dati.html"), ("#A", "00-uso.html"), ("#09-quadrupolo", "09-quadrupolo.html"), ("#p2", "pratica-orale.html")):
                c, q = fresh(); q.goto(T + h); q.wait_for_url("**/" + end, timeout=10000); shut(c, q)
            c, q = fresh(); q.goto(T + "#i-capitoli"); q.wait_for_timeout(500); assert q.url.endswith("index.html#i-capitoli"); shut(c, q)
        step("teoria/#5, #cap-12, #A, #09-quadrupolo, #p2 forward to the chapter; section anchors stay", alias)

        def toapp():
            c, q = fresh(); q.goto(U + "static/teoria/12-dati.html"); q.wait_for_selector("#tab-dati")
            q.click("#tab-dati"); q.wait_for_url("**/#/dati"); q.wait_for_function("window.qqReady===true", timeout=60000)
            assert q.locator("#nav button.on").get_attribute("data-v") == "data"
            q.click("#nav button[data-v=theory]"); q.wait_for_selector("#tframe"); q.wait_for_timeout(800)
            fr = q.frame_locator("#tframe"); assert fr.locator("#tabs").count() == 0, "no second header inside the program"
            shut(c, q)
        step("the «Dati» tab leads to #/dati; inside the program the Teoria has no second header", toapp)

        def crawl():
            c, q = fresh(); q.goto(U + "static/teoria/index.html")
            bad = q.evaluate("""async () => {
              const base = location.href, seen = new Set(), todo = [base], bad = [];
              while (todo.length) {
                const u = todo.pop(); if (seen.has(u)) continue; seen.add(u);
                const res = await fetch(u); if (!res.ok) { bad.push(u + " " + res.status); continue }
                const doc = new DOMParser().parseFromString(await res.text(), "text/html");
                for (const a of doc.querySelectorAll("a[href]")) {
                  const h = a.getAttribute("href"); if (/^(mailto:|javascript:|https?:)/.test(h) || h.startsWith("#")) continue;
                  const t = new URL(h, u); t.hash = ""; if (t.origin !== location.origin) continue;
                  if (t.pathname.includes("/teoria/") && /\\.html$/.test(t.pathname)) todo.push(t.href); else { const x = await fetch(t.href, { method: "HEAD" }); if (!x.ok && !seen.has(t.href)) bad.push(u + " -> " + h + " " + x.status); seen.add(t.href) }
                }
              }
              return { bad, n: seen.size }
            }""")
            assert not bad["bad"] and bad["n"] > 30, bad
            shut(c, q)
        step("crawler: every internal link of the Teoria answers", crawl)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
