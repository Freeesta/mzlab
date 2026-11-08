# e2e_impronta_opfs: a sample the readers module converts at the loading (a Q-TOF profile .D, tools/agilent_sintetico.py write_lcms) keeps ONE mzML in OPFS: the copy of the
# engine (files/), not also the module's output in conv/ (conversione.js CONV.settle); the page still reads that mzML and «Download .mzML» still saves it. Synthetic data, no laboratory
# files. Needs the site, the engine .wasm and the readers modules: python3 tools/prepara_sito.py, tools/build_lettori_wasm.sh site/static, then python3 tests_e2e/e2e_impronta_opfs.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
if not (ROOT / "site" / "static" / "wasm-lettori" / "manifest.json").exists():
    print("  ", ("site", "FAIL the readers modules not built: tools/build_lettori_wasm.sh site/static")); sys.exit(1)
D = Path(tempfile.mkdtemp(prefix="impronta_")); top = D / "cartella"; top.mkdir()
agilent_sintetico.write_lcms(top, "perfil.D", "profile", n=2000)
# every file of OPFS mzlab/ with its size
WALK = """async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
  await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; }"""
NOCONV = "(" + WALK + ")().then(o => Object.keys(o).every(k => !k.startsWith('conv/')))"
def one_copy():
    r = Run(port=8864, wd="/tmp/wd_impronta_opfs")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            wait_for_js(pg, "window.MZLAB_LR !== undefined", 60000)
            pg.set_input_files("#pickdir", str(top))
            wait_for_js(pg, "ST.files.some(f => f.name === 'perfil.mzML' && OPENNOW.isOpen(f.name)) && UPBUSY === 0 && OPENNOW.busy() === 0", 180000)
            try: wait_for_js(pg, NOCONV, 15000)      # the module's copy goes right after the loading
            except TimeoutError: pass
            o = pg.evaluate(WALK)
            assert [k for k in o if k.startswith("conv/")] == [], f"the module's mzML stays in conv/ next to the engine's copy: {o}"
            assert o.get("files/perfil.mzML", 0) > 0, o
            head = pg.evaluate("CONV.files.get('perfil.mzML').slice(0, 64).text()")
            assert head.startswith("<?xml"), head
            with pg.expect_download(timeout=120000) as d:
                pg.evaluate("document.querySelector('#flist [data-dl=\"perfil.mzML\"]').click()")
                pg.wait_for_selector(".dlmenu"); pg.evaluate("document.querySelector('.dlmenu [data-dlfmt=standard]').click()")
            got = D / "got.mzML"; d.value.save_as(str(got))
            assert got.read_bytes()[:5] == b"<?xml" and got.stat().st_size > 0, got.stat().st_size
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("a profile .D read by the readers module keeps one mzML in OPFS; the page reads it and «Download .mzML» saves it", one_copy)
shutil.rmtree(D, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
