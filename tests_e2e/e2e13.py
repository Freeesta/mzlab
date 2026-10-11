"""Browser version (the site that GitHub Pages serves): Pyodide runs the program, no server involved.
Build first: python tools/build_site.py [--pyodide-dir DIR]"""
import subprocess, sys, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, mz, SH

import os
BASE = os.environ.get("QQQ_BASE", "")          # e.g. QqQ-lab: GitHub Pages serves a project site under /<repo>/
PORT = 8831
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300]))

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site") if not BASE else "/tmp/qq_pages"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1500, "height": 2200}); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        pg.goto(f"http://127.0.0.1:{PORT}/{BASE + '/' if BASE else ''}")
        def boot():
            pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("window.QQQ_BROWSER === true")
        step("engine loads and the start screen appears", boot)
        def load():
            pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_MS2-t15"), mz("B_MRM-t0")])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 5", timeout=120000); pg.wait_for_timeout(300)
            t = pg.inner_text("#flist"); assert "MRM" in t and "MS2" in t, t
        step("upload five mzML files and read the experiment type", load)
        def openit():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000); pg.wait_for_timeout(3000)
            pg.screenshot(path=SH + "130_browser.png")
            pg.wait_for_function("E.panels[0] && E.panels[0]._a && E.panels[0]._a.sr.length >= 1", timeout=60000)      # the traces arrive when the engine has computed them
            assert pg.evaluate("E.panels[0]._a && E.panels[0]._a.sr.length") == 1      # at the first load only the first file of each kind is open
        step("open: TIC drawn", openit)
        def xic():
            pg.click("#np-xic"); pg.fill("#xic-mz", "364"); pg.click("#xic-go"); pg.wait_for_timeout(3000)
            assert pg.evaluate("E.panels.some(p=>p.type==='xic' && p._a && p._a.sr.length>0)")
        step("XIC from the dialog", xic)
        def spec():
            pg.evaluate("E.panels[0].cur=null"); pg.dispatch_event(".pnl.chrom canvas", "dblclick")
        def formula():
            r = pg.evaluate("fetch('api/formula?f=C15H12N2O&adduct=%5BM%2BH%5D%2B').then(r=>r.json())"); assert abs(r["mz"] - 237.1022) < 0.001, r
        step("formula endpoint", formula)
        def reload():
            pg.reload(); pg.wait_for_selector(".pnl canvas", timeout=120000); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.files.length") == 5, pg.evaluate("E.files.length")
        step("reload resumes the session from the browser storage", reload)
        def tabs():
            pg.click("#nav button[data-v=draw]"); pg.wait_for_function("window.TPDraw !== undefined || document.querySelector('#kframe')", timeout=60000); pg.wait_for_timeout(1500)
            pg.click("#nav button[data-v=theory]"); pg.wait_for_timeout(1500)
            assert "static/teoria/index.html" in pg.evaluate("document.getElementById('tframe').src")
            pg.click("#nav button[data-v=data]")
        step("Disegno and Teoria tabs work from the static site", tabs)
        def mrm_spec():
            assert pg.evaluate("E.panels.some(p=>p.type==='spec' && p._a)"), "spectrum panel"
            pg.click("#dtabs [data-t=mrm]"); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.some(p=>p.type==='mrm' && p._a && p._a.sr.length>0)")
        step("spectrum and MRM panels", mrm_spec)
        def clear_all():
            pg.click("#newrun"); pg.click("#askyes") if pg.locator("#askyes").count() else None
            pg.wait_for_timeout(2000)
        step("new session", clear_all)
        def offline():
            c2 = b.new_context(); q = c2.new_page(); q.goto(f"http://127.0.0.1:{PORT}/{BASE + '/' if BASE else ''}"); q.wait_for_selector("#drop", timeout=120000)
            q.evaluate("navigator.serviceWorker.ready.then(()=>1)")
            q.reload(); q.wait_for_selector("#drop", timeout=120000); q.wait_for_timeout(2000)      # now served through the service worker: fills the cache
            n = q.evaluate("caches.keys().then(async ks=>{let n=0;for(const k of ks){n+=(await (await caches.open(k)).keys()).length}return n})"); assert n > 10, n
            c2.set_offline(True)
            q.reload(); q.wait_for_selector("#drop", timeout=120000)
            q.set_input_files("#pick", [mz("B_FullMass-t0")]); q.wait_for_function("document.querySelectorAll('#flist tr').length >= 2", timeout=60000)
            c2.close()
        step("works offline after the first visit (service worker)", offline)
        b.close()
finally:
    srv.terminate()
print("ERRORS", [e for e in errs if "favicon" not in e[1]][:8])
