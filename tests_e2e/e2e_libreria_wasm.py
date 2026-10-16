"""Spectral libraries with the WebAssembly engine (crates/mzlab-lib-wasm) on the built site: a synthetic MSP is imported into a .mzlib in OPFS, identity and
analogues are searched, the library survives a reload, an old MZL1 archive (made with ?libjs=1) is converted once and its old files go, a dead engine is
replaced (and a second death within a minute sends the page to the JavaScript path), and the .wasm stays under 600 KB.
Needs the site: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm-lib)."""
from pathlib import Path
import json, os, shutil, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT
from lib import copia as lib_copia, scrivi as lib_scrivi

from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:400]))

wasm = ROOT / "site" / "static" / "wasm-lib" / "mzlab_lib_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
built_here = False
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        built_here = True
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm"], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_lib_wasm.wasm"], cwd=ROOT, check=True)
        if shutil.which("wasm-opt"):
            bg = str(wasm.parent / "mzlab_lib_wasm_bg.wasm")
            subprocess.run(["wasm-opt", "-Oz", "--enable-bulk-memory", "--enable-sign-ext", "--enable-mutable-globals", "--enable-nontrapping-float-to-int", "-o", bg, bg], cwd=ROOT, check=True)
            built_here = False                                         # the final size can be checked
    else:
        print("  ", ("wasm", "FAIL site/static/wasm-lib missing and no cargo + wasm-bindgen to build it")); sys.exit(0)
# the page of the site is a copy: make sure it is the current worker
for f in ("libreria-worker.js", "libreria.js", "libreria-cartella.js"):
    lib_copia(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)

def rec(name, prec, peaks, extra="Ion_mode: Positive\nPrecursor_type: [M+H]+\nCollision_energy: 30\nInstrument_type: QTRAP\n"):
    return f"Name: {name}\nPrecursorMZ: {prec}\n{extra}Num Peaks: {len(peaks)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in peaks) + "\n"
PEAKS = [(60.5, 100), (100.1, 400), (150.2, 900), (200.3, 300), (250.4, 200), (280.7, 150)]
MSP = "\n".join([rec("Match", 300.1, PEAKS), rec("Spostato", 300.1, [(m + 40, i) for m, i in PEAKS]), rec("Parziale", 300.1, PEAKS[:3]), rec("Lontano", 400.1, PEAKS),
                 rec("Negativo", 300.1, PEAKS, "Ion_mode: Negative\nPrecursor_type: [M-H]-\n"), "Name: Senza precursore\nNum Peaks: 1\n100 100\n"])
