"""mzFinder, ingress (TP_Mine/files/finder-idx.js): a high-resolution mzML that has no .mzidx in OPFS (picked with «Choose mzML...» or taken from the IndexedDB of an older visit)
gets its index from the SAME builder as the main program (same bytes), then classify and the Dig run in Rust, equal to the case «Reload from Data», and Pyodide is never requested.
Builds the site (python3 tools/build_site.py) and static/wasm-feat (tools/build_feat_wasm.sh); SKIP without cargo, wasm-bindgen or numpy. MZLAB_PIPE_SITE=<built site with
static/wasm-feat> reuses a site (the TP_Mine files are copied over it). Run from the repo root: python3 tests_e2e/e2e_tpmine_ingresso.py"""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="tpmine_ingresso_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
mzml = sorted(str(f) for f in ser.glob("*.mzML"))
names = [Path(f).name for f in mzml]
PARENT = "C21H32N2O2"

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
# SHA-256 of every .mzidx of OPFS, by name ({} when there is none)
SHAS = """async () => { const out = {}; try { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx');
  for await (const [n, h] of d.entries()) { if (n.endsWith('.part')) continue; const b = await (await h.getFile()).arrayBuffer();
    out[n] = [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join(''); } } catch (e) {} return out; }"""
# the files of an older visit: the bytes in the object store «files» of qqq_lab
TO_IDB = """async (list) => { const db = await new Promise((res, rej) => { const q = indexedDB.open('qqq_lab'); q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error); });
  const bufs = []; for (const u of list) bufs.push([u.split('/').pop(), await (await fetch(u)).arrayBuffer()]);
  await new Promise((res, rej) => { const tx = db.transaction('files', 'readwrite'), s = tx.objectStore('files'); for (const [n, b] of bufs) s.put(b, n); tx.oncomplete = res; tx.onerror = () => rej(tx.error); }); db.close(); }"""
TABLE = "[...document.querySelectorAll('#hr-t tr.clk')].map(r => r.textContent.replace(/\\s+/g, ' ').trim())"
data = site / "static" / "dati_finder"; data.mkdir(exist_ok=True)
for f in mzml: shutil.copy2(f, data / Path(f).name)


def dig(pg):
    pg.fill("#tp-mol", PARENT); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
    t = pg.evaluate(TABLE)
    for _ in range(40):                                     # some cells (t0, Blank) arrive a moment after the rows: read until two reads agree
        pg.wait_for_timeout(500); u = pg.evaluate(TABLE)
        if u == t: break
        t = u
    return t


def diff(a, b):                                           # the first row that differs, for the log of a red run
    for i in range(max(len(a), len(b))):
        x, y = (a[i] if i < len(a) else None), (b[i] if i < len(b) else None)
        if x != y: return f"{len(a)} vs {len(b)} rows; row {i}: {x!r} vs {y!r}"
    return "same rows"


def rows_ready(pg):
    pg.wait_for_function(f"document.querySelectorAll('#tp-files tr').length >= {len(names) + 1}", timeout=240000)
    assert "LC-HRMS" in pg.inner_text("#tp-fsum"), pg.inner_text("#tp-fsum")


INIT = "try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}"


def fresh(b):
    """A new browser profile (empty OPFS and IndexedDB), the page loaded and mzFinder ready."""
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.add_init_script(INIT)
    pg.on("console", lambda m: print("console:", m.text[:300], flush=True) if "mzFinder" in m.text else None)      # why the engine fell back, in the log of a red run
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.context.on("request", lambda r: reqs.append(r.url))
    pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
    return pg


try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = fresh(b)
        ref = {}

        def from_data():                                    # the main program uploads and indexes the files; mzFinder takes them from the Data tab
            pg.set_input_files("#pick", mzml)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(names)}", timeout=120000)
            for _ in range(240):
                ref.update(pg.evaluate(SHAS))
                if len(ref) == len(names): break
                pg.wait_for_timeout(500)
            assert len(ref) == len(names), ref
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.click("#tp-page"); rows_ready(pg)
            ref["table"] = dig(pg)
            assert ref["table"], "the Dig from the Data tab has no candidate"
        step("reference: the Data tab indexes the files, mzFinder digs from it", from_data)

        same = lambda got: got == {k: v for k, v in ref.items() if k != "table"}

        def settle(q):                                      # the indexes of the last files may still be written when the table has its rows
            got = q.evaluate(SHAS)
            for _ in range(240):
                if same(got): break
                q.wait_for_timeout(500); got = q.evaluate(SHAS)
            return got

        def chosen():                                       # a new profile: no file, no index in OPFS; «Choose mzML...» gives the files to the worker
            q = fresh(b); assert q.evaluate(SHAS) == {}
            q.click('#nav button[data-v="tpmine"]'); q.wait_for_selector("#tp-go")
            q.set_input_files("#tp-in", mzml); rows_ready(q)
            got = settle(q); assert same(got), (got, ref)
            t = dig(q); assert t == ref["table"], "the Dig of the chosen files differs from the one from the Data tab: " + diff(t, ref["table"])
        step("Choose mzML...: the .mzidx equals the one of the main program, the Dig equals the one from the Data tab", chosen)

        def from_idb():                                     # a new profile with the bytes only in IndexedDB (an older visit), no index in OPFS
            q = fresh(b)
            q.evaluate(TO_IDB, [f"static/dati_finder/{n}" for n in names])
            q.click('#nav button[data-v="tpmine"]'); q.wait_for_selector("#tp-go")
            q.click("#tp-page"); rows_ready(q)
            got = settle(q); assert same(got), (got, ref)
            t = dig(q)
            assert t == ref["table"], "the Dig of the IndexedDB files differs: " + diff(t, ref["table"]) + " | files: " + q.inner_text("#tp-files")[:300].replace("\n", " ")
        step("IndexedDB: same index, same Dig", from_idb)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
        step("no pyodide* request with high-resolution input", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
