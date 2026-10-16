"""The site keeps the mzML files and their .mzidx indexes in OPFS (mounted with WORKERFS, no copy in the memory of the engine); a reload does not read the
spectra again; what an older visit left in IndexedDB passes to OPFS; the gear shows the space and frees the indexes of removed files; «New session» empties
OPFS. Build first: python tools/build_site.py"""
from pathlib import Path
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, mz, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

LIST = """async (kind) => { const out = []; try { const r = await navigator.storage.getDirectory(); const d = await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle(kind);
  for await (const [n, h] of d.entries()) { if (n.endsWith('.part')) continue; try { out.push([n, (await h.getFile()).size]); } catch (e) { /* being written */ } } } catch (e) {} return out; }"""
IDBCOUNT = """() => new Promise(res => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => { const db = r.result;
  if (!db.objectStoreNames.contains('files')) return res(-1); const q = db.transaction('files').objectStore('files').count(); q.onsuccess = () => { db.close(); res(q.result); }; }; r.onerror = () => res(-2); })"""
NOTEBOOK = """() => new Promise(res => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => { const db = r.result; const q = db.transaction('kv').objectStore('kv').get('notebook');
  q.onsuccess = () => { db.close(); res(String(q.result || '')); }; }; r.onerror = () => res(''); })"""
FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(locale="it-IT", viewport={"width": 1400, "height": 1000}); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
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
        def manifest():
            # the manifest of qqq_lab has a row for every file of OPFS (kept up to date by the archive) and kv.schema says the data are at the current schema
            m = pg.evaluate("""async () => { const db = await new Promise((ok, no) => { const r = indexedDB.open("qqq_lab"); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); });
              const all = s => new Promise(ok => { const r = db.transaction(s).objectStore(s).getAll(); r.onsuccess = () => ok(r.result); });
              const kv = await new Promise(ok => { const r = db.transaction("kv").objectStore("kv").get("schema"); r.onsuccess = () => ok(r.result); });
              const rows = await all("manifest"); db.close(); return { kv, paths: rows.map(x => x.path).sort(), tipi: [...new Set(rows.map(x => x.tipo))].sort(), idx: rows.filter(x => x.path.startsWith("idx/")).map(x => [x.versione_formato, !!x.crate, !!x.sorgente_sha256]) }; }""")
            assert m["kv"] == 3, m
            assert m["idx"] == [[1, True, True]] * 3, m                      # the version, the crate and the source are read from each index
            assert sum(p.startswith("files/") for p in m["paths"]) == 3 and sum(p.startswith("idx/") for p in m["paths"]) == 3, m
        step("the manifest has a row for every file and the schema is current", manifest)
        def open_and_draw():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels[0]._a && E.panels[0]._a.sr.length") >= 1
            pg.click("#np-xic"); pg.fill("#xic-mz", "364"); pg.click("#xic-go"); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.some(p=>p.type==='xic' && p._a && p._a.sr.length>0)")
            assert not pg.evaluate("window.MZLAB_RUST.stats.mzmlOf || []"), pg.evaluate("window.MZLAB_RUST.stats.mzmlOf")      # no mzML read: everything came from the indexes
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
            assert not pg.evaluate("window.MZLAB_RUST.stats.mzmlOf || []"), pg.evaluate("window.MZLAB_RUST.stats.mzmlOf")      # the scan table comes from the index
        step("after a reload the scan table comes from the index (the spectra are not walked again)", reload)
        def removed():
            pg.evaluate("fetch('api/remove', {method:'POST', body: JSON.stringify({name: 'B_FullMass-t60.mzML'})}).then(r=>r.text())"); pg.wait_for_timeout(1500)
            j = pg.evaluate("fetch('api/spazio').then(r=>r.json())"); assert j["idx_removed"] == 0 and j["n_removed"] == 0, j      # the route frees the orphan index at once
            assert len(pg.evaluate(LIST, "idx")) == 2 and len(pg.evaluate(LIST, "files")) == 2
        step("the index of a removed file is freed first", removed)
        def gear():
            pg.click("#np-set"); pg.wait_for_selector("#uipset", timeout=10000)
            assert pg.locator("#uip-space-val").count() == 0 and pg.locator("#uip-arch-der").count() == 0, "no storage section in the gear"
            pg.keyboard.press("Escape")
            pg.evaluate("fetch('api/spazio?derivati=1', {method:'POST'}).then(r=>r.text())")          # the route stays: the indexes go, the files stay
            for _ in range(40):
                if pg.evaluate(LIST, "idx") == []: break
                pg.wait_for_timeout(500)
            assert pg.evaluate(LIST, "idx") == [] and len(pg.evaluate(LIST, "files")) == 2, (pg.evaluate(LIST, "idx"), pg.evaluate(LIST, "files"))
        step("the gear has no storage section; the derived data are freed by the route", gear)
        def new_session():
            pg.click("#newrun"); pg.wait_for_selector("#resetdlg [data-reset=files]", timeout=10000); pg.click("#resetdlg [data-reset=files]")
            ok = False
            for _ in range(60):
                try:
                    if pg.evaluate(LIST, "files") == [] and pg.evaluate(LIST, "idx") == []: ok = True; break
                except Exception: pass                     # the page is reloading
                pg.wait_for_timeout(500)
            assert ok, "OPFS not emptied"
        step("new session empties OPFS", new_session)
        # --- an older visit: files in IndexedDB, then the normal site
        ctx2 = b.new_context(locale="it-IT", viewport={"width": 1400, "height": 1000}); q = ctx2.new_page()
        q.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
        q.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        q.goto(URL); q.wait_for_selector("#drop", timeout=120000)
        # what the old versions did: the files in the IndexedDB store «files» and nothing in OPFS
        TO_IDB = """async () => { const root = await navigator.storage.getDirectory(), top = await root.getDirectoryHandle('mzlab'), d = await top.getDirectoryHandle('files'), put = [];
          for await (const [n, h] of d.entries()) put.push([n, await (await h.getFile()).arrayBuffer()]);
          const db = await new Promise((res, rej) => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error); });
          await new Promise(res => { const t = db.transaction('files', 'readwrite'); for (const [n, b] of put) t.objectStore('files').put(b, n); t.oncomplete = () => res(); });
          db.close(); await top.removeEntry('files', { recursive: true }); await top.removeEntry('idx', { recursive: true }); return put.length; }"""
        def old_way():
            q.set_input_files("#pick", FILES[:2])
            q.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            for _ in range(120):
                if len(q.evaluate(LIST, "idx")) == 2: break
                q.wait_for_timeout(500)
            assert q.evaluate(TO_IDB) == 2
            assert q.evaluate(IDBCOUNT) == 2 and q.evaluate(LIST, "files") == []
        step("an older visit left the files in IndexedDB", old_way)
        def migrate():
            q.goto(URL); q.wait_for_selector("#drop", timeout=120000); q.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000); q.wait_for_timeout(3000)
            assert q.evaluate(IDBCOUNT) == 0, q.evaluate(IDBCOUNT)
            assert len(q.evaluate(LIST, "files")) == 2, q.evaluate(LIST, "files")
            q.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length === 2", timeout=30000)
        step("the files of an older visit pass from IndexedDB to OPFS", migrate)
        ctx2.close(); b.close()
finally:
    srv.terminate()
print("ERRORS", errs)
sys.exit(1 if errs else 0)
