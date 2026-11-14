"""«Download .mzML» of a sample the readers module converted at the loading (shell.js `moduloToMzml`) follows the format picked in the menu: the second sample of
`mistopi` (tools/wiff_mixed_pi.py: a product-ion scan only the `sciex` module reads) is saved as mzML and as mzML + Numpress, and each file is byte for byte the one
of `mzlab-cli convert --compression C` (conv.dad false: the optical matrix of the file of the loading is for the UV panel only). Needs cargo, the site, the engine .wasm and the `sciex`
module: python3 tools/prepara_sito.py, tools/build_lettori_wasm.sh site/static."""
# covers: mzlab/web/conversione.js, mzlab/web/lettori-worker
import os, socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists() or not (ROOT / "site" / "static" / "wasm-lettori" / "manifest.json").exists():
    print("  ", ("site", "FAIL site/, its engine or the readers modules not built: python3 tools/prepara_sito.py; tools/build_lettori_wasm.sh site/static")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="numpress_modulo_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_mixed_pi.py"), str(D)], check=True, capture_output=True)
PAIR = [D / "mistopi.wiff", D / "mistopi.wiff.scan"]
cli = {}
for k in ("standard", "smallest"):
    subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "convert", str(PAIR[0]), "--out", str(D / f"cli_{k}"), "--fresh", "--workers", "1",
                    "--compression", k], check=True, cwd=ROOT)
    cli[k] = sorted((D / f"cli_{k}").glob("*.mzML"))      # NAME-ID.mzML, one per sample in the order of the pair
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(300000):      # wait for the server to listen (connection attempts, not a fixed pause)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=0.2).close(); break
    except OSError: pass
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}, accept_downloads=True); c.set_default_timeout(120000); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)))); pg.add_init_script(INIT)
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        pg.set_input_files("#pick", [str(x) for x in PAIR])
        # the mzML files the module wrote at the loading (CONV.files: the pair goes to the module whole); sample 2 is the one the engine does not read
        pg.wait_for_function("CONV.files.size >= 2 && document.querySelectorAll('#flist [data-dl]').length >= 2", timeout=180000)
        name, k = sorted(pg.evaluate("[...CONV.files.keys()]"))[1], 1
        saved = {}
        def save(fmt):
            with pg.expect_download(timeout=120000) as d:
                # the button of that row opens the menu and its item saves (both by script: the row may sit under the sticky header of the page)
                pg.evaluate("n => document.querySelector(`#flist [data-dl=\"${CSS.escape(n)}\"]`).click()", name)
                pg.wait_for_selector(".dlmenu"); pg.evaluate("f => document.querySelector(`.dlmenu [data-dlfmt=${f}]`).click()", fmt)
            path = D / f"site_{fmt}.mzML"; d.value.save_as(str(path)); saved[fmt] = path
        def same(fmt):
            save(fmt)
            assert len(cli[fmt]) == 2, cli[fmt]
            got, want = saved[fmt].read_bytes(), cli[fmt][k].read_bytes()
            assert got == want, (fmt, len(got), len(want))
        step("a sample of the readers module saved as mzML is the file of the CLI", lambda: same("standard"))
        step("the same sample saved as mzML + Numpress is the Numpress file of the CLI", lambda: same("smallest"))
        def smaller():
            assert saved["smallest"].stat().st_size < saved["standard"].stat().st_size, {f: v.stat().st_size for f, v in saved.items()}
        step("the Numpress file is smaller", smaller)
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
        b.close()
finally:
    srv.terminate()
