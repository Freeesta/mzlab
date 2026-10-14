"""The «Cartella delle librerie» on the built site (WebAssembly engine): the OPFS folder `librerie/` and a folder handle (the same code as showDirectoryPicker, here an OPFS
subdirectory): inventory and the four states (ready, to index, update available is in e2e_libreria_scarica, damaged), a file that changes (touched = still ready, changed = to
index), an index written once in `.mzlab/`, a package `.mzlib` put in the folder, an imported file, removing the index never removes the source, the window with the states.
Needs the site: python tools/build_site.py and the engine in site/static/wasm-lib (built here when cargo and wasm-bindgen exist; the CI builds it in pages.yml)."""
from pathlib import Path
import json, os, shutil, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8837
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:500]))

wasm = ROOT / "site" / "static" / "wasm-lib" / "mzlab_lib_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm"], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_lib_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm-lib missing and no cargo + wasm-bindgen to build it")); sys.exit(0)
for f in ("libreria-worker.js", "libreria.js", "libreria-cartella.js"):
    shutil.copyfile(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)
for f in (ROOT / "mzlab" / "web" / "lang").glob("*.js"):
    shutil.copyfile(f, ROOT / "site" / "static" / "lang" / f.name)

def rec(name, prec, peaks):
    return f"Name: {name}\nPrecursorMZ: {prec}\nIon_mode: Positive\nPrecursor_type: [M+H]+\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in peaks) + "\n"
PEAKS = [(60.5, 100), (100.1, 400), (150.2, 900), (200.3, 300), (250.4, 200), (280.7, 150)]
MSP_A = "\n".join([rec("Match", 300.1, PEAKS), rec("Lontano", 400.1, PEAKS)])
MSP_B = rec("Fratello", 300.1, PEAKS[:4]) + "\n" + rec("Altro", 500.1, PEAKS)
MSP_A2 = MSP_A + "\n" + rec("Nuovo", 600.1, PEAKS)
QPEAKS = [list(p) for p in PEAKS]

