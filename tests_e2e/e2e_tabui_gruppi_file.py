"""Tabui, low resolution: the table of the files with role, condition, time and replicate (TP_Mine/lazy/gruppi-campioni.js, campioni/2). Synthetic Full Scan series (TP_Mine/tests/lr_synth.py)
copied under names of two conditions (G_O2_t30, G_O2_t60, G_N2_t30, G_N2_t60) and a shared blank. Checks: opening Tabui downloads nothing from lazy/ (the script comes with the second
full-scan file); the conditions are proposed from the names (yellow until «Confirm conditions»), the blank is shared; «Set condition…» on two files, the reference condition, keyboard (Tab,
arrows in a select, Enter on «Confirm conditions»); the notebook (NB.campioni, NB.gruppi_opts) holds it and a reload gives the same table; MZLAB_LR_MISS empty. Builds the site and static/wasm-feat
(SKIP without cargo, wasm-bindgen or numpy); MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one. Run from the repo root."""
import os, shutil, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the wasm of the engine and of mzFinder"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402

tmp = Path(tempfile.mkdtemp(prefix="tabui_gr_files_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:                                                      # fresh sources of mzFinder over the site that was built
    mz = site / "static" / "mzfinder"
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, mz / f.name)
    (mz / "lazy").mkdir(exist_ok=True)
    for f in (ROOT / "TP_Mine" / "lazy").glob("*"): shutil.copy2(f, mz / "lazy" / f.name)
    import json; ix = mz / "indice.json"; d = json.loads(ix.read_text()); d["lazy"] = sorted(f.name for f in (ROOT / "TP_Mine" / "lazy").glob("*.js")); ix.write_text(json.dumps(d))
src = lr_synth.write_series(tmp / "ser", times=(30, 60), blank=True)
by = {Path(f["path"]).name: Path(f["path"]) for f in src}
(tmp / "g").mkdir()
NAMES = ["G_O2_t30.mzML", "G_O2_t60.mzML", "G_N2_t30.mzML", "G_N2_t60.mzML", "G_blank.mzML"]
for n, s in zip(NAMES, ["lr_FullMass-t30.mzML", "lr_FullMass-t60.mzML", "lr_FullMass-t30.mzML", "lr_FullMass-t60.mzML", "lr_blank.mzML"]):
    shutil.copy2(by[s], tmp / "g" / n)
paths = [str(tmp / "g" / n) for n in NAMES]
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; reqs = []
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
G = "#tp-gfiles"
COND = f"[...document.querySelectorAll('{G} .tpg-cond')].map(e => e.value)"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce")
        pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.on("pageerror", lambda e: errs.append(str(e)))
        ctx.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)

        def lazy_idle():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go", timeout=60000)
            assert not [u for u in reqs if "/mzfinder/lazy/" in u], [u for u in reqs if "/lazy/" in u]
            assert pg.evaluate("document.querySelector('#tp-gfiles').hidden")
            idx = pg.evaluate("fetch('static/mzfinder/indice.json').then(r => r.json()).then(j => j.lazy)")
            assert "gruppi-campioni.js" in idx, idx
        step("opening Tabui downloads nothing from lazy/ (the file is listed in indice.json)", lazy_idle)

        def open_files():
            pg.click('#nav button[data-v="data"]') if pg.locator('#nav button[data-v="data"]').count() else None
            pg.set_input_files("#pick", paths)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=240000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-page")
            pg.click("#tp-page"); pg.wait_for_selector(f"{G} table .tpg-cond", timeout=120000)
            assert [u for u in reqs if "/mzfinder/lazy/gruppi-campioni.js" in u], "the script was not asked for"
            assert pg.locator(f"{G} tr .tpg-cond").count() == len(NAMES)
        step("two or more full-scan files: the lazy script loads and the table appears", open_files)

        def proposed():
            c = pg.evaluate(COND)
            assert c[0] == c[1] != "" and c[2] == c[3] != "" and c[0] != c[2] and c[4] == "", c
            assert pg.locator(f"{G} tr.tpg-prop").count() == 4, pg.locator(f"{G} tr.tpg-prop").count()
            roles = pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)")
            assert roles == ["sample"] * 4 + ["blank"], roles
            assert pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-time')].map(e => e.value)")[:2] == ["30", "60"]
            assert pg.locator(f"{G} .tpg-rep").first.get_attribute("aria-label").startswith("Replicate of ")
        step("conditions, roles and times proposed from the names; the blank is shared; yellow rows", proposed)

        def confirm_keys():
            pg.focus(f"{G} button:text-is('Confirm conditions')"); pg.keyboard.press("Enter")
            pg.wait_for_function(f"document.querySelectorAll('{G} tr.tpg-prop').length === 0", timeout=20000)
            assert pg.evaluate("document.activeElement.textContent.trim()") == "Confirm conditions", pg.evaluate("document.activeElement.outerHTML")
        step("Enter on «Confirm conditions» removes the yellow and keeps the focus", confirm_keys)

        def set_two():
            pg.check(f"{G} .tpg-sel[data-i='0']"); pg.check(f"{G} .tpg-sel[data-i='2']")
            pg.click(f"{G} button:text-is('Set condition…')")
            assert pg.evaluate("document.activeElement.getAttribute('aria-label')") == "Condition for the selected files"
            pg.keyboard.type("Air"); pg.keyboard.press("Enter")
            pg.wait_for_function(f"({COND})[0] === 'Air'", timeout=20000)
            c = pg.evaluate(COND); assert c[0] == "Air" and c[2] == "Air" and c[1] != "Air" and c[3] != "Air", c
            assert pg.evaluate("document.activeElement.textContent.trim()") == "Set condition…"
        step("«Set condition…» on two files sets both and returns the focus", set_two)

        def reference():
            opts = pg.evaluate(f"[...document.querySelectorAll('{G} select[aria-label=\"Reference condition\"] option')].map(o => o.value)")
            assert "Air" in opts and "" in opts, opts
            pg.select_option(f"{G} select[aria-label='Reference condition']", "Air")
            pg.wait_for_function("NB.gruppi_opts && NB.gruppi_opts.reference === 'Air'", timeout=20000)
            assert pg.evaluate("NB.campioni.campioni.length") == len(NAMES)
        step("the reference condition is chosen and kept in NB.gruppi_opts", reference)

        def keyboard():
            pg.focus(f"{G} .tpg-role[data-i='4']"); pg.keyboard.press("ArrowDown")
            pg.wait_for_function(f"document.querySelector('{G} .tpg-role[data-i=\"4\"]').value !== 'blank'", timeout=20000)
            assert pg.evaluate("document.activeElement.classList.contains('tpg-role')"), "the focus stays on the select after the change"
            pg.keyboard.press("ArrowUp"); pg.wait_for_function(f"document.querySelector('{G} .tpg-role[data-i=\"4\"]').value === 'blank'", timeout=20000)
            pg.focus(f"{G} .tpg-cond[data-i='1']"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.activeElement.classList.contains('tpg-time')"), pg.evaluate("document.activeElement.outerHTML")
        step("keyboard: arrows in the role select keep the focus; Tab goes from condition to time", keyboard)

        def warnings():
            t = pg.inner_text(f"{G} [role=status]"); assert t.strip() != "", "no warning for a series without a t0"
            assert "No t0" in t and not any(w in t for w in ("Nessun", "tempo")), t
        step("the warnings are in a role=status region, in English", warnings)

        def reload():
            before = pg.evaluate(COND); roles = pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)")
            pg.wait_for_timeout(800); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
            pg.set_input_files("#pick", paths)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=240000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-page"); pg.click("#tp-page")
            pg.wait_for_selector(f"{G} table .tpg-cond", timeout=120000)
            assert pg.evaluate(COND) == before, (before, pg.evaluate(COND))
            assert pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)") == roles
            assert pg.input_value(f"{G} select[aria-label='Reference condition']") == "Air"
            assert pg.locator(f"{G} tr.tpg-prop").count() == 0, "the stored conditions come back confirmed"
        step("after a reload the table is the same, from the notebook", reload)

        def clean():
            assert not errs, errs[:3]
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
            assert not [u for u in reqs if "pyodide" in u.lower()]
        step("no page error, no pyodide* request, MZLAB_LR_MISS empty", clean)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
sys.exit(1 if any(not s[1].startswith("ok") for s in steps) else 0)
