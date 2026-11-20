"""Drawing, failed editor load: when Ketcher's page or its scripts do not arrive, the editor area says so at once (not after the 60 s wait) with a «Try again» button, and that button loads the editor again.
The service worker is blocked so that the routes below decide what the network answers."""
# covers: mzlab/web/draw.js, mzlab/web/disegno-avvio.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8958, wd="/tmp/wd_davviofallito")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
STATE = """() => { const f = document.getElementById('kfail'), l = document.getElementById('kload'), w = document.getElementById('smi-warn'), fr = document.getElementById('kframe'), b = document.getElementById('kfail-retry');
  const vis = e => !!e && !e.closest('[hidden]') && e.getBoundingClientRect().width > 0, bb = b ? b.getBoundingClientRect() : null;
  return { fail: vis(f), text: f ? f.innerText.trim() : '', role: f ? f.getAttribute('role') : '', loading: vis(l), note: vis(w) ? w.innerText.trim() : '', frame: getComputedStyle(fr).visibility,
    button: bb ? [Math.round(bb.width), Math.round(bb.height)] : null, inView: bb ? bb.top >= 0 && bb.bottom <= innerHeight && bb.left >= 0 && bb.right <= innerWidth : false, ready: !!(window.TPDraw && TPDraw.ready()) }; }"""
FAILED = "(() => { const f = document.getElementById('kfail'); return !!f && !f.hidden; })()"
def failed_then_retry(pg, pattern):
    pg.route(pattern, lambda route: route.fulfill(status=404, body="not found"))
    pg.click("#nav [data-v=draw]")
    wait_for_js(pg, FAILED, timeout=10000)                 # far below the 60 s wait
    frames(pg, 3); s = pg.evaluate(STATE)
    assert s["fail"] and s["text"].startswith("Ketcher did not start") and s["role"] == "alert", s
    assert not s["loading"], ("the loading note stays beside the failure", s)
    assert s["frame"] == "hidden", ("the broken frame is shown", s)
    assert s["button"] and min(s["button"]) >= 24 and s["inView"], s
    pg.unroute(pattern)
    pg.click("#kfail-retry")
    wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready()) && !document.getElementById('kframe').classList.contains('kwait')", timeout=60000)
    frames(pg, 3); s = pg.evaluate(STATE)
    assert not s["fail"] and not s["loading"] and s["note"] == "" and s["frame"] == "visible", s
try:
    with sync_playwright() as p:
        pg = r.page(p, service_workers="block"); pg.set_viewport_size({"width": 1300, "height": 900})
        step("1 Ketcher's page missing: the failure shows at once with «Try again», which loads the editor", lambda: failed_then_retry(pg, "**/vendor/ketcher/index.html"))
        def hidden_panels():
            pg.click("#side-toggle"); frames(pg, 3)
            pg.route("**/vendor/ketcher/index.html", lambda route: route.fulfill(status=404, body="not found"))
            pg.reload(); wait_for_js(pg, FAILED, timeout=10000)        # the address is #/draw: the early start of first-paint.js meets the same failure
            frames(pg, 3); s = pg.evaluate(STATE); assert s["fail"] and s["inView"] and not s["loading"], s
            pg.unroute("**/vendor/ketcher/index.html"); pg.click("#kfail-retry")
            wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.click("#side-toggle"); frames(pg, 3)
        step("2 panels hidden and a direct #/draw address: the failure is still on screen", hidden_panels)
        def twice():
            hits = [0]
            def gone(route): hits[0] += 1; route.fulfill(status=404, body="not found")
            pg.route("**/vendor/ketcher/index.html", gone)
            pg.reload(); wait_for_js(pg, FAILED, timeout=10000)
            n = hits[0]
            with pg.expect_request("**/vendor/ketcher/index.html", timeout=10000): pg.click("#kfail-retry")      # the click hides the notice at once and asks for the frame again
            assert hits[0] > n, "«Try again» did not ask for Ketcher's page again"
            wait_for_js(pg, FAILED, timeout=10000)             # ... and that attempt fails too: the notice is back, not an endless «Loading the editor»
            frames(pg, 3); s = pg.evaluate(STATE); assert s["fail"] and s["inView"] and not s["loading"] and not s["ready"], s
            assert pg.evaluate("document.activeElement && document.activeElement.id") == "kfail-retry", "the focus is not on the new «Try again»"
            pg.unroute("**/vendor/ketcher/index.html"); pg.click("#kfail-retry")
            wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready()) && !document.getElementById('kframe').classList.contains('kwait')", timeout=60000)
            frames(pg, 3); s = pg.evaluate(STATE); assert not s["fail"] and not s["loading"] and s["frame"] == "visible", s
        step("3 a direct #/draw address and a retry that fails too: the notice and the button come back, the third try loads the editor", twice)
        r.b.close()
        pg = r.page(p, service_workers="block"); pg.set_viewport_size({"width": 1300, "height": 900}); home = pg.url
        def after_ready():
            pg.evaluate("() => { const real = window.nbEnsure; window.nbEnsure = () => { window.nbEnsure = real; return Promise.reject(new Error('test: the notebook did not open')); }; }")
            pg.click("#nav [data-v=draw]"); wait_for_js(pg, FAILED, timeout=60000)
            frames(pg, 3); s = pg.evaluate(STATE); assert s["fail"] and not s["loading"] and s["frame"] == "hidden", s
            pg.click("#kfail-retry")
            wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready()) && !document.getElementById('kframe').classList.contains('kwait')", timeout=60000)
            frames(pg, 3); s = pg.evaluate(STATE); assert not s["fail"] and not s["loading"] and s["frame"] == "visible", s
        step("4 a failure after Ketcher is ready: «Try again» is not a dead button", after_ready)
        def keeps_drawing():
            assert pg.evaluate("TPDraw.load('CCO')"); wait_for_js(pg, "typeof NB !== 'undefined' && !!NB.ket && NB.ket.includes('\"O\"')", timeout=10000)
            pg.evaluate("async () => { await nbSave(true); }")
            pg.goto(home); pg.reload(); pg.wait_for_selector("#nav [data-v=draw]")
            # the saved drawing is put back, then the next step of the start fails once
            pg.evaluate("""() => { const real = window.nbEnsure; window.nbEnsure = async () => { window.nbEnsure = real; await real();
              const ed = document.getElementById('kframe').contentWindow.ketcher.editor, sub = ed.subscribe; ed.subscribe = () => { ed.subscribe = sub; throw new Error('test: failure after the drawing was put back'); }; }; }""")
            pg.click("#nav [data-v=draw]"); wait_for_js(pg, FAILED, timeout=60000)
            assert not pg.evaluate("TPDraw.ready()"), "the editor counts as ready under the failure notice"
            pg.click("#kfail-retry")
            wait_for_js(pg, "!!(window.TPDraw && TPDraw.ready()) && !document.getElementById('kframe').classList.contains('kwait')", timeout=60000)
            frames(pg, 3); smi = pg.evaluate("TPDraw.smiles()"); assert smi in ("CCO", "OCC", "C(C)O"), ("the saved drawing is not back after the retry", smi)
            assert pg.evaluate("NB.ket.includes('\"O\"')"), "the retry overwrote the saved drawing with a blank one"
        step("5 a failure after the saved drawing was put back: the retry shows it again and does not overwrite it", keeps_drawing)
        r.b.close()
        pg = r.page(p, service_workers="block"); pg.set_viewport_size({"width": 1300, "height": 900})
        step("6 Ketcher's scripts missing: the same failure and the same way back", lambda: failed_then_retry(pg, "**/vendor/ketcher/**/*.js"))
finally:
    r.close()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
