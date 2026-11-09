# e2e_modulo_irraggiungibile: a profile .D (tools/agilent_sintetico.py write_lcms) needs the readers module; when wasm-lettori/ cannot be downloaded (offline, a network failure, a file
# missing on the server) the note of the loading says that the reader of the format did not download (err.file.moduleLoad), not that the calculation engine did not load, in both
# languages; nothing opens and no page error. Synthetic data. Needs the site: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_modulo_irraggiungibile.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
D = Path(tempfile.mkdtemp(prefix="modirr_")); top = D / "cartella"; top.mkdir()
agilent_sintetico.write_lcms(top, "perfil.D", "profile", n=200)
NOTE = "[...document.querySelectorAll('[role=alert],[role=status],.note,.toast')].map(n => n.innerText).join(' | ')"
def note_in(lang, want, not_want):
    r = Run(port=8865 if lang == "it" else 8866, wd="/tmp/wd_modirr_" + lang)
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.context.route("**/wasm-lettori/**", lambda rt: rt.fulfill(status=404, body="not here"))      # what a page offline or a server without the modules gives
            pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}")
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            wait_for_js(pg, "window.MZLAB_LR !== undefined", 60000)
            pg.set_input_files("#pickdir", str(top))
            wait_for_js(pg, f"{NOTE}.includes('perfil')", 60000)
            t = pg.evaluate(NOTE)
            assert want in t and not_want not in t, t
            assert pg.evaluate("ST.files.length") == 0, pg.evaluate("ST.files.map(f => f.name)")
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("module unreachable, Italian: the note names the reader that did not download, not the engine",
     lambda: note_in("it", "non si è scaricato", "motore di calcolo non si è caricato"))
step("module unreachable, English: the same note in English",
     lambda: note_in("en", "did not download", "calculation engine did not load"))
shutil.rmtree(D, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
