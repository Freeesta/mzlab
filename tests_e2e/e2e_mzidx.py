"""The site keeps the mzML files and their .mzidx indexes in OPFS (mounted with WORKERFS, no copy in the memory of the engine); a reload does not read the
spectra again; what an older visit left in IndexedDB passes to OPFS; the gear shows the space and frees the indexes of removed files; «New session» empties
OPFS. Build first: python tools/build_site.py"""
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, mz, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

LIST = """async (kind) => { const out = []; try { const r = await navigator.storage.getDirectory(); const d = await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle(kind);
  for await (const [n, h] of d.entries()) out.push([n, (await h.getFile()).size]); } catch (e) {} return out; }"""
IDBCOUNT = """() => new Promise(res => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => { const db = r.result;
  if (!db.objectStoreNames.contains('files')) return res(-1); const q = db.transaction('files').objectStore('files').count(); q.onsuccess = () => { db.close(); res(q.result); }; }; r.onerror = () => res(-2); })"""
NOTEBOOK = """() => new Promise(res => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => { const db = r.result; const q = db.transaction('kv').objectStore('kv').get('notebook');
  q.onsuccess = () => { db.close(); res(String(q.result || '')); }; }; r.onerror = () => res(''); })"""
FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(locale="it-IT", viewport={"width": 1400, "height": 1000}); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        def upload():
            pg.set_input_files("#pick", FILES)
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 3", timeout=120000)
            got = pg.evaluate(LIST, "files"); assert len(got) == 3 and all(s > 1000 for _, s in got), got
        step("uploaded mzML are written to OPFS", upload)
        def indexes():
            for _ in range(120):
                if len(pg.evaluate(LIST, "idx")) == 3: break
                pg.wait_for_timeout(500)
            got = pg.evaluate(LIST, "idx"); assert len(got) == 3 and all(n.endswith(".mzML.mzidx") for n, _ in got), got
        step("the .mzidx of each file appears in OPFS right after the upload", indexes)
        def open_and_draw():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels[0]._a && E.panels[0]._a.sr.length") >= 1
            pg.click("#np-xic"); pg.fill("#xic-mz", "364"); pg.click("#xic-go"); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.some(p=>p.type==='xic' && p._a && p._a.sr.length>0)")
            perf = pg.evaluate("fetch('api/perf').then(r=>r.json())"); assert all(f["indexed"] for f in perf["files"]), perf
        step("TIC and XIC are drawn from the mounted index", open_and_draw)
        def space():
            j = pg.evaluate("fetch('api/spazio').then(r=>r.json())"); assert j["opfs"] and j["files"] > 0 and j["idx"] > 0 and j["idx_removed"] == 0, j
        step("api/spazio reports files and indexes", space)
        def reload():
            for _ in range(40):                                  # the notebook (the open files) is saved a moment after the last change
                if '"session"' in pg.evaluate(NOTEBOOK): break
                pg.wait_for_timeout(250)
            pg.reload(); pg.wait_for_selector(".pnl canvas", timeout=120000); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.files.length") == 3, pg.evaluate("E.files.length")
            perf = pg.evaluate("fetch('api/perf').then(r=>r.json())")
            assert perf["files"] and all(f["indexed"] and "costruzione .mzidx" not in f["timing"] for f in perf["files"]), perf
        step("after a reload the scan table comes from the index (the spectra are not walked again)", reload)
        def removed():
            pg.evaluate("fetch('api/remove', {method:'POST', body: JSON.stringify({name: 'B_FullMass-t60.mzML'})}).then(r=>r.text())"); pg.wait_for_timeout(1500)
            j = pg.evaluate("fetch('api/spazio').then(r=>r.json())"); assert j["idx_removed"] > 0 and j["n_removed"] == 1, j
            j = pg.evaluate("fetch('api/spazio', {method:'POST'}).then(r=>r.json())"); assert j["idx_removed"] == 0, j
            assert len(pg.evaluate(LIST, "idx")) == 2 and len(pg.evaluate(LIST, "files")) == 2
        step("the index of a removed file is freed first", removed)
        def gear():
            pg.click("#np-set"); pg.wait_for_selector("#uip-space-row:not([hidden])", timeout=10000)
            t = pg.inner_text("#uip-space-val"); assert "MB" in t, t
            pg.keyboard.press("Escape")
        step("the gear shows the space", gear)
        def new_session():
            pg.click("#newrun"); pg.wait_for_selector("#askok", timeout=10000); pg.click("#askok")
            ok = False
            for _ in range(60):
                try:
                    if pg.evaluate(LIST, "files") == [] and pg.evaluate(LIST, "idx") == []: ok = True; break
                except Exception: pass                     # the page is reloading
                pg.wait_for_timeout(500)
            assert ok, "OPFS not emptied"
        step("new session empties OPFS", new_session)
        # --- an older visit: files in IndexedDB (?opfs=0), then the normal site
        ctx2 = b.new_context(locale="it-IT", viewport={"width": 1400, "height": 1000}); q = ctx2.new_page()
        q.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
        q.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
        q.goto(URL + "?opfs=0"); q.wait_for_selector("#drop", timeout=120000)
        def old_way():
            q.set_input_files("#pick", FILES[:2])
            q.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000); q.wait_for_timeout(1500)
            assert q.evaluate(IDBCOUNT) == 2 and q.evaluate(LIST, "files") == []
        step("?opfs=0: files stay in IndexedDB, as before", old_way)
        def migrate():
            q.goto(URL); q.wait_for_selector("#drop", timeout=120000); q.wait_for_function("window.qqReady !== false", timeout=60000); q.wait_for_timeout(3000)
            assert q.evaluate(IDBCOUNT) == 0, q.evaluate(IDBCOUNT)
            assert len(q.evaluate(LIST, "files")) == 2, q.evaluate(LIST, "files")
            q.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length === 2", timeout=30000)
        step("the files of an older visit pass from IndexedDB to OPFS", migrate)
        ctx2.close(); b.close()
finally:
    srv.terminate()
print("ERRORS", errs)
sys.exit(1 if errs else 0)
