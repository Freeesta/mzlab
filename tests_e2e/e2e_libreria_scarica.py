"""Download of the public library packages on the built site (WebAssembly engine): the catalog and the pieces are synthetic (a real .mzlib cut into small pieces by
tools/librerie_pacchetti.py) and served from site/static/librerie/. Nothing is requested before the click; a corrupt piece is fetched again; an interrupted
download resumes from the first missing piece; the old package stays until the new one is verified; a cancel keeps the verified pieces; «update available» is known
at the next visit without asking the network.
Needs the site: python tools/build_site.py and the engine in site/static/wasm-lib (built here when cargo and wasm-bindgen exist; the CI builds it in pages.yml)."""
import base64, json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
from lib import copia as lib_copia, scrivi as lib_scrivi

sys.path.insert(0, str(ROOT / "tools"))
import librerie_pacchetti as lp

from lib import free_port; PORT = free_port()
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
for f in ("libreria-worker.js", "libreria.js", "libreria-cartella.js", "libreria-scarica.js"):
    lib_copia(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)
for f in (ROOT / "mzlab" / "web" / "lang").glob("*.js"):
    lib_copia(f, ROOT / "site" / "static" / "lang" / f.name)

def rec(name, prec, peaks):
    return f"Name: {name}\nPrecursorMZ: {prec}\nIon_mode: Positive\nPrecursor_type: [M+H]+\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.4f}\t{i:.1f}" for m, i in peaks) + "\n"
PEAKS = [(60.5, 100), (100.1, 400), (150.2, 900), (200.3, 300), (250.4, 200), (280.7, 150)]
MSP1 = "\n".join(rec(f"Voce{i}", 300.1 + i * 7, [(m + i, v) for m, v in PEAKS]) for i in range(40))
MSP2 = MSP1 + "\n" + rec("SoloNellaV2", 300.1, PEAKS)
QPEAKS = [list(p) for p in PEAKS]
OUT = ROOT / "site" / "static" / "librerie"
lp.PIECE = 2048

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
JS = """
window.T = {
  wipe: async () => { const root = await navigator.storage.getDirectory(); try { await root.removeEntry("librerie", { recursive: true }); } catch (e) {} },
  names: async () => { const out = []; const walk = async (d, pre) => { for await (const [n, h] of d.entries()) { out.push(pre + n); if (h.kind === "directory") await walk(h, pre + n + "/"); } }; try { await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle("librerie"), ""); } catch (e) {} return out.sort(); },
  states: async () => (await LIB.call("list")).voci.map(v => [v.nome, v.tipo, v.stato, v.versione || null]),
  search: async ([q, prec]) => { const r = await LIB.call("search", { peaks: q, prec: prec || 300.1, pol: 1, tol: { v: 10, unit: "ppm" }, frag: 0.01 }); return r.results.map(x => x.name); },
  b64: async (path) => { const parts = path.split("/"); let d = await (await navigator.storage.getDirectory()).getDirectoryHandle("librerie"); for (const p of parts.slice(0, -1)) d = await d.getDirectoryHandle(p); const b = new Uint8Array(await (await (await d.getFileHandle(parts[parts.length - 1])).getFile()).arrayBuffer()); let s = ""; for (let i = 0; i < b.length; i += 8192) s += String.fromCharCode(...b.subarray(i, i + 8192)); return btoa(s); },
};"""
hits, plan, held = {}, {}, []
def handler(route):
    u = route.request.url; k = u.split("/static/librerie/")[1]; hits[k] = hits.get(k, 0) + 1
    act = plan.get(k, lambda n: "ok")(hits[k])
    if act == "abort": return route.abort()
    if act == "corrupt":
        r = route.fetch(); b = bytearray(r.body()); b[3] ^= 0xFF; return route.fulfill(response=r, body=bytes(b))
    if act == "hold": held.append(route); return          # no answer until the test cancels: the page's fetch stays pending
    return route.continue_()

