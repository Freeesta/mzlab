"""A Thermo .raw is read by the engine THROUGH the `thermo` readers module (crates/mzlab-lettori-wasm, mzlab/web/lettore-esterno.js, crates/mzlab-wasm/src/esterno.rs): no mzML in between, the module is downloaded
only when a .raw arrives, and what the page shows is what the engine gave while the Thermo reader was linked in it: the TIC, a scan and «Download .mzML» (byte for byte) against `raw_referenza`
(crates/mzlab-core/examples, the reader of the core). Needs the private data repository (MZLAB_DATI, or `mzlab-dati` beside this one: the smallest .raw of HRMS/; skipped without it), cargo, the site and both .wasm:
python tools/build_site.py, tools/build_lettori_wasm.sh site/static (MZLAB_PIPE_SITE=<built site> reuses a site). Run from the repo root: python3 tests_e2e/e2e_thermo_modulo.py"""
import hashlib, json, os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import agilent_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

def find_raw():
    cands = [os.environ.get("MZLAB_DATI"), str(ROOT.parent / "mzlab-dati")]
    raws = [p for d in cands if d and (Path(d) / "HRMS").is_dir() for p in (Path(d) / "HRMS").rglob("*.raw") if p.is_file()]
    return min(raws, key=lambda p: p.stat().st_size) if raws else None

raw = find_raw()
if raw is None:
    print("   SKIP e2e_thermo_modulo: no .raw in the data repository (MZLAB_DATI)"); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="thermo_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
# what the reader of the core gives (the engine's path before the reader moved to the module)
ref_mzml = tmp / "ref.mzML"
out = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-core", "--example", "raw_referenza", "--", str(raw), str(ref_mzml)], cwd=ROOT, check=True, capture_output=True, text=True).stdout
ref = json.loads(out); ref_sha = hashlib.sha256(ref_mzml.read_bytes()).hexdigest(); name = raw.stem + ".mzML"
folder = agilent_sintetico.write(tmp / "serie", "uno.D", 40).parent

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []; asked = []
def page(b, wasm_log):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("request", lambda r: wasm_log.append(r.url) if "wasm-lettori/" in r.url else None)
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
    pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
    return pg
J = "async u => { const r = await fetch(u); return await r.json(); }"
SHA = """async n => { const rb = await window.MZLAB_RUST_READY, f = CONV.files.get(n) || await rb.nativeToMzml(n, 1 << 20, undefined, undefined);
  const h = await crypto.subtle.digest("SHA-256", await f.arrayBuffer()); return Array.from(new Uint8Array(h), b => b.toString(16).padStart(2, "0")).join(""); }"""
res = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        thermo = []; pg = page(b, thermo)
        def opens():
            pg.set_input_files("#pick", str(raw))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=300000)
            names = pg.evaluate("ST.files.map(x => x.name)"); assert names == [name], names
            assert pg.evaluate("async n => (await window.MZLAB_RUST_READY).isNative(n)", name), "the .raw is not read as it is"
            pg.click("#opbtn"); pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=300000)
        step("a .raw is listed as an mzML and read as it is (no mzML made)", opens)
        def downloaded():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in thermo}
            assert "thermo" in fams and fams <= {"thermo", "manifest.json"}, thermo
        step("only the module `thermo` is downloaded (with the manifest)", downloaded)
        def tic():
            t = pg.evaluate(J, "api/chrom?k=0&level=1"); y = t.get("y") or t.get("tic")
            assert len(t["rt"]) == len(ref["tic"]["rt"]), (len(t["rt"]), len(ref["tic"]["rt"]))
            assert all(abs(a - c) <= 1e-3 + 1e-3 * abs(c) for a, c in zip(t["rt"], ref["tic"]["rt"])), "rt"
            bad = [(i, a, c) for i, (a, c) in enumerate(zip(y, ref["tic"]["y"])) if abs(a - c) > 0.06 + 1e-4 * abs(c)]
            assert not bad, ("tic", len(bad), bad[:3], list(t.keys()))
        step("the TIC is the one of the reader of the core", tic)
        def scan():
            s = pg.evaluate(J, f"api/scan?k=0&sid={ref['scan']['k']}")
            assert s["mz"] == ref["scan"]["mz"] or all(abs(a - c) < 1e-4 for a, c in zip(s["mz"], ref["scan"]["mz"])), "mz"
            y = s.get("y") or s.get("intensity"); assert len(y) == len(ref["scan"]["y"]), (len(y), len(ref["scan"]["y"]))
            assert all(abs(a - c) <= 0.06 + 1e-4 * abs(c) for a, c in zip(y, ref["scan"]["y"])), "intensity"
        step("a scan has the peaks of the reader of the core", scan)
        def mzml():
            sha = pg.evaluate(SHA, name); assert sha == ref_sha, (sha, ref_sha)
        step("«Download .mzML» is the mzML of the reader of the core, byte for byte", mzml)
        def no_miss():
            miss = [m for m in pg.evaluate("window.MZLAB_LR_MISS || []") if "api/spectrum" not in m]       # the Data view asks the unit-resolution spectrum route when it opens a high-resolution file
            assert not miss, miss
        step("MZLAB_LR_MISS is empty", no_miss)
        def no_thermo_for_others():
            other = []; pg2 = page(b, other)
            pg2.set_input_files("#pickdir", str(folder))
            pg2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg2.click("#opbtn"); pg2.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            assert not other, other
        step("a file that is not a .raw (Agilent .D) does not download any module", no_thermo_for_others)
        def broken():
            import base64
            head = base64.b64encode(raw.read_bytes()[:4096]).decode()
            pg3 = page(b, [])
            r = pg3.evaluate("""async head => { const rb = await window.MZLAB_RUST_READY;
              const blob = new Blob([Uint8Array.from(atob(head), c => c.charCodeAt(0))]);
              try { await rb.nativeSamples([{ name: "rotto.raw", blob }]); return { ok: true }; } catch (e) { return { ok: false, key: e.json && e.json.error_key, message: String(e.message) }; } }""", head)
            assert not r["ok"] and str(r["key"]).startswith("err.file."), r
        step("a truncated .raw is refused", broken)
        def baf():
            # the big DDA (595 MB, 56 061 scans) opens in the site and the map appears; skipped when the file is not around
            cands = [os.environ.get("MZLAB_RAW_BAF")] + [str(Path(d) / "dati_formati" / "thermo_raw" / "BAF_GEE_37.raw") for d in (os.environ.get("MZLAB_DATI"), str(ROOT.parent / "mzlab-dati")) if d]
            big = next((Path(c) for c in cands if c and Path(c).is_file()), None)
            if big is None:
                print("   SKIP e2e_thermo_modulo BAF: the big .raw is not around (MZLAB_RAW_BAF)"); return
            pg4 = page(b, [])
            pg4.set_input_files("#pick", str(big))
            pg4.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=600000)
            pg4.click("#opbtn"); pg4.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=900000)
            pg4.click("#np-map"); pg4.wait_for_selector(".pnl.map canvas", timeout=600000)
            pg4.wait_for_function("(() => { const m = E.panels.find(q => q.type === 'map'); return !!m && !!m.cv && m.cv.width > 0; })()", timeout=600000)
            miss = [m for m in pg4.evaluate("window.MZLAB_LR_MISS || []") if "api/spectrum" not in m]
            assert not miss, miss
        step("the big DDA .raw (BAF) opens and the map appears, MZLAB_LR_MISS stays empty", baf)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