# write / read / remove inside the OPFS folder `librerie/` (or a subdirectory of it standing for a chosen folder)
JS = """
window.T = {
  dir: async (sub) => { let d = await (await navigator.storage.getDirectory()).getDirectoryHandle("librerie", { create: true }); if (sub) d = await d.getDirectoryHandle(sub, { create: true }); return d; },
  put: async ([path, text, sub]) => { const parts = path.split("/"); let d = await T.dir(sub); for (const p of parts.slice(0, -1)) d = await d.getDirectoryHandle(p, { create: true }); const fh = await d.getFileHandle(parts[parts.length - 1], { create: true }); const w = await fh.createWritable(); await w.write(text); await w.close(); },
  size: async ([path, sub]) => { const parts = path.split("/"); let d = await T.dir(sub); try { for (const p of parts.slice(0, -1)) d = await d.getDirectoryHandle(p); return (await (await d.getFileHandle(parts[parts.length - 1])).getFile()).size; } catch (e) { return null; } },
  trunc: async ([path, n, sub]) => { const parts = path.split("/"); let d = await T.dir(sub); for (const p of parts.slice(0, -1)) d = await d.getDirectoryHandle(p); const f = await (await d.getFileHandle(parts[parts.length - 1])).getFile(), b = await f.slice(0, n).arrayBuffer(); const w = await (await d.getFileHandle(parts[parts.length - 1])).createWritable(); await w.write(b); await w.close(); },
  copy: async ([from, to, sub]) => { const parts = from.split("/"); let d = await T.dir(sub); for (const p of parts.slice(0, -1)) d = await d.getDirectoryHandle(p); const f = await (await d.getFileHandle(parts[parts.length - 1])).getFile(); const top = await T.dir(sub); const w = await (await top.getFileHandle(to, { create: true })).createWritable(); await w.write(await f.arrayBuffer()); await w.close(); },
  states: async () => (await LIB.call("list")).voci.map(v => [v.nome, v.tipo, v.stato]),
  search: async (q) => { const r = await LIB.call("search", { peaks: q, prec: 300.1, pol: 1, tol: { v: 10, unit: "ppm" }, frag: 0.01 }); return r.results.map(x => x.name); },
  wipe: async () => { const root = await navigator.storage.getDirectory(); try { await root.removeEntry("librerie", { recursive: true }); } catch (e) {} },
};"""

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); ctx = b.new_context(viewport={"width": 1400, "height": 1000}, locale="it-IT"); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and not r.url.endswith("favicon.ico") else None)
        def boot():
            pg.goto(URL); pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=60000); pg.evaluate(JS)
        boot(); pg.evaluate("T.wipe()")
        def run_folder(sub, label):
            S = lambda: pg.evaluate("T.states()")
            def empty():
                r = pg.evaluate("LIB.call('list')"); assert r["voci"] == [] and r["folder"]["mode"] == label, r
            step(f"[{label}] an empty folder", empty)
            def fresh():
                pg.evaluate("T.put", ["a.msp", MSP_A, sub]); pg.evaluate("T.put", ["b.msp", MSP_B, sub]); pg.evaluate("T.put", ["notes.pdf", "x", sub])
                s = S(); assert s == [["a.msp", "sorgente", "da_indicizzare"], ["b.msp", "sorgente", "da_indicizzare"]], s
            step(f"[{label}] new sources are «to index», other files are ignored", fresh)
            def index():
                r = pg.evaluate("LIB.call('index', { name: 'a.msp' })"); assert r["info"]["n"] == 2, r
                s = S(); assert s[0] == ["a.msp", "sorgente", "pronta"] and s[1][2] == "da_indicizzare", s
                assert pg.evaluate("T.size", [".mzlab/a.msp.mzlib", sub]) > 100 and pg.evaluate("T.size", [".mzlab/a.msp.idx.json", sub]) > 50
                assert pg.evaluate("T.size", ["a.msp", sub]) == len(MSP_A.encode()), "the source must stay as it is"
                assert pg.evaluate("T.search", QPEAKS) == ["Match"], pg.evaluate("T.search", QPEAKS)
            step(f"[{label}] indexed once into `.mzlab/`, the source untouched, the library searchable", index)
            def touched():
                before = pg.evaluate("T.size", [".mzlab/a.msp.mzlib", sub])
                time.sleep(0.05); pg.evaluate("T.put", ["a.msp", MSP_A, sub])             # same bytes, new date: the hash says «unchanged»
                s = S(); assert s[0] == ["a.msp", "sorgente", "pronta"], s
                assert pg.evaluate("T.size", [".mzlab/a.msp.mzlib", sub]) == before
            step(f"[{label}] a touched file with the same content stays ready", touched)
            def changed():
                pg.evaluate("T.put", ["a.msp", MSP_A2, sub]); s = S(); assert s[0] == ["a.msp", "sorgente", "da_indicizzare"], s
                r = pg.evaluate("LIB.call('index', { name: 'a.msp' })"); assert r["info"]["n"] == 3, r
                assert S()[0][2] == "pronta" and pg.evaluate("T.search", QPEAKS) == ["Match"]
            step(f"[{label}] a changed source is «to index» again and its new index replaces the old", changed)
            def damaged():
                pg.evaluate("LIB.call('index', { name: 'b.msp' })"); pg.evaluate("T.trunc", [".mzlab/b.msp.mzlib", 40, sub])
                s = S(); assert s[1] == ["b.msp", "sorgente", "danneggiata"], s
                r = pg.evaluate("LIB.call('search', { peaks: %s, prec: 300.1, pol: 1, tol: { v: 10, unit: 'ppm' }, frag: 0.01 }).then(r => r.results.map(x => x.name))" % json.dumps(QPEAKS)); assert r == ["Match"], r
                pg.evaluate("LIB.call('index', { name: 'b.msp' })"); assert S()[1][2] == "pronta"
            step(f"[{label}] a truncated index is «damaged» (the search goes on with the others) and indexing again mends it", damaged)
            def package():
                pg.evaluate("T.copy", [".mzlab/b.msp.mzlib", "c.mzlib", sub]); pg.evaluate("T.put", ["d.mzlib", "this is not a library at all, only text", sub])
                s = {x[0]: x for x in S()}; assert s["c.mzlib"] == ["c.mzlib", "pacchetto", "pronta"] and s["d.mzlib"][2] == "danneggiata", s
            step(f"[{label}] a .mzlib in the folder is ready when it opens, damaged when it does not", package)
            def imported():
                pg.evaluate("""async () => { await LIB.call("add", { file: new File([%s], "esterna.msp", { type: "text/plain" }) }); }""" % json.dumps(rec("Esterno", 300.1, PEAKS)))
                s = {x[0]: x for x in S()}; assert s["esterna.msp"] == ["esterna.msp", "importata", "pronta"], s
                assert "Esterno" in pg.evaluate("T.search", QPEAKS)
            step(f"[{label}] an imported file is indexed without being copied", imported)
            def remove():
                pg.evaluate("LIB.call('remove', { id: 'f:a.msp' })"); s = {x[0]: x for x in S()}
                assert s["a.msp"][2] == "da_indicizzare" and pg.evaluate("T.size", ["a.msp", sub]) == len(MSP_A2.encode()), s
                pg.evaluate("LIB.call('remove', { id: 'f:c.mzlib' })"); assert pg.evaluate("T.size", ["c.mzlib", sub]) is None
            step(f"[{label}] removing a source only removes its index; a package is removed", remove)
        run_folder(None, "opfs")
        def window_():
            pg.evaluate("LIB.open()"); pg.wait_for_selector("#libdlg #lib-list table", timeout=20000)
            t = pg.inner_text("#libdlg #lib-list"); assert "Pronta" in t and "Da indicizzare" in t and "Danneggiata" in t, t
            assert "Archivio del browser" in pg.inner_text("#libdlg #lib-fwhere")
            pg.click("#libdlg [data-idx='a.msp']"); pg.wait_for_selector("#libdlg [data-rm='f:a.msp']", timeout=30000)
            pg.click("#lib-x")
        step("the window shows folder, states and the «Indicizza» action", window_)
        def persist():
            boot(); s = pg.evaluate("T.states()"); assert ["a.msp", "sorgente", "pronta"] in s and ["esterna.msp", "importata", "pronta"] in s, s
            assert "Esterno" in pg.evaluate("T.search", QPEAKS)
        step("after a reload the folder and its libraries are still there", persist)
        pg.evaluate("T.wipe()"); boot()
        pg.evaluate("""async () => { await LIB.call("folder", { mode: "fs", handle: await T.dir("scelta") }); }""")
        run_folder("scelta", "fs")
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
