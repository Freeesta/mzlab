"""Tabui: the table of the files with role, condition, time and replicate (TP_Mine/lazy/gruppi-campioni.js, campioni/2), alone on a test page (a container the test adds; Tabui does not use it yet).
Names of two conditions (G_O2_t30, G_O2_t60, G_N2_t30, G_N2_t60) and a shared blank. Checks: opening Tabui downloads nothing from lazy/ (TPLAZY.load brings the script once); the conditions are proposed from the names (yellow until «Confirm conditions»), the blank is shared; «Set condition…» on two files, the reference condition, keyboard (Tab,
arrows in a select, Enter on «Confirm conditions»); the notebook (NB.campioni, NB.gruppi_opts) holds it and a reload gives the same table; MZLAB_LR_MISS empty. Builds the site
(SKIP without cargo or wasm-bindgen); MZLAB_PIPE_SITE=<built site with the engine .wasm> reuses one. Run from the repo root."""
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

tmp = Path(tempfile.mkdtemp(prefix="tabui_gr_files_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
else:                                                      # fresh sources of mzFinder over the site that was built
    mz = site / "static" / "mzfinder"
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, mz / f.name)
    (mz / "lazy").mkdir(exist_ok=True)
    for f in (ROOT / "TP_Mine" / "lazy").glob("*"): shutil.copy2(f, mz / "lazy" / f.name)
    import json; ix = mz / "indice.json"; d = json.loads(ix.read_text()); d["lazy"] = sorted(f.name for f in (ROOT / "TP_Mine" / "lazy").glob("*.js")); ix.write_text(json.dumps(d))
NAMES = ["G_O2_t30.mzML", "G_O2_t60.mzML", "G_N2_t30.mzML", "G_N2_t60.mzML", "G_blank.mzML"]      # names only: the table needs no data
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; reqs = []
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
G = "#tp-gfiles"
COND = f"[...document.querySelectorAll('{G} .tpg-cond')].map(e => e.value)"
BYNAME = f"Object.fromEntries([...document.querySelectorAll('{G} table tr')].slice(1).map(r => [r.children[1].textContent.trim(), r.querySelector('.tpg-cond').value]))"   # the Dig sorts the files by time: rows are found by name
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce")
        pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.on("pageerror", lambda e: errs.append(str(e)))
        ctx.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)

        def lazy_idle():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go", timeout=60000)
            assert not [u for u in reqs if "/mzfinder/lazy/" in u], [u for u in reqs if "/lazy/" in u]
            idx = pg.evaluate("fetch('static/mzfinder/indice.json').then(r => r.json()).then(j => j.lazy)")
            assert "gruppi-campioni.js" in idx, idx
        step("opening Tabui downloads nothing from lazy/ (the file is listed in indice.json)", lazy_idle)

        # the test page: a container in the page, the component mounted by the test (Tabui does not use it yet)
        MOUNT = """async (names) => {
            await window.nbEnsure();                                       // the notebook is read first, or a save would be refused
            await window.TPLAZY.load("gruppi-campioni.js"); await window.TPLAZY.load("gruppi-campioni.js");
            const R = await import(URL.createObjectURL(new Blob([window.QTOOLS.ctx.files["finder-ruoli.js"]], { type: "text/javascript" })));
            const call = async (op, j) => { const r = await fetch("api/campioni/" + op, { method: "POST", body: j }); const t = await r.text(); if (!r.ok) throw new Error(t); return t; };
            let el = document.getElementById("tp-gfiles"); if (el) { el.textContent = ""; } else { el = document.createElement("div"); el.id = "tp-gfiles"; document.body.prepend(el); }
            window.GRCHANGES = [];
            window.GRCTL = await window.TPGRUPPI_FILES.mount(el, names, { R, call, nb: NB, save: nbSave, onChange: r => window.GRCHANGES.push(r) });
        }"""
        def open_table():
            pg.evaluate(MOUNT, NAMES); pg.wait_for_selector(f"{G} table .tpg-cond", timeout=120000)
            n = pg.evaluate("performance.getEntriesByType('resource').filter(e => e.name.includes('/mzfinder/lazy/gruppi-campioni.js')).length")
            assert n == 1, ("the script is asked for once", n, [u for u in reqs if "/lazy/" in u])
            assert pg.locator(f"{G} tr .tpg-cond").count() == len(NAMES)
            assert pg.evaluate("window.GRCHANGES.length") >= 1, "the page is told what the table holds at once"
        step("the lazy script loads once and the table appears", open_table)

        def proposed():
            c = pg.evaluate(BYNAME)
            assert c["G_O2_t30.mzML"] == c["G_O2_t60.mzML"] != "" and c["G_N2_t30.mzML"] == c["G_N2_t60.mzML"] != "" and c["G_O2_t30.mzML"] != c["G_N2_t30.mzML"] and c["G_blank.mzML"] == "", c
            assert pg.locator(f"{G} tr.tpg-prop").count() == 4, pg.locator(f"{G} tr.tpg-prop").count()
            roles = pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)")
            assert roles == ["sample"] * 4 + ["blank"], roles
            assert pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-time')].map(e => e.value)") == ["30", "60", "30", "60", ""]
            assert pg.locator(f"{G} .tpg-rep").first.get_attribute("aria-label").startswith("Replicate of ")
            last = pg.evaluate("window.GRCHANGES.at(-1)"); assert sorted(last["conditions"]) == ["G_N2", "G_O2"], last["conditions"]
        step("conditions, roles and times proposed from the names; the blank is shared; yellow rows", proposed)

        def confirm_keys():
            pg.focus(f"{G} button:text-is('Confirm conditions')"); pg.keyboard.press("Enter")
            pg.wait_for_function(f"document.querySelectorAll('{G} tr.tpg-prop').length === 0", timeout=20000)
            pg.wait_for_function("document.activeElement && document.activeElement.textContent.trim() === 'Confirm conditions'", timeout=20000)
        step("Enter on «Confirm conditions» removes the yellow and keeps the focus", confirm_keys)

        def set_two():
            for n in ("G_O2_t30", "G_O2_t60"): pg.locator(f"{G} tr", has_text=n).locator(".tpg-sel").check()
            pg.click(f"{G} button:text-is('Set condition…')")
            pg.wait_for_function("document.activeElement && document.activeElement.getAttribute('aria-label') === 'Condition for the selected files'", timeout=20000)
            pg.keyboard.type("Air"); pg.keyboard.press("Enter")
            pg.wait_for_function(f"Object.values({BYNAME}).includes('Air')", timeout=20000)
            c = pg.evaluate(BYNAME); air = sorted(n for n in c if c[n] == "Air")
            assert air == ["G_O2_t30.mzML", "G_O2_t60.mzML"], c
            pg.wait_for_function("document.activeElement && document.activeElement.textContent.trim() === 'Set condition…'", timeout=20000)
        step("«Set condition…» on two files sets both and returns the focus", set_two)

        def reference():
            opts = pg.evaluate(f"[...document.querySelectorAll('{G} select[aria-label=\"Reference condition\"] option')].map(o => o.value)")
            assert "Air" in opts and "" in opts, opts
            pg.select_option(f"{G} select[aria-label='Reference condition']", "Air")
            pg.wait_for_function("NB.gruppi_opts && NB.gruppi_opts.reference === 'Air'", timeout=20000)
            assert pg.evaluate("NB.campioni.campioni.length") == len(NAMES)
            assert pg.evaluate("window.GRCHANGES.at(-1).reference") == "Air"
        step("the reference condition is chosen, kept in NB.gruppi_opts and given to the page", reference)

        def keyboard():
            pg.focus(f"{G} .tpg-role[data-i='4']"); pg.keyboard.press("ArrowDown")
            pg.wait_for_function(f"document.querySelector('{G} .tpg-role[data-i=\"4\"]').value !== 'blank'", timeout=20000)
            pg.wait_for_function("document.activeElement && document.activeElement.classList.contains('tpg-role') && document.activeElement.value !== 'blank'", timeout=20000)      # drawn again, the focus is back
            pg.keyboard.press("ArrowUp"); pg.wait_for_function(f"document.querySelector('{G} .tpg-role[data-i=\"4\"]').value === 'blank'", timeout=20000)
            pg.wait_for_function("document.activeElement && document.activeElement.classList.contains('tpg-role') && document.activeElement.value === 'blank'", timeout=20000)      # drawn again, the focus is back
            pg.focus(f"{G} .tpg-cond[data-i='1']"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.activeElement.classList.contains('tpg-time')"), pg.evaluate("document.activeElement.outerHTML")
        step("keyboard: arrows in the role select; Tab goes from condition to time", keyboard)

        def warnings():
            t = pg.inner_text(f"{G} [role=status]"); assert "No t0" in t and not any(w in t for w in ("Nessun", "tempo")), t
        step("the warnings are in a role=status region, in English", warnings)

        def reload():
            before = pg.evaluate(BYNAME); roles = pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)")
            pg.evaluate("nbSave(true)")
            pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(j => 'campioni' in j && 'gruppi_opts' in j)", timeout=20000)
            kept = pg.evaluate("fetch('api/notebook').then(r => r.json()).then(j => Object.keys(j))")
            assert "campioni" in kept and "gruppi_opts" in kept, ("the notebook did not keep it", kept)
            pg.reload(); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=120000)
            pg.wait_for_function("typeof window.nbEnsure === 'function'", timeout=120000); pg.evaluate("window.nbEnsure()")
            try: pg.wait_for_function("window.QTOOLS && window.QTOOLS.ctx && NB.campioni", timeout=30000)
            except Exception: raise AssertionError("NB after the reload: " + pg.evaluate("JSON.stringify([Object.keys(NB), !!(window.QTOOLS && QTOOLS.ctx)])"))
            pg.evaluate(MOUNT, NAMES); pg.wait_for_selector(f"{G} table .tpg-cond", timeout=120000)
            assert pg.evaluate(BYNAME) == before, (before, pg.evaluate(BYNAME))
            now = pg.evaluate(f"[...document.querySelectorAll('{G} .tpg-role')].map(e => e.value)"); assert now == roles, (roles, now)
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
