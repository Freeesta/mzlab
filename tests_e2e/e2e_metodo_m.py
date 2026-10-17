"""An Agilent method folder *.m loaded with «Load a folder» next to a .D run (cartella-carica.js, agilent-d.js, metodo-m.js, Rust engine agilent_m_metodo): it is recognised (not
«ignored», not «no compatible file») and the Method window shows its compounds (expected RT, qualifier ions with their response, calibration levels) from the engine's metodo/1.
Synthetic folders of tools/agilent_sintetico.py (no laboratory data). Needs the site and the .wasm like e2e_agilent."""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, ready
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico

from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ and its .wasm not built: python tools/build_site.py")); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="metodo_m_e2e_"))
top = D / "serie"
agilent_sintetico.write(top, "uno.D", 120)
agilent_sintetico.write_method_m(top, "Metodo prova per SIM.m")
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(100):      # wait until the static server accepts connections
    try: socket.create_connection(("127.0.0.1", PORT), 0.2).close(); break
    except OSError: pass
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        def folder():
            pg.set_input_files("#pickdir", str(top))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            t = pg.inner_text("#cartinfo")
            assert "Metodo Agilent «Metodo prova per SIM.m» (.m) riconosciuto" in t and "ignorat" not in t and "nessun file" not in t, t
        step("the .m folder next to a .D run is recognised, not ignored", folder)
        def window():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.click("#np-method"); ready(pg)
            pg.wait_for_function("document.querySelector('#bigdlg').innerText.includes('composto-a')", timeout=60000)
            t = pg.inner_text("#bigdlg")
            assert "Metodo Agilent «Metodo prova per SIM.m» (.m)" in t and "composto-b" in t and "10.25" in t and "7.5" in t, t
            assert "108.1 (25 %, 20–30)" in t and "135.1 (15 %" in t and "0.1, 1, 10 ng/ml" in t, t
            assert "80 %" in t and "fitLinear" in t, t
        step("the Method window shows the compounds, ions and levels of the .m folder", window)
        c.close(); b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
for e in errs: print("  ", e)
sys.exit(1 if errs else 0)