def publish(msp, version, wrong_total=False):
    """A package whose .mzlib is built by the engine of the page from `msp`: cut into pieces of 2 KiB and described in catalogo.json."""
    pg.evaluate("T.wipe()"); boot(); pg.evaluate("""async (t) => { await LIB.call("add", { file: new File([t], "x.msp", { type: "text/plain" }) }); }""", msp)
    raw = base64.b64decode(pg.evaluate("T.b64", ".mzlab/x.msp.mzlib"))
    with tempfile.TemporaryDirectory() as d:
        f = Path(d) / "p.mzlib"; f.write_bytes(raw)
        if OUT.exists(): shutil.rmtree(OUT)
        e = lp.entry("prova-pos", "Prova · ESI+ · MS2", 1, version, "Test", "https://example.org", "CC0; prova", {"CC0": 1}, {"Prova": 1}, f, OUT, "Attribuzione di prova " + version)
    if wrong_total: e["sha256"] = "0" * 64
    (OUT / "catalogo.json").write_text(json.dumps({"formato": "librerie/1", "pezzo": lp.PIECE, "librerie": [e], "esterne": []}), encoding="utf-8")
    pg.evaluate("T.wipe()"); boot()
    return e

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); ctx = b.new_context(viewport={"width": 1400, "height": 1000}, locale="it-IT"); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        reqs = []
        pg.on("request", lambda r: reqs.append(r.url) if "/static/librerie/" in r.url else None)
        pg.route("**/static/librerie/**/*.bin", handler)
        def boot():
            pg.goto(URL); pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=60000); pg.evaluate(JS)
        boot()
        e1 = publish(MSP1, "1"); assert len(e1["pezzi"]) >= 3, len(e1["pezzi"])
        boot()
        def no_network():
            pg.evaluate("LIB.open()"); pg.wait_for_selector("#libdlg #lib-avail-load", timeout=20000); pg.wait_for_selector("#libdlg #lib-list p.muted", timeout=20000)
            assert reqs == [], reqs
            assert "Nessuna libreria" in pg.inner_text("#libdlg #lib-list")
        step("opening the window asks the network for nothing", no_network)
        def show():
            pg.click("#lib-avail-load"); pg.wait_for_selector("#libdlg [data-dl='prova-pos']", timeout=20000)
            t = pg.text_content("#lib-avail-list"); assert "Prova · ESI+ · MS2" in t and "Non scaricata" in t and "CC0" in t and "Attribuzione di prova 1" in t, t
            assert any(u.endswith("catalogo.json") for u in reqs) and not any(u.endswith(".bin") for u in reqs), reqs
        step("the list comes on a click: name, version, licence, size, state and the attribution", show)
        def interrupted():
            plan["prova-pos/001.bin"] = lambda n: "corrupt" if n == 1 else "ok"
            plan["prova-pos/002.bin"] = lambda n: "abort" if n == 1 else "ok"
            pg.check("#libdlg [data-dl='prova-pos']"); pg.click("#lib-avail-go")
            pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('Nessuna connessione')", timeout=30000)
            assert hits["prova-pos/000.bin"] == 1 and hits["prova-pos/001.bin"] == 2, hits           # the corrupt piece was fetched again
            assert hits["prova-pos/002.bin"] == 1
            names = pg.evaluate("T.names()"); assert [n for n in names if n.endswith(".piece")] == [".mzlab/prova-pos.000.piece", ".mzlab/prova-pos.001.piece"], names
            assert "prova-pos.mzlib" not in names and pg.evaluate("T.states()") == []
        step("a corrupt piece is fetched again; an interruption keeps the verified pieces and installs nothing", interrupted)
        def resumed():
            pg.check("#libdlg [data-dl='prova-pos']"); pg.click("#lib-avail-go")
            pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('Scaricate e verificate: 1')", timeout=30000)
            assert hits["prova-pos/000.bin"] == 1 and hits["prova-pos/001.bin"] == 2 and hits["prova-pos/002.bin"] == 2, hits   # only what was missing
            names = pg.evaluate("T.names()"); assert "prova-pos.mzlib" in names and not any(n.endswith((".piece", ".new")) for n in names), names
            s = pg.evaluate("T.states()"); assert s == [["prova-pos.mzlib", "pacchetto", "pronta", "1"]], s
            assert "Scaricata" in pg.inner_text("#lib-avail-list")
            r = pg.evaluate("T.search", [[[m + 3, v] for m, v in PEAKS], 321.1]); assert "Voce3" in r, r
        step("the download resumes from the first missing piece and the package is ready and searchable", resumed)
        # a second version, published while the first one is installed
        def update_flow():
            # keep the installed v1, publish a v2 whose total is wrong: the pieces are fine one by one, the whole is not
            keep = pg.evaluate("T.b64", "prova-pos.mzlib"); side = pg.evaluate("T.b64", ".mzlab/prova-pos.mzlib.pkg.json"); cat = pg.evaluate("T.b64", ".mzlab/catalogo.json")
            pg.evaluate("""async ([a, b, c]) => { const w = async (p, s) => { const parts = p.split("/"); let d = await (await navigator.storage.getDirectory()).getDirectoryHandle("librerie", { create: true }); for (const q of parts.slice(0, -1)) d = await d.getDirectoryHandle(q, { create: true });
                const f = await (await d.getFileHandle(parts[parts.length - 1], { create: true })).createWritable(); await f.write(Uint8Array.from(atob(s), ch => ch.charCodeAt(0))); await f.close(); };
              await w("prova-pos.mzlib", a); await w(".mzlab/prova-pos.mzlib.pkg.json", b); await w(".mzlab/catalogo.json", c); }""", [keep, side, cat])
            e2 = publish(MSP2, "2", wrong_total=True)
            pg.evaluate("""async ([a, b, c]) => { const w = async (p, s) => { const parts = p.split("/"); let d = await (await navigator.storage.getDirectory()).getDirectoryHandle("librerie", { create: true }); for (const q of parts.slice(0, -1)) d = await d.getDirectoryHandle(q, { create: true });
                const f = await (await d.getFileHandle(parts[parts.length - 1], { create: true })).createWritable(); await f.write(Uint8Array.from(atob(s), ch => ch.charCodeAt(0))); await f.close(); };
              await w("prova-pos.mzlib", a); await w(".mzlab/prova-pos.mzlib.pkg.json", b); await w(".mzlab/catalogo.json", c); }""", [keep, side, cat])
            boot()
            s = pg.evaluate("T.states()"); assert s == [["prova-pos.mzlib", "pacchetto", "pronta", "1"]], s              # the cached catalog is the old one: nothing to update yet
            pg.evaluate("LIB.open()"); pg.click("#lib-avail-load"); pg.wait_for_selector("#libdlg [data-dl='prova-pos']", timeout=20000)
            assert "Aggiornamento disponibile" in pg.text_content("#lib-avail-list"), pg.text_content("#lib-avail-list")
            assert pg.evaluate("T.states()")[0][2] == "aggiornamento"
            pg.check("#libdlg [data-dl='prova-pos']"); pg.click("#lib-avail-go")
            pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('non valido')", timeout=30000)
            s = pg.evaluate("T.states()"); assert s[0][0] == "prova-pos.mzlib" and s[0][3] == "1", s                      # the old package is untouched
            names = pg.evaluate("T.names()"); assert not any(n.endswith((".piece", ".new")) for n in names), names
            assert pg.evaluate("T.b64", "prova-pos.mzlib") == keep
            # the right total: the update replaces it
            e2["sha256"] = hashlib_sha(OUT, e2); (OUT / "catalogo.json").write_text(json.dumps({"formato": "librerie/1", "pezzo": lp.PIECE, "librerie": [e2], "esterne": []}), encoding="utf-8")
            pg.click("#lib-avail-load"); pg.wait_for_selector("#libdlg [data-dl='prova-pos']"); pg.check("#libdlg [data-dl='prova-pos']"); pg.click("#lib-avail-go")
            pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('Scaricate e verificate: 1')", timeout=30000)
            s = pg.evaluate("T.states()"); assert s == [["prova-pos.mzlib", "pacchetto", "pronta", "2"]], s
            assert "SoloNellaV2" in pg.evaluate("T.search", [QPEAKS, 300.1])
        def hashlib_sha(out, e):
            import hashlib
            return hashlib.sha256(b"".join((out / p["file"]).read_bytes() for p in e["pezzi"])).hexdigest()
        step("an update with a wrong total fails and leaves the old package; with the right one it replaces it", update_flow)
        def known_next_visit():
            e3 = json.loads((OUT / "catalogo.json").read_text(encoding="utf-8"))["librerie"][0]; e3["versione"] = "3"
            (OUT / "catalogo.json").write_text(json.dumps({"formato": "librerie/1", "pezzo": lp.PIECE, "librerie": [e3], "esterne": []}), encoding="utf-8")
            pg.click("#lib-avail-load"); pg.wait_for_function("document.querySelector('#lib-avail-list').innerText.includes('Aggiornamento disponibile')", timeout=20000)
            before = len(reqs); boot()
            s = pg.evaluate("T.states()"); assert s == [["prova-pos.mzlib", "pacchetto", "aggiornamento", "2"]], s
            assert len(reqs) == before, "no network call without a click"
        step("«update available» is known at the next visit without asking the network", known_next_visit)
        def cancel():
            pg.evaluate("T.wipe()"); boot(); plan.clear(); hits.clear()
            plan["prova-pos/001.bin"] = lambda n: "hold"
            pg.evaluate("LIB.open()"); pg.click("#lib-avail-load"); pg.wait_for_selector("#libdlg [data-dl='prova-pos']"); pg.check("#libdlg [data-dl='prova-pos']"); pg.click("#lib-avail-go")
            pg.wait_for_selector("#lib-avail-cancel:not([hidden])"); pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('pezzo 1 di')", timeout=20000)
            pg.click("#lib-avail-cancel")
            pg.wait_for_function("document.querySelector('#lib-avail-st').innerText.includes('annullato')", timeout=30000)
            names = pg.evaluate("T.names()"); assert "prova-pos.mzlib" not in names and ".mzlab/prova-pos.000.piece" in names, names
            for r in held:
                try: r.abort()
                except Exception: pass
        step("cancel stops the download, keeps the verified pieces and installs nothing", cancel)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(OUT, ignore_errors=True)
