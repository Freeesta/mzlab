"""The readers modules, one per family (crates/mzlab-lettori-wasm, mzlab/web/lettori-worker.js, rule R of shell.js upload0) proved with a reader the engine already has: with ?lettori=modulo a synthetic Agilent .D folder
(tools/agilent_sintetico.py, no laboratory data) goes to the module, which writes its mzML in OPFS; the file enters the list as any mzML, gives the same TIC as the engine's own path, «Download .mzML» gives it
byte for byte as the engine writes it and without converting again, and nothing misses the engine (MZLAB_LR_MISS empty). Needs the site and both .wasm: python tools/build_site.py, tools/build_lettori_wasm.sh
(MZLAB_PIPE_SITE=<built site> reuses a site). Run from the repo root: python3 tests_e2e/e2e_lettori_modulo.py"""
import os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import agilent_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="lettori_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
N = 120
folder = agilent_sintetico.write(tmp / "serie", "uno.D", N)
top = folder.parent
lc_top = tmp / "lc"
agilent_sintetico.write_lcms(lc_top, "mrm.D", "mrm"); agilent_sintetico.write_lcms(lc_top, "perfil.D", "profile")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):   # the server answers
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []; asked = []
LOAD = """async () => {
  const rb = await window.MZLAB_RUST_READY;
  await new Promise(r => setTimeout(r, 300));
  const tic = await (await fetch("api/chrom?k=0&level=1")).json();
  return { files: ST.files.map(x => x.name), native: rb.isNative("uno.mzML"), tic, miss: (window.MZLAB_LR_MISS || []).length, loaded: !!(window.LETTORI_MOD && window.LETTORI_MOD.loaded), kept: CONV.files.has("uno.mzML") };
}"""
MZML = """async () => {
  const rb = await window.MZLAB_RUST_READY, f = CONV.files.get("uno.mzML") || await rb.nativeToMzml("uno.mzML", 1 << 20, undefined, undefined);
  const b = new Uint8Array(await f.arrayBuffer()); let s = ""; for (let i = 0; i < b.length; i += 32768) s += String.fromCharCode(...b.subarray(i, i + 32768));
  return btoa(s);
}"""

def opened(pg):
    pg.set_input_files("#pickdir", str(top))
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)

def page(b, query):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("request", lambda r: asked.append(r.url) if "wasm-lettori/" in r.url else None)
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1{query}"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
    return pg
res = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        eng = page(b, ""); mod = page(b, "&lettori=modulo")
        def engine_path():
            opened(eng); r = eng.evaluate(LOAD); assert r["files"] == ["uno.mzML"] and r["native"] and not r["loaded"], r
            res["eng"] = r; res["eng_mzml"] = eng.evaluate(MZML)
        step("without the parameter the engine reads the .D folder and the module is not started", engine_path)
        def module_path():
            opened(mod); r = mod.evaluate(LOAD); assert r["files"] == ["uno.mzML"] and not r["native"] and r["loaded"] and r["kept"], r
            res["mod"] = r; res["mod_mzml"] = mod.evaluate(MZML)
        step("with ?lettori=modulo the module writes the mzML and the file is listed as an mzML", module_path)
        def same_tic():
            assert res["mod"]["tic"] == res["eng"]["tic"] and len(res["mod"]["tic"].get("t", res["mod"]["tic"].get("rt", [1] * N))) > 0, (res["mod"]["tic"], res["eng"]["tic"])
        step("the TIC is the one of the engine's path", same_tic)
        def same_bytes():
            assert res["mod_mzml"] == res["eng_mzml"] and len(res["mod_mzml"]) > 1000, (len(res["mod_mzml"]), len(res["eng_mzml"]))
        step("«Download .mzML» is the engine's mzML byte for byte, without converting again", same_bytes)
        def no_miss():
            assert res["mod"]["miss"] == 0 and res["eng"]["miss"] == 0, (res["mod"]["miss"], res["eng"]["miss"])
        step("MZLAB_LR_MISS is empty", no_miss)
        def one_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "agilent" in fams and fams <= {"agilent", "manifest.json"}, asked
        step("only the module of the family (agilent) is downloaded, not sciex or thermo", one_module)
        def cancel():
            r = mod.evaluate("""async () => { const c = new AbortController(); c.abort(); try { await LETTORI_MOD.toMzml("agilent", [{ name: "a", blob: new Blob([]) }], 0, "x.mzML", undefined, c.signal); return "no"; } catch (e) { return e.cancelled ? "cancelled" : String(e); } }""")
            assert r == "cancelled", r
        step("a cancelled conversion stops", cancel)
        def lc_runs():
            lc = page(b, "&lettori=modulo")
            lc.set_input_files("#pickdir", str(lc_top))
            lc.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            r = lc.evaluate("""async () => {
              const rb = await window.MZLAB_RUST_READY, out = {};
              for (const n of ["mrm.mzML", "perfil.mzML"]) {
                const f = CONV.files.get(n) || await rb.nativeToMzml(n, 1 << 20, undefined, undefined);
                out[n] = await f.text();
              }
              return { names: [...document.querySelectorAll("#flist tr")].map(t => (t.innerText.match(/\\S+\\.mzML/) || [""])[0]).filter(Boolean).sort(), mrm: out["mrm.mzML"], perfil: out["perfil.mzML"], miss: (window.MZLAB_LR_MISS || []).length };
            }""")
            assert r["names"] == ["mrm.mzML", "perfil.mzML"] and r["miss"] == 0, (r["names"], r["miss"])
            assert "SRM SIC Q1=301.1 Q3=150.05" in r["mrm"] and "SRM SIC Q1=301.1 Q3=200.05" in r["mrm"], "no SRM chromatograms"
            assert 'MS:1000130' in r["perfil"] and 'profile' in r["perfil"].lower(), "profile spectra, positive"
            assert 'spectrumList count="40"' in r["perfil"], "profile spectra"
        step("a QqQ MRM run and a Q-TOF profile run open through the module (SRM chromatograms, profile spectra)", lc_runs)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
