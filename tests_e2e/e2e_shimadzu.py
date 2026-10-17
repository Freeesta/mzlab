"""Shimadzu files in the page (crates/mzlab-lettori/src/shimadzu, module `shimadzu`, rule R of shell.js upload0): a synthetic .qgd and a synthetic .lcd (tools/shimadzu_sintetico.py, invented numbers, no laboratory data)
dropped in the start page go to the module without any parameter, are listed as mzML, and give the TIC the container states (the sum of each scan's counts); a Q-TOF .lcd is refused with a notice and is not listed; a .lcdx is
still «cannot be opened»; MZLAB_LR_MISS stays empty; only the shimadzu module is downloaded. Needs the site and the modules: python tools/build_site.py, tools/build_lettori_wasm.sh (MZLAB_PIPE_SITE=<built site> reuses a site).
Run from the repo root: python3 tests_e2e/e2e_shimadzu.py"""
import os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import shimadzu_sintetico as sz

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="shimadzu_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
N = 12
qgd, qtic = sz.qgd(N); lcd, ltic = sz.lcd(N)
files = {"gc.qgd": qgd, "lc.lcd": lcd}
for nm, b in files.items(): (tmp / nm).write_bytes(b)
(tmp / "tof.lcd").write_bytes(sz.qtof()); (tmp / "x.lcdx").write_bytes(b"not a container")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []; asked = []
def page(b):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("request", lambda r: asked.append(r.url) if "wasm-lettori/" in r.url else None)
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
    return pg
TIC = """async (k) => { await window.MZLAB_RUST_READY; const t = await (await fetch("api/chrom?k=" + k + "&level=1")).json(); return { y: t.y, keys: Object.keys(t), er: t.error_key || "", pr: JSON.stringify(t.params || {}), n: ST.files.length, files: ST.files.map(x => x.name), miss: (window.MZLAB_LR_MISS || []).length }; }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        pg = page(b)
        def drop():
            pg.set_input_files("#pick", [str(tmp / n) for n in ("gc.qgd", "lc.lcd", "tof.lcd")])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            names = pg.evaluate("ST.files.map(x => x.name)")
            assert sorted(names) == ["gc.mzML", "lc.mzML"], names
        step("a .qgd and an .lcd are converted by the module and listed as mzML; the Q-TOF .lcd is not listed", drop)
        def notice():
            t = pg.inner_text("body")
            assert "Shimadzu" in t or "tof.lcd" in t, t[:400]
        step("the refused Q-TOF file is named in a notice", notice)
        def tics():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
            for nm, want in (("gc.mzML", qtic), ("lc.mzML", ltic)):
                i = pg.evaluate("n => ST.files.findIndex(x => x.name === n)", nm)
                r = pg.evaluate(TIC, i); y = r.get("y") or r.get("tic", {}).get("y")
                assert y and len(y) == N, (nm, r.get('er'), r.get('pr'), r.get('n'), i); assert all(abs(a - c) <= 1e-3 * max(c, 1) for a, c in zip(y, want)), (nm, y and y[:3], want[:3])
                assert r["miss"] == 0, r["miss"]
        step("the TIC of each is the sum of the counts the container states, and nothing misses the engine", tics)
        def one_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "shimadzu" in fams and fams <= {"shimadzu", "manifest.json"}, asked
        step("only the shimadzu module is downloaded", one_module)
        def lcdx():
            pg2 = page(b); pg2.set_input_files("#pick", [str(tmp / "x.lcdx")])
            pg2.wait_for_function("document.querySelector('#unopen') && !document.querySelector('#unopen').hidden", timeout=15000)
            assert ".lcdx" in pg2.inner_text("#unopen")
        step(".lcdx stays «cannot be opened in the browser»", lcdx)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
