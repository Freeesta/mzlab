"""Start-up of the site: static title, ONE status line with a progress bar, and a saved-files database that is blocked by another tab
or never answers (message, «start from scratch» button = same clean slate as «New session»). Build first: python tools/build_site.py"""
import re, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

# a v1 database (as an older version of the program left it) kept open by a page that never closes it: the upgrade to v2 is blocked
HOLD = """() => new Promise((res, rej) => {
  const r = indexedDB.open("qqq_lab", 1);
  r.onupgradeneeded = () => { const db = r.result; db.createObjectStore("files"); db.createObjectStore("kv"); };
  r.onsuccess = () => { window.__held = r.result; res(true); }; r.onerror = () => rej(String(r.error));
})"""
GONE = "() => new Promise(res => { if (window.__held) { window.__held.close(); window.__held = null; } const r = indexedDB.deleteDatabase('qqq_lab'); r.onsuccess = r.onerror = r.onblocked = () => res(true); })"

def title_static():
    html = (ROOT / "mzlab/web/index.html").read_text(encoding="utf-8")
    assert "<title>{APP} · Analisi MS</title>" in html
    built = (ROOT / "site/index.html").read_text(encoding="utf-8")
    assert re.search(r"<title>[^<]*· Analisi MS</title>", built) and 'og:title" content="' in built and "Analisi MS" in re.search(r'og:title" content="([^"]*)"', built).group(1)

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    step("static title and Open Graph", title_static)
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(locale="it-IT", viewport={"width": 1300, "height": 900})
        holder = ctx.new_page(); holder.goto(URL + "static/teoria/00-uso.html")
        holder.evaluate(GONE)
        def blocked():
            holder.evaluate(HOLD)
            pg = ctx.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.goto(URL + "?ripresa=4000&motore=python")
            pg.wait_for_function("document.getElementById('ldmsg') && /altra scheda/.test(document.getElementById('ldmsg').textContent)", timeout=90000)
            assert pg.locator("#ldbar").count() == 0, "bar on the loading screen"
            assert pg.locator("#ldsub").count() == 0, "second status line"
            pg.wait_for_selector("#ldact button", timeout=60000)
            assert "Riparti da zero" in pg.inner_text("#ldact") and "non risponde" in pg.inner_text("#ldmsg"), pg.inner_text("#loading")
            pg.click("#ldact button")
            pg.wait_for_selector("#drop", timeout=120000)
            assert "riparti" not in pg.url, pg.url
            pg.close()
        step("upgrade blocked by another tab: message, then «start from scratch» works", blocked)
        def keeps_notebook():
            holder.evaluate(GONE)
            holder.evaluate("""() => new Promise((res, rej) => {
              const r = indexedDB.open("qqq_lab", 2);
              r.onupgradeneeded = () => { const db = r.result; db.createObjectStore("files"); db.createObjectStore("kv"); db.createObjectStore("liste_utente", { keyPath: "id" }); };
              r.onsuccess = () => { const db = r.result, t = db.transaction(["files", "kv"], "readwrite"); t.objectStore("files").put(new ArrayBuffer(8), "old.mzML"); t.objectStore("kv").put('{"k":1}', "notebook"); t.oncomplete = () => { db.close(); res(true); }; };
              r.onerror = () => rej(String(r.error));
            })""")
            pg = ctx.new_page(); pg.goto(URL + "?riparti=1&motore=python")
            pg.wait_for_selector("#drop", timeout=120000)
            left = pg.evaluate("""() => new Promise(res => { const r = indexedDB.open("qqq_lab"); r.onsuccess = () => { const db = r.result, t = db.transaction(["files", "kv"]);
              const a = t.objectStore("files").count(), n = t.objectStore("kv").get("notebook"); t.oncomplete = () => { db.close(); res([a.result, n.result]); }; }; })""")
            assert left[0] == 0 and left[1] is None, left
            pg.close()
        step("«start from scratch» deletes the saved files and the notebook", keeps_notebook)
        def status_while_loading():
            holder.evaluate(GONE)
            pg = ctx.new_page(); pg.goto(URL + "?motore=python")
            pg.wait_for_selector("#ldmsg", state="attached")
            assert pg.locator("#ldbar").count() == 0 and pg.locator("#loading [role=progressbar]").count() == 0, "bar on the loading screen"
            assert pg.get_attribute("#ldmsg", "role") == "status" and pg.get_attribute("#loading", "aria-busy") == "true"
            seen = set()
            def poll():
                t = pg.evaluate("document.getElementById('ldmsg') ? document.getElementById('ldmsg').textContent : ''")
                if t: seen.add(t)
                return not pg.locator("#loading").is_visible() or pg.locator("#drop").count() > 0
            for _ in range(240):
                if poll(): break
                pg.wait_for_timeout(250)
            assert seen, "status line never had text"
            pg.wait_for_selector("#drop", timeout=120000); pg.close()
        step("one status line (role=status) and no bar on the loading screen", status_while_loading)
        def resume_after_freeze():
            holder.evaluate(GONE)
            pg = ctx.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
            # a Worker whose engine "answers badly" to the health check once __dead is set (what a Python left without GIL does after a freeze or discard)
            pg.add_init_script("""(() => { const pm = Worker.prototype.postMessage; Worker.prototype.postMessage = function (d, t) {
              if (d && d.ping && window.__dead) { const w = this; setTimeout(() => w.onmessage({ data: { type: "pong", ok: false } }), 10); return; } return pm.call(this, d, t); }; })()""")
            pg.goto(URL + "?motore=python"); pg.wait_for_selector("#drop", timeout=120000)      # the health check is of Pyodide
            pg.evaluate("window.__mark = 1")
            pg.evaluate("window.dispatchEvent(Object.assign(new Event('pageshow'), { persisted: true })); document.dispatchEvent(new Event('resume')); window.MZLAB_ENGINE_CHECK()")
            pg.wait_for_timeout(1500)
            assert pg.evaluate("window.__mark") == 1, "healthy engine: the page must not reload"
            r = pg.evaluate("fetch('api/ping').then(x => x.status)"); assert r == 200, r
            pg.evaluate("window.__dead = true; sessionStorage.removeItem('qqq.recovered')")
            with pg.expect_navigation(timeout=60000): pg.evaluate("window.MZLAB_ENGINE_CHECK()")
            pg.wait_for_selector("#drop", timeout=120000)
            assert pg.evaluate("window.__mark") is None and not pg.evaluate("window.qqFailed"), "page reloaded, no error"
            assert "NoGil" not in pg.inner_text("body")
            # second time within a minute: no reload loop, a short message in the language of the page
            pg.evaluate("window.__dead = true; window.__mark = 2; window.MZLAB_ENGINE_CHECK()")
            pg.wait_for_selector("#rust-note", timeout=10000)
            assert pg.evaluate("window.__mark") == 2 and "ricarica" in pg.inner_text("#rust-note"), pg.inner_text("#rust-note")
            pg.close()
        step("tab back after freeze/discard: engine checked, one reload, no NoGilError, no loop", resume_after_freeze)
        b.close()
finally:
    srv.terminate()
sys.exit(1 if errs else 0)
