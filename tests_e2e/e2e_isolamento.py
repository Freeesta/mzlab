"""Cross-origin isolation from the service worker (sw.js + browser.js isolateOnce): a server WITHOUT COOP/COEP, one reload on the first visit, then
crossOriginIsolated and SharedArrayBuffer; a file opens, Ketcher opens, no COEP error, MZLAB_LR_MISS empty; ?cap-off=isolamento never reloads. Build first: python tools/build_site.py"""
from pathlib import Path
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port, ready

PORT = free_port(); errs = []
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); msgs = []
        pg = b.new_page(); pg.add_init_script(SKIP_TOUR)
        pg.add_init_script("try{if(window!==top)throw 0;const k='iso.log';sessionStorage.setItem(k,(sessionStorage.getItem(k)||'')+(crossOriginIsolated?'I':'n'));}catch(_){}")   # one letter per page load
        pg.on("console", lambda m: msgs.append(m.text) if m.type in ("error", "warning") else None)
        navs = []; pg.on("framenavigated", lambda f: navs.append(1) if f == pg.main_frame else None)
        def first():
            t0 = time.time(); pg.goto(URL)
            pg.wait_for_function("crossOriginIsolated === true", timeout=30000)
            print("   reload to isolated ms:", round((time.time() - t0) * 1000))
            log = pg.evaluate("sessionStorage.getItem('iso.log')"); assert log == "nI", "loads (n = not isolated, I = isolated): " + log
            assert pg.evaluate("typeof SharedArrayBuffer === 'function' && new WebAssembly.Memory({initial:1,maximum:1,shared:true}).buffer instanceof SharedArrayBuffer")
        step("first visit: one reload, then isolated", first)
        def once():
            pg.wait_for_timeout(3000); n = len(navs); pg.wait_for_timeout(2000); assert len(navs) == n, "reloaded again"
            assert pg.evaluate("window.CAPACITA && CAPACITA.snapshot().isolamento === true")
        step("no second reload; capacita.isolamento", once)
        def dati():
            pg.evaluate("location.hash = '#/data'"); pg.wait_for_selector("#demobtn", timeout=30000)
            pg.click("#demobtn"); pg.click("#demochoice [data-demo=full]")
            pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=60000)
            pg.click("text=Carica dati"); ready(pg)
            assert pg.evaluate("tabFiles('full').length") == 5
            assert not pg.evaluate("(window.MZLAB_LR_MISS || []).length")
        step("engine idle, MZLAB_LR_MISS empty", dati)
        def ketcher():
            fr = pg.evaluate("""() => { const f = document.getElementById('kframe'); return !!f; }""")
            assert fr
            pg.evaluate("location.hash = '#/draw'"); pg.wait_for_timeout(500)
            pg.wait_for_function("document.getElementById('kframe').contentDocument && document.getElementById('kframe').contentDocument.body && document.getElementById('kframe').contentDocument.body.children.length > 0", timeout=60000)
        step("Ketcher opens", ketcher)
        step("no COEP error in console", lambda: (_ for _ in ()).throw(AssertionError(msgs[:3])) if any("COEP" in m or "Cross-Origin-Embedder" in m or "ERR_BLOCKED_BY_RESPONSE" in m for m in msgs) else None)
        def capoff():
            c = b.new_context(); q = c.new_page(); q.add_init_script(SKIP_TOUR); q.goto(URL + "?cap-off=isolamento"); q.wait_for_timeout(6000)
            assert not q.evaluate("crossOriginIsolated") and q.evaluate("!sessionStorage.getItem('mzlab.coiReload')"), "must not reload"
            c.close()
        step("?cap-off=isolamento: no reload", capoff)
        b.close()
finally:
    srv.terminate()
print("FAIL:", errs) if errs else print("OK e2e_isolamento")
sys.exit(1 if errs else 0)