ANA = "\n".join([rec("Analogo", 300.1, PEAKS), rec("Estraneo", 310.0, [(55.0, 100), (95.0, 300), (120.0, 800), (180.0, 300), (222.0, 100)])])
QPEAKS = [list(p) for p in PEAKS]
APEAKS = [[m + 16 if m > 120 else m, i] for m, i in PEAKS]          # the analogue plus 16 Da on the last peaks
ADD = """async ([name, text]) => { const r = await LIB.call("add", { file: new File([text], name, { type: "text/plain" }) }); return r.info; }"""
SEARCH = """async ([peaks, prec, pol]) => { const r = await LIB.call("search", { peaks, prec, pol, tol: { v: 10, unit: "ppm" }, frag: 0.01 }); return { names: r.results.map(x => x.name), ent: r.results.map(x => x.ent), cos: r.results.map(x => x.cos), shared: r.results.map(x => x.shared), lic: r.results.map(x => x.lib), q: r.query.mz.length }; }"""
FILES = """async () => { const out = []; const walk = async (d, pre) => { for await (const [n, h] of d.entries()) { out.push(pre + n); if (h.kind === "directory") await walk(h, pre + n + "/"); } }; await walk(await navigator.storage.getDirectory(), ""); return out.sort(); }"""

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); ctx = b.new_context(viewport={"width": 1400, "height": 1000}); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        def boot(url=URL):
            pg.goto(url); pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=60000)
        def engine():
            boot(); e = pg.evaluate("LIB.engine.info()"); assert e["wasm"] is True, e
            size = (ROOT / "site" / "static" / "wasm-lib" / "mzlab_lib_wasm_bg.wasm").stat().st_size
            if not built_here: assert size < 600 * 1024, f"the .wasm is {size} bytes (limit 600 KB)"      # built here without wasm-opt: the size is only checked in the CI, which optimises it
        step("the WebAssembly engine loads (the .wasm is under 600 KB)", engine)
        def imp():
            info = pg.evaluate(ADD, ["prova.msp", MSP])
            assert info["store"] == "mzlib" and info["n"] == 5 and info["dropped"] == 1 and info["noPrec"] == 1 and info["pos"] == 4 and info["neg"] == 1, info
            f = pg.evaluate(FILES); assert any(x.endswith(".mzlib") for x in f) and not any((x.endswith(".tmp") and x != "probe.tmp") or x.endswith(".idx") for x in f) and any(x.startswith("librerie/.mzlab/prova.msp.") for x in f), f
        step("import: an index in `.mzlab/` of the OPFS folder, no temporary file, one record discarded", imp)
        def search():
            r = pg.evaluate(SEARCH, [QPEAKS, 300.1, 1])
            assert r["names"][0] == "Match" and "Negativo" not in r["names"] and "Lontano" not in r["names"] and {"Spostato", "Parziale"} <= set(r["names"]), r
            assert r["ent"][0] > 0.999 and r["cos"][0] > 0.999 and r["shared"][0] == 6, r
            assert r["ent"] == sorted(r["ent"], reverse=True) and r["q"] == 6, r
        step("identity: Match first, other polarity and far precursor left out", search)
        def listing():
            pg.evaluate("LIB.open()"); pg.wait_for_selector("#libdlg #lib-list table", timeout=20000)
            t = pg.inner_text("#libdlg #lib-list"); assert "prova" in t and "5" in t, t
            pg.click("#lib-x")
        step("the Librerie window lists it", listing)
        def analogs():
            info = pg.evaluate(ADD, ["analoghi.msp", ANA]); assert info["n"] == 2, info
            r = pg.evaluate("""async (q) => { const r = await LIB.call("analogs", { peaks: q, prec: 316.1, pol: 1, frag: 0.01, maxDelta: 200, max: 10 }); return r.results.map(x => ({ name: x.name, mcos: x.mcos, delta: x.delta, shifted: x.shifted, matched: x.matched })); }""", APEAKS)
            a = [x for x in r if x["name"] == "Analogo"]
            assert a and a[0]["mcos"] > 0.99 and abs(a[0]["delta"] - 16.0) < 1e-6 and a[0]["shifted"] == 4 and a[0]["matched"] == 6 and not any(x["name"] == "Estraneo" for x in r), r
        step("analogues: the shifted neighbour with delta 16 and 4 shifted peaks", analogs)
        def reload_():
            boot(); l = pg.evaluate("LIB.call('list').then(r => r.libs.map(x => [x.name, x.store, x.n]))")
            assert ["prova", "mzlib", 5] in l and ["analoghi", "mzlib", 2] in l, l
            r = pg.evaluate(SEARCH, [QPEAKS, 300.1, 1]); assert r["names"][0] == "Match", r
        step("after a reload the libraries are still there and searchable", reload_)
        def remove():
            for x in pg.evaluate("LIB.call('list').then(r => r.libs.map(x => x.id))"): pg.evaluate("id => LIB.call('remove', { id })", x)
            f = pg.evaluate(FILES); assert not any(n.endswith((".mzlib", ".info.json")) or (n.endswith(".tmp") and n != "probe.tmp") for n in f), f
        step("remove: every file of the library goes", remove)
        def migrate():
            boot(URL + "?libjs=1"); e = pg.evaluate("LIB.engine.info()"); assert e["wasm"] is False, e
            info = pg.evaluate(ADD, ["vecchia.msp", MSP]); assert info.get("store") is None and info["n"] == 5, info
            f = pg.evaluate(FILES); assert any(x.endswith(".idx") for x in f) and not any(x.endswith(".mzlib") for x in f), f
            r0 = pg.evaluate(SEARCH, [QPEAKS, 300.1, 1]); assert r0["names"][0] == "Match", r0
            boot(); l = pg.evaluate("LIB.call('list').then(r => r.libs.map(x => [x.name, x.store, x.n]))")
            assert l == [["vecchia", "mzlib", 5]], l
            f = pg.evaluate(FILES); assert any(x.endswith(".mzlib") for x in f) and not any(x.endswith((".idx", ".peaks", ".meta.json")) for x in f), f
            r1 = pg.evaluate(SEARCH, [QPEAKS, 300.1, 1]); assert r1["names"][0] == "Match" and r1["ent"][0] > 0.99 and {"Spostato", "Parziale"} <= set(r1["names"]), r1
        step("an old MZL1 archive is converted once, the old files go, the search still works", migrate)
        def die_once():
            pg.evaluate("LIB.engine.crash()")
            r = pg.evaluate(SEARCH, [QPEAKS, 300.1, 1]); assert r["names"][0] == "Match", r
            assert pg.evaluate("LIB.engine.info().then(e => e.wasm)") is True
        step("the engine dies: a new worker answers", die_once)
        def die_twice():
            pg.evaluate("LIB.engine.crash()")
            assert pg.evaluate("LIB.engine.fallback") is True
            msg = pg.evaluate("""LIB.call("search", { peaks: %s, prec: 300.1, pol: 1, tol: { v: 10, unit: "ppm" }, frag: 0.01 }).then(() => "ok", e => e.key)""" % json.dumps(QPEAKS))
            assert msg == "lib.err.needWasm", msg
            assert pg.evaluate("LIB.engine.info().then(e => e.wasm)") is False
        step("a second death within a minute: the JavaScript path, the .mzlib asks for a reload", die_twice)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
